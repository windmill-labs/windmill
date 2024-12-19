use std::collections::HashMap;

use crate::{
    database_triggers::{
        client::{Error, PostgresSimpleClient},
        relation::RelationConverter,
        replication_message::{
            LogicalReplicationMessage::{Begin, Commit, Delete, Insert, Relation, Type, Update},
            ReplicationMessage,
        },
        run_job,
    },
    db::DB,
};
use futures::{pin_mut, StreamExt};
use rand::seq::SliceRandom;
use windmill_common::{
    resource::get_resource, variables::get_variable_or_self, worker::to_raw_value, INSTANCE_NAME,
};

use super::handler::{Database, DatabaseTrigger};

pub struct LogicalReplicationSettings {
    pub streaming: bool,
    pub binary: bool,
}

impl LogicalReplicationSettings {
    pub fn new(binary: bool, streaming: bool) -> Self {
        Self { binary, streaming }
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
            return None;
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

    let client = PostgresSimpleClient::new(&resource.value).await?;

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
                                PostgresSimpleClient::send_status_update(primary_keep_alive, &mut logical_replication_stream).await;
                            }
                        }
                        ReplicationMessage::XLogData(x_log_data) => {
                            let logical_replication_message = match x_log_data.parse(&logicail_replication_settings) {
                                Ok(logical_replication_message) => logical_replication_message,
                                Err(err) => {
                                    update_ping(&db, database_trigger, Some(&err.to_string())).await;
                                    return;
                                }
                            };

                            let json = match logical_replication_message {
                                Relation(relation_body) => {
                                    relations.add_relation(relation_body);
                                    None
                                }
                                Begin(_) | Type(_) => {
                                    None
                                }
                                Commit(commit) => {
                                    match sqlx::query_scalar!(
                                        r#"
                                        UPDATE 
                                            database_trigger
                                        SET 
                                            last_lsn = $1
                                        WHERE
                                            workspace_id = $2
                                            AND path = $3
                                            AND server_id = $4 
                                            AND enabled IS TRUE
                                        RETURNING 1
                                        "#,
                                        commit.end_lsn as i64,
                                        &database_trigger.workspace_id,
                                        &database_trigger.path,
                                        *INSTANCE_NAME
                                    )
                                    .fetch_optional(&db)
                                    .await
                                    {
                                        Ok(updated) => {
                                            if updated.flatten().is_none() {
                                                return;
                                            }
                                        }
                                        Err(err) => {
                                            tracing::warn!(
                                                "Error updating ping of database {}: {:?}",
                                                database_trigger.path,
                                                err
                                            );
                                            return;
                                        }
                                    };
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
                            if let Some((o_id, Ok(mut body), transaction_type)) = json {
                                let relation = match relations.get_relation(o_id) {
                                    Ok(relation) => relation,
                                    Err(err) => {
                                        update_ping(&db, database_trigger, Some(&err.to_string())).await;
                                        return;
                                    }
                                };
                                let database_info = HashMap::from([("schema_name".to_string(), relation.namespace.as_str()), ("table_name".to_string(), relation.name.as_str()), ("transaction_type".to_string(), transaction_type)]);
                                let extra = Some(HashMap::from([(
                                    "wm_trigger".to_string(),
                                    to_raw_value(&serde_json::json!({"kind": "database", })),
                                )]));
                                body.insert("trigger_info".to_string(), to_raw_value(&database_info));
                                let body = HashMap::from([("data".to_string(), to_raw_value(&serde_json::json!(body)))]);
                                let _ = run_job(Some(body), extra, &db, rsmq.clone(), database_trigger).await;
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
                database_resource_path
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
