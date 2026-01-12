use std::collections::HashMap;
use windmill_common::error::Result;
use windmill_common::DB;

use serde::Serialize;

use crate::native_triggers::{
    decrypt_oauth_data, list_native_triggers, update_native_trigger_error,
    update_native_trigger_service_config, External, ServiceName,
};

#[derive(Debug, Serialize)]
pub struct TriggerSyncInfo {
    pub external_id: String,
    pub script_path: String,
    pub action: SyncAction,
}

#[derive(Debug, Serialize)]
pub enum SyncAction {
    ErrorSet(String),
    ErrorCleared,
    ConfigUpdated,
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
    pub total_synced: usize,
    pub total_errors: usize,
    pub service_results: HashMap<ServiceName, ServiceSyncResult>,
}

#[derive(Debug)]
pub struct ServiceSyncResult {
    pub synced_triggers: Vec<TriggerSyncInfo>,
    pub errors: Vec<SyncError>,
}

pub const SYNC_INTERVAL: u64 = 5 * 60;

/// Macro helper to sync a single service
macro_rules! sync_service {
    ($db:expr, $workspaces:expr, $service_results:expr, $total_synced:expr, $total_errors:expr,
     $service_name:ident, $service_variant:ident, $handler_path:path) => {{
        use $handler_path as Handler;
        match sync_service_triggers::<Handler>($db, $workspaces, Handler).await {
            Ok(result) => {
                $total_synced += result.synced_triggers.len();
                $total_errors += result.errors.len();
                $service_results.insert(ServiceName::$service_variant, result);
            }
            Err(e) => {
                tracing::error!(
                    "Error syncing {} triggers: {:#}",
                    stringify!($service_variant),
                    e
                );
                $service_results.insert(
                    ServiceName::$service_variant,
                    ServiceSyncResult {
                        synced_triggers: Vec::new(),
                        errors: vec![SyncError {
                            resource_path: "background_sync".to_string(),
                            error_message: format!(
                                "Failed to sync {} triggers: {}",
                                stringify!($service_variant),
                                e
                            ),
                            error_type: "service_sync_error".to_string(),
                        }],
                    },
                );
                $total_errors += 1;
            }
        }
    }};
}

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
    let mut total_synced = 0;
    let mut total_errors = 0;

    // Sync all registered services
    // When adding a new service, add a new sync_service! call here
    #[cfg(feature = "native_triggers")]
    {
        sync_service!(
            db,
            &workspaces,
            service_results,
            total_synced,
            total_errors,
            nextcloud,
            Nextcloud,
            crate::native_triggers::nextcloud::NextCloud
        );
        // Add new services here:
        // sync_service!(db, &workspaces, service_results, total_synced, total_errors,
        //     newservice, NewService, crate::native_triggers::newservice::NewServiceHandler);
    }

    let result = BackgroundSyncResult {
        workspaces_processed: workspaces.len(),
        total_synced,
        total_errors,
        service_results,
    };

    tracing::debug!(
        "Completed native triggers sync: {} workspaces, {} synced, {} errors",
        result.workspaces_processed,
        result.total_synced,
        result.total_errors
    );

    Ok(result)
}

