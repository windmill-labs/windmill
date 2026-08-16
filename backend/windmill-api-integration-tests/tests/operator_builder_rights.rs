use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::workspaces::invalidate_operator_builder_cache;
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

async fn set_builder(db: &Pool<Postgres>, enabled: bool) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workspace_settings SET operator_settings = $1::text::jsonb WHERE workspace_id = $2",
    )
    .bind(format!(r#"{{"builder": {enabled}}}"#))
    .bind(WS)
    .execute(db)
    .await?;
    // The flag is read through a process-global 60s cache keyed by workspace id.
    invalidate_operator_builder_cache(WS);
    Ok(())
}

fn composition_flow(path: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "",
        "description": "",
        "schema": {},
        "value": {"modules": [{
            "id": "a",
            "value": {"type": "script", "path": "u/operator/some_script", "input_transforms": {}}
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

/// The whole boundary in one pass: builder rights let an operator compose deployed runnables and
/// nothing more, and the endpoints that author code stay shut whether or not they are granted.
#[sqlx::test(migrations = "../migrations", fixtures("base", "permissions_test"))]
async fn test_operator_builder_rights_boundary(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let api = format!("http://localhost:{port}/api/w/{WS}");
    let c = operator_client();

    set_builder(&db, false).await?;
    let resp = c
        .post(format!("{api}/flows/create"))
        .json(&composition_flow("u/operator/f1"))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "an operator without builder rights must not create a flow"
    );

    set_builder(&db, true).await?;

    let resp = c
        .post(format!("{api}/flows/create"))
        .json(&composition_flow("u/operator/f1"))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "a builder must be able to create a composition-only flow: {}",
        resp.text().await?
    );

    let resp = c
        .post(format!("{api}/flows/create"))
        .json(&inline_code_flow("u/operator/f2"))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "a builder must not deploy a flow carrying inline code"
    );

    // Same for the preview path, which runs a request-supplied flow value rather than a stored one.
    let resp = c
        .post(format!("{api}/jobs/run/preview_flow"))
        .json(&json!({"value": inline_code_flow("u/operator/f2")["value"], "args": {}}))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "a builder must not preview a flow carrying inline code"
    );

    // Authoring code, in any of its shapes, stays shut with builder rights granted.
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

    // `*_raw_source` compiles caller-supplied sources with a bundler job on a worker. It sits
    // beside `create_app_raw`/`update_app_raw`, which builders MAY use, so it is the gate most
    // likely to be opened by mistake later.
    let raw_source_app = json!({
        "path": "u/operator/a1",
        "summary": "",
        "value": {"files": {"index.ts": "console.log(1)"}, "runnables": {}},
        "policy": {"execution_mode": "publisher"}
    });
    let resp = c
        .post(format!("{api}/apps/create_raw_source"))
        .json(&raw_source_app)
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "a builder must not compile app sources on a worker"
    );
    let resp = c
        .post(format!("{api}/apps/update_raw_source/u/operator/a1"))
        .json(&json!({"summary": "x"}))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "a builder must not compile app sources on a worker"
    );

    // Low-code apps carry inline scripts, so they stay shut too.
    let resp = c
        .post(format!("{api}/apps/create"))
        .json(&json!({
            "path": "u/operator/a2",
            "summary": "",
            "value": {"grid": []},
            "policy": {"execution_mode": "publisher"}
        }))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "a builder must not create a low-code app"
    );

    // A version-pinned step dispatches on its hash alone, so the pair must be real and readable:
    // otherwise a builder pins the hash of a script it cannot reach and runs that instead.
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema,
             summary, description, lock, extra_perms)
         VALUES ($1, 4242, 'u/operator/pinned', 'x', 'bun', 'script', 'operator', '{}', '', '', '', '{}')",
    )
    .bind(WS)
    .execute(&db)
    .await?;

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
    assert!(
        !resp.status().is_success(),
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

    invalidate_operator_builder_cache(WS);
    Ok(())
}
