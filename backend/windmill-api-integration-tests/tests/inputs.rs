use serde_json::json;
use sqlx::{Pool, Postgres};
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

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_inputs_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/inputs");

    // GET /history with fake runnable → 200 empty array
    let resp = authed(client().get(format!(
        "{base}/history?runnable_id=u/test-user/test&runnable_type=ScriptPath"
    )))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /inputs/history");

    // GET /list with fake runnable → 200 empty array
    let resp = authed(client().get(format!(
        "{base}/list?runnable_id=u/test-user/test&runnable_type=ScriptPath"
    )))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /inputs/list");

    // POST /create → 200, returns UUID
    let resp = authed(client().post(format!(
        "{base}/create?runnable_id=u/test-user/test&runnable_type=ScriptPath"
    )))
    .json(&json!({"name": "test_input", "args": {}}))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /inputs/create");
    let input_id: String = serde_json::from_str(&body)?;

    // GET /{id}/args → 200
    let resp = authed(client().get(format!("{base}/{input_id}/args")))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /inputs/{id}/args");

    // POST /delete/{id} → 200
    let resp = authed(client().post(format!("{base}/delete/{input_id}")))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /inputs/delete/{id}");

    Ok(())
}

// A scheduled runnable with a dynamic-skip handler runs as a `singlestepflow`, not
// `flow`/`script` (see windmill-queue schedule.rs).
// Both a flow's and a script's history must surface their own singlestepflow runs, but a
// script and flow may share a path, so each side must match only the wrapped kind that
// belongs to it — the other must not leak in.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_input_history_singlestepflow_flow_vs_script(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Both rows share runnable_path 'f/test/scheduled'.
    let flow_job = insert_singlestepflow(&db, "flow").await?;
    let script_job = insert_singlestepflow(&db, "script").await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/inputs");

    let history_ids = |runnable_type: &'static str| {
        let base = base.clone();
        async move {
            let resp = authed(client().get(format!(
                "{base}/history?runnable_id=f/test/scheduled&runnable_type={runnable_type}"
            )))
            .send()
            .await?;
            let status = resp.status().as_u16();
            let body = resp.text().await?;
            assert_2xx(status, &body, "GET /inputs/history");
            let inputs: Vec<serde_json::Value> = serde_json::from_str(&body)?;
            anyhow::Ok(
                inputs
                    .iter()
                    .filter_map(|i| i.get("id").and_then(|v| v.as_str()).map(String::from))
                    .collect::<std::collections::HashSet<_>>(),
            )
        }
    };

    let flow_hist = history_ids("FlowPath").await?;
    assert!(
        flow_hist.contains(&flow_job.to_string()),
        "flow-wrapped singlestepflow missing from flow history: {flow_hist:?}",
    );
    assert!(
        !flow_hist.contains(&script_job.to_string()),
        "script-wrapped singlestepflow leaked into flow history: {flow_hist:?}",
    );

    let script_hist = history_ids("ScriptPath").await?;
    assert!(
        script_hist.contains(&script_job.to_string()),
        "script-wrapped singlestepflow missing from script history: {script_hist:?}",
    );
    assert!(
        !script_hist.contains(&flow_job.to_string()),
        "flow-wrapped singlestepflow leaked into script history: {script_hist:?}",
    );

    Ok(())
}

// Insert a completed root singlestepflow at path f/test/scheduled wrapping `wrapped_type`
// ('flow' or 'script') as its single module — mirrors the schedule.rs wrapper shape.
async fn insert_singlestepflow(
    db: &Pool<Postgres>,
    wrapped_type: &str,
) -> anyhow::Result<uuid::Uuid> {
    let id = uuid::Uuid::new_v4();
    let raw_flow = json!({ "modules": [{ "id": "a", "value": { "type": wrapped_type } }] });
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, tag, created_by, permissioned_as, \
         permissioned_as_email, kind, runnable_path, raw_flow, same_worker, visible_to_owner) \
         VALUES ($1, 'test-workspace', 'flow', 'test-user', 'u/test-user', \
         'test@windmill.dev', 'singlestepflow', 'f/test/scheduled', $2, false, true)",
    )
    .bind(id)
    .bind(sqlx::types::Json(&raw_flow))
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, deleted, status) \
         VALUES ($1, 'test-workspace', 1, false, 'success')",
    )
    .bind(id)
    .execute(db)
    .await?;
    Ok(id)
}

