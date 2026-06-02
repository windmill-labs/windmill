//! Regression test for the single-job read authorization bypass.
//!
//! The single-job read endpoints (`/jobs_u/get`, `/completed/get`,
//! `/completed/get_result`, `/get_args`, `/get_logs`, `/getupdate`, ...) fetch a
//! job through the root DB handle, filtered only by job id + workspace. That is
//! required for the unauthenticated approval / public-trigger / anonymous-job
//! flows, but for a *logged-in* user it meant any workspace member — including a
//! plain viewer with no ACL on the runnable — could read another user's job
//! args/result/logs simply by obtaining the job UUID, even though the same job is
//! hidden from them in `jobs/list` (RLS-filtered) and the underlying script
//! returns 404.
//!
//! The fix (`require_job_read_access`) gates the authenticated case: a caller may
//! read a job they created (covers app components / webhooks / their own runs)
//! or one visible to them under the same RLS as `jobs/list` (admins bypass);
//! otherwise 404. Unauthenticated access is unchanged (anonymous jobs only).
//!
//! This test pins down, against the `jobs_read_auth` fixture:
//!   - a viewer is denied the victim job's full record / result / result_maybe /
//!     args / logs / live update by UUID, and the secret never appears in the
//!     body (the core fix; pre-fix these returned 200 with the secret),
//!   - the job's owner and an admin can still read it (no over-blocking),
//!   - the "app component" affordance survives: a viewer who *launched* a job
//!     (created_by) running as someone else's identity can still read its result,
//!   - unauthenticated behavior is unchanged: anonymous jobs readable, the
//!     non-anonymous victim job rejected.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const VICTIM: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const APP_JOB: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
const ANON_JOB: &str = "cccccccc-cccc-cccc-cccc-cccccccccccc";
const FLOW_JOB: &str = "dddddddd-dddd-dddd-dddd-dddddddddddd";
const STEP_JOB: &str = "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee";
// Deep nesting: top (not visible) -> mid (visible via folder) -> deep leaf.
const TOP_SECRET_FLOW: &str = "ffffffff-ffff-ffff-ffff-ffffffffffff";
const DEEP_LEAF_JOB: &str = "88888888-8888-8888-8888-888888888888";

// Secrets that must never leak to an unauthorized viewer.
const RESULT_SECRET: &str = "RESULT_SECRET";
const ARGS_SECRET: &str = "LEAK_TEST_ARGS";
const LOGS_SECRET: &str = "LEAK_TEST_LOGS";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

async fn get(base: &str, path: &str, token: Option<&str>) -> (reqwest::StatusCode, String) {
    let mut req = client().get(format!("{base}/{path}"));
    if let Some(token) = token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    let resp = req.send().await.expect("request");
    let status = resp.status();
    let body = resp.text().await.expect("body");
    (status, body)
}

