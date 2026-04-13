//! Integration tests for fork deployment requests + comments.
//!
//! Covers: one-open-per-fork constraint, assignee ACL validation, anchor
//! obsolescence when an item in the fork changes, merge-close lifecycle,
//! replies, reply-to-reply rejection, and cancel ACL.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

#[sqlx::test(fixtures("fork_deployment_requests"))]
async fn test_fork_deployment_request_lifecycle(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let fork_base = format!("http://localhost:{port}/api/w/fork-ws");

    // ---- 1. eligible_deployers returns admin + deployer, not random ----
    let resp = authed(
        client().get(format!("{fork_base}/deployment_request/eligible_deployers")),
        "FDR_OWNER_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let deployers: Vec<Value> = resp.json().await?;
    let usernames: Vec<&str> = deployers
        .iter()
        .filter_map(|d| d.get("username").and_then(|v| v.as_str()))
        .collect();
    assert!(usernames.contains(&"fdr-admin"));
    assert!(usernames.contains(&"fdr-deployer"));
    assert!(!usernames.contains(&"fdr-random"));

    // ---- 2. createDeploymentRequest with valid assignees succeeds ----
    let resp = authed(
        client().post(format!("{fork_base}/deployment_request")),
        "FDR_OWNER_TOKEN",
    )
    .json(&json!({ "assignees": ["fdr-admin", "fdr-deployer"] }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "create: {}",
        resp.text().await.unwrap_or_default()
    );
    let created: Value = resp.json().await?;
    let request_id = created.get("id").and_then(|v| v.as_i64()).unwrap();
    assert_eq!(
        created.get("requested_by").and_then(|v| v.as_str()),
        Some("fdr-owner")
    );
    assert_eq!(
        created
            .get("assignees")
            .and_then(|v| v.as_array())
            .map(|a| a.len()),
        Some(2)
    );

    // ---- 3. second create fails with 409 because one-open constraint ----
    let resp = authed(
        client().post(format!("{fork_base}/deployment_request")),
        "FDR_OWNER_TOKEN",
    )
    .json(&json!({ "assignees": ["fdr-admin"] }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        409,
        "second create should return 409 Conflict: {}",
        resp.status()
    );

    // ---- 4. getOpenRequest returns the request ----
    let resp = authed(
        client().get(format!("{fork_base}/deployment_request/open")),
        "FDR_RANDOM_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let open: Value = resp.json().await?;
    assert_eq!(open.get("id").and_then(|v| v.as_i64()), Some(request_id));

    // ---- 5. random user posts a general comment (anyone with fork access) ----
    let resp = authed(
        client().post(format!(
            "{fork_base}/deployment_request/{request_id}/comment"
        )),
        "FDR_RANDOM_TOKEN",
    )
    .json(&json!({ "body": "Looks good overall" }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "general comment: {}",
        resp.text().await.unwrap_or_default()
    );
    let top_comment: Value = resp.json().await?;
    let top_id = top_comment.get("id").and_then(|v| v.as_i64()).unwrap();

    // ---- 6. admin posts an anchored comment ----
    let resp = authed(
        client().post(format!(
            "{fork_base}/deployment_request/{request_id}/comment"
        )),
        "FDR_ADMIN_TOKEN",
    )
    .json(&json!({
        "body": "Can you revisit this script?",
        "anchor_kind": "script",
        "anchor_path": "f/shared/script1"
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "anchored comment: {}",
        resp.text().await.unwrap_or_default()
    );
    let anchored: Value = resp.json().await?;
    let anchored_id = anchored.get("id").and_then(|v| v.as_i64()).unwrap();
    assert_eq!(
        anchored.get("anchor_kind").and_then(|v| v.as_str()),
        Some("script")
    );
    assert_eq!(
        anchored.get("obsolete").and_then(|v| v.as_bool()),
        Some(false)
    );

    // ---- 7. reply to top-level comment ----
    let resp = authed(
        client().post(format!(
            "{fork_base}/deployment_request/{request_id}/comment"
        )),
        "FDR_OWNER_TOKEN",
    )
    .json(&json!({ "body": "Thanks!", "parent_id": top_id }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let reply: Value = resp.json().await?;
    let reply_id = reply.get("id").and_then(|v| v.as_i64()).unwrap();
    assert_eq!(
        reply.get("parent_id").and_then(|v| v.as_i64()),
        Some(top_id)
    );

    // ---- 7b. reply-to-reply is rejected (2-level max) ----
    let resp = authed(
        client().post(format!(
            "{fork_base}/deployment_request/{request_id}/comment"
        )),
        "FDR_OWNER_TOKEN",
    )
    .json(&json!({ "body": "nested!", "parent_id": reply_id }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        400,
        "reply-to-reply should be rejected: {}",
        resp.status()
    );

    // ---- 8. anchored comment becomes obsolete when item changes in fork ----
    // The CE stub of `mark_anchor_obsolete` is a no-op (the EE module fills
    // it in). Simulate the EE effect directly so the CE test can verify
    // the GET response renders obsolete comments correctly.
    sqlx::query!(
        r#"
            UPDATE workspace_fork_deployment_request_comment c
            SET obsolete = true
            FROM workspace_fork_deployment_request r
            WHERE c.request_id = r.id
              AND r.fork_workspace_id = 'fork-ws'
              AND r.closed_at IS NULL
              AND c.anchor_kind = 'script'
              AND c.anchor_path = 'f/shared/script1'
        "#
    )
    .execute(&db)
    .await?;
    let resp = authed(
        client().get(format!("{fork_base}/deployment_request/open")),
        "FDR_OWNER_TOKEN",
    )
    .send()
    .await?;
    let open: Value = resp.json().await?;
    let comments = open.get("comments").and_then(|v| v.as_array()).unwrap();
    let anchored_after = comments
        .iter()
        .find(|c| c.get("id").and_then(|v| v.as_i64()) == Some(anchored_id))
        .unwrap();
    assert_eq!(
        anchored_after.get("obsolete").and_then(|v| v.as_bool()),
        Some(true),
        "anchored comment should be obsolete"
    );
    let top_after = comments
        .iter()
        .find(|c| c.get("id").and_then(|v| v.as_i64()) == Some(top_id))
        .unwrap();
    assert_eq!(
        top_after.get("obsolete").and_then(|v| v.as_bool()),
        Some(false),
        "general comment should NOT be obsolete"
    );

    // ---- 9. non-requester non-admin cannot cancel ----
    let resp = authed(
        client().post(format!(
            "{fork_base}/deployment_request/{request_id}/cancel"
        )),
        "FDR_RANDOM_TOKEN",
    )
    .send()
    .await?;
    assert!(
        !resp.status().is_success(),
        "random cancel should fail: {}",
        resp.status()
    );

    // ---- 10. requester can cancel ----
    let resp = authed(
        client().post(format!(
            "{fork_base}/deployment_request/{request_id}/cancel"
        )),
        "FDR_OWNER_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "cancel: {}",
        resp.text().await.unwrap_or_default()
    );

    // ---- 11. after cancel a new request can be opened ----
    let resp = authed(
        client().post(format!("{fork_base}/deployment_request")),
        "FDR_OWNER_TOKEN",
    )
    .json(&json!({ "assignees": ["fdr-admin"] }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "re-create: {}",
        resp.text().await.unwrap_or_default()
    );
    let new_req: Value = resp.json().await?;
    let new_req_id = new_req.get("id").and_then(|v| v.as_i64()).unwrap();

    // ---- 12. ineligible assignee rejected ----
    authed(
        client().post(format!(
            "{fork_base}/deployment_request/{new_req_id}/cancel"
        )),
        "FDR_OWNER_TOKEN",
    )
    .send()
    .await?;
    let resp = authed(
        client().post(format!("{fork_base}/deployment_request")),
        "FDR_OWNER_TOKEN",
    )
    .json(&json!({ "assignees": ["fdr-random"] }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        400,
        "ineligible assignee should fail: {}",
        resp.status()
    );

    // ---- 13. merge-close: create a request, close as merged, comments flip ----
    let resp = authed(
        client().post(format!("{fork_base}/deployment_request")),
        "FDR_OWNER_TOKEN",
    )
    .json(&json!({ "assignees": ["fdr-admin"] }))
    .send()
    .await?;
    let final_req: Value = resp.json().await?;
    let final_id = final_req.get("id").and_then(|v| v.as_i64()).unwrap();

    authed(
        client().post(format!("{fork_base}/deployment_request/{final_id}/comment")),
        "FDR_ADMIN_TOKEN",
    )
    .json(&json!({ "body": "lgtm" }))
    .send()
    .await?;

    let resp = authed(
        client().post(format!(
            "{fork_base}/deployment_request/{final_id}/close_merged"
        )),
        "FDR_ADMIN_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200);

    // After close_merged, getOpenRequest returns null.
    let resp = authed(
        client().get(format!("{fork_base}/deployment_request/open")),
        "FDR_OWNER_TOKEN",
    )
    .send()
    .await?;
    let after_merge = resp.text().await?;
    assert!(
        after_merge == "null" || after_merge == "",
        "open should be null after merge-close, got: {}",
        after_merge
    );

    // And every comment on the closed request is now obsolete.
    let obsolete_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) as \"c!\" FROM workspace_fork_deployment_request_comment WHERE request_id = $1 AND obsolete = false",
        final_id,
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        obsolete_count, 0,
        "all comments on merged request should be obsolete"
    );

    Ok(())
}