// Each way a run of f/test/step can start, as the history lists it with `started_by`: top-level
// runs by origin, and runs inside other runs grouped under the top-level run containing them.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_input_history_started_by(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    const STEP: &str = "f/test/step";

    let manual = insert_run(&db, Run { path: STEP, at: 1, ..Run::default() }).await?;
    let scheduled = insert_run(
        &db,
        Run { path: STEP, trigger_kind: Some("schedule"), failed: true, at: 2, ..Run::default() },
    )
    .await?;
    let retry = insert_run(
        &db,
        Run {
            path: STEP,
            parent_job: Some(scheduled),
            trigger_kind: Some("schedule"),
            at: 3,
            ..Run::default()
        },
    )
    .await?;
    sqlx::query("INSERT INTO native_retry_attempt (job_id, attempt) VALUES ($1, 1)")
        .bind(retry)
        .execute(&db)
        .await?;
    // A workflow-as-code task carries its script's path under a run of that script: hidden.
    let wac_task = insert_run(
        &db,
        Run { path: STEP, parent_job: Some(manual), at: 4, ..Run::default() },
    )
    .await?;

    let flow = insert_run(
        &db,
        Run { kind: "flow", path: "f/test/flow", at: 5, ..Run::default() },
    )
    .await?;
    let step = insert_run(
        &db,
        Run {
            path: STEP,
            parent_job: Some(flow),
            root_job: Some(flow),
            innermost_root: Some(flow),
            flow_step_id: Some("a"),
            at: 6,
            ..Run::default()
        },
    )
    .await?;
    let agent = insert_run(
        &db,
        Run {
            kind: "aiagent",
            path: "f/test/flow/b",
            parent_job: Some(flow),
            root_job: Some(flow),
            innermost_root: Some(flow),
            flow_step_id: Some("b"),
            at: 7,
            ..Run::default()
        },
    )
    .await?;
    let tool = insert_run(
        &db,
        Run {
            path: STEP,
            parent_job: Some(agent),
            root_job: Some(agent),
            innermost_root: Some(flow),
            at: 8,
            ..Run::default()
        },
    )
    .await?;

    let caller = insert_run(&db, Run { path: "f/test/caller", at: 9, ..Run::default() }).await?;
    let call = insert_run(
        &db,
        Run {
            path: STEP,
            parent_job: Some(caller),
            innermost_root: Some(caller),
            trigger_kind: Some("webhook"),
            at: 10,
            ..Run::default()
        },
    )
    .await?;
    let failing = insert_run(
        &db,
        Run { path: "f/test/failing", failed: true, at: 11, ..Run::default() },
    )
    .await?;
    let handler = insert_run(
        &db,
        Run {
            path: STEP,
            parent_job: Some(failing),
            innermost_root: Some(failing),
            created_by: "error_handler",
            at: 12,
            ..Run::default()
        },
    )
    .await?;

    // A run still in progress, older than the two runs a one-entry page reads by creation time.
    let long_running = insert_run(
        &db,
        Run { path: "f/test/long", running: true, at: 13, ..Run::default() },
    )
    .await?;
    for at in [14, 15] {
        insert_run(&db, Run { path: "f/test/long", at, ..Run::default() }).await?;
    }

    // test-user-2 can read the runs of its own script, not the flow run they are steps of nor
    // that flow's loop container.
    let private_flow = insert_run(
        &db,
        Run { kind: "flow", path: "u/test-user/private_flow", at: 15, ..Run::default() },
    )
    .await?;
    let own_step = insert_run(
        &db,
        Run {
            path: "u/test-user-2/helper",
            permissioned_as: "u/test-user-2",
            parent_job: Some(private_flow),
            root_job: Some(private_flow),
            innermost_root: Some(private_flow),
            flow_step_id: Some("a"),
            at: 16,
            ..Run::default()
        },
    )
    .await?;
    let container = insert_run(
        &db,
        Run {
            kind: "flownode",
            path: "u/test-user/private_flow/loop-0",
            parent_job: Some(private_flow),
            innermost_root: Some(private_flow),
            at: 17,
            ..Run::default()
        },
    )
    .await?;
    let mut in_loop = vec![];
    for at in [18, 19] {
        let run = Run {
            path: "u/test-user-2/helper",
            permissioned_as: "u/test-user-2",
            parent_job: Some(container),
            innermost_root: Some(private_flow),
            flow_step_id: Some("b"),
            at,
            ..Run::default()
        };
        in_loop.push(insert_run(&db, run).await?);
    }

    // A flow sharing the script's path: its step is a flow step, not a run the parent lists.
    let same_path_flow = insert_run(
        &db,
        Run { kind: "flow", path: STEP, at: 20, ..Run::default() },
    )
    .await?;
    let same_path_step = insert_run(
        &db,
        Run {
            path: STEP,
            parent_job: Some(same_path_flow),
            innermost_root: Some(same_path_flow),
            flow_step_id: Some("a"),
            at: 21,
            ..Run::default()
        },
    )
    .await?;

    // f/test/busy is called 5,000 times by one run. Before that: a call still in progress inside
    // another flow, and a failed top-level run with its retry; the list's window passes over
    // both. After: two calls from a long flow, which started a while after it was created, as a
    // scheduled run does. Opening each group must find the runs the list counts.
    let running_flow = insert_run(
        &db,
        Run { kind: "flow", path: "f/test/running_flow", running: true, at: 22, ..Run::default() },
    )
    .await?;
    let running_call = insert_run(
        &db,
        Run {
            path: "f/test/busy",
            parent_job: Some(running_flow),
            innermost_root: Some(running_flow),
            flow_step_id: Some("a"),
            running: true,
            at: 22,
            ..Run::default()
        },
    )
    .await?;
    let failed_call = insert_run(
        &db,
        Run { path: "f/test/busy", failed: true, at: 22, ..Run::default() },
    )
    .await?;
    let retried_call = insert_run(
        &db,
        Run { path: "f/test/busy", parent_job: Some(failed_call), at: 22, ..Run::default() },
    )
    .await?;
    sqlx::query("INSERT INTO native_retry_attempt (job_id, attempt) VALUES ($1, 1)")
        .bind(retried_call)
        .execute(&db)
        .await?;
    let long_flow = insert_run(
        &db,
        Run { kind: "flow", path: "f/test/long_flow", at: 22, ..Run::default() },
    )
    .await?;
    sqlx::query(
        "UPDATE v2_job_completed SET started_at = started_at + interval '8 seconds' WHERE id = $1",
    )
    .bind(long_flow)
    .execute(&db)
    .await?;
    let busy_flow = insert_run(
        &db,
        Run { kind: "flow", path: "f/test/busy_flow", at: 23, ..Run::default() },
    )
    .await?;
    sqlx::query(
        "WITH jobs AS (\
            INSERT INTO v2_job (id, workspace_id, tag, created_by, permissioned_as, \
             permissioned_as_email, kind, runnable_path, parent_job, flow_innermost_root_job, \
             flow_step_id, created_at, args, same_worker, visible_to_owner) \
            SELECT gen_random_uuid(), 'test-workspace', 'deno', 'test-user', 'u/test-user', \
             'test@windmill.dev', 'script', 'f/test/busy', $1, $1, 'a', \
             now() - interval '1 hour' + make_interval(secs => 23 + i / 1000.0), '{}'::jsonb, \
             false, true \
            FROM generate_series(1, 5000) i \
            RETURNING id, workspace_id, created_at) \
         INSERT INTO v2_job_completed (id, workspace_id, duration_ms, deleted, status, \
          started_at, completed_at) \
         SELECT id, workspace_id, 1, false, 'success', created_at, created_at FROM jobs",
    )
    .bind(busy_flow)
    .execute(&db)
    .await?;
    // Without statistics on these rows, the planner picks nested loops that take seconds.
    sqlx::query("ANALYZE v2_job, v2_job_completed")
        .execute(&db)
        .await?;
    // A workflow-as-code run in a later segment: its queue row's start is the segment's, and its
    // first start is in extras. Opening its group must still find an earlier segment's task.
    let workflow = insert_run(
        &db,
        Run { path: "f/test/workflow", running: true, at: 24, ..Run::default() },
    )
    .await?;
    let task = insert_run(
        &db,
        Run { path: "f/test/task", parent_job: Some(workflow), at: 25, ..Run::default() },
    )
    .await?;
    sqlx::query(
        "UPDATE v2_job_queue SET started_at = started_at + interval '5 seconds', \
         extras = jsonb_build_object('wac_first_started_at', started_at) WHERE id = $1",
    )
    .bind(workflow)
    .execute(&db)
    .await?;
    let mut late_calls = vec![];
    for at in [30, 31] {
        let run = Run {
            path: "f/test/busy",
            parent_job: Some(long_flow),
            innermost_root: Some(long_flow),
            flow_step_id: Some("b"),
            at,
            ..Run::default()
        };
        late_calls.push(insert_run(&db, run).await?);
    }

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let fetch = |token: &'static str, query: String| {
        let url = format!(
            "http://localhost:{port}/api/w/test-workspace/inputs/history\
             ?runnable_type=ScriptPath{query}"
        );
        async move {
            let resp = client()
                .get(url)
                .header("Authorization", format!("Bearer {token}"))
                .send()
                .await?;
            let status = resp.status().as_u16();
            let body = resp.text().await?;
            assert_2xx(status, &body, "GET /inputs/history");
            anyhow::Ok(serde_json::from_str::<Vec<serde_json::Value>>(&body)?)
        }
    };
    let history = |query: String| fetch("SECRET_TOKEN", format!("&runnable_id={STEP}{query}"));
    let ids = |entries: &[serde_json::Value]| {
        entries
            .iter()
            .map(|e| e["id"].as_str().unwrap_or_default().to_string())
            .collect::<Vec<_>>()
    };
    let top_level = "manual,api,schedule,app,trigger,preview";
    let everything = format!("{top_level},flow_step,agent_tool,script_call,handler");

    let default = history(format!("&started_by={top_level}")).await?;
    assert_eq!(ids(&default), [scheduled.to_string(), manual.to_string()]);
    assert_eq!(default[0]["group"]["retries"], 1);
    assert!(
        default[1].get("group").is_none(),
        "the hidden task counts: {default:?}"
    );

    let all = history(format!("&started_by={everything}")).await?;
    assert_eq!(
        ids(&all),
        [same_path_step, handler, call, flow, scheduled, manual].map(|id| id.to_string())
    );
    assert!(!ids(&all).contains(&wac_task.to_string()));
    assert_eq!(all[0]["started_by"], "flow_step");
    assert_eq!(all[0]["top_run"]["id"], same_path_flow.to_string());
    assert_eq!(all[1]["started_by"], "handler");
    assert_eq!(all[1]["top_run"]["id"], failing.to_string());
    assert_eq!(all[2]["started_by"], "script_call");
    assert_eq!(all[2]["parent_path"], "f/test/caller");
    assert_eq!(all[3]["group"]["runs"], 2);
    assert_eq!(all[3]["top_run"]["path"], "f/test/flow");

    let tools = history("&started_by=agent_tool".to_string()).await?;
    assert_eq!(ids(&tools), [tool.to_string()]);
    assert_eq!(tools[0]["top_run"]["id"], flow.to_string());

    let in_flow = history(format!("&started_by={everything}&top_run={flow}")).await?;
    assert_eq!(ids(&in_flow), [step.to_string(), tool.to_string()]);
    assert_eq!(in_flow[0]["flow_step_id"], "a");
    assert_eq!(in_flow[1]["started_by"], "agent_tool");

    let retries = history(format!("&started_by={everything}&top_run={scheduled}")).await?;
    assert_eq!(ids(&retries), [retry.to_string()]);
    assert_eq!(retries[0]["retry"], 1);
    assert_eq!(retries[0]["started_by"], "schedule");

    let first = fetch(
        "SECRET_TOKEN",
        format!("&runnable_id=f/test/long&started_by={top_level}&per_page=1"),
    )
    .await?;
    assert_eq!(ids(&first), [long_running.to_string()]);
    assert_eq!(first[0]["running"], true);
    assert_eq!(first[0]["args"], serde_json::json!({}));

    let helper = "&runnable_id=u/test-user-2/helper&started_by=flow_step";
    let steps = fetch("SECRET_TOKEN_2", helper.to_string()).await?;
    assert_eq!(ids(&steps), [private_flow.to_string()]);
    assert!(steps[0]["top_run"].get("path").is_none());
    assert_eq!(steps[0]["group"]["runs"], 3);
    let opened = fetch("SECRET_TOKEN_2", format!("{helper}&top_run={private_flow}")).await?;
    assert_eq!(
        ids(&opened),
        [own_step, in_loop[0], in_loop[1]].map(|id| id.to_string())
    );

    let busy = "&runnable_id=f/test/busy&started_by=flow_step";
    let listed = fetch("SECRET_TOKEN", busy.to_string()).await?;
    assert_eq!(listed[0]["top_run"]["id"], running_flow.to_string());
    assert_eq!(listed[0]["group"]["running"], 1);
    assert_eq!(listed[1]["top_run"]["id"], long_flow.to_string());
    assert_eq!(listed[1]["group"]["runs"], 2);
    assert_eq!(listed[1]["group"]["complete"], true);
    let opened = fetch("SECRET_TOKEN", format!("{busy}&top_run={running_flow}")).await?;
    assert_eq!(ids(&opened), [running_call.to_string()]);
    let manual = "&runnable_id=f/test/busy&started_by=manual";
    let retried = fetch("SECRET_TOKEN", manual.to_string()).await?;
    assert_eq!(ids(&retried), [failed_call.to_string()]);
    assert_eq!(retried[0]["group"]["retries"], 1);
    let opened = fetch("SECRET_TOKEN", format!("{manual}&top_run={failed_call}")).await?;
    assert_eq!(ids(&opened), [retried_call.to_string()]);
    let calls = format!("&runnable_id=f/test/task&started_by=script_call&top_run={workflow}");
    let opened = fetch("SECRET_TOKEN", calls).await?;
    assert_eq!(ids(&opened), [task.to_string()]);
    let opened = fetch("SECRET_TOKEN", format!("{busy}&top_run={long_flow}")).await?;
    assert_eq!(
        ids(&opened),
        late_calls
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
    );

    Ok(())
}

