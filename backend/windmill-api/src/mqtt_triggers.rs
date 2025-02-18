use crate::{
    capture::{insert_capture_payload, MqttTriggerConfig, TriggerKind},
    db::{ApiAuthed, DB},
    jobs::{run_flow_by_path_inner, run_script_by_path_inner, RunJobQuery},
    resources::try_get_resource_from_db_as,
    users::fetch_api_authed,
};
use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use axum::{
    routing::{delete, get, post},
    Router,
};
use http::StatusCode;
use itertools::Itertools;
use rumqttc::{v5::AsyncClient as AsyncClientV5, AsyncClient as AsyncClientV3, MqttOptions};
use serde::{Deserialize, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{
    error::BoxDynError,
    postgres::{PgHasArrayType, PgTypeInfo, PgValueRef},
    Decode, Encode, FromRow, Postgres, Type,
};
use std::collections::HashMap;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::{
    auth,
    db::UserDB,
    error::{self, JsonResult},
    utils::{not_found_if_none, paginate, report_critical_error, Pagination, StripPath},
    worker::{to_raw_value, CLOUD_HOSTED},
    INSTANCE_NAME,
};

use rand::seq::SliceRandom;
use serde_json::{to_value, value::RawValue, Value};
use sqlx::types::Json as SqlxJson;

use windmill_queue::PushArgsOwned;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_mqtt_trigger))
        .route("/list", get(list_mqtt_triggers))
        .route("/get/*path", get(get_mqtt_trigger))
        .route("/update/*path", post(update_mqtt_trigger))
        .route("/delete/*path", delete(delete_mqtt_trigger))
        .route("/exists/*path", get(exists_mqtt_trigger))
        .route("/setenabled/*path", post(set_enabled))
}

async fn run_job(
    args: Option<HashMap<String, Box<RawValue>>>,
    extra: Option<HashMap<String, Box<RawValue>>>,
    db: &DB,
    trigger: &MqttTrigger,
) -> anyhow::Result<()> {
    let args = PushArgsOwned { args: args.unwrap_or_default(), extra };

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        db,
        Some(format!("mqtt-{}", trigger.path)),
    )
    .await?;

    let user_db = UserDB::new(db.clone());

    let run_query = RunJobQuery::default();

    if trigger.is_flow {
        run_flow_by_path_inner(
            authed,
            db.clone(),
            user_db,
            trigger.workspace_id.clone(),
            StripPath(trigger.script_path.to_owned()),
            run_query,
            args,
            None,
        )
        .await?;
    } else {
        run_script_by_path_inner(
            authed,
            db.clone(),
            user_db,
            trigger.workspace_id.clone(),
            StripPath(trigger.script_path.to_owned()),
            run_query,
            args,
            None,
        )
        .await?;
    }

    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LastWillConfig {
    topic: String,
    payload: Vec<u8>,
    qos: QualityOfService,
    retain: bool,
}

trait MqttClient {}

impl MqttClient for AsyncClientV5 {}

impl MqttClient for AsyncClientV3 {}

struct MqttAsyncClient<T>(pub T)
where
    T: MqttClient;

impl<T> MqttAsyncClient<T> where T: MqttClient {}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommonMqttConfig {
    client_id: Option<String>,
    will: Option<LastWillConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum QualityOfService {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MqttV3Config {
    #[serde(flatten)]
    base: CommonMqttConfig,
    clean_session: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MqttV5Config {
    #[serde(flatten)]
    base: CommonMqttConfig,
    clean_start: Option<bool>,
    keep_alive: Option<u16>,
    session_expiration: Option<u16>,
    receive_maximum: Option<u16>,
    maximum_packet_size: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Type)]
#[sqlx(type_name = "MQTT_CLIENT_VERSION")]
#[sqlx(rename_all = "lowercase")]
pub enum MqttClientVersion {
    V3,
    V5,
}

#[derive(Debug, Deserialize)]
pub struct MqttResource {
    username: Option<String>,
    password: Option<String>,
    port: u16,
    host: String,
    ca_certificate: Option<Vec<u8>>,
}
#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct SubscribeTopic {
    qos: u8,
    topic: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewMqttTrigger {
    mqtt_resource_path: String,
    subscribe_topics: Vec<SubscribeTopic>,
    v3_config: Option<MqttV3Config>,
    v5_config: Option<MqttV5Config>,
    client_version: MqttClientVersion,
    path: String,
    script_path: String,
    is_flow: bool,
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EditMqttTrigger {
    mqtt_resource_path: String,
    subscribe_topics: Vec<SubscribeTopic>,
    v3_config: Option<MqttV3Config>,
    v5_config: Option<MqttV5Config>,
    client_version: MqttClientVersion,
    path: String,
    script_path: String,
    is_flow: bool,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct MqttTrigger {
    pub mqtt_resource_path: String,
    pub subscribe_topics: Vec<SqlxJson<SubscribeTopic>>,
    pub v3_config: Option<SqlxJson<MqttV3Config>>,
    pub v5_config: Option<SqlxJson<MqttV5Config>>,
    pub client_version: MqttClientVersion,
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub workspace_id: String,
    pub edited_by: String,
    pub email: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub extra_perms: Option<serde_json::Value>,
    pub error: Option<String>,
    pub server_id: Option<String>,
    pub last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
}

#[derive(Deserialize, Serialize)]
pub struct ListMqttTriggerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
    pub path_start: Option<String>,
}

#[derive(Deserialize)]
pub struct SetEnabled {
    pub enabled: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Common(#[from] windmill_common::error::Error),
}

pub async fn create_mqtt_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(new_mqtt_trigger): Json<NewMqttTrigger>,
) -> error::Result<(StatusCode, String)> {
    if *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "Mqtt triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host".to_string(),
        ));
    }

