/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(feature = "http_trigger")]
use {
    crate::http_trigger_args::{HttpMethod, RawHttpTriggerArgs},
    axum::response::{IntoResponse, Response},
    std::collections::HashMap,
};

#[cfg(all(feature = "enterprise", feature = "gcp_trigger"))]
use {
    crate::gcp_triggers_ee::{
        manage_google_subscription, process_google_push_request, validate_jwt_token,
        CreateUpdateConfig, SubscriptionMode,
    },
    axum::extract::Request,
    http::HeaderMap,
    utils::empty_as_none,
};

#[cfg(all(feature = "enterprise", feature = "sqs_trigger"))]
use windmill_common::auth::aws::AwsAuthResourceType;

#[cfg(any(
    feature = "http_trigger",
    all(feature = "enterprise", feature = "gcp_trigger")
))]
use {serde::de::DeserializeOwned, windmill_common::error::Error};

#[cfg(all(feature = "enterprise", feature = "kafka"))]
use crate::kafka_triggers_ee::KafkaTriggerConfigConnection;

#[cfg(feature = "mqtt_trigger")]
use crate::mqtt_triggers::{MqttClientVersion, MqttV3Config, MqttV5Config, SubscribeTopic};

#[cfg(all(feature = "enterprise", feature = "nats"))]
use crate::nats_triggers_ee::NatsTriggerConfigConnection;

#[cfg(feature = "postgres_trigger")]
use {
    crate::postgres_triggers::{
        create_logical_replication_slot_query, create_publication_query, drop_publication_query,
        generate_random_string, get_database_connection, PublicationData,
    },
    itertools::Itertools,
    pg_escape::quote_literal,
};

use crate::{
    args::RawWebhookArgs,
    db::{ApiAuthed, DB},
    trigger_helpers::{RunnableFormat, RunnableFormatVersion},
    users::fetch_api_authed,
    utils::RunnableKind,
};

use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, head, post},
    Json, Router,
};

use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::types::Json as SqlxJson;

use windmill_common::{
    db::UserDB,
    error::{JsonResult, Result},
    utils::{not_found_if_none, paginate, Pagination, StripPath},
    worker::{to_raw_value, CLOUD_HOSTED},
};

use windmill_queue::{PushArgs, PushArgsOwned, TriggerKind};

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
        .route(
            "/move/:runnable_kind/*path",
            post(move_captures_and_configs),
        )
        .route("/:id", delete(delete_capture))
        .route("/:id", get(get_capture))
}

pub fn workspaced_unauthed_service() -> Router {
    let router = Router::new().route(
        "/webhook/:runnable_kind/*path",
        head(|| async {}).post(webhook_payload),
    );

    #[cfg(any(
        feature = "http_trigger",
        all(feature = "enterprise", feature = "gcp_trigger")
    ))]
    {
        #[cfg(feature = "http_trigger")]
        let router = router.route("/http/:runnable_kind/:path/*route_path", {
            head(|| async {}).fallback(http_payload)
        });

        #[cfg(all(feature = "enterprise", feature = "gcp_trigger"))]
        let router = router.route("/gcp/:runnable_kind/*path", post(gcp_payload));

        router
    }

    #[cfg(not(any(
        feature = "http_trigger",
        all(feature = "enterprise", feature = "gcp_trigger")
    )))]
    {
        router
    }
}

#[cfg(feature = "http_trigger")]
#[derive(Serialize, Deserialize)]
struct HttpTriggerConfig {
    route_path: String,
    http_method: HttpMethod,
    raw_string: Option<bool>,
    wrap_body: Option<bool>,
}

#[cfg(all(feature = "enterprise", feature = "kafka"))]
#[derive(Serialize, Deserialize)]
pub struct KafkaTriggerConfig {
    #[serde(flatten)]
    pub connection: KafkaTriggerConfigConnection,
    pub topics: Vec<String>,
    pub group_id: String,
}

#[cfg(all(feature = "enterprise", feature = "sqs_trigger"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct SqsTriggerConfig {
    pub queue_url: String,
    pub aws_resource_path: String,
    pub message_attributes: Option<Vec<String>>,
    pub aws_auth_resource_type: AwsAuthResourceType,
}

