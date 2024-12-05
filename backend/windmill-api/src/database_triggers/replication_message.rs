use std::io;

use byteorder::{BigEndian, ReadBytesExt};
use bytes::Bytes;

const PRIMARY_KEEPALIVE_BYTE: u8 = b'k';
const X_LOG_DATA_BYTE: u8 = b'w';

#[derive(Debug)]
pub struct PrimaryKeepAliveBody {
    wal_end: u64,
    timestamp: i64,
    reply: bool,
}

impl PrimaryKeepAliveBody {
    pub fn wal_end(&self) -> u64 {
        self.wal_end
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn reply(&self) -> bool {
        self.reply
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

const REPLICA_IDENTITY_DEFAULT_BYTE: u8 = b'd';
const REPLICA_IDENTITY_NOTHING_BYTE: u8 = b'n';
const REPLICA_IDENTITY_FULL_BYTE: u8 = b'f';
const REPLICA_IDENTITY_INDEX_BYTE: u8 = b'i';

#[derive(Debug)]
pub enum TupleData {
    Null,
    UnchangedToast,
    Text(Bytes),
    Binary(Bytes),
}

impl TupleData {
    fn parse(buf: Bytes) -> io::Result<Self> {
        let (byte, mut message) = buf.split_first().unwrap();

        let tuple = match *byte {
            TUPLE_DATA_NULL_BYTE => TupleData::Null,
            TUPLE_DATA_TOAST_BYTE => TupleData::UnchangedToast,
            TUPLE_DATA_TEXT_BYTE => {
                let len = message.read_i32::<BigEndian>()?;
                let mut data = vec![0; len as usize];
                TupleData::Text(data.into())
            }
            TUPLE_DATA_BINARY_BYTE => {
                let len = message.read_i32::<BigEndian>()?;
                let mut data = vec![0; len as usize];
                TupleData::Binary(data.into())
            }
            byte => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown replication message tag `{}`", byte),
                ));
            }
        };

        Ok(tuple)
    }
}

#[derive(Debug)]
pub struct Tuple(Vec<TupleData>);

impl Tuple {
    pub fn tuple_data(&self) -> &[TupleData] {
        &self.0
    }
}

#[derive(Debug)]
pub struct InsertBody {
    pub rel_id: u32,
    pub tuple: Tuple,
}

#[derive(Debug)]
pub struct CommitBody {
    pub flags: i8,
    pub commit_lsn: u64,
    pub end_lsn: u64,
    pub timestamp: i64,
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
    pub name: Bytes,
    pub type_id: i32,
    pub type_modifier: i32,
}

#[derive(Debug)]
pub struct RelationBody {
    pub rel_id: u32,
    pub namespace: Bytes,
    pub name: Bytes,
    pub replica_identity: ReplicaIdentity,
    pub columns: Vec<Column>,
}

#[derive(Debug)]
pub struct TypeBody {
    id: u32,
    namespace: Bytes,
    name: Bytes,
}

#[derive(Debug)]
pub struct UpdateBody {
    rel_id: u32,
    old_tuple: Option<Tuple>,
    key_tuple: Option<Tuple>,
    new_tuple: Tuple,
}

#[derive(Debug)]
pub struct DeleteBody {
    rel_id: u32,
    old_tuple: Option<Tuple>,
    key_tuple: Option<Tuple>,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum LogicalReplicationMessage {
    Begin,
    Commit(CommitBody),
    Relation,
    Type,
    Insert,
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

impl XLogDataBody {
    pub fn new(wal_start: u64, wal_end: u64, timestamp: i64, data: Bytes) -> XLogDataBody {
        XLogDataBody { wal_start, wal_end, timestamp, data }
    }

    pub fn parse(self) -> io::Result<LogicalReplicationMessage> {
        let bytes = self.data;
        let (byte, mut buf) = bytes.split_first().unwrap();
        let logical_replication_message = match *byte {
            BEGIN_BYTE => LogicalReplicationMessage::Begin,
            COMMIT_BYTE => LogicalReplicationMessage::Commit(CommitBody {
                flags: buf.read_i8()?,
                commit_lsn: buf.read_u64::<BigEndian>()?,
                end_lsn: buf.read_u64::<BigEndian>()?,
                timestamp: buf.read_i64::<BigEndian>()?,
            }),
            RELATION_BYTE => {
                /*let rel_id = buf.read_u32::<BigEndian>()?;
                let namespace = buf.read_cstr()?;
                let name = buf.read_cstr()?;
                let replica_identity = match buf.read_u8()? {
                    REPLICA_IDENTITY_DEFAULT_BYTE => ReplicaIdentity::Default,
                    REPLICA_IDENTITY_NOTHING_BYTE => ReplicaIdentity::Nothing,
                    REPLICA_IDENTITY_FULL_BYTE => ReplicaIdentity::Full,
                    REPLICA_IDENTITY_INDEX_BYTE => ReplicaIdentity::Index,
                    tag => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("unknown replica identity tag `{}`", tag),
                        ));
                    }
                };
                let column_len = buf.read_i16::<BigEndian>()?;

                let mut columns = Vec::with_capacity(column_len as usize);
                for _ in 0..column_len {
                    columns.push(Column::parse(&mut buf)?);
                }*/

                LogicalReplicationMessage::Relation
            }
            TYPE_BYTE => LogicalReplicationMessage::Type,
            INSERT_BYTE => {
                /*let rel_id = buf.read_u32::<BigEndian>()?;
                let tag = buf.read_u8()?;

                let tuple = match tag {
                    TUPLE_NEW_BYTE => Tuple::parse(&mut buf)?,
                    tag => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("unexpected tuple tag `{}`", tag),
                        ));
                    }
                };*/

                LogicalReplicationMessage::Insert
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
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown replication message tag `{}`", byte),
                ));
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
                ReplicationMessage::XLogData(XLogDataBody { wal_start, wal_end, timestamp, data })
            }
            PRIMARY_KEEPALIVE_BYTE => {
                let wal_end = message.read_u64::<BigEndian>()?;
                let timestamp = message.read_i64::<BigEndian>()?;
                let reply = message.read_u8()?;
                ReplicationMessage::PrimaryKeepAlive(PrimaryKeepAliveBody {
                    wal_end,
                    timestamp,
                    reply: reply == 1,
                })
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
