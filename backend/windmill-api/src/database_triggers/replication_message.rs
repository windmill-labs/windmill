use core::str;
use std::{
    cmp,
    io::{self, Read},
    str::Utf8Error,
};

use byteorder::{BigEndian, ReadBytesExt};
use bytes::Bytes;
use memchr::memchr;
use thiserror::Error;

use super::trigger::LogicalReplicationSettings;
const PRIMARY_KEEPALIVE_BYTE: u8 = b'k';
const X_LOG_DATA_BYTE: u8 = b'w';

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
const TRUNCATE_BYTE: u8 = b'T';
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
pub struct BeginBody {
    last_lsn: i64,
    timestamp: i64,
    transaction_id: i32,
}

impl BeginBody {
    pub fn new(last_lsn: i64, timestamp: i64, transaction_id: i32) -> Self {
        Self { last_lsn, timestamp, transaction_id }
    }
}

#[derive(Debug)]
pub struct CommitBody {
    pub flags: i8,
    pub commit_lsn: u64,
    pub end_lsn: u64,
    pub timestamp: i64,
}

impl CommitBody {
    pub fn new(flags: i8, commit_lsn: u64, end_lsn: u64, timestamp: i64) -> Self {
        Self { flags, commit_lsn, end_lsn, timestamp }
    }
}

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
    pub type_id: i32,
    pub type_modifier: i32,
}

impl Column {
    pub fn new(flags: i8, name: String, type_id: i32, type_modifier: i32) -> Self {
        Self { flags, name, type_id, type_modifier }
    }
}

#[derive(Debug)]
pub struct RelationBody {
    pub transaction_id: Option<i32>,
    pub relation_id: i32,
    pub namespace: String,
    pub name: String,
    pub replica_identity: ReplicaIdentity,
    pub columns: Vec<Column>,
}

impl RelationBody {
    pub fn new(
        transaction_id: Option<i32>,
        relation_id: i32,
        namespace: String,
        name: String,
        replica_identity: ReplicaIdentity,
        columns: Vec<Column>,
    ) -> Self {
        Self { transaction_id, relation_id, namespace, name, replica_identity, columns }
    }
}

#[derive(Debug)]
pub struct TypeBody {
    oid: i32,
    namespace: String,
    name: String,
}

impl TypeBody {
    pub fn new(oid: i32, namespace: String, name: String) -> TypeBody {
        TypeBody { oid, namespace, name }
    }
}

#[derive(Debug)]
pub struct InsertBody {
    pub transaction_id: Option<i32>,
    pub o_id: i32,
    pub tuple: Vec<TupleData>,
}

impl InsertBody {
    pub fn new(transaction_id: Option<i32>, o_id: i32, tuple: Vec<TupleData>) -> Self {
        Self { transaction_id, o_id, tuple }
    }
}

#[derive(Debug)]
pub struct UpdateBody {
    rel_id: u32,
    old_tuple: Option<Vec<TupleData>>,
    key_tuple: Option<Vec<TupleData>>,
    new_tuple: Vec<TupleData>,
}

#[derive(Debug)]
pub struct DeleteBody {
    rel_id: u32,
    old_tuple: Option<Vec<TupleData>>,
    key_tuple: Option<Vec<TupleData>>,
}

#[derive(Debug)]
pub enum TupleData {
    Null,
    UnchangedToast,
    Text(Bytes),
    Binary(Bytes),
}

