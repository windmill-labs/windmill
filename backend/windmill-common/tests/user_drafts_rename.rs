use sqlx::{Pool, Postgres};
use windmill_common::user_drafts::rename_drafts_of_email;

/// A rename onto the same address has to be a no-op: the helper clears a draft the destination
/// already holds at the same item, and every row would be its own destination. SCIM PATCH sends
/// `userName` unconditionally, so an IdP re-sending an unchanged one reaches this.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn renaming_onto_the_same_address_keeps_the_drafts(db: Pool<Postgres>) {
    sqlx::query(
        "INSERT INTO draft(workspace_id, path, typ, value, email) \
         VALUES ('test-workspace', 'u/test-user/s', 'script', '{}'::json, 'test@windmill.dev')",
    )
    .execute(&db)
    .await
    .expect("failed to seed draft");

    let mut conn = db.acquire().await.unwrap();
    rename_drafts_of_email(&mut conn, "test@windmill.dev", "test@windmill.dev")
        .await
        .unwrap();

    let kept: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM draft WHERE email = 'test@windmill.dev'")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(kept, 1);
}
