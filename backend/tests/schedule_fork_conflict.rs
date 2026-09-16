//! Enabling a schedule in a fork warns about every ancestor sharing the path,
//! not only the direct parent: the row was cloned down the whole chain, so the
//! cron is shared with whichever ancestors still hold a copy.

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

async fn set_enabled(
    base: &str,
    path: &str,
    enabled: bool,
    force: bool,
) -> anyhow::Result<(u16, String)> {
    let resp = reqwest::Client::new()
        .post(format!("{base}/api/w/sfc-leaf/schedules/setenabled/{path}"))
        .header("Authorization", "Bearer SFC_ADMIN_TOKEN")
        .json(&json!({ "enabled": enabled, "force": force }))
        .send()
        .await?;
    Ok((resp.status().as_u16(), resp.text().await?))
}

#[sqlx::test(fixtures("schedule_fork_conflict"))]
async fn enabling_in_a_fork_names_the_nearest_ancestor_sharing_the_path(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}", server.addr.port());

    let (status, body) = set_enabled(&base, "f/shared/nightly", true, false).await?;
    assert_eq!(status, 400, "{body}");
    assert!(
        body.contains("fork-conflict:schedule:sfc-root"),
        "the middle fork deleted its copy, so the root is the one still sharing the cron: {body}"
    );

    let (status, body) = set_enabled(&base, "f/shared/own", true, false).await?;
    assert_eq!(
        status, 200,
        "a path nothing upstream has enables freely: {body}"
    );

    sqlx::query(
        "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, enabled, script_path, args, is_flow, email, timezone, extra_perms, permissioned_as)
         VALUES ('sfc-mid', 'f/shared/nightly', 'sfc-admin', NOW(), '0 0 0 * * *', false, 'f/shared/job', '{}', false, 'sfc-admin@windmill.dev', 'UTC', '{}', 'u/sfc-admin')",
    )
    .execute(&db)
    .await?;
    let (status, body) = set_enabled(&base, "f/shared/nightly", true, false).await?;
    assert_eq!(status, 400, "{body}");
    assert!(
        body.contains("fork-conflict:schedule:sfc-mid"),
        "with the parent holding a copy again, it is the nearest and gets named: {body}"
    );
    Ok(())
}
