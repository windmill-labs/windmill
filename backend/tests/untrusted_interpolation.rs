//! Handler, trigger, public-app and agent tool jobs run with more rights than whoever wrote some of their
//! arguments, so nothing in those arguments may be resolved by the worker with the job's identity.

use axum::{routing::post, Json as AxumJson, Router};
use futures::StreamExt;
use serde_json::{json, value::RawValue, Value};
use sqlx::{types::Json, Pool, Postgres};
use std::time::Duration;
use uuid::Uuid;
use windmill_common::{flows::FlowValue, jobs::JobPayload};
use windmill_test_utils::*;

const SECRET: &str = "s3cr3t-value";
const ENV_SECRET: &str = "env-s3cr3t";
const READERS: &str =
    r#"{"g/error_handler": false, "g/success_handler": false, "g/publishers": false}"#;

async fn seed(db: &Pool<Postgres>) -> anyhow::Result<()> {
    // Every test of this file is serial: the workspace env cache is process-wide.
    sqlx::query(
        "INSERT INTO workspace_env (workspace_id, name, value) VALUES ('test-workspace', 'API_KEY', $1)",
    )
    .bind(ENV_SECRET)
    .execute(db)
    .await?;
    windmill_common::variables::CUSTOM_ENVS_CACHE.remove("test-workspace");
    // How an app run as its publisher reads a secret: through the publisher's groups.
    sqlx::query(
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms)
         VALUES ('test-workspace', 'publishers', '', '{}')",
    )
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO usr_to_group (workspace_id, group_, usr)
         VALUES ('test-workspace', 'publishers', 'test-user')",
    )
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
         VALUES ('test-workspace', 'u/test-user/secret', $1, false, '', $2::jsonb)",
    )
    .bind(format!("\"{SECRET}\""))
    .bind(READERS)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO resource_type (workspace_id, name, schema, description, created_by)
         VALUES ('test-workspace', 'c_cfg', '{}', '', 'test-user')",
    )
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by)
         VALUES ('test-workspace', 'u/test-user/cfg', '{\"set_by\": \"admin\"}', 'c_cfg', $1::jsonb,
             'test-user')",
    )
    .bind(READERS)
    .execute(db)
    .await?;
    // The lock must be non-null for the script to count as deployed.
    sqlx::query(
        r#"INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema,
             summary, description, lock)
           VALUES ('test-workspace', 1234, 'f/handlers/h', $1, 'bun', 'script', 'test-user', '{}',
             '', '', E'{}\n//bun.lock\n<empty>')"#,
    )
    .bind(
        "export async function main(error: any, cfg: any, success_result: any, pipeline: any,
             price: any, note: any, leak: any, log: any) {
           return { error, cfg, success_result, pipeline, price, note, leak, log };
         }",
    )
    .execute(db)
    .await?;
    Ok(())
}

async fn run(db: &Pool<Postgres>, port: u16, id: Uuid) -> Value {
    let done = listen_for_completed_jobs(db).await.find(&id);
    tokio::time::timeout(Duration::from_secs(60), in_test_worker(db, done, port))
        .await
        .expect("job did not complete");
    let job = completed_job(id, db).await;
    assert!(job.success, "{:?}", job.result);
    job.json_result().unwrap()
}

fn assert_omitted(v: &Value) {
    let s = v.as_str().unwrap_or_default();
    assert!(
        s.starts_with("The following variable has been omitted for security reasons"),
        "{v}"
    );
}

/// What a failed job sends its handler: references in its error, and the `$WM_TOKEN` the worker
/// replaces with the handler's token in an argument that holds one.
fn failed_result() -> Box<RawValue> {
    // Raw text: a number survives only if nothing on the way parses it.
    RawValue::from_string(
        r#"{"error": {"message": "$var:u/test-user/secret", "name": "$WM_TOKEN",
            "stack": "$jsonvar:u/test-user/secret"},
            "meta": {"code": 1.10, "id": 18446744073709551617}, "log": "a\u0000b"}"#
            .to_string(),
    )
    .unwrap()
}

fn extra_args() -> Json<Box<RawValue>> {
    Json(serde_json::value::to_raw_value(&json!({"cfg": "$res:u/test-user/cfg"})).unwrap())
}

