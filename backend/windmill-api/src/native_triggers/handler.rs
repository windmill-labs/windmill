use crate::{
    db::ApiAuthed,
    native_triggers::{
        delete_native_trigger, exists_native_trigger, get_native_trigger,
        get_native_trigger_by_external_id, list_native_triggers, store_native_trigger,
        update_native_trigger, External, NativeTrigger, NativeTriggerData, ServiceName,
    },
    utils::check_scopes,
};
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::StripPath,
    DB,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

use std::collections::HashMap;

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
pub struct SyncResult {
    pub already_in_sync: bool,
    pub added_count: usize,
    pub added_triggers: Vec<String>,
    pub total_external: usize,
    pub total_windmill: usize,
}

async fn create_native_trigger<T: External>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, String)>,
    Json(data): Json<NativeTriggerData<T::Payload>>,
) -> Result<(StatusCode, String)> {
    let service = ServiceName::from_str(&service_name)?;
    if service != T::SERVICE_NAME {
        return Err(Error::BadRequest(format!(
            "Service mismatch: expected {}, got {}",
            T::SERVICE_NAME.as_str(),
            service_name
        )));
    }

    check_scopes(&authed, || {
        format!("native_triggers:write:{}", &data.path)
    })?;

    // Extract resource_path from payload (it should be in metadata)
    let temp_metadata = handler.extract_metadata_from_payload(&data.payload, None)?;

    // Fetch the resource using try_get_resource_from_db_as
    let resource: T::Resource = crate::resources::try_get_resource_from_db_as(
        &authed,
        Some(user_db.clone()),
        &db,
        &temp_metadata.resource_path,
        &workspace_id,
    )
    .await?;

    let metadata = handler
        .create(&resource, &data.path, &data.payload)
        .await?;

    let mut tx = user_db.begin(&authed).await?;

    store_native_trigger(
        &mut *tx,
        &authed,
        &workspace_id,
        &data.path,
        service,
        metadata,
    )
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("native_triggers.{}.create", service.as_str()),
        ActionKind::Create,
        &workspace_id,
        Some(&data.path),
        None,
    )
    .await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        DeployedObject::NativeTrigger {
            path: data.path.clone(),
        },
        Some(format!(
            "{} native trigger '{}' created",
            T::DISPLAY_NAME,
            data.path
        )),
        true,
    )
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, data.path))
}

async fn update_native_trigger_handler<T: External>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name, path)): Path<(String, String, StripPath)>,
    Json(data): Json<NativeTriggerData<T::Payload>>,
) -> Result<String> {
    let path = path.to_path();
    let service = ServiceName::from_str(&service_name)?;
    if service != T::SERVICE_NAME {
        return Err(Error::BadRequest(format!(
            "Service mismatch: expected {}, got {}",
            T::SERVICE_NAME.as_str(),
            service_name
        )));
    }

    check_scopes(&authed, || {
        format!("native_triggers:write:{}", &data.path)
    })?;

    let existing = get_native_trigger(&db, &workspace_id, path, service).await?;

    // Fetch the resource using try_get_resource_from_db_as
    let resource: T::Resource = crate::resources::try_get_resource_from_db_as(
        &authed,
        Some(user_db.clone()),
        &db,
        &existing.resource_path,
        &workspace_id,
    )
    .await?;

    let metadata = handler
        .update(
            &resource,
            &existing.external_id,
            &data.path,
            &data.payload,
        )
        .await?;

    let mut tx = user_db.begin(&authed).await?;

    update_native_trigger(
        &mut *tx,
        &authed,
        &workspace_id,
        path,
        &data.path,
        service,
        metadata,
    )
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("native_triggers.{}.update", service.as_str()),
        ActionKind::Update,
        &workspace_id,
        Some(&data.path),
        None,
    )
    .await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &workspace_id,
        DeployedObject::NativeTrigger {
            path: data.path.clone(),
        },
        Some(format!(
            "{} native trigger '{}' updated",
            T::DISPLAY_NAME,
            data.path
        )),
        true,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Native trigger '{}' updated", data.path))
}

