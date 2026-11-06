//! Only a workspace admin can delete a draft that belongs to another user.
//!
//! A discard through `drafts/update` is scoped to the caller's own row, so it
//! never reaches someone else's draft. `drafts/delete_for_user` is the admin
//! route for that, and it must stay closed to everyone else.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn test_only_admin_deletes_another_users_draft(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/drafts");
    let client = reqwest::Client::new();
    let path = "script/u/test-user-2/theirs";

    let save = |token: &'static str| {
        client
            .post(format!("{base}/update/{path}"))
            .header("Authorization", format!("Bearer {token}"))
            .json(&json!({ "value": { "path": "u/test-user-2/theirs", "content": "" } }))
            .send()
    };
    let resp = save("SECRET_TOKEN_2").await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let delete = |token: &'static str| {
        client
            .post(format!(
                "{base}/delete_for_user/{path}?username=test-user-2"
            ))
            .header("Authorization", format!("Bearer {token}"))
            .send()
    };
    let own_draft = || async {
        client
            .get(format!("{base}/get_own/{path}"))
            .header("Authorization", "Bearer SECRET_TOKEN_2")
            .send()
            .await?
            .json::<Value>()
            .await
    };

    let resp = delete("SECRET_TOKEN_3").await?;
    assert_eq!(resp.status(), 403, "{}", resp.text().await?);
    assert!(
        !own_draft().await?.is_null(),
        "a non-admin deleted the draft"
    );

    let resp = delete("SECRET_TOKEN").await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    assert!(own_draft().await?.is_null(), "the draft is still there");

    let resp = delete("SECRET_TOKEN").await?;
    assert_eq!(resp.status(), 404, "{}", resp.text().await?);

    // Without a username the delete takes every draft at the path.
    for token in ["SECRET_TOKEN_2", "SECRET_TOKEN"] {
        let resp = save(token).await?;
        assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    }
    let delete_all = |token: &'static str| {
        client
            .post(format!("{base}/delete_for_user/{path}"))
            .header("Authorization", format!("Bearer {token}"))
            .send()
    };
    let resp = delete_all("SECRET_TOKEN_3").await?;
    assert_eq!(resp.status(), 403, "{}", resp.text().await?);
    let resp = delete_all("SECRET_TOKEN").await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let left: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM draft WHERE workspace_id = 'test-workspace' AND path = 'u/test-user-2/theirs'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(left, 0, "drafts survived a path-wide delete");
    Ok(())
}

/// `others_only` is what the review page's "all drafts" view uses: it removes
/// every other user's draft at the path, keeps the admin's own, and also covers
/// the drawer kinds, whose drafts the username-targeted delete refuses.
#[sqlx::test(fixtures("base"))]
async fn test_admin_deletes_others_drafts_keeping_own(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/drafts");
    let client = reqwest::Client::new();
    let path = "variable/u/test-user-2/theirs";

    for token in ["SECRET_TOKEN_2", "SECRET_TOKEN"] {
        let resp = client
            .post(format!("{base}/update/{path}"))
            .header("Authorization", format!("Bearer {token}"))
            .json(&json!({ "value": { "variable": { "path": "u/test-user-2/theirs" } } }))
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    }

    let delete_others = |token: &'static str| {
        client
            .post(format!("{base}/delete_for_user/{path}?others_only=true"))
            .header("Authorization", format!("Bearer {token}"))
            .send()
    };
    let resp = delete_others("SECRET_TOKEN_3").await?;
    assert_eq!(resp.status(), 403, "{}", resp.text().await?);
    let resp = delete_others("SECRET_TOKEN").await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let owners: Vec<String> = sqlx::query_scalar(
        "SELECT email FROM draft WHERE workspace_id = 'test-workspace' AND path = 'u/test-user-2/theirs'",
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(
        owners,
        vec!["test@windmill.dev".to_string()],
        "only the admin's draft should remain"
    );

    let resp = delete_others("SECRET_TOKEN").await?;
    assert_eq!(resp.status(), 404, "{}", resp.text().await?);
    Ok(())
}
