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
