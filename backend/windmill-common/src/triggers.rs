use lazy_static::lazy_static;
use quick_cache::sync::Cache;

pub use windmill_types::triggers::*;

lazy_static! {
    pub static ref RUNNABLE_FORMAT_VERSION_CACHE: Cache<RunnableFormatCacheKey, RunnableFormat> =
        Cache::new(1000);
}

/// Marks a moved trigger as not yet re-pointed at its new path.
///
/// Written in the same statement that moves the row, so it is committed before the rename returns.
/// Re-pointing the webhook happens afterwards and off the request, and its outcome — success or
/// failure — replaces this. Anything that stops it getting that far, a shutdown included, leaves
/// the trigger visibly unfinished rather than quietly pointing at a path nothing serves.
pub const REREGISTRATION_PENDING: &str =
    "The runnable was renamed and the webhook registered on the service has not been re-pointed at \
     it yet. If this persists, save the trigger again.";

/// A `native_trigger` row carried onto the new path by a rename. The webhook registered on
/// the external service embeds the runnable path, so each of these still has to be
/// re-registered — see `windmill_native_triggers::rename`.
#[derive(Debug, Clone)]
pub struct MovedNativeTrigger {
    pub service_name: String,
    pub external_id: String,
    /// Where this rename put the trigger. Re-registration mints a `jobs:run:*` token for the
    /// runnable it finds, so it must confirm the row still names this one: anything that moved it
    /// afterwards was authorized separately, and its choice is not this rename's to overwrite.
    pub script_path: String,
    pub is_flow: bool,
}

/// Update `script_path` across all trigger tables when a runnable (script or flow) is renamed.
/// For long-running triggers (with `server_id`), also resets `server_id = NULL` to force
/// the heartbeat-based restart mechanism to pick up the new config.
///
/// Returns the `native_trigger` rows that moved.
pub async fn update_triggers_script_path(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new_path: &str,
    old_path: &str,
    w_id: &str,
    is_flow: bool,
) -> Result<Vec<MovedNativeTrigger>, sqlx::Error> {
    // Triggers without server_id (request/response or webhook-based).
    // `native_trigger` rows are only listed when a runnable still exists at `script_path`,
    // so a row left behind on the old path is invisible in the UI and unrecoverable.
    let moved_native_triggers = sqlx::query_as!(
        MovedNativeTrigger,
        "WITH \
         t1 AS (UPDATE http_trigger SET script_path = $1 WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t2 AS (UPDATE email_trigger SET script_path = $1 WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4) \
         UPDATE native_trigger SET script_path = $1, updated_at = NOW(), error = $5 WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4 \
         RETURNING service_name::text AS \"service_name!\", external_id, script_path, is_flow",
        new_path,
        old_path,
        w_id,
        is_flow,
        REREGISTRATION_PENDING,
    )
    .fetch_all(&mut **tx)
    .await?;

    // Triggers with server_id (long-running listeners, reset server_id to force restart)
    sqlx::query!(
        "WITH \
         t1 AS (UPDATE websocket_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t2 AS (UPDATE kafka_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t3 AS (UPDATE postgres_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t4 AS (UPDATE mqtt_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t5 AS (UPDATE nats_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t6 AS (UPDATE sqs_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4), \
         t7 AS (UPDATE amqp_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4) \
         UPDATE gcp_trigger SET script_path = $1, server_id = NULL WHERE script_path = $2 AND workspace_id = $3 AND is_flow = $4",
        new_path,
        old_path,
        w_id,
        is_flow,
    )
    .execute(&mut **tx)
    .await?;

    Ok(moved_native_triggers)
}
