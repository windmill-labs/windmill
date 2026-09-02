/*
 * The job's own token (`$WM_TOKEN`) stays valid well past the job it was minted
 * for, and job logs are persisted to `job_logs` and optionally to object storage,
 * so a script that echoes the token would otherwise park a live credential in
 * durable storage. `run_worker` registers the token with `sensitive_log_masks`
 * for the job it pulled; this pins that the persisted log carries the masked form.
 */

use sqlx::{Pool, Postgres};
use windmill_common::{
    jobs::{JobPayload, RawCode},
    scripts::ScriptLang,
};
use windmill_test_utils::*;

/// Prefix of a serialized job token: `jwt_` plus the base64 of a JWT header.
/// The masked form keeps only `jwt` + the last three characters, so it never matches.
const RAW_TOKEN_PREFIX: &str = "jwt_ey";

#[sqlx::test(fixtures("base"))]
async fn test_job_token_masked_in_persisted_logs(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "echo \"running with --token $WM_TOKEN\"".to_string(),
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
    .run_until_complete(&db, false, port)
    .await;
    assert!(job.success, "job should have succeeded");

    let logs =
        sqlx::query_scalar::<_, Option<String>>("SELECT logs FROM job_logs WHERE job_id = $1")
            .bind(job.id)
            .fetch_one(&db)
            .await?
            .unwrap_or_default();

    assert!(
        !logs.contains(RAW_TOKEN_PREFIX),
        "an unmasked job token reached the persisted logs: {logs}"
    );
    assert!(
        logs.contains("secret value was masked"),
        "expected the masking notice in logs: {logs}"
    );
    Ok(())
}
