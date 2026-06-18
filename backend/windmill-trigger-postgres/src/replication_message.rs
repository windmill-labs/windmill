#![allow(unused)]

use core::str;
use std::{
    cmp,
    io::{self, Cursor, Read},
    str::Utf8Error,
};

use byteorder::{BigEndian, ReadBytesExt};
use bytes::Bytes;
use rust_postgres::types::{Oid, Type};
use thiserror::Error;

use super::listener::LogicalReplicationSettings;
const PRIMARY_KEEPALIVE_BYTE: u8 = b'k';
const X_LOG_DATA_BYTE: u8 = b'w';

/**
* This implementation is inspired by Postgres replication functionality
* from https://github.com/supabase/pg_replicate
*
* Original implementation:
* - https://github.com/supabase/pg_replicate/blob/main/pg_replicate/src/conversions/cdc_event.rs
*
*/

#[derive(Debug)]
pub struct PrimaryKeepAliveBody {
    pub wal_end: u64,
    pub timestamp: i64,
    pub reply: bool,
}

impl PrimaryKeepAliveBody {
    pub fn new(wal_end: u64, timestamp: i64, reply: bool) -> PrimaryKeepAliveBody {
        PrimaryKeepAliveBody { wal_end, timestamp, reply }
    }
}

const BEGIN_BYTE: u8 = b'B';
const COMMIT_BYTE: u8 = b'C';
const ORIGIN_BYTE: u8 = b'O';
const RELATION_BYTE: u8 = b'R';
const TYPE_BYTE: u8 = b'Y';
const INSERT_BYTE: u8 = b'I';
const UPDATE_BYTE: u8 = b'U';
const DELETE_BYTE: u8 = b'D';
const TUPLE_NEW_BYTE: u8 = b'N';
const TUPLE_KEY_BYTE: u8 = b'K';
const TUPLE_OLD_BYTE: u8 = b'O';
const TUPLE_DATA_NULL_BYTE: u8 = b'n';
const TUPLE_DATA_TOAST_BYTE: u8 = b'u';
const TUPLE_DATA_TEXT_BYTE: u8 = b't';
const TUPLE_DATA_BINARY_BYTE: u8 = b'b';

const REPLICA_IDENTITY_DEFAULT_BYTE: i8 = 0x64;
const REPLICA_IDENTITY_NOTHING_BYTE: i8 = 0x6E;
const REPLICA_IDENTITY_FULL_BYTE: i8 = 0x66;
const REPLICA_IDENTITY_INDEX_BYTE: i8 = 0x69;

#[derive(Debug)]
pub enum ReplicaIdentity {
    Default,
    Nothing,
    Full,
    Index,
}

#[derive(Debug)]
pub struct Column {
    pub flags: i8,
    pub name: String,
    pub type_o_id: Option<Type>,
    pub type_modifier: i32,
}

impl Column {
    pub fn new(flags: i8, name: String, type_o_id: Option<Type>, type_modifier: i32) -> Self {
        Self { flags, name, type_o_id, type_modifier }
    }
}

pub type Columns = Vec<Column>;

#[derive(Debug)]
pub struct RelationBody {
    pub transaction_id: Option<i32>,
    pub o_id: Oid,
    pub namespace: String,
    pub name: String,
    pub replica_identity: ReplicaIdentity,
    pub columns: Columns,
}

impl RelationBody {
    pub fn new(
        transaction_id: Option<i32>,
        o_id: Oid,
        namespace: String,
        name: String,
        replica_identity: ReplicaIdentity,
        columns: Columns,
    ) -> Self {
        Self { transaction_id, o_id, namespace, name, replica_identity, columns }
    }
}

#[derive(Debug)]
pub struct InsertBody {
    pub transaction_id: Option<i32>,
    pub o_id: Oid,
    pub tuple: Vec<TupleData>,
}

impl InsertBody {
    pub fn new(transaction_id: Option<i32>, o_id: Oid, tuple: Vec<TupleData>) -> Self {
        Self { transaction_id, o_id, tuple }
    }
}

#[derive(Debug)]
pub struct UpdateBody {
    transaction_id: Option<i32>,
    pub o_id: Oid,
    pub old_tuple: Option<Vec<TupleData>>,
    pub key_tuple: Option<Vec<TupleData>>,
    pub new_tuple: Vec<TupleData>,
}

