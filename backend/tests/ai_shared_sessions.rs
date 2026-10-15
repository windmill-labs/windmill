//! Shared AI sessions: a private share is served only to members of its workspace, a public
//! one to anyone holding the link, and only its author decides which it is.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const MEMBER: &str = "Bearer SECRET_TOKEN_2";
const OTHER_MEMBER: &str = "Bearer SECRET_TOKEN_3";
const OUTSIDER: &str = "Bearer OUTSIDER_TOKEN";

#[sqlx::test(fixtures("base"))]
async fn private_share_is_for_members_and_public_share_for_anyone(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::raw_sql(
        "INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
             VALUES ('outsider@windmill.dev', 'not-a-real-hash', 'password', false, true, 'Outsider');
         INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
             VALUES (encode(sha256('OUTSIDER_TOKEN'::bytea), 'hex'), 'OUTSIDER_T', 'OUTSIDER_TOKEN',
                     'outsider@windmill.dev', 'outsider token', false);",
    )
    .execute(&db)
    .await?;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let authed = format!("http://localhost:{port}/api/w/test-workspace/ai/shared_sessions");
    let get = |id: &str| {
        format!("http://localhost:{port}/api/w/test-workspace/ai_u/shared_sessions/get/{id}")
    };
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{authed}/share"))
        .header("Authorization", MEMBER)
        .json(&json!({
            "session_id": "session-1",
            "name": "Debugging",
            "message_count": 1,
            "snapshot": { "messages": [{ "role": "user", "content": "hi" }], "artifacts": [] },
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let share: Value = resp.json().await?;
    assert_eq!(share["is_public"], false, "a new share must start private");
    let id = share["id"].as_str().unwrap().to_string();

    let status = |token: Option<&'static str>| {
        let mut req = client.get(get(&id));
        if let Some(token) = token {
            req = req.header("Authorization", token);
        }
        async move { anyhow::Ok(req.send().await?.status().as_u16()) }
    };

    assert_eq!(status(Some(OTHER_MEMBER)).await?, 200);
    assert_eq!(
        status(None).await?,
        401,
        "a private share was served anonymously"
    );
    assert_eq!(
        status(Some(OUTSIDER)).await?,
        401,
        "a private share was served to a non-member"
    );

    let resp = client
        .post(format!("{authed}/set_public/{id}"))
        .header("Authorization", OTHER_MEMBER)
        .json(&json!({ "is_public": true }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        404,
        "a member who is not the author made the share public"
    );

    let resp = client
        .post(format!("{authed}/set_public/{id}"))
        .header("Authorization", MEMBER)
        .json(&json!({ "is_public": true }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let resp = client.get(get(&id)).send().await?;
    assert_eq!(
        resp.status(),
        200,
        "a public share was not served anonymously"
    );
    let body: Value = resp.json().await?;
    assert_eq!(body["snapshot"]["messages"][0]["content"], "hi");
    assert_eq!(body["can_unshare"], false);

    Ok(())
}
