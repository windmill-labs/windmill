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
// A queued/running job (no completed row) owned by test-user-2.
const RUNNING_JOB: &str = "77777777-7777-7777-7777-777777777777";
// An app-component job launched BY the admin embed viewer (created_by test-user).
const EMBED_OWN_JOB: &str = "12121212-1212-1212-1212-121212121212";
// A QUEUED job launched by the embed viewer (created_by test-user) — cancelable by it.
const EMBED_OWN_QUEUED: &str = "13131313-1313-1313-1313-131313131313";

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

async fn post(base: &str, path: &str, token: Option<&str>) -> (reqwest::StatusCode, String) {
    let mut req = client()
        .post(format!("{base}/{path}"))
        .json(&serde_json::json!({}));
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
            "get_flow_all_logs_structured",
            format!("get_flow_all_logs_structured/{VICTIM}"),
        ),
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
            reqwest::StatusCode::FORBIDDEN,
            "viewer must get 403 on {name} (got {status}): {body}"
        );
        for secret in [RESULT_SECRET, ARGS_SECRET, LOGS_SECRET] {
            assert!(
                !body.contains(secret),
                "viewer response for {name} leaked `{secret}`: {body}"
            );
        }
    }

    // The 403 for an existing-but-forbidden job carries actionable guidance
    // (request a share link), distinguishing it from a plain not-found.
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{VICTIM}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
    assert!(
        body.to_lowercase().contains("share"),
        "403 body should guide the user to request a share link: {body}"
    );

    // A genuinely non-existent job is a 404, not a 403 — existence is only disclosed
    // for jobs that actually exist in the workspace.
    let missing = "00000000-0000-4000-8000-000000000000";
    let (status, _) = get(
        &base,
        &format!("completed/get_result/{missing}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "a non-existent job must be 404, not 403 (got {status})"
    );

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
        reqwest::StatusCode::FORBIDDEN,
        "viewer must get 403 on result_by_id (got {status}): {body}"
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
        reqwest::StatusCode::FORBIDDEN,
        "viewer must get 403 on get_otel_traces (got {status}): {body}"
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
        reqwest::StatusCode::FORBIDDEN,
        "top flow in an unreadable folder must stay denied (got {status}): {body}"
    );

    // ---- APP EMBED TOKEN: confined to jobs the viewer LAUNCHED, not everything
    //      the (admin) viewer can otherwise read. The token carries the `app_embed`
    //      sentinel; an admin's normal token reads VICTIM (asserted above), but the
    //      embed token must stop at the `created_by == viewer` grant so user-authored
    //      app JS can't reuse it to read unrelated jobs by UUID.
    // Its own launched component job (created_by == viewer) still reads.
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{EMBED_OWN_JOB}"),
        Some("EMBED_APP_TOKEN"),
    )
    .await;
    assert!(
        status.is_success(),
        "embed token must read a job it launched (got {status}): {body}"
    );
    assert!(
        body.contains("EMBED_OWN_RESULT"),
        "embed token should get its own launched job result: {body}"
    );
    // The VICTIM job — created by another user but readable by this admin viewer's
    // normal token (asserted above) — is denied to the embed token across result /
    // logs / live update. NotFound (not 403) so the untrusted app can't even probe
    // existence, and no secret leaks.
    for path in [
        format!("completed/get_result/{VICTIM}"),
        format!("get_logs/{VICTIM}"),
        format!("getupdate/{VICTIM}?only_result=true"),
    ] {
        let (status, body) = get(&base, &path, Some("EMBED_APP_TOKEN")).await;
        assert_eq!(
            status,
            reqwest::StatusCode::NOT_FOUND,
            "embed token must not read a job it did not launch ({path}, got {status}): {body}"
        );
        for secret in [RESULT_SECRET, ARGS_SECRET, LOGS_SECRET] {
            assert!(
                !body.contains(secret),
                "embed token response for {path} leaked `{secret}`: {body}"
            );
        }
    }

    // ---- APP EMBED TOKEN: cancellation confined to the app's own jobs. The token
    //      may cancel a job it launched (created_by == viewer), but `cancel_job_api`
    //      denies (NotFound) a job created by someone else, even though cancel
    //      otherwise has no per-job ownership check.
    let (status, body) = post(
        &base,
        &format!("queue/cancel/{EMBED_OWN_QUEUED}"),
        Some("EMBED_APP_TOKEN"),
    )
    .await;
    assert!(
        status.is_success(),
        "embed token must cancel a job it launched (got {status}): {body}"
    );
    let (status, body) = post(
        &base,
        &format!("queue/cancel/{RUNNING_JOB}"),
        Some("EMBED_APP_TOKEN"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "embed token must not cancel another user's job (got {status}): {body}"
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

    // ---- SHARE READ LINK (view_token) ----
    // The owner (test-user-2) mints a share token for the victim job.
    let (status, mint_body) = get(
        &authed_base,
        &format!("job_view_token/{VICTIM}"),
        Some("SECRET_TOKEN_2"),
    )
    .await;
    assert!(
        status.is_success(),
        "owner must be able to mint a share token (got {status}): {mint_body}"
    );
    let token = mint_body.trim().trim_matches('"').to_string();
    assert!(
        token.starts_with(VICTIM),
        "token must encode the job id: {token}"
    );

    // The viewer (no ACL) can now read the victim job via the share link.
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{VICTIM}?view_token={token}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert!(
        status.is_success(),
        "view_token must grant the viewer read of the shared job (got {status}): {body}"
    );
    assert!(
        body.contains(RESULT_SECRET),
        "shared job result must be returned with a valid view_token: {body}"
    );
    // ...and its args/logs too (whole detail page).
    let (status, _) = get(
        &base,
        &format!("get_args/{VICTIM}?view_token={token}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert!(
        status.is_success(),
        "view_token must also grant args (got {status})"
    );

    // The token is scoped: it does NOT authorize an unrelated job.
    let (status, _) = get(
        &base,
        &format!("completed/get_result/{ANON_JOB}?view_token={token}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::FORBIDDEN,
        "a victim-scoped token must not authorize a different job (got {status})"
    );

    // A garbage token is rejected (falls through to the normal 404).
    let (status, _) = get(
        &base,
        &format!("completed/get_result/{VICTIM}?view_token={VICTIM}.deadbeef"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::FORBIDDEN,
        "an invalid view_token must not grant access (got {status})"
    );

    // A share token authorizes the shared job's whole flow subtree: the owner mints
    // for the top secret flow, and the viewer can then read its deep leaf.
    let (status, mint_body) = get(
        &authed_base,
        &format!("job_view_token/{TOP_SECRET_FLOW}"),
        Some("SECRET_TOKEN_2"),
    )
    .await;
    assert!(
        status.is_success(),
        "owner mints token for top flow (got {status}): {mint_body}"
    );
    let top_token = mint_body.trim().trim_matches('"').to_string();
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{DEEP_LEAF_JOB}?view_token={top_token}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert!(
        status.is_success(),
        "a flow's share token must authorize its deep descendants (got {status}): {body}"
    );

    // A viewer who cannot read a job cannot mint a share token for it.
    let (status, _) = get(
        &authed_base,
        &format!("job_view_token/{TOP_SECRET_FLOW}"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::FORBIDDEN,
        "a non-reader must not be able to mint a share token (got {status})"
    );

    // ---- TAG-SCOPED token must not mint a token outside its allowed tags ----
    // SCOPED_DENO_TOKEN (test-user-2, scope `if_jobs:filter_tags:deno`) can read both
    // VICTIM (tag deno) and FLOW_JOB (tag flow) by RLS, but minting must honor the
    // tag scope: allowed for the deno job, denied for the flow job.
    let (status, body) = get(
        &authed_base,
        &format!("job_view_token/{VICTIM}"),
        Some("SCOPED_DENO_TOKEN"),
    )
    .await;
    assert!(
        status.is_success(),
        "tag-scoped token may mint for an in-scope (deno) job (got {status}): {body}"
    );
    let (status, _) = get(
        &authed_base,
        &format!("job_view_token/{FLOW_JOB}"),
        Some("SCOPED_DENO_TOKEN"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "tag-scoped token must NOT mint for an out-of-scope (flow) job (got {status})"
    );

    // ---- USE side: a tag-scoped token must not use someone else's valid view_token
    //      to read an out-of-scope job, even via handlers that don't tag-filter their
    //      data query (result_by_id, get_otel_traces, get_flow_debug_info). ----
    // An unscoped owner mints a valid token for the flow (tag 'flow').
    let (status, mint_body) = get(
        &authed_base,
        &format!("job_view_token/{FLOW_JOB}"),
        Some("SECRET_TOKEN_2"),
    )
    .await;
    assert!(
        status.is_success(),
        "owner mints flow token (got {status}): {mint_body}"
    );
    let flow_token = mint_body.trim().trim_matches('"').to_string();

    // The deno-scoped token presents that valid flow token to the non-tag-filtered
    // endpoints — must still be denied (flow tag is out of its scope).
    for path in [
        format!("get_otel_traces/{FLOW_JOB}?view_token={flow_token}"),
        format!("result_by_id/{FLOW_JOB}/somenode?view_token={flow_token}"),
    ] {
        let (status, _) = get(&authed_base, &path, Some("SCOPED_DENO_TOKEN")).await;
        assert_eq!(
            status,
            reqwest::StatusCode::NOT_FOUND,
            "tag-scoped token must not use a view_token to read an out-of-scope job ({path}, got {status})"
        );
    }

    // ...but the deno-scoped token CAN use an in-scope (deno) view_token.
    let (status, body) = get(
        &base,
        &format!("completed/get_result/{VICTIM}?view_token={token}"),
        Some("SCOPED_DENO_TOKEN"),
    )
    .await;
    assert!(
        status.is_success(),
        "tag-scoped token may use a view_token for an in-scope (deno) job (got {status}): {body}"
    );

    // ---- get_result_maybe?get_started=true must authorize before disclosing the
    //      running-state of a queued (not-yet-completed) private job. ----
    // Viewer (no ACL) must be denied rather than told the job is started.
    let (status, body) = get(
        &base,
        &format!("completed/get_result_maybe/{RUNNING_JOB}?get_started=true"),
        Some("SECRET_TOKEN_3"),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::FORBIDDEN,
        "viewer must be denied the running-state of a private queued job (got {status}): {body}"
    );
    assert!(
        !body.contains("\"started\""),
        "denied response must not disclose started-state: {body}"
    );
    // The owner still gets the in-progress response.
    let (status, body) = get(
        &base,
        &format!("completed/get_result_maybe/{RUNNING_JOB}?get_started=true"),
        Some("SECRET_TOKEN_2"),
    )
    .await;
    assert!(
        status.is_success() && body.contains("\"started\":true"),
        "owner must see the running job as started (got {status}): {body}"
    );

    Ok(())
}
