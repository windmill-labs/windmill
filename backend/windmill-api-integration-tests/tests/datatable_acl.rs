//! Who may read and change a data table's grants and owners. On the Enterprise Edition: its
//! administrators, from the workspace that governs it. Without it: nobody. Each refusal is decided
//! before anything connects to the data table, so the fixture's database never has to exist.

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
#[cfg(all(feature = "private", feature = "enterprise"))]
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

#[cfg(all(feature = "private", feature = "enterprise"))]
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

/// Not even reading, and not even on a data table that is not under roles — which any member
/// reaches, so only the edition stands between them and the instance's credentials.
#[cfg(not(all(feature = "private", feature = "enterprise")))]
#[sqlx::test(migrations = "../migrations", fixtures("base", "datatable_roles"))]
async fn only_the_enterprise_edition_has_the_access_editor(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::query(
        "UPDATE workspace_settings
         SET datatable = datatable #- '{datatables,main,permissions}'
         WHERE workspace_id = 'test-workspace'",
    )
    .execute(&db)
    .await?;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let read = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/workspaces/datatable_acl/main?kind=database"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?;
    let mut responses = vec![("read", read)];
    for action in ["plan", "apply"] {
        responses.push((
            action,
            post_acl(port, "test-workspace", action, "SECRET_TOKEN").await?,
        ));
    }
    for (action, resp) in responses {
        assert_eq!(resp.status(), 400, "{action}");
        let body = resp.text().await?;
        assert!(
            body.contains("Data table roles are a Windmill Enterprise Edition feature"),
            "{action}: {body}"
        );
    }
    Ok(())
}
