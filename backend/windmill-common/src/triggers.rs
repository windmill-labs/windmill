use lazy_static::lazy_static;
use quick_cache::sync::Cache;

pub use windmill_types::triggers::*;

lazy_static! {
    pub static ref RUNNABLE_FORMAT_VERSION_CACHE: Cache<RunnableFormatCacheKey, RunnableFormat> =
        Cache::new(1000);
}

/// Identifiers for a native trigger that needs async external service re-registration
/// after a script/flow rename.
#[derive(Clone)]
pub struct NativeTriggerToReregister {
    pub external_id: String,
    pub service_name: String,
}

/// Update `script_path` across all trigger tables when a script or flow is renamed.
/// For long-running triggers (with `server_id`), also resets `server_id = NULL` to force
/// the heartbeat-based restart mechanism to pick up the new config.
/// Native triggers also get their DB `script_path` updated here.
///
/// Note: native triggers additionally need async webhook re-registration with external
/// services — use `windmill_trigger::script_path_rename::update_triggers_on_script_rename`
/// which wraps this function and handles that.
pub async fn update_triggers_script_path(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new_path: &str,
    old_path: &str,
    w_id: &str,
    is_flow: bool,
) -> Result<(), sqlx::Error> {
    // Triggers without server_id (request/response or webhook-based), including native_trigger
    sqlx::query!(
        "WITH \
         t1 AS (UPDATE http_trigger SET script_path = $1 WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t2 AS (UPDATE email_trigger SET script_path = $1 WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4) \
         UPDATE native_trigger SET script_path = $1 WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4",
        new_path,
        old_path,
        w_id,
        is_flow,
    )
    .execute(&mut **tx)
    .await?;

    // Triggers with server_id (long-running listeners, reset server_id to force restart)
    sqlx::query!(
        "WITH \
         t1 AS (UPDATE websocket_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t2 AS (UPDATE kafka_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t3 AS (UPDATE postgres_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t4 AS (UPDATE mqtt_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t5 AS (UPDATE nats_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t6 AS (UPDATE sqs_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4) \
         UPDATE gcp_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4",
        new_path,
        old_path,
        w_id,
        is_flow,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}
