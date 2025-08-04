use std::{collections::HashMap, pin::Pin};

use crate::{
    capture::{insert_capture_payload, PostgresTriggerConfig},
    db::{ApiAuthed, DB},
    postgres_triggers::{
        relation::RelationConverter,
        replication_message::{
            LogicalReplicationMessage::{Begin, Commit, Delete, Insert, Relation, Type, Update},
            ReplicationMessage,
        },
        run_job,
    },
    resources::try_get_resource_from_db_as,
    trigger_helpers::TriggerJobArgs,
    users::fetch_api_authed,
};

use bytes::{BufMut, Bytes, BytesMut};
use chrono::TimeZone;
use futures::{pin_mut, SinkExt, StreamExt};
use pg_escape::{quote_identifier, quote_literal};
use rand::seq::SliceRandom;
use rust_postgres::{Client, CopyBothDuplex, SimpleQueryMessage};
use serde::Deserialize;
use serde_json::value::RawValue;
use sqlx::types::Json as SqlxJson;

use windmill_common::{
    db::UserDB,
    error::{self, to_anyhow},
    triggers::TriggerKind,
    utils::report_critical_error,
    worker::to_raw_value,
    INSTANCE_NAME,
};

use super::{
    drop_publication, get_default_pg_connection, get_raw_postgres_connection,
    handler::{drop_logical_replication_slot, Postgres, PostgresTrigger},
    replication_message::PrimaryKeepAliveBody,
    Error, ERROR_PUBLICATION_NAME_NOT_EXISTS, ERROR_REPLICATION_SLOT_NOT_EXISTS,
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

pub struct PostgresSimpleClient(Client);

impl PostgresSimpleClient {
    async fn new(database: &Postgres) -> Result<Self, Error> {
        let client = get_raw_postgres_connection(database, true).await?;

        Ok(PostgresSimpleClient(client))
    }

    async fn execute_query(
        &self,
        query: &str,
    ) -> Result<Vec<SimpleQueryMessage>, rust_postgres::Error> {
        self.0.simple_query(query).await
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
                .map_err(to_anyhow)?,
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

async fn loop_ping(db: &DB, pg: &PostgresConfig, error: Option<&str>) {
    loop {
        if pg.update_ping(db, error).await.is_none() {
            return;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

enum PostgresConfig {
    Trigger(PostgresTrigger),
    Capture(CaptureConfigForPostgresTrigger),
}

impl PostgresTrigger {
    async fn try_to_listen_to_database_transactions(
        self,
        db: DB,
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> () {
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
            self.workspace_id,
            self.path,
        )
        .fetch_optional(&db)
        .await;
        match postgres_trigger {
            Ok(has_lock) => {
                if has_lock.flatten().unwrap_or(false) {
                    tracing::info!("Spawning new task to listen_to_database_transaction");
                    tokio::spawn(async move {
                        listen_to_transactions(
                            PostgresConfig::Trigger(self),
                            db.clone(),
                            killpill_rx,
                        )
                        .await;
                    });
                } else {
                    tracing::info!("Postgres trigger {} already being listened to", self.path);
                }
            }
            Err(err) => {
                tracing::error!(
                    "Error acquiring lock for postgres trigger {}: {:?}",
                    self.path,
                    err
                );
            }
        };
    }

    async fn update_ping(&self, db: &DB, error: Option<&str>) -> Option<()> {
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
            &self.workspace_id,
            &self.path,
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
                        &self.workspace_id,
                        &self.path,
                    )
                    .execute(db)
                    .await
                    .ok();
                    tracing::info!(
                        "Postgres trigger {} changed, disabled, or deleted, stopping...",
                        self.path
                    );
                    return None;
                }
            }
            Err(err) => {
                tracing::warn!(
                    "Error updating ping of postgres trigger {}: {:?}",
                    self.path,
                    err
                );
            }
        };

        Some(())
    }

    async fn disable_with_error(&self, db: &DB, error: String) -> () {
        match sqlx::query!(
            r#"
                UPDATE 
                    postgres_trigger 
                SET 
                    enabled = FALSE, 
                    error = $1, 
                    server_id = NULL, 
                    last_server_ping = NULL 
                WHERE 
                    workspace_id = $2 AND 
                    path = $3
            "#,
            error,
            self.workspace_id,
            self.path,
        )
        .execute(db)
        .await
        {
            Ok(_) => {
                report_critical_error(
                    format!(
                        "Disabling postgres trigger {} because of error: {}",
                        self.path, error
                    ),
                    db.clone(),
                    Some(&self.workspace_id),
                    None,
                )
                .await;
            }
            Err(disable_err) => {
                report_critical_error(
                    format!("Could not disable postgres trigger {} with err {}, disabling because of error {}", self.path, disable_err, error), 
                    db.clone(),
                    Some(&self.workspace_id),
                    None,
                ).await;
            }
        }
    }

    async fn fetch_authed(&self, db: &DB) -> error::Result<ApiAuthed> {
        fetch_api_authed(
            self.edited_by.clone(),
            self.email.clone(),
            &self.workspace_id,
            db,
            Some(format!("pg-{}", self.path)),
        )
        .await
    }

    async fn handle(&self, db: &DB, payload: HashMap<String, Box<RawValue>>) -> () {
        if let Err(err) = run_job(payload, db, self).await {
            report_critical_error(
                format!(
                    "Failed to trigger job from postgres {}: {:?}",
                    self.path, err
                ),
                db.clone(),
                Some(&self.workspace_id),
                None,
            )
            .await;
        };
    }
}

impl TriggerJobArgs<HashMap<String, Box<RawValue>>> for PostgresTrigger {
    fn v1_payload_fn(payload: HashMap<String, Box<RawValue>>) -> HashMap<String, Box<RawValue>> {
        payload
    }

    fn v2_payload_fn(payload: HashMap<String, Box<RawValue>>) -> HashMap<String, Box<RawValue>> {
        payload
    }

    fn trigger_kind() -> TriggerKind {
        TriggerKind::Postgres
    }
}

struct PgInfo<'a> {
    postgres_resource_path: &'a str,
    publication_name: &'a str,
    replication_slot_name: &'a str,
    workspace_id: &'a str,
}

impl PostgresConfig {
    async fn update_ping(&self, db: &DB, error: Option<&str>) -> Option<()> {
        match self {
            PostgresConfig::Trigger(trigger) => trigger.update_ping(db, error).await,
            PostgresConfig::Capture(capture) => capture.update_ping(db, error).await,
        }
    }

    async fn disable_with_error(&self, db: &DB, error: String) -> () {
        match self {
            PostgresConfig::Trigger(trigger) => trigger.disable_with_error(&db, error).await,
            PostgresConfig::Capture(capture) => capture.disable_with_error(db, error).await,
        }
    }

    fn retrieve_info(&self) -> PgInfo<'_> {
        let postgres_resource_path;
        let publication_name;
        let replication_slot_name;
        let workspace_id;

        match self {
            PostgresConfig::Trigger(trigger) => {
                postgres_resource_path = &trigger.postgres_resource_path;
                publication_name = &trigger.publication_name;
                replication_slot_name = &trigger.replication_slot_name;
                workspace_id = &trigger.workspace_id;
            }
            PostgresConfig::Capture(capture) => {
                postgres_resource_path = &capture.trigger_config.postgres_resource_path;
                workspace_id = &capture.workspace_id;
                publication_name = capture.trigger_config.publication_name.as_ref().unwrap();
                replication_slot_name = capture
                    .trigger_config
                    .replication_slot_name
                    .as_ref()
                    .unwrap();
            }
        };

        PgInfo { postgres_resource_path, replication_slot_name, workspace_id, publication_name }
    }

    async fn start_logical_replication_streaming(
        &self,
        db: &DB,
    ) -> std::result::Result<(CopyBothDuplex<Bytes>, LogicalReplicationSettings), Error> {
        let PgInfo {
            publication_name,
            replication_slot_name,
            workspace_id,
            postgres_resource_path,
        } = self.retrieve_info();

        let authed = match self {
            PostgresConfig::Trigger(trigger) => trigger.fetch_authed(db).await?,
            PostgresConfig::Capture(capture) => capture.fetch_authed(db).await?,
        };

        let database = try_get_resource_from_db_as::<Postgres>(
            &authed,
            Some(UserDB::new(db.clone())),
            &db,
            postgres_resource_path,
            workspace_id,
        )
        .await?;

        let client = PostgresSimpleClient::new(&database).await?;

        let publication = client
            .execute_query(&format!(
                "SELECT pubname FROM pg_publication WHERE pubname = {}",
                quote_literal(&publication_name)
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
                quote_literal(&replication_slot_name)
            ))
            .await
            .map_err(to_anyhow)?;

        if !replication_slot.row_exist() {
            return Err(Error::BadConfig(
                ERROR_REPLICATION_SLOT_NOT_EXISTS.to_string(),
            ));
        }

        let (logical_replication_stream, logical_replication_settings) = client
            .get_logical_replication_stream(&publication_name, &replication_slot_name)
            .await
            .map_err(to_anyhow)?;

        Ok((logical_replication_stream, logical_replication_settings))
    }

    fn get_path(&self) -> &str {
        match self {
            PostgresConfig::Trigger(trigger) => &trigger.path,
            PostgresConfig::Capture(capture) => &capture.path,
        }
    }

    async fn handle(&self, db: &DB, payload: HashMap<String, Box<RawValue>>) -> () {
        match self {
            PostgresConfig::Trigger(trigger) => trigger.handle(&db, payload).await,
            PostgresConfig::Capture(capture) => capture.handle(&db, payload).await,
        }
    }

    async fn cleanup(&self, db: &DB) -> Result<(), Error> {
        match self {
            PostgresConfig::Trigger(_) => Ok(()),
            PostgresConfig::Capture(capture) => {
                let publication_name = capture.trigger_config.publication_name.as_ref().unwrap();
                let replication_slot_name = capture
                    .trigger_config
                    .replication_slot_name
                    .as_ref()
                    .unwrap();
                let postgres_resource_path = &capture.trigger_config.postgres_resource_path;
                let workspace_id = &capture.workspace_id;
                let authed = capture.fetch_authed(&db).await?;

                let user_db = UserDB::new(db.clone());

                let mut pg_connection = get_default_pg_connection(
                    authed.clone(),
                    Some(user_db.clone()),
                    &db,
                    postgres_resource_path,
                    workspace_id,
                )
                .await?;

                if capture.trigger_config.basic_mode.unwrap_or(false) {
                    drop_logical_replication_slot(&mut pg_connection, replication_slot_name)
                        .await?;

                    drop_publication(&mut pg_connection, publication_name).await?;
                }

                Ok(())
            }
        }
    }
}

