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
    let resp = authed(client().get(format!("{base}/proxy/none/users/whoami")))
        .send()
        .await?;
    assert_eq!(resp.status(), 400);
    assert!(resp.text().await?.contains("Connect to"));

    let target =
        json!({ "base_url": format!("http://localhost:{port}"), "workspace_id": "test-workspace" });

    // A token obtained for another target is refused before it is sent anywhere.
    let resp = authed(client().post(format!("{base}/connect")))
        .json(&json!({"token": "SECRET_TOKEN", "target": { "base_url": format!("http://localhost:{port}"), "workspace_id": "elsewhere" }}))
        .send()
        .await?;
    assert_eq!(resp.status(), 400);

    // A token the target refuses is not stored.
    let resp = authed(client().post(format!("{base}/connect")))
        .json(&json!({"token": "not-a-token", "target": target}))
        .send()
        .await?;
    assert_eq!(resp.status(), 400);

    let resp = authed(client().post(format!("{base}/connect")))
        .json(&json!({"token": "SECRET_TOKEN", "target": target}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let connection = resp.json::<serde_json::Value>().await?;
    assert_eq!(connection["remote_email"], "test@windmill.dev");
    let key = connection["proxy_key"].as_str().unwrap().to_string();

    let resp = authed(client().get(format!("{base}/proxy/{key}/users/whoami")))
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

    // A link from elsewhere rides the session cookie but cannot know the key, so it cannot spend
    // the stored token.
    // Nor can a credential that may not use the proxy read the key, to build such a link itself.
    sqlx::query!(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, read_only)
         VALUES (encode(sha256('READ_ONLY_TOKEN'::bytea), 'hex'), 'READ_ONLY_', 'READ_ONLY_TOKEN',
                 'test@windmill.dev', 'read only', false, true)"
    )
    .execute(&db)
    .await?;
    let resp = client()
        .get(format!("{base}/target"))
        .header("Authorization", "Bearer READ_ONLY_TOKEN")
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    assert!(resp.json::<serde_json::Value>().await?["connection"].is_null());

    let guessed = "x".repeat(key.len());
    let resp = authed(client().get(format!("{base}/proxy/{guessed}/users/whoami")))
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
    let resp = authed(client().get(format!("{base}/proxy/{key}/users/whoami")))
        .send()
        .await?;
    assert_eq!(resp.status(), 502);

    // Deleting the account must take its token with it, or the next account created with that
    // address would act on the remote as this one.
    let resp = client()
        .post(format!("{base}/connect"))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .json(&json!({"token": "SECRET_TOKEN_2", "target": target}))
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
    let resp = authed(client().get(format!("{base}/proxy/{key}/users/whoami")))
        .send()
        .await?;
    assert_eq!(resp.status(), 400);

    Ok(())
}

/// A connect waits on the remote, which leaves time for the caller to be removed from the
/// workspace: its row must not land after the removal's cleanup, to come back on a re-add.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_remote_deploy_connect_racing_workspace_removal(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // A remote whose `whoami` answers only once the removal below has committed.
    let (reached_tx, reached_rx) = tokio::sync::oneshot::channel::<()>();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
    let gate = std::sync::Arc::new(tokio::sync::Mutex::new(Some((reached_tx, release_rx))));
    let remote = axum::Router::new().route(
        "/api/w/prod/users/whoami",
        axum::routing::get(move || {
            let gate = gate.clone();
            async move {
                if let Some((reached, release)) = gate.lock().await.take() {
                    let _ = reached.send(());
                    let _ = release.await;
                }
                axum::Json(json!({ "email": "remote@example.com" }))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let remote_port = listener.local_addr()?.port();
    tokio::spawn(async move { axum::serve(listener, remote).await });
    sqlx::query!(
        "UPDATE workspace_settings SET remote_deploy_target = $1
         WHERE workspace_id = 'test-workspace'",
        json!({ "base_url": format!("http://127.0.0.1:{remote_port}"), "workspace_id": "prod" })
    )
    .execute(&db)
    .await?;

    let connect = tokio::spawn(
        client()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/remote_deploy/connect"
            ))
            .header("Authorization", "Bearer SECRET_TOKEN_2")
            .json(&json!({
                "token": "remote-token",
                "target": { "base_url": format!("http://127.0.0.1:{remote_port}"), "workspace_id": "prod" }
            }))
            .send(),
    );
    tokio::time::timeout(std::time::Duration::from_secs(30), reached_rx).await??;
    let resp = authed(client().delete(format!(
        "http://localhost:{port}/api/w/test-workspace/users/delete/test-user-2"
    )))
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    release_tx.send(()).unwrap();
    // Refused for no longer being a member, rather than failing before it got this far.
    let resp = connect.await??;
    assert_eq!(resp.status(), 400);
    assert!(resp.text().await?.contains("Only a member"));

    let left = sqlx::query_scalar!(
        "SELECT count(*) FROM remote_deploy_token WHERE email = 'test2@windmill.dev'"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(left, Some(0));
    Ok(())
}
