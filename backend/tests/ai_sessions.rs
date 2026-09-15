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

async fn rotate(base: &str, key: &str) -> anyhow::Result<()> {
    let resp = authed(
        client().post(format!("{base}/workspaces/encryption_key")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "new_key": key }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    Ok(())
}

/// The user's prefix on disk, `windmill_ai_sessions/{w_id}/g{generation}/{email hash}`,
/// under whichever generation is current.
fn user_root(storage_dir: &std::path::Path, email: &str) -> std::path::PathBuf {
    let workspace = storage_dir.join("windmill_ai_sessions/test-workspace");
    let hash = calculate_hash(email);
    std::fs::read_dir(&workspace)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join(&hash))
        .find(|path| path.exists())
        .expect("the user has backups under the current key")
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
                "whole": true,
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

    // A part more parts follow names its push, or the session would stay listed between
    // the parts: one that does not is refused before anything of it lands.
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s1", "partial": true, "delete_chats": ["c1"] }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);
    assert_eq!(
        list(&base, "SECRET_TOKEN").await?["sessions"][0]["id"],
        "s1"
    );
    assert_eq!(
        pull(&base, "SECRET_TOKEN", &["s1"]).await?["sessions"][0]["chats"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // A part with more of the session to follow lists nothing; the part that completes
    // the push does, newest first.
    let ids = |listing: &Value| -> Vec<String> {
        listing["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect()
    };
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s2", "whole": true, "push": "p2", "opens": true, "head": { "id": "s2", "workspace_id": "test-workspace", "createdAt": 2, "chatId": "c" }, "partial": true }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(ids(&list(&base, "SECRET_TOKEN").await?), vec!["s1"]);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s2", "whole": true, "push": "p2", "chats": [{ "id": "c", "record": { "id": "c" } }] }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(ids(&list(&base, "SECRET_TOKEN").await?), vec!["s2", "s1"]);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "removed": ["s2"] }),
    )
    .await?;
    assert_eq!(resp.status(), 200);

    // A session that outgrew one answer (three chats of 12 MB against the 32 MB budget)
    // comes in pages, each naming where the next picks up, and nothing is left out.
    let big = "y".repeat(12 * 1024 * 1024);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s4", "whole": true, "push": "p4", "opens": true, "head": { "id": "s4", "workspace_id": "test-workspace", "createdAt": 4, "chatId": "c1" }, "partial": true }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    for cid in ["c1", "c2", "c3"] {
        let resp = push(
            &base,
            "SECRET_TOKEN",
            json!({
                "owner": "test@windmill.dev",
                "sessions": [{ "id": "s4", "whole": true, "push": "p4", "chats": [{ "id": cid, "record": { "id": cid, "big": big } }], "partial": cid != "c3" }]
            }),
        )
        .await?;
        assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    }
    let mut pages = vec![];
    let mut resume = json!(null);
    loop {
        let body = if resume.is_null() {
            json!({ "ids": ["s4"] })
        } else {
            json!({ "ids": ["s4"], "resume": resume })
        };
        let resp = authed(
            client().post(format!("{base}/ai/sessions/pull")),
            "SECRET_TOKEN",
        )
        .json(&body)
        .send()
        .await?;
        assert_eq!(resp.status(), 200, "{}", resp.text().await?);
        let pulled: Value = resp.json().await?;
        let page = pulled["sessions"][0].clone();
        assert_eq!(page["id"], "s4");
        resume = page["next"].clone();
        pages.push(page);
        if resume.is_null() {
            break;
        }
        assert!(pages.len() < 5, "a paged pull must end");
    }
    assert!(pages.len() >= 2, "36 MB must not fit one answer");
    // Every page of an unchanged backup carries the same listing fingerprint; a chat added
    // to the session changes it, which is what tells a browser its pages do not belong
    // together any more.
    let listing = pages[0]["listing"].clone();
    assert!(listing.is_string());
    assert!(pages.iter().all(|p| p["listing"] == listing));
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s4", "chats": [{ "id": "c0", "record": { "id": "c0", "n": 1 } }] }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let pulled = pull(&base, "SECRET_TOKEN", &["s4"]).await?;
    assert_ne!(pulled["sessions"][0]["listing"], listing);
    // So does a chat rewritten at the same size: the fingerprint takes in the entity tag,
    // not only the size and a modification time the store may report coarsely.
    let listing = pulled["sessions"][0]["listing"].clone();
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s4", "chats": [{ "id": "c0", "record": { "id": "c0", "n": 2 } }] }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let pulled = pull(&base, "SECRET_TOKEN", &["s4"]).await?;
    assert_ne!(pulled["sessions"][0]["listing"], listing);
    let mut chat_ids: Vec<String> = pages
        .iter()
        .flat_map(|p| p["chats"].as_array().unwrap().iter())
        .map(|c| c["id"].as_str().unwrap().to_string())
        .collect();
    chat_ids.sort();
    assert_eq!(chat_ids, vec!["c1", "c2", "c3"]);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "removed": ["s4"] }),
    )
    .await?;
    assert_eq!(resp.status(), 200);

    // More objects than a page keeps listing metadata for, from a store that lists in no
    // order: the pages still carry every one of them, each once.
    let many = 5001;
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s5", "whole": true, "push": "p5", "opens": true, "head": { "id": "s5", "workspace_id": "test-workspace", "createdAt": 5, "chatId": "c00000" }, "partial": true }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    for start in (0..many).step_by(100) {
        let chats: Vec<Value> = (start..(start + 100).min(many))
            .map(|i| json!({ "id": format!("c{i:05}"), "record": { "id": format!("c{i:05}") } }))
            .collect();
        let resp = push(
            &base,
            "SECRET_TOKEN",
            json!({
                "owner": "test@windmill.dev",
                "sessions": [{ "id": "s5", "whole": true, "push": "p5", "chats": chats, "partial": start + 100 < many }]
            }),
        )
        .await?;
        assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    }
    let mut seen = std::collections::HashSet::new();
    let mut resume = json!(null);
    let mut pages = 0;
    loop {
        let body = if resume.is_null() {
            json!({ "ids": ["s5"] })
        } else {
            json!({ "ids": ["s5"], "resume": resume })
        };
        let resp = authed(
            client().post(format!("{base}/ai/sessions/pull")),
            "SECRET_TOKEN",
        )
        .json(&body)
        .send()
        .await?;
        assert_eq!(resp.status(), 200, "{}", resp.text().await?);
        let pulled: Value = resp.json().await?;
        let page = &pulled["sessions"][0];
        for c in page["chats"].as_array().unwrap() {
            assert!(
                seen.insert(c["id"].as_str().unwrap().to_string()),
                "a chat came twice"
            );
        }
        pages += 1;
        resume = page["next"].clone();
        if resume.is_null() {
            break;
        }
        assert!(pages < 5, "a paged pull must end");
    }
    assert_eq!(pages, 2);
    assert_eq!(seen.len(), many);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "removed": ["s5"] }),
    )
    .await?;
    assert_eq!(resp.status(), 200);

    // A head at exactly its cap round-trips: the ciphertext read back is a block larger.
    let mut big_head = json!({ "id": "s3", "workspace_id": "test-workspace", "createdAt": 3, "chatId": "c", "pad": "" });
    let pad = 1024 * 1024 - serde_json::to_string(&big_head)?.len();
    big_head["pad"] = json!("x".repeat(pad));
    assert_eq!(serde_json::to_string(&big_head)?.len(), 1024 * 1024);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "sessions": [{ "id": "s3", "whole": true, "head": big_head }] }),
    )
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let pulled = pull(&base, "SECRET_TOKEN", &["s3"]).await?;
    assert_eq!(pulled["sessions"][0]["head"], big_head);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "removed": ["s3"] }),
    )
    .await?;
    assert_eq!(resp.status(), 200);

    // An object larger than any push writes, planted with the bucket's credentials at a
    // predictable key, is not read.
    let planted =
        user_root(storage_dir.path(), "test@windmill.dev").join("sessions/planted/head.json");
    std::fs::create_dir_all(planted.parent().unwrap())?;
    std::fs::File::create(&planted)?.set_len(32 * 1024 * 1024 + 1)?;
    let pulled = pull(&base, "SECRET_TOKEN", &["planted"]).await?;
    assert_eq!(pulled["sessions"], json!([]));
    std::fs::remove_dir_all(planted.parent().unwrap())?;
    // Under a session that exists, a planted chat is skipped without buffering and without
    // the page ending before it, so the pull neither balloons nor loops.
    let planted_chat =
        user_root(storage_dir.path(), "test@windmill.dev").join("sessions/s1/chats/planted.json");
    std::fs::File::create(&planted_chat)?.set_len(32 * 1024 * 1024 + 1)?;
    let mut resume = json!(null);
    let mut pages = 0;
    let mut chat_ids = vec![];
    loop {
        let body = if resume.is_null() {
            json!({ "ids": ["s1"] })
        } else {
            json!({ "ids": ["s1"], "resume": resume })
        };
        let resp = authed(
            client().post(format!("{base}/ai/sessions/pull")),
            "SECRET_TOKEN",
        )
        .json(&body)
        .send()
        .await?;
        assert_eq!(resp.status(), 200, "{}", resp.text().await?);
        let pulled: Value = resp.json().await?;
        assert_eq!(pulled["sessions"][0]["head"], head);
        for c in pulled["sessions"][0]["chats"].as_array().unwrap() {
            chat_ids.push(c["id"].as_str().unwrap().to_string());
        }
        pages += 1;
        resume = pulled["sessions"][0]["next"].clone();
        if resume.is_null() {
            break;
        }
        assert!(pages < 5, "a planted object must not keep the pull going");
    }
    chat_ids.sort();
    assert_eq!(chat_ids, vec!["c1", "c2"]);
    std::fs::remove_file(&planted_chat)?;

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
    let first = user_root(storage_dir.path(), "test@windmill.dev");
    let second = first
        .parent()
        .unwrap()
        .join(calculate_hash("test2@windmill.dev"));
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

    // Rotating the workspace key moves the routes to a fresh generation's prefix and deletes
    // the older ones off the request rather than re-key anything; the answers name the new
    // generation (`backup_generation`), which is what makes every browser push its sessions
    // whole again, while `storage_id` names the storage and stays.
    let before = list(&base, "SECRET_TOKEN").await?;
    rotate(&base, &"b".repeat(64)).await?;
    for _ in 0..100 {
        if files_under(storage_dir.path()).is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(
        files_under(storage_dir.path()).is_empty(),
        "a rotation must leave no backup object behind"
    );
    let listing = list(&base, "SECRET_TOKEN").await?;
    assert_eq!(listing["sessions"], json!([]));
    assert_eq!(listing["storage_id"], before["storage_id"]);
    assert_ne!(
        listing["backup_generation"], before["backup_generation"],
        "a rotation must bump the backup generation"
    );
    assert_eq!(
        pull(&base, "SECRET_TOKEN", &["s1"]).await?["sessions"],
        json!([])
    );
    // The browser's next push fills the storage back under the new key.
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s1", "whole": true, "head": head, "chats": [{ "id": "c2", "record": { "id": "c2" } }] }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let pulled = pull(&base, "SECRET_TOKEN", &["s1"]).await?;
    assert_eq!(pulled["sessions"][0]["head"], head);
    assert_eq!(pulled["sessions"][0]["chats"][0]["id"], "c2");
    // Setting the key already in place is not a rotation the browsers would notice, so it
    // keeps the backups.
    let same = list(&base, "SECRET_TOKEN").await?;
    rotate(&base, &"b".repeat(64)).await?;
    let listing = list(&base, "SECRET_TOKEN").await?;
    assert_eq!(listing["storage_id"], same["storage_id"]);
    assert_eq!(listing["backup_generation"], same["backup_generation"]);
    assert_eq!(listing["sessions"][0]["id"], "s1");
    assert_eq!(
        pull(&base, "SECRET_TOKEN", &["s1"]).await?["sessions"][0]["head"],
        head
    );

    // A push that does not open the session whole rides on the head in the storage; with
    // none there (another device removed the backup, or nothing was ever pushed) it is
    // refused and lists nothing, head or no head on it, until the session goes whole.
    let s6_head =
        json!({ "id": "s6", "workspace_id": "test-workspace", "createdAt": 6, "chatId": "c" });
    let not_listed = |listing: Value| {
        listing["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["id"] != "s6")
    };
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s6", "chats": [{ "id": "c", "record": { "id": "c" } }] }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let answer: Value = resp.json().await?;
    assert_eq!(answer["results"][0]["needs_whole"], true);
    assert!(not_listed(list(&base, "SECRET_TOKEN").await?));
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s6", "whole": true, "head": s6_head, "chats": [{ "id": "c", "record": { "id": "c" } }] }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let answer: Value = resp.json().await?;
    assert!(answer["results"][0]["needs_whole"].is_null());
    assert_eq!(
        list(&base, "SECRET_TOKEN").await?["sessions"][0]["id"],
        "s6"
    );
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "removed": ["s6"] }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s6", "head": s6_head, "chats": [{ "id": "c2", "record": { "id": "c2" } }] }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let answer: Value = resp.json().await?;
    assert_eq!(answer["results"][0]["needs_whole"], true);
    assert!(not_listed(list(&base, "SECRET_TOKEN").await?));
    assert!(
        files_under(&user_root(storage_dir.path(), "test@windmill.dev").join("sessions/s6"))
            .is_empty()
    );

    // Between the parts of a whole push (head landed, marker not yet), an incremental push
    // from another device is refused too: it rides on a listed session, and there is none
    // until the last part, which lists it.
    let s8_head =
        json!({ "id": "s8", "workspace_id": "test-workspace", "createdAt": 8, "chatId": "c1" });
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s8", "whole": true, "push": "p8", "opens": true, "head": s8_head, "partial": true }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s8", "chats": [{ "id": "c9", "record": { "id": "c9" } }] }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let answer: Value = resp.json().await?;
    assert_eq!(answer["results"][0]["needs_whole"], true);
    let listed = |listing: Value| {
        listing["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == "s8")
    };
    assert!(!listed(list(&base, "SECRET_TOKEN").await?));
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s8", "whole": true, "push": "p8", "chats": [{ "id": "c1", "record": { "id": "c1" } }] }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    assert!(listed(list(&base, "SECRET_TOKEN").await?));
    assert_eq!(
        pull(&base, "SECRET_TOKEN", &["s8"]).await?["sessions"][0]["chats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap().to_string())
            .collect::<Vec<_>>(),
        vec!["c1"]
    );
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "removed": ["s8"] }),
    )
    .await?;
    assert_eq!(resp.status(), 200);

    // A whole push replaces the backup: what the storage held of the session and the new
    // push does not carry (a chat deleted while the workspace was on another storage) goes.
    let s9_head =
        json!({ "id": "s9", "workspace_id": "test-workspace", "createdAt": 9, "chatId": "c1" });
    let s9_chats = |ids: &[&str]| -> Vec<Value> {
        ids.iter()
            .map(|c| json!({ "id": c, "record": { "id": c } }))
            .collect()
    };
    let pulled_chats = |pulled: Value| -> Vec<String> {
        pulled["sessions"][0]["chats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap().to_string())
            .collect()
    };
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s9", "whole": true, "head": s9_head, "chats": s9_chats(&["c1", "c2"]) }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        pulled_chats(pull(&base, "SECRET_TOKEN", &["s9"]).await?),
        vec!["c1", "c2"]
    );
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s9", "whole": true, "epoch": 1, "head": s9_head, "chats": s9_chats(&["c1"]) }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        pulled_chats(pull(&base, "SECRET_TOKEN", &["s9"]).await?),
        vec!["c1"]
    );
    // The marker carries the move count the push named, once; an incremental push at
    // another count rides on nothing, one at the same count lands.
    let s9_epochs = |listing: Value| -> Vec<Value> {
        listing["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["id"] == "s9")
            .map(|s| s["epoch"].clone())
            .collect()
    };
    assert_eq!(
        s9_epochs(list(&base, "SECRET_TOKEN").await?),
        vec![json!(1)]
    );
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s9", "chats": s9_chats(&["c7"]) }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let answer: Value = resp.json().await?;
    assert_eq!(answer["results"][0]["needs_whole"], true);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s9", "epoch": 1, "chats": s9_chats(&["c7"]) }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        pulled_chats(pull(&base, "SECRET_TOKEN", &["s9"]).await?),
        vec!["c1", "c7"]
    );

    // An incremental push split over parts unlists the session while it is in progress (a
    // pull between two parts would take a mix of old and new pieces for the backup) and
    // lists it again with the last part; while one is in progress or abandoned, a push that
    // is not part of it is refused, so the browser's next push of the session goes whole.
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s9", "epoch": 1, "push": "i1", "opens": true, "chats": s9_chats(&["c8"]), "partial": true }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        s9_epochs(list(&base, "SECRET_TOKEN").await?),
        Vec::<Value>::new()
    );
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s9", "epoch": 1, "chats": s9_chats(&["c11"]) }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let answer: Value = resp.json().await?;
    assert_eq!(answer["results"][0]["needs_whole"], true);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s9", "epoch": 1, "push": "i1", "chats": s9_chats(&["c9"]) }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        s9_epochs(list(&base, "SECRET_TOKEN").await?),
        vec![json!(1)]
    );
    assert_eq!(
        pulled_chats(pull(&base, "SECRET_TOKEN", &["s9"]).await?),
        vec!["c1", "c7", "c8", "c9"]
    );
    assert_eq!(
        s9_epochs(list(&base, "SECRET_TOKEN").await?),
        vec![json!(1)]
    );

    // Two devices pushing the session whole at once: the push that opened later replaced
    // the earlier one's pieces, so the earlier one's last part is refused and lists nothing,
    // and the session is listed with the later push's pieces alone.
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s9", "whole": true, "push": "t3", "opens": true, "head": s9_head, "chats": s9_chats(&["c3"]), "partial": true }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s9", "whole": true, "push": "t4", "opens": true, "head": s9_head, "chats": s9_chats(&["c4"]), "partial": true }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s9", "whole": true, "push": "t3", "chats": s9_chats(&["c5"]) }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    let answer: Value = resp.json().await?;
    assert_eq!(answer["results"][0]["needs_whole"], true);
    assert!(list(&base, "SECRET_TOKEN").await?["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["id"] != "s9"));
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s9", "whole": true, "push": "t4", "chats": s9_chats(&["c6"]) }]
        }),
    )
    .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        pulled_chats(pull(&base, "SECRET_TOKEN", &["s9"]).await?),
        vec!["c4", "c6"]
    );
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "removed": ["s9"] }),
    )
    .await?;
    assert_eq!(resp.status(), 200);

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
        listing["storage_id"].is_string(),
        "an answer names its storage: {listing}"
    );
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

    // A whole push opens with its head; one without is refused before anything of it lands,
    // and nothing lists the session.
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "sessions": [{ "id": "s7", "whole": true, "chats": [{ "id": "c", "record": { "id": "c" } }] }] }),
    )
    .await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);
    assert!(list(&base, "SECRET_TOKEN").await?["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["id"] != "s7"));
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "removed": ["a/b"] }),
    )
    .await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);

    // An image is a base64 data URL, stored and served verbatim; anything JSON would have
    // to escape (and so inflate past the pull budget) is refused.
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({
            "owner": "test@windmill.dev",
            "sessions": [{ "id": "s1", "images": [{ "chat_id": "c1", "id": "i1", "data_url": "data:image/png;base64,\u{0001}\u{0001}\"" }] }]
        }),
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

    // ...and across the whole request, not only per entry.
    let sessions: Vec<Value> = (0..100)
        .map(|i| {
            json!({ "id": format!("s{i}"), "delete_chats": (0..50).map(|j| format!("c{j}")).collect::<Vec<_>>() })
        })
        .collect();
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "sessions": sessions }),
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

