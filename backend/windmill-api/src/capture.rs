/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(feature = "http_trigger")]
use http::HeaderMap;
#[cfg(feature = "http_trigger")]
use serde::de::DeserializeOwned;
#[cfg(feature = "http_trigger")]
use std::collections::HashMap;
#[cfg(feature = "http_trigger")]
use windmill_common::error::Error;
#[cfg(feature = "http_trigger")]
use crate::http_triggers::{build_http_trigger_extra, HttpMethod};
#[cfg(all(feature = "enterprise", feature = "kafka"))]
use crate::kafka_triggers_ee::KafkaTriggerConfigConnection;
#[cfg(all(feature = "enterprise", feature = "nats"))]
use crate::nats_triggers_ee::NatsTriggerConfigConnection;
#[cfg(feature = "postgres_trigger")]
use crate::postgres_triggers::{
    create_logical_replication_slot_query, create_publication_query, drop_publication_query,
    generate_random_string, get_database_connection, PublicationData,
};
#[cfg(feature = "postgres_trigger")]
use itertools::Itertools;
#[cfg(feature = "postgres_trigger")]
use pg_escape::quote_literal;

use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, head, post},
    Json, Router,
};
use hyper::StatusCode;
use std::fmt;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::types::Json as SqlxJson;
use windmill_common::{
    db::UserDB,
    error::{JsonResult, Result},
    utils::{not_found_if_none, paginate, Pagination, StripPath},
    worker::{to_raw_value, CLOUD_HOSTED},
};
use windmill_queue::{PushArgs, PushArgsOwned};
use crate::{
    args::WebhookArgs,
    db::{ApiAuthed, DB},
    users::fetch_api_authed,
};

const KEEP_LAST: i64 = 20;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/set_config", post(set_config))
        .route(
            "/ping_config/:trigger_kind/:runnable_kind/*path",
            post(ping_config),
        )
        .route("/get_configs/:runnable_kind/*path", get(get_configs))
        .route("/list/:runnable_kind/*path", get(list_captures))
        .route("/:id", delete(delete_capture))
        .route("/:id", get(get_capture))
}

pub fn workspaced_unauthed_service() -> Router {
    let router = Router::new().route(
        "/webhook/:runnable_kind/*path",
        head(|| async {}).post(webhook_payload),
    );

    #[cfg(feature = "http_trigger")]
    {
        router.route("/http/:runnable_kind/:path/*route_path", {
            head(|| async {}).fallback(http_payload)
        })
    }

    #[cfg(not(feature = "http_trigger"))]
    {
        router
    }
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug)]
#[sqlx(type_name = "TRIGGER_KIND", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TriggerKind {
    Webhook,
    Http,
    Websocket,
    Kafka,
    Email,
    Nats,
    Mqtt,
    Postgres
}

impl fmt::Display for TriggerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TriggerKind::Webhook => "webhook",
            TriggerKind::Http => "http",
            TriggerKind::Websocket => "websocket",
            TriggerKind::Kafka => "kafka",
            TriggerKind::Email => "email",
            TriggerKind::Nats => "nats",
            TriggerKind::Mqtt => "mqtt",
            TriggerKind::Postgres => "postgres",
        };
        write!(f, "{}", s)
    }
}

#[cfg(feature = "http_trigger")]
#[derive(Serialize, Deserialize)]
struct HttpTriggerConfig {
    route_path: String,
    http_method: HttpMethod,
}

#[cfg(all(feature = "enterprise", feature = "kafka"))]
#[derive(Serialize, Deserialize)]
pub struct KafkaTriggerConfig {
    #[serde(flatten)]
    pub connection: KafkaTriggerConfigConnection,
    pub topics: Vec<String>,
    pub group_id: String,
}

#[cfg(all(feature = "enterprise", feature = "nats"))]
#[derive(Serialize, Deserialize)]
pub struct NatsTriggerConfig {
    #[serde(flatten)]
    pub connection: NatsTriggerConfigConnection,
    pub subjects: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumer_name: Option<String>,
    pub use_jetstream: bool,
}

#[cfg(feature = "mqtt_trigger")]
#[derive(Debug, Serialize, Deserialize)]
pub struct MqttTriggerConfig {
    topics: Vec<String>
}
#[cfg(feature = "postgres_trigger")]
#[derive(Serialize, Deserialize, Debug)]
pub struct PostgresTriggerConfig {
    pub postgres_resource_path: String,
    pub publication_name: Option<String>,
    pub replication_slot_name: Option<String>,
    pub publication: PublicationData,
}

