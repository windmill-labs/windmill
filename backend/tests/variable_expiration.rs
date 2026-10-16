//! Tests for the variable expiration sweep. No worker: the sweep ends at the push, and the
//! queued job is what the tests assert on.

use sqlx::{Pool, Postgres};
use windmill_queue::variable_expiration::dispatch_expiring_variables;
use windmill_test_utils::{initialize_tracing, ApiServer};

const WS: &str = "test-workspace";

async fn configure_handler(db: &Pool<Postgres>, handler: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE workspace_settings SET variable_expiration_handler = $2::jsonb WHERE workspace_id = $1")
        .bind(WS)
        .bind(handler)
        .execute(db)
        .await?;
    Ok(())
}

/// Seed a deployed script the handler setting can point at.
async fn seed_handler_script(db: &Pool<Postgres>, path: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, language, lock)
           VALUES ($1, $2, $3, '', '', 'export function main() {}', 'test-user', 'bun', '')"#,
    )
    .bind(WS)
    .bind(path.bytes().fold(0i64, |h, b| h.wrapping_mul(31).wrapping_add(b as i64)))
    .bind(path)
    .execute(db)
    .await?;
    Ok(())
}

async fn seed_variable(db: &Pool<Postgres>, path: &str, expires_in: &str) -> anyhow::Result<()> {
    sqlx::query(&format!(
        r#"INSERT INTO variable (workspace_id, path, value, is_secret, description, value_expires_at)
           VALUES ($1, $2, 'MUST_NOT_LEAK', true, 'a credential', now() + interval '{expires_in}')"#
    ))
    .bind(WS)
    .bind(path)
    .execute(db)
    .await?;
    Ok(())
}

async fn queued_handler_jobs(db: &Pool<Postgres>, path: &str) -> anyhow::Result<i64> {
    Ok(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM v2_job WHERE runnable_path = $1")
            .bind(path)
            .fetch_one(db)
            .await?,
    )
}

/// The sweep dispatches inside the lead window, leaves everything outside it alone, and
/// does not fire twice for the same expiry.
#[sqlx::test(fixtures("base"))]
async fn test_dispatch_within_lead_time_once(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    configure_handler(&db, r#"{"path": "script/u/test-user/expiry_handler"}"#).await?;

    seed_variable(&db, "u/test-user/due", "30 minutes").await?;
    seed_variable(&db, "u/test-user/far", "10 days").await?;

    dispatch_expiring_variables(&db).await;

    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1,
        "exactly the variable inside the lead window should dispatch"
    );

    // Admins grant this group access to the secrets it rotates, so the job must run as it.
    let (path, is_secret, permissioned_as, permissioned_as_email): (String, bool, String, String) =
        sqlx::query_as(
            "SELECT args->>'variable_path', (args->>'is_secret')::boolean, permissioned_as,
             permissioned_as_email FROM v2_job WHERE runnable_path = $1",
        )
        .bind("u/test-user/expiry_handler")
        .fetch_one(&db)
        .await?;
    assert_eq!(path, "u/test-user/due");
    assert!(is_secret);
    assert_eq!(permissioned_as, "g/variable_expiration_handler");
    assert_eq!(
        permissioned_as_email,
        "variable_expiration_handler@windmill.dev"
    );

    let args: String = sqlx::query_scalar("SELECT args::text FROM v2_job WHERE runnable_path = $1")
        .bind("u/test-user/expiry_handler")
        .fetch_one(&db)
        .await?;
    assert!(
        !args.contains("MUST_NOT_LEAK"),
        "handler args leaked the value: {args}"
    );

    // The claim is what makes concurrent replicas safe.
    dispatch_expiring_variables(&db).await;
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1,
        "a dispatched variable must not fire again"
    );

    Ok(())
}

/// `notify_before_secs` on the handler replaces the default window.
#[sqlx::test(fixtures("base"))]
async fn test_notify_before_sets_the_window(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    configure_handler(
        &db,
        r#"{"path": "script/u/test-user/expiry_handler", "notify_before_secs": 1200}"#,
    )
    .await?;

    seed_variable(&db, "u/test-user/inside", "10 minutes").await?;
    seed_variable(&db, "u/test-user/outside", "30 minutes").await?;

    dispatch_expiring_variables(&db).await;

    let paths: Vec<String> =
        sqlx::query_scalar("SELECT args->>'variable_path' FROM v2_job WHERE runnable_path = $1")
            .bind("u/test-user/expiry_handler")
            .fetch_all(&db)
            .await?;
    assert_eq!(paths, vec!["u/test-user/inside".to_string()]);
    Ok(())
}

