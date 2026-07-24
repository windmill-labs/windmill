//! Regression tests for the merged runnables listing endpoint
//! (`GET /w/{workspace}/runnables/list`), which UNION-ALLs scripts, flows and
//! apps into one keyset-paginated, globally-ordered stream.
//!
//! The delicate part is the keyset cursor `(sort_key, path, kind, tiebreak)`.
//! These tests pin two properties a naive cursor would break:
//!   1. Cross-kind ties — a script and a flow sharing the exact same
//!      `(sort_key, path)` must each appear exactly once across pages, never
//!      duplicated or skipped, in every order.
//!   2. Archived versions — several archived script versions at one path share
//!      `(name, path, kind)`; the `hash` tiebreak keeps them individually
//!      reachable when a group crosses a page boundary.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn new_script(path: &str, summary: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "description": "",
        "content": "export async function main() {}",
        "language": "deno",
        "schema": { "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object", "properties": {}, "required": [] }
    })
}

fn new_flow(path: &str, summary: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "description": "",
        "value": { "modules": [] },
        "schema": { "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object", "properties": {}, "required": [] }
    })
}

/// Page through the endpoint one item at a time and return the ordered list of
/// `type:path` identifiers.
async fn paginate_all(port: u16, query: &str) -> Vec<String> {
    let base = format!("http://localhost:{port}/api/w/test-workspace/runnables/list");
    let mut out = vec![];
    let mut cursor: Option<String> = None;
    for _ in 0..50 {
        let mut url = format!("{base}?{query}&per_page=1");
        if let Some(c) = &cursor {
            url.push_str(&format!("&cursor={c}"));
        }
        let resp = authed(client().get(&url), "SECRET_TOKEN")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "list should succeed");
        let body: serde_json::Value = resp.json().await.unwrap();
        for it in body["items"].as_array().unwrap() {
            out.push(format!(
                "{}:{}",
                it["type"].as_str().unwrap(),
                it["path"].as_str().unwrap()
            ));
        }
        match body["next_cursor"].as_str() {
            Some(c) => cursor = Some(c.to_string()),
            None => break,
        }
    }
    out
}

fn assert_no_dupes(items: &[String], label: &str) {
    let mut sorted = items.to_vec();
    sorted.sort();
    let mut deduped = sorted.clone();
    deduped.dedup();
    assert_eq!(
        sorted, deduped,
        "{label}: keyset pagination must not duplicate or skip items, got {items:?}"
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_runnables_keyset_no_dupes_across_kind_ties(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // A script and a flow sharing a path, plus two more scripts.
    for (p, s) in [
        ("u/test-user/tie", "Tie item"),
        ("u/test-user/aaa", "Aaa"),
        ("u/test-user/zzz", "Zzz"),
    ] {
        let r = authed(
            client().post(format!("{base}/scripts/create")),
            "SECRET_TOKEN",
        )
        .json(&new_script(p, s))
        .send()
        .await?;
        assert_eq!(r.status(), 201, "create script: {}", r.text().await?);
    }
    let r = authed(
        client().post(format!("{base}/flows/create")),
        "SECRET_TOKEN",
    )
    .json(&new_flow("u/test-user/tie", "Tie item"))
    .send()
    .await?;
    assert_eq!(r.status(), 201, "create flow: {}", r.text().await?);

    // Force an exact (sort_time, path, summary) tie between the script and flow
    // at u/test-user/tie so the cross-kind boundary is actually exercised.
    let ts = "2026-07-20 10:00:00+00";
    sqlx::query(
        "UPDATE script SET created_at = $1::timestamptz WHERE workspace_id = 'test-workspace'",
    )
    .bind(ts)
    .execute(&db)
    .await?;
    sqlx::query(
        "UPDATE flow SET edited_at = $1::timestamptz WHERE workspace_id = 'test-workspace'",
    )
    .bind(ts)
    .execute(&db)
    .await?;

    // Expected: 3 scripts + 1 flow, each exactly once.
    for q in [
        "order_by=updated&order_desc=true",
        "order_by=name&order_desc=false",
        "order_by=name&order_desc=true",
    ] {
        let items = paginate_all(port, q).await;
        assert_eq!(items.len(), 4, "{q}: expected 4 items, got {items:?}");
        assert_no_dupes(&items, q);
        assert!(
            items.contains(&"flow:u/test-user/tie".to_string()),
            "{q}: flow present"
        );
        assert!(
            items.contains(&"script:u/test-user/tie".to_string()),
            "{q}: script present"
        );
    }
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_runnables_archived_versions_paginate_via_hash(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Three archived script versions at one path with identical summary — they
    // share (name, path, kind); only the `hash` tiebreak keeps them distinct.
    for (h, sec) in [(811111111i64, 0), (822222222, 1), (833333333, 2)] {
        sqlx::query(
            "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, created_at, language, archived)
             VALUES ('test-workspace', $1, 'u/test-user/av', 'AV', '', 'x', 'test-user', ('2026-07-19 10:00:0' || $2)::timestamptz, 'deno', true)",
        )
        .bind(h)
        .bind(sec.to_string())
        .execute(&db)
        .await?;
    }

    // Favorite the path: a path-based favorite marks *every* archived version
    // starred. The archived view must not pin/cap starred (that would drop
    // versions past the cap from every page), so all three must still page once.
    sqlx::query(
        "INSERT INTO favorite (usr, workspace_id, path, favorite_kind) VALUES ('test-user', 'test-workspace', 'u/test-user/av', 'script')",
    )
    .execute(&db)
    .await?;

    // paginate_all pages one at a time, so every version crosses a page boundary.
    let items = paginate_all(port, "order_by=name&order_desc=false&show_archived=true").await;
    let av = items
        .iter()
        .filter(|i| *i == "script:u/test-user/av")
        .count();
    assert_eq!(
        av, 3,
        "all three favorited archived versions must page exactly once, got {items:?}"
    );
    Ok(())
}

fn new_app(path: &str, summary: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "value": {},
        "policy": { "execution_mode": "publisher", "triggerables": {} }
    })
}

/// A single list request; returns the ordered `type:path` identifiers.
async fn list_once(port: u16, query: &str) -> Vec<String> {
    let url = format!("http://localhost:{port}/api/w/test-workspace/runnables/list?{query}");
    let resp = authed(client().get(&url), "SECRET_TOKEN")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "list should succeed for {query}");
    let body: serde_json::Value = resp.json().await.unwrap();
    body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|it| {
            format!(
                "{}:{}",
                it["type"].as_str().unwrap(),
                it["path"].as_str().unwrap()
            )
        })
        .collect()
}

