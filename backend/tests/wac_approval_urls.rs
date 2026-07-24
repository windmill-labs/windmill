//! `jobs/wac_approval_urls/{job}/{step_key}` mints the resume URLs a WAC
//! workflow routes through its own channel. They must address the same
//! `resume_job` record the step's built-in buttons use — i.e. carry
//! `approval_resume_id(step_key)` — and be signed so the unauthenticated resume
//! route accepts them, without that route becoming any easier to forge.

use sqlx::{Pool, Postgres};
use windmill_common::wac::approval_resume_id;
use windmill_test_utils::*;

const WAC_JOB: &str = "a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1";

/// The minted URLs point at `BASE_URL`, not the ephemeral test server.
fn to_test_url(base: &str, url: &str) -> String {
    let (_, path) = url.split_once("/api/").expect("url has an /api/ segment");
    format!("{base}/{path}")
}

#[sqlx::test(fixtures("base", "wac_approval_urls"))]
async fn wac_approval_urls_bind_to_step_key(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());
    let client = reqwest::Client::new();

    let urls: serde_json::Value = client
        .get(format!(
            "{base}/w/test-workspace/jobs/wac_approval_urls/{WAC_JOB}/manager"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let resume = urls["resume"].as_str().expect("resume url").to_string();
    let manager_id = approval_resume_id("manager");
    assert!(
        resume.contains(&format!("/jobs_u/resume/{WAC_JOB}/{manager_id}/")),
        "resume url must carry the step key's resume_id: {resume}"
    );
    assert!(
        urls["cancel"]
            .as_str()
            .is_some_and(|c| c.contains(&format!("/jobs_u/cancel/{WAC_JOB}/{manager_id}/"))),
        "cancel url must carry the same resume_id: {urls}"
    );
    assert_ne!(
        manager_id,
        approval_resume_id("finance"),
        "distinct approval steps must not share a resume_job record"
    );

    // A signature is only valid for the resume_id it was minted for, so the URL
    // can't be retargeted at another step of the same workflow.
    let retargeted = resume.replace(
        &format!("/{manager_id}/"),
        &format!("/{}/", approval_resume_id("finance")),
    );
    let status = client
        .post(to_test_url(&base, &retargeted))
        .json(&serde_json::json!({}))
        .send()
        .await?
        .status();
    assert!(
        !status.is_success(),
        "signature minted for `manager` must not resume another step (got {status})"
    );

    // The genuine URL resumes without any credential — possession of the
    // signature is the authority, as for the built-in approval buttons.
    let resp = client
        .post(to_test_url(&base, &resume))
        .json(&serde_json::json!({ "ok": true }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "minted resume url must be accepted: {} {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );

    let (approved, value): (bool, sqlx::types::Json<serde_json::Value>) = sqlx::query_as(
        "SELECT approved, value FROM resume_job WHERE job = $1::uuid AND resume_id = $2",
    )
    .bind(WAC_JOB)
    .bind(manager_id as i32)
    .fetch_one(&db)
    .await?;
    assert!(approved);
    assert_eq!(value.0, serde_json::json!({ "ok": true }));

    Ok(())
}

/// Approval rows are consumed oldest-first regardless of resume_id (WIN-2241), so
/// a URL minted for a later step and clicked while an earlier one is pending would
/// otherwise resolve the earlier step with this approver's answer.
#[sqlx::test(fixtures("base", "wac_approval_urls"))]
async fn wac_approval_url_for_another_step_is_rejected(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());
    let client = reqwest::Client::new();

    // The workflow mints both approvals' URLs up front, then suspends on `legal`.
    let mut minted = Vec::new();
    for step_key in ["legal", "finance"] {
        let urls: serde_json::Value = client
            .get(format!(
                "{base}/w/test-workspace/jobs/wac_approval_urls/{WAC_JOB}/{step_key}"
            ))
            .header("Authorization", "Bearer SECRET_TOKEN")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        minted.push(urls["resume"].as_str().expect("resume url").to_string());
    }
    sqlx::query(
        "INSERT INTO v2_job_status (id, workflow_as_code_status) VALUES ($1::uuid, $2)
         ON CONFLICT (id) DO UPDATE SET workflow_as_code_status =
            v2_job_status.workflow_as_code_status || EXCLUDED.workflow_as_code_status",
    )
    .bind(WAC_JOB)
    .bind(sqlx::types::Json(serde_json::json!({
        "_checkpoint": { "pending_steps": { "mode": "approval", "keys": ["legal"], "job_ids": {} } }
    })))
    .execute(&db)
    .await?;

    let resp = client
        .post(to_test_url(&base, &minted[1]))
        .json(&serde_json::json!({}))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    assert_eq!(
        status,
        reqwest::StatusCode::BAD_REQUEST,
        "finance's url must not resume the pending `legal` step: {body}"
    );
    assert!(
        body.contains("finance"),
        "error must name the bound step: {body}"
    );

    // The pending step's own url still works.
    let resp = client
        .post(to_test_url(&base, &minted[0]))
        .json(&serde_json::json!({}))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "the pending step's url must still resume: {}",
        resp.text().await.unwrap_or_default()
    );

    Ok(())
}
