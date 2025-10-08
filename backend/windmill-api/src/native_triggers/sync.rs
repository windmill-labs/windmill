use std::sync::Arc;
use tokio::sync::RwLock;
use windmill_common::{error::Result, utils::report_critical_error, DB};

use crate::native_triggers::ServiceName;

/// Synchronization state for a service
struct SyncState {
    last_sync: Option<chrono::DateTime<chrono::Utc>>,
    error_count: usize,
}

lazy_static::lazy_static! {
    static ref SYNC_STATE: Arc<RwLock<std::collections::HashMap<ServiceName, SyncState>>> =
        Arc::new(RwLock::new(std::collections::HashMap::new()));
}

/// Start background synchronization loop for all native triggers
pub fn start_sync_loop(
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("Starting native triggers sync loop");

        // Initial delay before first sync
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes

        loop {
            tokio::select! {
                _ = killpill_rx.recv() => {
                    tracing::info!("Stopping native triggers sync loop");
                    break;
                }
                _ = interval.tick() => {
                    todo!()
                    /*if let Err(e) = sync_all_triggers(&db).await {
                        tracing::error!("Error syncing native triggers: {:#}", e);
                        report_critical_error(
                            format!("Native triggers sync failed: {:#}", e),
                            db.clone(),
                            None,
                            None
                        )
                        .await;
                    }*/
                }
            }
        }
    })
}

/// Sync all native triggers across all workspaces
/*async fn sync_all_triggers(db: &DB) -> Result<()> {
    tracing::debug!("Starting native triggers sync");

    // Get all workspaces
    let workspaces = sqlx::query_scalar!(
        r#"
        SELECT id
        FROM workspace
        WHERE deleted = false
        "#
    )
    .fetch_all(db)
    .await?;

    for workspace_id in workspaces {
        if let Err(e) = sync_workspace_triggers(db, &workspace_id).await {
            tracing::error!(
                "Error syncing native triggers for workspace {}: {:#}",
                workspace_id,
                e
            );
        }
    }

    tracing::debug!("Completed native triggers sync");
    Ok(())
}

/// Sync triggers for a specific workspace
async fn sync_workspace_triggers(db: &DB, workspace_id: &str) -> Result<()> {
    // Sync NextCloud triggers
    #[cfg(feature = "nextcloud_trigger")]
    {
        use crate::native_triggers::nextcloud::NextCloud;
        sync_service_triggers::<NextCloud>(db, workspace_id, NextCloud).await?;
    }

    Ok(())
}
*/
/// Sync triggers for a specific external service
/*async fn sync_service_triggers<T: External>(db: &DB, workspace_id: &str, handler: T) -> Result<()> {
    // Update sync state
    {
        let mut state = SYNC_STATE.write().await;
        let service_state = state
            .entry(T::SERVICE_NAME)
            .or_insert(SyncState { last_sync: None, error_count: 0 });
        service_state.last_sync = Some(chrono::Utc::now());
    }

    // Get all triggers for this service in this workspace
    let triggers =
        list_native_triggers(db, workspace_id, Some(T::SERVICE_NAME), None, None).await?;

    tracing::debug!(
        "Syncing {} {} triggers for workspace {}",
        triggers.len(),
        T::SERVICE_NAME.as_str(),
        workspace_id
    );

    // Create a temporary authed user for API calls
    // We use the trigger's email/edited_by for authentication
    for trigger in triggers {
        // Fetch API authed context for this trigger's owner
        let authed = match fetch_api_authed(db, &trigger.email, &trigger.workspace_id).await {
            Ok(authed) => authed,
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch auth for trigger {} owned by {}: {:#}",
                    trigger.path,
                    trigger.email,
                    e
                );
                continue;
            }
        };

        // Check if trigger still exists on external service
        match handler
            .exists(db, &authed, workspace_id, &trigger.external_id)
            .await
        {
            Ok(true) => {
                // Trigger exists, nothing to do
                tracing::trace!(
                    "Trigger {} exists on {} (external_id: {})",
                    trigger.path,
                    T::SERVICE_NAME.as_str(),
                    trigger.external_id
                );
            }
            Ok(false) => {
                // Trigger doesn't exist on external service, delete from Windmill
                tracing::warn!(
                    "Trigger {} no longer exists on {}, deleting from Windmill",
                    trigger.path,
                    T::SERVICE_NAME.as_str()
                );

                match delete_stale_trigger(
                    db,
                    &trigger.workspace_id,
                    &trigger.path,
                    T::SERVICE_NAME,
                )
                .await
                {
                    Ok(_) => {
                        tracing::info!(
                            "Deleted stale trigger {} from workspace {}",
                            trigger.path,
                            workspace_id
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to delete stale trigger {} from workspace {}: {:#}",
                            trigger.path,
                            workspace_id,
                            e
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Error checking if trigger {} exists on {}: {:#}",
                    trigger.path,
                    T::SERVICE_NAME.as_str(),
                    e
                );

                // Increment error count
                let mut state = SYNC_STATE.write().await;
                if let Some(service_state) = state.get_mut(&T::SERVICE_NAME) {
                    service_state.error_count += 1;
                }
            }
        }
    }

    Ok(())
}*/

/// Delete a stale trigger from Windmill database
async fn delete_stale_trigger(
    db: &DB,
    workspace_id: &str,
    path: &str,
    service_name: ServiceName,
) -> Result<()> {
    use crate::native_triggers::delete_native_trigger;

    let mut tx = db.begin().await?;

    delete_native_trigger(&mut *tx, workspace_id, path, service_name).await?;

    // Also log this deletion
    sqlx::query!(
        r#"
        INSERT INTO audit (
            workspace_id,
            username,
            operation,
            action_kind,
            resource,
            parameters
        ) VALUES (
            $1,
            'system',
            $2,
            'delete',
            $3,
            $4
        )
        "#,
        workspace_id,
        format!("native_triggers.{}.auto_delete", service_name.as_str()),
        path,
        serde_json::json!({
            "reason": "trigger_no_longer_exists_on_external_service",
            "service": service_name.as_str(),
        })
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

/// Get sync statistics for monitoring
pub async fn get_sync_stats(
) -> std::collections::HashMap<ServiceName, (Option<chrono::DateTime<chrono::Utc>>, usize)> {
    let state = SYNC_STATE.read().await;
    state
        .iter()
        .map(|(service, state)| (*service, (state.last_sync, state.error_count)))
        .collect()
}
