//! Deleting an MCP OAuth access token revokes its refresh grant, and only that one; an access
//! token removed by the expiry sweep keeps its grant refreshable.
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

    // Connects a fresh client for the same user; returns (client_id, access, refresh).
    let connect = || async {
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
            .bearer_auth("SECRET_TOKEN")
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
    let refresh = |client_id: String, refresh_token: String| {
        http.post(format!("{base}/mcp/gateway/oauth/server/token"))
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &refresh_token),
                ("client_id", &client_id),
            ])
            .send()
    };

    let (deleted_client, deleted_access, deleted_refresh) = connect().await?;
    let (other_client, other_access, other_refresh) = connect().await?;

    let resp = http
        .delete(format!(
            "{base}/users/tokens/delete/{}",
            &deleted_access[..10]
        ))
        .bearer_auth("SECRET_TOKEN")
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let resp = refresh(deleted_client, deleted_refresh).await?;
    assert_eq!(resp.status(), 400);
    assert_eq!(
        resp.json::<serde_json::Value>().await?["error"],
        "invalid_grant"
    );

    sqlx::query(
        "UPDATE token SET expiration = now() - interval '1 second'
         WHERE token_hash = encode(sha256($1::bytea), 'hex')",
    )
    .bind(other_access.as_bytes())
    .execute(&db)
    .await?;
    sqlx::query("DELETE FROM token WHERE expiration <= now()")
        .execute(&db)
        .await?;
    let resp = refresh(other_client, other_refresh).await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    Ok(())
}
