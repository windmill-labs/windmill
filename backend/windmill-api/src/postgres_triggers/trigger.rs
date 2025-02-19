use std::{collections::HashMap, pin::Pin};

use crate::{
    capture::{insert_capture_payload, PostgresTriggerConfig, TriggerKind},
    db::{ApiAuthed, DB},
    postgres_triggers::{
        relation::RelationConverter,
        replication_message::{
            LogicalReplicationMessage::{Begin, Commit, Delete, Insert, Relation, Type, Update},
            ReplicationMessage,
        },
        run_job,
    },
    users::fetch_api_authed, resources::try_get_resource_from_db_as,
};
use bytes::{BufMut, Bytes, BytesMut};
use chrono::TimeZone;
use futures::{pin_mut, SinkExt, StreamExt};
use native_tls::TlsConnector;
use pg_escape::{quote_identifier, quote_literal};
use rand::seq::SliceRandom;
use rust_postgres::{config::SslMode, Client, Config, CopyBothDuplex, SimpleQueryMessage};
use rust_postgres_native_tls::MakeTlsConnector;
use serde::Deserialize;
use serde_json::value::RawValue;
use sqlx::types::Json as SqlxJson;

use windmill_common::{
    db::UserDB, error, utils::report_critical_error, worker::to_raw_value, INSTANCE_NAME,
};
use windmill_queue::PushArgsOwned;

use super::{
    drop_logical_replication_slot_query, drop_publication_query, get_database_connection,
    handler::{Postgres, PostgresTrigger},
    replication_message::PrimaryKeepAliveBody,
    ERROR_PUBLICATION_NAME_NOT_EXISTS, ERROR_REPLICATION_SLOT_NOT_EXISTS,
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
    Postgres(#[from] rust_postgres::Error),
    #[error("Error : {0}")]
    Common(#[from] windmill_common::error::Error),
    #[error("Tls Error: {0}")]
    Tls(#[from] native_tls::Error),
}

pub struct PostgresSimpleClient(Client);

impl PostgresSimpleClient {
    async fn new(database: &Postgres) -> Result<Self, Error> {
        let ssl_mode = match database.sslmode.as_ref() {
            "disable" => SslMode::Disable,
            "" | "prefer" | "allow" => SslMode::Prefer,
            "require" => SslMode::Require,
            "verify-ca" => SslMode::VerifyCa,
            "verify-full" => SslMode::VerifyFull,
            ssl_mode => {
                return Err(Error::Common(windmill_common::error::Error::BadRequest(
                    format!("Invalid ssl mode for postgres: {}, please put a valid ssl_mode among the following avalible ssl mode: ['disable', 'allow', 'prefer', 'verify-ca', 'verify-full']", ssl_mode),
                )))
            }
        };

        let mut config = Config::new();
        config
            .dbname(&database.dbname)
            .host(&database.host)
            .user(&database.user)
            .ssl_mode(ssl_mode)
            .replication_mode(rust_postgres::config::ReplicationMode::Logical);

        if let Some(port) = database.port {
            config.port(port);
        };

        if !database.password.is_empty() {
            config.password(&database.password);
        }

        if !database.root_certificate_pem.is_empty() {
            config.ssl_root_cert(database.root_certificate_pem.as_bytes());
        }

        let connector = MakeTlsConnector::new(TlsConnector::new()?);

        let (client, connection) = config.connect(connector).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::debug!("{:#?}", e);
            };
            tracing::info!("Successfully Connected into database");
        });

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
                .await?,
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

    async fn handle(
        &self,
        db: &DB,
        args: Option<HashMap<String, Box<RawValue>>>,
        extra: Option<HashMap<String, Box<RawValue>>>,
    ) -> () {
        if let Err(err) = run_job(args, extra, db, self).await {
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

    fn retrieve_info(&self) -> PgInfo {
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
            authed,
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
            .await?;

        if !publication.row_exist() {
            return Err(Error::Common(error::Error::BadConfig(
                ERROR_PUBLICATION_NAME_NOT_EXISTS.to_string(),
            )));
        }

        let replication_slot = client
            .execute_query(&format!(
                "SELECT slot_name FROM pg_replication_slots WHERE slot_name = {}",
                quote_literal(&replication_slot_name)
            ))
            .await?;

        if !replication_slot.row_exist() {
            return Err(Error::Common(error::Error::BadConfig(
                ERROR_REPLICATION_SLOT_NOT_EXISTS.to_string(),
            )));
        }

        let (logical_replication_stream, logical_replication_settings) = client
            .get_logical_replication_stream(&publication_name, &replication_slot_name)
            .await?;

        Ok((logical_replication_stream, logical_replication_settings))
    }

    fn get_path(&self) -> &str {
        match self {
            PostgresConfig::Trigger(trigger) => &trigger.path,
            PostgresConfig::Capture(capture) => &capture.path,
        }
    }

    async fn handle(
        &self,
        db: &DB,
        args: Option<HashMap<String, Box<RawValue>>>,
        extra: Option<HashMap<String, Box<RawValue>>>,
    ) -> () {
        match self {
            PostgresConfig::Trigger(trigger) => trigger.handle(&db, args, extra).await,
            PostgresConfig::Capture(capture) => capture.handle(&db, args, extra).await,
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

                let mut connection = get_database_connection(
                    authed.clone(),
                    Some(user_db.clone()),
                    &db,
                    postgres_resource_path,
                    workspace_id,
                )
                .await?;

                let query = drop_logical_replication_slot_query(replication_slot_name);

                let _ = sqlx::query(&query).execute(&mut connection).await;

                let query = drop_publication_query(publication_name);

                let _ = sqlx::query(&query).execute(&mut connection).await;

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
                                                    Some((insert.o_id, relations.body_to_json((insert.o_id, insert.tuple)), "insert"))
                                                }
                                                Update(update) => {
                                                    Some((update.o_id, relations.body_to_json((update.o_id, update.new_tuple)), "update"))
                                                }
                                                Delete(delete) => {
                                                    let body = delete.old_tuple.unwrap_or_else(|| delete.key_tuple.unwrap());
                                                    Some((delete.o_id, relations.body_to_json((delete.o_id, body)), "delete"))
                                                }
                                            };
                                            if let Some((o_id, Ok(body), transaction_type)) = json {
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
                                                    ("row".to_string(), to_raw_value(&body)),
                                                ]);
                                                let extra = Some(HashMap::from([(
                                                    "wm_trigger".to_string(),
                                                    to_raw_value(&serde_json::json!({"kind": "postgres", })),
                                                )]));


                                                let _ = pg.handle(&db, Some(database_info), extra).await;
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

    async fn handle(
        &self,
        db: &DB,
        args: Option<HashMap<String, Box<RawValue>>>,
        extra: Option<HashMap<String, Box<RawValue>>>,
    ) -> () {
        let args = PushArgsOwned { args: args.unwrap_or_default(), extra };
        let extra = args.extra.as_ref().map(to_raw_value);
        if let Err(err) = insert_capture_payload(
            db,
            &self.workspace_id,
            &self.path,
            self.is_flow,
            &TriggerKind::Postgres,
            args,
            extra,
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
