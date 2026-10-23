//! A workspace job pause holds new top-level jobs and the queued ones that have not started, and
//! resume releases exactly those: each when it was due, and not the jobs a suspended trigger holds.

use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_test_utils::*;

const ADMIN: &str = "SECRET_TOKEN";
const MEMBER: &str = "SECRET_TOKEN_2";

async fn kind_and_due(db: &Pool<Postgres>, id: Uuid) -> (String, chrono::DateTime<chrono::Utc>) {
    sqlx::query_as(
        "SELECT j.kind::text, q.scheduled_for FROM v2_job j JOIN v2_job_queue q USING (id)
         WHERE j.id = $1",
    )
    .bind(id)
    .fetch_one(db)
    .await
    .unwrap()
}

#[sqlx::test(fixtures("base"))]
async fn pause_holds_and_resume_releases(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let ws = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    let http = reqwest::Client::new();
    let post = |url: String, token: &str| http.post(url).bearer_auth(token);

    let resp = post(format!("{ws}/scripts/create"), MEMBER)
        .json(&json!({
            "path": "u/test-user-2/s", "summary": "", "description": "",
            "content": "echo hi", "language": "bash",
            "schema": {"type": "object", "properties": {}, "required": []}
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    // Runs as the admin whoever starts it.
    let resp = post(format!("{ws}/scripts/create"), ADMIN)
        .json(&json!({
            "path": "u/test-user/obo", "summary": "", "description": "",
            "content": "echo hi", "language": "bash", "on_behalf_of": "u/test-user",
            "schema": {"type": "object", "properties": {}, "required": []}
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    let resp = post(format!("{ws}/acls/add/script/u/test-user/obo"), ADMIN)
        .json(&json!({"owner": "u/test-user-2", "write": false}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let run = |token: &'static str, query: &str| {
        post(format!("{ws}/jobs/run/p/u/test-user-2/s{query}"), token).json(&json!({}))
    };
    let job_id = |resp: reqwest::Response| async move {
        let status = resp.status();
        let body = resp.text().await.unwrap();
        assert_eq!(status, 201, "{body}");
        body.parse::<Uuid>().unwrap()
    };

    // Queued before the pause, due later: the pause holds it and resume keeps its time.
    let due = chrono::Utc::now() + chrono::Duration::hours(1);
    let scheduled = job_id(
        run(
            MEMBER,
            &format!(
                "?scheduled_for={}",
                due.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            ),
        )
        .send()
        .await?,
    )
    .await;
    // Held by a suspended trigger, not by the pause.
    let suspended = job_id(run(MEMBER, "?suspended_mode=true").send().await?).await;

    let resp = post(format!("{ws}/workspaces/pause_jobs"), MEMBER)
        .json(&json!({"duration_s": 600}))
        .send()
        .await?;
    assert_eq!(resp.status(), 403, "pausing is admin only");
    let resp = post(format!("{ws}/workspaces/pause_jobs"), ADMIN)
        .json(&json!({"duration_s": 600}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    assert_eq!(kind_and_due(&db, scheduled).await.0, "unassigned_script");

    let held = job_id(run(MEMBER, "").send().await?).await;
    assert_eq!(kind_and_due(&db, held).await.0, "unassigned_script");

    // A member cannot step around the pause by naming a parent it does not own.
    let forged = job_id(
        run(MEMBER, &format!("?parent_job={}", Uuid::new_v4()))
            .send()
            .await?,
    )
    .await;
    assert_eq!(kind_and_due(&db, forged).await.0, "unassigned_script");

    // Running a script on behalf of an admin does not make the run an admin's.
    let obo = job_id(
        post(format!("{ws}/jobs/run/p/u/test-user/obo"), MEMBER)
            .json(&json!({}))
            .send()
            .await?,
    )
    .await;
    let run_as: String = sqlx::query_scalar("SELECT permissioned_as FROM v2_job WHERE id = $1")
        .bind(obo)
        .fetch_one(&db)
        .await?;
    assert_eq!(run_as, "u/test-user");
    assert_eq!(kind_and_due(&db, obo).await.0, "unassigned_script");

    // An admin's direct run is not held.
    let direct = job_id(run(ADMIN, "").send().await?).await;
    assert_eq!(kind_and_due(&db, direct).await.0, "script");

    // A caller waiting for the result is refused rather than left waiting.
    let resp = post(
        format!("{ws}/jobs/run_wait_result/p/u/test-user-2/s"),
        MEMBER,
    )
    .json(&json!({}))
    .send()
    .await?;
    assert_eq!(resp.status(), 503);
    assert!(resp.headers().contains_key("retry-after"));

    let resp = post(format!("{ws}/workspaces/resume_jobs"), ADMIN)
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<i64>().await?, 4);

    for id in [held, forged] {
        let (kind, at) = kind_and_due(&db, id).await;
        assert_eq!(kind, "script");
        assert!(at <= chrono::Utc::now());
    }
    let (kind, at) = kind_and_due(&db, scheduled).await;
    assert_eq!(kind, "script");
    assert_eq!(
        at.timestamp(),
        due.timestamp(),
        "a released job keeps when it was due"
    );
    assert_eq!(kind_and_due(&db, suspended).await.0, "unassigned_script");
    Ok(())
}
