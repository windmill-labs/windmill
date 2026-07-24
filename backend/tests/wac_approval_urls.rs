//! `jobs/wac_approval_urls/{job}/{step_key}` mints the resume URLs a WAC
//! workflow routes through its own channel. They must address the same
//! `resume_job` record the step's built-in buttons use — i.e. carry
//! `approval_resume_id(step_key)` — and be signed so the unauthenticated resume
//! route accepts them, without that route becoming any easier to forge.

use sqlx::{Pool, Postgres};
use windmill_common::wac::approval_resume_id;
use windmill_test_utils::*;

const WAC_JOB: &str = "a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1";
// Distinct keys whose SHA-256 prefixes collide in the u32 the resume routes take.
const APPROVAL_COLLISION_A: &str = "approval-12509";
const APPROVAL_COLLISION_B: &str = "approval-81661";

/// The minted URLs point at `BASE_URL`, not the ephemeral test server.
fn to_test_url(base: &str, url: &str) -> String {
    let (_, path) = url.split_once("/api/").expect("url has an /api/ segment");
    format!("{base}/{path}")
}

/// Park the job on `step_key`, as the worker does when that approval suspends.
async fn awaiting_approval(db: &Pool<Postgres>, step_key: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO v2_job_status (id, workflow_as_code_status) VALUES ($1::uuid, $2)
         ON CONFLICT (id) DO UPDATE SET workflow_as_code_status =
            v2_job_status.workflow_as_code_status || EXCLUDED.workflow_as_code_status",
    )
    .bind(WAC_JOB)
    .bind(sqlx::types::Json(serde_json::json!({
        "_checkpoint": { "pending_steps": { "mode": "approval", "keys": [step_key], "job_ids": {} } }
    })))
    .execute(db)
    .await?;
    Ok(())
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
    awaiting_approval(&db, "manager").await?;
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
    // Nothing pending yet: the link must not bank a row that the next approval
    // to be reached would consume, whichever step that turns out to be.
    let resp = client
        .post(to_test_url(&base, &minted[1]))
        .json(&serde_json::json!({}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "a bound link must not be bankable before its step awaits approval"
    );

    awaiting_approval(&db, "legal").await?;

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

/// The guards around minting: a step key must be non-empty, the job must be in the
/// caller's workspace and be WAC-shaped, and two keys may not share a resume id.
#[sqlx::test(fixtures("base", "wac_approval_urls"))]
async fn wac_approval_urls_mint_guards(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());
    let client = reqwest::Client::new();
    let mint = |ws: &str, job: &str, key: &str| {
        let url = format!("{base}/w/{ws}/jobs/wac_approval_urls/{job}/{key}");
        client.get(url).header("Authorization", "Bearer SECRET_TOKEN").send()
    };

    assert_eq!(
        mint("test-workspace", WAC_JOB, "%20").await?.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "a blank step key must be refused rather than silently meaning `approval`"
    );

    // `v2_job_status` is keyed by job id alone, so the mint must not accept a job
    // from another workspace and stamp its status row.
    assert_eq!(
        mint("test-workspace-2", WAC_JOB, "manager").await?.status(),
        reqwest::StatusCode::NOT_FOUND,
        "a job outside the caller's workspace must not be mintable"
    );

    // APPROVAL_COLLISION_A and _B hash to the same 32-bit resume id, so they would
    // share one resume_job row and one capability.
    assert!(mint("test-workspace", WAC_JOB, APPROVAL_COLLISION_A)
        .await?
        .status()
        .is_success());
    let resp = mint("test-workspace", WAC_JOB, APPROVAL_COLLISION_B).await?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST, "colliding key: {body}");
    assert!(body.contains(APPROVAL_COLLISION_A), "error must name the other key: {body}");

    Ok(())
}