async fn listen_to_transactions(
    pg: PostgresConfig,
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::select! {
        biased;
        _ = killpill_rx.recv() => {
            let _ = pg.cleanup(&db).await;
            return;
        }
        _ = loop_ping(&db, &pg, Some("Connecting...")) => {
            let _ = pg.cleanup(&db).await;
            return;
        }
        result = pg.start_logical_replication_streaming(&db) => {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    let _ = pg.cleanup(&db).await;
                    return;
                }
                _ = loop_ping(&db, &pg, None) => {
                    let _ = pg.cleanup(&db).await;
                    return;
                }
                _ = {
                    async {
                        match result {
                            Ok((logical_replication_stream, logical_replication_settings)) => {
                                pin_mut!(logical_replication_stream);
                                let mut relations = RelationConverter::new();
                                tracing::info!("Starting to listen for postgres trigger {}", pg.get_path());
                                loop {
                                    let message = logical_replication_stream.next().await;

                                    let message = match message {
                                        Some(message) => message,
                                        None => {
                                            tracing::error!("Stream for postgres trigger {} closed", pg.get_path());
                                            if let None = pg.update_ping(&db, Some("Stream closed")).await {
                                                return;
                                            }
                                            return;
                                        }
                                    };


                                    let message = match message {
                                        Ok(message) => message,
                                        Err(err) => {
                                            let err = format!("Postgres trigger named {} had an error while receiving a message : {}", pg.get_path(), err.to_string());
                                            pg.disable_with_error(&db, err).await;
                                            return;
                                        }
                                    };

                                    let logical_message = match ReplicationMessage::parse(message) {
                                        Ok(logical_message) => logical_message,
                                        Err(err) => {
                                            let err = format!("Postgres trigger named: {} had an error while parsing message: {}", pg.get_path(), err.to_string());
                                            pg.disable_with_error(&db, err).await;
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
                                            let logical_replication_message = match x_log_data.parse(&logical_replication_settings) {
                                                Ok(logical_replication_message) => logical_replication_message,
                                                Err(err) => {
                                                    tracing::error!("Postgres trigger named: {} had an error while trying to parse incomming stream message: {}", pg.get_path(), err.to_string());
                                                    continue;
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
                                                    Some((insert.o_id, Ok(None), relations.row_to_json((insert.o_id, insert.tuple)), "insert"))
                                                }
                                                Update(update) => {
                                                    let old_row = update.old_tuple.map(|old_tuple| relations.row_to_json((update.o_id, old_tuple))).transpose();
                                                    let row = relations.row_to_json((update.o_id, update.new_tuple));
                                                    Some((update.o_id, old_row, row, "update"))
                                                }
                                                Delete(delete) => {
                                                    let row = delete.old_tuple.unwrap_or_else(|| delete.key_tuple.unwrap());
                                                    Some((delete.o_id, Ok(None), relations.row_to_json((delete.o_id, row)), "delete"))
                                                }
                                            };
                                            match json {
                                                Some((o_id, Ok(old_row), Ok(row), transaction_type)) => {
                                                    let relation = match relations.get_relation(o_id) {
                                                        Ok(relation) => relation,
                                                        Err(err) => {
                                                            tracing::error!("Postgres trigger named: {}, error: {}", pg.get_path(), err.to_string());
                                                            continue;
                                                        }
                                                    };
                                                    let database_info = HashMap::from([
                                                        ("schema_name".to_string(), to_raw_value(&relation.namespace)),
                                                        ("table_name".to_string(), to_raw_value(&relation.name)),
                                                        ("transaction_type".to_string(), to_raw_value(&transaction_type)),
                                                        ("old_row".to_string(), to_raw_value(&old_row)),
                                                        ("row".to_string(), to_raw_value(&row)),
                                                    ]);


                                                    let _ = pg.handle(&db, database_info).await;
                                                }
                                                Some((o_id, old_row, row, transaction_type)) => {
                                                    let relation = match relations.get_relation(o_id) {
                                                        Ok(relation) => relation,
                                                        Err(err) => {
                                                            tracing::error!("Postgres trigger named: {}, error: {}", pg.get_path(), err.to_string());
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
                            Err(err) => {
                                tracing::error!("Postgres trigger error while trying to start logical replication streaming: {}", &err);
                                pg.disable_with_error(&db, err.to_string()).await
                            }
                        }
                    }
                } => {
                    let _ = pg.cleanup(&db).await;
                    return;
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct CaptureConfigForPostgresTrigger {
    trigger_config: SqlxJson<PostgresTriggerConfig>,
    path: String,
    is_flow: bool,
    workspace_id: String,
    owner: String,
    email: String,
}

impl CaptureConfigForPostgresTrigger {
    async fn try_to_listen_to_database_transactions(
        self,
        db: DB,
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> () {
        match sqlx::query_scalar!(
            r#"
            UPDATE 
                capture_config 
            SET 
                server_id = $1,
                last_server_ping = now(), 
                error = 'Connecting...' 
            WHERE 
                last_client_ping > NOW() - INTERVAL '10 seconds' AND 
                workspace_id = $2 AND 
                path = $3 AND 
                is_flow = $4 AND 
                trigger_kind = 'postgres' AND 
                (last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds') 
            RETURNING true
            "#,
            *INSTANCE_NAME,
            self.workspace_id,
            self.path,
            self.is_flow,
        )
        .fetch_optional(&db)
        .await
        {
            Ok(has_lock) => {
                if has_lock.flatten().unwrap_or(false) {
                    tokio::spawn(listen_to_transactions(
                        PostgresConfig::Capture(self),
                        db,
                        killpill_rx,
                    ));
                } else {
                    tracing::info!("Postgres {} already being listened to", self.path);
                }
            }
            Err(err) => {
                tracing::error!(
                    "Error acquiring lock for capture postgres {}: {:?}",
                    self.path,
                    err
                );
            }
        };
    }

    async fn update_ping(&self, db: &DB, error: Option<&str>) -> Option<()> {
        match sqlx::query_scalar!(
            r#"
            UPDATE 
                capture_config 
            SET 
                last_server_ping = now(), 
                error = $1 
            WHERE 
                workspace_id = $2 AND 
                path = $3 AND 
                is_flow = $4 AND 
                trigger_kind = 'postgres' AND 
                server_id = $5 AND 
                last_client_ping > NOW() - INTERVAL '10 seconds' 
            RETURNING 1
        "#,
            error,
            self.workspace_id,
            self.path,
            self.is_flow,
            *INSTANCE_NAME
        )
        .fetch_optional(db)
        .await
        {
            Ok(updated) => {
                if updated.flatten().is_none() {
                    // allow faster restart of postgres capture
                    sqlx::query!(
                        r#"UPDATE 
                        capture_config 
                    SET 
                        last_server_ping = NULL 
                    WHERE 
                        workspace_id = $1 AND 
                        path = $2 AND 
                        is_flow = $3 AND 
                        trigger_kind = 'postgres' AND 
                        server_id IS NULL
                    "#,
                        self.workspace_id,
                        self.path,
                        self.is_flow,
                    )
                    .execute(db)
                    .await
                    .ok();
                    tracing::info!(
                        "Postgres capture {} changed, disabled, or deleted, stopping...",
                        self.path
                    );
                    return None;
                }
            }
            Err(err) => {
                tracing::warn!(
                    "Error updating ping of capture postgres {}: {:?}",
                    self.path,
                    err
                );
            }
        };

        Some(())
    }

    async fn fetch_authed(&self, db: &DB) -> error::Result<ApiAuthed> {
        fetch_api_authed(
            self.owner.clone(),
            self.email.clone(),
            &self.workspace_id,
            db,
            Some(format!("postgres-{}", self.get_trigger_path())),
        )
        .await
    }

    fn get_trigger_path(&self) -> String {
        format!(
            "{}-{}",
            if self.is_flow { "flow" } else { "script" },
            self.path
        )
    }

    async fn disable_with_error(&self, db: &DB, error: String) -> () {
        if let Err(err) = sqlx::query!(
            r#"
                UPDATE 
                    capture_config 
                SET 
                    error = $1, 
                    server_id = NULL, 
                    last_server_ping = NULL 
                WHERE 
                    workspace_id = $2 AND 
                    path = $3 AND 
                    is_flow = $4 AND 
                    trigger_kind = 'postgres'
            "#,
            error,
            self.workspace_id,
            self.path,
            self.is_flow,
        )
        .execute(db)
        .await
        {
            tracing::error!("Could not disable postgres capture {} ({}) with err {}, disabling because of error {}", self.path, self.workspace_id, err, error);
        }
    }

    async fn handle(&self, db: &DB, payload: HashMap<String, Box<RawValue>>) -> () {
        let main_args = PostgresTrigger::build_job_args_v2(false, payload.clone(), HashMap::new());
        let preprocessor_args = PostgresTrigger::build_job_args_v2(true, payload, HashMap::new());
        if let Err(err) = insert_capture_payload(
            db,
            &self.workspace_id,
            &self.path,
            self.is_flow,
            &TriggerKind::Postgres,
            main_args,
            preprocessor_args,
            &self.owner,
        )
        .await
        {
            tracing::error!("Error inserting capture payload: {:?}", err);
        }
    }
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
                postgres_resource_path,
                error_handler_path,
                error_handler_args as "error_handler_args: _",
                retry as "retry: _",
                email_recipients
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
            triggers.shuffle(&mut rand::rng());
            for trigger in triggers {
                trigger
                    .try_to_listen_to_database_transactions(db.clone(), killpill_rx.resubscribe())
                    .await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching postgres triggers: {:?}", err);
        }
    };

    let postgres_triggers_capture = sqlx::query_as!(
        CaptureConfigForPostgresTrigger,
        r#"
            SELECT
                path,
                is_flow,
                workspace_id,
                owner,
                email,
                trigger_config as "trigger_config!: _"
            FROM
                capture_config
            WHERE
                trigger_kind = 'postgres' AND
                last_client_ping > NOW() - INTERVAL '10 seconds' AND
                trigger_config IS NOT NULL AND
                (last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')
            "#
    )
    .fetch_all(db)
    .await;

    match postgres_triggers_capture {
        Ok(mut captures) => {
            captures.shuffle(&mut rand::rng());
            for capture in captures {
                capture
                    .try_to_listen_to_database_transactions(db.clone(), killpill_rx.resubscribe())
                    .await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching captures postgres triggers: {:?}", err);
        }
    };
}

pub fn start_database(db: DB, mut killpill_rx: tokio::sync::broadcast::Receiver<()>) {
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
