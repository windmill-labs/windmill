use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_common::variables::{build_crypt, encrypt};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn proxied(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    authed(builder).header("X-Windmill-Remote-Deploy", "1")
}

/// The deploy target is this very server, which the proxy has no way to tell from another
/// instance: it only ever knows the configured URL, the caller's stored token, and the path.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_remote_deploy_proxy(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/remote_deploy");

    // Planted rather than set through the route, which is enterprise-gated.
    let set_target = |base_url: String, workspace_id: &'static str| {
        let db = db.clone();
        async move {
            sqlx::query!(
                "UPDATE workspace_settings SET remote_deploy_target = $1
                 WHERE workspace_id = 'test-workspace'",
                json!({ "base_url": base_url, "workspace_id": workspace_id })
            )
            .execute(&db)
            .await
            .unwrap();
        }
    };
    set_target(format!("http://localhost:{port}"), "test-workspace").await;

    let resp = authed(client().get(format!("{base}/target")))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let status = resp.json::<serde_json::Value>().await?;
    assert_eq!(status["target"]["workspace_id"], "test-workspace");
    assert!(status["connection"].is_null());

    // Deploying before connecting names the step that is missing, rather than reaching the target
    // with the caller's credentials for this instance.
    let resp = proxied(client().get(format!("{base}/proxy/users/whoami")))
        .send()
        .await?;
    assert_eq!(resp.status(), 400);
    assert!(resp.text().await?.contains("Connect to"));

    // A token the target refuses is not stored.
    let resp = authed(client().post(format!("{base}/connect")))
        .json(&json!({"token": "not-a-token"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 400);

    let resp = authed(client().post(format!("{base}/connect")))
        .json(&json!({"token": "SECRET_TOKEN"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?["remote_email"],
        "test@windmill.dev"
    );

    let resp = proxied(client().get(format!("{base}/proxy/users/whoami")))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    // Whatever the remote returns is served from this origin, so it must never render as a page.
    let csp = resp.headers()["content-security-policy"]
        .to_str()?
        .to_string();
    assert!(csp.starts_with("sandbox"), "{csp}");
    assert_eq!(
        resp.json::<serde_json::Value>().await?["email"],
        "test@windmill.dev"
    );

    // A link (a navigation, or a page on another origin) cannot set the header, so it cannot
    // spend the stored token.
    let resp = authed(client().get(format!("{base}/proxy/users/whoami")))
        .send()
        .await?;
    assert_eq!(resp.status(), 400);

    // A target that refuses the stored token must not answer 401: the browser reads an
    // unhandled 401 as its own session having expired and logs the user out of this instance.
    let mc = build_crypt(&db, "test-workspace").await?;
    sqlx::query!(
        "UPDATE remote_deploy_token SET token = $1 WHERE workspace_id = 'test-workspace'",
        encrypt(&mc, "revoked-token")
    )
    .execute(&db)
    .await?;
    let resp = proxied(client().get(format!("{base}/proxy/users/whoami")))
        .send()
        .await?;
    assert_eq!(resp.status(), 502);

    // Rows are keyed by email alone: deleting the account must take its token with it, or the
    // next account created with that address would act on the remote as this one.
    let resp = client()
        .post(format!("{base}/connect"))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .json(&json!({"token": "SECRET_TOKEN_2"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let resp = authed(client().delete(format!(
        "http://localhost:{port}/api/users/delete/test2@windmill.dev"
    )))
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let left = sqlx::query_scalar!(
        "SELECT count(*) FROM remote_deploy_token WHERE email = 'test2@windmill.dev'"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(left, Some(0));

    // The token was granted for one target and is never sent to another, so re-pointing the
    // workspace leaves the caller unconnected instead of handing its token to the new target.
    set_target(format!("http://localhost:{port}"), "other-workspace").await;
    let resp = authed(client().get(format!("{base}/target")))
        .send()
        .await?;
    assert!(resp.json::<serde_json::Value>().await?["connection"].is_null());
    let resp = proxied(client().get(format!("{base}/proxy/users/whoami")))
        .send()
        .await?;
    assert_eq!(resp.status(), 400);

    Ok(())
}
