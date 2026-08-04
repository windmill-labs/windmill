use sqlx::{Pool, Postgres};
use windmill_common::auth::fetch_authed_from_permissioned_as;

/// The address handed to `fetch_authed_from_permissioned_as` may come from a cache that a
/// username reassignment has outrun. It must not be believed: the workspace role is keyed on the
/// principal while `super_admin` and `email_to_igroup` are keyed on the address, so trusting a
/// stale one would run the new holder's job with the previous holder's instance privileges.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_stale_address_cannot_carry_the_previous_holders_privileges(db: Pool<Postgres>) {
    // `test-user` in the fixture is a superadmin with the address `test@windmill.dev`. Free the
    // username and hand it to somebody who is not, exactly as an offboard-then-onboard would.
    sqlx::query("DELETE FROM usr WHERE workspace_id = 'test-workspace' AND username = 'test-user'")
        .execute(&db)
        .await
        .expect("free the username");
    sqlx::query(
        "INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
         VALUES ('newcomer@windmill.dev', 'x', 'password', false, true, 'Newcomer')",
    )
    .execute(&db)
    .await
    .expect("create the new account");
    sqlx::query(
        "INSERT INTO usr(workspace_id, email, username, is_admin, role)
         VALUES ('test-workspace', 'newcomer@windmill.dev', 'test-user', false, 'User')",
    )
    .execute(&db)
    .await
    .expect("reassign the username");

    // What a replica that has not yet consumed the eviction would pass: the principal is the
    // reassigned username, the address is the one it cached for the previous holder.
    let authed = fetch_authed_from_permissioned_as(
        "u/test-user",
        "test@windmill.dev",
        "test-workspace",
        &db,
    )
    .await
    .expect("should authenticate the current holder");

    assert_eq!(
        authed.email, "newcomer@windmill.dev",
        "the principal's live address must win over the one supplied"
    );
    assert!(
        !authed.is_admin,
        "the new holder must not inherit the previous holder's superadmin"
    );
}
