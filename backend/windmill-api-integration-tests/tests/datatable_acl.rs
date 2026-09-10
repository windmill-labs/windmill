//! Who may change a data table's grants and owners: its administrators, from the workspace that
//! governs it, on an edition that has the planner. Each refusal is decided before anything
//! connects to the data table, so the fixture's database never has to exist.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn grant_select_on_public() -> Value {
    json!({
        "target": {"kind": "schema", "schema": "public"},
        "change": {"type": "grant", "role": "analytics", "privileges": ["SELECT"],
                   "scope": "all_tables"},
        "statements": [r#"GRANT SELECT ON ALL TABLES IN SCHEMA "public" TO "analytics""#]
    })
}

async fn post_acl(
    port: u16,
    w_id: &str,
    action: &str,
    token: &str,
) -> anyhow::Result<reqwest::Response> {
    Ok(reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/{w_id}/workspaces/datatable_acl/main/{action}"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&grant_select_on_public())
        .send()
        .await?)
}

/// A fork reaches the data table through a pointer: it may use it, never change what each role may
/// touch on it — not even as an admin of the fork.
#[sqlx::test(migrations = "../migrations", fixtures("base", "datatable_roles"))]
async fn a_fork_cannot_change_access_on_the_data_table_it_points_at(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    for action in ["plan", "apply"] {
        let resp = post_acl(port, "wm-fork-dt", action, "SECRET_TOKEN_2").await?;
        assert_eq!(resp.status(), 401, "{action}: {}", resp.text().await?);
    }
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "datatable_roles"))]
async fn a_member_who_is_not_an_admin_cannot_change_access(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    for action in ["plan", "apply"] {
        let resp = post_acl(port, "test-workspace", action, "SECRET_TOKEN_2").await?;
        assert_eq!(resp.status(), 401, "{action}: {}", resp.text().await?);
    }
    Ok(())
}

#[cfg(not(all(feature = "private", feature = "enterprise")))]
#[sqlx::test(migrations = "../migrations", fixtures("base", "datatable_roles"))]
async fn only_the_enterprise_edition_changes_access(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    for action in ["plan", "apply"] {
        let resp = post_acl(port, "test-workspace", action, "SECRET_TOKEN").await?;
        assert_eq!(resp.status(), 400, "{action}");
        let body = resp.text().await?;
        assert!(body.contains("Enterprise Edition"), "{action}: {body}");
    }
    Ok(())
}
