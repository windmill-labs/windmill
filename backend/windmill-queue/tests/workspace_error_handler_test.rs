// Alone in its binary: the worker caches a workspace's handler settings process-wide for 60s,
// keyed by workspace id, so a second test here would read the first one's settings.
mod workspace_error_handler {
    use serde_json::json;
    use sqlx::types::Json;
    use sqlx::{Pool, Postgres};
    use uuid::Uuid;

    use windmill_common::jobs::{JobKind, JobTriggerKind};
    use windmill_common::scripts::{ScriptHash, ScriptLang};
    use windmill_common::users::username_to_permissioned_as;
    use windmill_queue::jobs::{send_error_to_workspace_handler, MiniCompletedJob};

    const WS: &str = "test-workspace";
    const SCRIPT: &str = "f/system/test_script";

    fn failed(trigger_kind: JobTriggerKind) -> MiniCompletedJob {
        MiniCompletedJob {
            id: Uuid::new_v4(),
            workspace_id: WS.to_string(),
            runnable_id: Some(ScriptHash(100001)),
            scheduled_for: chrono::Utc::now(),
            parent_job: None,
            flow_innermost_root_job: None,
            runnable_path: Some(SCRIPT.to_string()),
            kind: JobKind::Script,
            started_at: Some(chrono::Utc::now()),
            permissioned_as: username_to_permissioned_as("test-user"),
            created_by: "test-user".to_string(),
            script_lang: Some(ScriptLang::Deno),
            permissioned_as_email: "test@windmill.dev".to_string(),
            flow_step_id: None,
            trigger_kind: Some(trigger_kind.into()),
            trigger: None,
            priority: None,
            concurrent_limit: None,
            tag: "deno".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            runnable_settings_handle: None,
            build_binary_only: false,
            job_token_scopes: None,
        }
    }

    async fn handler_jobs(db: &Pool<Postgres>) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM v2_job WHERE workspace_id = $1")
            .bind(WS)
            .fetch_one(db)
            .await
            .unwrap()
    }

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn muted_on_ui_runs_skips_only_ui_runs(db: Pool<Postgres>) -> anyhow::Result<()> {
        sqlx::query("UPDATE workspace_settings SET error_handler = $1 WHERE workspace_id = $2")
            .bind(json!({ "path": format!("script/{SCRIPT}"), "muted_on_ui_runs": true }))
            .bind(WS)
            .execute(&db)
            .await?;
        let error = json!({ "error": "boom" });

        send_error_to_workspace_handler(&failed(JobTriggerKind::Ui), false, &db, Json(&error))
            .await?;
        assert_eq!(
            handler_jobs(&db).await,
            0,
            "a ui run must not reach the handler"
        );

        send_error_to_workspace_handler(&failed(JobTriggerKind::Webhook), false, &db, Json(&error))
            .await?;
        assert_eq!(handler_jobs(&db).await, 1, "any other run still does");
        Ok(())
    }
}
