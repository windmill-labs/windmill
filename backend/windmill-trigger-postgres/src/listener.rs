use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};

use bytes::{BufMut, Bytes, BytesMut};
use chrono::TimeZone;
use futures::{pin_mut, SinkExt};
use pg_escape::{quote_identifier, quote_literal};
use rust_postgres::{Client, CopyBothDuplex, SimpleQueryMessage};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use windmill_api_auth::ApiAuthed;
use windmill_common::{
    db::UserDB,
    error::{to_anyhow, Error, Result},
    jobs::JobTriggerKind,
    worker::to_raw_value,
    DB,
};

use windmill_trigger::{
    listener::{update_rw_lock, ListeningTrigger},
    trigger_helpers::TriggerJobArgs,
    Listener,
};

use super::{
    drop_publication, ensure_not_under_roles, get_default_pg_connection,
    get_raw_postgres_connection,
    handler::drop_logical_replication_slot,
    relation::RelationConverter,
    replication_message::{
        LogicalReplicationMessage::{Begin, Commit, Delete, Insert, Relation, Type, Update},
        ReplicationMessage,
    },
    resolve_postgres_resource, Postgres, PostgresConfig, PostgresTrigger,
    ERROR_PUBLICATION_NAME_NOT_EXISTS,
};

const ERROR_REPLICATION_SLOT_NOT_EXISTS: &str = r#"The replication slot associated with this trigger no longer exists. Recreate a new replication slot or select an existing one in the advanced tab, or delete and recreate a new trigger"#;

pub struct LogicalReplicationSettings {
    pub streaming: bool,
}

impl LogicalReplicationSettings {
    pub fn new(streaming: bool) -> Self {
        Self { streaming }
    }
}

pub struct PostgresSimpleClient(Client);

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

impl PostgresSimpleClient {
    async fn new(database: &Postgres) -> Result<Self> {
        let client = get_raw_postgres_connection(database, true).await?;

        Ok(PostgresSimpleClient(client))
    }

    async fn execute_query(
        &self,
        query: &str,
    ) -> std::result::Result<Vec<SimpleQueryMessage>, rust_postgres::Error> {
        self.0.simple_query(query).await
    }

    async fn get_logical_replication_stream(
        &self,
        publication_name: &str,
        logical_replication_slot_name: &str,
    ) -> Result<(CopyBothDuplex<Bytes>, LogicalReplicationSettings)> {
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
                .map_err(to_anyhow)?,
            LogicalReplicationSettings::new(false),
        ))
    }

    /// Sends a Standby Status Update ('r') reporting `lsn` as the WAL position
    /// written, flushed and applied. Postgres only advances the replication
    /// slot's `confirmed_flush_lsn`/`restart_lsn` (and releases retained WAL)
    /// when it receives this with a flushed LSN past the current position.
    async fn send_status_update(lsn: u64, copy_both_stream: &mut Pin<&mut CopyBothDuplex<Bytes>>) {
        let mut buf = BytesMut::new();
        let ts = chrono::Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
        let ts = chrono::Utc::now()
            .signed_duration_since(ts)
            .num_microseconds()
            .unwrap_or(0);

        buf.put_u8(b'r');
        buf.put_u64(lsn);
        buf.put_u64(lsn);
        buf.put_u64(lsn);
        buf.put_i64(ts);
        buf.put_u8(0);
        if let Err(err) = copy_both_stream.send(buf.freeze()).await {
            tracing::warn!("Failed to send standby status update to postgres: {err}");
        } else {
            tracing::debug!("Sent standby status update with lsn {lsn}");
        }
    }
}

