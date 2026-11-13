use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const WS: &str = "test-workspace";

fn client_for(token: &str) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap()
}

async fn post(
    client: &reqwest::Client,
    port: u16,
    route: &str,
    body: serde_json::Value,
) -> anyhow::Result<u16> {
    let r = client
        .post(format!("http://localhost:{port}/api/w/{WS}/{route}"))
        .json(&body)
        .send()
        .await?;
    Ok(r.status().as_u16())
}

fn bash_script(path: &str, parent_hash: Option<&str>) -> serde_json::Value {
    json!({
        "path": path, "parent_hash": parent_hash, "summary": "", "description": "",
        "content": "echo hi", "language": "bash", "schema": {"type": "object", "properties": {}}
    })
}

/// A view-only grant shows a runnable and its runs but starts nothing, while a read grant made
/// the way every client made one before levels existed (`write: false`) keeps running.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_view_only_grants(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let admin = client_for("SECRET_TOKEN");
    let viewer = client_for("SECRET_TOKEN_2"); // test-user-2
    let runner = client_for("SECRET_TOKEN_3"); // test-user-3

    assert_eq!(
        post(&admin, port, "folders/create", json!({"name": "vo"})).await?,
        200
    );
    let grant = |owner: &str, body: serde_json::Value| {
        let mut body = body;
        body["owner"] = json!(owner);
        post(&admin, port, "acls/add/folder/vo", body)
    };
    assert_eq!(grant("u/test-user-2", json!({"level": "read"})).await?, 200);
    assert_eq!(grant("u/test-user-3", json!({"write": false})).await?, 200);
    assert_eq!(
        post(&admin, port, "scripts/create", bash_script("f/vo/s", None)).await?,
        201
    );
    let flow = json!({"path": "f/vo/fl", "summary": "", "value": {"modules": []},
        "schema": {"type": "object", "properties": {}}});
    assert_eq!(post(&admin, port, "flows/create", flow).await?, 201);

    let r = viewer
        .get(format!(
            "http://localhost:{port}/api/w/{WS}/scripts/get/p/f/vo/s"
        ))
        .send()
        .await?;
    assert_eq!(r.status(), 200, "the viewer reads the script");
    assert_eq!(
        post(&viewer, port, "jobs/run/p/f/vo/s", json!({})).await?,
        403
    );
    assert_eq!(
        post(&viewer, port, "jobs/run/f/f/vo/fl", json!({})).await?,
        403
    );
    let schedule = json!({"path": "u/test-user-2/s", "schedule": "0 0 0 1 1 *", "timezone": "UTC",
        "script_path": "f/vo/s", "is_flow": false, "args": {}, "enabled": false});
    assert_eq!(
        post(&viewer, port, "schedules/create", schedule).await?,
        403,
        "a schedule would run the script as the viewer"
    );
    assert_eq!(
        post(&runner, port, "jobs/run/p/f/vo/s", json!({})).await?,
        201
    );
    // The run just cached for the runner must not outlive their demotion.
    assert_eq!(grant("u/test-user-3", json!({"level": "read"})).await?, 200);
    assert_eq!(
        post(&runner, port, "jobs/run/p/f/vo/s", json!({})).await?,
        403
    );

    // The viewer reads the admin's queued run but cannot stop it, one by one or in a selection.
    let job = admin
        .post(format!(
            "http://localhost:{port}/api/w/{WS}/jobs/run/p/f/vo/s?scheduled_for=2100-01-01T00:00:00Z"
        ))
        .json(&json!({}))
        .send()
        .await?
        .text()
        .await?;
    assert_eq!(
        post(
            &viewer,
            port,
            &format!("jobs_u/queue/cancel/{job}"),
            json!({})
        )
        .await?,
        403
    );
    let cancelled: Vec<String> = viewer
        .post(format!(
            "http://localhost:{port}/api/w/{WS}/jobs/queue/cancel_selection"
        ))
        .json(&json!([job]))
        .send()
        .await?
        .json()
        .await?;
    assert!(cancelled.is_empty(), "cancelled {cancelled:?}");

    // Grants add up: an item grant that runs lifts the folder's view-only one.
    let item_grant = json!({"owner": "u/test-user-2", "level": "run"});
    assert_eq!(
        post(&admin, port, "acls/add/script/f/vo/s", item_grant).await?,
        200
    );
    assert_eq!(
        post(&viewer, port, "jobs/run/p/f/vo/s", json!({})).await?,
        201
    );

    // Kinds without a run level share as before.
    let var = json!({"path": "u/test-user/v", "value": "x", "is_secret": false, "description": ""});
    assert_eq!(post(&admin, port, "variables/create", var).await?, 201);
    let share = json!({"owner": "u/test-user-2", "write": false});
    assert_eq!(
        post(&admin, port, "acls/add/variable/u/test-user/v", share).await?,
        200
    );

    // An item-level view-only grant, kept by the next version of the script.
    let own = bash_script("u/test-user/own", None);
    assert_eq!(post(&admin, port, "scripts/create", own).await?, 201);
    let view_only = json!({"owner": "u/test-user-3", "level": "read"});
    assert_eq!(
        post(&admin, port, "acls/add/script/u/test-user/own", view_only).await?,
        200
    );
    assert_eq!(
        post(&runner, port, "jobs/run/p/u/test-user/own", json!({})).await?,
        403
    );
    let deployed: serde_json::Value = admin
        .get(format!(
            "http://localhost:{port}/api/w/{WS}/scripts/get/p/u/test-user/own"
        ))
        .send()
        .await?
        .json()
        .await?;
    let next = bash_script("u/test-user/own", deployed["hash"].as_str());
    assert_eq!(post(&admin, port, "scripts/create", next).await?, 201);
    assert_eq!(
        post(&runner, port, "jobs/run/p/u/test-user/own", json!({})).await?,
        403
    );

    Ok(())
}