#[cfg(feature = "websocket")]
#[derive(Serialize, Deserialize, Debug)]
pub struct WebsocketTriggerConfig {
    pub url: String,
    // have to use Value because RawValue is not supported inside untagged
    pub url_runnable_args: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum TriggerConfig {
    #[cfg(feature = "http_trigger")]
    Http(HttpTriggerConfig),
    #[cfg(feature = "postgres_trigger")]
    Postgres(PostgresTriggerConfig),
    #[cfg(feature = "websocket")]
    Websocket(WebsocketTriggerConfig),
    #[cfg(all(feature = "enterprise", feature = "kafka"))]
    Kafka(KafkaTriggerConfig),
    #[cfg(all(feature = "enterprise", feature = "nats"))]
    Nats(NatsTriggerConfig),
    #[cfg(feature = "mqtt_trigger")]
    Mqtt(MqttTriggerConfig)
}

#[derive(Serialize, Deserialize)]
struct NewCaptureConfig {
    trigger_kind: TriggerKind,
    path: String,
    is_flow: bool,
    trigger_config: Option<TriggerConfig>,
}

#[derive(Serialize, Deserialize)]
struct CaptureConfig {
    trigger_config: Option<SqlxJson<Box<RawValue>>>,
    trigger_kind: TriggerKind,
    error: Option<String>,
    last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
}

async fn get_configs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, runnable_kind, path)): Path<(String, RunnableKind, StripPath)>,
) -> JsonResult<Vec<CaptureConfig>> {
    let mut tx = user_db.begin(&authed).await?;

    let configs = sqlx::query_as!(
        CaptureConfig,
        r#"SELECT trigger_config as "trigger_config: _", trigger_kind as "trigger_kind: _", error, last_server_ping
        FROM capture_config
        WHERE workspace_id = $1 AND path = $2 AND is_flow = $3"#,
        &w_id,
        &path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(configs))
}

#[cfg(feature = "postgres_trigger")]
async fn set_postgres_trigger_config(
    w_id: &str,
    authed: ApiAuthed,
    db: &DB,
    user_db: UserDB,
    mut capture_config: NewCaptureConfig,
) -> Result<NewCaptureConfig> {
    let Some(TriggerConfig::Postgres(mut postgres_config)) = capture_config.trigger_config else {
        return Err(windmill_common::error::Error::BadRequest(
            "Invalid postgres config".to_string(),
        ));
    };

    let mut connection = get_database_connection(
        authed,
        Some(user_db),
        &db,
        &postgres_config.postgres_resource_path,
        &w_id,
    )
    .await?;

    let publication_name = postgres_config
        .publication_name
        .get_or_insert(format!("windmill_capture_{}", generate_random_string()));
    let replication_slot_name = postgres_config
        .replication_slot_name
        .get_or_insert(publication_name.clone());

    let query = drop_publication_query(&publication_name);

    sqlx::query(&query).execute(&mut connection).await?;

    let query = create_publication_query(
        &publication_name,
        postgres_config.publication.table_to_track.as_deref(),
        &postgres_config
            .publication
            .transaction_to_track
            .iter()
            .map(AsRef::as_ref)
            .collect_vec(),
    );

    sqlx::query(&query).execute(&mut connection).await?;

    let query = format!(
        "SELECT 1 from pg_replication_slots WHERE slot_name = {}",
        quote_literal(replication_slot_name)
    );

    let row = sqlx::query(&query).fetch_optional(&mut connection).await?;

    if row.is_none() {
        let query = create_logical_replication_slot_query(&replication_slot_name);
        sqlx::query(&query).execute(&mut connection).await?;
    }
    capture_config.trigger_config = Some(TriggerConfig::Postgres(postgres_config));
    Ok(capture_config)
}

