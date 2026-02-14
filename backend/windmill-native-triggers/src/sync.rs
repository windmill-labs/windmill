use std::collections::HashMap;
use windmill_common::error::Result;
use windmill_common::DB;

use serde::Serialize;

use crate::ServiceName;

#[cfg(feature = "native_trigger")]
use crate::{
    decrypt_oauth_data, list_native_triggers, update_native_trigger_error,
    update_native_trigger_service_config, External, NativeTrigger,
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

#[allow(unused_variables, unused_mut)]
pub async fn sync_all_triggers(db: &DB) -> Result<BackgroundSyncResult> {
    tracing::info!("Starting native triggers sync");

    let mut service_results: HashMap<ServiceName, ServiceSyncResult> = HashMap::new();
    let mut total_synced = 0;
    let mut total_errors = 0;
    let mut workspaces_processed = 0;

    // Sync all registered services
    // Each service only syncs workspaces that have the corresponding integration configured
    #[cfg(feature = "native_trigger")]
    {
        use crate::google::Google;
        use crate::nextcloud::NextCloud;

        // Nextcloud sync
        let (service_name, result) = sync_service_triggers(db, NextCloud).await;
        total_synced += result.synced_triggers.len();
        total_errors += result.errors.len();
        service_results.insert(service_name, result);

        // Google sync (handles both Drive and Calendar triggers)
        let (service_name, result) = sync_service_triggers(db, Google).await;
        total_synced += result.synced_triggers.len();
        total_errors += result.errors.len();
        service_results.insert(service_name, result);
    }

    // Count unique workspaces processed across all services
    for result in service_results.values() {
        workspaces_processed += result
            .synced_triggers
            .iter()
            .map(|t| &t.external_id)
            .collect::<std::collections::HashSet<_>>()
            .len();
    }

    let result =
        BackgroundSyncResult { workspaces_processed, total_synced, total_errors, service_results };

    tracing::info!(
        "Completed native triggers sync: {} updated, {} errors",
        result.total_synced,
        result.total_errors
    );

    Ok(result)
}

#[cfg(feature = "native_trigger")]
async fn sync_service_triggers<T: External>(
    db: &DB,
    handler: T,
) -> (ServiceName, ServiceSyncResult) {
    let mut all_synced_triggers = Vec::new();
    let mut all_errors = Vec::new();

    // Use the integration service for lookup (e.g., GoogleDrive/GoogleCalendar -> Google)
    let integration_service = T::SERVICE_NAME.integration_service();

    // Only sync workspaces that have the corresponding integration configured
    let workspaces_with_integration = match sqlx::query_scalar!(
        r#"
        SELECT wi.workspace_id
        FROM workspace_integrations wi
        JOIN workspace w ON w.id = wi.workspace_id
        WHERE wi.service_name = $1
          AND wi.oauth_data IS NOT NULL
          AND w.deleted = false
        "#,
        integration_service as ServiceName
    )
    .fetch_all(db)
    .await
    {
        Ok(workspaces) => workspaces,
        Err(e) => {
            tracing::error!(
                "Error querying workspaces with {} integration: {:#}",
                T::SERVICE_NAME.as_str(),
                e
            );
            all_errors.push(SyncError {
                resource_path: "database".to_string(),
                error_message: format!("Failed to query workspaces: {}", e),
                error_type: "database_error".to_string(),
            });
            return (
                T::SERVICE_NAME,
                ServiceSyncResult { synced_triggers: Vec::new(), errors: all_errors },
            );
        }
    };

    if workspaces_with_integration.is_empty() {
        tracing::debug!(
            "No workspaces with {} integration configured, skipping sync",
            T::SERVICE_NAME.as_str()
        );
        return (
            T::SERVICE_NAME,
            ServiceSyncResult { synced_triggers: Vec::new(), errors: Vec::new() },
        );
    }

    tracing::info!(
        "Found {} workspaces with {} integration configured",
        workspaces_with_integration.len(),
        T::SERVICE_NAME.as_str()
    );

    for workspace_id in workspaces_with_integration {
        let sync_result = sync_workspace_triggers::<T>(db, &workspace_id, &handler).await;

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

    (
        T::SERVICE_NAME,
        ServiceSyncResult { synced_triggers: all_synced_triggers, errors: all_errors },
    )
}

#[cfg(feature = "native_trigger")]
pub async fn sync_workspace_triggers<T: External>(
    db: &DB,
    workspace_id: &str,
    handler: &T,
) -> Result<(Vec<TriggerSyncInfo>, Vec<SyncError>)> {
    tracing::info!(
        "Syncing {} triggers for workspace '{}'",
        T::SERVICE_NAME.as_str(),
        workspace_id
    );

    let windmill_triggers =
        list_native_triggers(db, workspace_id, T::SERVICE_NAME, None, None, None, None).await?;

    if windmill_triggers.is_empty() {
        tracing::info!(
            "No {} triggers found for workspace '{}'",
            T::SERVICE_NAME.as_str(),
            workspace_id
        );
        return Ok((Vec::new(), Vec::new()));
    }

    let mut synced = Vec::new();
    let mut errors = Vec::new();

    // Use the integration service for OAuth lookup (e.g., GoogleDrive/GoogleCalendar -> Google)
    let integration_service = T::SERVICE_NAME.integration_service();

    let oauth_data = {
        match decrypt_oauth_data(db, workspace_id, integration_service).await {
            Ok(oauth_data) => oauth_data,
            Err(e) => {
                tracing::error!(
                    "Failed to get workspace integration OAuth data for {}: {}",
                    workspace_id,
                    e
                );
                errors.push(SyncError {
                    resource_path: format!("workspace:{}", workspace_id),
                    error_message: format!("Failed to get workspace integration OAuth data: {}", e),
                    error_type: "oauth_error".to_string(),
                });
                return Ok((Vec::new(), errors));
            }
        }
    };

    handler
        .maintain_triggers(
            db,
            workspace_id,
            &windmill_triggers,
            &oauth_data,
            &mut synced,
            &mut errors,
        )
        .await;

    tracing::info!(
        "Sync completed for {} in workspace '{}'. Updated: {}, Errors: {}",
        T::SERVICE_NAME.as_str(),
        workspace_id,
        synced.len(),
        errors.len()
    );

    Ok((synced, errors))
}

/// Reusable reconciliation logic for services with real external state (e.g. Nextcloud).
/// Compares external triggers with DB triggers: sets errors for missing ones,
/// clears errors and updates config for existing ones.
#[cfg(feature = "native_trigger")]
pub async fn reconcile_with_external_state(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
    windmill_triggers: &[NativeTrigger],
    external_triggers: &[(String, serde_json::Value)],
    synced: &mut Vec<TriggerSyncInfo>,
    errors: &mut Vec<SyncError>,
) {
    // Build a map of external trigger IDs to their config
    let mut external_trigger_map: HashMap<String, &serde_json::Value> = HashMap::new();
    for (external_id, config) in external_triggers {
        external_trigger_map.insert(external_id.clone(), config);
    }

    for trigger in windmill_triggers {
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
                    service_name,
                    &trigger.external_id,
                    Some(&error_msg),
                )
                .await
                {
                    Ok(()) => {
                        synced.push(TriggerSyncInfo {
                            external_id: trigger.external_id.clone(),
                            script_path: trigger.script_path.clone(),
                            action: SyncAction::ErrorSet(error_msg),
                        });
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to update error for trigger (external_id: '{}'): {}",
                            trigger.external_id,
                            e
                        );
                        errors.push(SyncError {
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
            let external_service_config = external_trigger_map.get(&trigger.external_id).unwrap();

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
                    service_name,
                    &trigger.external_id,
                    None,
                )
                .await
                {
                    Ok(()) => {
                        synced.push(TriggerSyncInfo {
                            external_id: trigger.external_id.clone(),
                            script_path: trigger.script_path.clone(),
                            action: SyncAction::ErrorCleared,
                        });
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to clear error for trigger (external_id: '{}'): {}",
                            trigger.external_id,
                            e
                        );
                        errors.push(SyncError {
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
            let stored_service_config = trigger
                .service_config
                .clone()
                .unwrap_or(serde_json::Value::Null);

            if **external_service_config != stored_service_config {
                tracing::info!(
                    "Trigger (external_id: '{}', script_path: '{}') config differs from external service, updating local config",
                    trigger.external_id,
                    trigger.script_path
                );

                match update_native_trigger_service_config(
                    db,
                    workspace_id,
                    service_name,
                    &trigger.external_id,
                    external_service_config,
                )
                .await
                {
                    Ok(()) => {
                        synced.push(TriggerSyncInfo {
                            external_id: trigger.external_id.clone(),
                            script_path: trigger.script_path.clone(),
                            action: SyncAction::ConfigUpdated,
                        });
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to update config for trigger (external_id: '{}'): {}",
                            trigger.external_id,
                            e
                        );
                        errors.push(SyncError {
                            resource_path: format!("workspace:{}", workspace_id),
                            error_message: format!(
                                "Failed to update config for trigger (external_id: '{}'): {}",
                                trigger.external_id, e
                            ),
                            error_type: "database_update_error".to_string(),
                        });
                    }
                }
            } else {
                tracing::info!(
                    "Trigger (external_id: '{}', script_path: '{}') config is the same as external service, no update needed",
                    trigger.external_id,
                    trigger.script_path
                );
            }
        }
    }
}
