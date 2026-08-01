//! A script's on-behalf-of identity must drive the permissions of the jobs it produces, not
//! just their address. The principal is the only stored half; the address is derived from it,
//! so a request naming one, the other, or a mismatched pair all resolve to one account.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn script_body(path: &str, on_behalf_of_permissioned_as: Option<&str>) -> serde_json::Value {
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
    body
}

/// Deploys as `test-user` (admin) so the recorded identity is nobody's default: neither
/// the caller's nor the deployer's. Returns the hex hash the run-by-hash route parses.
async fn create_script(
    base: &str,
    path: &str,
    on_behalf_of_permissioned_as: Option<&str>,
) -> anyhow::Result<String> {
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&script_body(path, on_behalf_of_permissioned_as))
    .send()
    .await?;
    let status = resp.status();
    let hash = resp.text().await?;
    assert_eq!(status, 201, "creating {path}: {hash}");
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

async fn stored_permissioned_as(
    db: &Pool<Postgres>,
    table: &str,
    path: &str,
) -> anyhow::Result<Option<String>> {
    // `table` is a literal from this test, never caller input.
    Ok(sqlx::query_scalar(&format!(
        "SELECT on_behalf_of_permissioned_as FROM {table} \
         WHERE path = $1 AND workspace_id = 'test-workspace' AND NOT archived"
    ))
    .bind(path)
    .fetch_one(db)
    .await?)
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
    assert_eq!(
        stored_permissioned_as(&db, "script", "u/test-user/obo_recorded")
            .await?
            .as_deref(),
        Some("u/original-user")
    );

    // A client that predates the field names only the email; deriving the principal from
    // it is what stops a routine redeploy from handing the script to whoever deploys it.
    let derived = create_script(&base, "u/test-user/obo_derived", None).await?;
    assert_eq!(
        stored_permissioned_as(&db, "script", "u/test-user/obo_derived")
            .await?
            .as_deref(),
        Some("u/original-user")
    );

    let recorded_job = run_by_hash(&base, &recorded).await?;
    let derived_job = run_by_hash(&base, &derived).await?;

    let jobs = sqlx::query!(
        "SELECT id, permissioned_as, permissioned_as_email FROM v2_job WHERE id = ANY($1)",
        &[recorded_job, derived_job][..]
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
        identity(derived_job),
        (
            "u/original-user".to_string(),
            "original@windmill.dev".to_string()
        ),
        "a principal derived from the address drives the job the same way an explicit one does"
    );

    // A superadmin acting outside their workspaces has no `usr` row. Dropping them on an
    // email-only redeploy would keep their superadmin email next to the deployer's
    // permissions — the hybrid identity this whole change exists to remove.
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user/obo_superadmin",
        "summary": "",
        "description": "",
        "content": "export async function main() { return 42; }",
        "language": "deno",
        "on_behalf_of_email": "superadmin-external@windmill.dev",
        "preserve_on_behalf_of": true,
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "creating: {}", resp.text().await?);
    assert_eq!(
        stored_permissioned_as(&db, "script", "u/test-user/obo_superadmin")
            .await?
            .as_deref(),
        Some("u/superadmin-external")
    );

    // The synthetic group namespace is not reserved, so a real account holding such an
    // address must win over the like-named group — otherwise an email-only deploy would
    // hand the runnable that group's folder access.
    sqlx::query!(
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms) \
         VALUES ('test-workspace', 'ops', '', '{}') ON CONFLICT DO NOTHING"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "UPDATE usr SET email = 'group-ops@windmill.dev' WHERE workspace_id = 'test-workspace' \
         AND username = 'test-user-2'"
    )
    .execute(&db)
    .await?;
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user/obo_group_collision",
        "summary": "",
        "description": "",
        "content": "export async function main() { return 42; }",
        "language": "deno",
        "on_behalf_of_email": "group-ops@windmill.dev",
        "preserve_on_behalf_of": true,
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "creating: {}", resp.text().await?);
    assert_eq!(
        stored_permissioned_as(&db, "script", "u/test-user/obo_group_collision")
            .await?
            .as_deref(),
        Some("u/test-user-2"),
        "a real account must win over the like-named group"
    );

    // A pair naming two different principals would run as a composite of both.
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&script_body(
        "u/test-user/obo_mismatch",
        Some("u/test-user-2"),
    ))
    .send()
    .await?;
    assert_eq!(resp.status(), 400, "a mismatched pair must be rejected");

    // Flows resolve the same way, but through their own UPDATE — which must not drop the
    // principal when the body names only the email.
    let flow = json!({
        "path": "u/test-user/obo_flow",
        "summary": "",
        "value": { "modules": [] },
        "on_behalf_of_email": "original@windmill.dev",
        "on_behalf_of_permissioned_as": "u/original-user",
        "preserve_on_behalf_of": true,
    });
    let resp = authed(
        client().post(format!("{base}/flows/create")),
        "SECRET_TOKEN",
    )
    .json(&flow)
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "creating flow: {}", resp.text().await?);

    let mut update = flow.clone();
    update["summary"] = json!("edited");
    update
        .as_object_mut()
        .unwrap()
        .remove("on_behalf_of_permissioned_as");
    let resp = authed(
        client().post(format!("{base}/flows/update/u/test-user/obo_flow")),
        "SECRET_TOKEN",
    )
    .json(&update)
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "updating flow: {}", resp.text().await?);
    assert_eq!(
        stored_permissioned_as(&db, "flow", "u/test-user/obo_flow")
            .await?
            .as_deref(),
        Some("u/original-user"),
        "an update that names only the email must not drop the flow's principal"
    );

    Ok(())
}
