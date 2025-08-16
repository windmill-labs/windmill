use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use std::sync::Arc;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult, Result},
    utils::StripPath,
    worker::CLOUD_HOSTED,
    DB,
};
use windmill_git_sync::handle_deployment_metadata;

use crate::{
    db::ApiAuthed,
    triggers::{CreateTrigger, EditTrigger, StandardTriggerQuery, TriggerCrud},
    utils::check_scopes,
};

#[cfg(all(feature = "gcp_trigger", feature = "enterprise"))]
use crate::triggers::gcp::handler_oss::GcpTriggerHandler;
#[cfg(feature = "http_trigger")]
use crate::triggers::http::handler::HttpTriggerHandler;
#[cfg(all(feature = "kafka", feature = "enterprise"))]
use crate::triggers::kafka::handler_oss::KafkaTriggerHandler;
#[cfg(feature = "mqtt_trigger")]
use crate::triggers::mqtt::handler::MqttTriggerHandler;
#[cfg(all(feature = "nats", feature = "enterprise"))]
use crate::triggers::nats::handler_oss::NatsTriggerHandler;
#[cfg(feature = "postgres_trigger")]
use crate::triggers::postgres::handler::PostgresTriggerHandler;
#[cfg(all(feature = "sqs_trigger", feature = "enterprise"))]
use crate::triggers::sqs::handler_oss::SqsTriggerHandler;
#[cfg(feature = "websocket")]
use crate::triggers::websocket::handler::WebsocketTriggerHandler;

pub fn trigger_routes<T: TriggerCrud + 'static>() -> Router {
    let mut router = Router::new()
        .route("/create", post(create_trigger::<T>))
        .route("/list", get(list_triggers::<T>))
        .route("/get/*path", get(get_trigger::<T>))
        .route("/update/*path", post(update_trigger::<T>))
        .route("/delete/*path", delete(delete_trigger::<T>))
        .route("/exists/*path", get(exists_trigger::<T>));

    if T::SUPPORTS_ENABLED {
        router = router.route("/setenabled/*path", post(set_enabled_trigger::<T>));
    }

    if T::SUPPORTS_TEST_CONNECTION {
        router = router.route("/test", post(test_connection::<T>));
    }

    router
}

async fn create_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    raw_body: String,
) -> Result<(StatusCode, String)> {
    // Log the raw JSON payload
    tracing::info!(
        "Creating {} trigger with raw JSON payload: {}",
        T::TRIGGER_TYPE,
        raw_body
    );

    // Manually deserialize to get better error context
    let new_trigger: CreateTrigger<T::NewTriggerConfig> =
        serde_json::from_str(&raw_body).map_err(|e| {
            tracing::error!(
                "Failed to deserialize {} trigger JSON: {} - Raw payload: {}",
                T::TRIGGER_TYPE,
                e,
                raw_body
            );
            error::Error::BadRequest(format!("Failed to deserialize JSON: {}", e))
        })?;
    check_scopes(&authed, || {
        format!(
            "{}:write:{}",
            T::scope_domain_name(),
            &new_trigger.base.path
        )
    })?;

    if *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(format!(
            "{} triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host",
            T::TRIGGER_TYPE
        )));
    }

    handler
        .validate_new(&workspace_id, &new_trigger.config)
        .await?;

    let mut tx = user_db.begin(&authed).await?;

    let new_path = new_trigger.base.path.clone();

    handler
        .create_trigger(&db, &mut *tx, &authed, &workspace_id, new_trigger)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("{}_triggers.create", T::TRIGGER_TYPE),
        ActionKind::Create,
        &workspace_id,
        Some(&new_path),
        None,
    )
    .await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        T::get_deployed_object(new_path.clone()),
        Some(format!("{} '{}' created", T::DEPLOYMENT_NAME, new_path)),
        true,
    )
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, new_path))
}

async fn list_triggers<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Query(query): Query<StandardTriggerQuery>,
) -> JsonResult<Vec<T::Trigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let triggers = handler
        .list_triggers(&mut *tx, &workspace_id, Some(&query))
        .await?;
    tx.commit().await?;

    Ok(Json(triggers))
}

async fn get_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
) -> JsonResult<T::Trigger> {
    let path = path.to_path();
    check_scopes(&authed, || {
        format!("{}:read:{}", T::scope_domain_name(), &path)
    })?;

    let mut tx = user_db.begin(&authed).await?;
    let trigger = handler
        .get_trigger_by_path(&mut *tx, &workspace_id, path)
        .await?;

    tx.commit().await?;

    Ok(Json(trigger))
}

