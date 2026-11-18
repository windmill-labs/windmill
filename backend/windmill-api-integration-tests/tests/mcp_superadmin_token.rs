//! A superadmin's MCP token carries their superadmin rights (every workspace) only when asked,
//! and the OAuth consent path and the token-create path agree on it.
#![cfg(feature = "mcp")]

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const VERIFIER: &str = "e2e-pkce-verifier-0123456789-0123456789-abcdef";
const CHALLENGE: &str = "q9x8Sam6B6iSaQRwyLQdPrK3Ob2wfH1sNsUhYehHwh8";
const REDIRECT: &str = "http://localhost:9/cb";

async fn token_is_super_admin(db: &Pool<Postgres>, token: &str) -> anyhow::Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT super_admin FROM token WHERE token_hash = encode(sha256($1::bytea), 'hex')",
    )
    .bind(token.as_bytes())
    .fetch_one(db)
    .await?)
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_mcp_token_super_admin_is_opt_in(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start_mcp(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());
    let http = reqwest::Client::new();

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
    let approve = |bearer: &str, super_admin: bool| {
        http.post(format!("{base}/mcp/gateway/oauth/server/approve"))
            .bearer_auth(bearer)
            .json(&json!({
                "client_id": client_id, "redirect_uri": REDIRECT, "scope": "mcp:all", "state": "",
                "code_challenge": CHALLENGE, "code_challenge_method": "S256",
                "all_workspaces": true, "super_admin": super_admin,
            }))
            .send()
    };

    for super_admin in [false, true] {
        let code = approve("SECRET_TOKEN", super_admin)
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
        let oauth_token = issued["access_token"].as_str().expect(&issued.to_string());
        assert_eq!(token_is_super_admin(&db, oauth_token).await?, super_admin);

        let refreshed = http
            .post(format!("{base}/mcp/gateway/oauth/server/token"))
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", issued["refresh_token"].as_str().unwrap()),
                ("client_id", &client_id),
            ])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let refreshed = refreshed["access_token"].as_str().unwrap();
        assert_eq!(token_is_super_admin(&db, refreshed).await?, super_admin);

        let user_token = http
            .post(format!("{base}/users/tokens/create"))
            .bearer_auth("SECRET_TOKEN")
            .json(&json!({ "label": "mcp", "scopes": ["mcp:all"], "super_admin": super_admin }))
            .send()
            .await?
            .text()
            .await?;
        assert_eq!(token_is_super_admin(&db, &user_token).await?, super_admin);
    }

    assert_eq!(
        approve("SECRET_TOKEN_2", true).await?.status(),
        401,
        "only a superadmin may grant superadmin rights"
    );

    let opted_out = http
        .post(format!("{base}/users/tokens/create"))
        .bearer_auth("SECRET_TOKEN")
        .json(&json!({ "label": "mcp", "scopes": ["mcp:all"] }))
        .send()
        .await?
        .text()
        .await?;
    assert_eq!(
        approve(&opted_out, true).await?.status(),
        401,
        "an MCP token without superadmin access cannot approve one that has it"
    );

    http.post(format!("{base}/users/update/test@windmill.dev"))
        .bearer_auth("SECRET_TOKEN")
        .json(&json!({ "is_super_admin": true }))
        .send()
        .await?
        .error_for_status()?;
    assert!(
        !token_is_super_admin(&db, &opted_out).await?,
        "a promotion must not hand superadmin access to an MCP token minted without it"
    );
    Ok(())
}
