//! A script's on-behalf-of identity must drive the permissions of the jobs it
//! produces, not just their email. Rows written before the identity was recorded
//! keep falling back to the deployer, so upgrading an instance never widens what
//! an existing script can reach.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

/// Deploys as `test-user` (admin) so the recorded identity is nobody's default:
/// neither the caller's nor the deployer's.
async fn create_script(
    base: &str,
    path: &str,
    on_behalf_of_permissioned_as: Option<&str>,
) -> anyhow::Result<String> {
    let mut body = json!({
        "path": path,
        "summary": "",
        "description": "",
        "content": "export async function main() { return 42; }",
        "language": "deno",
        "on_behalf_of_email": "original@windmill.dev",
        "preserve_on_behalf_of": true,
        "auto_parent": true,
    });
    if let Some(permissioned_as) = on_behalf_of_permissioned_as {
        body["on_behalf_of_permissioned_as"] = json!(permissioned_as);
    }
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&body)
    .send()
    .await?;
    let status = resp.status();
    let hash = resp.text().await?;
    assert_eq!(status, 201, "creating {path}: {hash}");
    // The create endpoint renders the hash as hex, and that is also the form the
    // run-by-hash route parses, so keep it as a string.
    Ok(hash.trim().trim_matches('"').to_string())
}

async fn run_by_hash(base: &str, hash: &str) -> anyhow::Result<uuid::Uuid> {
    let resp = authed(
        client().post(format!("{base}/jobs/run/h/{hash}")),
        "SECRET_TOKEN",
    )
    .json(&json!({}))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 201, "running {hash}: {body}");
    Ok(uuid::Uuid::parse_str(body.trim().trim_matches('"'))?)
}

#[sqlx::test(fixtures("preserve_on_behalf_of"))]
async fn test_on_behalf_of_permissioned_as_drives_job_identity(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );

    let recorded =
        create_script(&base, "u/test-user/obo_recorded", Some("u/original-user")).await?;
    let legacy = create_script(&base, "u/test-user/obo_legacy", None).await?;

    let stored = sqlx::query!(
        "SELECT path, on_behalf_of_email, on_behalf_of_permissioned_as, created_by FROM script \
         WHERE hash = ANY($1) AND workspace_id = 'test-workspace' ORDER BY path",
        &[
            i64::from_str_radix(&recorded, 16)? as i64,
            i64::from_str_radix(&legacy, 16)? as i64
        ][..]
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(
        stored
            .iter()
            .map(|s| (
                s.path.as_str(),
                s.on_behalf_of_permissioned_as.as_deref(),
                s.created_by.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("u/test-user/obo_legacy", None, "test-user"),
            (
                "u/test-user/obo_recorded",
                Some("u/original-user"),
                "test-user"
            ),
        ],
    );

    let recorded_job = run_by_hash(&base, &recorded).await?;
    let legacy_job = run_by_hash(&base, &legacy).await?;

    let jobs = sqlx::query!(
        "SELECT id, permissioned_as, permissioned_as_email FROM v2_job WHERE id = ANY($1)",
        &[recorded_job, legacy_job][..]
    )
    .fetch_all(&db)
    .await?;
    let identity = |id: uuid::Uuid| {
        let job = jobs.iter().find(|j| j.id == id).expect("job was pushed");
        (
            job.permissioned_as.clone(),
            job.permissioned_as_email.clone(),
        )
    };

    assert_eq!(
        identity(recorded_job),
        (
            "u/original-user".to_string(),
            "original@windmill.dev".to_string()
        ),
        "a recorded permissioned_as must be what the job runs as"
    );
    assert_eq!(
        identity(legacy_job),
        (
            "u/test-user".to_string(),
            "original@windmill.dev".to_string()
        ),
        "without a recorded permissioned_as the job keeps running as the deployer"
    );

    // A client that preserves but predates the field sends the email alone. Inheriting
    // the recorded identity is what stops a routine redeploy from silently handing the
    // script back to whoever deployed it.
    let redeployed_hash = create_script(&base, "u/test-user/obo_recorded", None).await?;
    assert_ne!(redeployed_hash, recorded, "the redeploy must be a new version");
    let redeployed = sqlx::query_scalar!(
        "SELECT on_behalf_of_permissioned_as FROM script \
         WHERE path = 'u/test-user/obo_recorded' AND workspace_id = 'test-workspace' AND NOT archived"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(redeployed.as_deref(), Some("u/original-user"));

    Ok(())
}