async fn set_config(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    #[cfg(feature = "postgres_trigger")] Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(nc): Json<NewCaptureConfig>,
) -> Result<()> {
    #[cfg(feature = "postgres_trigger")]
    let nc = if let TriggerKind::Postgres = nc.trigger_kind {
        set_postgres_trigger_config(&w_id, authed.clone(), &db, user_db.clone(), nc).await?
    }
    else {
        nc
    };

    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "INSERT INTO capture_config
            (workspace_id, path, is_flow, trigger_kind, trigger_config, owner, email)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (workspace_id, path, is_flow, trigger_kind)
            DO UPDATE SET trigger_config = $5, owner = $6, email = $7, server_id = NULL, error = NULL",
        &w_id,
        &nc.path,
        nc.is_flow,
        nc.trigger_kind as TriggerKind,
        nc.trigger_config.map(|x| SqlxJson(to_raw_value(&x))) as Option<SqlxJson<Box<RawValue>>>,
        &authed.username,
        &authed.email,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

async fn ping_config(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, trigger_kind, runnable_kind, path)): Path<(
        String,
        TriggerKind,
        RunnableKind,
        StripPath,
    )>,
) -> Result<()> {
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "UPDATE capture_config SET last_client_ping = now() WHERE workspace_id = $1 AND path = $2 AND is_flow = $3 AND trigger_kind = $4",
        &w_id,
        &path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
        trigger_kind as TriggerKind,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Capture {
    id: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    trigger_kind: TriggerKind,
    payload: SqlxJson<Box<serde_json::value::RawValue>>,
    trigger_extra: Option<SqlxJson<Box<serde_json::value::RawValue>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RunnableKind {
    Script,
    Flow,
}

#[derive(Deserialize)]
struct ListCapturesQuery {
    trigger_kind: Option<TriggerKind>,
    page: Option<usize>,
    per_page: Option<usize>,
}

async fn list_captures(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, runnable_kind, path)): Path<(String, RunnableKind, StripPath)>,
    Query(query): Query<ListCapturesQuery>,
) -> JsonResult<Vec<Capture>> {
    let mut tx = user_db.begin(&authed).await?;

    let (per_page, offset) = paginate(Pagination { page: query.page, per_page: query.per_page });

    let captures = sqlx::query_as!(
        Capture,
        r#"SELECT id, created_at, trigger_kind as "trigger_kind: _", CASE WHEN pg_column_size(payload) < 40000 THEN payload ELSE '"WINDMILL_TOO_BIG"'::jsonb END as "payload!: _", trigger_extra as "trigger_extra: _"
        FROM capture
        WHERE workspace_id = $1
            AND path = $2 AND is_flow = $3
            AND ($4::trigger_kind IS NULL OR trigger_kind = $4)
        ORDER BY created_at DESC
        OFFSET $5
        LIMIT $6"#,
        &w_id,
        &path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
        query.trigger_kind as Option<TriggerKind>,
        offset as i64,
        per_page as i64,
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(captures))
}

async fn get_capture(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, i64)>,
) -> JsonResult<Capture> {
    let mut tx = user_db.begin(&authed).await?;
    let capture = sqlx::query_as!(
        Capture,
        r#"SELECT id, created_at, trigger_kind as "trigger_kind: _", payload as "payload!: _", trigger_extra as "trigger_extra: _" FROM capture WHERE id = $1 AND workspace_id = $2"#,
        id,
        &w_id,
    )
    .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(capture))
}

