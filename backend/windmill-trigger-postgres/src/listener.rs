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
    utils::{report_critical_error, report_recovered_critical_error},
    worker::to_raw_value,
    DB,
};

use windmill_trigger::{listener::ListeningTrigger, trigger_helpers::TriggerJobArgs, Listener};

use super::{
    drop_publication, get_default_pg_connection, get_raw_postgres_connection,
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

// Wait this long between reconnection attempts after a connection failure or a
// dropped replication stream. Matches the Kafka trigger listener's backoff.
const RECONNECT_DELAY_SECS: u64 = 30;

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
/// Returns `Error::BadConfig` when the publication or slot is missing (an
/// unrecoverable misconfiguration). Any other error is treated as transient
/// (connection refused, network interruption, ...) and is retried by the caller.
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

    let replication_slot = client
        .execute_query(&format!(
            "SELECT slot_name FROM pg_replication_slots WHERE slot_name = {}",
            quote_literal(replication_slot_name)
        ))
        .await
        .map_err(to_anyhow)?;

    if !replication_slot.row_exist() {
        return Err(Error::BadConfig(
            ERROR_REPLICATION_SLOT_NOT_EXISTS.to_string(),
        ));
    }

    client
        .get_logical_replication_stream(publication_name, replication_slot_name)
        .await
}

#[async_trait::async_trait]
impl Listener for PostgresTrigger {
    type Consumer = ApiAuthed;
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
        // The actual replication connection is established (and retried) inside
        // `consume`. Here we only resolve the auth context that connection needs.
        let authed = listening_trigger
            .authed(db, &Self::TRIGGER_KIND.to_string())
            .await?;

        Ok(Some(authed))
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
        let authed = consumer;
        // Consecutive failed connection attempts. Reset to 0 once the replication
        // stream is (re)established.
        let mut tries = 0_usize;

        loop {
            let (logical_replication_stream, logical_replication_settings) =
                match connect_logical_replication_stream(&authed, db, listening_trigger).await {
                    Ok(stream) => stream,
                    // Publication or replication slot missing: retrying cannot fix
                    // this, so disable the trigger as before.
                    Err(Error::BadConfig(err)) => {
                        self.disable_with_error(db, listening_trigger, err).await;
                        return;
                    }
                    // Transient failure (connection refused, network drop, ...):
                    // back off and retry instead of permanently disabling.
                    Err(err) => {
                        let status = format!(
                            "Failed to connect (attempt {}), retrying in {} seconds: {}",
                            tries + 1,
                            RECONNECT_DELAY_SECS,
                            err
                        );
                        if let None = self
                            .update_ping_and_loop_ping_status(
                                db,
                                listening_trigger,
                                err_message.clone(),
                                Some(status),
                            )
                            .await
                        {
                            return;
                        }

                        tracing::error!(
                            "Failed to connect postgres trigger {} (attempt {}), retrying in {} seconds: {}",
                            &listening_trigger.path,
                            tries + 1,
                            RECONNECT_DELAY_SECS,
                            err
                        );

                        if tries % 10 == 0 && listening_trigger.trigger_mode {
                            report_critical_error(
                                format!(
                                    "Failed to connect postgres trigger {} (attempt {}), retrying in {} seconds. This alert will repeat every 10 failed attempts. Error: {}",
                                    &listening_trigger.path,
                                    tries + 1,
                                    RECONNECT_DELAY_SECS,
                                    err
                                ),
                                db.clone(),
                                Some(&listening_trigger.workspace_id),
                                Some(&format!("postgres_trigger:{}", &listening_trigger.path)),
                            )
                            .await;
                        }

                        tries += 1;
                        tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
                        continue;
                    }
                };

            // The retry counter is reset (and recovery reported, ping cleared) only
            // once the stream actually delivers a message in the inner loop. A
            // connection that drops before making any progress therefore keeps
            // counting toward the reconnection alert instead of ping-ponging
            // silently.
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

            'stream: loop {
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
                    Some(message) => message,
                    None => {
                        if let None = self
                            .update_ping_and_loop_ping_status(
                                db,
                                listening_trigger,
                                err_message.clone(),
                                Some(format!(
                                    "Stream closed (attempt {}), reconnecting in {} seconds",
                                    tries + 1,
                                    RECONNECT_DELAY_SECS
                                )),
                            )
                            .await
                        {
                            return;
                        }
                        tracing::error!(
                            "Stream for postgres trigger {} closed (attempt {}), reconnecting in {} seconds",
                            &listening_trigger.path,
                            tries + 1,
                            RECONNECT_DELAY_SECS
                        );
                        if tries % 10 == 0 && listening_trigger.trigger_mode {
                            report_critical_error(
                                format!(
                                    "Postgres trigger {} stream closed (attempt {}), reconnecting in {} seconds. This alert will repeat every 10 failed attempts.",
                                    &listening_trigger.path,
                                    tries + 1,
                                    RECONNECT_DELAY_SECS
                                ),
                                db.clone(),
                                Some(&listening_trigger.workspace_id),
                                Some(&format!("postgres_trigger:{}", &listening_trigger.path)),
                            )
                            .await;
                        }
                        tries += 1;
                        tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
                        break 'stream;
                    }
                };

                let message = match message {
                    Ok(message) => message,
                    Err(err) => {
                        if let None = self
                            .update_ping_and_loop_ping_status(
                                db,
                                listening_trigger,
                                err_message.clone(),
                                Some(format!(
                                    "Error receiving message (attempt {}), reconnecting in {} seconds: {}",
                                    tries + 1,
                                    RECONNECT_DELAY_SECS,
                                    err
                                )),
                            )
                            .await
                        {
                            return;
                        }
                        tracing::error!(
                            "Postgres trigger {} had an error while receiving a message (attempt {}), reconnecting in {} seconds: {}",
                            &listening_trigger.path,
                            tries + 1,
                            RECONNECT_DELAY_SECS,
                            err.to_string()
                        );
                        if tries % 10 == 0 && listening_trigger.trigger_mode {
                            report_critical_error(
                                format!(
                                    "Postgres trigger {} error while receiving a message (attempt {}), reconnecting in {} seconds. This alert will repeat every 10 failed attempts. Error: {}",
                                    &listening_trigger.path,
                                    tries + 1,
                                    RECONNECT_DELAY_SECS,
                                    err
                                ),
                                db.clone(),
                                Some(&listening_trigger.workspace_id),
                                Some(&format!("postgres_trigger:{}", &listening_trigger.path)),
                            )
                            .await;
                        }
                        tries += 1;
                        tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
                        break 'stream;
                    }
                };

                // First successful read after a (re)connection means the stream is
                // making progress: clear the error status, report recovery, and
                // reset the retry counter. Deferred to here (rather than on connect)
                // so a stream that drops before delivering anything keeps counting
                // toward the reconnection alert.
                if tries > 0 {
                    if let None = self
                        .update_ping_and_loop_ping_status(
                            db,
                            listening_trigger,
                            err_message.clone(),
                            None,
                        )
                        .await
                    {
                        return;
                    }
                    if listening_trigger.trigger_mode {
                        report_recovered_critical_error(
                            format!("Postgres trigger {} reconnected", &listening_trigger.path),
                            db.clone(),
                            Some(&listening_trigger.workspace_id),
                            Some(&format!("postgres_trigger:{}", &listening_trigger.path)),
                        )
                        .await;
                    }
                    tries = 0;
                }

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
                                    .map(|old_tuple| {
                                        relations.row_to_json((update.o_id, old_tuple))
                                    })
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