impl UpdateBody {
    pub fn new(
        transaction_id: Option<i32>,
        o_id: Oid,
        old_tuple: Option<Vec<TupleData>>,
        key_tuple: Option<Vec<TupleData>>,
        new_tuple: Vec<TupleData>,
    ) -> Self {
        Self { transaction_id, o_id, old_tuple, key_tuple, new_tuple }
    }
}

#[derive(Debug)]
pub struct DeleteBody {
    transaction_id: Option<i32>,
    pub o_id: Oid,
    pub old_tuple: Option<Vec<TupleData>>,
    pub key_tuple: Option<Vec<TupleData>>,
}

impl DeleteBody {
    pub fn new(
        transaction_id: Option<i32>,
        o_id: Oid,
        old_tuple: Option<Vec<TupleData>>,
        key_tuple: Option<Vec<TupleData>>,
    ) -> Self {
        Self { transaction_id, o_id, old_tuple, key_tuple }
    }
}

#[derive(Debug)]
pub enum TupleData {
    Null,
    UnchangedToast,
    Text(Bytes),
    Binary(Bytes),
}

impl TupleData {
    fn parse(buf: &mut Buffer) -> Result<Vec<TupleData>, ConversionError> {
        let number_of_columns = buf.read_i16::<BigEndian>()?;
        let mut tuples = Vec::with_capacity(number_of_columns as usize);
        for _ in 0..number_of_columns {
            let byte = buf.read_u8()?;
            let tuple_data = match byte {
                TUPLE_DATA_NULL_BYTE => TupleData::Null,
                TUPLE_DATA_TOAST_BYTE => TupleData::UnchangedToast,
                TUPLE_DATA_TEXT_BYTE => {
                    let len = buf.read_i32::<BigEndian>()?;
                    let mut data = vec![0; len as usize];
                    buf.read_exact(&mut data)?;
                    TupleData::Text(data.into())
                }
                TUPLE_DATA_BINARY_BYTE => {
                    let len = buf.read_i32::<BigEndian>()?;
                    let mut data = vec![0; len as usize];
                    buf.read_exact(&mut data)?;
                    TupleData::Binary(data.into())
                }
                byte => {
                    return Err(ConversionError::Io(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("unknown replication message byte `{}`", byte),
                    )));
                }
            };

            tuples.push(tuple_data);
        }

        Ok(tuples)
    }
}

#[derive(Debug)]
pub enum TransactionBody {
    Insert(InsertBody),
    Update(UpdateBody),
    Delete(DeleteBody),
}

#[non_exhaustive]
#[derive(Debug)]
pub enum LogicalReplicationMessage {
    Begin,
    Commit,
    Relation(RelationBody),
    Type,
    Insert(InsertBody),
    Update(UpdateBody),
    Delete(DeleteBody),
}

#[derive(Debug)]
pub struct XLogDataBody {
    pub wal_start: u64,
    pub wal_end: u64,
    pub timestamp: i64,
    pub data: Bytes,
}

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Error: {0}")]
    Io(#[from] io::Error),
    #[error("Utf8Error conversion: {0}")]
    Utf8(#[from] Utf8Error),
}

struct Buffer {
    bytes: Bytes,
    idx: usize,
}

impl Buffer {
    pub fn new(bytes: Bytes, idx: usize) -> Buffer {
        Buffer { bytes, idx }
    }

    fn slice(&self) -> &[u8] {
        &self.bytes[self.idx..]
    }

    fn read_cstr(&mut self) -> Result<String, ConversionError> {
        match self.slice().iter().position(|&x| x == 0) {
            Some(pos) => {
                let start = self.idx;
                let end = start + pos;
                let cstr = str::from_utf8(&self.bytes[start..end])?.to_owned();
                self.idx = end + 1;
                Ok(cstr)
            }
            None => Err(ConversionError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected EOF",
            ))),
        }
    }
}

impl Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = {
            let slice = self.slice();
            let len = cmp::min(slice.len(), buf.len());
            buf[..len].copy_from_slice(&slice[..len]);
            len
        };
        self.idx += len;
        Ok(len)
    }
}

impl XLogDataBody {
    pub fn new(wal_start: u64, wal_end: u64, timestamp: i64, data: Bytes) -> XLogDataBody {
        XLogDataBody { wal_start, wal_end, timestamp, data }
    }

