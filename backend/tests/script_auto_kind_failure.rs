use std::collections::HashMap;

use sqlx::{Pool, Postgres};
use windmill_api_client::types::{NewScript, ScriptLang};
use windmill_test_utils::init_client;

fn quick_ns(content: &str, path: &str, kind: Option<&str>) -> NewScript {
    NewScript {
        draft_only: None,
        content: content.into(),
        language: ScriptLang::Bun,
        lock: None,
        parent_hash: None,
        path: path.into(),
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
        description: "".to_string(),
        envs: vec![],
        is_template: None,
        kind: kind.map(|s| s.to_string()),
        summary: "".to_string(),
        tag: None,
        schema: HashMap::new(),
        ws_error_handler_muted: Some(false),
        priority: None,
        delete_after_secs: None,
        timeout: None,
        restart_unless_cancelled: None,
        deployment_message: None,
        concurrency_key: None,
        visible_to_runner_only: None,
        auto_kind: None,
        codebase: None,
        has_preprocessor: None,
        on_behalf_of_email: None,
        assets: vec![],
        modules: None,
    }
}

/// Regression: a `failure`-kind script must never be marked `auto_kind = 'lib'`
/// even if the parser fails to detect a `main` function, because the flow
/// error-handler picker filters out lib scripts and would otherwise hide it.
#[sqlx::test(fixtures("base"))]
async fn failure_kind_script_without_main_is_not_marked_lib(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let (client, _port, _s) = init_client(db.clone()).await;

    // Content with no `main` — TS parser would normally set auto_kind = 'lib'.
    client
        .create_script(
            "test-workspace",
            &quick_ns(
                "export function notMain() { return 42 }",
                "u/test-user/failure_no_main",
                Some("failure"),
            ),
        )
        .await
        .unwrap();

    let auto_kind: Option<String> = sqlx::query_scalar(
        "SELECT auto_kind FROM script \
         WHERE workspace_id = $1 AND path = $2",
    )
    .bind("test-workspace")
    .bind("u/test-user/failure_no_main")
    .fetch_one(&db)
    .await?;

    assert_ne!(
        auto_kind.as_deref(),
        Some("lib"),
        "failure-kind script must not be marked as 'lib' auto_kind, got {:?}",
        auto_kind
    );

    Ok(())
}

/// Sibling: a normal `script` kind WITHOUT main should still be marked `lib`
/// (so it stays hidden from the regular script picker). Guards against an
/// over-broad sanitizer accidentally clearing the value for plain scripts.
#[sqlx::test(fixtures("base"))]
async fn regular_script_without_main_is_still_marked_lib(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (client, _port, _s) = init_client(db.clone()).await;

    client
        .create_script(
            "test-workspace",
            &quick_ns(
                "export function notMain() { return 42 }",
                "u/test-user/script_no_main",
                Some("script"),
            ),
        )
        .await
        .unwrap();

    let auto_kind: Option<String> = sqlx::query_scalar(
        "SELECT auto_kind FROM script \
         WHERE workspace_id = $1 AND path = $2",
    )
    .bind("test-workspace")
    .bind("u/test-user/script_no_main")
    .fetch_one(&db)
    .await?;

    assert_eq!(
        auto_kind.as_deref(),
        Some("lib"),
        "regular script without main should be marked 'lib', got {:?}",
        auto_kind
    );

    Ok(())
}
