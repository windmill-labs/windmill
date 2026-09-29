use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres, Row};

use windmill_test_utils::*;

/// The fork copies version history in bulk, so the rows it creates are only tied back to their
/// source by the order and id mapping the statements build. Seeded out of id order (the newest
/// version first) so an aggregate ordered by id rather than `created_at` shows up here.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_fork_clones_version_history(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    sqlx::query(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, schema,
                           extra_perms, versions)
         VALUES ('test-workspace', 'u/test-user/f', '', '', '{\"modules\":[]}', 'test-user',
                 '{}', '{}', '{}')",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "WITH v AS (
            INSERT INTO flow_version (workspace_id, path, value, created_by, created_at)
            VALUES ('test-workspace', 'u/test-user/f', '{\"n\":2}', 'test-user', now()),
                   ('test-workspace', 'u/test-user/f', '{\"n\":1}', 'test-user', now() - interval '1 hour')
            RETURNING id, created_at
         )
         UPDATE flow SET versions = (SELECT array_agg(id ORDER BY created_at) FROM v)
         WHERE workspace_id = 'test-workspace' AND path = 'u/test-user/f'",
    )
    .execute(&db)
    .await?;

    let app_id: i64 = sqlx::query_scalar(
        "INSERT INTO app (workspace_id, path, summary, policy, versions)
         VALUES ('test-workspace', 'u/test-user/a', '', '{}', '{}') RETURNING id",
    )
    .fetch_one(&db)
    .await?;
    let (latest, older): (i64, i64) = {
        let ids: Vec<i64> = sqlx::query_scalar(
            "INSERT INTO app_version (app_id, value, created_by, created_at, raw_app)
             VALUES ($1, '{\"n\":2}', 'test-user', now(), true),
                    ($1, '{\"n\":1}', 'test-user', now() - interval '1 hour', true)
             RETURNING id",
        )
        .bind(app_id)
        .fetch_all(&db)
        .await?;
        (ids[0], ids[1])
    };
    sqlx::query("UPDATE app SET versions = ARRAY[$2, $3] WHERE id = $1")
        .bind(app_id)
        .bind(older)
        .bind(latest)
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO app_bundles (app_version_id, w_id, file_type, data)
         VALUES ($1, 'test-workspace', 'js', 'latest'), ($2, 'test-workspace', 'js', 'older')",
    )
    .bind(latest)
    .bind(older)
    .execute(&db)
    .await?;
    let code_sha256 = format!("{:x}", Sha256::digest(b"code"));
    sqlx::query(
        "INSERT INTO app_script (app, hash, lock, code, code_sha256)
         VALUES ($1, 'source-hash', 'the lock', 'code', $2)",
    )
    .bind(app_id)
    .bind(&code_sha256)
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{}/api/w/test-workspace/workspaces/create_fork",
            server.addr.port()
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "id": "wm-fork-versions", "name": "Fork" }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "creating the fork: {}",
        resp.text().await?
    );

    let flow_values: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT fv.value FROM flow f, unnest(f.versions) WITH ORDINALITY u(id, ord)
         JOIN flow_version fv ON fv.id = u.id AND fv.workspace_id = 'wm-fork-versions'
         WHERE f.workspace_id = 'wm-fork-versions' AND f.path = 'u/test-user/f' ORDER BY u.ord",
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(flow_values, vec![json!({"n": 1}), json!({"n": 2})]);

    let fork_app = sqlx::query(
        "SELECT id, versions FROM app WHERE workspace_id = 'wm-fork-versions' AND path = 'u/test-user/a'",
    )
    .fetch_one(&db)
    .await?;
    let fork_app_id: i64 = fork_app.get("id");
    let fork_versions: Vec<i64> = fork_app.get("versions");
    let app_values: Vec<String> = sqlx::query_scalar(
        "SELECT av.value::text FROM unnest($1::bigint[]) WITH ORDINALITY u(id, ord)
         JOIN app_version av ON av.id = u.id AND av.app_id = $2 ORDER BY u.ord",
    )
    .bind(&fork_versions)
    .bind(fork_app_id)
    .fetch_all(&db)
    .await?;
    assert_eq!(app_values, vec![r#"{"n":1}"#, r#"{"n":2}"#]);

    // Only the current version's bundle is reachable, so only it is carried over.
    let bundles: Vec<(i64, Vec<u8>)> = sqlx::query_as(
        "SELECT app_version_id, data FROM app_bundles WHERE w_id = 'wm-fork-versions'",
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(bundles, vec![(fork_versions[1], b"latest".to_vec())]);

    let mut hasher = Sha256::new();
    hasher.update(fork_app_id.to_be_bytes());
    hasher.update(hex::decode(&code_sha256)?);
    hasher.update(b"the lock");
    let hash: String = sqlx::query_scalar("SELECT hash FROM app_script WHERE app = $1")
        .bind(fork_app_id)
        .fetch_one(&db)
        .await?;
    assert_eq!(hash, hex::encode(hasher.finalize()));

    Ok(())
}