impl TupleData {
    fn parse(mut buf: Buffer) -> Result<Vec<TupleData>, ConversionError> {
        let number_of_columns = buf.read_i16::<BigEndian>().map_err(ConversionError::Io)?;
        let mut tuples = Vec::with_capacity(number_of_columns as usize);
        for _ in 0..number_of_columns {
            let byte = buf.read_u8().map_err(ConversionError::Io)?;
            let tuple_data = match byte {
                TUPLE_DATA_NULL_BYTE => TupleData::Null,
                TUPLE_DATA_TOAST_BYTE => TupleData::UnchangedToast,
                TUPLE_DATA_TEXT_BYTE => {
                    let len = buf.read_i32::<BigEndian>().map_err(ConversionError::Io)?;
                    let bytes = buf.read_n_byte(len as usize);
                    TupleData::Text(bytes)
                }
                TUPLE_DATA_BINARY_BYTE => {
                    let len = buf.read_i32::<BigEndian>().map_err(ConversionError::Io)?;
                    let bytes = buf.read_n_byte(len as usize);
                    TupleData::Text(bytes)
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

#[non_exhaustive]
#[derive(Debug)]
pub enum LogicalReplicationMessage {
    Begin(BeginBody),
    Commit(CommitBody),
    Relation(RelationBody),
    Type(TypeBody),
    Insert(InsertBody),
    Update,
    Delete,
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
    Io(io::Error),
    #[error("Utf8Error conversion: {0}")]
    Utf8(Utf8Error),
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
        match memchr(0, self.slice()) {
            Some(pos) => {
                let start = self.idx;
                let end = start + pos;
                let cstr = str::from_utf8(&self.bytes[start..end])
                    .map_err(ConversionError::Utf8)?
                    .to_owned();
                self.idx = end + 1;
                Ok(cstr)
            }
            None => Err(ConversionError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected EOF",
            ))),
        }
    }

    fn read_n_byte(&mut self, nbyte: usize) -> Bytes {
        let end = self.idx + nbyte;
        let buf = self.bytes.slice(self.idx..end);
        self.idx += nbyte;
        buf
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
        let byte = buf.read_u8().map_err(ConversionError::Io)?;

        let logical_replication_message = match byte {
            BEGIN_BYTE => {
                let last_lsn = buf.read_i64::<BigEndian>().map_err(ConversionError::Io)?;
                let timestamp = buf.read_i64::<BigEndian>().map_err(ConversionError::Io)?;
                let transaction_id = buf.read_i32::<BigEndian>().map_err(ConversionError::Io)?;

                LogicalReplicationMessage::Begin(BeginBody::new(
                    last_lsn,
                    timestamp,
                    transaction_id,
                ))
            }
            COMMIT_BYTE => {
                let flags = buf.read_i8().map_err(ConversionError::Io)?;
                let commit_lsn = buf.read_u64::<BigEndian>().map_err(ConversionError::Io)?;
                let end_lsn = buf.read_u64::<BigEndian>().map_err(ConversionError::Io)?;
                let timestamp = buf.read_i64::<BigEndian>().map_err(ConversionError::Io)?;
                LogicalReplicationMessage::Commit(CommitBody::new(
                    flags, commit_lsn, end_lsn, timestamp,
                ))
            }
            RELATION_BYTE => {
                let transaction_id = match logical_replication_settings.streaming {
                    true => Some(buf.read_i32::<BigEndian>().map_err(ConversionError::Io)?),
                    false => None,
                };

                let relation_id = buf.read_i32::<BigEndian>().map_err(ConversionError::Io)?;
                let namespace = buf.read_cstr()?;
                let name = buf.read_cstr()?;
                let replica_identity = match buf.read_i8().map_err(ConversionError::Io)? {
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

                let num_of_column = buf.read_i16::<BigEndian>().map_err(ConversionError::Io)?;

                let mut columns = Vec::with_capacity(num_of_column as usize);
                for _ in 0..num_of_column {
                    let flags = buf.read_i8().map_err(ConversionError::Io)?;
                    let name = buf.read_cstr()?;
                    let type_id = buf.read_i32::<BigEndian>().map_err(ConversionError::Io)?;
                    let type_modifier = buf.read_i32::<BigEndian>().map_err(ConversionError::Io)?;
                    let column = Column::new(flags, name, type_id, type_modifier);

                    columns.push(column);
                }

                LogicalReplicationMessage::Relation(RelationBody::new(
                    transaction_id,
                    relation_id,
                    namespace,
                    name,
                    replica_identity,
                    columns,
                ))
            }
            TYPE_BYTE => {
                let oid = buf.read_i32::<BigEndian>().map_err(ConversionError::Io)?;
                let namespace = buf.read_cstr()?;
                let name = buf.read_cstr()?;

                LogicalReplicationMessage::Type(TypeBody::new(oid, namespace, name))
            }
            INSERT_BYTE => {
                let transaction_id = match logical_replication_settings.streaming {
                    true => Some(buf.read_i32::<BigEndian>().map_err(ConversionError::Io)?),
                    false => None,
                };
                let o_id = buf.read_i32::<BigEndian>().map_err(ConversionError::Io)?;
                let byte = buf.read_u8().map_err(ConversionError::Io)?;

                let tuple = match byte {
                    TUPLE_NEW_BYTE => TupleData::parse(buf)?,
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
                /*let rel_id = buf.read_u32::<BigEndian>()?;
                let tag = buf.read_u8()?;

                let mut key_tuple = None;
                let mut old_tuple = None;

                let new_tuple = match tag {
                    TUPLE_NEW_BYTE => Tuple::parse(&mut buf)?,
                    TUPLE_OLD_BYTE | TUPLE_KEY_BYTE => {
                        if tag == TUPLE_OLD_BYTE {
                            old_tuple = Some(Tuple::parse(&mut buf)?);
                        } else {
                            key_tuple = Some(Tuple::parse(&mut buf)?);
                        }

                        match buf.read_u8()? {
                            TUPLE_NEW_BYTE => Tuple::parse(&mut buf)?,
                            tag => {
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    format!("unexpected tuple tag `{}`", tag),
                                ));
                            }
                        }
                    }
                    byte => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("unknown tuple tag `{}`", tag),
                        ));
                    }
                };*/

                LogicalReplicationMessage::Update
            }
            DELETE_BYTE => {
                /*let rel_id = buf.read_u32::<BigEndian>()?;
                let tag = buf.read_u8()?;

                let mut key_tuple = None;
                let mut old_tuple = None;

                match tag {
                    TUPLE_OLD_BYTE => old_tuple = Some(Tuple::parse(&mut buf)?),
                    TUPLE_KEY_BYTE => key_tuple = Some(Tuple::parse(&mut buf)?),
                    tag => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("unknown tuple tag `{}`", tag),
                        ));
                    }
                }*/

                LogicalReplicationMessage::Delete
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
