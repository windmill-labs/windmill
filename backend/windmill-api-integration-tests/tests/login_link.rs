use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn login_link_is_single_use_and_same_origin(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api");
    let mint = |token: &'static str, body: serde_json::Value| {
        client()
            .post(format!("{base}/users/login_links"))
            .header("Authorization", format!("Bearer {token}"))
            .json(&body)
            .send()
    };

    // Only a superadmin mints.
    let resp = mint("SECRET_TOKEN_2", json!({"email": "test2@windmill.dev"})).await?;
    assert_eq!(resp.status(), 401);

    // A superadmin account is never a valid target: the minting credential must not
    // become an instance-wide role.
    let resp = mint("SECRET_TOKEN", json!({"email": "test@windmill.dev"})).await?;
    assert_eq!(resp.status(), 400);

    // An off-origin destination is refused before anything is minted.
    let resp = mint(
        "SECRET_TOKEN",
        json!({"email": "test2@windmill.dev", "rd": "https://evil.example/"}),
    )
    .await?;
    assert_eq!(resp.status(), 400);

    let resp = mint(
        "SECRET_TOKEN",
        json!({"email": "test2@windmill.dev", "rd": "/user/workspaces?x=1"}),
    )
    .await?;
    assert_eq!(resp.status(), 201);
    let link = resp.json::<serde_json::Value>().await?;
    let path = link["url"]
        .as_str()
        .unwrap()
        .split_once("/api")
        .unwrap()
        .1
        .to_string();
    let consume_url = format!("{base}{path}");

    // A promotion inside the link's window is re-checked at open time: no session,
    // and the link is not spent while the account is privileged.
    sqlx::query("UPDATE password SET super_admin = true WHERE email = 'test2@windmill.dev'")
        .execute(&db)
        .await?;
    let resp = client().get(&consume_url).send().await?;
    assert_eq!(resp.status(), 302);
    assert_eq!(
        resp.headers()["location"],
        "/user/login_link_expired?reason=invalid"
    );
    assert!(resp.headers().get("set-cookie").is_none());
    sqlx::query("UPDATE password SET super_admin = false WHERE email = 'test2@windmill.dev'")
        .execute(&db)
        .await?;

    // First open: session cookie for the target account, redirected to the stored rd.
    let resp = client().get(&consume_url).send().await?;
    assert_eq!(resp.status(), 302);
    assert_eq!(resp.headers()["location"], "/user/workspaces?x=1");
    assert_eq!(resp.headers()["referrer-policy"], "no-referrer");
    let cookie = resp
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|c| c.to_str().unwrap().to_string())
        .find(|c| c.starts_with("token="))
        .expect("session cookie");
    assert!(cookie.contains("HttpOnly"));
    let session = cookie
        .split(';')
        .next()
        .unwrap()
        .trim_start_matches("token=")
        .to_string();
    let resp = client()
        .get(format!("{base}/users/whoami"))
        .header("Authorization", format!("Bearer {session}"))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?["email"],
        "test2@windmill.dev"
    );

    // Second open: burned, no cookie, bounced to the explanation page.
    let resp = client().get(&consume_url).send().await?;
    assert_eq!(resp.status(), 302);
    assert_eq!(
        resp.headers()["location"],
        "/user/login_link_expired?reason=used"
    );
    assert!(resp.headers().get("set-cookie").is_none());

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn login_link_mint_can_require_a_login_type(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api");
    let mint = || {
        client()
            .post(format!("{base}/users/login_links"))
            .header("Authorization", "Bearer SECRET_TOKEN")
            .json(&json!({"email": "test2@windmill.dev", "require_login_type": "pending_oauth"}))
            .send()
    };

    // A password account is not the account the caller created: no link.
    let resp = mint().await?;
    assert_eq!(resp.status(), 409);
    assert!(resp.text().await?.contains("login_type_mismatch"));

    sqlx::query!(
        "UPDATE password SET login_type = 'pending_oauth', password_hash = NULL WHERE email = 'test2@windmill.dev'"
    )
    .execute(&db)
    .await?;
    let resp = mint().await?;
    assert_eq!(resp.status(), 201);
    Ok(())
}