async fn sync_service_triggers<T: External>(
    db: &DB,
    workspaces: &[String],
    handler: T,
) -> Result<ServiceSyncResult> {
    let mut all_synced_triggers = Vec::new();
    let mut all_errors = Vec::new();

    for workspace_id in workspaces {
        tracing::debug!(
            "Syncing {} triggers for workspace {}",
            T::SERVICE_NAME.as_str(),
            workspace_id
        );
        let sync_result = sync_workspace_triggers::<T>(db, workspace_id, &handler).await;

        match sync_result {
            Ok((synced_triggers, errors)) => {
                all_synced_triggers.extend(synced_triggers);
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

    Ok(ServiceSyncResult { synced_triggers: all_synced_triggers, errors: all_errors })
}

#[cfg(feature = "native_triggers")]
pub async fn sync_workspace_triggers<T: External>(
    db: &DB,
    workspace_id: &str,
    handler: &T,
) -> Result<(Vec<TriggerSyncInfo>, Vec<SyncError>)> {
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

    let mut all_synced_triggers = Vec::new();
    let mut all_sync_errors = Vec::new();

    let oauth_data = {
        match decrypt_oauth_data(db, db, workspace_id, T::SERVICE_NAME).await {
            Ok(oauth_data) => oauth_data,
            Err(e) => {
                tracing::error!(
                    "Failed to get workspace integration OAuth data for {}: {}",
                    workspace_id, e
                );
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
            tracing::error!(
                "Failed to fetch external triggers for {}: {}",
                workspace_id, e
            );
            all_sync_errors.push(SyncError {
                resource_path: format!("workspace:{}", workspace_id),
                error_message: format!("Failed to fetch external triggers: {}", e),
                error_type: "external_service_error".to_string(),
            });
            return Ok((Vec::new(), all_sync_errors));
        }
    };
    tx.commit().await?;

    // Build a map of external trigger IDs to their data
    let mut external_trigger_map: HashMap<String, &T::TriggerData> = HashMap::new();
    for external_trigger in &external_triggers {
        let external_id = handler.get_external_id_from_trigger_data(external_trigger);
        external_trigger_map.insert(external_id, external_trigger);
    }

    for trigger in &windmill_triggers {
        if !external_trigger_map.contains_key(&trigger.external_id) {
            // Trigger no longer exists on external service - set error
            let error_msg = "Trigger no longer exists on external service".to_string();

            if trigger.error.as_deref() != Some(&error_msg) {
                tracing::info!(
                    "Trigger (external_id: '{}', script_path: '{}') no longer exists in external service, setting error",
                    trigger.external_id,
                    trigger.script_path
                );

                match update_native_trigger_error(
                    db,
                    workspace_id,
                    T::SERVICE_NAME,
                    &trigger.external_id,
                    Some(&error_msg),
                )
                .await
                {
                    Ok(()) => {
                        all_synced_triggers.push(TriggerSyncInfo {
                            external_id: trigger.external_id.clone(),
                            script_path: trigger.script_path.clone(),
                            action: SyncAction::ErrorSet(error_msg),
                        });
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to update error for trigger (external_id: '{}'): {}",
                            trigger.external_id, e
                        );
                        all_sync_errors.push(SyncError {
                            resource_path: format!("workspace:{}", workspace_id),
                            error_message: format!(
                                "Failed to update error for trigger (external_id: '{}'): {}",
                                trigger.external_id, e
                            ),
                            error_type: "database_update_error".to_string(),
                        });
                    }
                }
            }
        } else {
            // Trigger exists on external service
            let external_trigger_data = external_trigger_map.get(&trigger.external_id).unwrap();

            // Clear error if it was set
            if trigger.error.is_some() {
                tracing::info!(
                    "Trigger (external_id: '{}', script_path: '{}') exists on external service, clearing error",
                    trigger.external_id,
                    trigger.script_path
                );

                match update_native_trigger_error(
                    db,
                    workspace_id,
                    T::SERVICE_NAME,
                    &trigger.external_id,
                    None,
                )
                .await
                {
                    Ok(()) => {
                        all_synced_triggers.push(TriggerSyncInfo {
                            external_id: trigger.external_id.clone(),
                            script_path: trigger.script_path.clone(),
                            action: SyncAction::ErrorCleared,
                        });
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to clear error for trigger (external_id: '{}'): {}",
                            trigger.external_id, e
                        );
                        all_sync_errors.push(SyncError {
                            resource_path: format!("workspace:{}", workspace_id),
                            error_message: format!(
                                "Failed to clear error for trigger (external_id: '{}'): {}",
                                trigger.external_id, e
                            ),
                            error_type: "database_update_error".to_string(),
                        });
                    }
                }
            }

            // Compare service_config and update if different
            let external_service_config =
                handler.extract_service_config_from_trigger_data(external_trigger_data);
            let stored_service_config = trigger
                .config
                .get("service_config")
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            // Parse external_service_config to Value for semantic comparison
            let external_config_value: serde_json::Value =
                serde_json::from_str(external_service_config.get())
                    .unwrap_or(serde_json::Value::Null);

            if external_config_value != stored_service_config {
                tracing::info!(
                    "Trigger (external_id: '{}', script_path: '{}') config differs from external service, updating local config",
                    trigger.external_id,
                    trigger.script_path
                );

                match update_native_trigger_service_config(
                    db,
                    workspace_id,
                    T::SERVICE_NAME,
                    &trigger.external_id,
                    &*external_service_config,
                )
                .await
                {
                    Ok(()) => {
                        all_synced_triggers.push(TriggerSyncInfo {
                            external_id: trigger.external_id.clone(),
                            script_path: trigger.script_path.clone(),
                            action: SyncAction::ConfigUpdated,
                        });
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to update config for trigger (external_id: '{}'): {}",
                            trigger.external_id, e
                        );
                        all_sync_errors.push(SyncError {
                            resource_path: format!("workspace:{}", workspace_id),
                            error_message: format!(
                                "Failed to update config for trigger (external_id: '{}'): {}",
                                trigger.external_id, e
                            ),
                            error_type: "database_update_error".to_string(),
                        });
                    }
                }
            }
        }
    }

    tracing::info!(
        "Sync completed for {} in workspace '{}'. Synced: {}, Errors: {}",
        T::SERVICE_NAME.as_str(),
        workspace_id,
        all_synced_triggers.len(),
        all_sync_errors.len()
    );

    Ok((all_synced_triggers, all_sync_errors))
}
