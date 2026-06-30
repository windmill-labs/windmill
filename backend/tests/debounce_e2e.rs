// End-to-end debounce test: drives the FULL real path — real `push()` (EE
// `maybe_debounce` collapsing the batch), real `pull()`, real
// `maybe_apply_debouncing` (claim + accumulate), and a real worker executing the
// surviving flow — then asserts the executed result contains every accumulated item.
// Runs on --features deno_core,enterprise,private (debounce is EE/compile-gated).
#[cfg(all(feature = "deno_core", feature = "enterprise", feature = "private"))]
mod debounce_e2e {
    use serde_json::json;
    use sqlx::{Pool, Postgres};
    use uuid::Uuid;
    use windmill_common::flows::FlowValue;
    use windmill_common::jobs::JobPayload;
    use windmill_common::worker::Connection;
    use windmill_test_utils::*;

    async fn initialize_tracing() {
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let _ = windmill_common::tracing_init::initialize_tracing(
                "test",
                &windmill_common::utils::Mode::Standalone,
                "test",
            );
        });
    }

    /// A one-step flow that debounces on `items` and returns `flow_input.items`,
    /// so the flow's result is exactly the accumulated batch.
    fn debounce_flow() -> FlowValue {
        serde_json::from_value(json!({
            "modules": [{
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export async function main(items: any[]) { return items }",
                    "input_transforms": {
                        "items": { "type": "javascript", "expr": "flow_input.items" }
                    }
                }
            }],
            "debounce_delay_s": 1,
            "debounce_key": "e2e_debounce_key",
            "debounce_args_to_accumulate": ["items"]
        }))
        .expect("valid flow value")
    }

    /// Fire three same-key messages; only the survivor runs, and it must execute once
    /// with ALL accumulated items (none dropped, none duplicated).
    #[sqlx::test(fixtures("base"))]
    async fn debounce_accumulation_runs_once_with_all_items(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // Push 3 debounced flow jobs (same key) — each collapses the previous; the last
        // is the survivor. scheduled_for is ~1s out, so all 3 land before any worker pull.
        let mut survivor = Uuid::nil();
        let mut superseded = Vec::new();
        for n in [1i64, 2, 3] {
            if survivor != Uuid::nil() {
                superseded.push(survivor);
            }
            survivor = RunJob::from(JobPayload::RawFlow {
                value: debounce_flow(),
                path: None,
                restarted_from: None,
            })
            .arg("items", json!([n]))
            .push(&db)
            .await;
        }

        // Run a real worker until the survivor completes.
        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker(Connection::Sql(db.clone()), listener.find(&survivor), port).await;

        let cj = completed_job(survivor, &db).await;
        assert!(cj.success, "survivor flow must succeed");
        let mut result: Vec<i64> =
            serde_json::from_value(cj.json_result().expect("result present"))
                .expect("result is an array of numbers");
        result.sort();
        assert_eq!(
            result,
            vec![1, 2, 3],
            "survivor must execute exactly once with ALL accumulated items"
        );

        // The two superseded messages must have been debounced (skipped), not run.
        // (use a lightweight status query — skipped jobs have NULL started_at, which the
        // full CompletedJob row decoder rejects)
        for s in superseded {
            let skipped: Option<bool> =
                sqlx::query_scalar("SELECT status = 'skipped' FROM v2_job_completed WHERE id = $1")
                    .bind(s)
                    .fetch_optional(&db)
                    .await?;
            assert_eq!(
                skipped,
                Some(true),
                "superseded message {s} must be debounced (skipped), not executed"
            );
        }

        Ok(())
    }
}
