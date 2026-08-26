use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn assert_2xx(status: u16, body: &str, endpoint: &str) {
    assert!(
        (200..300).contains(&status),
        "{endpoint} returned {status}: {body}",
    );
}

fn assert_route_reachable(status: u16, body: &str, endpoint: &str) {
    assert!(
        status != 404 || !body.is_empty(),
        "Router-level 404 for {endpoint}",
    );
}

async fn insert_completed_job(db: &Pool<Postgres>) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, kind, tag, args)
         VALUES ($1, 'test-workspace', 'test-user', 'u/test-user', 'script', 'deno', '{}'::jsonb)",
    )
    .bind(id)
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, result, status)
         VALUES ($1, 'test-workspace', 100, '42'::jsonb, 'success')",
    )
    .bind(id)
    .execute(db)
    .await
    .unwrap();
    id
}

#[allow(dead_code)]
async fn create_script(port: u16) -> String {
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": "u/test-user/test_job_script",
            "summary": "test",
            "description": "",
            "content": "export function main() { return 42; }",
            "language": "deno",
            "schema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {},
                "required": []
            }
        }))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "create script: {}",
        resp.status()
    );
    "u/test-user/test_job_script".to_string()
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_authed_list_and_count(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    // --- List/count endpoints (2xx with empty results) ---

    let resp = authed(client().get(format!("{base}/list"))).send().await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/list",
    );

    let resp = authed(client().get(format!("{base}/queue/list")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/queue/list",
    );

    let resp = authed(client().get(format!("{base}/queue/count")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/queue/count",
    );

    let resp = authed(client().get(format!("{base}/completed/list")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/list",
    );

    let resp = authed(client().get(format!("{base}/completed/count")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/count",
    );

    // --- Global endpoints ---

    let resp = client()
        .get(format!("http://localhost:{port}/api/jobs/db_clock"))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/db_clock",
    );

    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/jobs/completed/count_by_tag"
    )))
    .send()
    .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/count_by_tag",
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_authed_completed_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    let job_id = insert_completed_job(&db).await;

    let resp = authed(client().get(format!("{base}/completed/get/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/get",
    );

    let resp = authed(client().get(format!("{base}/completed/get_result/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/get_result",
    );

    let resp = authed(client().get(format!("{base}/completed/get_result_maybe/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/get_result_maybe",
    );

    let resp = authed(client().get(format!("{base}/completed/get_timing/{job_id}")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/completed/get_timing",
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_authed_run_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    // Run preview — no pre-existing script needed
    let resp = authed(client().post(format!("{base}/run/preview")))
        .json(&json!({
            "content": "export function main() { return 1; }",
            "language": "deno",
            "args": {}
        }))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /jobs/run/preview",
    );

    // Run preview flow
    let resp = authed(client().post(format!("{base}/run/preview_flow")))
        .json(&json!({
            "value": {"modules": []},
            "args": {}
        }))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /jobs/run/preview_flow",
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_jobs_authed_reachability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");
    let fake = Uuid::nil();

    // These need complex runtime but should hit the handler (not 404)

    let resp = authed(client().post(format!("{base}/flow/resume/{fake}")))
        .json(&json!({}))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /jobs/flow/resume",
    );

    let resp = authed(client().get(format!("{base}/job_signature/{fake}/1")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/job_signature",
    );

    let resp = authed(client().get(format!("{base}/resume_urls/{fake}/1")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/resume_urls",
    );

    let resp = authed(client().get(format!("{base}/result_by_id/{fake}/step1")))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /jobs/result_by_id",
    );

    // Sent the way the generated client sends it. A handler whose `Path` tuple has drifted from
    // the route is rejected by axum before it runs, which surfaces as a routing error rather
    // than the handler's own answer, so reaching the handler is what this pins.
    let resp = authed(client().post(format!("{base}/restart/f/{fake}")))
        .json(&json!({ "step_id": "a" }))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_route_reachable(status, &body, "POST /jobs/restart/f");
    assert!(
        !body.contains("path arguments"),
        "POST /jobs/restart/f never reached its handler: {status} {body}",
    );
    #[cfg(not(feature = "enterprise"))]
    assert!(
        body.contains("only available in enterprise version"),
        "POST /jobs/restart/f must report the enterprise gate outside EE: {status} {body}",
    );

    let resp = authed(client().post(format!("{base}/run/workflow_as_code/{fake}/main")))
        .json(&json!({}))
        .send()
        .await?;
    assert_route_reachable(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /jobs/run/workflow_as_code",
    );

    Ok(())
}

/// Resolving is authorized by row-level security on `v2_job` alone: `v2_job_completed`
/// has RLS disabled, so a regression here silently lets any member annotate any run.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_resolve_completed_jobs_scoping(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    async fn seed(db: &Pool<Postgres>, owner: &str, status: &str, script: &str) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email,
                                 kind, tag, runnable_path, visible_to_owner)
             VALUES ($1, 'test-workspace', $2, $3, $4, 'script', 'deno', $5, true)",
        )
        .bind(id)
        .bind(owner)
        .bind(format!("u/{owner}"))
        .bind(format!("{owner}@windmill.dev"))
        .bind(format!("u/{owner}/{script}"))
        .execute(db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, result, status)
             VALUES ($1, 'test-workspace', 100, '42'::jsonb, $2::job_status)",
        )
        .bind(id)
        .bind(status)
        .execute(db)
        .await
        .unwrap();
        id
    }

    let mine = seed(&db, "test-user-2", "failure", "some_script").await;
    let theirs = seed(&db, "test-user", "failure", "some_script").await;
    let mine_succeeded = seed(&db, "test-user-2", "success", "some_script").await;
    let unrelated_success = seed(&db, "test-user-2", "success", "other_script").await;

    // test-user-2 is a plain non-admin, non-operator member of the workspace.
    let member = |b: reqwest::RequestBuilder| b.header("Authorization", "Bearer SECRET_TOKEN_2");

    let resp = member(client().post(format!("{base}/completed/resolve")))
        .json(&json!({ "job_ids": [mine, theirs, mine_succeeded], "note": "expected" }))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /jobs/completed/resolve");
    let resolved: Vec<Uuid> = serde_json::from_str(&body)?;
    assert_eq!(
        resolved,
        vec![mine],
        "only the caller's own failed run may be resolved"
    );

    let row: (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT c.status::text, r.resolved_by, r.note
           FROM v2_job_completed c JOIN job_resolution r ON r.job_id = c.id
          WHERE c.id = $1",
    )
    .bind(mine)
    .fetch_one(&db)
    .await?;
    assert_eq!(row.0, "failure", "resolving must not change job status");
    // Who resolved it and why are enterprise-only; CE records only that it was handled.
    #[cfg(feature = "enterprise")]
    {
        assert_eq!(row.1.as_deref(), Some("test-user-2"));
        assert_eq!(row.2.as_deref(), Some("expected"));
    }
    #[cfg(not(feature = "enterprise"))]
    {
        assert_eq!(row.1, None, "attribution is an EE feature");
        assert_eq!(row.2, None, "notes are an EE feature");
    }

    // Re-resolving without a note must keep the existing one: a bulk selection routinely
    // includes already-resolved failures, and silently blanking their notes loses the only
    // record of why they were handled.
    let resp = member(client().post(format!("{base}/completed/resolve")))
        .json(&json!({ "job_ids": [mine] }))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "re-resolve without a note",
    );
    let kept: Option<String> =
        sqlx::query_scalar("SELECT note FROM job_resolution WHERE job_id = $1")
            .bind(mine)
            .fetch_one(&db)
            .await?;
    #[cfg(feature = "enterprise")]
    assert_eq!(kept.as_deref(), Some("expected"));
    #[cfg(not(feature = "enterprise"))]
    assert_eq!(
        kept, None,
        "no note is stored outside EE, so none can be lost"
    );

    let resp = member(client().post(format!("{base}/completed/unresolve")))
        .json(&json!({ "job_ids": [mine, theirs] }))
        .send()
        .await?;
    let body = resp.text().await?;
    let unresolved: Vec<Uuid> = serde_json::from_str(&body)?;
    assert_eq!(unresolved, vec![mine]);

    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM job_resolution")
        .fetch_one(&db)
        .await?;
    assert_eq!(remaining, 0);

    // A supersession the server cannot prove must resolve nothing: `unrelated_success` is
    // visible and successful, so only the runnable check stands between an arbitrary caller and
    // a failure stamped with provenance that never happened.
    let resp = member(client().post(format!("{base}/completed/resolve")))
        .json(&json!({ "job_ids": [mine], "superseded_by": unrelated_success }))
        .send()
        .await?;
    let body = resp.text().await?;
    let ids: Vec<Uuid> = serde_json::from_str(&body)?;
    assert!(
        ids.is_empty(),
        "a supersession by an unrelated run must be rejected, got {body}"
    );
    let none: Option<Uuid> =
        sqlx::query_scalar("SELECT job_id FROM job_resolution WHERE job_id = $1")
            .bind(mine)
            .fetch_optional(&db)
            .await?;
    assert_eq!(none, None, "a rejected claim must not resolve the failure");

    // Provenance the server established itself is not accountability, so it is recorded even
    // where a typed note is not. This is the one thing that must differ from `note` in CE.
    let resp = member(client().post(format!("{base}/completed/resolve")))
        .json(&json!({ "job_ids": [mine], "superseded_by": mine_succeeded }))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "resolve with a verified supersession",
    );
    let system_note: Option<String> =
        sqlx::query_scalar("SELECT note FROM job_resolution WHERE job_id = $1")
            .bind(mine)
            .fetch_one(&db)
            .await?;
    assert_eq!(
        system_note.as_deref(),
        Some("Superseded by a successful re-run"),
        "a verified supersession must be recorded regardless of licence"
    );

    // Machine provenance only fills a blank: re-running a failure someone already explained
    // must not replace their words with the generic supersession wording.
    let resp = member(client().post(format!("{base}/completed/resolve")))
        .json(&json!({ "job_ids": [mine], "note": "known upstream outage" }))
        .send()
        .await?;
    assert_2xx(resp.status().as_u16(), &resp.text().await?, "typed note");
    let resp = member(client().post(format!("{base}/completed/resolve")))
        .json(&json!({ "job_ids": [mine], "superseded_by": mine_succeeded }))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "supersede an explained failure",
    );
    let after: Option<String> =
        sqlx::query_scalar("SELECT note FROM job_resolution WHERE job_id = $1")
            .bind(mine)
            .fetch_one(&db)
            .await?;
    #[cfg(feature = "enterprise")]
    assert_eq!(
        after.as_deref(),
        Some("known upstream outage"),
        "a person's explanation must survive a later supersession"
    );
    #[cfg(not(feature = "enterprise"))]
    assert_eq!(
        after.as_deref(),
        Some("Superseded by a successful re-run"),
        "no typed note is stored outside EE, so provenance fills the blank"
    );

    sqlx::query("DELETE FROM job_resolution WHERE job_id = $1")
        .bind(mine)
        .execute(&db)
        .await?;

    // Tag scope restricts reads outside RLS, so it has to bind the evidence too: this run is a
    // genuine later success of the same runnable and is rejected only by its tag. Without that
    // predicate the response would disclose whether a run the token cannot read succeeded.
    let out_of_scope_success = seed(&db, "test-user-2", "success", "some_script").await;
    sqlx::query("UPDATE v2_job SET tag = 'restricted' WHERE id = $1")
        .bind(out_of_scope_success)
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes)
         VALUES (encode(sha256('TAG_SCOPED_2'::bytea), 'hex'), 'TAG_SCOP', 'TAG_SCOPED_2',
                 'test2@windmill.dev', 'tag scoped', false, ARRAY['if_jobs:filter_tags:deno'])",
    )
    .execute(&db)
    .await?;
    let resp = client()
        .post(format!("{base}/completed/resolve"))
        .header("Authorization", "Bearer TAG_SCOPED_2")
        .json(&json!({ "job_ids": [mine], "superseded_by": out_of_scope_success }))
        .send()
        .await?;
    let body = resp.text().await?;
    let ids: Vec<Uuid> = serde_json::from_str(&body)?;
    assert!(
        ids.is_empty(),
        "an out-of-scope run must not serve as evidence, got {body}"
    );
    sqlx::query("DELETE FROM job_resolution WHERE job_id = $1")
        .bind(mine)
        .execute(&db)
        .await?;

    // A flow step is a failure too, but resolving one would render it orange inside a flow
    // whose own status is still red, so the endpoint must skip it.
    let step = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email,
                             kind, tag, runnable_path, visible_to_owner, flow_step_id)
         VALUES ($1, 'test-workspace', 'test-user-2', 'u/test-user-2', 'test2@windmill.dev',
                 'script', 'deno', 'u/test-user-2/some_script', true, 'a')",
    )
    .bind(step)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, result, status)
         VALUES ($1, 'test-workspace', 100, '42'::jsonb, 'failure'::job_status)",
    )
    .bind(step)
    .execute(&db)
    .await?;
    let resp = member(client().post(format!("{base}/completed/resolve")))
        .json(&json!({ "job_ids": [step] }))
        .send()
        .await?;
    let ids: Vec<Uuid> = serde_json::from_str(&resp.text().await?)?;
    assert!(ids.is_empty(), "a flow step must not be resolvable");

    // Re-resolving when attribution is unavailable must not erase attribution already
    // recorded. CE reaches the identical code path as an EE instance whose runtime licence
    // lapsed: `resolution_attribution` returns NULLs in both cases, and without the COALESCE
    // the conflict update would overwrite a stored resolver with NULL.
    #[cfg(not(feature = "enterprise"))]
    {
        sqlx::query(
            "INSERT INTO job_resolution (job_id, workspace_id, resolved_by, note)
             VALUES ($1, 'test-workspace', 'earlier-admin', 'recorded under a valid licence')
             ON CONFLICT (job_id) DO UPDATE SET resolved_by = EXCLUDED.resolved_by,
                                                note = EXCLUDED.note",
        )
        .bind(mine)
        .execute(&db)
        .await?;
        let resp = member(client().post(format!("{base}/completed/resolve")))
            .json(&json!({ "job_ids": [mine] }))
            .send()
            .await?;
        assert_2xx(
            resp.status().as_u16(),
            &resp.text().await?,
            "re-resolve without attribution",
        );
        let kept: (Option<String>, Option<String>) =
            sqlx::query_as("SELECT resolved_by, note FROM job_resolution WHERE job_id = $1")
                .bind(mine)
                .fetch_one(&db)
                .await?;
        assert_eq!(
            kept.0.as_deref(),
            Some("earlier-admin"),
            "attribution recorded earlier must survive a re-resolve that cannot supply it"
        );
        assert_eq!(kept.1.as_deref(), Some("recorded under a valid licence"));
        sqlx::query("DELETE FROM job_resolution WHERE job_id = $1")
            .bind(mine)
            .execute(&db)
            .await?;
    }

    // Operators are read-only on runs; the endpoint must refuse them outright.
    sqlx::query("UPDATE usr SET operator = true WHERE username = 'test-user-3'")
        .execute(&db)
        .await?;
    let resp = client()
        .post(format!("{base}/completed/resolve"))
        .header("Authorization", "Bearer SECRET_TOKEN_3")
        .json(&json!({ "job_ids": [mine] }))
        .send()
        .await?;
    // Error::NotAuthorized maps to 401 (403 is RequireAdmin/PermissionDenied).
    assert_eq!(
        resp.status().as_u16(),
        401,
        "operators must be refused: {}",
        resp.text().await?
    );

    // The note is copied onto every resolved row, so an unbounded one multiplies by the
    // batch size; the cap must reject before any row is written.
    let resp = member(client().post(format!("{base}/completed/resolve")))
        .json(&json!({ "job_ids": [mine], "note": "x".repeat(2001) }))
        .send()
        .await?;
    assert_eq!(resp.status().as_u16(), 400, "{}", resp.text().await?);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM job_resolution")
        .fetch_one(&db)
        .await?;
    assert_eq!(after, 0, "a rejected note must not write any row");

    // The limit is characters, not bytes, so a multi-byte note the client accepted must
    // not fail server-side: 1000 4-byte chars is well over 2000 bytes but under the cap.
    let resp = member(client().post(format!("{base}/completed/resolve")))
        .json(&json!({ "job_ids": [mine], "note": "😀".repeat(1000) }))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "resolve with a multi-byte note",
    );

    Ok(())
}

/// "Resolved only" is a completed-jobs concept, so the bulk-action id list must not union
/// the queue: re-running "all jobs matching filters" under it would hit live jobs.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_list_filtered_job_uuids_resolved_excludes_queue(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let queued = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email,
                             kind, tag, runnable_path, visible_to_owner)
         VALUES ($1, 'test-workspace', 'test-user', 'u/test-user', 'test@windmill.dev',
                 'script', 'deno', 'u/test-user/queued', true)",
    )
    .bind(queued)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
         VALUES ($1, 'test-workspace', now(), 'deno')",
    )
    .bind(queued)
    .execute(&db)
    .await?;

    let url = format!(
        "http://localhost:{port}/api/w/test-workspace/jobs/list_filtered_uuids?resolved=true"
    );
    let resp = authed(client().get(&url)).send().await?;
    let body = resp.text().await?;
    let ids: Vec<Uuid> = serde_json::from_str(&body)?;
    assert!(
        !ids.contains(&queued),
        "a queued job must not match resolved=true, got: {body}"
    );

    Ok(())
}
