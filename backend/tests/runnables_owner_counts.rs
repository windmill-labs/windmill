//! Regression test for `GET /w/{workspace}/runnables/counts`, which the homepage
//! tree uses to label every folder / user node and to drop the empty ones.
//!
//! The endpoint runs off the non-RLS pool and re-derives visibility from `path`
//! (admin / folder read set / own user space) plus one `extra_perms` pass, so it
//! duplicates by hand what RLS otherwise enforces. That duplication is what can
//! drift: a change to the RLS policies, or to how `authed.folders`/`groups` are
//! populated, would silently make the counts over- or under-report. So the test
//! pins them to the ground truth rather than to their own implementation — the
//! counts must equal the per-owner grouping of `/runnables/list`, which is
//! RLS-enforced, across every way an item can become visible.

use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn owner_of(path: &str) -> String {
    path.split('/').take(2).collect::<Vec<_>>().join("/")
}

/// Per-owner counts derived from the RLS-enforced listing. Pipeline members are
/// dropped to match what the tree renders (and what /counts promises): they are
/// folded into their folder's single pipeline entry, never listed as rows.
async fn counts_from_list(port: u16, token: &str) -> HashMap<String, i64> {
    let base = format!("http://localhost:{port}/api/w/test-workspace/runnables/list");
    let mut out: HashMap<String, i64> = HashMap::new();
    let mut cursor: Option<String> = None;
    loop {
        let mut url = format!("{base}?per_page=100");
        if let Some(c) = &cursor {
            url.push_str(&format!("&cursor={c}"));
        }
        let resp = authed(client().get(&url), token).send().await.unwrap();
        assert_eq!(resp.status(), 200, "list should succeed");
        let body: serde_json::Value = resp.json().await.unwrap();
        for it in body["items"].as_array().unwrap() {
            if it["auto_kind"].as_str() == Some("pipeline") {
                continue;
            }
            *out.entry(owner_of(it["path"].as_str().unwrap()))
                .or_insert(0) += 1;
        }
        match body["next_cursor"].as_str() {
            Some(c) => cursor = Some(c.to_string()),
            None => break,
        }
    }
    out
}

async fn counts_endpoint_q(port: u16, token: &str, query: &str) -> HashMap<String, i64> {
    let url = format!("http://localhost:{port}/api/w/test-workspace/runnables/counts?{query}");
    let resp = authed(client().get(&url), token).send().await.unwrap();
    assert_eq!(resp.status(), 200, "counts should succeed");
    let body: serde_json::Value = resp.json().await.unwrap();
    body["counts"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_i64().unwrap()))
        .collect()
}

async fn counts_endpoint(port: u16, token: &str) -> HashMap<String, i64> {
    counts_endpoint_q(port, token, "").await
}

