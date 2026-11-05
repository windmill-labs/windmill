//! An app run reads the parsed path, so a spelling that parses to a pinned key must get that pin.

use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_test_utils::*;

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_viewer_pin_survives_another_spelling(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let api = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    let http = reqwest::Client::new();

    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema,
             summary, description, lock, extra_perms)
         VALUES ('test-workspace', 1, 'u/test-user/s', 'x', 'bun', 'script', 'test-user', '{}', '',
             '', '', '{}')",
    )
    .execute(&db)
    .await?;

    let pin = |msg: &str| json!({"static_inputs": {"msg": msg}, "one_of_inputs": {}});
    for (name, mode, pins, expected) in [
        (
            "no_component",
            "viewer",
            json!({"script/u/test-user/s": pin("pinned")}),
            Some("pinned"),
        ),
        (
            "component",
            "viewer",
            json!({"a:script/u/test-user/s": pin("pinned")}),
            Some("pinned"),
        ),
        (
            "exact_first",
            "viewer",
            json!({
                "a:script//u/test-user/s": pin("exact"),
                "a:script/u/test-user/s": pin("parsed"),
            }),
            Some("exact"),
        ),
        (
            "publisher",
            "publisher",
            json!({"a:script/u/test-user/s": pin("pinned")}),
            None,
        ),
    ] {
        let path = format!("u/test-user/{name}");
        let resp = http
            .post(format!("{api}/apps/create"))
            .bearer_auth("SECRET_TOKEN")
            .json(&json!({
                "path": path,
                "summary": "",
                "value": {},
                "policy": {"execution_mode": mode, "triggerables_v2": pins},
            }))
            .send()
            .await?;
        assert_eq!(resp.status(), 201, "{name}: {}", resp.text().await?);

        let resp = http
            .post(format!("{api}/apps_u/execute_component/{path}"))
            .bearer_auth("SECRET_TOKEN_2")
            .json(&json!({
                "component": "a",
                "path": "script//u/test-user/s",
                "args": {"msg": "caller"},
            }))
            .send()
            .await?;
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        let Some(expected) = expected else {
            assert_eq!(status, 400, "{name}: {body}");
            assert!(body.contains("forbidden by policy"), "{name}: {body}");
            continue;
        };
        assert_eq!(status, 200, "{name}: {body}");
        let msg: Option<String> =
            sqlx::query_scalar("SELECT args->>'msg' FROM v2_job WHERE id = $1")
                .bind(Uuid::parse_str(&body)?)
                .fetch_one(&db)
                .await?;
        assert_eq!(msg.as_deref(), Some(expected), "{name}");
    }
    Ok(())
}
