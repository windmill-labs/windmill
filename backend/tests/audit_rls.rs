//! A non-admin's transaction must accept an audit row recorded under another username, as a
//! job pushed to run as someone else writes one. A rejected insert aborts the transaction and
//! takes the caller's own write down with it.

use sqlx::{Pool, Postgres};

use windmill_test_utils::ensure_today_audit_partition;

#[sqlx::test]
async fn non_admin_writes_an_audit_row_for_another_user(db: Pool<Postgres>) -> anyhow::Result<()> {
    ensure_today_audit_partition(&db).await;

    let mut tx = db.begin().await?;
    sqlx::query("SET LOCAL ROLE windmill_user")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT set_config('session.user', 'bob', true)")
        .execute(&mut *tx)
        .await?;
    // The shape a schedule run as alice records without an Enterprise license: parameters
    // redacted, so no `end_user` names the schedule.
    sqlx::query(
        "INSERT INTO audit_partitioned
        (workspace_id, username, operation, action_kind, resource, parameters, email)
        VALUES ('admins', 'alice', 'redacted', 'execute', 'EE only', '{\"redacted\": \"-\"}', 'alice@windmill.dev')",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_partitioned WHERE username = 'alice'")
            .fetch_one(&db)
            .await?;
    assert_eq!(count, 1);
    Ok(())
}
