//! On the shared cloud (`CLOUD_HOSTED`), public (`anonymous`) apps take a paid plan.
//!
//! One test in its own binary on purpose: `CLOUD_HOSTED` is read once into a
//! `lazy_static`, so it must be set before anything reads it.
#![cfg(feature = "cloud")]

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";
const APP_PATH: &str = "u/test-user/public_app";

fn post(url: String) -> reqwest::RequestBuilder {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {ADMIN_TOKEN}"))
}

fn public_app() -> serde_json::Value {
    json!({
        "path": APP_PATH,
        "summary": "Public app",
        "value": {},
        "policy": { "execution_mode": "anonymous", "triggerables": {} }
    })
}

#[sqlx::test(fixtures("base"))]
async fn public_apps_take_a_paid_plan(db: Pool<Postgres>) -> anyhow::Result<()> {
    unsafe { std::env::set_var("CLOUD_HOSTED", "true") };
    initialize_tracing().await;
    // A fork of a paid root draws the root's plan, though it is not premium itself.
    sqlx::raw_sql(
        "INSERT INTO workspace (id, name, owner, premium) VALUES ('paid-root', 'paid-root', 'test-user', true);
         INSERT INTO workspace (id, name, owner, parent_workspace_id)
             VALUES ('paid-fork', 'paid-fork', 'test-user', 'paid-root');
         INSERT INTO workspace_settings (workspace_id) VALUES ('paid-root'), ('paid-fork');
         INSERT INTO usr (workspace_id, email, username, is_admin, role)
             VALUES ('paid-fork', 'test@windmill.dev', 'test-user', true, 'Admin');",
    )
    .execute(&db)
    .await?;
    let server = ApiServer::start(db.clone()).await?;
    let api = format!("http://localhost:{}/api/w", server.addr.port());
    let ws = format!("{api}/test-workspace");

    let resp = post(format!("{ws}/apps/create"))
        .json(&public_app())
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        400,
        "a free workspace cannot create a public app"
    );
    assert!(resp.text().await?.contains("paid plan"));

    // An app made public before the rule: its next version is refused too.
    let mut app = public_app();
    app["policy"]["execution_mode"] = json!("publisher");
    let resp = post(format!("{ws}/apps/create")).json(&app).send().await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    let resp = post(format!("{ws}/apps/update/{APP_PATH}"))
        .json(&json!({ "execution_mode": "anonymous" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "an app cannot be switched to public");
    sqlx::query(
        "UPDATE app SET policy = jsonb_set(policy, '{execution_mode}', '\"anonymous\"')
         WHERE path = $1 AND workspace_id = 'test-workspace'",
    )
    .bind(APP_PATH)
    .execute(&db)
    .await?;
    let resp = post(format!("{ws}/apps/update/{APP_PATH}"))
        .json(&json!({ "value": { "grid": [] } }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        400,
        "an already-public app cannot be redeployed"
    );

    // Taking it out of public stays allowed.
    let resp = post(format!("{ws}/apps/update/{APP_PATH}"))
        .json(&json!({ "execution_mode": "publisher" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let resp = post(format!("{api}/paid-fork/apps/create"))
        .json(&public_app())
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "a fork of a paid root may publish: {}",
        resp.text().await?
    );
    Ok(())
}
