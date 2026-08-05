//! A dev workspace may itself be paired with one (a "dev of a dev"). The guards that keep that
//! shape well-formed are what these tests pin: a chain must stay acyclic, and no two dev workspaces
//! in it may carry the same environment label — they inherit the same git-sync repositories, so an
//! equal label means both deploy to one branch.

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";

async fn attach(port: u16, prod: &str, body: serde_json::Value) -> (reqwest::StatusCode, String) {
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/{prod}/workspaces/attach_dev_workspace"
        ))
        .header("Authorization", format!("Bearer {ADMIN_TOKEN}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = resp.status();
    (status, resp.text().await.unwrap())
}

async fn detach(port: u16, prod: &str, dev: &str) -> (reqwest::StatusCode, String) {
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/{prod}/workspaces/detach_dev_workspace"
        ))
        .header("Authorization", format!("Bearer {ADMIN_TOKEN}"))
        .json(&json!({ "dev_workspace_id": dev }))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    (status, resp.text().await.unwrap())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "nested_dev_workspace"))]
async fn test_nested_dev_workspace_attach_guards(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // The family root is an ancestor of the dev workspace: reparenting it below would close a
    // parent<->child cycle and hang every hierarchy walk.
    let (status, body) = attach(
        port,
        "tw-dev",
        json!({ "dev_workspace_id": "test-workspace", "dev_workspace_label": "staging" }),
    )
    .await;
    assert!(
        status.is_client_error(),
        "cycle attach returned {status}: {body}"
    );
    assert!(body.contains("ancestor"), "unexpected error: {body}");

    // `tw-dev` is itself the 'dev' workspace of the root, so a dev nested under it cannot be one too.
    let (status, body) = attach(
        port,
        "tw-dev",
        json!({ "dev_workspace_id": "spare", "dev_workspace_label": "dev" }),
    )
    .await;
    assert!(
        status.is_client_error(),
        "label reuse returned {status}: {body}"
    );
    assert!(body.contains("tw-dev"), "unexpected error: {body}");

    // `standalone` brings its own 'dev'-labelled dev workspace into the chain, which collides with
    // `tw-dev` whatever label the candidate itself is given.
    let (status, body) = attach(
        port,
        "tw-dev",
        json!({ "dev_workspace_id": "standalone", "dev_workspace_label": "staging" }),
    )
    .await;
    assert!(
        status.is_client_error(),
        "subtree label reuse returned {status}: {body}"
    );
    assert!(body.contains("standalone-dev"), "unexpected error: {body}");

    // With a free label and nothing conflicting underneath, the nested pairing goes through.
    let (status, body) = attach(
        port,
        "tw-dev",
        json!({ "dev_workspace_id": "spare", "dev_workspace_label": "staging" }),
    )
    .await;
    assert!(
        status.is_success(),
        "nested attach returned {status}: {body}"
    );
    // Runtime-checked (not `query!`): a macro here would need its own `.sqlx` entry, which
    // `cargo sqlx prepare --workspace` does not produce for test targets.
    let (parent, is_dev, label): (Option<String>, bool, Option<String>) = sqlx::query_as(
        "SELECT parent_workspace_id, is_dev_workspace, dev_workspace_label FROM workspace WHERE id = 'spare'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(parent.as_deref(), Some("tw-dev"));
    assert!(is_dev);
    assert_eq!(label.as_deref(), Some("staging"));

    Ok(())
}

async fn archive(port: u16, w_id: &str) -> (reqwest::StatusCode, String) {
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/{w_id}/workspaces/archive"
        ))
        .header("Authorization", format!("Bearer {ADMIN_TOKEN}"))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    (status, resp.text().await.unwrap())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "nested_dev_workspace"))]
async fn test_teardown_refuses_to_strand_a_nested_dev(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // A `wm-fork-` workspace keeps its parent when it stops being a dev workspace, so it returns to
    // being a throwaway fork — which hosts no pairing, leaving `redev-dev` attached with no way to
    // reach it. Detach and archive both clear the flag, so both have to refuse.
    let (status, body) = detach(port, "prod-b", "wm-fork-redev").await;
    assert!(
        status.is_client_error(),
        "stranding detach returned {status}: {body}"
    );
    assert!(body.contains("redev-dev"), "unexpected error: {body}");

    let (status, body) = archive(port, "wm-fork-redev").await;
    assert!(
        status.is_client_error(),
        "stranding archive returned {status}: {body}"
    );
    assert!(body.contains("redev-dev"), "unexpected error: {body}");

    // Archive soft-deletes whatever the id looks like, so a prefix-less dev workspace strands its
    // own dev too — even though detaching that same workspace is fine (it returns to standalone).
    let (status, body) = archive(port, "c-dev").await;
    assert!(
        status.is_client_error(),
        "prefix-less stranding archive returned {status}: {body}"
    );
    assert!(body.contains("c-dev-dev"), "unexpected error: {body}");
    let (status, body) = detach(port, "prod-c", "c-dev").await;
    assert!(
        status.is_success(),
        "prefix-less detach returned {status}: {body}"
    );

    // Detaching from the bottom up is the supported order.
    let (status, body) = detach(port, "wm-fork-redev", "redev-dev").await;
    assert!(status.is_success(), "leaf detach returned {status}: {body}");
    let (status, body) = detach(port, "prod-b", "wm-fork-redev").await;
    assert!(
        status.is_success(),
        "detach after cleanup returned {status}: {body}"
    );

    Ok(())
}
