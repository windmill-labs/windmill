use windmill_common::{
    error::{self, Result},
    triggers::NativeTriggerToReregister,
    DB,
};

/// Update `script_path` across all trigger tables when a runnable (script or flow) is renamed.
///
/// - For long-running triggers (with `server_id`), resets `server_id = NULL` to force
///   the heartbeat-based restart mechanism to pick up the new config.
/// - For native triggers, updates the DB script_path and spawns async re-registration
///   with external services (token rotation + webhook URL update).
pub async fn update_triggers_on_runnable_rename(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    db: &DB,
    authed: &windmill_api_auth::ApiAuthed,
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

    // Find native triggers that were updated — they need async webhook re-registration
    let native_triggers = sqlx::query_as!(
        NativeTriggerToReregister,
        "SELECT external_id, service_name::TEXT as \"service_name!\" FROM native_trigger \
         WHERE script_path = $1 AND workspace_id = $2 AND is_flow = $3",
        new_path,
        w_id,
        is_flow,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| {
        error::Error::internal_err(format!(
            "Error querying native triggers for re-registration: {e:#}"
        ))
    })?;

    if !native_triggers.is_empty() {
        let db = db.clone();
        let authed = authed.clone();
        let w_id = w_id.to_string();
        tokio::spawn(async move {
            #[cfg(feature = "native_trigger")]
            windmill_native_triggers::handler::reregister_native_triggers_after_rename(
                &db,
                &authed,
                &w_id,
                native_triggers,
            )
            .await;
        });
    }

    Ok(())
}
