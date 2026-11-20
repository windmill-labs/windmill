//! A flow job pulled through a caller's own query (an agent worker's) is handed to the generic
//! flow tag, flow previews under the preview tag override included: an agent cannot drive a flow.
//!
//! Run with:
//!   cargo test -p windmill-queue --test custom_query_flow_requeue_test

use std::collections::HashMap;
use std::sync::atomic::Ordering;

use sqlx::{Pool, Postgres};
use windmill_common::jobs::JobPayload;
use windmill_common::worker::{make_pull_query, PREVIEW_TAGS_OVERRIDE};
use windmill_queue::{pull, push, PushArgs, PushIsolationLevel};

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn flow_preview_on_an_agent_tag_goes_to_the_flow_tag(db: Pool<Postgres>) {
    PREVIEW_TAGS_OVERRIDE.store(true, Ordering::Relaxed);

    let payload = JobPayload::RawFlow {
        value: serde_json::from_value(serde_json::json!({ "modules": [] })).unwrap(),
        path: None,
        restarted_from: None,
    };
    let args = HashMap::new();
    let (id, tx) = push(
        &db,
        PushIsolationLevel::IsolatedRoot(db.clone()),
        "test-workspace",
        payload,
        PushArgs { extra: None, args: &args },
        "test-user",
        "test@windmill.dev",
        "u/test-user".to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        Some("agentx".to_string()),
        None,
        None,
        None,
        None,
        false,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("push");
    tx.commit().await.expect("commit push");

    let query = (String::new(), make_pull_query(&["agentx".to_string()]));
    let pulled = pull(&db, false, "ag-test", Some(&query))
        .await
        .expect("pull");
    assert!(pulled.job.is_none(), "an agent must not receive a flow job");

    let (queue_tag, running): (String, bool) =
        sqlx::query_as("SELECT tag, running FROM v2_job_queue WHERE id = $1")
            .bind(id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!((queue_tag.as_str(), running), ("flow", false));
    // Its steps inherit this one.
    let job_tag: String = sqlx::query_scalar("SELECT tag FROM v2_job WHERE id = $1")
        .bind(id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(job_tag, "agentx");
}
