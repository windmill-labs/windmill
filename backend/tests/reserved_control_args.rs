//! `_ENTRYPOINT_OVERRIDE`, `_MODULES` and `_TEMP_SCRIPT_REFS` steer *how* a runnable runs:
//! which function the worker calls instead of `main`, what module files land in the job
//! directory, where its workspace imports resolve from. They are legitimate for a caller
//! running its own code under its own identity, and must not be settable by a third party
//! whose payload runs under somebody else's: an HTTP route body or native trigger message
//! (the job runs as the trigger's owner) and a public app viewer's inputs (as the
//! publisher).
//!
//! `push` stores the args with their `\u0000` escapes removed, keys included, so every
//! check also has to hold for a NUL-spelled key: `_ENTRYPOINT_OVERRIDE\0` is read back as
//! `_ENTRYPOINT_OVERRIDE`. The same rename is what lets a viewer's `tenant\0` land as
//! `tenant` over an app policy's pinned `tenant`.

use std::collections::HashMap;

use serde_json::{json, value::RawValue, Value};
use sqlx::{Pool, Postgres};
use windmill_common::{
    jobs::JobPayload,
    runnable_settings::{ConcurrencySettings, DebouncingSettings},
    scripts::{ScriptHash, ScriptLang},
    triggers::TriggerMetadata,
};
use windmill_queue::{PushArgs, PushArgsOwned, PushIsolationLevel};
use windmill_test_utils::*;

const SCRIPT_PATH: &str = "f/system/hello";
const SCRIPT_HASH: i64 = 123412;
/// Two top-level functions, as in the report: `main` authenticates, `apply` acts.
const SCRIPT_CONTENT: &str = "export async function main(data: string) { return data }\n\
                              export async function apply(action: string) { return action }\n";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(b: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    b.header("Authorization", format!("Bearer {}", token))
}

fn raw(v: Value) -> Box<RawValue> {
    serde_json::value::to_raw_value(&v).unwrap()
}

fn modules() -> Value {
    json!({ "bunfig.toml": { "content": "preload = [\"./x.ts\"]", "language": "bun" } })
}

/// Deploy `SCRIPT_PATH` (a non-null `lock` is what makes a script count as deployed).
async fn deploy_script(db: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by,
             schema, summary, description, lock)
           VALUES ('test-workspace', $1, $2, $3, 'bun', 'script', 'test-user', '{}', '', '',
             E'{}\n//bun.lock\n<empty>')"#,
    )
    .bind(SCRIPT_HASH)
    .bind(SCRIPT_PATH)
    .bind(SCRIPT_CONTENT)
    .execute(db)
    .await?;
    Ok(())
}

async fn job_args_and_entrypoint(
    db: &Pool<Postgres>,
    id: uuid::Uuid,
) -> anyhow::Result<(Value, Option<String>)> {
    Ok(
        sqlx::query_as("SELECT args, script_entrypoint_override FROM v2_job WHERE id = $1")
            .bind(id)
            .fetch_one(db)
            .await?,
    )
}

fn trigger_authed() -> windmill_api_auth::ApiAuthed {
    windmill_api_auth::ApiAuthed {
        email: "test@windmill.dev".to_string(),
        username: "test-user".to_string(),
        is_admin: true,
        is_operator: false,
        groups: vec![],
        folders: vec![],
        scopes: None,
        username_override: None,
        username_override_is_token_label: false,
        is_session_token: false,
        token_prefix: None,
        read_only: false,
        job_id: None,
        credential_expiry: None,
        mcp_client: None,
    }
}

/// `trigger_runnable_inner` is the one funnel every trigger reaches `push` through, so it
/// stands in here for the HTTP route whose body keys become the job's top-level args. The
/// sender is anonymous on a `none`/`custom_script` route; the job runs as the route owner.
#[sqlx::test(fixtures("base"))]
async fn trigger_args_lose_every_reserved_control_key(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    deploy_script(&db).await?;

    let args = PushArgsOwned {
        args: HashMap::from([
            // Picks `apply` instead of `main`, skipping whatever `main` authenticates.
            ("_ENTRYPOINT_OVERRIDE".to_string(), raw(json!("apply"))),
            ("_ENTRYPOINT_OVERRIDE\u{0}".to_string(), raw(json!("apply"))),
            ("_ENTRY\u{0}POINT_OVERRIDE".to_string(), raw(json!("apply"))),
            ("_MODULES\u{0}".to_string(), raw(modules())),
            (
                "_TEMP_SCRIPT_REFS\u{0}".to_string(),
                raw(json!({ "f/system/dep": "deadbeef" })),
            ),
            // An ordinary input must still get through, NUL-renamed or not.
            ("data".to_string(), raw(json!("payload"))),
            ("action\u{0}".to_string(), raw(json!("refund"))),
        ]),
        extra: None,
    };

    let (id, _, _, _, _) = windmill_trigger::trigger_helpers::trigger_runnable_inner(
        &db,
        None,
        None,
        trigger_authed(),
        "test-workspace",
        SCRIPT_PATH,
        false,
        args,
        None,
        None,
        None,
        "u/test-user/route".to_string(),
        None,
        TriggerMetadata::new(
            Some("u/test-user/route".to_string()),
            windmill_common::jobs::JobTriggerKind::Http,
        ),
        None,
    )
    .await?;

    let (args, entrypoint) = job_args_and_entrypoint(&db, id).await?;
    assert_eq!(
        args,
        json!({ "data": "payload", "action": "refund" }),
        "a trigger payload may carry the runnable's inputs and nothing that steers how it runs"
    );
    assert_eq!(
        entrypoint, None,
        "a trigger payload must not pick the function the worker calls instead of `main`"
    );
    Ok(())
}