/// Sets the object's modification time `days` back: the FilesystemStorage answers
/// `last_modified` from it, so this is a session no push touched since.
fn age_object(path: &std::path::Path, days: u64) -> std::io::Result<()> {
    let at = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86_400);
    std::fs::File::options()
        .write(true)
        .open(path)?
        .set_modified(at)
}

#[sqlx::test(fixtures("base"))]
async fn test_expired_backups_are_swept_by_age_and_left_out_of_the_listing(
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

    let chat = |sid: &str, cid: &str| {
        json!({ "id": cid, "record": { "id": cid, "sessionId": sid, "lastModified": 2,
                                        "actualMessages": [], "displayMessages": [] } })
    };
    let whole = |sid: &str| {
        json!({
            "id": sid, "whole": true, "epoch": 0,
            "head": { "id": sid, "workspace_id": "test-workspace", "createdAt": 1, "chatId": "c1" },
            "chats": [chat(sid, "c1")],
            "images": [{ "chat_id": "c1", "id": "img1", "data_url": "data:image/png;base64,AAAA" }],
            "artifacts": { "items": [], "versions": [] }
        })
    };
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev", "sessions": [whole("old"), whole("live")] }),
    )
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let root = user_root(storage_dir.path(), "test@windmill.dev");
    age_object(&root.join("index/old/0"), 40)?;

    let listed = |listing: Value| -> Vec<String> {
        let mut ids: Vec<String> = listing["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect();
        ids.sort();
        ids
    };
    let objects = |root: &std::path::Path| -> Vec<String> {
        files_under(root)
            .into_iter()
            .map(|(p, _)| p.strip_prefix(root).unwrap().to_string_lossy().into_owned())
            .collect()
    };

    // Without a retention nothing is swept, however old.
    windmill_api::sweep_expired_ai_session_backups(&db).await;
    assert_eq!(listed(list(&base, "SECRET_TOKEN").await?), ["live", "old"]);

    sqlx::query(
        "UPDATE workspace_settings SET ai_config = coalesce(ai_config, '{}'::jsonb) \
         || '{\"sessions_retention_days\": 30}' WHERE workspace_id = 'test-workspace'",
    )
    .execute(&db)
    .await?;

    // The listing leaves the expired session out before the sweep reaches it.
    assert_eq!(listed(list(&base, "SECRET_TOKEN").await?), ["live"]);
    assert!(root.join("sessions/old/head.json").exists());

    windmill_api::sweep_expired_ai_session_backups(&db).await;
    let remaining = objects(&root);
    assert!(
        remaining.iter().all(|p| !p.contains("/old/")),
        "{remaining:?}"
    );
    for kept in [
        "index/live/0",
        "sessions/live/head.json",
        "sessions/live/chats/c1.json",
        "images/live/c1/img1",
    ] {
        assert!(
            remaining.iter().any(|p| p == kept),
            "{kept} in {remaining:?}"
        );
    }
    assert_eq!(listed(list(&base, "SECRET_TOKEN").await?), ["live"]);

    // A second pass has nothing to do; a session pushed again since its marker aged is
    // renewed by the push, which rewrites the marker.
    windmill_api::sweep_expired_ai_session_backups(&db).await;
    age_object(&root.join("index/live/0"), 40)?;
    let resp = push(
        &base,
        "SECRET_TOKEN",
        json!({ "owner": "test@windmill.dev",
                "sessions": [{ "id": "live", "epoch": 0, "chats": [chat("live", "c2")] }] }),
    )
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    windmill_api::sweep_expired_ai_session_backups(&db).await;
    let mut after = objects(&root);
    after.sort();
    let mut expected = remaining.clone();
    expected.push("sessions/live/chats/c2.json".to_string());
    expected.sort();
    assert_eq!(after, expected);
    assert_eq!(listed(list(&base, "SECRET_TOKEN").await?), ["live"]);
    Ok(())
}
