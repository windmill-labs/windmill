use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

async fn deploy(port: u16, path: &str, content: &str) -> reqwest::Response {
    authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/create"
    )))
    .json(&json!({
        "path": path,
        "summary": "",
        "description": "",
        "content": content,
        "language": "deno",
        "schema": { "type": "object", "properties": {}, "required": [] }
    }))
    .send()
    .await
    .unwrap()
}

/// A `dbt://` relation is one graph node only while every side spells it the same
/// way, and three sides derive that spelling independently: the `// materialize`
/// target becomes an `asset.path`, a `// on` ref becomes a `script_trigger`, and
/// the deploy-time refusal joins the two. The unit tests on `sole_dbt_producer`
/// prove the predicate; only a deploy proves the handler feeds it the key the
/// table actually holds — so a canonicalization that drifted on one side would
/// pass those and split the node here.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_dbt_materialize_target_deploy_contract(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    sqlx::query!(
        r#"UPDATE workspace_settings
              SET dbt_warehouses = '{"main": {"resource_path": "u/test-user/wh"}}'::jsonb
            WHERE workspace_id = 'test-workspace'"#
    )
    .execute(&db)
    .await?;

    // Nothing generates warehouse DDL, so a managed target is refused rather than
    // degraded into the track-only mode it would silently become.
    let resp = deploy(
        port,
        "u/test-user/managed",
        "// materialize dbt://main/analytics/orders\nexport async function main() {}",
    )
    .await;
    assert_eq!(resp.status(), 400);
    assert!(resp.text().await?.contains("must be `manual`"));

    // The warehouse segment is the identity a dbt model keys on; a name the
    // workspace does not configure strands the write on an unreachable node.
    let resp = deploy(
        port,
        "u/test-user/unknown_wh",
        "// materialize manual dbt://nope/analytics/orders\nexport async function main() {}",
    )
    .await;
    assert_eq!(resp.status(), 400);
    assert!(resp.text().await?.contains("does not configure"));

    // Only the DuckDB executor runs `// data_test` probes, and it runs them around
    // a managed write — so a declarer in another language would deploy green with
    // its assertions silently never executed.
    let resp = deploy(
        port,
        "u/test-user/tested",
        "// materialize manual dbt://main/analytics/orders\n// data_test not_null id\nexport async function main() {}",
    )
    .await;
    assert_eq!(resp.status(), 400);
    assert!(resp
        .text()
        .await?
        .contains("`// data_test` is not supported"));

    // Both halves are held to the same relation: every producer is a whole
    // `<warehouse>/<schema>/<name>` under a configured warehouse, so a
    // subscription to anything else names something nothing can ever write.
    for (path, ref_, expected) in [
        (
            "u/test-user/partial_sub",
            "dbt://main/analytics",
            "not a whole warehouse relation",
        ),
        (
            "u/test-user/unknown_wh_sub",
            "dbt://nope/analytics/orders",
            "does not configure",
        ),
    ] {
        let resp = deploy(
            port,
            path,
            &format!("// on {ref_}\nexport async function main() {{}}"),
        )
        .await;
        assert_eq!(resp.status(), 400);
        assert!(resp.text().await?.contains(expected));
    }

    // Any language may declare the write — the DuckLake write engine is DuckDB's,
    // this declaration is not — and the target is canonicalized on the way into
    // `asset`, so a hand-written mixed-case spelling lands on the model's key.
    let resp = deploy(
        port,
        "u/test-user/ingest",
        "// materialize manual dbt://main/ANALYTICS/Orders\nexport async function main() {}",
    )
    .await;
    assert_eq!(resp.status(), 201);
    // The create response, not `{:x}` over the stored i64: `ScriptHash` decodes
    // hex and demands 8 bytes, while `LowerHex` drops leading zeros, so a hash
    // under 2^60 would 422 the rename below instead of reaching the refusal.
    let ingest_hash = resp.text().await?;
    let write = sqlx::query_scalar!(
        "SELECT path FROM asset WHERE workspace_id = 'test-workspace' AND kind = 'dbt' \
           AND usage_path = 'u/test-user/ingest' AND usage_access_type = 'w'"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(write, "main/analytics/orders");

    // That producer is native, so subscribing to what it writes is accepted — and
    // the `// on` ref has to canonicalize identically, or the row it stores names
    // a relation nothing produces.
    let resp = deploy(
        port,
        "u/test-user/consumer",
        "// on dbt://main/\"Analytics\"/\"Orders\"\nexport async function main() {}",
    )
    .await;
    assert_eq!(resp.status(), 201);
    let trigger_ref = sqlx::query_scalar!(
        "SELECT trigger_ref FROM script_trigger WHERE workspace_id = 'test-workspace' \
           AND runnable_path = 'u/test-user/consumer' AND trigger_kind = 'asset'"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(trigger_ref, "dbt://main/analytics/orders");

    // With dbt as the only producer the same subscription can never be woken — a
    // dbt run does not dispatch — so the deploy refuses it and names the project.
    sqlx::query!(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by,
                             language)
         VALUES ('test-workspace', 1, 'u/test-user/project', '', '', '', 'test-user', 'dbt')"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind)
         VALUES ('test-workspace', 'main/analytics/marts', 'dbt', 'w', 'u/test-user/project',
                 'script')"
    )
    .execute(&db)
    .await?;
    let resp = deploy(
        port,
        "u/test-user/mart_consumer",
        "// on dbt://main/analytics/MARTS\nexport async function main() {}",
    )
    .await;
    assert_eq!(resp.status(), 400);
    assert!(resp.text().await?.contains("u/test-user/project"));

    // A rename is the other half of that: the producer's write still sits at the
    // OLD path in the committed snapshot this deploy reads, while the same
    // transaction removes it — so it must not count as the producer that would
    // wake the subscription the rename adds.
    sqlx::query!(
        "INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind)
         VALUES ('test-workspace', 'main/analytics/orders', 'dbt', 'w', 'u/test-user/project',
                 'script')"
    )
    .execute(&db)
    .await?;
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/create"
    )))
    .json(&json!({
        "path": "u/test-user/ingest_renamed",
        "parent_hash": ingest_hash,
        "summary": "",
        "description": "",
        "content": "// on dbt://main/analytics/orders\nexport async function main() {}",
        "language": "deno",
        "schema": { "type": "object", "properties": {}, "required": [] }
    }))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 400);
    assert!(resp.text().await?.contains("u/test-user/project"));

    // Neither annotation is accepted on a dbt script: the graph ingest
    // republishes that path's asset and trigger rows wholesale, so either would
    // deploy something the dependency job then silently removes.
    for content in [
        "# materialize manual dbt://main/analytics/orders\nprofile:\n  warehouse: main\n",
        "# on dbt://main/analytics/orders\nprofile:\n  warehouse: main\n",
    ] {
        let resp = authed(client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/scripts/create"
        )))
        .json(&json!({
            "path": "u/test-user/dbt_project",
            "summary": "",
            "description": "",
            "content": content,
            "language": "dbt",
            "modules": { "dbt_project.yml": { "content": "name: p\n", "language": "dbt" } },
            "schema": { "type": "object", "properties": {}, "required": [] }
        }))
        .send()
        .await
        .unwrap();
        assert_eq!(resp.status(), 400);
        assert!(resp.text().await?.contains("a dbt script cannot"));
    }

    Ok(())
}