/// `remind_every_secs` runs the handler again once the interval has elapsed, and not before.
#[sqlx::test(fixtures("base"))]
async fn test_remind_every_repeats_the_notice(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    configure_handler(
        &db,
        r#"{"path": "script/u/test-user/expiry_handler", "remind_every_secs": 600}"#,
    )
    .await?;
    seed_variable(&db, "u/test-user/due", "30 minutes").await?;

    dispatch_expiring_variables(&db).await;
    dispatch_expiring_variables(&db).await;
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1,
        "no reminder before the interval has elapsed"
    );

    // A sweep landing a few seconds short of the interval must still remind.
    sqlx::query(
        "UPDATE variable SET expiration_notified_at = now() - interval '9 minutes 50 seconds'",
    )
    .execute(&db)
    .await?;
    dispatch_expiring_variables(&db).await;
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        2
    );

    sqlx::query(
        "UPDATE variable SET value_expires_at = now() - interval '1 minute',
         expiration_dispatched_for = now() - interval '1 minute',
         expiration_notified_at = now() - interval '11 minutes'",
    )
    .execute(&db)
    .await?;
    dispatch_expiring_variables(&db).await;
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        2
    );
    Ok(())
}

/// Moving `value_expires_at` re-arms the variable; re-writing the same date does not.
#[sqlx::test(fixtures("base"))]
async fn test_rearm_on_date_change(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!("http://localhost:{}", server.addr.port());
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    configure_handler(&db, r#"{"path": "script/u/test-user/expiry_handler"}"#).await?;
    seed_variable(&db, "u/test-user/due", "30 minutes").await?;

    // Through the real endpoint: writing the column directly would assert nothing.
    let update = async |body: serde_json::Value| -> anyhow::Result<u16> {
        Ok(reqwest::Client::new()
            .post(format!(
                "{base_url}/api/w/{WS}/variables/update/u/test-user/due"
            ))
            .bearer_auth("SECRET_TOKEN")
            .json(&body)
            .send()
            .await?
            .status()
            .as_u16())
    };

    dispatch_expiring_variables(&db).await;
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1
    );

    let stored: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "SELECT value_expires_at FROM variable WHERE workspace_id = $1 AND path = $2",
    )
    .bind(WS)
    .bind("u/test-user/due")
    .fetch_one(&db)
    .await?;

    // An unchanged `wmill sync push` rewrites the same date.
    assert_eq!(
        update(serde_json::json!({ "value_expires_at": stored })).await?,
        200
    );
    dispatch_expiring_variables(&db).await;
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1,
        "an unchanged expiry must not re-arm"
    );

    assert_eq!(
        update(serde_json::json!({ "description": "touched" })).await?,
        200
    );
    dispatch_expiring_variables(&db).await;
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1,
        "an update that omits value_expires_at must not re-arm"
    );

    assert_eq!(
        update(serde_json::json!({ "value_expires_at": stored + chrono::Duration::minutes(5) }))
            .await?,
        200
    );
    dispatch_expiring_variables(&db).await;
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        2,
        "a moved expiry must re-arm the handler"
    );

    Ok(())
}

/// With no handler configured nothing is dispatched, and `muted_on_user_path` keeps
/// personal-path variables out.
#[sqlx::test(fixtures("base"))]
async fn test_skips_unconfigured_and_muted_user_paths(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    seed_variable(&db, "u/test-user/personal", "30 minutes").await?;
    seed_variable(&db, "f/test_folder/shared", "30 minutes").await?;

    dispatch_expiring_variables(&db).await;
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        0
    );

    configure_handler(
        &db,
        r#"{"path": "script/u/test-user/expiry_handler", "muted_on_user_path": true}"#,
    )
    .await?;
    dispatch_expiring_variables(&db).await;

    let dispatched: Vec<String> =
        sqlx::query_scalar("SELECT args->>'variable_path' FROM v2_job WHERE runnable_path = $1")
            .bind("u/test-user/expiry_handler")
            .fetch_all(&db)
            .await?;
    assert_eq!(
        dispatched,
        vec!["f/test_folder/shared".to_string()],
        "a muted personal path must not dispatch"
    );

    Ok(())
}

