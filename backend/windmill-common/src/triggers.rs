use lazy_static::lazy_static;
use quick_cache::sync::Cache;

pub use windmill_types::triggers::*;

lazy_static! {
    pub static ref RUNNABLE_FORMAT_VERSION_CACHE: Cache<RunnableFormatCacheKey, RunnableFormat> =
        Cache::new(1000);
}

/// Update `script_path` across all trigger tables when a runnable (script or flow) is renamed.
/// For long-running triggers (with `server_id`), also resets `server_id = NULL` to force
/// the heartbeat-based restart mechanism to pick up the new config.
pub async fn update_triggers_script_path(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new_path: &str,
    old_path: &str,
    w_id: &str,
    is_flow: bool,
) -> Result<(), sqlx::Error> {
    // Triggers without server_id (request/response or webhook-based)
    sqlx::query!(
        "WITH \
         t1 AS (UPDATE http_trigger SET script_path = $1 WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4) \
         UPDATE email_trigger SET script_path = $1 WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4",
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