#[sqlx::test(fixtures("base", "jobs_read_auth"))]
async fn test_single_job_read_authorization(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs_u");
    // result_by_id / get_otel_traces live on the authed `/jobs` service, not `/jobs_u`.
    let authed_base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    // The endpoints that return the victim job's sensitive data by UUID.
    let endpoints = [
        ("get", format!("get/{VICTIM}")),
        ("completed/get", format!("completed/get/{VICTIM}")),
        (
            "completed/get_result",
            format!("completed/get_result/{VICTIM}"),
        ),
        (
            "completed/get_result_maybe",
            format!("completed/get_result_maybe/{VICTIM}"),
        ),
        ("get_args", format!("get_args/{VICTIM}")),
        ("get_logs", format!("get_logs/{VICTIM}")),
        (
            "get_completed_logs_tail",
            format!("get_completed_logs_tail/{VICTIM}"),
        ),
        ("get_flow_all_logs", format!("get_flow_all_logs/{VICTIM}")),
        (
            "completed/get_timing",
            format!("completed/get_timing/{VICTIM}"),
        ),
        ("getupdate", format!("getupdate/{VICTIM}?only_result=true")),
    ];

    // ---- CORE REGRESSION: the viewer (test-user-3) is denied on every endpoint
    //      and no secret ever appears in the body. Pre-fix these returned 200
    //      and leaked the secret.
    for (name, path) in &endpoints {
        let (status, body) = get(&base, path, Some("SECRET_TOKEN_3")).await;
        assert_eq!(
            status,
            reqwest::StatusCode::NOT_FOUND,
            "viewer must get 404 on {name} (got {status}): {body}"
        );
        for secret in [RESULT_SECRET, ARGS_SECRET, LOGS_SECRET] {
            assert!(
                !body.contains(secret),
                "viewer response for {name} leaked `{secret}`: {body}"
            );
        }
    }

    // ---- NO OVER-BLOCKING: the job's owner (test-user-2) can read its result.
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{VICTIM}"),
        Some("SECRET_TOKEN_2"),
    )
    .await;
    assert!(
        status.is_success(),
        "owner must still read their own job result (got {status}): {body}"
    );
    assert!(
        body.contains(RESULT_SECRET),
        "owner result must contain the value: {body}"
    );

    // ---- ADMIN BYPASS: an admin (test-user) can read any job in the workspace.
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{VICTIM}"),
        Some("SECRET_TOKEN"),
    )
    .await;
    assert!(
        status.is_success(),
        "admin must read any job (got {status}): {body}"
    );
    assert!(body.contains(RESULT_SECRET), "admin result body: {body}");

    // ---- APP AFFORDANCE: a viewer who LAUNCHED a job (created_by = viewer) that
    //      runs as another identity (permissioned_as = test-user-2,
    //      visible_to_owner = false) can still read its result. This is the app
    //      component-polling path; the fix must not break it.
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{APP_JOB}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert!(
        status.is_success(),
        "launcher must read a job they created even without ACL on the runnable (got {status}): {body}"
    );
    assert!(
        body.contains("visible_to_launcher"),
        "launcher should get the result they polled: {body}"
    );

    // ---- AUTHED `/jobs` endpoints in the same class: result_by_id (flow node
    //      result) and get_otel_traces (job telemetry). The viewer must be denied
    //      the victim by UUID. The auth gate runs before result/trace resolution,
    //      so 404 here is the gate, not incidental resolution failure.
    let (status, body) = get(
        &authed_base,
        &format!("result_by_id/{VICTIM}/somenode"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "viewer must get 404 on result_by_id (got {status}): {body}"
    );
    assert!(!body.contains(RESULT_SECRET), "result_by_id leaked: {body}");

    let (status, body) = get(
        &authed_base,
        &format!("get_otel_traces/{VICTIM}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "viewer must get 404 on get_otel_traces (got {status}): {body}"
    );

    // ---- FLOW VISIBILITY INHERITANCE: test-user-3 has folder ACL on the flow
    //      `f/shared/flow1` (run by test-user-2) but did NOT launch it, and has no
    //      ACL on the step's inner runnable `u/test-user-2/inner_secret`. They must
    //      still be able to (a) read the flow they can see, and (b) inspect its
    //      step result — visibility is inherited from the flow root. A naive
    //      "same as list" gate would 404 the step and break the flow-run UI.
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{FLOW_JOB}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert!(
        status.is_success(),
        "viewer with folder ACL must read the flow they can see (got {status}): {body}"
    );
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{STEP_JOB}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert!(
        status.is_success(),
        "viewer must inspect a step of a flow they can see, even without ACL on the step's runnable (got {status}): {body}"
    );
    assert!(
        body.contains("STEP_RESULT_INHERITED"),
        "step result should be returned via flow-root inheritance: {body}"
    );

    // ---- DEEP NESTING / MIDDLE-LAYER VISIBILITY: the deep leaf's root_job is the
    //      top flow (NOT visible to test-user-3), but an intermediate sub-flow
    //      (f/shared/mid) IS visible. Reading the leaf must succeed via that middle
    //      ancestor — i.e. the full parent chain is walked, not just [self, root].
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{DEEP_LEAF_JOB}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert!(
        status.is_success(),
        "deep leaf must be readable via a visible intermediate sub-flow (got {status}): {body}"
    );
    assert!(
        body.contains("DEEP_STEP_INHERITED"),
        "deep leaf result should be returned via mid-ancestor visibility: {body}"
    );
    // ...but the top flow itself, in a folder the viewer cannot read, stays denied.
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{TOP_SECRET_FLOW}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "top flow in an unreadable folder must stay denied (got {status}): {body}"
    );

    // ---- UNAUTHENTICATED, unchanged: an anonymous-created job is readable
    //      without a token (public trigger / public app result polling).
    let (status, body) = get(&base, &format!("completed/get_result/{ANON_JOB}"), None).await;
    assert!(
        status.is_success(),
        "anonymous job must remain readable unauthenticated (got {status}): {body}"
    );

    // ---- UNAUTHENTICATED, unchanged: the non-anonymous victim job is rejected
    //      for an unauthenticated caller (400, the pre-existing guard).
    let (status, body) = get(&base, &format!("completed/get_result/{VICTIM}"), None).await;
    assert_eq!(
        status,
        reqwest::StatusCode::BAD_REQUEST,
        "unauthenticated access to a non-anonymous job must stay rejected (got {status}): {body}"
    );
    assert!(
        !body.contains(RESULT_SECRET),
        "unauth body must not leak: {body}"
    );

    Ok(())
}