async fn update_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
    Json(edit_trigger): Json<EditTrigger<T::EditTriggerConfig>>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || {
        format!(
            "{}:write:{}",
            T::scope_domain_name(),
            &edit_trigger.base.path
        )
    })?;

    handler
        .validate_edit(&workspace_id, path, &edit_trigger.config)
        .await?;

    let mut tx = user_db.begin(&authed).await?;

    let new_path = edit_trigger.base.path.to_string();
    handler
        .update_trigger(&db, &mut *tx, &authed, &workspace_id, path, edit_trigger)
        .await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("{}_triggers.update", T::TRIGGER_TYPE),
        ActionKind::Update,
        &workspace_id,
        Some(&new_path),
        None,
    )
    .await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        T::get_deployed_object(new_path.clone()),
        Some(format!("{} '{}' updated", T::DEPLOYMENT_NAME, new_path)),
        true,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Trigger '{}' updated", path))
}

async fn delete_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || {
        format!("{}:write:{}", T::scope_domain_name(), &path)
    })?;

    let mut tx = user_db.begin(&authed).await?;
    let deleted = handler
        .delete_by_path(&mut *tx, &workspace_id, path)
        .await?;

    if !deleted {
        return Err(error::Error::NotFound(format!(
            "Trigger not found at path: {}",
            path
        )));
    }

    audit_log(
        &mut *tx,
        &authed,
        &format!("{}_triggers.delete", T::TRIGGER_TYPE),
        ActionKind::Delete,
        &workspace_id,
        Some(&path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Trigger '{}' deleted", path))
}

async fn exists_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    check_scopes(&authed, || {
        format!("{}:read:{}", T::scope_domain_name(), path)
    })?;
    let mut tx = user_db.begin(&authed).await?;
    let exists = handler.exists(&mut *tx, &workspace_id, path).await?;
    tx.commit().await?;

    Ok(Json(exists))
}

#[derive(serde::Deserialize)]
struct SetEnabledPayload {
    enabled: bool,
}

async fn set_enabled_trigger<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabledPayload>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("{}:write", T::scope_domain_name()))?;

    let mut tx = user_db.begin(&authed).await?;
    let updated = handler
        .set_enabled(&mut *tx, &workspace_id, path, payload.enabled)
        .await?;

    if !updated {
        return Err(error::Error::NotFound(format!(
            "Trigger not found at path: {}",
            path
        )));
    }

    tx.commit().await?;

    Ok(format!(
        "Trigger '{}' {}",
        path,
        if payload.enabled {
            "enabled"
        } else {
            "disabled"
        }
    ))
}

async fn test_connection<T: TriggerCrud>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Json(config): Json<T::TestConnectionConfig>,
) -> Result<()> {
    let connect_f = async move {
        handler
            .test_connection(&db, &authed, &user_db, &workspace_id, config)
            .await
    };

    tokio::time::timeout(tokio::time::Duration::from_secs(30), connect_f)
        .await
        .map_err(|_| {
            error::Error::BadConfig(format!("Timeout connecting to service after 30 seconds"))
        })??;
    Ok(())
}

#[allow(unused)]
pub fn complete_trigger_routes<T: TriggerCrud + 'static>(handler: T) -> Router {
    let standard_routes = trigger_routes::<T>();

    let additional_routes = handler.additional_routes();

    standard_routes
        .merge(additional_routes)
        .layer(Extension(Arc::new(handler)))
}

pub fn generate_trigger_routers() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new();

    #[cfg(feature = "http_trigger")]
    {
        router = router.nest(
            HttpTriggerHandler::ROUTE_PREFIX,
            complete_trigger_routes(HttpTriggerHandler),
        );
    }

    #[cfg(feature = "websocket")]
    {
        router = router.nest(
            WebsocketTriggerHandler::ROUTE_PREFIX,
            complete_trigger_routes(WebsocketTriggerHandler),
        );
    }

    #[cfg(all(feature = "enterprise", feature = "kafka"))]
    {
        router = router.nest(
            KafkaTriggerHandler::ROUTE_PREFIX,
            complete_trigger_routes(KafkaTriggerHandler),
        );
    }

    #[cfg(all(feature = "enterprise", feature = "nats"))]
    {
        router = router.nest(
            NatsTriggerHandler::ROUTE_PREFIX,
            complete_trigger_routes(NatsTriggerHandler),
        );
    }

    #[cfg(feature = "mqtt_trigger")]
    {
        router = router.nest(
            MqttTriggerHandler::ROUTE_PREFIX,
            complete_trigger_routes(MqttTriggerHandler),
        );
    }

    #[cfg(all(feature = "enterprise", feature = "sqs_trigger"))]
    {
        router = router.nest(
            SqsTriggerHandler::ROUTE_PREFIX,
            complete_trigger_routes(SqsTriggerHandler),
        );
    }

    #[cfg(all(feature = "enterprise", feature = "gcp_trigger"))]
    {
        router = router.nest(
            GcpTriggerHandler::ROUTE_PREFIX,
            complete_trigger_routes(GcpTriggerHandler),
        );
    }

    #[cfg(feature = "postgres_trigger")]
    {
        router = router.nest(
            PostgresTriggerHandler::ROUTE_PREFIX,
            complete_trigger_routes(PostgresTriggerHandler),
        );
    }

    router
}