/// Stored as jsonb stores it: numbers exact, the NUL jsonb refuses stripped.
async fn assert_handled(db: &Pool<Postgres>, port: u16, id: Uuid) -> Value {
    let stored: String = sqlx::query_scalar("SELECT args::text FROM v2_job WHERE id = $1")
        .bind(id)
        .fetch_one(db)
        .await
        .unwrap();
    assert!(
        stored.contains("1.10") && stored.contains("18446744073709551617"),
        "{stored}"
    );
    let out = run(db, port, id).await;
    assert_eq!(out["log"], json!("ab"));
    assert!(!out.to_string().contains(SECRET), "{out}");
    for k in ["message", "name", "stack"] {
        assert_omitted(&out["error"][k]);
    }
    // The handler's own settings, written by an admin.
    assert_eq!(out["cfg"], json!({"set_by": "admin"}));
    out
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn error_handler_resolves_nothing_the_failed_job_wrote(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    seed(&db).await?;
    let id = windmill_queue::push_error_handler(
        &db,
        Uuid::new_v4(),
        None,
        Some("u/test-user/failing".to_string()),
        false,
        "test-workspace",
        "script/f/handlers/h",
        Json(&failed_result()),
        None,
        None,
        Some(extra_args()),
        "test@windmill.dev",
        None,
        false,
        None,
    )
    .await?;
    assert_handled(&db, server.addr.port(), id).await;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn success_handler_resolves_nothing_the_job_returned(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    seed(&db).await?;
    let result = json!({
        "pipeline": [{"$match": {"currency": "$USD", "$or": [{"a": "$category"}]}}],
        "price": "$10.00",
        "note": "cost $var:u/test-user/secret",
        "leak": "\u{24}jsonvar:u/test-user/secret",
    });
    let id = windmill_queue::push_success_handler(
        &db,
        Uuid::new_v4(),
        None,
        Some("u/test-user/succeeding".to_string()),
        false,
        "test-workspace",
        "script/f/handlers/h",
        Json(&result),
        None,
        Some(extra_args()),
        "test@windmill.dev",
        None,
    )
    .await?;
    let out = run(&db, server.addr.port(), id).await;
    assert_omitted(&out["leak"]);
    // Holds the prefix that makes the worker interpolate the whole argument.
    assert!(out["note"]
        .as_str()
        .unwrap()
        .starts_with("cost The following variable"));
    for k in ["pipeline", "price"] {
        assert_eq!(out[k], result[k], "{k}");
    }
    assert_eq!(out["cfg"], json!({"set_by": "admin"}));
    Ok(())
}

#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn recovery_handler_resolves_nothing_the_schedule_jobs_wrote(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    seed(&db).await?;
    let job_id = Uuid::new_v4();
    let failed = failed_result();
    let success = json!({"leak": "$var:u/test-user/secret", "ok": {"$match": "$USD"}});
    windmill_queue::jobs_ee::handle_recovered_schedule(
        &db,
        job_id,
        "u/test-user/schedule",
        "u/test-user/flaky",
        false,
        "test-workspace",
        "script/f/handlers/h",
        Some(&failed),
        sqlx::types::chrono::Utc::now(),
        Json(&success),
        1,
        sqlx::types::chrono::Utc::now(),
        Some(extra_args()),
        "u/test-user",
    )
    .await?;
    let id: Uuid = sqlx::query_scalar("SELECT id FROM v2_job WHERE parent_job = $1")
        .bind(job_id)
        .fetch_one(&db)
        .await?;
    let out = assert_handled(&db, server.addr.port(), id).await;
    assert_omitted(&out["success_result"]["leak"]);
    assert_eq!(out["success_result"]["ok"], success["ok"]);
    Ok(())
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn public_app_resolves_nothing_the_viewer_sent(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    seed(&db).await?;
    let content = "export function main(a: any, b: any, c: any, d: any) { return { a, b, c, d } }";
    let ws = format!("http://localhost:{port}/api/w/test-workspace");
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{ws}/apps/create"))
        .bearer_auth("SECRET_TOKEN")
        .json(&json!({
            "path": "u/test-user/app",
            "summary": "",
            "value": {"runnables": {"c": {"type": "inline",
                "inlineScript": {"language": "bun", "content": content}}}},
            "policy": {"execution_mode": "anonymous", "triggerables_v2": {
                format!("c:rawscript/{}", windmill_common::utils::calculate_hash(content)):
                    {"static_inputs": {}, "one_of_inputs": {}}
            }},
        }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    // `\u0024` passes a text match on `$var:` until jsonb decodes it.
    let body = r#"{"component": "c",
        "raw_code": {"language": "bun", "content": "CONTENT"},
        "args": {"a": "\u0024var:u/test-user/secret", "b": {"x": ["$jsonvar:u/test-user/secret"]},
                 "c": {"pipeline": [{"$match": {"currency": "$USD"}}]},
                 "d": {"key": "$API_KEY", "note": "cost $var:x"}}}"#
        .replace("CONTENT", content);
    let resp = client
        .post(format!("{ws}/apps_u/execute_component/u/test-user/app"))
        .bearer_auth("SECRET_TOKEN_2")
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    let id: Uuid = resp.text().await?.parse()?;
    let out = run(&db, port, id).await;
    assert!(!out.to_string().contains(SECRET), "{out}");
    assert!(!out.to_string().contains(ENV_SECRET), "{out}");
    assert_omitted(&out["a"]);
    assert_omitted(&out["b"]["x"][0]);
    assert_eq!(
        out["c"],
        json!({"pipeline": [{"$match": {"currency": "$USD"}}]})
    );
    assert_eq!(out["d"]["key"], json!("$API_KEY"));
    Ok(())
}

#[cfg(feature = "http_trigger")]
#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn http_route_resolves_nothing_the_caller_sent(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    seed(&db).await?;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/http_triggers/create"
        ))
        .bearer_auth("SECRET_TOKEN")
        .json(&json!({
            "path": "u/test-user/route",
            "script_path": "f/handlers/h",
            "is_flow": false,
            "route_path": "untrusted",
            "request_type": "async",
            "authentication_method": "none",
            "http_method": "post",
            "is_static_website": false,
            "workspaced_route": false,
            "wrap_body": false,
            "raw_string": false,
        }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    // No credentials: the job still runs as the route's owner.
    let resp = client
        .post(format!("http://localhost:{port}/api/r/untrusted"))
        .header("content-type", "application/json")
        .body(
            r#"{"error": "$var:u/test-user/secret", "leak": ["$WM_TOKEN", "$var:u/test-user/secret"],
                "cfg": "$res:u/test-user/cfg", "price": "$10.00"}"#,
        )
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    let id: Uuid = resp.text().await?.trim_matches('"').parse()?;
    let out = run(&db, port, id).await;
    assert!(!out.to_string().contains(SECRET), "{out}");
    assert!(!out.to_string().contains("set_by"), "{out}");
    assert_omitted(&out["error"]);
    assert_omitted(&out["leak"][0]);
    assert_eq!(out["price"], json!("$10.00"));
    Ok(())
}

/// The flow resolves the approver's payload with its own token, and an approver controls what a
/// reference points to even when they can read it, so the payload may hold none.
#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn resume_takes_no_reference(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    seed(&db).await?;
    let flow: FlowValue = serde_json::from_value(json!({"modules": [
        {"id": "a", "suspend": {"required_events": 1},
         "value": {"type": "rawscript", "language": "bun", "input_transforms": {},
                   "content": "export function main() { return 1 }"}},
        {"id": "b",
         "value": {"type": "rawscript", "language": "bun",
                   "input_transforms": {"resume": {"type": "javascript", "expr": "resume"}},
                   "content": "export function main(resume: any) { return resume }"}},
    ]}))?;
    let flow = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .push(&db)
        .await;
    let mut completed = listen_for_completed_jobs(&db).await;
    let db_ = db.clone();
    let test = async move {
        let db = db_;
        completed.next().await.unwrap();
        while !sqlx::query_scalar::<_, bool>("SELECT suspend > 0 FROM v2_job_queue WHERE id = $1")
            .bind(flow)
            .fetch_one(&db)
            .await
            .unwrap()
        {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let resume = |payload: Value| {
            reqwest::Client::new()
                .post(format!(
                    "http://localhost:{port}/api/w/test-workspace/jobs_u/flow/resume_suspended/{flow}"
                ))
                .bearer_auth("SECRET_TOKEN")
                .json(&json!({"payload": payload}))
                .send()
        };
        // An admin, who can read the resource.
        let refused = resume(json!({"db": "$res:u/test-user/cfg"})).await.unwrap();
        assert_eq!(refused.status(), 400, "{}", refused.text().await.unwrap());
        let ok = resume(json!({"note": "$5.00"})).await.unwrap();
        assert!(ok.status().is_success(), "{}", ok.text().await.unwrap());
        completed.find(&flow).await.unwrap();
    };
    tokio::time::timeout(Duration::from_secs(60), in_test_worker(&db, test, port))
        .await
        .expect("flow did not complete");
    server.close().await?;
    let out = completed_job(flow, &db).await.json_result().unwrap();
    assert_eq!(out, json!({"note": "$5.00"}));
    Ok(())
}

/// Read as the jsonb args column holds it: whatever the worker's own test finds in the stored text,
/// the check finds too, and nothing more.
#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn reads_an_argument_as_jsonb_stores_it(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_common::{utils::strip_json_nul, variables::*};
    for arg in [
        r#""\u0024var:x""#,
        r#""$va\u0072:x""#,
        r#""$va\\u0072:x""#,
        r#""$va\u0000r:x""#,
        r#""\ud800 $var:x""#,
        r#"{"a": "\u0024res:x", "a": "ok"}"#,
        r#"{"a": "ok", "a": "\u0024var:x"}"#,
        r#"{"n": 1e400, "n": 0, "r": "\u0024var:x"}"#,
        r#"{"\u0024encrypted:k": 1}"#,
        r#""cost \u0024var:x""#,
        r#"{"$match": {"c": "$USD"}, "p": 1.10}"#,
        r#""$10.00""#,
    ] {
        let stored: Option<String> = match sqlx::query_scalar("SELECT $1::jsonb::text")
            .bind(strip_json_nul(arg).as_ref())
            .fetch_one(&db)
            .await
        {
            Ok(s) => Some(s),
            Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("22P02") => None,
            Err(e) => return Err(e.into()),
        };
        assert_eq!(
            untrusted_arg_interpolates(arg),
            stored.as_deref().map_or(true, arg_needs_interpolation),
            "{arg} stored as {stored:?}"
        );
    }
    Ok(())
}

