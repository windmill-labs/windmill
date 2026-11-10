//! Deleting an MCP OAuth access token revokes its refresh grant, and only that one; an access
//! token removed by the expiry sweep keeps its grant refreshable, until its user is disabled.
#![cfg(feature = "mcp")]

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const VERIFIER: &str = "e2e-pkce-verifier-0123456789-0123456789-abcdef";
const CHALLENGE: &str = "q9x8Sam6B6iSaQRwyLQdPrK3Ob2wfH1sNsUhYehHwh8";
const REDIRECT: &str = "http://localhost:9/cb";

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_deleted_mcp_token_cannot_refresh(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start_mcp(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());
    let http = reqwest::Client::new();
    let (http, base) = (&http, &base);

    // Connects a fresh client as the bearer's user; returns (client_id, access, refresh).
    let connect = |bearer: &'static str| async move {
        let client_id = http
            .post(format!("{base}/mcp/oauth/server/register"))
            .json(&json!({ "client_name": "test", "redirect_uris": [REDIRECT] }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?["client_id"]
            .as_str()
            .unwrap()
            .to_string();
        let code = http
            .post(format!("{base}/mcp/gateway/oauth/server/approve"))
            .bearer_auth(bearer)
            .json(&json!({
                "client_id": client_id, "redirect_uri": REDIRECT, "scope": "mcp:all", "state": "",
                "code_challenge": CHALLENGE, "code_challenge_method": "S256", "all_workspaces": true,
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?["code"]
            .as_str()
            .unwrap()
            .to_string();
        let issued = http
            .post(format!("{base}/mcp/gateway/oauth/server/token"))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("client_id", &client_id),
                ("redirect_uri", REDIRECT),
                ("code_verifier", VERIFIER),
            ])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        anyhow::Ok((
            client_id,
            issued["access_token"].as_str().unwrap().to_string(),
            issued["refresh_token"].as_str().unwrap().to_string(),
        ))
    };
    let refresh = |client_id: &str, refresh_token: &str| {
        http.post(format!("{base}/mcp/gateway/oauth/server/token"))
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", client_id),
            ])
            .send()
    };
    let expire_and_sweep = |access: String| {
        let db = db.clone();
        async move {
            sqlx::query(
                "UPDATE token SET expiration = now() - interval '1 second'
                 WHERE token_hash = encode(sha256($1::bytea), 'hex')",
            )
            .bind(access.as_bytes())
            .execute(&db)
            .await?;
            sqlx::query("DELETE FROM token WHERE expiration <= now()")
                .execute(&db)
                .await?;
            anyhow::Ok(())
        }
    };

    let (deleted_client, deleted_access, deleted_refresh) = connect("SECRET_TOKEN").await?;
    let (idle_client, idle_access, idle_refresh) = connect("SECRET_TOKEN_2").await?;

    let resp = http
        .delete(format!(
            "{base}/users/tokens/delete/{}",
            &deleted_access[..10]
        ))
        .bearer_auth("SECRET_TOKEN")
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let resp = refresh(&deleted_client, &deleted_refresh).await?;
    assert_eq!(resp.status(), 400);
    assert_eq!(
        resp.json::<serde_json::Value>().await?["error"],
        "invalid_grant"
    );

    expire_and_sweep(idle_access).await?;
    let resp = refresh(&idle_client, &idle_refresh).await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let rotated = resp.json::<serde_json::Value>().await?;

    expire_and_sweep(rotated["access_token"].as_str().unwrap().to_string()).await?;
    let resp = http
        .post(format!("{base}/users/update/test2@windmill.dev"))
        .bearer_auth("SECRET_TOKEN")
        .json(&json!({ "disabled": true }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let resp = refresh(&idle_client, rotated["refresh_token"].as_str().unwrap()).await?;
    assert_eq!(
        resp.status(),
        400,
        "a disabled user's idle client still refreshes"
    );

    Ok(())
}