    pub fn parse(
        self,
        logical_replication_settings: &LogicalReplicationSettings,
    ) -> Result<LogicalReplicationMessage, ConversionError> {
        let mut buf = Buffer::new(self.data.clone(), 0);
        let byte = buf.read_u8()?;

        let logical_replication_message = match byte {
            BEGIN_BYTE => {
                buf.read_i64::<BigEndian>()?;
                buf.read_i64::<BigEndian>()?;
                buf.read_i32::<BigEndian>()?;

                LogicalReplicationMessage::Begin
            }
            COMMIT_BYTE => {
                buf.read_i8()?;
                buf.read_u64::<BigEndian>()?;
                buf.read_u64::<BigEndian>()?;
                buf.read_i64::<BigEndian>()?;
                LogicalReplicationMessage::Commit
            }
            RELATION_BYTE => {
                let transaction_id = match logical_replication_settings.streaming {
                    true => Some(buf.read_i32::<BigEndian>()?),
                    false => None,
                };

                let o_id = buf.read_u32::<BigEndian>()?;
                let namespace = buf.read_cstr()?;
                let name = buf.read_cstr()?;
                let replica_identity = match buf.read_i8()? {
                    REPLICA_IDENTITY_DEFAULT_BYTE => ReplicaIdentity::Default,
                    REPLICA_IDENTITY_NOTHING_BYTE => ReplicaIdentity::Nothing,
                    REPLICA_IDENTITY_FULL_BYTE => ReplicaIdentity::Full,
                    REPLICA_IDENTITY_INDEX_BYTE => ReplicaIdentity::Index,
                    byte => {
                        return Err(ConversionError::Io(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("unknown replica identity byte `{}`", byte),
                        )));
                    }
                };

                let num_of_column = buf.read_i16::<BigEndian>()?;

                let mut columns = Vec::with_capacity(num_of_column as usize);
                for _ in 0..num_of_column {
                    let flags = buf.read_i8()?;
                    let name = buf.read_cstr()?;
                    let o_id = buf.read_u32::<BigEndian>()?;
                    let type_modifier = buf.read_i32::<BigEndian>()?;
                    let type_o_id = Type::from_oid(o_id);
                    let column = Column::new(flags, name, type_o_id, type_modifier);

                    columns.push(column);
                }

                LogicalReplicationMessage::Relation(RelationBody::new(
                    transaction_id,
                    o_id,
                    namespace,
                    name,
                    replica_identity,
                    columns,
                ))
            }
            TYPE_BYTE => {
                buf.read_u32::<BigEndian>()?;
                buf.read_cstr()?;
                buf.read_cstr()?;

                LogicalReplicationMessage::Type
            }
            INSERT_BYTE => {
                let transaction_id = match logical_replication_settings.streaming {
                    true => Some(buf.read_i32::<BigEndian>()?),
                    false => None,
                };
                let o_id = buf.read_u32::<BigEndian>()?;
                let byte = buf.read_u8()?;

                let tuple = match byte {
                    TUPLE_NEW_BYTE => TupleData::parse(&mut buf)?,
                    byte => {
                        return Err(ConversionError::Io(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("unexpected tuple byte `{}`", byte),
                        )));
                    }
                };

                LogicalReplicationMessage::Insert(InsertBody::new(transaction_id, o_id, tuple))
            }
            UPDATE_BYTE => {
                let transaction_id = match logical_replication_settings.streaming {
                    true => Some(buf.read_i32::<BigEndian>()?),
                    false => None,
                };
                let o_id = buf.read_u32::<BigEndian>()?;
                let byte = buf.read_u8()?;
                let mut key_tuple = None;
                let mut old_tuple = None;

                let new_tuple = match byte {
                    TUPLE_NEW_BYTE => TupleData::parse(&mut buf)?,
                    TUPLE_OLD_BYTE | TUPLE_KEY_BYTE => {
                        if byte == TUPLE_OLD_BYTE {
                            old_tuple = Some(TupleData::parse(&mut buf)?);
                        } else {
                            key_tuple = Some(TupleData::parse(&mut buf)?);
                        }
                        match buf.read_u8()? {
                            TUPLE_NEW_BYTE => TupleData::parse(&mut buf)?,
                            byte => {
                                return Err(ConversionError::Io(io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    format!("unexpected tuple byte `{}`", byte),
                                )));
                            }
                        }
                    }
                    byte => {
                        return Err(ConversionError::Io(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("unknown tuple byte `{}`", byte),
                        )));
                    }
                };

                LogicalReplicationMessage::Update(UpdateBody::new(
                    transaction_id,
                    o_id,
                    old_tuple,
                    key_tuple,
                    new_tuple,
                ))
            }
            DELETE_BYTE => {
                let transaction_id = match logical_replication_settings.streaming {
                    true => Some(buf.read_i32::<BigEndian>()?),
                    false => None,
                };
                let o_id = buf.read_u32::<BigEndian>()?;
                let tag = buf.read_u8()?;

                let mut key_tuple = None;
                let mut old_tuple = None;

                match tag {
                    TUPLE_OLD_BYTE => old_tuple = Some(TupleData::parse(&mut buf)?),
                    TUPLE_KEY_BYTE => key_tuple = Some(TupleData::parse(&mut buf)?),
                    tag => {
                        return Err(ConversionError::Io(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("unknown tuple tag `{}`", tag),
                        )));
                    }
                }

                LogicalReplicationMessage::Delete(DeleteBody::new(
                    transaction_id,
                    o_id,
                    old_tuple,
                    key_tuple,
                ))
            }
            byte => {
                return Err(ConversionError::Io(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown replication message tag `{}`", byte),
                )));
            }
        };

        Ok(logical_replication_message)
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub enum ReplicationMessage {
    XLogData(XLogDataBody),
    PrimaryKeepAlive(PrimaryKeepAliveBody),
}

