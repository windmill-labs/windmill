//! The `token_risk` view flags what a token review looks at, and the superadmin review pages
//! through it with a cursor, so a token used between two pages is not skipped.

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

    // Tokens the server manages on its own carry no flag, however risky they look.
    for (i, label) in [
        "session",
        "ephemeral-script",
        "mcp-oauth-x",
        "impersonation:a",
        "sdk_app:f",
    ]
    .iter()
    .enumerate()
    {
        let prefix = format!("managed-{i}");
        insert_token(&db, &prefix, admin).await;
        update(
            &db,
            &prefix,
            &format!(
                "label = '{label}', expiration = NULL, workspace_id = NULL, super_admin = true"
            ),
        )
        .await;
        assert_eq!(risks(&db, &prefix).await, Vec::<String>::new(), "{label}");
    }
    Ok(())
}

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
    let a = &first["tokens"][0];
    assert_eq!(a["token_prefix"], "page-a");
    assert!(first["counts"]["never_expires"].as_i64().unwrap() >= 3);

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
    assert_eq!(next["tokens"][0]["token_prefix"], "page-b");
    assert!(next.get("counts").is_none());
    Ok(())
}