struct Run {
    kind: &'static str,
    path: &'static str,
    parent_job: Option<uuid::Uuid>,
    root_job: Option<uuid::Uuid>,
    innermost_root: Option<uuid::Uuid>,
    flow_step_id: Option<&'static str>,
    trigger_kind: Option<&'static str>,
    created_by: &'static str,
    permissioned_as: &'static str,
    failed: bool,
    /// Started and not finished: a queue row instead of a completed one.
    running: bool,
    /// Seconds after a fixed start, so runs are ordered by insertion.
    at: i32,
}

impl Default for Run {
    fn default() -> Self {
        Run {
            kind: "script",
            path: "",
            parent_job: None,
            root_job: None,
            innermost_root: None,
            flow_step_id: None,
            trigger_kind: None,
            created_by: "test-user",
            permissioned_as: "u/test-user",
            failed: false,
            running: false,
            at: 0,
        }
    }
}

async fn insert_run(db: &Pool<Postgres>, run: Run) -> anyhow::Result<uuid::Uuid> {
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, tag, created_by, permissioned_as, \
         permissioned_as_email, kind, runnable_path, parent_job, root_job, \
         flow_innermost_root_job, flow_step_id, trigger_kind, created_at, args, same_worker, \
         visible_to_owner) \
         VALUES ($1, 'test-workspace', 'deno', $2, $11, 'test@windmill.dev', \
         $3::job_kind, $4, $5, $6, $7, $8, $9::job_trigger_kind, \
         now() - interval '1 hour' + make_interval(secs => $10), '{}'::jsonb, false, true)",
    )
    .bind(id)
    .bind(run.created_by)
    .bind(run.kind)
    .bind(run.path)
    .bind(run.parent_job)
    .bind(run.root_job)
    .bind(run.innermost_root)
    .bind(run.flow_step_id)
    .bind(run.trigger_kind)
    .bind(run.at as f64)
    .bind(run.permissioned_as)
    .execute(db)
    .await?;
    if run.running {
        sqlx::query(
            "INSERT INTO v2_job_queue (id, workspace_id, created_at, started_at, scheduled_for, \
             running, tag) \
             SELECT id, workspace_id, created_at, created_at, created_at, true, 'deno' \
             FROM v2_job WHERE id = $1",
        )
        .bind(id)
        .execute(db)
        .await?;
        return Ok(id);
    }
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, deleted, status, \
         started_at, completed_at) \
         SELECT id, workspace_id, 1, false, $2::job_status, created_at, created_at \
         FROM v2_job WHERE id = $1",
    )
    .bind(id)
    .bind(if run.failed { "failure" } else { "success" })
    .execute(db)
    .await?;
    Ok(id)
}
