use std::{collections::HashMap, pin::Pin};

use crate::{
    db::DB,
    postgres_triggers::{
        get_database_resource,
        relation::RelationConverter,
        replication_message::{
            LogicalReplicationMessage::{Begin, Commit, Delete, Insert, Relation, Type, Update},
            ReplicationMessage,
        },
        run_job,
    },
    users::fetch_api_authed,
};
use bytes::{BufMut, Bytes, BytesMut};
use chrono::TimeZone;
use futures::{pin_mut, SinkExt, StreamExt};
use pg_escape::{quote_identifier, quote_literal};
use rand::seq::SliceRandom;
use rust_postgres::{Client, Config, CopyBothDuplex, NoTls, SimpleQueryMessage};
use windmill_common::{db::UserDB, worker::to_raw_value, INSTANCE_NAME};

use super::{
    handler::{Database, PostgresTrigger},
    replication_message::PrimaryKeepAliveBody,
};

pub struct LogicalReplicationSettings {
    pub streaming: bool,
}

impl LogicalReplicationSettings {
    pub fn new(streaming: bool) -> Self {
        Self { streaming }
    }
}

#[allow(unused)]
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

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Error from database: {0}")]
    Postgres(rust_postgres::Error),
    #[error("Error : {0}")]
    Common(windmill_common::error::Error),
}

pub struct PostgresSimpleClient(Client);

impl PostgresSimpleClient {
    async fn new(database: &Database) -> Result<Self, Error> {
        let mut config = Config::new();
        config
            .dbname(&database.dbname)
            .host(&database.host)
            .port(database.port)
            .user(&database.user)
            .replication_mode(rust_postgres::config::ReplicationMode::Logical);

        if !database.password.is_empty() {
            config.password(&database.password);
        }

        let (client, connection) = config.connect(NoTls).await.map_err(Error::Postgres)?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::debug!("{:#?}", e);
            };
            tracing::info!("Successfully Connected into database");
        });

        Ok(PostgresSimpleClient(client))
    }

    async fn get_logical_replication_stream(
        &self,
        publication_name: &str,
        logical_replication_slot_name: &str,
    ) -> Result<(CopyBothDuplex<Bytes>, LogicalReplicationSettings), Error> {
        let options = format!(
            r#"("proto_version" '2', "publication_names" {})"#,
            quote_literal(publication_name),
        );

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
            LogicalReplicationSettings::new(false),
        ))
    }

    async fn send_status_update(
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
        tracing::debug!("Send update status message");
    }
}

async fn update_ping(
    db: &DB,
    postgres_trigger: &PostgresTrigger,
    error: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let updated = sqlx::query_scalar!(
        r#"
        UPDATE 
            postgres_trigger
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
        &postgres_trigger.workspace_id,
        &postgres_trigger.path,
        *INSTANCE_NAME
    )
    .fetch_optional(db)
    .await;

    match updated {
        Ok(updated) => {
            if updated.flatten().is_none() {
                // allow faster restart of database trigger
                sqlx::query!(
                    r#"
                UPDATE 
                    postgres_trigger 
                SET
                    last_server_ping = NULL 
                WHERE 
                    workspace_id = $1 
                    AND path = $2 
                    AND server_id IS NULL"#,
                    &postgres_trigger.workspace_id,
                    &postgres_trigger.path,
                )
                .execute(db)
                .await
                .ok();
                tracing::info!(
                    "Postgres trigger {} changed, disabled, or deleted, stopping...",
                    postgres_trigger.path
                );
                return Ok(false);
            }
        }
        Err(err) => return Err(err),
    };

    Ok(true)
}

