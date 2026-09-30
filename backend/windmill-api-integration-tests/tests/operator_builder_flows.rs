use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::workspaces::invalidate_operator_rights_cache;
use windmill_test_utils::*;

const WS: &str = "test-workspace";

fn operator_client() -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str("Bearer OPERATOR_TOKEN_1").unwrap(),
    );
    reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap()
}

async fn set_builder(db: &Pool<Postgres>, flows: bool) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workspace_settings SET operator_settings = $1::text::jsonb WHERE workspace_id = $2",
    )
    .bind(format!(r#"{{"builder_flows": {flows}}}"#))
    .bind(WS)
    .execute(db)
    .await?;
    // The right is read through a process-global 60s cache keyed by workspace id.
    invalidate_operator_rights_cache(WS);
    Ok(())
}

async fn add_script(db: &Pool<Postgres>, hash: i64, path: &str, owner: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema,
             summary, description, lock, extra_perms)
         VALUES ($1, $2, $3, 'x', 'bun', 'script', $4, '{}', '', '', '', '{}')",
    )
    .bind(WS)
    .bind(hash)
    .bind(path)
    .bind(owner)
    .execute(db)
    .await?;
    Ok(())
}

fn composition_flow_at(path: &str, step_path: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "",
        "description": "",
        "schema": {},
        "value": {"modules": [{
            "id": "a",
            "value": {"type": "script", "path": step_path, "input_transforms": {}}
        }]}
    })
}

fn inline_code_flow(path: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "",
        "description": "",
        "schema": {},
        "value": {"modules": [{
            "id": "a",
            "value": {
                "type": "rawscript",
                "content": "export async function main() { return 1 }",
                "language": "bun",
                "input_transforms": {}
            }
        }]}
    })
}

