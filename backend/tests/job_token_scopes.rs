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
const FLOW_JOB: &str = "b0000000-0000-0000-0000-000000000003";

async fn insert_job(
    db: &Pool<Postgres>,
    id: &str,
    parent: Option<&str>,
    scopes: &[&str],
) -> anyhow::Result<()> {
    let id = Uuid::parse_str(id)?;
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email,
            kind, script_lang, runnable_path, tag, parent_job)
        VALUES ($1, 'test-workspace', 'test-user-3', 'u/test-user-3', 'test3@windmill.dev',
            'script', 'deno', 'u/test-user-3/agent', 'deno', $2)",
    )
    .bind(id)
    .bind(parent.map(Uuid::parse_str).transpose()?)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO job_perms (job_id, email, username, is_admin, is_operator, folders, groups,
            workspace_id, job_token_scopes)
        VALUES ($1, 'test3@windmill.dev', 'test-user-3', false, false, '{}', '{}',
            'test-workspace', $2)",
    )
    .bind(id)
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
    insert_job(&db, FLOW_JOB, None, &["oidc:write"]).await?;
    insert_job(&db, OIDC_JOB, Some(FLOW_JOB), &["oidc:write"]).await?;
    insert_job(&db, RUN_JOB, None, &["jobs:run"]).await?;

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
    // The orchestrator reads a flow's step results with the token of the step that just ran.
    for (job, refused) in [(FLOW_JOB, false), (RUN_JOB, true)] {
        let resp = client
            .get(format!("{base}/jobs/result_by_id/{job}/a"))
            .bearer_auth(&oidc_token)
            .send()
            .await?;
        assert_eq!(resp.status() == StatusCode::FORBIDDEN, refused, "{job}");
    }
    // Its progress reaches only the flow it runs in.
    for (flow, refused) in [(FLOW_JOB, false), (RUN_JOB, true)] {
        let resp = client
            .post(format!("{base}/job_metrics/set_progress/{OIDC_JOB}"))
            .bearer_auth(&oidc_token)
            .json(&json!({ "percent": 50, "flow_job_id": flow }))
            .send()
            .await?;
        assert_eq!(resp.status() == StatusCode::FORBIDDEN, refused, "{flow}");
    }

    // Imported queue rows bypass push, so a restricted token cannot import, even an admin's.
    let admin_job = Uuid::parse_str("b0000000-0000-0000-0000-000000000004")?;
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email,
            kind, script_lang, runnable_path, tag)
        VALUES ($1, 'test-workspace', 'test-user', 'u/test-user', 'test@windmill.dev',
            'script', 'deno', 'u/test-user/agent', 'deno')",
    )
    .bind(admin_job)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO job_perms (job_id, email, username, is_admin, is_operator, folders, groups,
            workspace_id, job_token_scopes)
        VALUES ($1, 'test@windmill.dev', 'test-user', true, false, '{}', '{}', 'test-workspace',
            '{jobs:run}')",
    )
    .bind(admin_job)
    .execute(&db)
    .await?;
    let admin_token = windmill_common::auth::create_token_for_owner(
        &db,
        "test-workspace",
        "u/test-user",
        "ephemeral-script",
        300,
        "test@windmill.dev",
        &admin_job,
        None,
        None,
    )
    .await?;
    let resp = client
        .post(format!("{base}/jobs/queue/import"))
        .bearer_auth(&admin_token)
        .json(&json!([]))
        .send()
        .await?;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "{}", resp.text().await?);
    // Nor can it, even an admin's, place a child in an unrelated run: the flow-run routes
    // trust that lineage.
    let resp = client
        .post(format!(
            "{base}/jobs/run/p/u/test-user-3/open?root_job={RUN_JOB}"
        ))
        .bearer_auth(&admin_token)
        .json(&json!({}))
        .send()
        .await?;
    assert_eq!(resp.status(), StatusCode::CREATED, "{}", resp.text().await?);
    let child = Uuid::parse_str(&resp.text().await?)?;
    let claimed: bool = sqlx::query_scalar(
        "SELECT $2 IN (parent_job, root_job, flow_innermost_root_job) IS TRUE FROM v2_job
        WHERE id = $1",
    )
    .bind(child)
    .bind(Uuid::parse_str(RUN_JOB)?)
    .fetch_one(&db)
    .await?;
    assert!(!claimed);

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
            sqlx::query_scalar("SELECT job_token_scopes FROM job_perms WHERE job_id = $1")
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
    let value = json!({ "modules": [
        { "id": "a", "value": { "type": "identity" },
          "job_token_scopes": ["oidc:write", "variables:read"] },
        { "id": "b", "value": { "type": "identity" } },
    ] });
    sqlx::query(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at,
            schema, extra_perms, versions)
        VALUES ('test-workspace', 'u/test-user/agent_flow', '', '', $1, 'test-user', now(), '{}',
            '{}', '{7171}')",
    )
    .bind(&value)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by)
        VALUES (7171, 'test-workspace', 'u/test-user/agent_flow', $1, '{}', 'test-user')",
    )
    .bind(&value)
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let scopes = |v: &[&str]| Some(v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    // A step's own setting narrows the flow's restriction and never widens it; in an
    // unrestricted flow it restricts that step alone.
    for (flow_scopes, expected_a, expected_b) in [
        (scopes(&["oidc:write"]), scopes(&["oidc:write"]), scopes(&["oidc:write"])),
        (None, scopes(&["oidc:write", "variables:read"]), None),
    ] {
        let flow = RunJob::from(JobPayload::Flow {
            path: "u/test-user/agent_flow".to_string(),
            dedicated_worker: None,
            apply_preprocessor: false,
            version: 7171,
            labels: None,
            job_token_scopes: flow_scopes.clone(),
        })
        .run_until_complete(&db, false, server.addr.port())
        .await;

        let steps: Vec<(String, Option<Vec<String>>)> = sqlx::query_as(
            "SELECT j.flow_step_id, p.job_token_scopes FROM v2_job j
            JOIN job_perms p ON p.job_id = j.id
            WHERE j.parent_job = $1 ORDER BY j.flow_step_id",
        )
        .bind(flow.id)
        .fetch_all(&db)
        .await?;
        assert_eq!(
            steps,
            vec![("a".to_string(), expected_a), ("b".to_string(), expected_b)],
            "flow scopes {flow_scopes:?}"
        );
    }
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_deploy_without_the_field_keeps_the_restriction(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api/w/test-workspace", server.addr.port());
    let client = reqwest::Client::new();
    let script = |path: &str, scopes: Option<serde_json::Value>| {
        let mut body = json!({
            "path": path, "summary": "", "description": "", "content": "echo 42", "language": "bash",
        });
        if let Some(scopes) = scopes {
            body["job_token_scopes"] = scopes;
        }
        body
    };
    let stored = |path: &'static str| {
        let db = db.clone();
        async move {
            sqlx::query_scalar::<_, Option<Vec<String>>>(
                "SELECT job_token_scopes FROM script WHERE path = $1 ORDER BY created_at DESC LIMIT 1",
            )
            .bind(path)
            .fetch_one(&db)
            .await
        }
    };

    let resp = client
        .post(format!("{base}/scripts/create"))
        .bearer_auth("SECRET_TOKEN")
        .json(&script("u/test-user/agent", Some(json!(["oidc:write"]))))
        .send()
        .await?;
    assert_eq!(resp.status(), StatusCode::CREATED, "{}", resp.text().await?);

    // A rename by a client that does not know the field carries the restriction along.
    let resp = client
        .post(format!("{base}/scripts/update/u/test-user/agent"))
        .bearer_auth("SECRET_TOKEN")
        .json(&script("u/test-user/renamed", None))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    assert_eq!(stored("u/test-user/renamed").await?, Some(vec!["oidc:write".to_string()]));

    // An explicit null clears it.
    let resp = client
        .post(format!("{base}/scripts/update/u/test-user/renamed"))
        .bearer_auth("SECRET_TOKEN")
        .json(&script("u/test-user/renamed", Some(serde_json::Value::Null)))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    assert_eq!(stored("u/test-user/renamed").await?, None);
    Ok(())
}
