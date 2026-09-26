//! `parent_job` / `root_job` on the run endpoints set the lineage a job is identified by
//! (`WM_FLOW_JOB_ID`, `WM_ROOT_FLOW_JOB_ID`, the OIDC token's flow path). Only a job's own
//! `WM_TOKEN` may claim that job or its ancestors, and a workspace admin any job of the
//! workspace; a lineage anyone else names is dropped from the pushed job.

use reqwest::StatusCode;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_test_utils::*;

const ADMIN_JOB: &str = "a0000000-0000-0000-0000-000000000001";
const ROOT_JOB: &str = "a0000000-0000-0000-0000-000000000002";
const USER_JOB: &str = "a0000000-0000-0000-0000-000000000003";

async fn insert_job(
    db: &Pool<Postgres>,
    id: &str,
    user: &str,
    email: &str,
    root: Option<&str>,
) -> anyhow::Result<()> {
    let root = root.map(Uuid::parse_str).transpose()?;
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email,
            kind, script_lang, runnable_path, tag, parent_job, root_job, flow_innermost_root_job)
        VALUES ($1, 'test-workspace', $2, 'u/' || $2, $3, 'script', 'deno', 'u/x/y', 'deno',
            $4, $4, $4)",
    )
    .bind(Uuid::parse_str(id)?)
    .bind(user)
    .bind(email)
    .bind(root)
    .execute(db)
    .await?;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_run_lineage_must_be_the_callers_own(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    sqlx::query(
        "INSERT INTO script (workspace_id, created_by, content, schema, summary, description,
            path, hash, language, lock, kind)
        VALUES ('test-workspace', 'test-user-3', 'export function main() {}', '{}', '', '',
            'u/test-user-3/probe', 424242, 'deno', '', 'script')",
    )
    .execute(&db)
    .await?;
    insert_job(&db, ADMIN_JOB, "test-user", "test@windmill.dev", None).await?;
    insert_job(&db, ROOT_JOB, "test-user-3", "test3@windmill.dev", None).await?;
    insert_job(
        &db,
        USER_JOB,
        "test-user-3",
        "test3@windmill.dev",
        Some(ROOT_JOB),
    )
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    set_jwt_secret().await;
    let job_token = windmill_common::auth::create_token_for_owner(
        &db,
        "test-workspace",
        "u/test-user-3",
        "ephemeral-script",
        300,
        "test3@windmill.dev",
        &Uuid::parse_str(USER_JOB)?,
        None,
        None,
    )
    .await?;

    let client = reqwest::Client::new();
    let cases = [
        // a user's own token claims neither someone else's job nor even its own
        (
            "SECRET_TOKEN_3",
            Some(ADMIN_JOB),
            Some(ADMIN_JOB),
            None,
            None,
        ),
        ("SECRET_TOKEN_3", Some(USER_JOB), None, None, None),
        // what the SDKs send from inside a job: that job and its root flow
        (
            &job_token,
            Some(USER_JOB),
            Some(ROOT_JOB),
            Some(USER_JOB),
            Some(ROOT_JOB),
        ),
        (
            &job_token,
            Some(USER_JOB),
            Some(ADMIN_JOB),
            Some(USER_JOB),
            None,
        ),
        ("SECRET_TOKEN", Some(USER_JOB), None, Some(USER_JOB), None),
    ];
    for (token, parent, root, kept_parent, kept_root) in cases {
        let query = [("parent_job", parent), ("root_job", root)]
            .into_iter()
            .filter_map(|(k, v)| v.map(|v| format!("{k}={v}")))
            .collect::<Vec<_>>()
            .join("&");
        let resp = client
            .post(format!(
                "http://localhost:{}/api/w/test-workspace/jobs/run/p/u/test-user-3/probe?{query}",
                server.addr.port()
            ))
            .bearer_auth(token)
            .json(&serde_json::json!({}))
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        assert_eq!(status, StatusCode::CREATED, "{query}: {body}");
        let (stored_parent, stored_root): (Option<Uuid>, Option<Uuid>) =
            sqlx::query_as("SELECT parent_job, flow_innermost_root_job FROM v2_job WHERE id = $1")
                .bind(Uuid::parse_str(&body)?)
                .fetch_one(&db)
                .await?;
        let expected = |id: Option<&str>| id.map(Uuid::parse_str).transpose();
        assert_eq!(
            stored_parent,
            expected(kept_parent)?,
            "parent_job for {query}"
        );
        assert_eq!(stored_root, expected(kept_root)?, "root_job for {query}");
    }
    Ok(())
}
