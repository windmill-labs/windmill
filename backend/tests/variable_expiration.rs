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
        r#"INSERT INTO variable (workspace_id, path, value, is_secret, description, value_valid_until)
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

/// The sweep dispatches inside the handler's `notify_before_secs`, leaves everything outside it
/// alone, and does not fire twice for the same expiry.
#[sqlx::test(fixtures("base"))]
async fn test_dispatch_within_lead_time_once(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    configure_handler(
        &db,
        r#"{"path": "script/u/test-user/expiry_handler", "notify_before_secs": 3600}"#,
    )
    .await?;

    seed_variable(&db, "u/test-user/due", "30 minutes").await?;
    seed_variable(&db, "u/test-user/far", "2 hours").await?;

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

/// A variable's writer must not get the handler to resolve a reference as its own identity;
/// the admin's `extra_args` still resolve.
#[sqlx::test(fixtures("base"))]
async fn test_variable_fields_do_not_resolve_in_handler(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    configure_handler(
        &db,
        r#"{"path": "script/u/test-user/expiry_handler", "extra_args": {"slack": "$res:f/slack_bot/bot_token"}}"#,
    )
    .await?;
    seed_variable(&db, "u/test-user/due", "30 minutes").await?;
    sqlx::query(
        "UPDATE variable SET description = '$var:u/test-user/other' WHERE path = 'u/test-user/due'",
    )
    .execute(&db)
    .await?;

    dispatch_expiring_variables(&db).await;

    let (description, slack): (String, String) = sqlx::query_as(
        "SELECT args->>'description', args->>'slack' FROM v2_job WHERE runnable_path = $1",
    )
    .bind("u/test-user/expiry_handler")
    .fetch_one(&db)
    .await?;
    assert!(
        !description.starts_with("$var:"),
        "description still resolves: {description}"
    );
    assert_eq!(slack, "$res:f/slack_bot/bot_token");
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
        "UPDATE variable SET value_valid_until = now() - interval '1 minute',
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

/// Reminders due across a full pass's worth of workspaces must not delay a first notice.
#[sqlx::test(fixtures("base"))]
async fn test_first_notice_goes_before_reminders(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    configure_handler(&db, r#"{"path": "script/u/test-user/expiry_handler"}"#).await?;
    seed_variable(&db, "u/test-user/fresh", "20 minutes").await?;
    sqlx::raw_sql(
        r#"INSERT INTO workspace (id, name, owner) SELECT 'busy-' || i, 'busy-' || i, 'test-user' FROM generate_series(1, 100) i;
           INSERT INTO workspace_key (workspace_id, kind, key) SELECT 'busy-' || i, 'cloud', 'k' FROM generate_series(1, 100) i;
           INSERT INTO workspace_settings (workspace_id, variable_expiration_handler)
           SELECT 'busy-' || i, '{"path": "script/u/test-user/expiry_handler", "remind_every_secs": 60}'::jsonb FROM generate_series(1, 100) i;
           INSERT INTO variable (workspace_id, path, value, is_secret, description, value_valid_until,
                                 expiration_dispatched_for, expiration_notified_at)
           SELECT 'busy-' || i, 'f/creds/v', 'v', false, '', now() + interval '10 minutes',
                  now() + interval '10 minutes', now() - interval '2 minutes'
           FROM generate_series(1, 100) i;"#,
    )
    .execute(&db)
    .await?;

    dispatch_expiring_variables(&db).await;

    let notified: bool = sqlx::query_scalar(
        "SELECT expiration_dispatched_for IS NOT NULL FROM variable WHERE path = 'u/test-user/fresh'",
    )
    .fetch_one(&db)
    .await?;
    assert!(
        notified,
        "the first notice waited behind other workspaces' reminders"
    );
    Ok(())
}

/// Moving `value_valid_until` re-arms the variable; re-writing the same date does not.
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
        "SELECT value_valid_until FROM variable WHERE workspace_id = $1 AND path = $2",
    )
    .bind(WS)
    .bind("u/test-user/due")
    .fetch_one(&db)
    .await?;

    // An unchanged `wmill sync push` rewrites the same date.
    assert_eq!(
        update(serde_json::json!({ "value_valid_until": stored })).await?,
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
        "an update that omits value_valid_until must not re-arm"
    );

    assert_eq!(
        update(serde_json::json!({ "value_valid_until": stored + chrono::Duration::minutes(5) }))
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

/// `muted_on_user_path` keeps personal-path variables out.
#[sqlx::test(fixtures("base"))]
async fn test_muted_user_paths(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    seed_variable(&db, "u/test-user/personal", "30 minutes").await?;
    seed_variable(&db, "f/test_folder/shared", "30 minutes").await?;
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

    // More overdue rows than one pass carries.
    sqlx::raw_sql(
        r#"INSERT INTO workspace (id, name, owner) VALUES ('poison-ws', 'poison-ws', 'test-user');
           INSERT INTO workspace_key (workspace_id, kind, key) VALUES ('poison-ws', 'cloud', 'k');
           INSERT INTO workspace_settings (workspace_id, variable_expiration_handler)
           VALUES ('poison-ws', '{"path": "script/u/test-user/does_not_exist"}'::jsonb);
           INSERT INTO variable (workspace_id, path, value, is_secret, description, value_valid_until)
           SELECT 'poison-ws', 'f/creds/v' || i, 'v', false, '', now() - (i || ' minutes')::interval
           FROM generate_series(1, 120) i;"#,
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
    let claimed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM variable WHERE workspace_id = 'poison-ws' AND expiration_dispatched_for IS NOT NULL",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        claimed, 0,
        "a handler that failed to start must leave its rows to retry"
    );

    Ok(())
}

/// A pass's worth of workspaces with broken handlers and older expiries must not starve a
/// healthy one forever: their rows stay due, so every pass would pick them again.
#[sqlx::test(fixtures("base"))]
async fn test_many_broken_handlers_do_not_starve_a_healthy_one(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    configure_handler(&db, r#"{"path": "script/u/test-user/expiry_handler"}"#).await?;
    seed_variable(&db, "u/test-user/due", "30 minutes").await?;
    sqlx::raw_sql(
        r#"INSERT INTO workspace (id, name, owner) SELECT 'broken-' || i, 'broken-' || i, 'test-user' FROM generate_series(1, 100) i;
           INSERT INTO workspace_key (workspace_id, kind, key) SELECT 'broken-' || i, 'cloud', 'k' FROM generate_series(1, 100) i;
           INSERT INTO workspace_settings (workspace_id, variable_expiration_handler)
           SELECT 'broken-' || i, '{"path": "script/u/test-user/does_not_exist"}'::jsonb FROM generate_series(1, 100) i;
           INSERT INTO variable (workspace_id, path, value, is_secret, description, value_valid_until)
           SELECT 'broken-' || i, 'f/creds/v', 'v', false, '', now() - interval '1 day' FROM generate_series(1, 100) i;"#,
    )
    .execute(&db)
    .await?;

    for _ in 0..10 {
        dispatch_expiring_variables(&db).await;
    }

    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1,
        "the healthy workspace never got a pass"
    );
    Ok(())
}

/// A workspace waiting out a failed start must not keep waiting once its handler is deployed.
#[sqlx::test(fixtures("base"))]
async fn test_deploying_the_handler_ends_the_wait(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    configure_handler(&db, r#"{"path": "script/u/test-user/expiry_handler"}"#).await?;
    seed_variable(&db, "u/test-user/due", "30 minutes").await?;

    dispatch_expiring_variables(&db).await;
    seed_handler_script(&db, "u/test-user/expiry_handler").await?;
    dispatch_expiring_variables(&db).await;

    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1,
        "the deployed handler waited out the delay"
    );
    Ok(())
}

/// A handler deployed while the failing pass is still running must end the wait too. The
/// trigger deploys it in the same statement that records the failure.
#[sqlx::test(fixtures("base"))]
async fn test_deploying_the_handler_mid_pass_ends_the_wait(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    configure_handler(&db, r#"{"path": "script/u/test-user/expiry_handler"}"#).await?;
    seed_variable(&db, "u/test-user/due", "30 minutes").await?;
    sqlx::raw_sql(
        r#"CREATE FUNCTION deploy_mid_pass() RETURNS trigger LANGUAGE plpgsql AS $$
           BEGIN
               INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, language, lock)
               VALUES (NEW.workspace_id, 42, 'u/test-user/expiry_handler', '', '', 'export function main() {}', 'test-user', 'bun', '');
               RETURN NEW;
           END $$;
           CREATE TRIGGER deploy_mid_pass BEFORE UPDATE OF variable_expiration_failed_at ON workspace_settings
           FOR EACH ROW EXECUTE FUNCTION deploy_mid_pass();"#,
    )
    .execute(&db)
    .await?;

    dispatch_expiring_variables(&db).await;
    sqlx::raw_sql("DROP TRIGGER deploy_mid_pass ON workspace_settings")
        .execute(&db)
        .await?;
    dispatch_expiring_variables(&db).await;

    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/expiry_handler").await?,
        1,
        "the handler deployed mid-pass waited out the delay"
    );
    Ok(())
}

/// A handler script still waiting on its dependency job is not a failure: writing its lock
/// changes nothing the wait would see.
#[sqlx::test(fixtures("base"))]
async fn test_pending_handler_lock_is_not_a_failure(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    configure_handler(&db, r#"{"path": "script/u/test-user/pending_lock_handler"}"#).await?;
    seed_variable(&db, "u/test-user/due", "30 minutes").await?;
    seed_handler_script(&db, "u/test-user/pending_lock_handler").await?;
    sqlx::query("UPDATE script SET lock = NULL WHERE workspace_id = $1")
        .bind(WS)
        .execute(&db)
        .await?;

    dispatch_expiring_variables(&db).await;
    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/pending_lock_handler").await?,
        0
    );
    sqlx::query("UPDATE script SET lock = '' WHERE workspace_id = $1")
        .bind(WS)
        .execute(&db)
        .await?;
    dispatch_expiring_variables(&db).await;

    assert_eq!(
        queued_handler_jobs(&db, "u/test-user/pending_lock_handler").await?,
        1,
        "the handler waited out the delay once its lock landed"
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

    for path in [
        "u/test-user/expiry_handler",
        "script/u/test-user/does_not_exist",
    ] {
        assert_eq!(
            edit("SECRET_TOKEN", serde_json::json!({ "path": path })).await?,
            400,
            "{path}"
        );
    }

    // A handler that failed to start waits out its retry delay; saving a fix must not.
    sqlx::query("UPDATE workspace_settings SET variable_expiration_failed_at = now() WHERE workspace_id = $1")
        .bind(WS)
        .execute(&db)
        .await?;
    assert_eq!(edit("SECRET_TOKEN", handler).await?, 200);
    assert_eq!(
        stored()
            .await?
            .and_then(|h| h["path"].as_str().map(String::from)),
        Some("script/u/test-user/expiry_handler".to_string())
    );
    let waiting: bool = sqlx::query_scalar(
        "SELECT variable_expiration_failed_at IS NOT NULL FROM workspace_settings WHERE workspace_id = $1",
    )
    .bind(WS)
    .fetch_one(&db)
    .await?;
    assert!(!waiting, "saving the handler must end its retry delay");

    assert_eq!(edit("SECRET_TOKEN", serde_json::json!({})).await?, 200);
    assert!(
        stored().await?.is_none(),
        "an omitted path clears the handler"
    );

    Ok(())
}