/// Resolves the Postgres resource, validates that the configured publication and
/// replication slot still exist, and opens a fresh logical replication stream.
///
/// Returns `Error::BadConfig` when the publication or slot is missing or invalidated, or the
/// data table is under roles (unrecoverable misconfigurations). Whether any other error is
/// retried is up to `is_transient_error`.
/// The resource is re-resolved on every call so credential rotations are picked
/// up across reconnections.
async fn connect_logical_replication_stream(
    authed: &ApiAuthed,
    db: &DB,
    listening_trigger: &ListeningTrigger<PostgresConfig>,
) -> Result<(CopyBothDuplex<Bytes>, LogicalReplicationSettings)> {
    let ListeningTrigger { workspace_id, trigger_config, .. } = listening_trigger;
    let PostgresConfig { postgres_resource_path, publication_name, replication_slot_name, .. } =
        trigger_config;

    // Retrying cannot lift roles, so this disables the trigger like a missing slot does.
    ensure_not_under_roles(db, workspace_id, postgres_resource_path)
        .await
        .map_err(|e| match e {
            Error::BadRequest(msg) => Error::BadConfig(msg),
            e => e,
        })?;

    let database = resolve_postgres_resource(
        authed,
        Some(UserDB::new(db.clone())),
        db,
        postgres_resource_path,
        workspace_id,
    )
    .await?;

    let client = PostgresSimpleClient::new(&database).await?;

    let publication = client
        .execute_query(&format!(
            "SELECT pubname FROM pg_publication WHERE pubname = {}",
            quote_literal(publication_name)
        ))
        .await
        .map_err(to_anyhow)?;

    if !publication.row_exist() {
        return Err(Error::BadConfig(
            ERROR_PUBLICATION_NAME_NOT_EXISTS.to_string(),
        ));
    }

    // `invalidation_reason` (PG 17+) and `conflicting` (PG 16) are read through jsonb so the
    // query still runs on versions that lack those columns.
    let replication_slot = client
        .execute_query(&format!(
            r#"SELECT COALESCE(
                j->>'invalidation_reason',
                CASE WHEN j->>'wal_status' = 'lost' THEN 'wal_removed'
                     WHEN j->>'conflicting' = 'true' THEN 'conflict with recovery' END
            ) FROM pg_replication_slots s, to_jsonb(s) j WHERE slot_name = {}"#,
            quote_literal(replication_slot_name)
        ))
        .await
        .map_err(to_anyhow)?;

    let Some(slot) = replication_slot.iter().find_map(|m| match m {
        SimpleQueryMessage::Row(row) => Some(row),
        _ => None,
    }) else {
        return Err(Error::BadConfig(
            ERROR_REPLICATION_SLOT_NOT_EXISTS.to_string(),
        ));
    };

    if let Some(reason) = slot.get(0) {
        return Err(Error::BadConfig(format!(
            "The replication slot `{replication_slot_name}` associated with this trigger has been invalidated by PostgreSQL (reason: `{reason}`). Changes since the invalidation cannot be replayed. Drop the slot (Advanced tab, slots) and save the trigger again to recreate it, or select another slot."
        )));
    }

    client
        .get_logical_replication_stream(publication_name, replication_slot_name)
        .await
}

#[async_trait::async_trait]
impl Listener for PostgresTrigger {
    type Consumer = (CopyBothDuplex<Bytes>, LogicalReplicationSettings);
    type Extra = ();
    type ExtraState = ();
    const JOB_TRIGGER_KIND: JobTriggerKind = JobTriggerKind::Postgres;