/// One tool call with `arguments`, then a final answer once the tool's reply is in.
fn model(arguments: &'static str) -> Router {
    Router::new().route(
        "/v1/chat/completions",
        post(move |AxumJson(body): AxumJson<Value>| async move {
            let answered = body["messages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["role"] == "tool");
            let delta = if answered {
                json!({"role": "assistant", "content": "done"})
            } else {
                json!({"role": "assistant", "tool_calls": [{"index": 0, "id": "call_0",
                    "type": "function", "function": {"name": "echo", "arguments": arguments}}]})
            };
            let event = json!({"choices": [{"index": 0, "delta": delta, "finish_reason": null}]});
            let end = json!({"choices": [{"index": 0, "delta": {},
                "finish_reason": if answered {"stop"} else {"tool_calls"}}]});
            (
                [("content-type", "text/event-stream")],
                format!("data: {event}\n\ndata: {end}\n\ndata: [DONE]\n\n"),
            )
        }),
    )
}

/// What the tool replied to the model.
async fn tool_reply(db: &Pool<Postgres>, port: u16, arguments: &'static str) -> String {
    std::env::set_var("ALLOW_PRIVATE_AI_BASE_URLS", "true");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let stub = tokio::spawn(async move { axum::serve(listener, model(arguments)).await });
    let flow: FlowValue = serde_json::from_value(json!({"modules": [{"id": "agent", "value": {
        "type": "aiagent",
        "tools": [{"id": "echo", "summary": "echo", "value": {
            "type": "rawscript", "language": "bun", "input_transforms": {},
            "content": "export async function main(x: any) { return x }"}}],
        "input_transforms": {
            "provider": {"type": "static", "value": {"kind": "customai", "model": "m",
                "resource": {"base_url": format!("{base}/v1")}}},
            "user_message": {"type": "static", "value": "go"},
            "max_iterations": {"type": "static", "value": 2}
        }
    }}]}))
    .unwrap();
    let id = RunJob::from(JobPayload::RawFlow {
        value: flow,
        path: Some("u/test-user/agent".into()),
        restarted_from: None,
    })
    .push(db)
    .await;
    let out = run(db, port, id).await;
    stub.abort();
    out["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "tool")
        .unwrap()["content"]
        .as_str()
        .unwrap()
        .to_string()
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn agent_tool_call_resolves_nothing_the_model_wrote(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    seed(&db).await?;
    // jsonb keeps the last copy of a duplicate key, the one a Rust decode refuses to read past.
    let reply = tool_reply(
        &db,
        port,
        r#"{"x": {"n": 1e400, "n": 0, "r": "\u0024var:u/test-user/secret"}}"#,
    )
    .await;
    assert!(!reply.contains(SECRET), "{reply}");
    assert!(reply.contains("Tool call refused"), "{reply}");

    // A tool the model calls without arguments, as Anthropic sends it.
    let reply = tool_reply(&db, port, "").await;
    assert!(!reply.contains("Tool call refused"), "{reply}");

    let reply = tool_reply(
        &db,
        port,
        r#"{"x": {"pipeline": [{"$match": {"currency": "$USD", "_id": "$category"}}]}}"#,
    )
    .await;
    assert_eq!(
        serde_json::from_str::<Value>(&reply)?,
        json!({"pipeline": [{"$match": {"currency": "$USD", "_id": "$category"}}]})
    );
    Ok(())
}