async fn get_native_trigger_handler<T: External>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, service_name, path)): Path<(String, String, StripPath)>,
) -> JsonResult<FullTriggerResponse<T::TriggerData>> {
    let path = path.to_path();
    let service = ServiceName::from_str(&service_name)?;
    if service != T::SERVICE_NAME {
        return Err(Error::BadRequest(format!(
            "Service mismatch: expected {}, got {}",
            T::SERVICE_NAME.as_str(),
            service_name
        )));
    }

    check_scopes(&authed, || format!("native_triggers:read:{}", path))?;

    let windmill_trigger = get_native_trigger(&db, &workspace_id, path, service).await?;

    // Fetch the resource using try_get_resource_from_db_as
    let resource: T::Resource = crate::resources::try_get_resource_from_db_as(
        &authed,
        Some(user_db.clone()),
        &db,
        &windmill_trigger.resource_path,
        &workspace_id,
    )
    .await?;

    let exists = handler
        .exists(
            &resource,
            &windmill_trigger.external_id,
        )
        .await?;

    if !exists {
        tracing::warn!(
            "Native trigger {} no longer exists on external service {}, will be deleted",
            path,
            service.as_str()
        );

        return Err(Error::NotFound(format!(
            "Trigger no longer exists on external service {}",
            service.as_str()
        )));
    }

    let external_data = handler
        .get(&resource, &windmill_trigger.external_id)
        .await?;

    Ok(Json(FullTriggerResponse {
        windmill_data: windmill_trigger,
        external_data,
    }))
}

/// Delete a native trigger
async fn delete_native_trigger_handler<T: External>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name, path)): Path<(String, String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    let service = ServiceName::from_str(&service_name)?;
    if service != T::SERVICE_NAME {
        return Err(Error::BadRequest(format!(
            "Service mismatch: expected {}, got {}",
            T::SERVICE_NAME.as_str(),
            service_name
        )));
    }

    check_scopes(&authed, || format!("native_triggers:write:{}", path))?;

    let existing = get_native_trigger(&db, &workspace_id, path, service).await?;

    // Fetch the resource using try_get_resource_from_db_as
    let resource: T::Resource = crate::resources::try_get_resource_from_db_as(
        &authed,
        Some(user_db.clone()),
        &db,
        &existing.resource_path,
        &workspace_id,
    )
    .await?;

    handler
        .delete(&resource, &existing.external_id)
        .await?;

    let mut tx = user_db.begin(&authed).await?;

    let deleted = delete_native_trigger(&mut *tx, &workspace_id, path, service).await?;

    if !deleted {
        return Err(Error::NotFound(format!(
            "Native trigger not found at path: {}",
            path
        )));
    }

    audit_log(
        &mut *tx,
        &authed,
        &format!("native_triggers.{}.delete", service.as_str()),
        ActionKind::Delete,
        &workspace_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Native trigger '{}' deleted", path))
}

async fn exists_native_trigger_handler<T: External>(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, service_name, path)): Path<(String, String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let service = ServiceName::from_str(&service_name)?;
    if service != T::SERVICE_NAME {
        return Err(Error::BadRequest(format!(
            "Service mismatch: expected {}, got {}",
            T::SERVICE_NAME.as_str(),
            service_name
        )));
    }

    check_scopes(&authed, || format!("native_triggers:read:{}", path))?;

    let exists = exists_native_trigger(&db, &workspace_id, path, service).await?;

    Ok(Json(exists))
}

async fn list_native_triggers_handler<T: External>(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, service_name)): Path<(String, String)>,
    Query(query): Query<ListQuery>,
) -> JsonResult<Vec<NativeTrigger>> {
    let service = ServiceName::from_str(&service_name)?;
    if service != T::SERVICE_NAME {
        return Err(Error::BadRequest(format!(
            "Service mismatch: expected {}, got {}",
            T::SERVICE_NAME.as_str(),
            service_name
        )));
    }

    let triggers =
        list_native_triggers(&db, &workspace_id, Some(service), query.page, query.per_page)
            .await?;

    Ok(Json(triggers))
}

