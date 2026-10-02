//! A job token restricted by `job_token_scopes` reaches only what its scopes allow, plus the
//! runtime routes about its own job, and no job it creates holds a wider token than it.

use reqwest::StatusCode;
use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::jobs::JobPayload;
use windmill_test_utils::*;

const OIDC_JOB: &str = "b0000000-0000-0000-0000-000000000001";
const RUN_JOB: &str = "b0000000-0000-0000-0000-000000000002";

async fn insert_job(db: &Pool<Postgres>, id: &str, scopes: &[&str]) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email,
            kind, script_lang, runnable_path, tag, job_token_scopes)
        VALUES ($1, 'test-workspace', 'test-user-3', 'u/test-user-3', 'test3@windmill.dev',
            'script', 'deno', 'u/test-user-3/agent', 'deno', $2)",
    )
    .bind(Uuid::parse_str(id)?)
    .bind(scopes)
    .execute(db)
    .await?;
    Ok(())
}

async fn job_token(db: &Pool<Postgres>, id: &str) -> anyhow::Result<String> {
    Ok(windmill_common::auth::create_token_for_owner(
        db,
        "test-workspace",
        "u/test-user-3",
        "ephemeral-script",
        300,
        "test3@windmill.dev",
        &Uuid::parse_str(id)?,
        None,
        None,
    )
    .await?)
}

async fn insert_script(
    db: &Pool<Postgres>,
    path: &str,
    hash: i64,
    scopes: Option<&[&str]>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO script (workspace_id, created_by, content, schema, summary, description,
            path, hash, language, lock, kind, job_token_scopes)
        VALUES ('test-workspace', 'test-user-3', 'export function main() {}', '{}', '', '',
            $1, $2, 'deno', '', 'script', $3)",
    )
    .bind(path)
    .bind(hash)
    .bind(scopes)
    .execute(db)
    .await?;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_restricted_job_token_is_confined(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    insert_script(&db, "u/test-user-3/open", 515151, None).await?;
    insert_script(
        &db,
        "u/test-user-3/oidc_only",
        525252,
        Some(&["oidc:write"]),
    )
    .await?;
    insert_job(&db, OIDC_JOB, &["oidc:write"]).await?;
    insert_job(&db, RUN_JOB, &["jobs:run"]).await?;

    let server = ApiServer::start(db.clone()).await?;
    set_jwt_secret().await;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    let client = reqwest::Client::new();
    let oidc_token = job_token(&db, OIDC_JOB).await?;

    let refused = [
        client.get(format!("{base}/variables/get_value/u/test-user-3/secret")),
        client
            .post(format!("{base}/scripts/create"))
            .json(&json!({})),
        client
            .post(format!("{base}/schedules/create"))
            .json(&json!({})),
        client
            .post(format!("{base}/jobs/run/p/u/test-user-3/open"))
            .json(&json!({})),
    ];
    for request in refused {
        let resp = request.bearer_auth(&oidc_token).send().await?;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "{}",
            resp.text().await?
        );
    }
    let own = client
        .get(format!("{base}/jobs_u/get_root_job_id/{OIDC_JOB}"))
        .bearer_auth(&oidc_token)
        .send()
        .await?;
    assert_eq!(own.status(), StatusCode::OK, "{}", own.text().await?);

    // A job it starts is capped at its own scopes, intersected with the target's setting.
    let run_token = job_token(&db, RUN_JOB).await?;
    for (path, expected) in [
        ("open", vec!["jobs:run".to_string()]),
        ("oidc_only", vec![]),
    ] {
        let resp = client
            .post(format!("{base}/jobs/run/p/u/test-user-3/{path}"))
            .bearer_auth(&run_token)
            .json(&json!({}))
            .send()
            .await?;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let child = Uuid::parse_str(&resp.text().await?)?;
        let scopes: Option<Vec<String>> =
            sqlx::query_scalar("SELECT job_token_scopes FROM v2_job WHERE id = $1")
                .bind(child)
                .fetch_one(&db)
                .await?;
        assert_eq!(scopes, Some(expected), "child of {path}");
    }
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_flow_steps_inherit_the_flow_restriction(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let value = json!({ "modules": [{ "id": "a", "value": { "type": "identity" } }] });
    sqlx::query(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at,
            schema, extra_perms, versions, job_token_scopes)
        VALUES ('test-workspace', 'u/test-user/restricted', '', '', $1, 'test-user', now(), '{}',
            '{}', '{7171}', '{oidc:write}')",
    )
    .bind(&value)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by)
        VALUES (7171, 'test-workspace', 'u/test-user/restricted', $1, '{}', 'test-user')",
    )
    .bind(&value)
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let flow = RunJob::from(JobPayload::Flow {
        path: "u/test-user/restricted".to_string(),
        dedicated_worker: None,
        apply_preprocessor: false,
        version: 7171,
        labels: None,
        job_token_scopes: Some(vec!["oidc:write".to_string()]),
    })
    .run_until_complete(&db, false, server.addr.port())
    .await;

    let step: Option<Vec<String>> =
        sqlx::query_scalar("SELECT job_token_scopes FROM v2_job WHERE parent_job = $1")
            .bind(flow.id)
            .fetch_one(&db)
            .await?;
    assert_eq!(step, Some(vec!["oidc:write".to_string()]));
    Ok(())
}
