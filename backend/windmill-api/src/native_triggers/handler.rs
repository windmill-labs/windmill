use crate::{
    db::ApiAuthed,
    native_triggers::{
        delete_native_trigger, get_workspace_integration, list_native_triggers,
        store_native_trigger, update_native_trigger, External, NativeTrigger, NativeTriggerData,
        ServiceName,
    },
    utils::check_scopes,
};
use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Postgres;
use std::sync::Arc;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::{RunnableKind, StripPath},
    DB,
};

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
    pub id: i64,
}

async fn create_native_trigger<T: External>(
    Extension(handler): Extension<Arc<T>>,
    Extension(service_name): Extension<ServiceName>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Json(data): Json<NativeTriggerData<T::Payload>>,
) -> JsonResult<CreateTriggerResponse> {
    let _ = handler.validate_data_config(&data);

    check_scopes(&authed, || {
        format!("native_triggers:write:{}", &data.runnable_path)
    })?;

    let mut tx = user_db.begin(&authed).await?;
    let integration = get_workspace_integration(&mut *tx, &workspace_id, service_name).await?;

    let oauth_data: T::OAuthData = serde_json::from_value(integration.oauth_data).map_err(|e| {
        Error::InternalErr(format!(
            "Failed to parse {} OAuth data: {}",
            T::DISPLAY_NAME,
            e
        ))
    })?;

    let trigger_id =
        store_native_trigger(&mut *tx, &authed, &workspace_id, service_name, &data).await?;

    let resp = handler
        .create(&workspace_id, trigger_id, &oauth_data, &data, &db, &mut tx)
        .await?;

    let (external_id, _) = handler.external_id_and_metadata_from_response(&resp);

    sqlx::query!(
        r#"
            UPDATE
                native_triggers
            SET
                external_id = $1
            WHERE
                workspace_id = $2
                AND id = $3
                AND service_name = $4
        "#,
        external_id,
        &workspace_id,
        trigger_id,
        service_name as ServiceName
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("native_triggers.{}.create", service_name),
        ActionKind::Create,
        &workspace_id,
        None,
        None,
    )
    .await?;

    /*handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        DeployedObject::NativeTrigger { path: data.path.clone() },
        Some(format!(
            "{} native trigger '{}' created",
            T::DISPLAY_NAME,
            data.path
        )),
        true,
    )
    .await?;
    */

    tx.commit().await?;

    Ok(Json(CreateTriggerResponse { id: trigger_id }))
}

async fn update_native_trigger_handler<T: External>(
    Extension(handler): Extension<Arc<T>>,
    Extension(service_name): Extension<ServiceName>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, id)): Path<(String, i64)>,
    Json(data): Json<NativeTriggerData<T::Payload>>,
) -> Result<String> {
    let _ = handler.validate_data_config(&data);

    check_scopes(&authed, || {
        format!("native_triggers:write:{}", &data.runnable_path)
    })?;

    let mut tx = user_db.begin(&authed).await?;

    let existing = get_native_trigger(&mut *tx, &workspace_id, id, service_name).await?;

    let integration = get_workspace_integration(&mut *tx, &workspace_id, service_name).await?;

    let oauth_data: T::OAuthData = serde_json::from_value(integration.oauth_data).map_err(|e| {
        Error::InternalErr(format!(
            "Failed to parse {} OAuth data: {}",
            T::DISPLAY_NAME,
            e
        ))
    })?;

    handler
        .update(
            &workspace_id,
            id,
            &oauth_data,
            &existing.external_id,
            &data,
            &db,
            &mut tx,
        )
        .await?;

    update_native_trigger(&mut *tx, &authed, &workspace_id, id, service_name, &data).await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("native_triggers.{}.update", service_name),
        ActionKind::Update,
        &workspace_id,
        None,
        None,
    )
    .await?;

    /*handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        DeployedObject::NativeTrigger { path: data.path.clone() },
        Some(format!(
            "{} native trigger '{}' updated",
            T::DISPLAY_NAME,
            data.path
        )),
        true,
    )
    .await?;*/

    tx.commit().await?;

    Ok(format!("Native trigger updated"))
}