/// The whole boundary in one pass: the builder right lets an operator compose runnables that are
/// already deployed and nothing more, and the endpoints that author code stay shut whether or not
/// it is granted.
#[sqlx::test(migrations = "../migrations", fixtures("base", "permissions_test"))]
async fn test_operator_builder_flows_boundary(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let api = format!("http://localhost:{port}/api/w/{WS}");
    let c = operator_client();

    // A composition-only flow references a runnable that exists and the builder can read, so the
    // fixture needs one.
    add_script(&db, 4241, "u/operator/some_script", "operator").await?;

    set_builder(&db, false).await?;
    let resp = c
        .post(format!("{api}/flows/create"))
        .json(&composition_flow_at(
            "u/operator/f1",
            "u/operator/some_script",
        ))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "an operator without the builder right must not create a flow"
    );

    set_builder(&db, true).await?;
    // A payload that omits the key, like a git-sync file that predates it, keeps the right.
    let resp = reqwest::Client::new()
        .post(format!("{api}/workspaces/operator_settings"))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({"runs": true}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let resp = c
        .post(format!("{api}/flows/create"))
        .json(&composition_flow_at(
            "u/operator/f1",
            "u/operator/some_script",
        ))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "a builder must be able to create a composition-only flow: {}",
        resp.text().await?
    );

    let mut with_dyn_code = composition_flow_at("u/operator/f1", "u/operator/some_script");
    with_dyn_code["schema"] =
        json!({"x-windmill-dyn-select-code": "x", "x-windmill-dyn-select-lang": "bun"});
    let resp = c
        .post(format!("{api}/flows/update/u/operator/f1"))
        .json(&with_dyn_code)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "a builder must not save a flow carrying dropdown code"
    );
    let resp = c
        .post(format!("{api}/jobs/run/dynamic_select"))
        .json(
            &json!({"entrypoint_function": "f", "runnable_ref": {"source": "inline", "code": "x"}}),
        )
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "an operator's inline dropdown refusal must not be a 401"
    );

    // A dependency job rewrites bookkeeping stored under the path it names, so a builder must not
    // aim one at a path it cannot write.
    let resp = c
        .post(format!("{api}/jobs/run/flow_dependencies"))
        .json(&json!({"path": "u/alice/private_flow", "flow_value": {"modules": []}}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "a builder must not run a dependency job on a path it cannot write"
    );

    // A legacy draft has no owner, so it lists as the builder's own, and a script draft is one
    // the builder cannot write: that row must read as not writable, not fail the whole list.
    sqlx::query(
        "INSERT INTO draft (workspace_id, path, typ, value) VALUES ($1, 'u/operator/some_script', 'script', '{}')",
    )
    .bind(WS)
    .execute(&db)
    .await?;
    let resp = c.get(format!("{api}/drafts/list")).send().await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let drafts: serde_json::Value = resp.json().await?;
    assert_eq!(drafts[0]["can_write"], false, "{drafts}");

    let resp = c
        .post(format!("{api}/flows/create"))
        .json(&inline_code_flow("u/operator/f2"))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "a builder must not deploy a flow carrying inline code"
    );

    // Draft storage strips a NUL, turning these keys into the plain ones the check reads.
    let mut nul_value = inline_code_flow("u/operator/f1");
    let v = nul_value.as_object_mut().unwrap().remove("value").unwrap();
    nul_value["value\0"] = v;
    let mut nul_dyn = composition_flow_at("u/operator/f1", "u/operator/some_script");
    nul_dyn["schema"] = json!({"x-windmill-dyn-select-code\0": "x"});
    for (draft, expected) in [
        (
            composition_flow_at("u/operator/f1", "u/operator/some_script"),
            200,
        ),
        (inline_code_flow("u/operator/f1"), 403),
        (with_dyn_code.clone(), 403),
        (nul_value, 403),
        (nul_dyn, 403),
    ] {
        let resp = c
            .post(format!("{api}/drafts/update/flow/u/operator/f1"))
            .json(&json!({ "value": draft, "force": true }))
            .send()
            .await?;
        let status = resp.status();
        assert_eq!(
            status,
            expected,
            "flow draft {draft}: {}",
            resp.text().await?
        );
    }

    // Same for the preview path, which runs a request-supplied flow value rather than a stored one.
    let resp = c
        .post(format!("{api}/jobs/run/preview_flow"))
        .json(&json!({"value": inline_code_flow("u/operator/f2")["value"], "args": {}}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "a builder must not preview a flow carrying inline code"
    );

    // Authoring code directly stays shut with the right granted.
    let resp = c
        .post(format!("{api}/scripts/create"))
        .json(&json!({
            "path": "u/operator/s1",
            "summary": "",
            "description": "",
            "content": "export async function main() { return 1 }",
            "language": "bun",
            "is_template": false
        }))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "a builder must not create a script"
    );

    // `permissions_test` gives the operator fixture no rights on `u/alice/**`.
    add_script(&db, 4243, "u/alice/private", "alice").await?;
    let resp = c
        .post(format!("{api}/flows/create"))
        .json(&composition_flow_at("u/operator/f4", "u/alice/private"))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "a builder must not compose a runnable it cannot read"
    );

    add_script(&db, 4242, "u/operator/pinned", "operator").await?;
    let pinned = |hash: &str| {
        json!({
            "path": "u/operator/f3", "summary": "", "description": "", "schema": {},
            "value": {"modules": [{
                "id": "a",
                "value": {
                    "type": "script", "path": "u/operator/pinned", "hash": hash,
                    "input_transforms": {}
                }
            }]}
        })
    };
    let resp = c
        .post(format!("{api}/flows/create"))
        .json(&pinned("0000000000000000"))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        403,
        "a builder must not pin a hash that is not a version of the step's path"
    );
    let resp = c
        .post(format!("{api}/flows/create"))
        .json(&pinned("0000000000001092"))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "a builder must be able to pin the real version of a readable script: {}",
        resp.text().await?
    );

    Ok(())
}
