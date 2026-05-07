//! Tests for `/jobs/run/batch_rerun_jobs` and `/jobs/list_selected_job_groups`.
//!
//! These cover both the long-existing baseline behavior (regular Script and Flow
//! jobs reran with default args, `use_latest_version=true`, and `input_transforms`)
//! and the SingleStepFlow projection paths added in the PR that introduced this
//! file. The endpoint had zero coverage before — every change here exists because
//! a code reviewer or reader caught it; the tests exist so the next reader doesn't
//! have to.

use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use windmill_common::{
    flows::Retry,
    jobs::{JobKind, JobPayload},
    runnable_settings::{ConcurrencySettings, DebouncingSettings},
    scripts::{ScriptHash, ScriptLang},
};

use windmill_test_utils::*;

const WORKSPACE: &str = "test-workspace";
const SCRIPT_PATH: &str = "u/test-user/rerun_script";
const SCRIPT_HASH: i64 = 1111111111;
const FLOW_PATH: &str = "u/test-user/rerun_flow";
const FLOW_VERSION: i64 = 2222222222;

/// Mark a queued job as completed with success — needed so it's eligible for
/// `list_selected_job_groups` / `batch_rerun_jobs` (both join `v2_job_completed`).
async fn complete_job(db: &Pool<Postgres>, job_id: Uuid) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO v2_job_completed (workspace_id, id, result, status, duration_ms, started_at, completed_at)
         VALUES ($1, $2, '{}'::jsonb, 'success', 0, now(), now())",
    )
    .bind(WORKSPACE)
    .bind(job_id)
    .execute(db)
    .await?;
    Ok(())
}

fn script_payload() -> JobPayload {
    JobPayload::ScriptHash {
        hash: ScriptHash(SCRIPT_HASH),
        path: SCRIPT_PATH.to_string(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        language: ScriptLang::Deno,
        priority: None,
        apply_preprocessor: false,
        concurrency_settings: ConcurrencySettings::default().into(),
        debouncing_settings: DebouncingSettings::default(),
        labels: None,
    }
}

fn flow_payload() -> JobPayload {
    JobPayload::Flow {
        path: FLOW_PATH.to_string(),
        dedicated_worker: None,
        apply_preprocessor: false,
        version: FLOW_VERSION,
        labels: None,
    }
}

fn ssf_script_payload(hash: Option<ScriptHash>, retry: Option<Retry>) -> JobPayload {
    JobPayload::SingleStepFlow {
        path: SCRIPT_PATH.to_string(),
        hash,
        flow_version: None,
        args: Default::default(),
        retry,
        error_handler_path: None,
        error_handler_args: None,
        skip_handler: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        priority: None,
        tag_override: None,
        trigger_path: None,
        apply_preprocessor: false,
        concurrency_settings: ConcurrencySettings::default(),
        debouncing_settings: DebouncingSettings::default(),
    }
}

fn ssf_flow_payload() -> JobPayload {
    JobPayload::SingleStepFlow {
        path: FLOW_PATH.to_string(),
        hash: None,
        flow_version: Some(FLOW_VERSION),
        args: Default::default(),
        retry: None,
        error_handler_path: None,
        error_handler_args: None,
        skip_handler: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        priority: None,
        tag_override: None,
        trigger_path: None,
        apply_preprocessor: false,
        concurrency_settings: ConcurrencySettings::default(),
        debouncing_settings: DebouncingSettings::default(),
    }
}

/// Push + complete a job so it's eligible for batch rerun.
async fn push_completed(
    db: &Pool<Postgres>,
    payload: JobPayload,
    args: Vec<(&str, serde_json::Value)>,
) -> anyhow::Result<Uuid> {
    let mut runner = RunJob::from(payload);
    for (k, v) in args {
        runner = runner.arg(k.to_string(), v);
    }
    let id = runner.push(db).await;
    complete_job(db, id).await?;
    Ok(id)
}

/// POST `/jobs/run/batch_rerun_jobs` and parse the SSE-style line-per-result
/// stream into a list of (uuid, error?) pairs.
async fn batch_rerun(
    client: &windmill_api_client::Client,
    body: serde_json::Value,
) -> anyhow::Result<Vec<Result<Uuid, String>>> {
    let response = client
        .client()
        .post(format!(
            "{}/w/{}/jobs/run/batch_rerun_jobs",
            client.baseurl(),
            WORKSPACE
        ))
        .json(&body)
        .send()
        .await?;
    assert!(
        response.status().is_success(),
        "batch_rerun_jobs returned {}",
        response.status()
    );
    let body = response.text().await?;
    Ok(body
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            if let Some(err) = l.strip_prefix("Error: ") {
                Err(err.to_string())
            } else {
                Ok(Uuid::parse_str(l.trim()).expect("rerun line should be a UUID"))
            }
        })
        .collect())
}

