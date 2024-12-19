use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
};

use bytes::{BufMut, Bytes, BytesMut};
use chrono::TimeZone;
use futures::SinkExt;
use pg_escape::{quote_identifier, quote_literal};
use rust_postgres::{Client, Config, CopyBothDuplex, NoTls, SimpleQueryMessage};
use sqlx::{postgres::PgConnectOptions, Connection, PgConnection};

use super::{
    handler::Database, replication_message::PrimaryKeepAliveBody,
    trigger::LogicalReplicationSettings,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error from database: {0}")]
    Sqlx(sqlx::Error),
    #[error("The following tables do not exist in your database: {0}")]
    MissingTables(String),
    #[error("Error from database: {0}")]
    Postgres(rust_postgres::Error),
    #[error("{0}")]
    CommonError(windmill_common::error::Error),
    #[error("Slot name already exist, choose another name")]
    SlotAlreadyExist,
    #[error("Publication name already exist, choose another name")]
    PublicationAlreadyExist,
}

trait RowExist {
    fn row_exist(&self) -> bool;
}

impl RowExist for Vec<SimpleQueryMessage> {
    fn row_exist(&self) -> bool {
        self.iter()
            .find_map(|element| {
                if let SimpleQueryMessage::CommandComplete(value) = element {
                    Some(*value)
                } else {
                    None
                }
            })
            .is_some_and(|value| value > 0)
    }
}

pub struct PostgresSimpleClient(Client);

impl PostgresSimpleClient {
    pub async fn new(database: &Database) -> Result<Self, Error> {
        let mut config = Config::new();

        config
            .dbname(&database.db_name)
            .host(&database.host)
            .port(database.port)
            .user(&database.username)
            .replication_mode(rust_postgres::config::ReplicationMode::Logical);

        if let Some(password) = &database.password {
            config.password(password);
        }

        let (client, connection) = config.connect(NoTls).await.map_err(Error::Postgres)?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                println!("{:#?}", e);
            };
            tracing::info!("Successfully Connected into database");
        });

        Ok(PostgresSimpleClient(client))
    }

    pub async fn get_logical_replication_stream(
        &self,
        publication_name: &str,
        logical_replication_slot_name: &str,
    ) -> Result<(CopyBothDuplex<Bytes>, LogicalReplicationSettings), Error> {
        let binary_format = true;
        let options = match binary_format {
            true => format!(
                r#"("proto_version" '2', "publication_names" {})"#,
                //r#"("proto_version" '2', "publication_names" {}, "binary")"#,
                quote_literal(publication_name),
            ),
            false => format!(
                r#"("proto_version" '2', "publication_names" {})"#,
                quote_literal(publication_name),
            ),
        };

        let query = format!(
            r#"START_REPLICATION SLOT {} LOGICAL 0/0 {}"#,
            quote_identifier(logical_replication_slot_name),
            options
        );

        Ok((
            self.0
                .copy_both_simple::<bytes::Bytes>(query.as_str())
                .await
                .map_err(Error::Postgres)?,
            LogicalReplicationSettings::new(binary_format, false),
        ))
    }

    pub async fn send_status_update(
        primary_keep_alive: PrimaryKeepAliveBody,
        copy_both_stream: &mut Pin<&mut CopyBothDuplex<Bytes>>,
    ) {
        let mut buf = BytesMut::new();
        let ts = chrono::Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
        let ts = chrono::Utc::now()
            .signed_duration_since(ts)
            .num_microseconds()
            .unwrap_or(0);

        buf.put_u8(b'r');
        buf.put_u64(primary_keep_alive.wal_end);
        buf.put_u64(primary_keep_alive.wal_end);
        buf.put_u64(primary_keep_alive.wal_end);
        buf.put_i64(ts);
        buf.put_u8(0);
        copy_both_stream.send(buf.freeze()).await.unwrap();
        tracing::info!("Send update status message");
    }

    async fn get_slot(&self, slot_name: &str) -> Result<bool, Error> {
        let query = format!(
            r#"select 1 from pg_replication_slots where slot_name = {};"#,
            quote_literal(slot_name)
        );

        let query_result = self.0.simple_query(&query).await.map_err(Error::Postgres)?;

        if query_result.row_exist() {
            return Ok(true);
        }

        Ok(false)
    }

    async fn check_if_publication_exists(&self, publication: &str) -> Result<bool, Error> {
        let publication_exists_query = format!(
            "select 1 as exists from pg_publication where pubname = {};",
            quote_literal(publication)
        );
        let rows = self
            .0
            .simple_query(&publication_exists_query)
            .await
            .map_err(Error::Postgres)?;
        for msg in rows {
            if let SimpleQueryMessage::Row(_) = msg {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

pub struct PostgresExtendedClient(PgConnection);

impl PostgresExtendedClient {
    pub async fn new(db: &Database) -> Result<Self, Error> {
        let options = {
            let options = PgConnectOptions::new()
                .host(&db.host)
                .database(&db.db_name)
                .port(db.port)
                .username(&db.username);

            if let Some(password) = &db.password {
                options.password(password)
            } else {
                options
            }
        };

        let connection = PgConnection::connect_with(&options)
            .await
            .map_err(Error::Sqlx)?;

        Ok(PostgresExtendedClient(connection))
    }
}

impl Deref for PostgresExtendedClient {
    type Target = PgConnection;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PostgresExtendedClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