async fn insert_script(
    db: &Pool<Postgres>,
    hash: i64,
    path: &str,
    extra_perms: &str,
    auto_kind: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, language, archived, extra_perms, auto_kind)
         VALUES ('test-workspace', $1, $2, '', '', 'x', 'test-user', 'deno', false, $3::jsonb, $4)",
    )
    .bind(hash)
    .bind(path)
    .bind(extra_perms)
    .bind(auto_kind)
    .execute(db)
    .await?;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_runnable_counts_match_rls_listing(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // `team` is readable by test-user-2; `secret` is not. Everything visible to
    // test-user-2 out of `secret` can only get there through an explicit share.
    for (name, extra_perms) in [("team", r#"{"u/test-user-2": false}"#), ("secret", "{}")] {
        sqlx::query(
            "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms)
             VALUES ('test-workspace', $1, $1, ARRAY[]::text[], $2::jsonb)",
        )
        .bind(name)
        .bind(extra_perms)
        .execute(&db)
        .await?;
    }
    // `empty` has no runnables at all: it must be absent from the counts so the
    // tree can drop it.
    sqlx::query(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms)
         VALUES ('test-workspace', 'empty', 'empty', ARRAY[]::text[], '{\"u/test-user-2\": false}'::jsonb)",
    )
    .execute(&db)
    .await?;

    sqlx::query(
        "INSERT INTO usr_to_group (workspace_id, group_, usr) VALUES ('test-workspace', 'all', 'test-user-2')",
    )
    .execute(&db)
    .await?;

    // Visible through the folder grant.
    insert_script(&db, 9000001, "f/team/s1", "{}", None).await?;
    insert_script(&db, 9000002, "f/team/s2", "{}", None).await?;
    // Visible through the caller's own user space.
    insert_script(&db, 9000003, "u/test-user-2/mine", "{}", None).await?;
    // Not visible at all: another user's space, and an unreadable folder.
    insert_script(&db, 9000004, "u/test-user/theirs", "{}", None).await?;
    insert_script(&db, 9000005, "f/secret/hidden", "{}", None).await?;
    // Visible only through an individual share out of an unreadable folder.
    insert_script(
        &db,
        9000006,
        "f/secret/shared_to_user",
        r#"{"u/test-user-2": false}"#,
        None,
    )
    .await?;
    // Visible only through a group the caller belongs to.
    insert_script(
        &db,
        9000007,
        "f/secret/shared_to_group",
        r#"{"g/all": false}"#,
        None,
    )
    .await?;
    // A pipeline member in a readable folder: listed as the folder's pipeline
    // entry, not as a runnable, so it must not inflate f/team's count.
    insert_script(&db, 9000008, "f/team/step", "{}", Some("pipeline")).await?;

    sqlx::query(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, schema, archived)
         VALUES ('test-workspace', 'f/team/fl1', '', '', '{}'::jsonb, 'test-user', '{}'::json, false)",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO app (workspace_id, path, summary, policy, versions, extra_perms)
         VALUES ('test-workspace', 'f/team/app1', '', '{}'::jsonb, ARRAY[]::bigint[], '{}'::jsonb)",
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // test-user-2 is a plain member: the folder grant, their own space, the direct
    // share and the group share must each land, and nothing else.
    let counts = counts_endpoint(port, "SECRET_TOKEN_2").await;
    assert_eq!(
        counts,
        counts_from_list(port, "SECRET_TOKEN_2").await,
        "non-admin counts must match the RLS-enforced listing"
    );
    assert_eq!(
        counts,
        HashMap::from([
            ("f/team".to_string(), 4),
            ("u/test-user-2".to_string(), 1),
            ("f/secret".to_string(), 2),
        ]),
        "expected the folder grant (2 scripts + flow + app, pipeline member excluded), \
         the own user space, and the two shares out of f/secret"
    );

    // An admin bypasses RLS entirely (and takes the grouped whole-workspace path
    // rather than the per-owner prefix scans), so the counts must widen accordingly.
    assert_eq!(
        counts_endpoint(port, "SECRET_TOKEN").await,
        counts_from_list(port, "SECRET_TOKEN").await,
        "admin counts must match the RLS-enforced listing"
    );

    // Every kind is its own count subquery, so a repeated entry must not be counted
    // twice (nor multiply the scans).
    assert_eq!(
        counts_endpoint_q(port, "SECRET_TOKEN_2", "kinds=script,script").await,
        counts_endpoint_q(port, "SECRET_TOKEN_2", "kinds=script").await,
        "duplicate kinds must not double the counts"
    );

    // Operators see flows and apps like anyone else; only library scripts are hidden
    // from them. (test-user-3 has no grants, so their own space is the whole story.)
    sqlx::query("UPDATE usr SET operator = true WHERE workspace_id = 'test-workspace' AND username = 'test-user-3'")
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, schema, archived)
         VALUES ('test-workspace', 'u/test-user-3/fl', '', '', '{}'::jsonb, 'test-user-3', '{}'::json, false)",
    )
    .execute(&db)
    .await?;
    insert_script(&db, 9000009, "u/test-user-3/lib", "{}", Some("lib")).await?;
    assert_eq!(
        counts_endpoint_q(port, "SECRET_TOKEN_3", "include_without_main=true").await,
        HashMap::from([("u/test-user-3".to_string(), 1)]),
        "an operator's flow must be counted, and library scripts stay hidden from them \
         even when asked for"
    );
    Ok(())
}
