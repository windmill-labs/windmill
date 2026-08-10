use crate::{
    classify_read_failure, decrypt_oauth_data, delete_native_trigger, delete_token_by_hash,
    get_native_trigger, list_native_triggers, lock::TriggerLock, map_external_error,
    map_external_error_with, rotate_webhook_token, store_native_trigger,
    sync::EXTERNAL_TRIGGER_MISSING_ERROR, update_native_trigger_error,
    update_native_trigger_if_runnable_unchanged, webhook_token_label, webhook_token_scopes,
    External, ExternalReadFailure, NativeTrigger, NativeTriggerConfig, NativeTriggerData,
    ServiceName,
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

/// A trigger may only point at a live runnable.
///
/// The webhook URL is built from this path, so a trigger pointed at a path the runnable has left
/// keeps delivering there: for a flow the row is gone and the trigger vanishes from listings,
/// for a script the abandoned version is still resolvable and fires stale code indefinitely. A
/// client that loaded before a rename submits the old path in good faith, so this needs no
/// concurrency to happen. The writer checks above do not cover it: they return early for admins
/// and path owners without ever looking at the runnable.
async fn require_runnable_exists(db: &DB, w_id: &str, path: &str, is_flow: bool) -> Result<()> {
    let exists = if is_flow {
        sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM flow WHERE path = $1 AND workspace_id = $2)",
            path,
            w_id
        )
        .fetch_one(db)
        .await?
    } else {
        // Renaming a script archives the version at the old path instead of removing it, so a
        // plain existence check would still accept a path the script has moved off. Every deploy
        // archives its parent, leaving exactly one non-archived version at a live path and none
        // at an abandoned one. Soft-delete sets `archived` too, and `deleted` is checked because
        // it, not `archived`, is what stops a version from being resolved for execution.
        sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM script WHERE path = $1 AND workspace_id = $2 \
             AND archived = false AND deleted = false)",
            path,
            w_id
        )
        .fetch_one(db)
        .await?
    };

    if exists.unwrap_or(false) {
        Ok(())
    } else {
        Err(Error::BadRequest(format!(
            "There is no {kind} at {path} to trigger. If the {kind} was renamed since this page \
             was loaded, reload and try again; otherwise point this trigger at an existing {kind} \
             or delete it.",
            kind = if is_flow { "flow" } else { "script" }
        )))
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
    /// Why `external_data` is missing, when the service could not be read. The stored
    /// configuration is still returned so the trigger stays viewable and editable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTriggerResponse {
    pub external_id: String,
}