/// The counterpart: `_ENTRYPOINT_OVERRIDE` stays for a caller that runs the script under
/// its own identity with `jobs:run` — the script editor's preprocessor test, dynamic-select
/// helpers, the CLI and workflow-as-code all send it this way (GHSA-wxjq-w5pj-jqhx bounds
/// what it may contain, not who may set it). Stripping it in `push` would break them.
#[sqlx::test(fixtures("base"))]
async fn an_authenticated_push_keeps_the_entrypoint_override(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    deploy_script(&db).await?;

    // Both spellings: `push` normalizes the names to what it stores, so the NUL-spelled one
    // selects the function exactly like the plain one instead of being silently different.
    for key in ["_ENTRYPOINT_OVERRIDE", "_ENTRYPOINT_OVERRIDE\u{0}"] {
        let args = HashMap::from([
            (key.to_string(), raw(json!("apply"))),
            ("action".to_string(), raw(json!("refund"))),
        ]);
        let (id, tx) = windmill_queue::push(
            &db,
            PushIsolationLevel::IsolatedRoot(db.clone()),
            "test-workspace",
            JobPayload::ScriptHash {
                hash: ScriptHash(SCRIPT_HASH),
                path: SCRIPT_PATH.to_string(),
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                language: ScriptLang::Bun,
                priority: None,
                apply_preprocessor: false,
                concurrency_settings: ConcurrencySettings::default(),
                debouncing_settings: DebouncingSettings::default(),
                labels: None,
                job_token_scopes: None,
            },
            PushArgs { args: &args, extra: None },
            "test-user",
            "test@windmill.dev",
            "u/test-user".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            None,
            true,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            None,
        )
        .await?;
        tx.commit().await?;
        let (args, entrypoint) = job_args_and_entrypoint(&db, id).await?;
        assert_eq!(
            entrypoint.as_deref(),
            Some("apply"),
            "an authenticated caller running a script as itself may still pick the entrypoint \
             (sent as {key:?})"
        );
        assert_eq!(
            args,
            json!({ "_ENTRYPOINT_OVERRIDE": "apply", "action": "refund" }),
            "the stored name is the normalized one (sent as {key:?})"
        );
    }
    Ok(())
}