async fn delete_capture(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((_, id)): Path<(String, i64)>,
) -> Result<()> {
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!("DELETE FROM capture WHERE id = $1", id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct ActiveCaptureOwner {
    owner: String,
    email: String,
}

pub async fn get_active_capture_owner_and_email(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
    kind: &TriggerKind,
) -> Result<(String, String)> {
    let capture_config = sqlx::query_as!(
        ActiveCaptureOwner,
        "SELECT owner, email
        FROM capture_config
        WHERE workspace_id = $1 AND path = $2 AND is_flow = $3 AND trigger_kind = $4 AND last_client_ping > NOW() - INTERVAL '10 seconds'",
        &w_id,
        &path,
        is_flow,
        kind as &TriggerKind,
    )
    .fetch_optional(db)
    .await?;

    let capture_config = not_found_if_none(
        capture_config,
        &format!("capture config for {} trigger", kind),
        path,
    )?;

    Ok((capture_config.owner, capture_config.email))
}

#[cfg(feature = "http_trigger")]
async fn get_capture_trigger_config_and_owner<T: DeserializeOwned>(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
    kind: &TriggerKind,
) -> Result<(T, String, String)> {
    #[derive(Deserialize)]
    struct CaptureTriggerConfigAndOwner {
        trigger_config: Option<SqlxJson<Box<RawValue>>>,
        owner: String,
        email: String,
    }

    let capture_config = sqlx::query_as!(
        CaptureTriggerConfigAndOwner,
        r#"SELECT trigger_config as "trigger_config: _", owner, email
        FROM capture_config
        WHERE workspace_id = $1 AND path = $2 AND is_flow = $3 AND trigger_kind = $4 AND last_client_ping > NOW() - INTERVAL '10 seconds'"#,
        &w_id,
        &path,
        is_flow,
        kind as &TriggerKind,
    )
    .fetch_optional(db)
    .await?;

    let capture_config = not_found_if_none(
        capture_config,
        &format!("capture config for {} trigger", kind),
        path,
    )?;

    let trigger_config = not_found_if_none(
        capture_config.trigger_config,
        &format!("capture {} trigger config", kind),
        path,
    )?;

    Ok((
        serde_json::from_str(trigger_config.get()).map_err(|e| {
            Error::internal_err(format!(
                "error parsing capture config for {} trigger: {}",
                kind, e
            ))
        })?,
        capture_config.owner,
        capture_config.email,
    ))
}

async fn clear_captures_history(db: &DB, w_id: &str) -> Result<()> {
    if *CLOUD_HOSTED {
        /* Retain only KEEP_LAST most recent captures in this workspace. */
        sqlx::query!(
            "DELETE FROM capture
            WHERE workspace_id = $1
                AND created_at <=
                    (
                        SELECT created_at
                            FROM capture
                            WHERE workspace_id = $1
                        ORDER BY created_at DESC
                            OFFSET $2
                            LIMIT 1
                    )",
            &w_id,
            KEEP_LAST,
        )
        .execute(db)
        .await?;
    }
    Ok(())
}

pub async fn insert_capture_payload(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
    trigger_kind: &TriggerKind,
    payload: PushArgsOwned,
    trigger_extra: Option<Box<RawValue>>,
    owner: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO capture (workspace_id, path, is_flow, trigger_kind, payload, trigger_extra, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7)",
        &w_id,
        path,
        is_flow,
        trigger_kind as &TriggerKind,
        SqlxJson(to_raw_value(&PushArgs {
            args: &payload.args,
            extra: payload.extra
        })) as SqlxJson<Box<RawValue>>,
        trigger_extra.map(SqlxJson) as Option<SqlxJson<Box<RawValue>>>,
        owner,
    )
    .execute(db)
    .await?;

    clear_captures_history(db, &w_id).await?;

    Ok(())
}

async fn webhook_payload(
    Extension(db): Extension<DB>,
    Path((w_id, runnable_kind, path)): Path<(String, RunnableKind, StripPath)>,
    args: WebhookArgs,
) -> Result<StatusCode> {
    let (owner, email) = get_active_capture_owner_and_email(
        &db,
        &w_id,
        &path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
        &TriggerKind::Webhook,
    )
    .await?;

    let authed = fetch_api_authed(owner.clone(), email, &w_id, &db, None).await?;
    let args = args.to_push_args_owned(&authed, &db, &w_id).await?;

    insert_capture_payload(
        &db,
        &w_id,
        &path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
        &TriggerKind::Webhook,
        args,
        Some(to_raw_value(&serde_json::json!({
            "wm_trigger": {
                "kind": "webhook",
            }
        }))),
        &owner,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(feature = "http_trigger")]
async fn http_payload(
    Extension(db): Extension<DB>,
    Path((w_id, kind, path, route_path)): Path<(String, RunnableKind, String, StripPath)>,
    Query(query): Query<HashMap<String, String>>,
    method: http::Method,
    headers: HeaderMap,
    args: WebhookArgs,
) -> Result<StatusCode> {
    let route_path = route_path.to_path();
    let path = path.replace(".", "/");

    let (http_trigger_config, owner, email): (HttpTriggerConfig, _, _) =
        get_capture_trigger_config_and_owner(
            &db,
            &w_id,
            &path,
            matches!(kind, RunnableKind::Flow),
            &TriggerKind::Http,
        )
        .await?;

    let authed = fetch_api_authed(owner.clone(), email, &w_id, &db, None).await?;
    let args = args.to_push_args_owned(&authed, &db, &w_id).await?;

    let mut router = matchit::Router::new();
    router.insert(&http_trigger_config.route_path, ()).ok();
    let match_ = router.at(route_path).ok();

    let match_ = not_found_if_none(match_, "capture http trigger", &route_path)?;

    let matchit::Match { params, .. } = match_;

    let params: HashMap<String, String> = params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let extra: HashMap<String, Box<RawValue>> = HashMap::from_iter(vec![(
        "wm_trigger".to_string(),
        build_http_trigger_extra(
            &http_trigger_config.route_path,
            route_path,
            &method,
            &params,
            &query,
            &headers,
        )
        .await,
    )]);

    insert_capture_payload(
        &db,
        &w_id,
        &path,
        matches!(kind, RunnableKind::Flow),
        &TriggerKind::Http,
        args,
        Some(to_raw_value(&extra)),
        &owner,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
