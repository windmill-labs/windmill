use serde_json::value::RawValue;
use std::pin::Pin;

use crate::{
    database_triggers::{
        relation::RelationConverter,
        replication_message::{
            LogicalReplicationMessage::{Begin, Commit, Delete, Insert, Relation, Type, Update},
            ReplicationMessage,
        },
        run_job,
    },
    db::DB,
};
use bytes::{BufMut, Bytes, BytesMut};
use chrono::TimeZone;
use futures::{pin_mut, SinkExt, StreamExt};
use itertools::Itertools;
use pg_escape::{quote_identifier, quote_literal};
use rand::seq::SliceRandom;
use rust_postgres::{Client, Config, CopyBothDuplex, NoTls, SimpleQueryMessage};
use windmill_common::{resource::get_resource, variables::get_variable_or_self, INSTANCE_NAME};

use super::{
    handler::{Database, DatabaseTrigger, TableToTrack, TransactionType},
    replication_message::PrimaryKeepAliveBody,
    SqlxJson,
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
    #[error("Publication name already exist, choose another anme")]
    PublicationAlreadyExist,
}
pub struct LogicalReplicationSettings {
    pub streaming: bool,
    pub binary: bool,
}

impl LogicalReplicationSettings {
    pub fn new(binary: bool, streaming: bool) -> Self {
        Self { binary, streaming }
    }
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

struct PostgresClient {
    client: Client,
}

impl PostgresClient {
    pub async fn new(database: &Database) -> Result<PostgresClient, Error> {
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

        Ok(PostgresClient { client })
    }

    pub async fn check_if_table_exists(
        &self,
        db_name: &str,
        table_to_track: &[&str],
    ) -> Result<(), Error> {
        if table_to_track.is_empty() {
            return Ok(());
        }
        let table_names = table_to_track
            .iter()
            .map(|table| quote_literal(table))
            .join(",");
        let query = format!(
            r#"
                WITH target_tables AS (
                    SELECT unnest(ARRAY[{}]) AS table_name
                )
                SELECT t.table_name
                FROM 
                    target_tables t
                LEFT JOIN 
                    information_schema.tables ist
                ON 
                    t.table_name = ist.table_name
                    AND ist.table_type = 'BASE TABLE'
                    AND ist.table_catalog = {}
                    AND ist.table_schema NOT IN ('pg_catalog', 'information_schema')
                WHERE 
                    ist.table_name IS NULL;
                "#,
            table_names,
            quote_literal(db_name)
        );
        let rows = self
            .client
            .simple_query(&query)
            .await
            .map_err(Error::Postgres)?;

        if !rows.row_exist() {
            return Ok(());
        }

        Err(Error::MissingTables(
            rows.into_iter()
                .filter_map(|row| {
                    if let SimpleQueryMessage::Row(row) = row {
                        return Some(row.get("table_name").unwrap().to_string());
                    }
                    None
                })
                .collect_vec()
                .join(", "),
        ))
    }

    pub async fn get_logical_replication_stream(
        &self,
        publication_name: &str,
        logical_replication_slot_name: &str,
    ) -> Result<(CopyBothDuplex<Bytes>, LogicalReplicationSettings), Error> {
        let binary_format = true;
        let options = match binary_format {
            true => format!(
                r#"("proto_version" '2', "publication_names" {}, "binary")"#,
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
            self.client
                .copy_both_simple::<bytes::Bytes>(query.as_str())
                .await
                .map_err(Error::Postgres)?,
            LogicalReplicationSettings::new(binary_format, false),
        ))
    }

    pub async fn send_status_update(
        &self,
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

        let query_result = self
            .client
            .simple_query(&query)
            .await
            .map_err(Error::Postgres)?;

        if query_result.row_exist() {
            return Ok(true);
        }

        Ok(false)
    }

    async fn create_slot(&self, slot_name: &str) -> Result<(), Error> {
        let query = format!(
            "SELECT * FROM pg_create_logical_replication_slot({}, 'pgoutput')",
            quote_literal(slot_name)
        );
        self.client
            .simple_query(&query)
            .await
            .map_err(Error::Postgres)?;
        Ok(())
    }

    pub async fn get_or_create_slot(&self, slot_name: &str) -> Result<(), Error> {
        if self.get_slot(slot_name).await? {
            tracing::info!("Slot name {} already exists", slot_name);
            return Ok(());
        }
        tracing::info!("Slot name {} do not exist, trying to create it", slot_name);
        self.create_slot(slot_name).await
    }

