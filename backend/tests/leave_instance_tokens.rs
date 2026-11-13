//! Leaving the instance ends every credential of the account, as deleting it does: a token kept
//! past the leave would authenticate as whoever later takes the address.

use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn test_leave_instance_deletes_tokens(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api/users", server.addr.port());
    let client = reqwest::Client::new();

    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin)
         VALUES (encode(sha256('OTHER_TOKEN_2'::bytea), 'hex'), 'OTHER_TOKE', 'OTHER_TOKEN_2',
                 'test2@windmill.dev', 'api', false)",
    )
    .execute(&db)
    .await?;

    let resp = client
        .post(format!("{base}/leave_instance"))
        .bearer_auth("SECRET_TOKEN_2")
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let left: i64 =
        sqlx::query_scalar("SELECT count(*) FROM token WHERE email = 'test2@windmill.dev'")
            .fetch_one(&db)
            .await?;
    assert_eq!(left, 0, "tokens of the account survived the leave");

    let whoami = |token: &'static str| {
        client
            .get(format!("{base}/whoami"))
            .bearer_auth(token)
            .send()
    };
    assert_eq!(whoami("OTHER_TOKEN_2").await?.status(), 401);
    assert_eq!(whoami("SECRET_TOKEN_3").await?.status(), 200);
    Ok(())
}