/// A workspace whose handler cannot be resolved must not starve the others of the per-pass
/// cap, although its variables are the most overdue on the instance.
#[sqlx::test(fixtures("base"))]
async fn test_broken_handler_does_not_starve_other_workspaces(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    configure_handler(&db, r#"{"path": "script/u/test-user/expiry_handler"}"#).await?;
    seed_variable(&db, "u/test-user/due", "30 minutes").await?;

    sqlx::query(
        "INSERT INTO workspace (id, name, owner) VALUES ('poison-ws', 'poison-ws', 'test-user')",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO workspace_key (workspace_id, kind, key) VALUES ('poison-ws', 'cloud', 'k')",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        r#"INSERT INTO workspace_settings (workspace_id, variable_expiration_handler)
           VALUES ('poison-ws', '{"path": "script/u/test-user/does_not_exist"}'::jsonb)"#,
    )
    .execute(&db)
    .await?;
    // More overdue rows than one pass carries.
    sqlx::query(
        r#"INSERT INTO variable (workspace_id, path, value, is_secret, description, value_expires_at)
           SELECT 'poison-ws', 'f/creds/v' || i, 'v', false, '', now() - (i || ' minutes')::interval
           FROM generate_series(1, 120) i"#,
    )
    .execute(&db)
    .await?;

    dispatch_expiring_variables(&db).await;

    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1,
        "a workspace with an unresolvable handler must not starve the rest of the instance"
    );
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/does_not_exist").await?,
        0
    );

    Ok(())
}

/// The handler endpoint: admin-only, existence-checked, and clearing by omission, which
/// `wmill sync push` relies on to propagate an absent handler.
#[sqlx::test(fixtures("base"))]
async fn test_edit_handler_endpoint(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!("http://localhost:{}", server.addr.port());

    let edit = async |token: &str, body: serde_json::Value| -> anyhow::Result<u16> {
        Ok(reqwest::Client::new()
            .post(format!(
                "{base_url}/api/w/{WS}/workspaces/edit_variable_expiration_handler"
            ))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?
            .status()
            .as_u16())
    };
    let stored = async || -> anyhow::Result<Option<serde_json::Value>> {
        Ok(sqlx::query_scalar(
            "SELECT variable_expiration_handler FROM workspace_settings WHERE workspace_id = $1",
        )
        .bind(WS)
        .fetch_one(&db)
        .await?)
    };

    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    let handler = serde_json::json!({ "path": "script/u/test-user/expiry_handler" });
    assert_eq!(edit("SECRET_TOKEN_2", handler.clone()).await?, 403);
    assert!(stored().await?.is_none());

    for path in ["u/test-user/expiry_handler", "script/u/test-user/does_not_exist"] {
        assert_eq!(
            edit("SECRET_TOKEN", serde_json::json!({ "path": path })).await?,
            400,
            "{path}"
        );
    }

    assert_eq!(edit("SECRET_TOKEN", handler).await?, 200);
    assert_eq!(
        stored()
            .await?
            .and_then(|h| h["path"].as_str().map(String::from)),
        Some("script/u/test-user/expiry_handler".to_string())
    );

    // `extra_args` holds the built-ins' channel. The endpoint replaces the setting rather than
    // merging, so the settings tab sends back what it loaded.
    assert_eq!(
        edit(
            "SECRET_TOKEN",
            serde_json::json!({
                "path": "script/u/test-user/expiry_handler",
                "extra_args": { "channel": "windmill-alerts" }
            })
        )
        .await?,
        200
    );
    assert_eq!(
        stored().await?.map(|h| h["extra_args"].clone()),
        Some(serde_json::json!({ "channel": "windmill-alerts" }))
    );

    assert_eq!(
        edit(
            "SECRET_TOKEN",
            serde_json::json!({
                "path": "script/u/test-user/expiry_handler",
                "extra_args": "not-an-object"
            })
        )
        .await?,
        400
    );

    assert_eq!(edit("SECRET_TOKEN", serde_json::json!({})).await?, 200);
    assert!(
        stored().await?.is_none(),
        "an omitted path clears the handler"
    );

    Ok(())
}
