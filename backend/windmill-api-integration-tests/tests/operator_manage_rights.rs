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

fn new_schedule(path: &str) -> serde_json::Value {
    json!({
        "path": path,
        "schedule": "0 0 * * * *",
        "timezone": "UTC",
        "script_path": "u/operator/some_script",
        "is_flow": false,
        "enabled": false,
    })
}

/// Schedules and triggers are rights operators hold until an admin withdraws them, so the stored
/// setting has to survive a payload that never mentions it: this endpoint takes whole-object
/// bodies from git-sync files written before the keys existed, and either default would let such a
/// file silently flip the right on every pull.
#[sqlx::test(migrations = "../migrations", fixtures("base", "permissions_test"))]
async fn test_operator_manage_rights(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let api = format!("http://localhost:{port}/api/w/{WS}");
    let admin = reqwest::Client::new();
    let c = operator_client();

    let set_settings = async |body: serde_json::Value| -> anyhow::Result<u16> {
        let resp = admin
            .post(format!("{api}/workspaces/operator_settings"))
            .header("Authorization", "Bearer SECRET_TOKEN")
            .json(&body)
            .send()
            .await?;
        let status = resp.status().as_u16();
        invalidate_operator_rights_cache(WS);
        Ok(status)
    };

    // Never configured: the right is held.
    let resp = c
        .post(format!("{api}/schedules/create"))
        .json(&new_schedule("u/operator/sched_default"))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    // An admin withdraws it.
    assert_eq!(set_settings(json!({"manage_schedules": false})).await?, 200);

    let resp = c
        .post(format!("{api}/schedules/create"))
        .json(&new_schedule("u/operator/sched_withdrawn"))
        .send()
        .await?;
    assert_eq!(resp.status(), 401, "{}", resp.text().await?);

    // A payload omitting the key must not restore it. This is what an older git-sync settings file
    // looks like, and what a serde or SQL default of either polarity would get wrong.
    assert_eq!(set_settings(json!({"runs": true})).await?, 200);

    let resp = c
        .post(format!("{api}/schedules/create"))
        .json(&new_schedule("u/operator/sched_still_withdrawn"))
        .send()
        .await?;
    assert_eq!(resp.status(), 401, "{}", resp.text().await?);

    // And the other direction: omitting the key must not withdraw a stored grant, which is what a
    // plain `bool` field would do by serializing its own default over it.
    assert_eq!(set_settings(json!({"manage_schedules": true})).await?, 200);
    assert_eq!(set_settings(json!({"runs": true})).await?, 200);

    let resp = c
        .post(format!("{api}/schedules/create"))
        .json(&new_schedule("u/operator/sched_restored"))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    Ok(())
}
