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

/// A disabled member still holds its username in the workspace. Workspace usernames are only
/// unique per workspace, so an unrelated instance superadmin can share it, and falling through to
/// the `password` fallback would run the disabled member's jobs as that superadmin.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_disabled_member_never_resolves_to_a_same_named_superadmin(db: Pool<Postgres>) {
    sqlx::query(
        "UPDATE usr SET disabled = true WHERE workspace_id = 'test-workspace' AND username = 'test-user-2'",
    )
    .execute(&db)
    .await
    .expect("disable the member");
    sqlx::query(
        "INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
         VALUES ('other-superadmin@windmill.dev', 'x', 'password', true, true, 'Other', 'test-user-2')",
    )
    .execute(&db)
    .await
    .expect("create the same-named superadmin");

    for supplied in ["test2@windmill.dev", "other-superadmin@windmill.dev"] {
        let authed =
            fetch_authed_from_permissioned_as("u/test-user-2", supplied, "test-workspace", &db)
                .await;
        assert!(
            authed.is_err(),
            "a disabled member must not authenticate (supplied {supplied}): {:?}",
            authed.map(|a| (a.email, a.is_admin))
        );
    }
}