async fn seed_mixed(base: &str, db: &Pool<Postgres>) -> anyhow::Result<()> {
    for (p, s) in [
        ("f/alpha/one", "Deploy tool"),
        ("f/alpha/two", "Cleanup job"),
        ("f/beta/one", "Beta thing"),
        ("u/test-user/solo", "Solo script"),
    ] {
        let r = authed(
            client().post(format!("{base}/scripts/create")),
            "SECRET_TOKEN",
        )
        .json(&new_script(p, s))
        .send()
        .await?;
        assert_eq!(r.status(), 201, "create script {p}: {}", r.text().await?);
    }
    let r = authed(
        client().post(format!("{base}/flows/create")),
        "SECRET_TOKEN",
    )
    .json(&new_flow("f/alpha/flowy", "Alpha flow"))
    .send()
    .await?;
    assert_eq!(r.status(), 201, "create flow: {}", r.text().await?);
    let r = authed(client().post(format!("{base}/apps/create")), "SECRET_TOKEN")
        .json(&new_app("f/beta/appy", "Beta app"))
        .send()
        .await?;
    assert_eq!(r.status(), 201, "create app: {}", r.text().await?);
    // Silence unused warnings on db in case future assertions drop it.
    let _ = db;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_runnables_path_start_scopes_to_folder(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");
    seed_mixed(&base, &db).await?;

    // path_start scopes to exactly one folder subtree (the folder-navigation path).
    let mut alpha = list_once(port, "path_start=f/alpha/").await;
    alpha.sort();
    assert_eq!(
        alpha,
        vec![
            "flow:f/alpha/flowy".to_string(),
            "script:f/alpha/one".to_string(),
            "script:f/alpha/two".to_string(),
        ],
        "path_start=f/alpha/ must return only that folder's items"
    );

    // A prefix that matches nothing returns an empty list, not an error.
    let empty = list_once(port, "path_start=f/nope/").await;
    assert!(
        empty.is_empty(),
        "unknown folder must be empty, got {empty:?}"
    );
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_runnables_search_and_kind_filters(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");
    seed_mixed(&base, &db).await?;

    // Case-insensitive substring search over summary/path.
    let deploy = list_once(port, "search=deploy").await;
    assert_eq!(
        deploy,
        vec!["script:f/alpha/one".to_string()],
        "search must substring-match the summary only"
    );

    // kinds filter selects a single kind.
    let flows = list_once(port, "kinds=flow").await;
    assert_eq!(flows, vec!["flow:f/alpha/flowy".to_string()], "kinds=flow");
    let apps = list_once(port, "kinds=app").await;
    assert_eq!(apps, vec!["app:f/beta/appy".to_string()], "kinds=app");
    let scripts = list_once(port, "kinds=script").await;
    assert_eq!(
        scripts.len(),
        4,
        "kinds=script -> 4 scripts, got {scripts:?}"
    );
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_runnables_starred_pinned_first(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");
    seed_mixed(&base, &db).await?;

    // Favorite a path that would otherwise sort last (name Z-ish); it must lead.
    sqlx::query(
        "INSERT INTO favorite (usr, workspace_id, path, favorite_kind) VALUES ('test-user','test-workspace','u/test-user/solo','script')",
    )
    .execute(&db)
    .await?;

    let items = list_once(port, "order_by=name&order_desc=false").await;
    assert_eq!(
        items.first().map(String::as_str),
        Some("script:u/test-user/solo"),
        "starred item pins to the top of the browse view regardless of order, got {items:?}"
    );
    Ok(())
}
