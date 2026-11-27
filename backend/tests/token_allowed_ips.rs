//! A token restricted to `allowed_ips` works only from those networks, and cannot mint a
//! credential that works from anywhere else. Setting a list is EE-only.
#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn test_token_allowed_ips(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api/users", server.addr.port());
    let client = reqwest::Client::new();

    for (token, nets) in [
        ("RESTRICTED_TOKEN", "{10.9.9.0/24}"),
        ("LOOPBACK_TOKEN", "{127.0.0.1/32,::1/128}"),
    ] {
        sqlx::query(
            "INSERT INTO token (token_hash, token_prefix, email, label, super_admin, allowed_ips)
             VALUES (encode(sha256($1::bytea), 'hex'), substr($1, 1, 10), 'test@windmill.dev',
                     'ci', false, $2::cidr[])",
        )
        .bind(token)
        .bind(nets)
        .execute(&db)
        .await?;
    }

    // The test client connects from loopback, a trusted proxy by default, so
    // X-Forwarded-For stands in for a client on another network.
    let whoami = |token: &'static str, from: Option<&'static str>| {
        let req = client.get(format!("{base}/whoami")).bearer_auth(token);
        let req = match from {
            Some(ip) => req.header("X-Forwarded-For", ip),
            None => req,
        };
        req.send()
    };
    assert_eq!(whoami("RESTRICTED_TOKEN", None).await?.status(), 401);
    assert_eq!(
        whoami("RESTRICTED_TOKEN", Some("10.9.9.5")).await?.status(),
        200
    );
    // The identity is cached now; the address is still checked.
    assert_eq!(
        whoami("RESTRICTED_TOKEN", Some("8.8.8.8")).await?.status(),
        401
    );
    assert_eq!(whoami("LOOPBACK_TOKEN", None).await?.status(), 200);
    assert_eq!(
        whoami("LOOPBACK_TOKEN", Some("10.9.9.5")).await?.status(),
        401
    );

    let create = |body: serde_json::Value| {
        client
            .post(format!("{base}/tokens/create"))
            .bearer_auth("RESTRICTED_TOKEN")
            .header("X-Forwarded-For", "10.9.9.5")
            .json(&body)
            .send()
    };
    let resp = create(serde_json::json!({ "label": "child" })).await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    let inherited: Vec<String> =
        sqlx::query_scalar("SELECT unnest(allowed_ips)::text FROM token WHERE label = 'child'")
            .fetch_all(&db)
            .await?;
    assert_eq!(inherited, vec!["10.9.9.0/24"]);
    let resp = create(serde_json::json!({ "label": "wide", "allowed_ips": ["0.0.0.0/0"] })).await?;
    assert_eq!(resp.status(), 403);

    let resp = client
        .get(format!("{base}/refresh_token"))
        .bearer_auth("RESTRICTED_TOKEN")
        .header("X-Forwarded-For", "10.9.9.5")
        .send()
        .await?;
    assert_eq!(resp.status(), 401);
    Ok(())
}
