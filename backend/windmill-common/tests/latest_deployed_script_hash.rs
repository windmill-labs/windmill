use sqlx::{Pool, Postgres};
use windmill_common::get_latest_deployed_script_hash;

const WORKSPACE: &str = "test-workspace";

async fn deploy_version(db: &Pool<Postgres>, path: &str, hash: i64, lock: Option<&str>, age: f64) {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, \
         language, kind, lock, created_at) \
         VALUES ($1, $2, $3, '', '', 'def main(): pass', 'test-user', 'python3', 'script', $4, \
         now() - make_interval(secs => $5))",
    )
    .bind(WORKSPACE)
    .bind(hash)
    .bind(path)
    .bind(lock)
    .bind(age)
    .execute(db)
    .await
    .expect("failed to deploy script version");
}

/// A version whose dependency job has not written its lockfile yet is not runnable, so the answer
/// stays on the version before it. Callers cache that answer, and only `pending_lock` tells them
/// the dependency job is about to change it under them.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn pending_lock_flags_an_answer_a_dependency_job_is_about_to_change(db: Pool<Postgres>) {
    let path = "f/test/probe";
    deploy_version(&db, path, 1, Some(""), 2.0).await;

    let latest = get_latest_deployed_script_hash(&db, path, WORKSPACE)
        .await
        .unwrap();
    assert_eq!(latest.hash, Some(1));
    assert!(!latest.pending_lock);

    deploy_version(&db, path, 2, None, 1.0).await;

    let latest = get_latest_deployed_script_hash(&db, path, WORKSPACE)
        .await
        .unwrap();
    assert_eq!(latest.hash, Some(1), "an unlocked version is not runnable");
    assert!(latest.pending_lock, "but it is about to become the answer");

    sqlx::query("UPDATE script SET lock = '' WHERE workspace_id = $1 AND hash = 2")
        .bind(WORKSPACE)
        .execute(&db)
        .await
        .expect("failed to write the lockfile");

    let latest = get_latest_deployed_script_hash(&db, path, WORKSPACE)
        .await
        .unwrap();
    assert_eq!(latest.hash, Some(2));
    assert!(!latest.pending_lock);
}

/// A version that will never become runnable must not keep the path on the short TTL: its
/// dependency job either reported a failure, or is old enough that it is not coming.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn pending_lock_gives_up_on_a_dependency_job_that_will_not_land(db: Pool<Postgres>) {
    let failed = "f/test/failed";
    deploy_version(&db, failed, 1, Some(""), 2.0).await;
    deploy_version(&db, failed, 2, None, 1.0).await;
    sqlx::query("UPDATE script SET lock_error_logs = 'boom' WHERE workspace_id = $1 AND hash = 2")
        .bind(WORKSPACE)
        .execute(&db)
        .await
        .expect("failed to report the lock error");

    let latest = get_latest_deployed_script_hash(&db, failed, WORKSPACE)
        .await
        .unwrap();
    assert_eq!(latest.hash, Some(1));
    assert!(!latest.pending_lock);

    let abandoned = "f/test/abandoned";
    deploy_version(&db, abandoned, 3, Some(""), 3600.0).await;
    deploy_version(&db, abandoned, 4, None, 1800.0).await;

    let latest = get_latest_deployed_script_hash(&db, abandoned, WORKSPACE)
        .await
        .unwrap();
    assert_eq!(latest.hash, Some(3));
    assert!(!latest.pending_lock);
}