/// Sync triggers from external service to Windmill
async fn sync_triggers_handler<T: External>(
    Extension(handler): Extension<Arc<T>>,
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, String)>,
) -> JsonResult<SyncResult> {
    let service = ServiceName::from_str(&service_name)?;
    if service != T::SERVICE_NAME {
        return Err(Error::BadRequest(format!(
            "Service mismatch: expected {}, got {}",
            T::SERVICE_NAME.as_str(),
            service_name
        )));
    }

    tracing::info!(
        "Syncing {} triggers for workspace {}",
        service.as_str(),
        workspace_id
    );

    // Get all triggers from Windmill first to find resource_path
    let windmill_triggers = list_native_triggers(&db, &workspace_id, service, None, None).await?;

    // If no triggers exist in Windmill, there's nothing to sync from
    if windmill_triggers.is_empty() {
        return Ok(Json(SyncResult {
            already_in_sync: true,
            added_count: 0,
            added_triggers: Vec::new(),
            total_external: 0,
            total_windmill: 0,
        }));
    }

    // Fetch the resource from the first trigger
    let resource: T::Resource = crate::resources::try_get_resource_from_db_as(
        &authed,
        Some(user_db.clone()),
        &db,
        &windmill_triggers[0].resource_path,
        &workspace_id,
    )
    .await?;

    // Get all triggers from external service
    let external_triggers = handler.list_all(&resource).await?;

    // Build a set of external IDs that exist in Windmill
    let windmill_external_ids: std::collections::HashSet<String> = windmill_triggers
        .iter()
        .map(|t| t.external_id.clone())
        .collect();

    // Find triggers that exist in external service but not in Windmill
    let mut added_triggers = Vec::new();
    let mut tx = user_db.begin(&authed).await?;

    for external_trigger in &external_triggers {
        let external_id = get_external_trigger_id(external_trigger);

        if !windmill_external_ids.contains(&external_id) {
            // This trigger doesn't exist in Windmill, add it
            tracing::info!(
                "Adding missing trigger with external_id {} to Windmill",
                external_id
            );

            // Extract metadata from external trigger
            let metadata = extract_metadata_from_external_trigger::<T>(external_trigger)?;

            // Generate a unique path for this trigger
            let path = format!("auto_sync/{}", external_id);

            // Store in Windmill
            store_native_trigger(&mut *tx, &authed, &workspace_id, &path, service, metadata)
                .await?;

            added_triggers.push(path.clone());

            // Log the addition
            audit_log(
                &mut *tx,
                &authed,
                &format!("native_triggers.{}.sync_add", service.as_str()),
                ActionKind::Create,
                &workspace_id,
                Some(&path),
                Some(
                    [("external_id".to_string(), serde_json::json!(external_id))]
                        .iter()
                        .cloned()
                        .collect(),
                ),
            )
            .await?;
        }
    }

    tx.commit().await?;

    let already_in_sync = added_triggers.is_empty();
    let result = SyncResult {
        already_in_sync,
        added_count: added_triggers.len(),
        added_triggers,
        total_external: external_triggers.len(),
        total_windmill: windmill_triggers.len() + added_triggers.len(),
    };

    tracing::info!(
        "Sync complete for {} in workspace {}: {} triggers added",
        service.as_str(),
        workspace_id,
        result.added_count
    );

    Ok(Json(result))
}

/// Helper to extract external ID from trigger data
fn get_external_trigger_id<T: External>(trigger: &T::TriggerData) -> String {
    // This assumes TriggerData has an `id` field
    // We need to use serde to extract it generically
    let json = serde_json::to_value(trigger).unwrap();
    json.get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/*fn extract_metadata_from_external_trigger<T: External>(
    trigger: &T::TriggerData,
) -> Result<crate::native_triggers::TriggerMetadata> {
    use crate::native_triggers::TriggerMetadata;

    let json = serde_json::to_value(trigger)
        .map_err(|e| Error::InternalErr(format!("Failed to serialize trigger: {}", e)))?;

    let external_id = json
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::BadRequest("Trigger missing 'id' field".to_string()))?
        .to_string();

    let summary = json
        .get("event_type")
        .and_then(|v| v.as_str())
        .map(|et| {
            if let Some(resource) = json.get("resource_path").and_then(|v| v.as_str()) {
                format!("{} trigger for {}", et, resource)
            } else {
                format!("{} trigger", et)
            }
        })
        .unwrap_or_else(|| "External trigger".to_string());

    Ok(TriggerMetadata {
        external_id,
        summary,
        metadata: Some(json),
    })
}*/

pub fn service_routes<T: External + 'static>(handler: T) -> Router {
    let additional_routes = handler.additional_routes();

    let handler_arc = Arc::new(handler);

    let standard_routes = Router::new()
        .route("/create", post(create_native_trigger::<T>))
        .route("/list", get(list_native_triggers_handler::<T>))
        .route("/get/*path", get(get_native_trigger_handler::<T>))
        .route("/update/*path", post(update_native_trigger_handler::<T>))
        .route("/delete/*path", delete(delete_native_trigger_handler::<T>))
        .route("/exists/*path", get(exists_native_trigger_handler::<T>))
        .route("/sync", post(sync_triggers_handler::<T>));

    standard_routes
        .merge(additional_routes)
        .layer(Extension(handler_arc))
}

pub fn generate_native_trigger_routers() -> Router {
    let mut router = Router::new();

    #[cfg(feature = "nextcloud_trigger")]
    {
        use crate::native_triggers::nextcloud::NextCloudHandler;

        router = router.nest(
            &format!("/{}", crate::native_triggers::ServiceName::Nextcloud.as_str()),
            service_routes(NextCloudHandler),
        );
    }

    router
}