#[cfg(all(feature = "enterprise", feature = "gcp_trigger"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct GcpTriggerConfig {
    pub gcp_resource_path: String,
    pub subscription_mode: SubscriptionMode,
    #[serde(default, deserialize_with = "empty_as_none")]
    pub subscription_id: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none")]
    pub base_endpoint: Option<String>,
    #[serde(flatten)]
    pub create_update: Option<CreateUpdateConfig>,
    pub topic_id: String,
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
    pub mqtt_resource_path: String,
    pub subscribe_topics: Vec<SubscribeTopic>,
    pub v3_config: Option<MqttV3Config>,
    pub v5_config: Option<MqttV5Config>,
    pub client_version: Option<MqttClientVersion>,
    pub client_id: Option<String>,
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
    #[cfg(all(feature = "enterprise", feature = "sqs_trigger"))]
    Sqs(SqsTriggerConfig),
    #[cfg(all(feature = "enterprise", feature = "kafka"))]
    Kafka(KafkaTriggerConfig),
    #[cfg(all(feature = "enterprise", feature = "nats"))]
    Nats(NatsTriggerConfig),
    #[cfg(feature = "mqtt_trigger")]
    Mqtt(MqttTriggerConfig),
    #[cfg(all(feature = "enterprise", feature = "gcp_trigger"))]
    Gcp(GcpTriggerConfig),
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
        r#"
        SELECT 
            trigger_config AS "trigger_config: _", 
            trigger_kind AS "trigger_kind: _", 
            error, 
            last_server_ping
        FROM 
            capture_config
        WHERE 
            workspace_id = $1 
            AND path = $2 
            AND is_flow = $3
        "#,
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

#[inline]
#[cfg(not(feature = "postgres_trigger"))]
async fn set_postgres_trigger_config(
    _w_id: &str,
    _authed: ApiAuthed,
    _db: &DB,
    _user_db: UserDB,
    capture_config: NewCaptureConfig,
) -> Result<NewCaptureConfig> {
    Ok(capture_config)
}

#[cfg(all(feature = "enterprise", feature = "gcp_trigger"))]
async fn set_gcp_trigger_config(
    w_id: &str,
    authed: ApiAuthed,
    db: &DB,
    mut capture_config: NewCaptureConfig,
) -> Result<NewCaptureConfig> {
    let Some(TriggerConfig::Gcp(mut gcp_config)) = capture_config.trigger_config else {
        return Err(windmill_common::error::Error::BadRequest(
            "Invalid GCP Pub/Sub config".to_string(),
        ));
    };

    let config = manage_google_subscription(
        authed,
        db,
        w_id,
        &gcp_config.gcp_resource_path,
        &capture_config.path,
        &gcp_config.topic_id,
        &mut gcp_config.subscription_id,
        &mut gcp_config.base_endpoint,
        gcp_config.subscription_mode,
        gcp_config.create_update,
        false,
        capture_config.is_flow,
    )
    .await?;
    gcp_config.create_update = Some(config);
    gcp_config.subscription_mode = SubscriptionMode::CreateUpdate;
    capture_config.trigger_config = Some(TriggerConfig::Gcp(gcp_config));

    Ok(capture_config)
}

#[inline]
#[cfg(not(all(feature = "enterprise", feature = "gcp_trigger")))]
async fn set_gcp_trigger_config(
    _w_id: &str,
    _authed: ApiAuthed,
    _db: &DB,
    capture_config: NewCaptureConfig,
) -> Result<NewCaptureConfig> {
    Ok(capture_config)
}

