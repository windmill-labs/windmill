#![cfg(feature = "deno_core")]

use std::collections::HashMap;

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::{flows::Retry, jobs::JobTriggerKind, triggers::TriggerMetadata};
use windmill_queue::PushArgsOwned;
use windmill_test_utils::*;

/// A trigger with a retry or an error handler runs its target as the only step of a flow.
/// A Hub script step is addressed by path, and must still run the script's preprocessor
/// on the trigger's `event`, as the same trigger without a retry does.
#[sqlx::test(fixtures("base"))]
async fn hub_target_with_retry_runs_its_preprocessor(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let version = "990000004";
    let hub_dir = &*windmill_common::worker::HUB_CACHE_DIR;
    tokio::fs::create_dir_all(hub_dir).await?;
    tokio::fs::write(
        format!("{hub_dir}/{version}"),
        json!({
            "content": "export async function preprocessor(event: any) { return { x: `pre:${event.kind}` } }\n\
                        export async function main(x: string) { return x }\n",
            "lockfile": "{}\n//bun.lock\n<empty>",
            "language": "bun",
            "schema": {},
            "summary": null,
        })
        .to_string(),
    )
    .await?;

    let retry: Retry =
        serde_json::from_value(json!({ "constant": { "attempts": 1, "seconds": 1 } }))?;
    let args = PushArgsOwned {
        args: HashMap::from([(
            "event".to_string(),
            serde_json::value::to_raw_value(&json!({ "kind": "http" }))?,
        )]),
        extra: None,
    };
    let listener = listen_for_completed_jobs(&db).await;
    let (id, _, _, _, _) = windmill_trigger::trigger_helpers::trigger_runnable_inner(
        &db,
        None,
        None,
        windmill_api_auth::ApiAuthed {
            email: "test@windmill.dev".to_string(),
            username: "test-user".to_string(),
            is_admin: true,
            ..Default::default()
        },
        "test-workspace",
        &format!("hub/{version}/test/pre"),
        false,
        args,
        Some(&sqlx::types::Json(retry)),
        None,
        None,
        "u/test-user/route".to_string(),
        None,
        TriggerMetadata::new(Some("u/test-user/route".to_string()), JobTriggerKind::Http),
        None,
    )
    .await?;

    in_test_worker(&db, listener.find(&id), server.addr.port()).await;
    let job = completed_job(id, &db).await;
    assert_eq!(job.json_result(), Some(json!("pre:http")));
    Ok(())
}
