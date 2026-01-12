use crate::{
    db::ApiAuthed,
    native_triggers::{
        delete_native_trigger, get_native_trigger, get_workspace_integration, list_native_triggers,
        store_native_trigger, update_native_trigger_error, EventType, External, NativeTrigger,
        NativeTriggerConfig, NativeTriggerData, ServiceName,
    },
    users::{create_token_internal, NewToken},
    utils::check_scopes,
};
use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use std::sync::Arc;
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
        crate::flows::require_is_writer(authed, path, w_id, db).await
    } else {
        crate::scripts::require_is_writer(authed, path, w_id, db).await
    }
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct FullTriggerResponse<T: Serialize> {
    #[serde(flatten)]
    pub windmill_data: NativeTrigger,
    pub external_data: T,
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
    let label = format!(
        "native-triggers-webhook-{}-{}",
        service_name.as_str(),
        rd_string(5)
    );
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
    Json(mut data): Json<NativeTriggerData<T::Payload>>,
) -> JsonResult<CreateTriggerResponse> {
    check_scopes(&authed, || {
        format!("native_triggers:write:{}", &data.script_path)
    })?;
    require_is_writer_on_runnable(&authed, &data.script_path, data.is_flow, &workspace_id, db.clone()).await?;

    let _ = handler.validate_data_config(&data);
    let mut tx = user_db.begin(&authed).await?;

    let EventType::Webhook(webhook) = &mut data.event_type;

    webhook.token = new_webhook_token(
        &mut *tx,
        &db,
        &authed,
        &data.script_path,
        data.is_flow,
        &workspace_id,
        service_name,
    )
    .await?;

    let integration = get_workspace_integration(&mut *tx, &workspace_id, service_name).await?;

    let oauth_data: T::OAuthData = serde_json::from_value(integration.oauth_data).map_err(|e| {
        Error::InternalErr(format!(
            "Failed to parse {} OAuth data: {}",
            T::DISPLAY_NAME,
            e
        ))
    })?;

    let resp = handler
        .create(&workspace_id, &oauth_data, &data, &db, &mut tx)
        .await?;

    let (external_id, _) = handler.external_id_and_metadata_from_response(&resp);

    let config = NativeTriggerConfig {
        script_path: data.script_path.clone(),
        is_flow: data.is_flow,
        event_type: data.event_type.clone(),
    };

    let service_config = handler.extract_service_config_from_payload(&data.payload);

    store_native_trigger(
        &mut *tx,
        &workspace_id,
        service_name,
        &external_id,
        &config,
        Some(&*service_config),
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
    Json(mut data): Json<NativeTriggerData<T::Payload>>,
) -> Result<String> {
    check_scopes(&authed, || {
        format!("native_triggers:write:{}", &data.script_path)
    })?;
    require_is_writer_on_runnable(&authed, &data.script_path, data.is_flow, &workspace_id, db.clone()).await?;
    let _ = handler.validate_data_config(&data);

    let mut tx = user_db.begin(&authed).await?;

    let existing = get_native_trigger(&mut *tx, &workspace_id, service_name, &external_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Native trigger not found: {}", external_id)))?;

    let existing_event_type: EventType = serde_json::from_value(existing.config)
        .map_err(|e| Error::InternalErr(format!("Failed to parse config: {}", e)))?;

    let EventType::Webhook(webhook) = &mut data.event_type;
    let EventType::Webhook(existing_webhook) = existing_event_type;
    webhook.token = existing_webhook.token;

    let integration = get_workspace_integration(&mut *tx, &workspace_id, service_name).await?;

    let oauth_data: T::OAuthData = serde_json::from_value(integration.oauth_data).map_err(|e| {
        Error::InternalErr(format!(
            "Failed to parse {} OAuth data: {}",
            T::DISPLAY_NAME,
            e
        ))
    })?;

    handler
        .update(&workspace_id, &oauth_data, &external_id, &data, &db, &mut tx)
        .await?;

    let config = NativeTriggerConfig {
        script_path: data.script_path.clone(),
        is_flow: data.is_flow,
        event_type: data.event_type.clone(),
    };

    let service_config = handler.extract_service_config_from_payload(&data.payload);

    store_native_trigger(
        &mut *tx,
        &workspace_id,
        service_name,
        &external_id,
        &config,
        Some(&*service_config),
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
    require_is_writer_on_runnable(&authed, &windmill_trigger.script_path, windmill_trigger.is_flow, &workspace_id, db.clone()).await?;

    let integration = get_workspace_integration(&mut *tx, &workspace_id, service_name).await?;

    let oauth_data: T::OAuthData = serde_json::from_value(integration.oauth_data).map_err(|e| {
        Error::InternalErr(format!(
            "Failed to parse {} OAuth data: {}",
            T::DISPLAY_NAME,
            e
        ))
    })?;

    let native_trigger = handler
        .get(&workspace_id, &oauth_data, &external_id, &db, &mut tx)
        .await;

    let native_trigger_config = match native_trigger {
        Ok(native_cfg) => {
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
            native_cfg
        }
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

    let full_resp = Json(FullTriggerResponse {
        windmill_data: windmill_trigger,
        external_data: native_trigger_config,
    });

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
    require_is_writer_on_runnable(&authed, &existing.script_path, existing.is_flow, &workspace_id, db.clone()).await?;

    let integration = get_workspace_integration(&mut *tx, &workspace_id, service_name).await?;

    let oauth_data: T::OAuthData = serde_json::from_value(integration.oauth_data).map_err(|e| {
        Error::InternalErr(format!(
            "Failed to parse {} OAuth data: {}",
            T::DISPLAY_NAME,
            e
        ))
    })?;

    handler
        .delete(&workspace_id, &oauth_data, &external_id, &db, &mut tx)
        .await?;

    let deleted =
        delete_native_trigger(&mut *tx, &workspace_id, service_name, &external_id).await?;

    if !deleted {
        return Err(Error::NotFound(format!("Native trigger not found")));
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

async fn exists_native_trigger_handler<T: External>(
    Extension(service_name): Extension<ServiceName>,
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, external_id)): Path<(String, String)>,
) -> JsonResult<bool> {
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM native_triggers
            WHERE
                workspace_id = $1 AND
                service_name = $2 AND
                external_id = $3
        )
        "#,
        workspace_id,
        service_name as ServiceName,
        external_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn list_native_triggers_handler<T: External>(
    Extension(service_name): Extension<ServiceName>,
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> JsonResult<Vec<NativeTrigger>> {
    let triggers = list_native_triggers(
        &db,
        &workspace_id,
        service_name,
        query.page,
        query.per_page,
    )
    .await?;
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
        )
        .route(
            "/exists/:external_id",
            get(exists_native_trigger_handler::<T>),
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

    #[cfg(feature = "native_triggers")]
    {
        use crate::native_triggers::nextcloud::NextCloud;

        // Register all service routes here
        // When adding a new service:
        // 1. Import the handler: use crate::native_triggers::newservice::NewServiceHandler;
        // 2. Add the route: .nest("/newservice", service_routes(NewServiceHandler))
        return router.nest("/nextcloud", service_routes(NextCloud));
        // Add new services here:
        // .nest("/newservice", service_routes(NewServiceHandler))
    }

    #[cfg(not(feature = "native_triggers"))]
    {
        router
    }
}