async fn loop_ping(db: &DB, postgres_trigger: &PostgresTrigger, error: Option<&str>) {
    loop {
        let result = update_ping(db, postgres_trigger, error).await;

        match result {
            Ok(false) => return,
            Err(err) => {
                tracing::warn!(
                    "Error updating ping of postgres trigger {}: {:?}",
                    postgres_trigger.path,
                    err
                );
                return;
            }
            _ => {}
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn listen_to_transactions(
    postgres_trigger: &PostgresTrigger,
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Error> {
    let authed = fetch_api_authed(
        postgres_trigger.edited_by.clone(),
        postgres_trigger.email.clone(),
        &postgres_trigger.workspace_id,
        &db,
        None,
    )
    .await
    .map_err(Error::Common)?;

    let database = get_database_resource(
        authed,
        Some(UserDB::new(db.clone())),
        &db,
        &postgres_trigger.postgres_resource_path,
        &postgres_trigger.workspace_id,
    )
    .await
    .map_err(Error::Common)?;

    let client = PostgresSimpleClient::new(&database).await?;

    let (logical_replication_stream, logicail_replication_settings) = client
        .get_logical_replication_stream(
            &postgres_trigger.publication_name,
            &postgres_trigger.replication_slot_name,
        )
        .await?;

    pin_mut!(logical_replication_stream);

    tokio::select! {
        biased;
        _ = killpill_rx.recv() => {
            Ok(())
        }
        _ = loop_ping(&db, postgres_trigger, None) => {
            Ok(())
        }
        _ = async {
                let mut relations = RelationConverter::new();
                tracing::info!("Starting to listen for postgres trigger {}", postgres_trigger.path);
                loop {
                    let message = logical_replication_stream.next().await;

                    if message.is_none() {
                        tracing::info!("Stream for postgres trigger {} is empty, leaving....", postgres_trigger.path);
                        return;
                    }

                    let message = message.unwrap();

                    if let Err(err) = &message {
                        tracing::debug!("{}: {}", &postgres_trigger.path, err.to_string());
                        return;
                    }

                    let message = message.unwrap();

                    let logical_message = match ReplicationMessage::parse(message) {
                        Ok(logical_message) => logical_message,
                        Err(err) => {
                            tracing::debug!("{}", err.to_string());
                            let _ = update_ping(&db, postgres_trigger, Some(&err.to_string())).await;
                            return;
                        }
                    };


                    match logical_message {
                        ReplicationMessage::PrimaryKeepAlive(primary_keep_alive) => {
                            if primary_keep_alive.reply {
                                PostgresSimpleClient::send_status_update(primary_keep_alive, &mut logical_replication_stream).await;
                            }
                        }
                        ReplicationMessage::XLogData(x_log_data) => {
                            let logical_replication_message = match x_log_data.parse(&logicail_replication_settings) {
                                Ok(logical_replication_message) => logical_replication_message,
                                Err(err) => {
                                    let _ = update_ping(&db, postgres_trigger, Some(&err.to_string())).await;
                                    return;
                                }
                            };

                            let json = match logical_replication_message {
                                Relation(relation_body) => {
                                    relations.add_relation(relation_body);
                                    None
                                }
                                Begin | Type | Commit => {
                                    None
                                }
                                Insert(insert) => {
                                    Some((insert.o_id, relations.body_to_json((insert.o_id, insert.tuple)), "insert"))
                                }
                                Update(update) => {
                                    Some((update.o_id, relations.body_to_json((update.o_id, update.new_tuple)), "update"))
                                }
                                Delete(delete) => {
                                    let body = delete.old_tuple.unwrap_or(delete.key_tuple.unwrap());
                                    Some((delete.o_id, relations.body_to_json((delete.o_id, body)), "delete"))
                                }
                            };
                            if let Some((o_id, Ok(body), transaction_type)) = json {
                                let relation = match relations.get_relation(o_id) {
                                    Ok(relation) => relation,
                                    Err(err) => {
                                        let _ = update_ping(&db, postgres_trigger, Some(&err.to_string())).await;
                                        return;
                                    }
                                };
                                let database_info = HashMap::from([
                                    ("schema_name".to_string(), to_raw_value(&relation.namespace)),
                                    ("table_name".to_string(), to_raw_value(&relation.name)),
                                    ("transaction_type".to_string(), to_raw_value(&transaction_type)),
                                    ("row".to_string(), to_raw_value(&body)),
                                ]);
                                let extra = Some(HashMap::from([(
                                    "wm_trigger".to_string(),
                                    to_raw_value(&serde_json::json!({"kind": "postgres", })),
                                )]));
                                let _ = run_job(Some(database_info), extra, &db, postgres_trigger).await;
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
    pg_trigger: PostgresTrigger,
    db: DB,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let postgres_trigger = sqlx::query_scalar!(
        r#"
        UPDATE postgres_trigger 
        SET 
            server_id = $1, 
            last_server_ping = now(),
            error = 'Connecting...'
        WHERE 
            enabled IS TRUE 
            AND workspace_id = $2 
            AND path = $3 
            AND (last_server_ping IS NULL 
                OR last_server_ping < now() - INTERVAL '15 seconds'
            ) 
        RETURNING true
        "#,
        *INSTANCE_NAME,
        pg_trigger.workspace_id,
        pg_trigger.path,
    )
    .fetch_optional(&db)
    .await;
    match postgres_trigger {
        Ok(has_lock) => {
            if has_lock.flatten().unwrap_or(false) {
                tracing::info!("Spawning new task to listen_to_database_transaction");
                tokio::spawn(async move {
                    let result = listen_to_transactions(&pg_trigger, db.clone(), killpill_rx).await;
                    if let Err(e) = result {
                        let _ = update_ping(&db, &pg_trigger, Some(e.to_string().as_str())).await;
                    };
                });
            } else {
                tracing::info!(
                    "Postgres trigger {} already being listened to",
                    pg_trigger.path
                );
            }
        }
        Err(err) => {
            tracing::error!(
                "Error acquiring lock for postgres trigger {}: {:?}",
                pg_trigger.path,
                err
            );
        }
    };
}

async fn listen_to_unlistened_database_events(
    db: &DB,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) {
    let postgres_triggers = sqlx::query_as!(
        PostgresTrigger,
        r#"
            SELECT
                workspace_id,
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
                postgres_resource_path
            FROM
                postgres_trigger
            WHERE
                enabled IS TRUE
                AND (last_server_ping IS NULL OR
                    last_server_ping < now() - interval '15 seconds'
                )
            "#
    )
    .fetch_all(db)
    .await;

    match postgres_triggers {
        Ok(mut triggers) => {
            triggers.shuffle(&mut rand::thread_rng());
            for trigger in triggers {
                try_to_listen_to_database_transactions(
                    trigger,
                    db.clone(),
                    killpill_rx.resubscribe(),
                )
                .await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching postgres triggers: {:?}", err);
        }
    };
}

pub async fn start_database(db: DB, mut killpill_rx: tokio::sync::broadcast::Receiver<()>) {
    tokio::spawn(async move {
        listen_to_unlistened_database_events(&db, &killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_database_events(&db,  &killpill_rx).await
                }
            }
        }
    });
}
