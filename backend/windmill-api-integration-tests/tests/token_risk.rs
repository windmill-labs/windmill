//! The `token_risk` view flags what a token review looks at, and the superadmin review (EE)
//! pages through it with a cursor, so a token used between two pages is not skipped. Without
//! `enterprise` (the CE image builds `private` alone), only its counts are served.

use sqlx::{Pool, Postgres, Row};
use windmill_test_utils::*;

/// A token with no risk: bound to test-workspace, expiring, just used, minted without rights.
async fn insert_token(db: &Pool<Postgres>, prefix: &str, email: &str) {
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, email, label, expiration, workspace_id, \
         super_admin, admin) \
         VALUES (md5($1), $1, $2, $1, now() + interval '30 days', 'test-workspace', false, false)",
    )
    .bind(prefix)
    .bind(email)
    .execute(db)
    .await
    .unwrap();
}

async fn update(db: &Pool<Postgres>, prefix: &str, set: &str) {
    sqlx::query(&format!("UPDATE token SET {set} WHERE token_prefix = $1"))
        .bind(prefix)
        .execute(db)
        .await
        .unwrap();
}

async fn risks(db: &Pool<Postgres>, prefix: &str) -> Vec<String> {
    sqlx::query("SELECT risks FROM token_risk WHERE token_prefix = $1")
        .bind(prefix)
        .fetch_one(db)
        .await
        .unwrap()
        .get("risks")
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_token_risk_rules(db: Pool<Postgres>) -> anyhow::Result<()> {
    // test@ is an admin of test-workspace, test2@ only a member.
    let admin = "test@windmill.dev";
    let member = "test2@windmill.dev";
    let cases: &[(&str, &str, &str, &[&str])] = &[
        ("clean", admin, "read_only = false", &[]),
        ("never", admin, "expiration = NULL", &["never_expires"]),
        ("unbound", admin, "workspace_id = NULL", &["all_workspaces"]),
        (
            "stale",
            admin,
            "last_used_at = now() - interval '91 days'",
            &["unused"],
        ),
        (
            "recent",
            admin,
            "last_used_at = now() - interval '89 days'",
            &[],
        ),
        (
            "sa",
            admin,
            "super_admin = true, admin = true",
            &["superadmin"],
        ),
        ("adm", admin, "admin = true", &["admin"]),
        // Rights the owner does not hold are not flagged, nor rights the token was minted without.
        ("member", member, "admin = true", &[]),
        // Scopes or read-only mode narrow the rights; a job tag filter alone does not.
        ("scoped", admin, "admin = true, scopes = '{jobs:read}'", &[]),
        ("ro", admin, "admin = true, read_only = true", &[]),
        (
            "tags",
            admin,
            "admin = true, scopes = '{if_jobs:filter_tags:prod}'",
            &["admin"],
        ),
    ];
    for (prefix, email, set, expected) in cases {
        insert_token(&db, prefix, email).await;
        update(&db, prefix, set).await;
        assert_eq!(&risks(&db, prefix).await, expected, "{prefix}");
    }

    // Tokens the server manages on its own carry no flag, however risky they look: a label
    // only the server mints, or a system label `tokens/create` also accepts on a token as
    // short-lived as the system's own. The same label on a long-lived token proves nothing.
    let managed: &[(&str, &str, bool)] = &[
        ("session", "expiration = NULL", true),
        ("ephemeral-script", "expiration = NULL", true),
        ("mcp-oauth-x", "expiration = NULL", true),
        ("impersonation:a", "expiration = created_at + interval '24 hours'", true),
        ("Ephemeral lsp token", "expiration = created_at + interval '72 hours'", true),
        ("debugger-token", "expiration = NULL", false),
        ("sdk_app:f", "expiration = created_at + interval '30 days'", false),
    ];
    for (i, (label, expiration, exempt)) in managed.iter().enumerate() {
        let prefix = format!("managed-{i}");
        insert_token(&db, &prefix, admin).await;
        update(
            &db,
            &prefix,
            &format!("label = '{label}', {expiration}, workspace_id = NULL, super_admin = true"),
        )
        .await;
        assert_eq!(risks(&db, &prefix).await.is_empty(), *exempt, "{label}");
    }
    Ok(())
}

#[cfg(feature = "enterprise")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_token_review_cursor_survives_use(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let url = format!(
        "http://localhost:{}/api/users/tokens/list_as_super_admin",
        server.addr.port()
    );
    let http = reqwest::Client::new();

    // Flagged (no expiration) and staler than the fixture's tokens, in this order.
    for (prefix, days) in [("page-a", 12), ("page-b", 11), ("page-c", 10)] {
        insert_token(&db, prefix, "test@windmill.dev").await;
        update(
            &db,
            prefix,
            &format!("expiration = NULL, last_used_at = now() - interval '{days} days'"),
        )
        .await;
    }

    let first: serde_json::Value = http
        .get(&url)
        .bearer_auth("SECRET_TOKEN")
        .query(&[("per_page", "1")])
        .send()
        .await?
        .json()
        .await?;
    let a = &first[0];
    assert_eq!(a["token_prefix"], "page-a");

    // page-a is used before the next page is read, which moves it to the end of the order:
    // an offset would now skip page-b.
    update(&db, "page-a", "last_used_at = now()").await;

    let next: serde_json::Value = http
        .get(&url)
        .bearer_auth("SECRET_TOKEN")
        .query(&[
            ("per_page", "1"),
            ("after_last_used_at", a["last_used_at"].as_str().unwrap()),
            ("after_token_prefix", "page-a"),
        ])
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(next[0]["token_prefix"], "page-b");
    Ok(())
}

#[cfg(not(feature = "enterprise"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_token_review_list_is_ee(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api/users/tokens", server.addr.port());
    let http = reqwest::Client::new();
    insert_token(&db, "victim", "test2@windmill.dev").await;

    let counts = http
        .get(format!("{base}/risk_counts_as_super_admin"))
        .bearer_auth("SECRET_TOKEN")
        .send()
        .await?;
    assert_eq!(counts.status(), 200);
    let list = http
        .get(format!("{base}/list_as_super_admin"))
        .bearer_auth("SECRET_TOKEN")
        .send()
        .await?;
    assert_eq!(list.status(), 400);
    let delete = http
        .delete(format!("{base}/delete_as_super_admin/test2@windmill.dev/victim"))
        .bearer_auth("SECRET_TOKEN")
        .send()
        .await?;
    assert_eq!(delete.status(), 400);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM token WHERE token_prefix = 'victim'")
        .fetch_one(&db)
        .await?;
    assert_eq!(left, 1);
    Ok(())
}