async fn get_native_trigger_handler<T: External>(
    Extension(handler): Extension<Arc<T>>,
    Extension(service_name): Extension<ServiceName>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, id, runnable_path)): Path<(String, i64, StripPath)>,
) -> JsonResult<FullTriggerResponse<T::TriggerData>> {
    let runnable_path = runnable_path.to_path();

    check_scopes(&authed, || {
        format!("native_triggers:read:{}", runnable_path)
    })?;

    let mut tx = user_db.begin(&authed).await?;

    let windmill_trigger = get_native_trigger(&mut *tx, &workspace_id, id, service_name).await?;

    let integration = get_workspace_integration(&mut *tx, &workspace_id, service_name).await?;

    let oauth_data: T::OAuthData = serde_json::from_value(integration.oauth_data).map_err(|e| {
        Error::InternalErr(format!(
            "Failed to parse {} OAuth data: {}",
            T::DISPLAY_NAME,
            e
        ))
    })?;

    let native_trigger = handler
        .get(
            &workspace_id,
            &oauth_data,
            &windmill_trigger.external_id,
            &db,
            &mut tx,
        )
        .await;
    let native_trigger_config = match native_trigger {
        Ok(native_cfg) => native_cfg,
        Err(Error::NotFound(_)) => {
            tracing::warn!(
            "Native trigger no longer exists on external service {}, auto-deleting from database",
            service_name
        );
            let deleted = delete_native_trigger(&mut *tx, &workspace_id, id, service_name).await?;

            if deleted {
                audit_log(
                &mut *tx,
                &authed,
                &format!("native_triggers.{}.auto_delete", service_name),
                ActionKind::Delete,
                &workspace_id,
                Some(&format!("Auto-deleted trigger {} (external_id: {}) because it no longer exists on external service", 
                    windmill_trigger.runnable_path, windmill_trigger.external_id)),
                None,
            )
            .await?;

                tracing::info!(
                    "Auto-deleted native trigger {} from database (external_id: {})",
                    windmill_trigger.runnable_path,
                    windmill_trigger.external_id
                );
            }

            return Err(Error::NotFound(format!(
            "Trigger '{}' (external_id: {}) no longer exists on external service {} and has been automatically deleted",
            windmill_trigger.runnable_path,
            windmill_trigger.external_id,
            service_name
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

#[derive(Debug, Deserialize, Serialize)]
struct IdAndRunnablePath {
    id: i64,
    runnable_path: String,
}

async fn delete_native_trigger_handler<T: External>(
    Extension(handler): Extension<Arc<T>>,
    Extension(service_name): Extension<ServiceName>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
    Json(IdAndRunnablePath { id, runnable_path }): Json<IdAndRunnablePath>,
) -> Result<String> {
    check_scopes(&authed, || {
        format!("native_triggers:write:{}", runnable_path)
    })?;

    let mut tx = user_db.begin(&authed).await?;

    let existing = get_native_trigger(&mut *tx, &workspace_id, id, service_name).await?;

    let integration = get_workspace_integration(&mut *tx, &workspace_id, service_name).await?;

    let oauth_data: T::OAuthData = serde_json::from_value(integration.oauth_data).map_err(|e| {
        Error::InternalErr(format!(
            "Failed to parse {} OAuth data: {}",
            T::DISPLAY_NAME,
            e
        ))
    })?;

    handler
        .delete(
            &workspace_id,
            &oauth_data,
            &existing.external_id,
            &db,
            &mut tx,
        )
        .await?;

    let deleted = delete_native_trigger(&mut *tx, &workspace_id, id, service_name).await?;

    if !deleted {
        return Err(Error::NotFound(format!("Native trigger not found",)));
    }

    audit_log(
        &mut *tx,
        &authed,
        &format!("native_triggers.{}.delete", service_name),
        ActionKind::Delete,
        &workspace_id,
        None,
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Native trigger deleted"))
}

async fn exists_native_trigger_handler<T: External>(
    Extension(service_name): Extension<ServiceName>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    Json(IdAndRunnablePath { id, runnable_path }): Json<IdAndRunnablePath>,
) -> JsonResult<bool> {
    check_scopes(&authed, || {
        format!("native_triggers:read:{}", &runnable_path)
    })?;

    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 
                1
            FROM 
                native_triggers
            WHERE 
                workspace_id = $1 AND 
                id = $2 AND 
                service_name = $3 AND
                runnable_path = $4
        )
        "#,
        workspace_id,
        id,
        service_name as ServiceName,
        runnable_path
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
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
    )
    .await?;
    tx.commit().await?;
    Ok(Json(triggers))
}

pub async fn get_native_trigger<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    id: i64,
    service_name: ServiceName,
) -> Result<NativeTrigger> {
    let native_trigger = sqlx::query_as!(
        NativeTrigger,
        r#"
        SELECT
            id,
            runnable_path,
            runnable_kind AS "runnable_kind!: RunnableKind",
            service_name as "service_name!: ServiceName",
            external_id,
            workspace_id,
            summary,
            metadata,
            edited_by,
            email,
            edited_at
        FROM
            native_triggers
        WHERE
            workspace_id = $1
            AND id = $2
            AND service_name = $3
        "#,
        workspace_id,
        id,
        service_name as ServiceName
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Native trigger not found at path")));

    native_trigger
}

pub fn service_routes<T: External + 'static>(handler: T) -> Router {
    let additional_routes = handler.additional_routes();
    let service_name = T::SERVICE_NAME;

    let handler_arc = Arc::new(handler);

    let standard_routes = Router::new()
        .route("/create", post(create_native_trigger::<T>))
        .route("/list", get(list_native_triggers_handler::<T>))
        .route("/get/:id/*path", get(get_native_trigger_handler::<T>))
        .route("/update/:id", post(update_native_trigger_handler::<T>))
        .route("/delete", delete(delete_native_trigger_handler::<T>))
        .route("/exists", post(exists_native_trigger_handler::<T>));

    standard_routes
        .merge(additional_routes)
        .layer(Extension(handler_arc))
        .layer(Extension(service_name))
}

pub fn generate_native_trigger_routers() -> Router {
    let router = Router::new();

    #[cfg(feature = "native_triggers")]
    {
        use crate::native_triggers::nextcloud::NextCloud;

        return router.nest("/nextcloud", service_routes(NextCloud));
    }

    #[cfg(not(feature = "native_triggers"))]
    {
        router
    }
}
