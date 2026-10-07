use std::collections::HashMap;

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_api_client::types::{NewScript, ScriptLang};
use windmill_test_utils::init_client;

const PATH: &str = "u/test-user/inferred_schema";

fn bun_ns(content: &str, parent_hash: Option<String>) -> NewScript {
    NewScript {
        draft_only: None,
        content: content.into(),
        language: ScriptLang::Bun,
        lock: None,
        parent_hash,
        path: PATH.into(),
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
        description: "".to_string(),
        envs: vec![],
        is_template: None,
        kind: None,
        summary: "".to_string(),
        tag: None,
        // What this client sends for "no schema".
        schema: HashMap::new(),
        ws_error_handler_muted: Some(false),
        priority: None,
        delete_after_secs: None,
        timeout: None,
        restart_unless_cancelled: None,
        deployment_message: None,
        concurrency_key: None,
        visible_to_runner_only: None,
        auto_kind: None,
        codebase: None,
        has_preprocessor: None,
        on_behalf_of_email: None,
        assets: vec![],
        modules: None,
    }
}

async fn head(db: &Pool<Postgres>) -> anyhow::Result<(i64, Value)> {
    Ok(sqlx::query_as(
        "SELECT hash, schema::jsonb FROM script \
         WHERE workspace_id = 'test-workspace' AND path = $1 AND archived = false",
    )
    .bind(PATH)
    .fetch_one(db)
    .await?)
}

/// A deploy without a schema (MCP, a bare API call) gets the one the editor would
/// have sent, and a new version keeps what the user wrote on the previous one's.
#[sqlx::test(fixtures("base"))]
async fn schemaless_deploy_infers_schema_keeping_annotations(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let (client, _port, _s) = init_client(db.clone()).await;

    client
        .create_script(
            "test-workspace",
            &bun_ns(
                "export function main(n: number, name = \"x\") { return n }",
                None,
            ),
        )
        .await?;
    let (hash, schema) = head(&db).await?;
    assert_eq!(schema["required"], json!(["n"]));
    assert_eq!(schema["properties"]["n"]["type"], "number");
    assert_eq!(schema["properties"]["name"]["default"], "x");

    sqlx::query(
        "UPDATE script SET schema = jsonb_set(schema::jsonb, '{properties,n,description}', \
         '\"how many\"')::json WHERE hash = $1",
    )
    .bind(hash)
    .execute(&db)
    .await?;

    client
        .create_script(
            "test-workspace",
            &bun_ns(
                "export function main(n: number, flag: boolean) { return n }",
                Some(format!("{hash:016x}")),
            ),
        )
        .await?;
    let (hash, schema) = head(&db).await?;
    assert_eq!(schema["required"], json!(["n", "flag"]));
    assert_eq!(schema["properties"]["n"]["description"], "how many");
    assert_eq!(schema["properties"]["flag"]["type"], "boolean");
    assert!(schema["properties"].get("name").is_none());

    // Code that does not parse keeps the schema rather than losing it for good.
    client
        .create_script(
            "test-workspace",
            &bun_ns(
                "export function main(n: number {",
                Some(format!("{hash:016x}")),
            ),
        )
        .await?;
    assert_eq!(head(&db).await?.1, schema);

    Ok(())
}
