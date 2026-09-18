#![cfg(feature = "private")]

use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// Each table is limited on its own before the union is paginated, so a page past the first must
/// still see every row ranked ahead of it in both tables.
#[sqlx::test(fixtures("base"))]
async fn test_list_audit_paginates_across_legacy_and_partitioned(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    for (table, id) in [
        ("audit", 1),
        ("audit", 2),
        ("audit", 3),
        ("audit_partitioned", 101),
        ("audit_partitioned", 102),
        ("audit_partitioned", 103),
    ] {
        sqlx::query(&format!(
            "INSERT INTO {table} (workspace_id, id, username, operation, action_kind)
             VALUES ('test-workspace', $1, 'test-user', 'test.op', 'execute')"
        ))
        .bind(id as i64)
        .execute(&db)
        .await?;
    }

    let mut ids = vec![];
    for page in 1..=4 {
        let response = client
            .client()
            .get(format!(
                "{}/w/test-workspace/audit/list?operation=test.op&per_page=2&page={page}",
                client.baseurl()
            ))
            .send()
            .await?;
        assert!(
            response.status().is_success(),
            "page {page}: {}",
            response.text().await?
        );
        let rows = response.json::<Vec<serde_json::Value>>().await?;
        ids.extend(rows.iter().map(|r| r["id"].as_i64().unwrap()));
    }
    assert_eq!(ids, vec![103, 102, 101, 3, 2, 1]);

    Ok(())
}
