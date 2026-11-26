//! `deploy_from_workspace`: the caller's own access in each workspace, a secret re-encrypted
//! under the target's key, and a create-only folder translated to the target's users.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

async fn call(
    port: u16,
    token: &str,
    method: reqwest::Method,
    url: &str,
    body: Option<Value>,
) -> anyhow::Result<(u16, String)> {
    let mut req = reqwest::Client::new()
        .request(method, format!("http://localhost:{port}/api/{url}"))
        .header("Authorization", format!("Bearer {token}"));
    if let Some(body) = body {
        req = req.json(&body);
    }
    let resp = req.send().await?;
    Ok((resp.status().as_u16(), resp.text().await?))
}

async fn deploy(port: u16, token: &str, body: Value) -> anyhow::Result<(u16, String)> {
    call(
        port,
        token,
        reqwest::Method::POST,
        "w/wd-parent/workspaces/deploy_from_workspace",
        Some(body),
    )
    .await
}

#[sqlx::test(fixtures("workspace_deploy"))]
async fn test_deploy_from_workspace(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let admin = "WD_ADMIN_TOKEN";

    let (status, body) = call(
        port,
        admin,
        reqwest::Method::POST,
        "w/wd-fork/variables/create",
        Some(json!({ "path": "u/wd-admin/secret", "value": "s3cr3t", "is_secret": true, "description": "" })),
    )
    .await?;
    assert_eq!(status, 201, "{body}");

    // A member of the target who cannot read the source gets nothing out of it.
    let (status, body) = deploy(
        port,
        "WD_OUTSIDER_TOKEN",
        json!({ "source_workspace": "wd-fork", "kind": "variable", "path": "u/wd-admin/secret" }),
    )
    .await?;
    assert_eq!(status, 401, "{body}");

    // The two workspaces have different keys: the value must arrive decrypted, then re-encrypted.
    let (status, body) = deploy(
        port,
        admin,
        json!({ "source_workspace": "wd-fork", "kind": "variable", "path": "u/wd-admin/secret" }),
    )
    .await?;
    assert_eq!(status, 200, "{body}");
    let (_, body) = call(
        port,
        admin,
        reqwest::Method::GET,
        "w/wd-parent/variables/get_value/u/wd-admin/secret",
        None,
    )
    .await?;
    assert_eq!(serde_json::from_str::<String>(&body)?, "s3cr3t");

    // Create-only refuses to overwrite.
    let (status, _) = deploy(
        port,
        admin,
        json!({ "source_workspace": "wd-fork", "kind": "variable", "path": "u/wd-admin/secret", "create_only": true }),
    )
    .await?;
    assert_eq!(status, 409);

    // A create-only folder names the target's users: dev's username differs between the two,
    // and a group missing from the target is dropped rather than carried.
    let (status, body) = call(
        port,
        admin,
        reqwest::Method::POST,
        "w/wd-fork/folders/create",
        Some(json!({ "name": "shared", "owners": ["u/dev-in-fork", "g/forkonly"], "extra_perms": { "u/dev-in-fork": true, "g/forkonly": false } })),
    )
    .await?;
    assert_eq!(status, 200, "{body}");
    let (status, body) = deploy(
        port,
        admin,
        json!({ "source_workspace": "wd-fork", "kind": "folder", "path": "f/shared", "create_only": true }),
    )
    .await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body)?["dropped_access"],
        json!(["g/forkonly", "g/forkonly"])
    );
    let (_, body) = call(
        port,
        admin,
        reqwest::Method::GET,
        "w/wd-parent/folders/get/shared",
        None,
    )
    .await?;
    let folder: Value = serde_json::from_str(&body)?;
    let owners = folder["owners"].as_array().unwrap();
    assert!(owners.contains(&json!("u/dev-in-parent")), "{folder}");
    assert!(
        !owners
            .iter()
            .any(|o| o == "u/dev-in-fork" || o == "g/forkonly"),
        "{folder}"
    );

    // A step's script hash names a source version and is cleared; a static input shaped like a
    // step is data and is kept.
    let (status, hash) = call(
        port,
        admin,
        reqwest::Method::POST,
        "w/wd-fork/scripts/create",
        Some(json!({ "path": "u/wd-admin/step", "summary": "", "description": "", "content": "echo 1", "language": "bash" })),
    )
    .await?;
    assert_eq!(status, 201, "{hash}");
    let lookalike = json!({ "type": "script", "hash": "keep" });
    let step = json!({ "type": "script", "path": "u/wd-admin/step", "hash": hash,
        "input_transforms": { "x": { "type": "static", "value": lookalike } } });
    let (status, body) = call(
        port,
        admin,
        reqwest::Method::POST,
        "w/wd-fork/flows/create",
        Some(
            json!({ "path": "u/wd-admin/flow", "summary": "", "description": "", "schema": {},
            "value": { "modules": [{ "id": "a", "value": step }] } }),
        ),
    )
    .await?;
    assert_eq!(status, 201, "{body}");
    let (status, body) = deploy(
        port,
        admin,
        json!({ "source_workspace": "wd-fork", "kind": "flow", "path": "u/wd-admin/flow" }),
    )
    .await?;
    assert_eq!(status, 200, "{body}");
    let (_, body) = call(
        port,
        admin,
        reqwest::Method::GET,
        "w/wd-parent/flows/get/u/wd-admin/flow",
        None,
    )
    .await?;
    let step = &serde_json::from_str::<Value>(&body)?["value"]["modules"][0]["value"];
    assert!(step.get("hash").is_none(), "{step}");
    assert_eq!(step["input_transforms"]["x"]["value"], lookalike);

    // Removing archives a script rather than deleting it.
    let (status, body) = call(
        port,
        admin,
        reqwest::Method::POST,
        "w/wd-parent/scripts/create",
        Some(json!({ "path": "f/shared/s", "summary": "", "description": "", "content": "echo 1", "language": "bash" })),
    )
    .await?;
    assert_eq!(status, 201, "{body}");
    let (status, body) = deploy(
        port,
        admin,
        json!({ "kind": "script", "path": "f/shared/s", "delete": true }),
    )
    .await?;
    assert_eq!(status, 200, "{body}");
    let (_, body) = call(
        port,
        admin,
        reqwest::Method::GET,
        "w/wd-parent/scripts/get/p/f/shared/s",
        None,
    )
    .await?;
    assert_eq!(
        serde_json::from_str::<Value>(&body)?["archived"],
        json!(true)
    );

    Ok(())
}
