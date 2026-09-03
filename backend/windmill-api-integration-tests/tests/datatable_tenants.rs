use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

/// Who the data table's one role currently lets run as it.
async fn tenants(db: &Pool<Postgres>) -> Vec<String> {
    let value: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT datatable->'datatables'->'main'->'permissions'->'roles'->'analyst'->'tenants'
         FROM workspace_settings WHERE workspace_id = 'test-workspace'",
    )
    .fetch_one(db)
    .await
    .unwrap();
    serde_json::from_value(value.unwrap()).unwrap()
}

/// A tenant is a name, and a name outlives the principal that held it: whoever
/// takes it next would run as the role it still names. Every route that frees
/// one has to take it off the role, which is easy to miss from any single one of
/// them — so they are pinned together.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn freeing_a_principal_takes_its_datatable_tenant(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    sqlx::query(
        r#"UPDATE workspace_settings SET datatable = $1 WHERE workspace_id = 'test-workspace'"#,
    )
    .bind(json!({
        "datatables": {
            "main": {
                "database": { "resource_type": "instance", "resource_path": "dt_main" },
                "permissions": { "enabled": true, "roles": {
                    "admin": { "tenants": [] },
                    "analyst": { "tenants": [
                        "*", "u/test-user-2", "u/test-user-3", "g/leaving_group", "f/leaving_folder"
                    ]}
                }}
            }
        }
    }))
    .execute(&db)
    .await?;

    for (endpoint, body) in [
        ("groups/create", json!({ "name": "leaving_group" })),
        ("folders/create", json!({ "name": "leaving_folder" })),
    ] {
        let resp = authed(client().post(format!("{ws}/{endpoint}")), "SECRET_TOKEN")
            .json(&body)
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "{endpoint}: {}", resp.text().await?);
    }

    let resp = authed(
        client().delete(format!("{ws}/groups/delete/leaving_group")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "delete group: {}", resp.text().await?);
    assert!(!tenants(&db).await.contains(&"g/leaving_group".to_string()));

    let resp = authed(
        client().delete(format!("{ws}/folders/delete/leaving_folder")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "delete folder: {}", resp.text().await?);
    assert!(!tenants(&db).await.contains(&"f/leaving_folder".to_string()));

    // Leaving frees the username as surely as an admin removing the member does.
    let resp = authed(client().post(format!("{ws}/users/leave")), "SECRET_TOKEN_2")
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "leave: {}", resp.text().await?);
    assert!(!tenants(&db).await.contains(&"u/test-user-2".to_string()));

    let resp = authed(
        client().delete(format!(
            "http://localhost:{port}/api/users/delete/test3@windmill.dev"
        )),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "global delete: {}", resp.text().await?);
    assert!(!tenants(&db).await.contains(&"u/test-user-3".to_string()));

    // What no deletion named is left alone — the wildcard above all, which is
    // not a principal and cannot be freed.
    assert_eq!(tenants(&db).await, vec!["*".to_string()]);

    Ok(())
}