    let NewMqttTrigger {
        mqtt_resource_path,
        subscribe_topics,
        path,
        script_path,
        enabled,
        is_flow,
        v3_config,
        v5_config,
        client_version,
    } = new_mqtt_trigger;

    let mut tx = user_db.begin(&authed).await?;

    let subscribe_topics = subscribe_topics.into_iter().map(SqlxJson).collect_vec();
    let v3_config = v3_config.map(SqlxJson);
    let v5_config = v5_config.map(SqlxJson);

    sqlx::query!(
        r#"
        INSERT INTO mqtt_trigger (
            mqtt_resource_path,
            subscribe_topics,
            client_version,
            v3_config,
            v5_config,
            workspace_id,
            path, 
            script_path, 
            is_flow, 
            email, 
            enabled, 
            edited_by
        ) 
        VALUES (
            $1, 
            $2, 
            $3, 
            $4, 
            $5, 
            $6, 
            $7,
            $8,
            $9,
            $10,
            $11,
            $12
        )"#,
        mqtt_resource_path,
        subscribe_topics.as_slice() as &[SqlxJson<SubscribeTopic>],
        client_version as MqttClientVersion,
        v3_config as Option<SqlxJson<MqttV3Config>>,
        v5_config as Option<SqlxJson<MqttV5Config>>,
        &w_id,
        &path,
        script_path,
        is_flow,
        &authed.email,
        enabled,
        &authed.username
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "mqtt_triggers.create",
        ActionKind::Create,
        &w_id,
        Some(path.as_str()),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, path.to_string()))
}

pub async fn list_mqtt_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(lst): Query<ListMqttTriggerQuery>,
) -> error::JsonResult<Vec<MqttTrigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(Pagination { per_page: lst.per_page, page: lst.page });
    let mut sqlb = SqlBuilder::select_from("mqtt_trigger")
        .fields(&[
            "mqtt_resource_path",
            "subscribe_topics",
            "v3_config",
            "v5_config",
            "workspace_id",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "email",
            "edited_at",
            "server_id",
            "last_server_ping",
            "extra_perms",
            "error",
            "enabled",
        ])
        .order_by("edited_at", true)
        .and_where("workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();
    if let Some(path) = lst.path {
        sqlb.and_where_eq("script_path", "?".bind(&path));
    }
    if let Some(is_flow) = lst.is_flow {
        sqlb.and_where_eq("is_flow", "?".bind(&is_flow));
    }
    if let Some(path_start) = &lst.path_start {
        sqlb.and_where_like_left("path", path_start);
    }
    let sql = sqlb
        .sql()
        .map_err(|e| error::Error::InternalErr(e.to_string()))?;
    let rows = sqlx::query_as::<_, MqttTrigger>(&sql)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| {
            tracing::debug!("Error fetching mqtt_trigger: {:#?}", e);
            windmill_common::error::Error::InternalErr("server error".to_string())
        })?;
    tx.commit().await.map_err(|e| {
        tracing::debug!("Error commiting mqtt_trigger: {:#?}", e);
        windmill_common::error::Error::InternalErr("server error".to_string())
    })?;

    Ok(Json(rows))
}

