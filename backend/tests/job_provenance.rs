//! `deployed` gates what job OIDC tokens let external trust policies accept, so a job
//! whose code or path the caller chose must never come out deployed.

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::{
    job_provenance::{job_provenance, ClaimedItem, ClaimedPath, JobProvenance},
    scripts::ScriptHash,
};

async fn provenance(db: &Pool<Postgres>, job: &str) -> JobProvenance {
    job_provenance(db, &Uuid::parse_str(job).unwrap(), "test-workspace")
        .await
        .unwrap()
        .unwrap()
}

async fn deployed(db: &Pool<Postgres>, job: &str) -> bool {
    provenance(db, job).await.deployed
}

async fn claims(db: &Pool<Postgres>, job: &str) -> Vec<(String, ClaimedItem)> {
    provenance(db, job)
        .await
        .claimed_paths
        .into_iter()
        .map(|ClaimedPath { path, item, .. }| (path, item))
        .collect()
}

fn claim(path: &str, item: ClaimedItem) -> (String, ClaimedItem) {
    (path.to_string(), item)
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

#[sqlx::test(fixtures("base", "job_provenance"))]
async fn app_script_is_deployed_only_from_a_deployed_app_run(db: Pool<Postgres>) {
    assert!(deployed(&db, "3bb0c0de-0000-4000-8000-000000000007").await);
    assert!(deployed(&db, "3bb0c0de-0000-4000-8000-000000000010").await);
    assert!(!deployed(&db, "3bb0c0de-0000-4000-8000-000000000008").await);
    assert_eq!(
        claims(&db, "3bb0c0de-0000-4000-8000-000000000008").await,
        [claim("f/t/app", ClaimedItem::App)]
    );
}

/// Inline steps of a flow deployed before flow nodes run as previews, at paths the
/// worker derives under the flow's.
#[sqlx::test(fixtures("base", "job_provenance"))]
async fn preview_step_of_deployed_flow_is_deployed_within_its_path(db: Pool<Postgres>) {
    for step in ["09", "0b"] {
        let p = provenance(&db, &format!("3bb0c0de-0000-4000-8000-0000000000{step}")).await;
        assert!(p.deployed && p.claimed_paths.is_empty(), "step {step}");
    }
    assert!(!deployed(&db, "3bb0c0de-0000-4000-8000-00000000000c").await);
    assert!(!deployed(&db, "3bb0c0de-0000-4000-8000-000000000011").await);
    assert_eq!(
        claims(&db, "3bb0c0de-0000-4000-8000-00000000000c").await,
        [claim("f/t/agentx", ClaimedItem::Script)]
    );
}

/// A token acts as every path its chain took from a request: the top-most preview, and
/// any step whose path leaves its parent's.
#[sqlx::test(fixtures("base", "job_provenance"))]
async fn claimed_paths_are_the_request_supplied_origins(db: Pool<Postgres>) {
    let flow = || claim("f/t/agent", ClaimedItem::Flow);
    assert_eq!(claims(&db, "3bb0c0de-0000-4000-8000-000000000004").await, [flow()]);
    assert_eq!(claims(&db, "3bb0c0de-0000-4000-8000-00000000000e").await, [flow()]);
    assert_eq!(
        claims(&db, "3bb0c0de-0000-4000-8000-00000000000f").await,
        [flow(), claim("f/prod/deploy", ClaimedItem::Script)]
    );
    assert_eq!(claims(&db, "3bb0c0de-0000-4000-8000-000000000006").await, [flow()]);
    assert!(claims(&db, "3bb0c0de-0000-4000-8000-000000000002").await.is_empty());
    // A parent whose path names no item cannot cover the claims below it.
    assert_eq!(
        claims(&db, "3bb0c0de-0000-4000-8000-000000000013").await,
        [claim("f/prod/deploy", ClaimedItem::Script)]
    );
}

/// Any past version can be run by hash or flow version id: it stays deployed, but only
/// the versions the paths currently resolve to are latest.
#[sqlx::test(fixtures("base", "job_provenance", "job_provenance_versions"))]
async fn only_current_versions_are_latest(db: Pool<Postgres>) {
    let current = provenance(&db, "3bb0c0de-0000-4000-8000-000000000002").await;
    assert!(current.deployed && current.latest);
    let current_hash = ScriptHash(1111111111).to_string();
    assert_eq!(current.version.as_deref(), Some(current_hash.as_str()));
    assert_eq!(current.root_version.as_deref(), Some("2222222222"));

    let past_tool = provenance(&db, "3bb0c0de-0000-4000-8000-000000000101").await;
    assert!(past_tool.deployed && !past_tool.latest);

    let under_past_flow = provenance(&db, "3bb0c0de-0000-4000-8000-000000000103").await;
    assert!(under_past_flow.deployed && !under_past_flow.latest);
    assert_eq!(under_past_flow.root_version.as_deref(), Some("2222222221"));

    assert!(!deployed(&db, "3bb0c0de-0000-4000-8000-000000000104").await);
}

/// Flow steps run what the version of the flow above them defines, except under a
/// restart, which rebuilds a body from a past run, under the current flow version or with
/// no flow above it at all.
#[sqlx::test(fixtures("base", "job_provenance", "job_provenance_versions"))]
async fn flow_steps_are_latest_only_under_a_current_unrestarted_flow(db: Pool<Postgres>) {
    for (job, latest) in [
        ("105", true),
        ("106", true),
        ("107", true),
        ("108", false),
        ("109", false),
        ("110", false),
        ("111", false),
        ("112", false),
    ] {
        let p = provenance(&db, &format!("3bb0c0de-0000-4000-8000-000000000{job}")).await;
        assert!(p.deployed, "job {job}");
        assert_eq!(p.latest, latest, "job {job}");
    }
}