/// Read kind + args of a freshly-rerun job.
async fn rerun_job(db: &Pool<Postgres>, id: Uuid) -> anyhow::Result<(JobKind, serde_json::Value)> {
    let row = sqlx::query!(
        r#"SELECT kind AS "kind: JobKind", args::text AS "args!"
           FROM v2_job WHERE id = $1"#,
        id
    )
    .fetch_one(db)
    .await?;
    Ok((row.kind, serde_json::from_str(&row.args)?))
}

// ---------------------------------------------------------------------------
// Baseline regression — regular Script and Flow rerun.

/// Default-mode rerun on a regular Script: new Script job inherits original args.
#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn batch_rerun_script_default(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{}", server.addr.port()),
        "SECRET_TOKEN".to_string(),
    );

    let original = push_completed(&db, script_payload(), vec![("name", json!("orig"))]).await?;

    let results = batch_rerun(
        &client,
        json!({
            "job_ids": [original],
            "script_options_by_path": {},
            "flow_options_by_path": {},
        }),
    )
    .await?;
    assert_eq!(results.len(), 1, "expected one rerun");
    let new_id = results[0].as_ref().expect("rerun should succeed").clone();
    let (kind, args) = rerun_job(&db, new_id).await?;
    assert_eq!(kind, JobKind::Script);
    assert_eq!(args, json!({"name": "orig"}));
    Ok(())
}

/// `use_latest_version=true` reruns regular Script via path-based push.
#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn batch_rerun_script_latest_version(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{}", server.addr.port()),
        "SECRET_TOKEN".to_string(),
    );

    let original = push_completed(&db, script_payload(), vec![("name", json!("orig"))]).await?;

    let results = batch_rerun(
        &client,
        json!({
            "job_ids": [original],
            "script_options_by_path": {
                SCRIPT_PATH: { "use_latest_version": true }
            },
            "flow_options_by_path": {},
        }),
    )
    .await?;
    let new_id = results[0].as_ref().expect("rerun should succeed").clone();
    let (kind, args) = rerun_job(&db, new_id).await?;
    assert_eq!(kind, JobKind::Script);
    assert_eq!(args, json!({"name": "orig"}));
    Ok(())
}

/// `input_transforms` static value overrides original args on Script rerun.
#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn batch_rerun_script_static_transform(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{}", server.addr.port()),
        "SECRET_TOKEN".to_string(),
    );

    let original = push_completed(&db, script_payload(), vec![("name", json!("orig"))]).await?;

    let results = batch_rerun(
        &client,
        json!({
            "job_ids": [original],
            "script_options_by_path": {
                SCRIPT_PATH: {
                    "input_transforms": { "name": { "type": "static", "value": "\"X\"" } }
                }
            },
            "flow_options_by_path": {},
        }),
    )
    .await?;
    let new_id = results[0].as_ref().expect("rerun should succeed").clone();
    let (_, args) = rerun_job(&db, new_id).await?;
    assert_eq!(
        args,
        json!({"name": "\"X\""}),
        "static transform should override original args"
    );
    Ok(())
}

/// Regular Flow rerun: new Flow job inherits original args.
#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn batch_rerun_flow_default(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{}", server.addr.port()),
        "SECRET_TOKEN".to_string(),
    );

    let original = push_completed(&db, flow_payload(), vec![("name", json!("orig-flow"))]).await?;

    let results = batch_rerun(
        &client,
        json!({
            "job_ids": [original],
            "script_options_by_path": {},
            "flow_options_by_path": {},
        }),
    )
    .await?;
    let new_id = results[0].as_ref().expect("rerun should succeed").clone();
    let (kind, args) = rerun_job(&db, new_id).await?;
    assert_eq!(kind, JobKind::Flow);
    assert_eq!(args, json!({"name": "orig-flow"}));
    Ok(())
}

