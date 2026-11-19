//! The by-hash script reads must honor a path-scoped token: the route layer only checks
//! the scope domain, so each handler checks `scripts:read:<path>` once it knows the path.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn test_by_hash_reads_enforce_token_path_scope(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    for (path, hash) in [
        ("f/a/in_scope", 7_200_001_i64),
        ("f/b/out_of_scope", 7_200_002_i64),
    ] {
        sqlx::query(
            "INSERT INTO script(workspace_id, created_by, content, schema, summary, description, path, hash, language)
             VALUES ('test-workspace', 'test-user', 'export function main() {}', '{}', '', '', $1, $2, 'bun')",
        )
        .bind(path)
        .bind(hash)
        .execute(&db)
        .await?;
    }
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, scopes)
         VALUES (encode(sha256('SCOPED_TOKEN'::bytea), 'hex'), 'SCOPED_TOK', 'SCOPED_TOKEN',
                 'test@windmill.dev', 'scoped', ARRAY['scripts:read:f/a/*'])",
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace/scripts",
        server.addr.port()
    );
    let client = reqwest::Client::new();
    let status = |url: String| {
        let client = client.clone();
        async move {
            anyhow::Ok(
                client
                    .get(url)
                    .bearer_auth("SCOPED_TOKEN")
                    .send()
                    .await?
                    .status()
                    .as_u16(),
            )
        }
    };

    let in_hash = format!("{:016x}", 7_200_001_i64);
    let out_hash = format!("{:016x}", 7_200_002_i64);
    for endpoint in ["raw/h/{}.ts", "deployment_status/h/{}"] {
        let url = |h: &str| format!("{base}/{}", endpoint.replace("{}", h));
        assert_eq!(status(url(&in_hash)).await?, 200, "{endpoint} in scope");
        assert_eq!(
            status(url(&out_hash)).await?,
            403,
            "{endpoint} out of scope"
        );
    }
    Ok(())
}