    async fn get_consumer(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        _err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<Option<Self::Consumer>> {
        let authed = listening_trigger
            .authed(db, &Self::TRIGGER_KIND.to_string())
            .await?;
        connect_logical_replication_stream(&authed, db, listening_trigger)
            .await
            .map(Some)
    }
    async fn consume(
        &self,
        db: &DB,
        consumer: Self::Consumer,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
        _extra_state: Option<&Self::ExtraState>,
    ) {
        let (logical_replication_stream, logical_replication_settings) = consumer;
        pin_mut!(logical_replication_stream);
        let mut relations = RelationConverter::new();
        tracing::info!(
            "Starting to listen for postgres trigger {}",
            &listening_trigger.path
        );

        // Highest WAL position received (and processed) so far. Reported back
        // to Postgres via standby status updates so the replication slot can
        // advance and retained WAL gets released. Without periodic updates the
        // slot's LSNs stay frozen and WAL grows unbounded.
        let mut last_lsn: u64 = 0;
        let mut status_interval = tokio::time::interval(Duration::from_secs(10));
        status_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // First tick resolves immediately; consume it so the periodic cadence
        // starts one full interval from now.
        status_interval.tick().await;

        loop {
            let next = tokio::select! {
                _ = status_interval.tick() => None,
                message = logical_replication_stream.next() => Some(message),
            };

            let message = match next {
                None => {
                    PostgresSimpleClient::send_status_update(
                        last_lsn,
                        &mut logical_replication_stream,
                    )
                    .await;
                    continue;
                }
                Some(message) => message,
            };
            let message = match message {
                Some(Ok(message)) => message,
                Some(Err(err)) => {
                    tracing::warn!(
                            "Postgres trigger {} had an error while receiving a message, reconnecting: {}",
                            &listening_trigger.path,
                            err
                        );
                    update_rw_lock(
                        err_message,
                        Some(format!("Error receiving message: {}", err)),
                    )
                    .await;
                    return;
                }
                None => {
                    tracing::warn!(
                        "Stream for postgres trigger {} closed, reconnecting",
                        &listening_trigger.path
                    );
                    update_rw_lock(err_message, Some("Stream closed".to_string())).await;
                    return;
                }
            };

            let logical_message = match ReplicationMessage::parse(message) {
                Ok(logical_message) => logical_message,
                Err(err) => {
                    let err = format!(
                        "Postgres trigger named: {} had an error while parsing message: {}",
                        &listening_trigger.path,
                        err.to_string()
                    );
                    self.disable_with_error(db, listening_trigger, err).await;
                    return;
                }
            };

            match logical_message {
                ReplicationMessage::PrimaryKeepAlive(primary_keep_alive) => {
                    last_lsn = last_lsn.max(primary_keep_alive.wal_end);
                    if primary_keep_alive.reply {
                        PostgresSimpleClient::send_status_update(
                            last_lsn,
                            &mut logical_replication_stream,
                        )
                        .await;
                    }
                }
                ReplicationMessage::XLogData(x_log_data) => {
                    last_lsn = last_lsn.max(x_log_data.wal_end);
                    let logical_replication_message = match x_log_data
                        .parse(&logical_replication_settings)
                    {
                        Ok(logical_replication_message) => logical_replication_message,
                        Err(err) => {
                            tracing::error!("Postgres trigger named: {} had an error while trying to parse incomming stream message: {}", &listening_trigger.path, err.to_string());
                            continue;
                        }
                    };

                    let json = match logical_replication_message {
                        Relation(relation_body) => {
                            relations.add_relation(relation_body);
                            None
                        }
                        Begin | Type | Commit => None,
                        Insert(insert) => Some((
                            insert.o_id,
                            Ok(None),
                            relations.row_to_json((insert.o_id, insert.tuple)),
                            "insert",
                        )),
                        Update(update) => {
                            let old_row = update
                                .old_tuple
                                .map(|old_tuple| relations.row_to_json((update.o_id, old_tuple)))
                                .transpose();
                            let row = relations.row_to_json((update.o_id, update.new_tuple));
                            Some((update.o_id, old_row, row, "update"))
                        }
                        Delete(delete) => {
                            let row = delete
                                .old_tuple
                                .unwrap_or_else(|| delete.key_tuple.unwrap());
                            Some((
                                delete.o_id,
                                Ok(None),
                                relations.row_to_json((delete.o_id, row)),
                                "delete",
                            ))
                        }
                    };
                    match json {
                        Some((o_id, Ok(old_row), Ok(row), transaction_type)) => {
                            let relation = match relations.get_relation(o_id) {
                                Ok(relation) => relation,
                                Err(err) => {
                                    tracing::error!(
                                        "Postgres trigger named: {}, error: {}",
                                        &listening_trigger.path,
                                        err.to_string()
                                    );
                                    continue;
                                }
                            };
                            let database_info = HashMap::from([
                                ("schema_name".to_string(), to_raw_value(&relation.namespace)),
                                ("table_name".to_string(), to_raw_value(&relation.name)),
                                (
                                    "transaction_type".to_string(),
                                    to_raw_value(&transaction_type),
                                ),
                                ("old_row".to_string(), to_raw_value(&old_row)),
                                ("row".to_string(), to_raw_value(&row)),
                            ]);
                            let _ = self
                                .handle_event(
                                    db,
                                    listening_trigger,
                                    database_info,
                                    HashMap::new(),
                                    None,
                                )
                                .await;
                        }
                        Some((o_id, old_row, row, transaction_type)) => {
                            let relation = match relations.get_relation(o_id) {
                                Ok(relation) => relation,
                                Err(err) => {
                                    tracing::error!(
                                        "Postgres trigger named: {}, error: {}",
                                        &listening_trigger.path,
                                        err.to_string()
                                    );
                                    continue;
                                }
                            };

                            if let Err(err) = old_row {
                                tracing::error!(
                                    transaction_type = ?transaction_type,
                                    schema = %relation.namespace,
                                    table = %relation.name,
                                    error = %err,
                                    "Failed to decode OLD row for {} transaction on {}.{}",
                                    transaction_type,
                                    relation.namespace,
                                    relation.name,
                                );
                            }

                            if let Err(err) = row {
                                tracing::error!(
                                    transaction_type = ?transaction_type,
                                    schema = %relation.namespace,
                                    table = %relation.name,
                                    error = %err,
                                    "Failed to decode NEW row for {} transaction on {}.{}",
                                    transaction_type,
                                    relation.namespace,
                                    relation.name,
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn cleanup(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        _extra_state: Option<&Self::ExtraState>,
    ) -> Result<()> {
        let authed = listening_trigger
            .authed(db, &Self::TRIGGER_KIND.to_string())
            .await?;

        let user_db = UserDB::new(db.clone());

        let mut pg_connection = get_default_pg_connection(
            authed,
            Some(user_db),
            &db,
            &listening_trigger.trigger_config.postgres_resource_path,
            &listening_trigger.workspace_id,
        )
        .await?;

        if listening_trigger.trigger_config.basic_mode.unwrap_or(false) {
            drop_logical_replication_slot(
                &mut pg_connection,
                &listening_trigger.trigger_config.replication_slot_name,
            )
            .await?;

            drop_publication(
                &mut pg_connection,
                &listening_trigger.trigger_config.publication_name,
            )
            .await?;
        }

        Ok(())
    }
}
