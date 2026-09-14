//! The AI session backup routes (`/w/{w}/ai/sessions/*`): a browser pushes pieces of its
//! sessions into the workspace's object storage and pulls them back whole. Pinned against a
//! FilesystemStorage LFS so the test needs no object store, which also lets it read what
//! landed on disk: the objects must be ciphertext, since bucket credentials are shared far
//! more widely than a user's transcripts.
#![cfg(all(feature = "private", feature = "parquet"))]

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_common::utils::calculate_hash;
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

async fn configure_primary_lfs(db: &Pool<Postgres>, root_path: &str) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE workspace_settings SET large_file_storage = $1 WHERE workspace_id = $2",
        json!({
            "type": "FilesystemStorage",
            "root_path": root_path,
            "public_resource": null,
            "advanced_permissions": null
        }),
        "test-workspace"
    )
    .execute(db)
    .await?;
    Ok(())
}

async fn list(base: &str, token: &str) -> anyhow::Result<Value> {
    let resp = authed(client().get(format!("{base}/ai/sessions/list")), token)
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    Ok(resp.json().await?)
}

async fn pull(base: &str, token: &str, ids: &[&str]) -> anyhow::Result<Value> {
    let resp = authed(client().post(format!("{base}/ai/sessions/pull")), token)
        .json(&json!({ "ids": ids }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    Ok(resp.json().await?)
}

async fn push(base: &str, token: &str, body: Value) -> anyhow::Result<reqwest::Response> {
    Ok(
        authed(client().post(format!("{base}/ai/sessions/push")), token)
            .json(&body)
            .send()
            .await?,
    )
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Every file under the storage root, as bytes.
fn files_under(root: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut out = vec![];
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push((path.clone(), std::fs::read(&path).unwrap_or_default()));
            }
        }
    }
    out
}