pub async fn get_mqtt_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<MqttTrigger> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();
    let trigger = sqlx::query_as!(
        MqttTrigger,
        r#"
        SELECT
            mqtt_resource_path,
            subscribe_topics as "subscribe_topics!: Vec<SqlxJson<SubscribeTopic>>",
            v3_config as "v3_config!: Option<SqlxJson<MqttV3Config>>",
            v5_config as "v5_config!: Option<SqlxJson<MqttV5Config>>",
            client_version AS "client_version: _",
            workspace_id,
            path,
            script_path,
            is_flow,
            edited_by,
            email,
            edited_at,
            server_id,
            last_server_ping,
            extra_perms,
            error,
            enabled
        FROM 
            mqtt_trigger
        WHERE 
            workspace_id = $1 AND 
            path = $2
        "#,
        w_id,
        &path
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let trigger = not_found_if_none(trigger, "Mqtt Trigger", path)?;

    Ok(Json(trigger))
}

pub async fn update_mqtt_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(mqtt_trigger): Json<EditMqttTrigger>,
) -> error::Result<String> {
    let workspace_path = path.to_path();
    let EditMqttTrigger {
        mqtt_resource_path,
        subscribe_topics,
        script_path,
        path,
        is_flow,
        v3_config,
        v5_config,
        client_version,
    } = mqtt_trigger;

    //valid_config_version(client_version, advance_configuration.as_ref())?;

    let mut tx = user_db.begin(&authed).await?;

    let subscribe_topics = subscribe_topics.into_iter().map(SqlxJson).collect_vec();

    let v3_config = v3_config.map(SqlxJson);
    let v5_config = v5_config.map(SqlxJson);

    sqlx::query!(
        r#"
            UPDATE 
                mqtt_trigger 
            SET
                mqtt_resource_path =  $1,
                subscribe_topics = $2,
                client_version = $3,
                v3_config = $4,
                v5_config = $5,
                is_flow = $6, 
                edited_by = $7, 
                email = $8,
                script_path = $9,
                path = $10,
                edited_at = now(), 
                error = NULL,
                server_id = NULL
            WHERE 
                workspace_id = $11 AND 
                path = $12
            "#,
        mqtt_resource_path,
        subscribe_topics.as_slice() as &[SqlxJson<SubscribeTopic>],
        client_version as MqttClientVersion,
        v3_config as Option<SqlxJson<MqttV3Config>>,
        v5_config as Option<SqlxJson<MqttV5Config>>,
        is_flow,
        &authed.username,
        &authed.email,
        script_path,
        path,
        w_id,
        workspace_path,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "mqtt_triggers.update",
        ActionKind::Create,
        &w_id,
        Some(&path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(workspace_path.to_string())
}

pub async fn delete_mqtt_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        r#"
        DELETE 
        FROM 
            mqtt_trigger 
        WHERE 
            workspace_id = $1 AND 
            path = $2
        "#,
        w_id,
        path,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "mqtt_triggers.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Mqtt trigger {path} deleted"))
}

pub async fn exists_mqtt_trigger(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 
                1 
            FROM 
                mqtt_trigger 
            WHERE 
                path = $1 AND 
                workspace_id = $2
        )"#,
        path,
        w_id,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    Ok(Json(exists))
}