/// `input_transforms` override original args on Flow rerun (path-based,
/// frontend forces use_latest_version=true for flow).
#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn batch_rerun_flow_static_transform(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{}", server.addr.port()),
        "SECRET_TOKEN".to_string(),
    );

    let original = push_completed(&db, flow_payload(), vec![("name", json!("orig"))]).await?;

    let results = batch_rerun(
        &client,
        json!({
            "job_ids": [original],
            "script_options_by_path": {},
            "flow_options_by_path": {
                FLOW_PATH: {
                    "use_latest_version": true,
                    "input_transforms": { "name": { "type": "static", "value": "\"FX\"" } }
                }
            },
        }),
    )
    .await?;
    let new_id = results[0].as_ref().expect("rerun should succeed").clone();
    let (_, args) = rerun_job(&db, new_id).await?;
    assert_eq!(args, json!({"name": "\"FX\""}));
    Ok(())
}

// ---------------------------------------------------------------------------
// SingleStepFlow projection — the bug class this PR fixes.

/// Default rerun of a script-wrapped SingleStepFlow lands as a plain Script.
#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn batch_rerun_ssf_script_default(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{}", server.addr.port()),
        "SECRET_TOKEN".to_string(),
    );

    let original = push_completed(
        &db,
        ssf_script_payload(Some(ScriptHash(SCRIPT_HASH)), None),
        vec![("name", json!("orig-ssf"))],
    )
    .await?;

    let results = batch_rerun(
        &client,
        json!({
            "job_ids": [original],
            "script_options_by_path": {},
            "flow_options_by_path": {},
        }),
    )
    .await?;
    let new_id = results[0].as_ref().expect("SSF should be reruable").clone();
    let (kind, args) = rerun_job(&db, new_id).await?;
    // Wrapper unwraps to plain Script (retry policy belongs to the trigger, not to "rerun").
    assert_eq!(kind, JobKind::Script);
    assert_eq!(args, json!({"name": "orig-ssf"}));
    Ok(())
}

/// `use_latest_version=true` + `input_transforms` on SSF — exercises the
/// `latest_schema` projection in `batch_rerun_handle_job`. Regression for the
/// fix in this PR's third commit; without it the transform silently no-ops.
#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn batch_rerun_ssf_script_latest_with_transform(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{}", server.addr.port()),
        "SECRET_TOKEN".to_string(),
    );

    let original = push_completed(
        &db,
        ssf_script_payload(Some(ScriptHash(SCRIPT_HASH)), None),
        vec![("name", json!("orig"))],
    )
    .await?;

    let results = batch_rerun(
        &client,
        json!({
            "job_ids": [original],
            "script_options_by_path": {
                SCRIPT_PATH: {
                    "use_latest_version": true,
                    "input_transforms": { "name": { "type": "static", "value": "\"S\"" } }
                }
            },
            "flow_options_by_path": {},
        }),
    )
    .await?;
    let new_id = results[0]
        .as_ref()
        .expect("SSF rerun should succeed")
        .clone();
    let (_, args) = rerun_job(&db, new_id).await?;
    assert_eq!(args, json!({"name": "\"S\""}));
    Ok(())
}

/// Flow-wrapped SingleStepFlow projects to Flow and reruns by path.
#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn batch_rerun_ssf_flow(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{}", server.addr.port()),
        "SECRET_TOKEN".to_string(),
    );

    let original = push_completed(
        &db,
        ssf_flow_payload(),
        vec![("name", json!("orig-ssf-flow"))],
    )
    .await?;

    let results = batch_rerun(
        &client,
        json!({
            "job_ids": [original],
            "script_options_by_path": {},
            "flow_options_by_path": {
                FLOW_PATH: {
                    "use_latest_version": true,
                    "input_transforms": { "name": { "type": "static", "value": "\"FF\"" } }
                }
            },
        }),
    )
    .await?;
    let new_id = results[0]
        .as_ref()
        .expect("SSF flow should be reruable")
        .clone();
    let (kind, args) = rerun_job(&db, new_id).await?;
    assert_eq!(kind, JobKind::Flow);
    assert_eq!(args, json!({"name": "\"FF\""}));
    Ok(())
}

