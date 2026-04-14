use windmill_common::error::{self, Result};

/// Update `script_path` across all trigger tables when a runnable (script or flow) is renamed.
///
/// - For long-running triggers (with `server_id`), resets `server_id = NULL` to force
///   the heartbeat-based restart mechanism to pick up the new config.
pub async fn update_triggers_on_runnable_rename(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new_path: &str,
    old_path: &str,
    w_id: &str,
    is_flow: bool,
) -> Result<()> {
    windmill_common::triggers::update_triggers_script_path(tx, new_path, old_path, w_id, is_flow)
        .await
        .map_err(|e| {
            error::Error::internal_err(format!(
                "Error updating triggers due to runnable path change: {e:#}"
            ))
        })?;

    Ok(())
}