impl ReplicationMessage {
    pub fn parse(buf: Bytes) -> io::Result<Self> {
        let (byte, mut message) = buf.split_first().unwrap();

        let replication_message = match *byte {
            X_LOG_DATA_BYTE => {
                let len = buf.len();
                let wal_start = message.read_u64::<BigEndian>()?;
                let wal_end = message.read_u64::<BigEndian>()?;
                let timestamp = message.read_i64::<BigEndian>()?;
                let len = len - message.len();
                let data = buf.slice(len..);
                ReplicationMessage::XLogData(XLogDataBody::new(wal_start, wal_end, timestamp, data))
            }
            PRIMARY_KEEPALIVE_BYTE => {
                let wal_end = message.read_u64::<BigEndian>()?;
                let timestamp = message.read_i64::<BigEndian>()?;
                let reply = message.read_u8()?;
                ReplicationMessage::PrimaryKeepAlive(PrimaryKeepAliveBody::new(
                    wal_end,
                    timestamp,
                    reply == 1,
                ))
            }
            byte => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown replication message byte `{}`", byte),
                ));
            }
        };

        Ok(replication_message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_keepalive(wal_end: u64, timestamp: i64, reply: bool) -> Bytes {
        let mut buf = Vec::new();
        buf.push(PRIMARY_KEEPALIVE_BYTE);
        buf.extend_from_slice(&wal_end.to_be_bytes());
        buf.extend_from_slice(&timestamp.to_be_bytes());
        buf.push(if reply { 1 } else { 0 });
        Bytes::from(buf)
    }

    fn build_xlog_data(wal_start: u64, wal_end: u64, timestamp: i64, data: &[u8]) -> Bytes {
        let mut buf = Vec::new();
        buf.push(X_LOG_DATA_BYTE);
        buf.extend_from_slice(&wal_start.to_be_bytes());
        buf.extend_from_slice(&wal_end.to_be_bytes());
        buf.extend_from_slice(&timestamp.to_be_bytes());
        buf.extend_from_slice(data);
        Bytes::from(buf)
    }

    #[test]
    fn test_parse_keepalive_with_reply() {
        let buf = build_keepalive(100, 200, true);
        match ReplicationMessage::parse(buf).unwrap() {
            ReplicationMessage::PrimaryKeepAlive(body) => {
                assert_eq!(body.wal_end, 100);
                assert_eq!(body.timestamp, 200);
                assert!(body.reply);
            }
            _ => panic!("expected PrimaryKeepAlive"),
        }
    }

    #[test]
    fn test_parse_keepalive_without_reply() {
        let buf = build_keepalive(500, 1000, false);
        match ReplicationMessage::parse(buf).unwrap() {
            ReplicationMessage::PrimaryKeepAlive(body) => {
                assert_eq!(body.wal_end, 500);
                assert_eq!(body.timestamp, 1000);
                assert!(!body.reply);
            }
            _ => panic!("expected PrimaryKeepAlive"),
        }
    }

    #[test]
    fn test_parse_xlog_data() {
        let payload = b"test payload";
        let buf = build_xlog_data(10, 20, 30, payload);
        match ReplicationMessage::parse(buf).unwrap() {
            ReplicationMessage::XLogData(body) => {
                assert_eq!(body.wal_start, 10);
                assert_eq!(body.wal_end, 20);
                assert_eq!(body.timestamp, 30);
                assert_eq!(&body.data[..], payload);
            }
            _ => panic!("expected XLogData"),
        }
    }

    #[test]
    fn test_parse_unknown_byte() {
        let buf = Bytes::from(vec![0xFF, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(ReplicationMessage::parse(buf).is_err());
    }

    fn build_begin_message() -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(BEGIN_BYTE);
        buf.extend_from_slice(&0i64.to_be_bytes()); // lsn
        buf.extend_from_slice(&0i64.to_be_bytes()); // timestamp
        buf.extend_from_slice(&0i32.to_be_bytes()); // xid
        buf
    }

    fn build_commit_message() -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(COMMIT_BYTE);
        buf.push(0); // flags
        buf.extend_from_slice(&0u64.to_be_bytes()); // lsn
        buf.extend_from_slice(&0u64.to_be_bytes()); // end_lsn
        buf.extend_from_slice(&0i64.to_be_bytes()); // timestamp
        buf
    }

    fn build_insert_message(o_id: u32, tuple_data: &[(u8, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(INSERT_BYTE);
        buf.extend_from_slice(&o_id.to_be_bytes());
        buf.push(TUPLE_NEW_BYTE);
        buf.extend_from_slice(&(tuple_data.len() as i16).to_be_bytes());
        for (tag, data) in tuple_data {
            buf.push(*tag);
            match *tag {
                TUPLE_DATA_TEXT_BYTE | TUPLE_DATA_BINARY_BYTE => {
                    buf.extend_from_slice(&(data.len() as i32).to_be_bytes());
                    buf.extend_from_slice(data);
                }
                _ => {}
            }
        }
        buf
    }

    fn build_delete_message(o_id: u32, tuple_data: &[(u8, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(DELETE_BYTE);
        buf.extend_from_slice(&o_id.to_be_bytes());
        buf.push(TUPLE_OLD_BYTE);
        buf.extend_from_slice(&(tuple_data.len() as i16).to_be_bytes());
        for (tag, data) in tuple_data {
            buf.push(*tag);
            match *tag {
                TUPLE_DATA_TEXT_BYTE | TUPLE_DATA_BINARY_BYTE => {
                    buf.extend_from_slice(&(data.len() as i32).to_be_bytes());
                    buf.extend_from_slice(data);
                }
                _ => {}
            }
        }
        buf
    }

    fn settings(streaming: bool) -> LogicalReplicationSettings {
        LogicalReplicationSettings { streaming }
    }

    #[test]
    fn test_parse_begin() {
        let data = Bytes::from(build_begin_message());
        let body = XLogDataBody::new(0, 0, 0, data);
        match body.parse(&settings(false)).unwrap() {
            LogicalReplicationMessage::Begin => {}
            other => panic!("expected Begin, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_commit() {
        let data = Bytes::from(build_commit_message());
        let body = XLogDataBody::new(0, 0, 0, data);
        match body.parse(&settings(false)).unwrap() {
            LogicalReplicationMessage::Commit => {}
            other => panic!("expected Commit, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_insert_with_text_tuple() {
        let data = Bytes::from(build_insert_message(
            42,
            &[(TUPLE_DATA_TEXT_BYTE, b"hello")],
        ));
        let body = XLogDataBody::new(0, 0, 0, data);
        match body.parse(&settings(false)).unwrap() {
            LogicalReplicationMessage::Insert(insert) => {
                assert_eq!(insert.o_id, 42);
                assert_eq!(insert.tuple.len(), 1);
                match &insert.tuple[0] {
                    TupleData::Text(b) => assert_eq!(&b[..], b"hello"),
                    other => panic!("expected Text, got {:?}", other),
                }
            }
            other => panic!("expected Insert, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_insert_with_null_tuple() {
        let data = Bytes::from(build_insert_message(10, &[(TUPLE_DATA_NULL_BYTE, &[])]));
        let body = XLogDataBody::new(0, 0, 0, data);
        match body.parse(&settings(false)).unwrap() {
            LogicalReplicationMessage::Insert(insert) => {
                assert_eq!(insert.tuple.len(), 1);
                assert!(matches!(insert.tuple[0], TupleData::Null));
            }
            other => panic!("expected Insert, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_insert_with_toast_tuple() {
        let data = Bytes::from(build_insert_message(10, &[(TUPLE_DATA_TOAST_BYTE, &[])]));
        let body = XLogDataBody::new(0, 0, 0, data);
        match body.parse(&settings(false)).unwrap() {
            LogicalReplicationMessage::Insert(insert) => {
                assert!(matches!(insert.tuple[0], TupleData::UnchangedToast));
            }
            other => panic!("expected Insert, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_insert_multiple_columns() {
        let data = Bytes::from(build_insert_message(
            1,
            &[
                (TUPLE_DATA_TEXT_BYTE, b"col1"),
                (TUPLE_DATA_NULL_BYTE, &[]),
                (TUPLE_DATA_TEXT_BYTE, b"col3"),
            ],
        ));
        let body = XLogDataBody::new(0, 0, 0, data);
        match body.parse(&settings(false)).unwrap() {
            LogicalReplicationMessage::Insert(insert) => {
                assert_eq!(insert.tuple.len(), 3);
                assert!(matches!(&insert.tuple[0], TupleData::Text(_)));
                assert!(matches!(insert.tuple[1], TupleData::Null));
                assert!(matches!(&insert.tuple[2], TupleData::Text(_)));
            }
            other => panic!("expected Insert, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_delete_with_old_tuple() {
        let data = Bytes::from(build_delete_message(
            99,
            &[(TUPLE_DATA_TEXT_BYTE, b"old_val")],
        ));
        let body = XLogDataBody::new(0, 0, 0, data);
        match body.parse(&settings(false)).unwrap() {
            LogicalReplicationMessage::Delete(delete) => {
                assert_eq!(delete.o_id, 99);
                assert!(delete.old_tuple.is_some());
                assert!(delete.key_tuple.is_none());
            }
            other => panic!("expected Delete, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_relation() {
        let mut buf = Vec::new();
        buf.push(RELATION_BYTE);
        buf.extend_from_slice(&100u32.to_be_bytes()); // o_id
        buf.extend_from_slice(b"public\0"); // namespace
        buf.extend_from_slice(b"users\0"); // name
        buf.push(REPLICA_IDENTITY_DEFAULT_BYTE as u8); // replica identity
        buf.extend_from_slice(&1i16.to_be_bytes()); // num columns
                                                    // column: flags=0, name="id", type_oid=23 (INT4), type_modifier=-1
        buf.push(0); // flags
        buf.extend_from_slice(b"id\0"); // name
        buf.extend_from_slice(&23u32.to_be_bytes()); // type_oid (INT4)
        buf.extend_from_slice(&(-1i32).to_be_bytes()); // type_modifier

        let data = Bytes::from(buf);
        let body = XLogDataBody::new(0, 0, 0, data);
        match body.parse(&settings(false)).unwrap() {
            LogicalReplicationMessage::Relation(rel) => {
                assert_eq!(rel.o_id, 100);
                assert_eq!(rel.namespace, "public");
                assert_eq!(rel.name, "users");
                assert_eq!(rel.columns.len(), 1);
                assert_eq!(rel.columns[0].name, "id");
                assert!(matches!(rel.columns[0].type_o_id, Some(Type::INT4)));
            }
            other => panic!("expected Relation, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_insert_with_streaming_transaction_id() {
        let mut buf = Vec::new();
        buf.push(INSERT_BYTE);
        buf.extend_from_slice(&42i32.to_be_bytes()); // transaction_id
        buf.extend_from_slice(&10u32.to_be_bytes()); // o_id
        buf.push(TUPLE_NEW_BYTE);
        buf.extend_from_slice(&0i16.to_be_bytes()); // 0 columns

        let data = Bytes::from(buf);
        let body = XLogDataBody::new(0, 0, 0, data);
        match body.parse(&settings(true)).unwrap() {
            LogicalReplicationMessage::Insert(insert) => {
                assert_eq!(insert.transaction_id, Some(42));
                assert_eq!(insert.o_id, 10);
            }
            other => panic!("expected Insert, got {:?}", other),
        }
    }

    #[test]
    fn test_unknown_tuple_data_byte() {
        let mut buf = Vec::new();
        buf.push(INSERT_BYTE);
        buf.extend_from_slice(&1u32.to_be_bytes()); // o_id
        buf.push(TUPLE_NEW_BYTE);
        buf.extend_from_slice(&1i16.to_be_bytes()); // 1 column
        buf.push(0xFF); // invalid tuple data byte

        let data = Bytes::from(buf);
        let body = XLogDataBody::new(0, 0, 0, data);
        assert!(body.parse(&settings(false)).is_err());
    }
}