/// Mixed batch (script + flow + SSF-script + SSF-flow) — exercises the
/// dispatch arms across all four kinds in a single request.
#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn batch_rerun_mixed_kinds(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{}", server.addr.port()),
        "SECRET_TOKEN".to_string(),
    );

    let s = push_completed(&db, script_payload(), vec![("name", json!("S"))]).await?;
    let f = push_completed(&db, flow_payload(), vec![("name", json!("F"))]).await?;
    let ss = push_completed(
        &db,
        ssf_script_payload(Some(ScriptHash(SCRIPT_HASH)), None),
        vec![("name", json!("SS"))],
    )
    .await?;
    let sf = push_completed(&db, ssf_flow_payload(), vec![("name", json!("SF"))]).await?;

    let results = batch_rerun(
        &client,
        json!({
            "job_ids": [s, f, ss, sf],
            "script_options_by_path": {},
            "flow_options_by_path": {},
        }),
    )
    .await?;
    assert_eq!(
        results.len(),
        4,
        "all 4 kinds should rerun, got: {results:?}"
    );
    for r in &results {
        assert!(r.is_ok(), "expected all reruns to succeed, got {r:?}");
    }

    // Two new Scripts (regular + SSF-script projected) and two new Flows
    // (regular + SSF-flow projected).
    let counts = sqlx::query!(
        r#"SELECT kind AS "kind: JobKind", count(*) AS "count!"
           FROM v2_job
           WHERE workspace_id = $1
             AND id <> ALL($2)
             AND parent_job IS NULL
           GROUP BY kind
           ORDER BY kind::text"#,
        WORKSPACE,
        &[s, f, ss, sf][..]
    )
    .fetch_all(&db)
    .await?;
    let mut script = 0;
    let mut flow = 0;
    for c in counts {
        match c.kind {
            JobKind::Script => script = c.count,
            JobKind::Flow => flow = c.count,
            _ => {}
        }
    }
    assert_eq!(
        script, 2,
        "expected 2 new Script jobs (script + SSF-script)"
    );
    assert_eq!(flow, 2, "expected 2 new Flow jobs (flow + SSF-flow)");
    Ok(())
}

// ---------------------------------------------------------------------------
// list_selected_job_groups — the front door for the BatchReRun pane. Crashes
// here would stop the user before they could even click Re-run.

#[derive(serde::Deserialize)]
struct GroupResp {
    kind: String,
    script_path: String,
    schemas: Vec<SchemaEntry>,
    latest_schema: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct SchemaEntry {
    script_hash: Option<String>,
    schema: Option<serde_json::Value>,
    job_ids: Vec<Uuid>,
}

async fn list_groups(
    client: &windmill_api_client::Client,
    job_ids: &[Uuid],
) -> anyhow::Result<Vec<GroupResp>> {
    let response = client
        .client()
        .post(format!(
            "{}/w/{}/jobs/list_selected_job_groups",
            client.baseurl(),
            WORKSPACE
        ))
        .json(&job_ids)
        .send()
        .await?;
    assert!(response.status().is_success());
    Ok(response.json().await?)
}

/// SSF script-wrapped: pinned hash projected, schema non-null. Without this,
/// the BatchReRun pane crashes on `mergeSchemasForBatchReruns`.
#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn list_groups_ssf_script_has_schema(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{}", server.addr.port()),
        "SECRET_TOKEN".to_string(),
    );

    let id = push_completed(
        &db,
        ssf_script_payload(Some(ScriptHash(SCRIPT_HASH)), None),
        vec![("name", json!("x"))],
    )
    .await?;

    let groups = list_groups(&client, &[id]).await?;
    assert_eq!(groups.len(), 1);
    let g = &groups[0];
    assert_eq!(g.kind, "script", "SSF wrapping a script projects to script");
    assert_eq!(g.script_path, SCRIPT_PATH);
    assert!(g.latest_schema.is_some(), "latest_schema must resolve");
    assert_eq!(g.schemas.len(), 1, "expected one schema entry");
    let s = &g.schemas[0];
    assert!(s.schema.is_some(), "per-version schema must be non-null");
    let expected_hash = format!("{:0>16x}", SCRIPT_HASH as u64);
    assert_eq!(s.script_hash.as_deref(), Some(expected_hash.as_str()));
    assert_eq!(s.job_ids, vec![id]);
    Ok(())
}

/// SSF flow-wrapped: no hash to pin, but path-based fallback fills in schema.
#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn list_groups_ssf_flow_has_schema_via_path(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{}", server.addr.port()),
        "SECRET_TOKEN".to_string(),
    );

    let id = push_completed(&db, ssf_flow_payload(), vec![("name", json!("x"))]).await?;

    let groups = list_groups(&client, &[id]).await?;
    let g = groups.first().expect("expected one group");
    assert_eq!(g.kind, "flow", "SSF wrapping a flow projects to flow");
    assert!(g.latest_schema.is_some());
    assert_eq!(g.schemas.len(), 1);
    let s = &g.schemas[0];
    assert!(
        s.schema.is_some(),
        "flow-wrapped SSF schema falls back to latest by path"
    );
    Ok(())
}
