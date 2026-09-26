//! `deployed` gates what job OIDC tokens let external trust policies accept, so a job
//! whose code or path the caller chose must never come out deployed.

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::job_provenance::job_provenance;

async fn deployed(db: &Pool<Postgres>, job: &str) -> bool {
    job_provenance(db, &Uuid::parse_str(job).unwrap(), "test-workspace")
        .await
        .unwrap()
        .unwrap()
        .deployed
}

#[sqlx::test(fixtures("base", "job_provenance"))]
async fn step_of_deployed_flow_is_deployed(db: Pool<Postgres>) {
    let p = job_provenance(
        &db,
        &Uuid::parse_str("3bb0c0de-0000-4000-8000-000000000002").unwrap(),
        "test-workspace",
    )
    .await
    .unwrap()
    .unwrap();
    assert!(p.deployed);
    assert_eq!(p.root_path.as_deref(), Some("f/t/agent"));
}

#[sqlx::test(fixtures("base", "job_provenance"))]
async fn step_under_flow_preview_is_not_deployed(db: Pool<Postgres>) {
    assert!(!deployed(&db, "3bb0c0de-0000-4000-8000-000000000004").await);
}

#[sqlx::test(fixtures("base", "job_provenance"))]
async fn flow_running_another_flows_version_is_not_deployed(db: Pool<Postgres>) {
    assert!(!deployed(&db, "3bb0c0de-0000-4000-8000-000000000006").await);
}
