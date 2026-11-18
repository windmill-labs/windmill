//! Shared AI sessions: a private share is served only to members of its workspace, a public
//! one to anyone holding the link, and only its author decides which it is.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const ADMIN: &str = "Bearer SECRET_TOKEN";
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

    let delete = |token: &'static str| {
        client
            .delete(format!("{authed}/delete/{id}"))
            .header("Authorization", token)
            .send()
    };
    assert_eq!(
        delete(OTHER_MEMBER).await?.status(),
        404,
        "a member who is neither the author nor an admin took the share down"
    );
    assert_eq!(
        delete(ADMIN).await?.status(),
        200,
        "an admin could not take the share down"
    );
    assert_eq!(status(Some(OTHER_MEMBER)).await?, 404);

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn a_share_is_listed_for_a_member_only_once_they_open_it(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let authed = format!("http://localhost:{port}/api/w/test-workspace/ai/shared_sessions");
    let client = reqwest::Client::new();

    let share: Value = client
        .post(format!("{authed}/share"))
        .header("Authorization", MEMBER)
        .json(&json!({
            "session_id": "session-1",
            "name": "Debugging",
            "message_count": 1,
            "snapshot": { "messages": [], "artifacts": [] },
        }))
        .send()
        .await?
        .json()
        .await?;
    let id = share["id"].as_str().unwrap().to_string();

    let listed = |token: &'static str| {
        let req = client
            .get(format!("{authed}/list"))
            .header("Authorization", token);
        async move { anyhow::Ok(req.send().await?.json::<Vec<Value>>().await?) }
    };

    let mine = listed(MEMBER).await?;
    assert_eq!(mine.len(), 1, "the author does not see their own share");
    assert_eq!(mine[0]["session_id"], "session-1");
    assert!(
        listed(OTHER_MEMBER).await?.is_empty(),
        "a share was listed for a member who never opened its link"
    );

    let resp = client
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/ai_u/shared_sessions/get/{id}"
        ))
        .header("Authorization", OTHER_MEMBER)
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let opened = listed(OTHER_MEMBER).await?;
    assert_eq!(opened.len(), 1, "opening the link did not list the share");
    assert!(
        opened[0].get("session_id").is_none(),
        "the author's session id was sent to a reader"
    );
    assert!(opened[0]["opened_at"].is_string());

    let resp = client
        .delete(format!("{authed}/opened/{id}"))
        .header("Authorization", OTHER_MEMBER)
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    assert!(listed(OTHER_MEMBER).await?.is_empty());

    Ok(())
}
