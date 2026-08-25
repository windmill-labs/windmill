//! Tests for the variable expiration sweep (`windmill_queue::variable_expiration`).
//!
//! `dispatch_expiring_variables` is called directly against seeded `variable` /
//! `workspace_settings` rows — no worker is needed, since the sweep's job ends at the push
//! and the queued job is itself part of the assertions.

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

    // The identity is load-bearing, not incidental: the settings tab tells admins to grant
    // `g/variable_expiration_handler` access to the secrets it rotates, so the job must run as
    // that group and under its own email — which is also what keeps cloud metering off the
    // configuring user's quota.
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

    // Job args are stored in cleartext and shown in Runs, so the value must never be in them.
    let args: String = sqlx::query_scalar("SELECT args::text FROM v2_job WHERE runnable_path = $1")
        .bind("u/test-user/expiry_handler")
        .fetch_one(&db)
        .await?;
    assert!(
        !args.contains("MUST_NOT_LEAK"),
        "handler args leaked the value: {args}"
    );

    // A second pass must not re-dispatch: the claim is what makes replicas ticking
    // concurrently safe.
    dispatch_expiring_variables(&db).await;
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1,
        "a dispatched variable must not fire again"
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

    // Driven through the real endpoint: what re-arms is the stored date differing from
    // `expiration_dispatched_for`, so a test writing that column itself would assert nothing.
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

    // Without this, a `wmill sync push` of an unchanged spec re-fires the handler every push.
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
/// personal-path variables out — mirroring `error_handler_muted_on_user_path`.
#[sqlx::test(fixtures("base"))]
async fn test_skips_unconfigured_and_muted_user_paths(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    seed_variable(&db, "u/test-user/personal", "30 minutes").await?;
    seed_variable(&db, "f/test_folder/shared", "30 minutes").await?;

    // No handler configured yet.
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

/// A workspace whose handler cannot be resolved must not consume the per-pass cap. Its
/// variables are the most overdue on the instance, so without the back-off they refill the
/// limit on every pass and no other workspace ever dispatches.
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
    // More overdue rows than one pass can carry, so they alone would fill the cap.
    sqlx::query(
        r#"INSERT INTO variable (workspace_id, path, value, is_secret, description, value_expires_at)
           SELECT 'poison-ws', 'f/creds/v' || i, 'v', false, '', now() - (i || ' minutes')::interval
           FROM generate_series(1, 120) i"#,
    )
    .execute(&db)
    .await?;

    // First pass reaches only the poison workspace; the second finds it backing off.
    dispatch_expiring_variables(&db).await;
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

/// Pointing the workspace at another handler ends the back-off the broken one earned.
#[sqlx::test(fixtures("base"))]
async fn test_repointing_the_handler_ends_the_backoff(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    configure_handler(&db, r#"{"path": "script/u/test-user/does_not_exist"}"#).await?;
    seed_variable(&db, "u/test-user/due", "30 minutes").await?;

    // Earns a cooldown, which the next pass would otherwise spend skipping the workspace.
    dispatch_expiring_variables(&db).await;
    configure_handler(&db, r#"{"path": "script/u/test-user/expiry_handler"}"#).await?;
    dispatch_expiring_variables(&db).await;

    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1,
        "a repointed handler must be tried on the very next pass"
    );

    Ok(())
}

/// The endpoint that configures the handler: admin-only, prefix-checked, and clearing by
/// omission — which is what `wmill sync push` relies on to propagate an absent handler.
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

    let handler = serde_json::json!({ "path": "script/u/test-user/expiry_handler" });
    assert_eq!(edit("SECRET_TOKEN_2", handler.clone()).await?, 403);
    assert!(stored().await?.is_none());

    // A path the sweep could never resolve to a runnable is rejected up front.
    assert_eq!(
        edit(
            "SECRET_TOKEN",
            serde_json::json!({ "path": "u/test-user/expiry_handler" })
        )
        .await?,
        400
    );

    assert_eq!(edit("SECRET_TOKEN", handler).await?, 200);
    assert_eq!(
        stored()
            .await?
            .and_then(|h| h["path"].as_str().map(String::from)),
        Some("script/u/test-user/expiry_handler".to_string())
    );

    assert_eq!(edit("SECRET_TOKEN", serde_json::json!({})).await?, 200);
    assert!(
        stored().await?.is_none(),
        "an omitted path clears the handler"
    );

    Ok(())
}
