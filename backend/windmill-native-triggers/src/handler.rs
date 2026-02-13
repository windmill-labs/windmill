use crate::{
    decrypt_oauth_data, delete_native_trigger, delete_token_by_prefix, get_native_trigger,
    get_token_by_prefix, list_native_triggers, store_native_trigger, update_native_trigger_error,
    External, NativeTrigger, NativeTriggerConfig, NativeTriggerData, ServiceName,
};
use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use std::sync::Arc;
use windmill_api_auth::{
    check_scopes, create_token_internal, require_is_writer, ApiAuthed, NewToken,
};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::rd_string,
    DB,
};

async fn require_is_writer_on_runnable(
    authed: &ApiAuthed,
    path: &str,
    is_flow: bool,
    w_id: &str,
    db: DB,
) -> Result<()> {
    if is_flow {
        require_is_writer(
            authed,
            path,
            w_id,
            db,
            "SELECT extra_perms FROM flow WHERE path = $1 AND workspace_id = $2",
            "flow",
        )
        .await
    } else {
        require_is_writer(
            authed,
            path,
            w_id,
            db,
            "SELECT extra_perms FROM script WHERE path = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
            "script",
        )
        .await
    }
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct FullTriggerResponse<T: Serialize> {
    #[serde(flatten)]
    pub windmill_data: NativeTrigger,
    pub external_data: Option<T>,
}

#[derive(Debug, Serialize)]
pub struct CreateTriggerResponse {
    pub external_id: String,
}

async fn new_webhook_token(
    tx: &mut PgConnection,
    db: &DB,
    authed: &ApiAuthed,
    script_path: &str,
    is_flow: bool,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<String> {
    let kind = if is_flow { "flows" } else { "scripts" };

    let scopes = vec![format!("jobs:run:{kind}:{script_path}")];
    let label = format!("webhook-{}-{}", service_name.as_str(), rd_string(5));
    let token_config = NewToken::new(
        Some(label),
        None,
        None,
        Some(scopes),
        Some(workspace_id.to_owned()),
    );
    let token = create_token_internal(&mut *tx, &db, &authed, token_config).await?;

    Ok(token)
}

async fn create_native_trigger<T: External>(
    Extension(handler): Extension<Arc<T>>,
    Extension(service_name): Extension<ServiceName>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Json(data): Json<NativeTriggerData<T::ServiceConfig>>,
) -> JsonResult<CreateTriggerResponse> {
    check_scopes(&authed, || {
        format!("native_triggers:write:{}", &data.script_path)
    })?;
    require_is_writer_on_runnable(
        &authed,
        &data.script_path,
        data.is_flow,
        &workspace_id,
        db.clone(),
    )
    .await?;

    let mut tx = user_db.begin(&authed).await?;

    let webhook_token = new_webhook_token(
        &mut *tx,
        &db,
        &authed,
        &data.script_path,
        data.is_flow,
        &workspace_id,
        service_name,
    )
    .await?;

    let integration_service = service_name.integration_service();
    let oauth_data: T::OAuthData =
        decrypt_oauth_data(&db, &workspace_id, integration_service).await?;

    let resp = handler
        .create(
            &workspace_id,
            &oauth_data,
            &webhook_token,
            &data,
            &db,
            &mut tx,
        )
        .await?;

    let (external_id, _) = handler.external_id_and_metadata_from_response(&resp);

    // Some services (e.g. Google) can build service_config directly from the create response,
    // while others (e.g. Nextcloud) need an update+get cycle to correct the webhook URL
    // with the external_id assigned by the remote service.
    let service_config =
        if let Some(config) = handler.service_config_from_create_response(&data, &resp) {
            config
        } else {
            handler
                .update(
                    &workspace_id,
                    &oauth_data,
                    &external_id,
                    &webhook_token,
                    &data,
                    &db,
                    &mut tx,
                )
                .await?
        };

    let config = NativeTriggerConfig {
        script_path: data.script_path.clone(),
        is_flow: data.is_flow,
        webhook_token,
    };

    store_native_trigger(
        &mut *tx,
        &workspace_id,
        service_name,
        &external_id,
        &config,
        service_config,
    )
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("native_triggers.{}.create", service_name),
        ActionKind::Create,
        &workspace_id,
        Some(&external_id),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(Json(CreateTriggerResponse { external_id }))
}

async fn update_native_trigger_handler<T: External>(
    Extension(handler): Extension<Arc<T>>,
    Extension(service_name): Extension<ServiceName>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, external_id)): Path<(String, String)>,
    Json(data): Json<NativeTriggerData<T::ServiceConfig>>,
) -> Result<String> {
    check_scopes(&authed, || {
        format!("native_triggers:write:{}", &data.script_path)
    })?;
    require_is_writer_on_runnable(
        &authed,
        &data.script_path,
        data.is_flow,
        &workspace_id,
        db.clone(),
    )
    .await?;

    let integration_service = service_name.integration_service();
    let oauth_data: T::OAuthData =
        decrypt_oauth_data(&db, &workspace_id, integration_service).await?;

    let mut tx = user_db.clone().begin(&authed).await?;

    let existing = get_native_trigger(&mut *tx, &workspace_id, service_name, &external_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Native trigger not found: {}", external_id)))?;

    let runnable_changed =
        existing.script_path != data.script_path || existing.is_flow != data.is_flow;

    let webhook_token = match get_token_by_prefix(&db, &existing.webhook_token_prefix).await? {
        Some(token) if !runnable_changed => token,
        existing_token => {
            if let Some(_) = existing_token {
                delete_token_by_prefix(&db, &existing.webhook_token_prefix).await?;
            } else {
                tracing::warn!(
                    "Webhook token not found for trigger {} (prefix: {}), recreating token",
                    external_id,
                    existing.webhook_token_prefix
                );
            }
            let token = new_webhook_token(
                &mut *tx,
                &db,
                &authed,
                &data.script_path,
                data.is_flow,
                &workspace_id,
                service_name,
            )
            .await?;
            tx.commit().await?;
            tx = user_db.begin(&authed).await?;
            token
        }
    };

    let service_config = handler
        .update(
            &workspace_id,
            &oauth_data,
            &external_id,
            &webhook_token,
            &data,
            &db,
            &mut tx,
        )
        .await?;

    let config = NativeTriggerConfig {
        script_path: data.script_path.clone(),
        is_flow: data.is_flow,
        webhook_token,
    };

    store_native_trigger(
        &mut *tx,
        &workspace_id,
        service_name,
        &external_id,
        &config,
        service_config,
    )
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("native_triggers.{}.update", service_name),
        ActionKind::Update,
        &workspace_id,
        Some(&external_id),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Native trigger updated"))
}

async fn get_native_trigger_handler<T: External>(
    Extension(handler): Extension<Arc<T>>,
    Extension(service_name): Extension<ServiceName>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, external_id)): Path<(String, String)>,
) -> JsonResult<FullTriggerResponse<T::TriggerData>> {
    let mut tx = user_db.begin(&authed).await?;

    let windmill_trigger = get_native_trigger(&mut *tx, &workspace_id, service_name, &external_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Native trigger not found: {}", external_id)))?;

    check_scopes(&authed, || {
        format!("native_triggers:read:{}", &windmill_trigger.script_path)
    })?;
    require_is_writer_on_runnable(
        &authed,
        &windmill_trigger.script_path,
        windmill_trigger.is_flow,
        &workspace_id,
        db.clone(),
    )
    .await?;

    let integration_service = service_name.integration_service();
    let oauth_data: T::OAuthData =
        decrypt_oauth_data(&db, &workspace_id, integration_service).await?;

    let native_trigger = handler
        .get(&workspace_id, &oauth_data, &external_id, &db, &mut tx)
        .await;

    let external_data = match native_trigger {
        Ok(Some(native_cfg)) => {
            // Clear error if it was set
            if windmill_trigger.error.is_some() {
                update_native_trigger_error(
                    &mut *tx,
                    &workspace_id,
                    service_name,
                    &external_id,
                    None,
                )
                .await?;
            }
            Some(native_cfg)
        }
        Ok(None) => None,
        Err(Error::NotFound(_)) => {
            let error_msg = "Trigger no longer exists on external service".to_string();
            tracing::warn!(
                "Native trigger no longer exists on external service {}, setting error",
                service_name
            );

            update_native_trigger_error(
                &mut *tx,
                &workspace_id,
                service_name,
                &external_id,
                Some(&error_msg),
            )
            .await?;

            tx.commit().await?;

            return Err(Error::NotFound(format!(
                "Trigger '{}' no longer exists on external service {}",
                external_id, service_name
            )));
        }
        Err(e) => return Err(e),
    };

    let full_resp = Json(FullTriggerResponse { windmill_data: windmill_trigger, external_data });

    Ok(full_resp)
}

async fn delete_native_trigger_handler<T: External>(
    Extension(handler): Extension<Arc<T>>,
    Extension(service_name): Extension<ServiceName>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, external_id)): Path<(String, String)>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    let existing = get_native_trigger(&mut *tx, &workspace_id, service_name, &external_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Native trigger not found: {}", external_id)))?;

    check_scopes(&authed, || {
        format!("native_triggers:write:{}", &existing.script_path)
    })?;
    require_is_writer_on_runnable(
        &authed,
        &existing.script_path,
        existing.is_flow,
        &workspace_id,
        db.clone(),
    )
    .await?;

    let integration_service = service_name.integration_service();
    let oauth_data: T::OAuthData =
        decrypt_oauth_data(&db, &workspace_id, integration_service).await?;

    handler
        .delete(&workspace_id, &oauth_data, &external_id, &db, &mut tx)
        .await?;

    let deleted =
        delete_native_trigger(&mut *tx, &workspace_id, service_name, &external_id).await?;

    if !deleted {
        return Err(Error::NotFound(format!("Native trigger not found")));
    }

    // Delete the webhook token using its prefix
    if !delete_token_by_prefix(&db, &existing.webhook_token_prefix).await? {
        tracing::warn!(
            "Webhook token not found when deleting trigger {} (prefix: {})",
            external_id,
            existing.webhook_token_prefix
        );
    }

    audit_log(
        &mut *tx,
        &authed,
        &format!("native_triggers.{}.delete", service_name),
        ActionKind::Delete,
        &workspace_id,
        Some(&external_id),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Native trigger deleted"))
}

async fn list_native_triggers_handler<T: External>(
    Extension(service_name): Extension<ServiceName>,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> JsonResult<Vec<NativeTrigger>> {
    let mut tx = user_db.begin(&authed).await?;
    let triggers = list_native_triggers(
        &mut *tx,
        &workspace_id,
        service_name,
        query.page,
        query.per_page,
        query.path.as_deref(),
        query.is_flow,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(triggers))
}

pub fn service_routes<T: External + 'static>(handler: T) -> Router {
    let additional_routes = handler.additional_routes();
    let service_name = T::SERVICE_NAME;

    let handler_arc = Arc::new(handler);

    let standard_routes = Router::new()
        .route("/create", post(create_native_trigger::<T>))
        .route("/list", get(list_native_triggers_handler::<T>))
        .route("/get/:external_id", get(get_native_trigger_handler::<T>))
        .route(
            "/update/:external_id",
            post(update_native_trigger_handler::<T>),
        )
        .route(
            "/delete/:external_id",
            delete(delete_native_trigger_handler::<T>),
        );

    standard_routes
        .merge(additional_routes)
        .layer(Extension(handler_arc))
        .layer(Extension(service_name))
}

/// Generates routes for all registered native trigger services.
/// When adding a new service, add a new `.nest()` call here.
pub fn generate_native_trigger_routers() -> Router {
    let router = Router::new();

    #[cfg(feature = "native_trigger")]
    {
        use crate::google::Google;
        use crate::nextcloud::NextCloud;

        return router
            .nest("/nextcloud", service_routes(NextCloud))
            .nest("/google", service_routes(Google));
    }

    #[cfg(not(feature = "native_trigger"))]
    {
        router
    }
}