    async fn check_if_publication_exists(&self, publication: &str) -> Result<bool, Error> {
        let publication_exists_query = format!(
            "select 1 as exists from pg_publication where pubname = {};",
            quote_literal(publication)
        );
        let rows = self
            .client
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

    async fn create_publication(
        &self,
        publication_name: &str,
        tables: Option<Vec<&str>>,
        transaction_to_track: Option<&[TransactionType]>,
    ) -> Result<(), Error> {
        let mut query = String::new();
        let quoted_publication_name = quote_identifier(publication_name);
        query.push_str("CREATE PUBLICATION ");
        query.push_str(&quoted_publication_name);

        match tables {
            Some(table_to_track) if !table_to_track.is_empty() => {
                query.push_str(" FOR TABLE ONLY ");
                for (i, table) in table_to_track.iter().enumerate() {
                    let quoted_table = quote_identifier(table);
                    query.push_str(&quoted_table);

                    if i < table_to_track.len() - 1 {
                        query.push(',')
                    }
                }
            }
            _ => query.push_str(" FOR ALL TABLES "),
        };

        if let Some(transaction_to_track) = transaction_to_track {
            if !transaction_to_track.is_empty() {
                let transactions = || {
                    transaction_to_track
                        .iter()
                        .map(|transaction| match transaction {
                            TransactionType::Insert => "insert",
                            TransactionType::Update => "update",
                            TransactionType::Delete => "delete",
                        })
                        .join(",")
                };
                let with_parameter = format!(" WITH (publish = '{}'); ", transactions());
                query.push_str(&with_parameter);
            }
        }

        println!("{}", &query);

        self.client
            .simple_query(&query)
            .await
            .map_err(Error::Postgres)?;

        Ok(())
    }

    pub async fn create_publication_if_not_exist(
        &self,
        publication_name: &str,
        tables: Option<Vec<&str>>,
        transaction_to_track: Option<&[TransactionType]>,
    ) -> Result<(), Error> {
        if self.check_if_publication_exists(publication_name).await? {
            tracing::info!("Publication {} already exists", publication_name);
            return Ok(());
        }
        tracing::info!("Publication {} do no exist", publication_name);
        self.create_publication(publication_name, tables, transaction_to_track)
            .await
    }
}

async fn update_ping(
    db: &DB,
    database_trigger: &DatabaseTrigger,
    error: Option<&str>,
) -> Option<()> {
    match sqlx::query_scalar!(
        r#"
        UPDATE 
            database_trigger
        SET 
            last_server_ping = now(),
            error = $1
        WHERE
            workspace_id = $2
            AND path = $3
            AND server_id = $4 
            AND enabled IS TRUE
        RETURNING 1
        "#,
        error,
        database_trigger.workspace_id,
        database_trigger.path,
        *INSTANCE_NAME
    )
    .fetch_optional(db)
    .await
    {
        Ok(updated) => {
            if updated.flatten().is_none() {
                tracing::info!(
                    "Database {} changed, disabled, or deleted, stopping...",
                    database_trigger.path
                );
                return None;
            }
        }
        Err(err) => {
            tracing::warn!(
                "Error updating ping of database {}: {:?}",
                database_trigger.path,
                err
            );
        }
    };

    Some(())
}

async fn loop_ping(db: &DB, database_trigger: &DatabaseTrigger, error: Option<&str>) {
    loop {
        if update_ping(db, database_trigger, error).await.is_none() {
            return;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn listen_to_transactions(
    database_trigger: &DatabaseTrigger,
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Error> {
    let mut resource = get_resource::<Database>(
        &db,
        &database_trigger.database_resource_path,
        &database_trigger.workspace_id,
    )
    .await
    .map_err(Error::Sqlx)?;

    if resource.value.password.is_some() {
        let password = get_variable_or_self(
            resource.value.password.unwrap(),
            &db,
            &database_trigger.workspace_id,
        )
        .await
        .map_err(Error::CommonError)?;
        resource.value.password = Some(password)
    }

    let client = PostgresClient::new(&resource.value).await?;

    let table_to_track = if let Some(table_to_track) = &database_trigger.table_to_track {
        let table_to_track = table_to_track
            .iter()
            .map(|table| table.table_name.as_str())
            .collect_vec();
        client
            .check_if_table_exists(&resource.value.db_name, table_to_track.as_slice())
            .await?;
        Some(table_to_track)
    } else {
        None
    };

    tracing::info!("Starting tokio select futures");

    client
        .get_or_create_slot(&database_trigger.replication_slot_name)
        .await?;

    client
        .create_publication_if_not_exist(
            &database_trigger.publication_name,
            table_to_track,
            database_trigger.transaction_to_track.as_deref(),
        )
        .await?;

    let (logical_replication_stream, logicail_replication_settings) = client
        .get_logical_replication_stream(
            &database_trigger.publication_name,
            &database_trigger.replication_slot_name,
        )
        .await?;

    pin_mut!(logical_replication_stream);

    tokio::select! {
        biased;
        _ = killpill_rx.recv() => {
            Ok(())
        }
        _ = loop_ping(&db, database_trigger, None) => {
            Ok(())
        }
        _ = async {
                let mut relations = RelationConverter::new();
                tracing::info!("Start to listen for database transaction");
                loop {
                    let message = logical_replication_stream.next().await;

                    if message.is_none() {
                        tracing::info!("Stream is empty leaving....");
                        return;
                    }

                    let message = message.unwrap();

                    if let Err(err) = &message {
                        tracing::debug!("{}", err.to_string());
                        update_ping(&db, database_trigger, Some(&err.to_string())).await;
                        return;
                    }

                    let message = message.unwrap();

                    let logical_message = match ReplicationMessage::parse(message) {
                        Ok(logical_message) => logical_message,
                        Err(err) => {
                            tracing::debug!("{}", err.to_string());
                            update_ping(&db, database_trigger, Some(&err.to_string())).await;
                            return;
                        }
                    };

                    match logical_message {
                        ReplicationMessage::PrimaryKeepAlive(primary_keep_alive) => {
                            if primary_keep_alive.reply {
                                client.send_status_update(primary_keep_alive, &mut logical_replication_stream).await;
                            }
                        }
                        ReplicationMessage::XLogData(x_log_data) => {
                            let logical_replication_message = match x_log_data.parse(&logicail_replication_settings) {
                                Ok(logical_replication_message) => logical_replication_message,
                                Err(err) => {
                                    tracing::debug!("{}", err.to_string());
                                    update_ping(&db, database_trigger, Some(&err.to_string())).await;
                                    return;
                                }
                            };
                            println!("{:#?}", logical_replication_message);
                            let json = match logical_replication_message {
                                Relation(relation_body) => {
                                    relations.add_column(relation_body.o_id, relation_body.columns);
                                    None
                                }
                                Begin(_) | Commit(_) | Type(_) => {
                                    //println!("{:#?}", begin_body);
                                    None
                                }
                                Insert(insert) => {
                                    Some(relations.body_to_json((insert.o_id, insert.tuple)))
                                }
                                Update(update) => {
                                    Some(relations.body_to_json((update.o_id, update.new_tuple)))

                                }
                                Delete(delete) => {
                                    None
                                }
                            };

                            if let Some(Ok(json)) = json {

                                let _ = run_job(Some(json), &db, rsmq.clone(), database_trigger).await;
                                continue;
                            }

                        }
                    }
                }
        } => {
            Ok(())
        }
    }
}

async fn try_to_listen_to_database_transactions(
    db_trigger: DatabaseTrigger,
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let database_trigger = sqlx::query_scalar!(
        r#"
        UPDATE database_trigger 
        SET 
            server_id = $1, 
            last_server_ping = now() 
        WHERE 
            enabled IS TRUE 
            AND workspace_id = $2 
            AND path = $3 
            AND (
                server_id IS NULL 
                OR last_server_ping IS NULL 
                OR last_server_ping < now() - INTERVAL '15 seconds'
            ) 
        RETURNING true
        "#,
        *INSTANCE_NAME,
        db_trigger.workspace_id,
        db_trigger.path,
    )
    .fetch_optional(&db)
    .await;
    match database_trigger {
        Ok(has_lock) => {
            if has_lock.flatten().unwrap_or(false) {
                tracing::info!("Spawning new task to listen_to_database_transaction");
                tokio::spawn(async move {
                    let result =
                        listen_to_transactions(&db_trigger, db.clone(), rsmq, killpill_rx).await;
                    if let Err(e) = result {
                        update_ping(&db, &db_trigger, Some(e.to_string().as_str())).await;
                    };
                });
            } else {
                tracing::info!("Database {} already being listened to", db_trigger.path);
            }
        }
        Err(err) => {
            tracing::error!(
                "Error acquiring lock for database {}: {:?}",
                db_trigger.path,
                err
            );
        }
    };
}

async fn listen_to_unlistened_database_events(
    db: &DB,
    rsmq: &Option<rsmq_async::MultiplexedRsmq>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) {
    let database_triggers = sqlx::query_as!(
        DatabaseTrigger,
        r#"
            SELECT
                workspace_id,
                transaction_to_track AS "transaction_to_track: Vec<TransactionType>",
                path,
                script_path,
                replication_slot_name,
                publication_name,
                is_flow,
                edited_by,
                email,
                edited_at,
                server_id,
                last_server_ping,
                extra_perms,
                error,
                enabled,
                database_resource_path,
                table_to_track AS "table_to_track: SqlxJson<Vec<TableToTrack>>"
            FROM
                database_trigger
            WHERE
                enabled IS TRUE
                AND (
                    server_id IS NULL OR
                    last_server_ping IS NULL OR
                    last_server_ping < now() - interval '15 seconds'
                )
            "#
    )
    .fetch_all(db)
    .await;

    match database_triggers {
        Ok(mut triggers) => {
            triggers.shuffle(&mut rand::thread_rng());
            for trigger in triggers {
                try_to_listen_to_database_transactions(
                    trigger,
                    db.clone(),
                    rsmq.clone(),
                    killpill_rx.resubscribe(),
                )
                .await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching database triggers: {:?}", err);
        }
    };
}

pub async fn start_database(
    db: DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        listen_to_unlistened_database_events(&db, &rsmq, &killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_database_events(&db, &rsmq, &killpill_rx).await
                }
            }
        }
    });
}
