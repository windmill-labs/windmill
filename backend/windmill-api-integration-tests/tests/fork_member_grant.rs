use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

/// `test2` created the fork `wm-fork-test` but is only a developer in it.
const FORK_OWNER_TOKEN: &str = "SECRET_TOKEN_2";
/// `test3` is a developer of the parent and of the fork, but created neither.
const FORK_MEMBER_TOKEN: &str = "SECRET_TOKEN_3";

fn as_user(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

async fn add_user(
    port: u16,
    w_id: &str,
    token: &str,
    body: serde_json::Value,
) -> reqwest::Response {
    as_user(
        client().post(format!(
            "http://localhost:{port}/api/w/{w_id}/workspaces/add_user"
        )),
        token,
    )
    .json(&body)
    .send()
    .await
    .unwrap()
}

fn developer(email: &str) -> serde_json::Value {
    json!({ "email": email, "is_admin": false, "operator": false })
}

/// The creator of a fork may manage developers on it without being an admin of it, and may do
/// nothing beyond that. A fork clones its parent wholesale (secrets included), so each of these
/// bounds is what keeps the grant from becoming a way for any developer to widen access to the
/// parent's data or to mint an admin.
#[sqlx::test(migrations = "../migrations", fixtures("base", "fork_member_grant"))]
async fn test_fork_creator_can_only_manage_developers_on_their_fork(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // The creator adds a developer of the parent as a developer of their fork.
    let resp = add_user(
        port,
        "wm-fork-test",
        FORK_OWNER_TOKEN,
        developer("test3@windmill.dev"),
    )
    .await;
    assert_eq!(resp.status(), 201, "fork creator can add a developer");

    // ... but never as an admin.
    let resp = add_user(
        port,
        "wm-fork-test",
        FORK_OWNER_TOKEN,
        json!({ "email": "test4@windmill.dev", "is_admin": true, "operator": false }),
    )
    .await;
    assert_eq!(resp.status(), 403, "fork creator cannot add an admin");

    // ... nor anyone who is only an operator of the parent, which would widen their access.
    let resp = add_user(
        port,
        "wm-fork-test",
        FORK_OWNER_TOKEN,
        developer("test4@windmill.dev"),
    )
    .await;
    assert_eq!(
        resp.status(),
        403,
        "fork creator cannot add an operator of the parent"
    );

    // ... nor anyone from outside the parent workspace.
    let resp = add_user(
        port,
        "wm-fork-test",
        FORK_OWNER_TOKEN,
        developer("outsider@windmill.dev"),
    )
    .await;
    assert_eq!(
        resp.status(),
        403,
        "fork creator cannot add a non-member of the parent"
    );

    // The grant covers the fork alone, not the workspace it was forked from.
    let resp = add_user(
        port,
        "test-workspace",
        FORK_OWNER_TOKEN,
        developer("test4@windmill.dev"),
    )
    .await;
    assert_eq!(
        resp.status(),
        403,
        "fork creator gains nothing on the parent workspace"
    );

    // ... and belongs to the creator, not to every member of the fork.
    let resp = add_user(
        port,
        "wm-fork-test",
        FORK_MEMBER_TOKEN,
        developer("test4@windmill.dev"),
    )
    .await;
    assert_eq!(
        resp.status(),
        403,
        "a fork member who did not create it gains nothing"
    );

    // Removing is the counterpart of adding: allowed for the developer they just added...
    let resp = as_user(
        client().delete(format!(
            "http://localhost:{port}/api/w/wm-fork-test/users/delete/test-user-3"
        )),
        FORK_OWNER_TOKEN,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200, "fork creator can remove a developer");

    // ... but not for an admin of the fork.
    let resp = as_user(
        client().delete(format!(
            "http://localhost:{port}/api/w/wm-fork-test/users/delete/test-user"
        )),
        FORK_OWNER_TOKEN,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(
        resp.status(),
        403,
        "fork creator cannot remove an admin of the fork"
    );

    Ok(())
}