async fn set_config(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(nc): Json<NewCaptureConfig>,
) -> JsonResult<Option<TriggerConfig>> {
    let nc = match nc.trigger_kind {
        TriggerKind::Postgres => {
            set_postgres_trigger_config(&w_id, authed.clone(), &db, user_db.clone(), nc).await?
        }
        TriggerKind::Gcp => set_gcp_trigger_config(&w_id, authed.clone(), &db, nc).await?,
        _ => nc,
    };

    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        r#"
        INSERT INTO capture_config (
            workspace_id, path, is_flow, trigger_kind, trigger_config, owner, email
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7
        )
        ON CONFLICT (workspace_id, path, is_flow, trigger_kind)
        DO UPDATE 
        SET 
            trigger_config = $5, 
            owner = $6, 
            email = $7, 
            server_id = NULL, 
            error = NULL
        "#,
        &w_id,
        &nc.path,
        nc.is_flow,
        nc.trigger_kind as TriggerKind,
        nc.trigger_config
            .as_ref()
            .map(|x| SqlxJson(to_raw_value(&x))) as Option<SqlxJson<Box<RawValue>>>,
        &authed.username,
        &authed.email,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(nc.trigger_config))
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
        r#"
        UPDATE 
            capture_config
        SET 
            last_client_ping = NOW()
        WHERE 
            workspace_id = $1 
            AND path = $2 
            AND is_flow = $3 
            AND trigger_kind = $4
        "#,
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
    main_args: SqlxJson<Box<serde_json::value::RawValue>>,
    preprocessor_args: Option<SqlxJson<Box<serde_json::value::RawValue>>>,
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
        r#"
        SELECT 
            id, 
            created_at, 
            trigger_kind AS "trigger_kind: _",
            CASE 
                WHEN pg_column_size(main_args) < 40000 THEN main_args 
                ELSE '"WINDMILL_TOO_BIG"'::jsonb 
            END AS "main_args!: _",
            CASE
                WHEN pg_column_size(preprocessor_args) < 40000 THEN preprocessor_args
                ELSE '"WINDMILL_TOO_BIG"'::jsonb
            END AS "preprocessor_args: _"
        FROM 
            capture
        WHERE 
            workspace_id = $1 
            AND path = $2 
            AND is_flow = $3 
            AND ($4::trigger_kind IS NULL OR trigger_kind = $4)
        ORDER BY 
            created_at DESC
        OFFSET $5
        LIMIT $6
        "#,
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
        r#"
        SELECT 
            id, 
            created_at, 
            trigger_kind AS "trigger_kind: _", 
            main_args AS "main_args!: _", 
            preprocessor_args AS "preprocessor_args: _"
        FROM 
            capture
        WHERE 
            id = $1 
            AND workspace_id = $2
        "#,
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
    sqlx::query!(
        r#"
        DELETE FROM 
            capture
        WHERE 
            id = $1
        "#,
        id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Deserialize)]
struct MoveCapturesAndConfigsBody {
    new_path: String,
}