#[sqlx::test(fixtures("base"))]
async fn test_backups_round_trip_encrypted_and_scoped_to_the_user(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );

    // No storage configured: the browser is told to stop trying.
    let listing = list(&base, "SECRET_TOKEN").await?;
    assert_eq!(listing["enabled"], false);
    assert_eq!(listing["sessions"], json!([]));

    let storage_dir = tempfile::tempdir()?;
    configure_primary_lfs(&db, &storage_dir.path().to_string_lossy()).await?;

    let head =
        json!({ "id": "s1", "workspace_id": "test-workspace", "createdAt": 1, "chatId": "c1" });
    let chat = json!({ "id": "c1", "sessionId": "s1", "title": "MARKER_PLAINTEXT_TITLE", "lastModified": 2,
                       "actualMessages": [], "displayMessages": [{"role": "user", "content": "hello"}] });
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{
                "id": "s1",
                "head": head,
                "chats": [{ "id": "c1", "record": chat }, { "id": "c2", "record": { "id": "c2" } }],
                "images": [{ "chat_id": "c1", "id": "img1", "data_url": "data:image/png;base64,AAAA" }],
                "artifacts": { "items": [], "versions": [] }
            }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let pushed: Value = resp.json().await?;
    assert_eq!(pushed["enabled"], true);
    assert_eq!(pushed["results"], json!([{ "id": "s1" }]));

    let listing = list(&base, "SECRET_TOKEN").await?;
    assert_eq!(listing["enabled"], true);
    assert_eq!(listing["sessions"][0]["id"], "s1");

    let pulled = pull(&base, "SECRET_TOKEN", &["s1", "never-pushed"]).await?;
    assert_eq!(pulled["deferred"], json!([]));
    let sessions = pulled["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1, "an id with no backup is simply absent");
    let s1 = &sessions[0];
    assert_eq!(s1["head"], head);
    let mut chats = s1["chats"].as_array().unwrap().clone();
    chats.sort_by_key(|c| c["id"].as_str().unwrap().to_string());
    assert_eq!(chats[0]["record"], chat);
    assert_eq!(chats[1]["id"], "c2");
    assert_eq!(
        s1["images"],
        json!([{ "chat_id": "c1", "id": "img1", "data_url": "data:image/png;base64,AAAA" }])
    );
    assert_eq!(s1["artifacts"], json!({ "items": [], "versions": [] }));

    // Nothing on disk carries the transcript in the clear.
    let files = files_under(storage_dir.path());
    assert!(
        files.len() >= 4,
        "expected the pushed objects on disk, got {files:?}"
    );
    for (path, bytes) in &files {
        let text = String::from_utf8_lossy(bytes);
        assert!(
            !text.contains("MARKER_PLAINTEXT_TITLE") && !text.contains("base64,AAAA"),
            "{} holds plaintext",
            path.display()
        );
    }
    let key_paths: Vec<String> = files
        .iter()
        .map(|(p, _)| {
            p.strip_prefix(storage_dir.path())
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert!(
        key_paths
            .iter()
            .all(|p| p.starts_with("windmill_ai_sessions/test-workspace/")
                && !p.contains("test@windmill.dev")),
        "keys carry the workspace and never the email: {key_paths:?}"
    );

    // Another member of the workspace sees none of it.
    let other = list(&base, "SECRET_TOKEN_2").await?;
    assert_eq!(other["enabled"], true);
    assert_eq!(other["sessions"], json!([]));
    let other = pull(&base, "SECRET_TOKEN_2", &["s1"]).await?;
    assert_eq!(other["sessions"], json!([]));

    // Nor after copying the first user's ciphertext under their own prefix, which anyone
    // holding the bucket credentials can do: the key is bound to the user, not the workspace.
    let users_root = storage_dir
        .path()
        .join("windmill_ai_sessions/test-workspace");
    let first = users_root.join(calculate_hash("test@windmill.dev"));
    let second = users_root.join(calculate_hash("test2@windmill.dev"));
    copy_dir(&first, &second)?;
    let other = pull(&base, "SECRET_TOKEN_2", &["s1"]).await?;
    assert_eq!(
        other["sessions"],
        json!([]),
        "relocated ciphertext must not decrypt for another user"
    );
    std::fs::remove_dir_all(&second)?;

    // Deleting a chat takes its images along; a head-only push leaves the rest in place.
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s1", "delete_chats": ["c1"] }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let pulled = pull(&base, "SECRET_TOKEN", &["s1"]).await?;
    let s1 = &pulled["sessions"][0];
    assert_eq!(s1["head"], head);
    assert_eq!(s1["chats"].as_array().unwrap().len(), 1);
    assert_eq!(s1["chats"][0]["id"], "c2");
    assert_eq!(s1["images"], json!([]));

    // Removal empties both prefixes.
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "removed": ["s1"] }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let listing = list(&base, "SECRET_TOKEN").await?;
    assert_eq!(listing["sessions"], json!([]));
    assert!(
        files_under(storage_dir.path()).is_empty(),
        "removal must leave no object behind"
    );

    Ok(())
}

#[sqlx::test(fixtures("base", "jobs_read_auth"))]
async fn test_backup_writes_are_refused_for_the_wrong_owner_token_or_id(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    let storage_dir = tempfile::tempdir()?;
    configure_primary_lfs(&db, &storage_dir.path().to_string_lossy()).await?;

    // A push prepared for another user must not land under the caller's prefix.
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test2@windmill.dev", "sessions": [{ "id": "s1", "head": { "id": "s1" } }] }),
    )
    .await?;
    assert_eq!(resp.status(), 409, "{}", resp.text().await?);

    // Ids are what the server builds keys from.
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "sessions": [{ "id": "../s1", "head": { "id": "../s1" } }] }),
    )
    .await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "removed": ["a/b"] }),
    )
    .await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);

    // Nested lists are bounded too: each entry is an object-store call.
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s1", "delete_chats": (0..1001).map(|i| format!("c{i}")).collect::<Vec<_>>() }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);

    // A pull body is a handful of ids; a large one is refused before it is parsed.
    let resp = authed(
        client().post(format!("{base}/ai/sessions/pull")),
        "SECRET_TOKEN",
    )
    .header("Content-Type", "application/json")
    .body(format!("{{\"ids\":[\"{}\"]}}", "a".repeat(100_000)))
    .send()
    .await?;
    assert_eq!(resp.status(), 413, "{}", resp.text().await?);

    // A scoped token (here `jobs:read`) is minted for something narrower than the user's
    // whole assistant history.
    let resp = authed(
        client().get(format!("{base}/ai/sessions/list")),
        "SCOPED_DENO_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 403, "{}", resp.text().await?);

    assert!(files_under(storage_dir.path()).is_empty());
    Ok(())
}