pub(crate) async fn new_webhook_token(
    tx: &mut PgConnection,
    db: &DB,
    authed: &ApiAuthed,
    script_path: &str,
    is_flow: bool,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<String> {
    let scopes = webhook_token_scopes(script_path, is_flow);
    let label = webhook_token_label(service_name);
    let expiration = service_name
        .webhook_token_expiration()
        .map(|d| chrono::Utc::now() + d);
    let token_config = NewToken::new(
        Some(label),
        expiration,
        None,
        Some(scopes),
        Some(workspace_id.to_owned()),
        None,
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
    require_runnable_exists(&db, &workspace_id, &data.script_path, data.is_flow).await?;

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
        .await
        .map_err(map_external_error)?;

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
                .await
                .map_err(map_external_error)?
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
        data.summary.as_deref(),
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
    require_runnable_exists(&db, &workspace_id, &data.script_path, data.is_flow).await?;

    let integration_service = service_name.integration_service();
    let oauth_data: T::OAuthData =
        decrypt_oauth_data(&db, &workspace_id, integration_service).await?;

    let lock = TriggerLock::acquire(&db, &workspace_id, service_name, &external_id).await?;

    let mut tx = user_db.clone().begin(&authed).await?;

    let existing = get_native_trigger(&mut *tx, &workspace_id, service_name, &external_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Native trigger not found: {}", external_id)))?;

    let runnable_changed =
        existing.script_path != data.script_path || existing.is_flow != data.is_flow;

    // Track old token hash so we can clean it up after everything succeeds
    let mut old_token_hash_to_delete: Option<String> = None;

    let webhook_token = if runnable_changed {
        // Scopes change when the runnable changes — delete old, create fresh token
        old_token_hash_to_delete = Some(existing.webhook_token_hash.clone());
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
    } else {
        // Same runnable — rotate the token (mints a fresh label + expiration)
        match rotate_webhook_token(
            &db,
            &existing.webhook_token_hash,
            service_name,
            webhook_token_scopes(&data.script_path, data.is_flow),
        )
        .await?
        {
            Some(rotated) => {
                old_token_hash_to_delete = Some(rotated.old_token_hash);
                rotated.new_token
            }
            None => {
                // Old token gone — create a fresh one
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
        .await
        .map_err(map_external_error)?;

    let config = NativeTriggerConfig {
        script_path: data.script_path.clone(),
        is_flow: data.is_flow,
        webhook_token,
    };

    // `existing` was read before the network call, and a rename writes `script_path` from the
    // deploy transaction, which this lock does not cover. Writing a stale path back would undo the
    // rename and drop the trigger out of every listing, so refuse the edit instead. The rename's
    // own re-registration is queued behind this lock and puts the service back in step.
    let applied = update_native_trigger_if_runnable_unchanged(
        &mut *tx,
        &workspace_id,
        service_name,
        &external_id,
        &config,
        service_config,
        data.summary.as_deref(),
        &existing.script_path,
        existing.is_flow,
    )
    .await?;

    if !applied {
        return Err(Error::BadRequest(format!(
            "The runnable of {external_id} was renamed while this trigger was being saved, so the \
             edit was not applied. Reload and save again."
        )));
    }

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

    lock.release().await?;

    // Everything succeeded — clean up old token (best-effort)
    if let Some(old_hash) = old_token_hash_to_delete {
        if let Err(e) = delete_token_by_hash(&db, &old_hash).await {
            tracing::warn!(
                "Failed to delete old webhook token after trigger update: {}",
                e
            );
        }
    }

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

    let mut external_error = None;

    let external_data = match native_trigger {
        Ok(Some(native_cfg)) => {
            // Only the "no longer exists" error is disproven by the trigger being there; other
            // paths record failures (e.g. a webhook still aimed at a pre-rename path) that a
            // successful fetch says nothing about.
            if windmill_trigger.error.as_deref() == Some(EXTERNAL_TRIGGER_MISSING_ERROR) {
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
        Err(e) => match classify_read_failure(e) {
            ExternalReadFailure::Missing => {
                let error_msg = EXTERNAL_TRIGGER_MISSING_ERROR.to_string();
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
            // The service being unreadable says nothing about the trigger Windmill stores, and
            // failing here would leave the editor with no configuration to show at all. Report
            // what the service said alongside the stored configuration instead.
            ExternalReadFailure::Unreadable(message) => {
                tracing::warn!(
                    "Could not read trigger '{}' from {}, returning the stored configuration: {}",
                    external_id,
                    service_name,
                    message
                );
                external_error = Some(message);
                None
            }
            ExternalReadFailure::Internal(e) => return Err(e),
        },
    };

    tx.commit().await?;

    let full_resp = Json(FullTriggerResponse {
        windmill_data: windmill_trigger,
        external_data,
        external_error,
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
    let lock = TriggerLock::acquire(&db, &workspace_id, service_name, &external_id).await?;

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
        .await
        .map_err(|e| {
            map_external_error_with(e, |m| {
                let end = if m.ends_with(['.', '!', '?']) {
                    ""
                } else {
                    "."
                };
                format!("{m}{end} The trigger was kept in Windmill, so it can still fire.")
            })
        })?;

    let deleted =
        delete_native_trigger(&mut *tx, &workspace_id, service_name, &external_id).await?;

    if !deleted {
        return Err(Error::NotFound(format!("Native trigger not found")));
    }

    // Delete the webhook token using its hash
    if !delete_token_by_hash(&db, &existing.webhook_token_hash).await? {
        tracing::warn!(
            "Webhook token not found when deleting trigger {} (hash: {})",
            external_id,
            existing.webhook_token_hash
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
    lock.release().await?;

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
        .route("/get/{external_id}", get(get_native_trigger_handler::<T>))
        .route(
            "/update/{external_id}",
            post(update_native_trigger_handler::<T>),
        )
        .route(
            "/delete/{external_id}",
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
        use crate::github::GitHub;
        use crate::google::Google;
        use crate::nextcloud::NextCloud;

        return router
            .nest("/nextcloud", service_routes(NextCloud))
            .nest("/google", service_routes(Google))
            .nest("/github", service_routes(GitHub));
    }

    #[cfg(not(feature = "native_trigger"))]
    {
        router
    }
}