async fn move_captures_and_configs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, runnable_kind, old_path)): Path<(String, RunnableKind, StripPath)>,
    Json(body): Json<MoveCapturesAndConfigsBody>,
) -> Result<()> {
    let mut tx = user_db.begin(&authed).await?;
    let old_path = old_path.to_path();

    sqlx::query!(
        r#"
        UPDATE 
            capture_config
        SET 
            path = $1
        WHERE 
            path = $2 
            AND workspace_id = $3 
            AND is_flow = $4
        "#,
        body.new_path,
        old_path,
        &w_id,
        matches!(runnable_kind, RunnableKind::Flow),
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        UPDATE 
            capture
        SET 
            path = $1
        WHERE 
            path = $2 
            AND workspace_id = $3 
            AND is_flow = $4
        "#,
        body.new_path,
        old_path,
        &w_id,
        matches!(runnable_kind, RunnableKind::Flow),
    )
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
        r#"
        SELECT 
            owner, 
            email
        FROM 
            capture_config
        WHERE 
            workspace_id = $1 
            AND path = $2 
            AND is_flow = $3 
            AND trigger_kind = $4 
            AND last_client_ping > NOW() - INTERVAL '10 seconds'
        "#,
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

#[cfg(any(
    feature = "http_trigger",
    all(feature = "enterprise", feature = "gcp_trigger")
))]
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
        r#"
        SELECT 
            trigger_config AS "trigger_config: _", 
            owner, 
            email
        FROM 
            capture_config
        WHERE 
            workspace_id = $1
            AND path = $2
            AND is_flow = $3
            AND trigger_kind = $4
            AND last_client_ping > NOW() - INTERVAL '10 seconds'
            AND (
                $5::bool IS FALSE
                OR (
                    trigger_config IS NOT NULL
                    AND trigger_config ->> 'delivery_type' = 'push'
                )
            )
        "#,
        &w_id,
        &path,
        is_flow,
        kind as &TriggerKind,
        matches!(kind, TriggerKind::Gcp)
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
            r#"
        DELETE FROM 
            capture
        WHERE 
            workspace_id = $1
            AND created_at <= (
                SELECT 
                    created_at
                FROM 
                    capture
                WHERE 
                    workspace_id = $1
                ORDER BY 
                    created_at DESC
                OFFSET $2
                LIMIT 1
            )
        "#,
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
    main_args: PushArgsOwned,
    preprocessor_args: PushArgsOwned,
    owner: &str,
) -> Result<()> {
    sqlx::query!(
        r#"
    INSERT INTO 
        capture (
            workspace_id, path, is_flow, trigger_kind, main_args, preprocessor_args, created_by
        )
    VALUES (
        $1, $2, $3, $4, $5, $6, $7
    )
    "#,
        &w_id,
        path,
        is_flow,
        trigger_kind as &TriggerKind,
        SqlxJson(PushArgs { args: &main_args.args, extra: main_args.extra }) as SqlxJson<PushArgs>,
        SqlxJson(PushArgs { args: &preprocessor_args.args, extra: preprocessor_args.extra })
            as SqlxJson<PushArgs>,
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
    args: RawWebhookArgs,
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

    let args = args.process_args(&authed, &db, &w_id, None).await?;

    let preprocessor_args = args.clone().to_args_from_format(RunnableFormat {
        has_preprocessor: true,
        version: RunnableFormatVersion::V2,
    })?;

    let main_args = args.to_main_args()?;

    insert_capture_payload(
        &db,
        &w_id,
        &path.to_path(),
        matches!(runnable_kind, RunnableKind::Flow),
        &TriggerKind::Webhook,
        main_args,
        preprocessor_args,
        &owner,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(all(feature = "enterprise", feature = "gcp_trigger"))]
async fn gcp_payload(
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, runnable_kind, path)): Path<(String, RunnableKind, String)>,
    headers: HeaderMap,
    request: Request,
) -> Result<StatusCode> {
    use crate::{gcp_triggers_ee::GcpTrigger, trigger_helpers::TriggerJobArgs};

    let is_flow = matches!(runnable_kind, RunnableKind::Flow);
    let (gcp_trigger_config, owner, email): (GcpTriggerConfig, _, _) =
        get_capture_trigger_config_and_owner(&db, &w_id, &path, is_flow, &TriggerKind::Gcp).await?;

    let authed = fetch_api_authed(owner.clone(), email, &w_id, &db, None).await?;

    let Some(config) = &gcp_trigger_config.create_update else {
        return Err(Error::BadConfig("Bad config".to_string()));
    };

    validate_jwt_token(
        &db,
        user_db.clone(),
        authed.clone(),
        &headers,
        &gcp_trigger_config.gcp_resource_path,
        &w_id,
        config.delivery_config.as_ref().unwrap(),
    )
    .await?;

    let (payload, gcp) = process_google_push_request(headers, request).await?;

    let (main_args, preprocessor_args) = GcpTrigger::build_capture_payloads(payload, gcp);

    let _ = insert_capture_payload(
        &db,
        &w_id,
        &path,
        is_flow,
        &TriggerKind::Gcp,
        main_args,
        preprocessor_args,
        &owner,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(feature = "http_trigger")]
async fn http_payload(
    Extension(db): Extension<DB>,
    Path((w_id, runnable_kind, path, route_path)): Path<(String, RunnableKind, String, StripPath)>,
    args: RawHttpTriggerArgs,
) -> std::result::Result<StatusCode, Response> {
    let path = path.replace(".", "/");
    let is_flow = matches!(runnable_kind, RunnableKind::Flow);
    let route_path = route_path.to_path();
    let (http_trigger_config, owner, email): (HttpTriggerConfig, _, _) =
        get_capture_trigger_config_and_owner(&db, &w_id, &path, is_flow, &TriggerKind::Http)
            .await
            .map_err(|e| e.into_response())?;

    let authed = fetch_api_authed(owner.clone(), email, &w_id, &db, None)
        .await
        .map_err(|e| e.into_response())?;

    let args = args
        .process_args(
            &authed,
            &db,
            &w_id,
            http_trigger_config.raw_string.unwrap_or(false),
        )
        .await
        .map_err(|e| e.into_response())?;

    let mut router = matchit::Router::new();
    router.insert(&http_trigger_config.route_path, ()).ok();
    let match_ = router.at(route_path).ok();

    let match_ = not_found_if_none(match_, "capture http trigger", &route_path)
        .map_err(|e| e.into_response())?;

    let matchit::Match { params, .. } = match_;

    let params: HashMap<String, String> = params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let preprocessor_args = args
        .clone()
        .to_v2_preprocessor_args(&http_trigger_config.route_path, &route_path, &params)
        .map_err(|e| e.into_response())?;

    let main_args = args
        .to_main_args(http_trigger_config.wrap_body.unwrap_or(false))
        .map_err(|e| e.into_response())?;

    insert_capture_payload(
        &db,
        &w_id,
        &path,
        is_flow,
        &TriggerKind::Http,
        main_args,
        preprocessor_args,
        &owner,
    )
    .await
    .map_err(|e| e.into_response())?;

    Ok(StatusCode::NO_CONTENT)
}
