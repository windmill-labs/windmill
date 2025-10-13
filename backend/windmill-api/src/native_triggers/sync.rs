use std::collections::HashMap;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{db::UserDB, error::Result, DB};

use serde::Serialize;

use crate::{
    native_triggers::{
        decrypt_oauth_data, delete_native_trigger, list_native_triggers, External, ServiceName,
    },
    users::fetch_api_authed,
};

#[derive(Debug, Serialize)]
pub struct DeletedTriggerInfo {
    pub internal_id: i64,
    pub external_id: String,
    pub runnable_path: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct SyncError {
    pub resource_path: String,
    pub error_message: String,
    pub error_type: String,
}

#[derive(Debug)]
pub struct BackgroundSyncResult {
    pub workspaces_processed: usize,
    pub total_deleted: usize,
    pub total_errors: usize,
    pub service_results: HashMap<ServiceName, ServiceSyncResult>,
}

#[derive(Debug)]
pub struct ServiceSyncResult {
    pub deleted_triggers: Vec<DeletedTriggerInfo>,
    pub errors: Vec<SyncError>,
}

pub const SYNC_INTERVAL: u64 = 10 * 60;

pub async fn sync_all_triggers(db: &DB) -> Result<BackgroundSyncResult> {
    tracing::debug!("Starting native triggers sync");

    let workspaces = sqlx::query_scalar!(
        r#"
        SELECT id
        FROM workspace
        WHERE deleted = false
        "#
    )
    .fetch_all(db)
    .await?;

    let mut service_results: HashMap<ServiceName, ServiceSyncResult> = HashMap::new();
    let mut total_deleted = 0;
    let mut total_errors = 0;

    use crate::native_triggers::nextcloud::NextCloud;

    match sync_service_triggers::<NextCloud>(db, &workspaces, NextCloud).await {
        Ok(result) => {
            total_deleted += result.deleted_triggers.len();
            total_errors += result.errors.len();
            service_results.insert(ServiceName::Nextcloud, result);
        }
        Err(e) => {
            tracing::error!("Error syncing NextCloud triggers: {:#}", e);
            service_results.insert(
                ServiceName::Nextcloud,
                ServiceSyncResult {
                    deleted_triggers: Vec::new(),
                    errors: vec![SyncError {
                        resource_path: "background_sync".to_string(),
                        error_message: format!("Failed to sync NextCloud triggers: {}", e),
                        error_type: "service_sync_error".to_string(),
                    }],
                },
            );
            total_errors += 1;
        }
    }

    let result = BackgroundSyncResult {
        workspaces_processed: workspaces.len(),
        total_deleted,
        total_errors,
        service_results,
    };

    tracing::debug!(
        "Completed native triggers sync: {} workspaces, {} deleted, {} errors",
        result.workspaces_processed,
        result.total_deleted,
        result.total_errors
    );

    Ok(result)
}
async fn sync_service_triggers<T: External>(
    db: &DB,
    workspaces: &[String],
    handler: T,
) -> Result<ServiceSyncResult> {
    let mut all_deleted_triggers = Vec::new();
    let mut all_errors = Vec::new();

    for workspace_id in workspaces {
        tracing::debug!(
            "Syncing {} triggers for workspace {}",
            T::SERVICE_NAME.as_str(),
            workspace_id
        );
        let sync_result = sync_workspace_triggers::<T>(db, workspace_id, &handler).await;

        match sync_result {
            Ok((deleted_triggers, errors)) => {
                all_deleted_triggers.extend(deleted_triggers);
                all_errors.extend(errors);
            }
            Err(e) => {
                tracing::error!(
                    "Error syncing {} triggers for workspace {}: {:#}",
                    T::SERVICE_NAME.as_str(),
                    workspace_id,
                    e
                );
                all_errors.push(SyncError {
                    resource_path: format!("workspace:{}", workspace_id),
                    error_message: format!("Failed to sync workspace: {}", e),
                    error_type: "workspace_sync_error".to_string(),
                });
            }
        }
    }

    Ok(ServiceSyncResult { deleted_triggers: all_deleted_triggers, errors: all_errors })
}

#[cfg(feature = "native_triggers")]
pub async fn sync_workspace_triggers<T: External>(
    db: &DB,
    workspace_id: &str,
    handler: &T,
) -> Result<(Vec<DeletedTriggerInfo>, Vec<SyncError>)> {
    tracing::debug!(
        "Syncing {} triggers for workspace '{}'",
        T::SERVICE_NAME.as_str(),
        workspace_id
    );

    let windmill_triggers =
        list_native_triggers(db, workspace_id, T::SERVICE_NAME, None, None).await?;

    if windmill_triggers.is_empty() {
        tracing::debug!(
            "No {} triggers found for workspace '{}'",
            T::SERVICE_NAME.as_str(),
            workspace_id
        );
        return Ok((Vec::new(), Vec::new()));
    }

    let mut all_deleted_triggers = Vec::new();
    let mut all_sync_errors = Vec::new();

    let oauth_data = {
        match decrypt_oauth_data(db, db, workspace_id, T::SERVICE_NAME).await {
            Ok(oauth_data) => oauth_data,
            Err(e) => {
                all_sync_errors.push(SyncError {
                    resource_path: format!("workspace:{}", workspace_id),
                    error_message: format!("Failed to get workspace integration OAuth data: {}", e),
                    error_type: "oauth_error".to_string(),
                });
                return Ok((Vec::new(), all_sync_errors));
            }
        }
    };

    let mut tx = db.begin().await?;
    let external_triggers = match handler
        .list_all(workspace_id, &oauth_data, db, &mut tx)
        .await
    {
        Ok(triggers) => triggers,
        Err(e) => {
            all_sync_errors.push(SyncError {
                resource_path: format!("workspace:{}", workspace_id),
                error_message: format!("Failed to fetch external triggers: {}", e),
                error_type: "external_service_error".to_string(),
            });
            return Ok((Vec::new(), all_sync_errors));
        }
    };

    let mut external_trigger_ids: HashMap<String, bool> = HashMap::new();
    for external_trigger in &external_triggers {
        let external_id = handler.get_external_id_from_trigger_data(external_trigger);
        external_trigger_ids.insert(external_id, true);
    }

    for trigger in &windmill_triggers {
        if !external_trigger_ids.contains_key(&trigger.external_id) {
            tracing::info!(
                "Trigger '{}' (external_id: '{}') no longer exists in external service, deleting",
                trigger.runnable_path,
                trigger.external_id
            );

            let authed = match fetch_api_authed(
                trigger.edited_by.clone(),
                trigger.email.clone(),
                workspace_id,
                db,
                Some("background-sync".to_string()),
            )
            .await
            {
                Ok(authed) => authed,
                Err(e) => {
                    all_sync_errors.push(SyncError {
                        resource_path: format!("workspace:{}", workspace_id),
                        error_message: format!(
                            "Failed to get authentication for trigger {}: {}",
                            trigger.id, e
                        ),
                        error_type: "authentication_error".to_string(),
                    });
                    continue;
                }
            };

            let user_db = UserDB::new(db.clone());
            let mut tx = user_db.begin(&authed).await?;

            match delete_native_trigger(&mut *tx, workspace_id, trigger.id, T::SERVICE_NAME).await {
                Ok(true) => {
                    if let Err(audit_err) = audit_log(
                        &mut *tx,
                        &authed,
                        &format!("native_triggers.{}.background_sync_auto_delete", T::SERVICE_NAME.as_str()),
                        ActionKind::Delete,
                        workspace_id,
                        Some(&format!(
                            "Auto-deleted trigger '{}' (external_id: '{}') during background sync because it no longer exists in external service",
                            trigger.runnable_path,
                            trigger.external_id
                        )),
                        None,
                    ).await {
                        tracing::warn!(
                            "Failed to log audit for auto-deleted trigger {}: {}",
                            trigger.id,
                            audit_err
                        );
                    }

                    tx.commit().await?;

                    all_deleted_triggers.push(DeletedTriggerInfo {
                        internal_id: trigger.id,
                        external_id: trigger.external_id.clone(),
                        runnable_path: trigger.runnable_path.clone(),
                        reason: "Trigger no longer exists in external service (background sync)"
                            .to_string(),
                    });
                }
                Ok(false) => {
                    tracing::warn!(
                        "Trigger {} (external_id: '{}') was not found in database during deletion",
                        trigger.id,
                        trigger.external_id
                    );
                }
                Err(e) => {
                    all_sync_errors.push(SyncError {
                        resource_path: format!("workspace:{}", workspace_id),
                        error_message: format!(
                            "Failed to delete trigger '{}' (external_id: '{}'): {}",
                            trigger.runnable_path, trigger.external_id, e
                        ),
                        error_type: "database_deletion_error".to_string(),
                    });
                }
            }
        }
    }

    tracing::debug!(
        "Sync completed for {} in workspace '{}'. Deleted: {}, Errors: {}",
        T::SERVICE_NAME.as_str(),
        workspace_id,
        all_deleted_triggers.len(),
        all_sync_errors.len()
    );

    Ok((all_deleted_triggers, all_sync_errors))
}
