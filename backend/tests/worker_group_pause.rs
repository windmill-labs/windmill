//! In its own test binary: it flips the process-wide `WORKER_CONFIG`, which would stall every
//! other worker test sharing the process.

use futures::StreamExt;
use sqlx::{Pool, Postgres};
use std::time::Duration;
use windmill_common::{
    jobs::{JobPayload, RawCode},
    scripts::ScriptLang,
    worker::{Connection, WORKER_CONFIG},
};
use windmill_test_utils::*;

fn set_paused(paused: bool) {
    let mut wc = (**WORKER_CONFIG.load()).clone();
    wc.paused = paused;
    WORKER_CONFIG.store(std::sync::Arc::new(wc));
}

#[sqlx::test(fixtures("base"))]
async fn paused_worker_leaves_jobs_queued_until_resumed(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    set_paused(true);
    let id = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "echo done".to_string(),
        path: None,
        lock: None,
        language: ScriptLang::Bash,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
        tag: None,
    }))
    .push(&db)
    .await;

    let mut completed = listen_for_completed_jobs(&db).await;
    in_test_worker(
        Connection::Sql(db.clone()),
        async {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let running =
                sqlx::query_scalar::<_, bool>("SELECT running FROM v2_job_queue WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&db)
                    .await
                    .unwrap();
            assert_eq!(
                running,
                Some(false),
                "a paused worker must not pull the job"
            );

            set_paused(false);
            while completed.next().await != Some(id) {}
        },
        port,
    )
    .await;

    assert!(completed_job(id, &db).await.success);
    Ok(())
}