pub async fn set_enabled(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabled>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();

    // important to set server_id, last_server_ping and error to NULL to stop current mqtt listener
    let one_o = sqlx::query_scalar!(
        r#"
        UPDATE 
            mqtt_trigger 
        SET 
            enabled = $1, 
            email = $2, 
            edited_by = $3, 
            edited_at = now(), 
            server_id = NULL, 
            error = NULL
        WHERE 
            path = $4 AND 
            workspace_id = $5 
        RETURNING 1
        "#,
        payload.enabled,
        &authed.email,
        &authed.username,
        path,
        w_id,
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    not_found_if_none(one_o, "Mqtt trigger", path)?;

    audit_log(
        &mut *tx,
        &authed,
        "mqtt_triggers.setenabled",
        ActionKind::Update,
        &w_id,
        Some(path),
        Some([("enabled", payload.enabled.to_string().as_ref())].into()),
    )
    .await?;

    tx.commit().await?;

    Ok(format!(
        "succesfully updated mqttq trigger at path {} to status {}",
        path, payload.enabled
    ))
}

async fn loop_ping(db: &DB, mqtt: &MqttConfig, error: Option<&str>) {
    loop {
        if mqtt.update_ping(db, error).await.is_none() {
            return;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

enum MqttConfig {
    Trigger(MqttTrigger),
    Capture(CaptureConfigForMqttTrigger),
}

impl MqttTrigger {
    async fn try_to_listen_to_mqtt_messages(
        self,
        db: DB,
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> () {
        let mqtt_trigger = sqlx::query_scalar!(
            r#"
            UPDATE 
                mqtt_trigger 
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
        match mqtt_trigger {
            Ok(has_lock) => {
                if has_lock.flatten().unwrap_or(false) {
                    tracing::info!("Spawning new task to listen_to_database_transaction");
                    tokio::spawn(async move {
                        listen_to_messages(MqttConfig::Trigger(self), db.clone(), killpill_rx)
                            .await;
                    });
                } else {
                    tracing::info!("Mqtt trigger {} already being listened to", self.path);
                }
            }
            Err(err) => {
                tracing::error!(
                    "Error acquiring lock for mqtt trigger {}: {:?}",
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
                mqtt_trigger
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
                        mqtt_trigger 
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
                        "Mqtt trigger {} changed, disabled, or deleted, stopping...",
                        self.path
                    );
                    return None;
                }
            }
            Err(err) => {
                tracing::warn!(
                    "Error updating ping of mqtt trigger {}: {:?}",
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
                    mqtt_trigger 
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
                        "Disabling mqtt trigger {} because of error: {}",
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
                    format!("Could not disable mqtt trigger {} with err {}, disabling because of error {}", self.path, disable_err, error), 
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
            Some(format!("mqtt-{}", self.path)),
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
                format!("Failed to trigger job from mqtt {}: {:?}", self.path, err),
                db.clone(),
                Some(&self.workspace_id),
                None,
            )
            .await;
        };
    }
}

impl MqttConfig {
    async fn update_ping(&self, db: &DB, error: Option<&str>) -> Option<()> {
        match self {
            MqttConfig::Trigger(trigger) => trigger.update_ping(db, error).await,
            MqttConfig::Capture(capture) => capture.update_ping(db, error).await,
        }
    }

    async fn disable_with_error(&self, db: &DB, error: String) -> () {
        match self {
            MqttConfig::Trigger(trigger) => trigger.disable_with_error(&db, error).await,
            MqttConfig::Capture(capture) => capture.disable_with_error(db, error).await,
        }
    }

    async fn start_consuming_messages(&self, db: &DB) -> std::result::Result<(), Error> {
        let mqtt_resource_path;
        let topics;
        let workspace_id;
        let authed;

        match self {
            MqttConfig::Capture(capture) => {
                mqtt_resource_path = &capture.trigger_config.0.mqtt_resource_path;
                topics = capture
                    .trigger_config
                    .0
                    .subscribe_topics
                    .iter()
                    .collect_vec()
                    .as_slice();
                workspace_id = &capture.trigger_config.0.mqtt_resource_path;
                authed = capture.fetch_authed(&db).await?;
            }
            MqttConfig::Trigger(trigger) => {
                mqtt_resource_path = &trigger.mqtt_resource_path;
                topics = trigger
                    .subscribe_topics
                    .iter()
                    .map(|topic| &topic.0)
                    .collect_vec()
                    .as_slice();
                workspace_id = &trigger.workspace_id;
                authed = trigger.fetch_authed(&db).await?;
            }
        }

        let mqtt_resource = try_get_resource_from_db_as::<MqttResource>(
            authed,
            Some(UserDB::new(db.clone())),
            db,
            mqtt_resource_path,
            workspace_id,
        )
        .await?;

        //let options = MqttOptions::new(, host, port);
        Ok(())
    }

    async fn handle(
        &self,
        db: &DB,
        args: Option<HashMap<String, Box<RawValue>>>,
        extra: Option<HashMap<String, Box<RawValue>>>,
    ) -> () {
        match self {
            MqttConfig::Trigger(trigger) => trigger.handle(&db, args, extra).await,
            MqttConfig::Capture(capture) => capture.handle(&db, args, extra).await,
        }
    }
}

async fn listen_to_messages(
    mqtt: MqttConfig,
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::select! {
        biased;
        _ = killpill_rx.recv() => {
            return;
        }
        _ = loop_ping(&db, &mqtt, Some("Connecting...")) => {
            return;
        }
        result = mqtt.start_consuming_messages(&db) => {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = loop_ping(&db, &mqtt, None) => {
                    return;
                }
                _ = {
                    async {
                        match result {
                            Ok(_) => {
                                loop {

                                }
                            }
                            Err(err) => {
                                tracing::error!("Mqtt trigger error while trying to start listening to messages: {}", &err);
                                mqtt.disable_with_error(&db, err.to_string()).await
                            }
                        }
                    }
                } => {
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct CaptureConfigForMqttTrigger {
    trigger_config: SqlxJson<MqttTriggerConfig>,
    path: String,
    is_flow: bool,
    workspace_id: String,
    owner: String,
    email: String,
}

impl CaptureConfigForMqttTrigger {
    async fn try_to_listen_to_mqtt_messages(
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
                trigger_kind = 'mqtt' AND 
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
                    tokio::spawn(listen_to_messages(
                        MqttConfig::Capture(self),
                        db,
                        killpill_rx,
                    ));
                } else {
                    tracing::info!("Mqtt {} already being listened to", self.path);
                }
            }
            Err(err) => {
                tracing::error!(
                    "Error acquiring lock for capture mqtt {}: {:?}",
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
                trigger_kind = 'mqtt' AND 
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
                    // allow faster restart of mqtt capture
                    sqlx::query!(
                        r#"UPDATE 
                        capture_config 
                    SET 
                        last_server_ping = NULL 
                    WHERE 
                        workspace_id = $1 AND 
                        path = $2 AND 
                        is_flow = $3 AND 
                        trigger_kind = 'mqtt' AND 
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
                        "Mqtt capture {} changed, disabled, or deleted, stopping...",
                        self.path
                    );
                    return None;
                }
            }
            Err(err) => {
                tracing::warn!(
                    "Error updating ping of capture mqtt {}: {:?}",
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
            Some(format!("mqtt-{}", self.get_trigger_path())),
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
                    trigger_kind = 'mqtt'
            "#,
            error,
            self.workspace_id,
            self.path,
            self.is_flow,
        )
        .execute(db)
        .await
        {
            tracing::error!(
                "Could not disable mqtt capture {} ({}) with err {}, disabling because of error {}",
                self.path,
                self.workspace_id,
                err,
                error
            );
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
            &TriggerKind::Mqtt,
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

async fn listen_to_unlistened_mqtt_events(
    db: &DB,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) {
    let mqtt_triggers = sqlx::query_as!(
        MqttTrigger,
        r#"
            SELECT
                mqtt_resource_path,
                subscribe_topics as "subscribe_topics!: Vec<SqlxJson<SubscribeTopic>>",
                v3_config as "v3_config!: Option<SqlxJson<MqttV3Config>>",
                v5_config as "v5_config!: Option<SqlxJson<MqttV5Config>>",
                client_version as "client_version: _",
                workspace_id,
                path,
                script_path,
                is_flow,
                edited_by,
                email,
                edited_at,
                server_id,
                last_server_ping,
                extra_perms,
                error,
                enabled
            FROM
                mqtt_trigger
            WHERE
                enabled IS TRUE
                AND (last_server_ping IS NULL OR
                    last_server_ping < now() - interval '15 seconds'
                )
            "#
    )
    .fetch_all(db)
    .await;

    match mqtt_triggers {
        Ok(mut triggers) => {
            triggers.shuffle(&mut rand::rng());
            for trigger in triggers {
                trigger
                    .try_to_listen_to_mqtt_messages(db.clone(), killpill_rx.resubscribe())
                    .await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching mqtt triggers: {:?}", err);
        }
    };

    let mqtt_triggers_capture = sqlx::query_as!(
        CaptureConfigForMqttTrigger,
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
                trigger_kind = 'mqtt' AND
                last_client_ping > NOW() - INTERVAL '10 seconds' AND
                trigger_config IS NOT NULL AND
                (last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')
            "#
    )
    .fetch_all(db)
    .await;

    match mqtt_triggers_capture {
        Ok(mut captures) => {
            captures.shuffle(&mut rand::rng());
            for capture in captures {
                capture
                    .try_to_listen_to_mqtt_messages(db.clone(), killpill_rx.resubscribe())
                    .await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching captures mqtt triggers: {:?}", err);
        }
    };
}

pub fn start_mqtt_consumer(db: DB, mut killpill_rx: tokio::sync::broadcast::Receiver<()>) {
    tokio::spawn(async move {
        listen_to_unlistened_mqtt_events(&db, &killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_mqtt_events(&db,  &killpill_rx).await
                }
            }
        }
    });
}