/// Create a public (anonymous) app whose single component is the deployed script, with the
/// given policy inputs, and return its path.
async fn create_public_app(
    ws: &str,
    path: &str,
    static_inputs: Value,
    one_of_inputs: Value,
) -> anyhow::Result<()> {
    let resp = authed(client().post(format!("{ws}/apps/create")), "SECRET_TOKEN")
        .json(&json!({
            "path": path,
            "summary": "",
            "value": {},
            "policy": {
                "execution_mode": "anonymous",
                "triggerables_v2": {
                    format!("comp:script/{SCRIPT_PATH}"): {
                        "static_inputs": static_inputs,
                        "one_of_inputs": one_of_inputs,
                    }
                }
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create app: {}", resp.text().await?);
    Ok(())
}

/// A viewer of a public app runs the publisher's pinned script as the publisher: it may
/// fill in the inputs the policy leaves free and nothing else.
#[sqlx::test(fixtures("base"))]
async fn public_app_viewer_cannot_steer_the_pinned_runnable(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let ws = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    deploy_script(&db).await?;

    let app_path = "u/test-user/reserved_args";
    create_public_app(
        &ws,
        app_path,
        json!({ "tenant": "acme" }),
        json!({ "greeting": ["Hello,", "Bonjour,"] }),
    )
    .await?;

    // No `Authorization` header at all: the anonymous visitor of a public app.
    let run = |args: &'static str| {
        client()
            .post(format!("{ws}/apps_u/execute_component/{app_path}"))
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"component": "comp", "path": "script/{SCRIPT_PATH}", "args": {args}}}"#
            ))
            .send()
    };

    // A name spelled with a NUL is the same name once stored, so the policy's pinned value
    // must still win and the one-of list must still be enforced.
    let resp = run(r#"{"tenant\u0000": "globex", "greeting": "Hello,"}"#).await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 200, "run must be accepted: {body}");
    let (args, _) = job_args_and_entrypoint(&db, uuid::Uuid::parse_str(body.trim())?).await?;
    assert_eq!(
        args["tenant"],
        json!("acme"),
        "a NUL-renamed arg must not override an input the policy pins: {args}"
    );

    let resp = run(r#"{"greeting\u0000": "anything"}"#).await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 400,
        "a NUL-renamed arg must still be held to the policy's one_of list: {body}"
    );
    assert!(
        body.contains("must be one of"),
        "expected the one_of rejection, got: {body}"
    );

    // The control keys, in both spellings, in one request.
    let resp = run(
        r#"{"_ENTRYPOINT_OVERRIDE": "apply", "_ENTRYPOINT_OVERRIDE\u0000": "apply",
            "_MODULES": {"bunfig.toml": {"content": "x", "language": "bun"}},
            "_MODULES\u0000": {"bunfig.toml": {"content": "x", "language": "bun"}},
            "_TEMP_SCRIPT_REFS": {"f/system/dep": "deadbeef"},
            "_TEMP_SCRIPT_REFS\u0000": {"f/system/dep": "deadbeef"},
            "greeting": "Hello,"}"#,
    )
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 200, "run must be accepted: {body}");
    let (args, entrypoint) =
        job_args_and_entrypoint(&db, uuid::Uuid::parse_str(body.trim())?).await?;
    assert_eq!(
        entrypoint, None,
        "a viewer must not pick the function the worker calls instead of `main`"
    );
    assert_eq!(
        args,
        json!({ "tenant": "acme", "greeting": "Hello," }),
        "only the policy's pinned input and the viewer's allowed one may reach the job: {args}"
    );
    Ok(())
}

/// A deployed flow's dynamic-select code has no deployed runnable, so it is queued as a
/// `JobKind::Preview` job — the one kind on which the bun and python executors honor
/// `_TEMP_SCRIPT_REFS` and resolve the code's workspace imports from the temporary script
/// store, which any workspace member can write. The caller of this route only fills in that
/// code's inputs (it may be an operator, who is refused `jobs/run/preview` outright), so the
/// key must not reach the job, in either spelling.
#[sqlx::test(fixtures("base"))]
async fn flow_dynamic_select_strips_caller_control_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let ws = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );

    // Someone else's flow: `test-user-2` below can run it, not edit it.
    let flow_path = "u/test-user/dynselect_control_args";
    let resp = authed(client().post(format!("{ws}/flows/create")), "SECRET_TOKEN")
        .json(&json!({
            "path": flow_path,
            "summary": "",
            "description": "",
            "value": { "modules": [] },
            "schema": {
                "type": "object",
                "properties": { "region": { "type": "object", "format": "dynselect-region" } },
                "x-windmill-dyn-select-lang": "bun",
                "x-windmill-dyn-select-code":
                    "import { regions } from \"/f/system/regions\"\n\
                     export async function region() { return regions() }\n"
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create flow: {}", resp.text().await?);

    // Read-only for the caller below: running the dyn-select code is all it may do.
    sqlx::query(
        "UPDATE flow SET extra_perms = '{\"u/test-user-2\": false}'
         WHERE path = $1 AND workspace_id = 'test-workspace'",
    )
    .bind(flow_path)
    .execute(&db)
    .await?;

    let refs = json!({ "f/system/regions": "deadbeef" });
    let modules = json!({ "regions.ts": { "content": "export const v = 1", "language": "bun" } });
    let resp = authed(
        client().post(format!("{ws}/jobs/run/dynamic_select")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "entrypoint_function": "region",
        "runnable_ref": { "source": "deployed", "path": flow_path, "runnable_kind": "flow" },
        "args": {
            "_TEMP_SCRIPT_REFS": refs,
            "_TEMP_SCRIPT_REFS\u{0}": refs,
            "_MODULES": modules,
            "_MODULES\u{0}": modules,
            "filterText": "eu"
        }
    }))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 201, "dynamic select must be accepted: {body}");

    let id = uuid::Uuid::parse_str(body.trim())?;
    let kind: String = sqlx::query_scalar("SELECT kind::text FROM v2_job WHERE id = $1")
        .bind(id)
        .fetch_one(&db)
        .await?;
    assert_eq!(
        kind, "preview",
        "the premise of this test: a deployed flow's dyn-select code runs as a preview job"
    );
    let (args, _) = job_args_and_entrypoint(&db, id).await?;
    assert_eq!(
        args,
        json!({ "_ENTRYPOINT_OVERRIDE": "region", "filterText": "eu" }),
        "only the caller's inputs and the server-set entrypoint may reach the job"
    );
    Ok(())
}
