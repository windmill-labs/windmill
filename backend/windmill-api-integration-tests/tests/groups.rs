use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn group_url(port: u16, endpoint: &str, name: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/groups/{endpoint}/{name}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_group_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/groups");

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "name": "test_group",
            "summary": "A test group"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create: {}", resp.text().await?);

    // create second group
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "name": "another_group",
            "summary": "Another group"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create another: {}", resp.text().await?);

    // create duplicate -> error
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "name": "test_group",
            "summary": "Duplicate"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // --- get ---
    let resp = authed(client().get(group_url(port, "get", "test_group")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["name"], "test_group");
    assert_eq!(body["summary"], "A test group");

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().any(|g| g["name"] == "test_group"));

    // --- listnames ---
    let resp = authed(client().get(format!("{base}/listnames")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let names = resp.json::<Vec<String>>().await?;
    assert!(names.contains(&"test_group".to_string()));

    // --- update ---
    let resp = authed(client().post(group_url(port, "update", "test_group")))
        .json(&json!({"summary": "Updated summary"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed(client().get(group_url(port, "get", "test_group")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated summary");

    // --- adduser ---
    let resp = authed(client().post(group_url(port, "adduser", "test_group")))
        .json(&json!({"username": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "adduser: {}", resp.text().await?);

    // verify membership
    let resp = authed(client().get(group_url(port, "get", "test_group")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    let members = body["members"].as_array().unwrap();
    assert!(
        members.iter().any(|m| m.as_str() == Some("test-user")),
        "expected test-user in members, got: {:?}",
        members
    );

    // --- removeuser ---
    let resp = authed(client().post(group_url(port, "removeuser", "test_group")))
        .json(&json!({"username": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- is_owner ---
    let resp = authed(client().get(group_url(port, "is_owner", "test_group")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    // --- delete ---
    let resp = authed(client().delete(group_url(port, "delete", "another_group")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // verify deleted - get should 404 or the group shouldn't appear in list
    let resp = authed(client().get(format!("{base}/listnames")))
        .send()
        .await
        .unwrap();
    let names = resp.json::<Vec<String>>().await?;
    assert!(!names.contains(&"another_group".to_string()));

    // ===== Global (instance group) endpoints =====
    let global_base = format!("http://localhost:{port}/api/groups");

    // --- create instance group ---
    let resp = authed(client().post(format!("{global_base}/create")))
        .json(&json!({"name": "test_igroup", "summary": "Test instance group"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create igroup: {}", resp.text().await?);

    // --- list instance groups ---
    let resp = authed(client().get(format!("{global_base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().any(|g| g["name"] == "test_igroup"));

    // --- list_with_workspaces ---
    let resp = authed(client().get(format!("{global_base}/list_with_workspaces")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- get instance group ---
    let resp = authed(client().get(format!("{global_base}/get/test_igroup")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["name"], "test_igroup");
    assert_eq!(body["summary"], "Test instance group");

    // --- update instance group ---
    let resp = authed(client().post(format!("{global_base}/update/test_igroup")))
        .json(&json!({"new_summary": "Updated instance group"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update igroup: {}", resp.text().await?);

    // verify update
    let resp = authed(client().get(format!("{global_base}/get/test_igroup")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated instance group");

    // --- adduser to instance group ---
    let resp = authed(client().post(format!("{global_base}/adduser/test_igroup")))
        .json(&json!({"email": "test@windmill.dev"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "adduser igroup: {}", resp.text().await?);

    // verify membership
    let resp = authed(client().get(format!("{global_base}/get/test_igroup")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    let emails = body["emails"].as_array().unwrap();
    assert!(
        emails
            .iter()
            .any(|e| e.as_str() == Some("test@windmill.dev")),
        "expected test@windmill.dev in emails, got: {:?}",
        emails
    );

    // --- removeuser from instance group ---
    let resp = authed(client().post(format!("{global_base}/removeuser/test_igroup")))
        .json(&json!({"email": "test@windmill.dev"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- export (EE-gated) ---
    let resp = authed(client().get(format!("{global_base}/export")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "export igroups: unexpected status {}",
        resp.status()
    );

    // --- overwrite (EE-gated) ---
    let resp = authed(client().post(format!("{global_base}/overwrite")))
        .json(&json!([]))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "overwrite igroups: unexpected status {}",
        resp.status()
    );

    // --- delete instance group ---
    let resp = authed(client().delete(format!("{global_base}/delete/test_igroup")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "delete igroup: {}", resp.text().await?);

    // verify deleted
    let resp = authed(client().get(format!("{global_base}/list")))
        .send()
        .await
        .unwrap();
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.iter().any(|g| g["name"] == "test_igroup"));

    Ok(())
}

/// Deleting an instance group must not revoke workspace access a member still holds through
/// another configured group.
///
/// `added_via.group` records only the member's highest-precedence group, so any cleanup keyed
/// on that field alone evicts members who still qualify via a lower-precedence one. Membership
/// must be re-derived from all the groups the workspace still references.
#[cfg(feature = "private")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_delete_instance_group_preserves_access_via_other_group(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let global_base = format!("http://localhost:{port}/api/groups");
    let ws_base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    for g in ["igroup_a", "igroup_b"] {
        let resp = authed(client().post(format!("{global_base}/create")))
            .json(&json!({ "name": g, "summary": g }))
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "create {g}");
    }

    // multi@ belongs to both groups; only_a@ only to the group that gets deleted.
    for (g, email) in [
        ("igroup_a", "multi@example.com"),
        ("igroup_b", "multi@example.com"),
        ("igroup_a", "only_a@example.com"),
    ] {
        let resp = authed(client().post(format!("{global_base}/adduser/{g}")))
            .json(&json!({ "email": email }))
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "adduser {g}/{email}");
    }

    // igroup_a grants the higher-precedence role, so added_via lands on it.
    let resp = authed(client().post(format!("{ws_base}/edit_instance_groups")))
        .json(&json!({
            "groups": ["igroup_a", "igroup_b"],
            "roles": { "igroup_a": "admin", "igroup_b": "developer" }
        }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "edit_instance_groups: {}",
        resp.text().await?
    );

    let (is_admin, via): (bool, Option<String>) = sqlx::query_as(
        "SELECT is_admin, added_via->>'group' FROM usr
         WHERE workspace_id = 'test-workspace' AND email = 'multi@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert!(is_admin, "multi@ should start as admin via igroup_a");
    assert_eq!(via.as_deref(), Some("igroup_a"));

    // Workspace state that must survive the group removal. `delete_workspace_user_internal`
    // drops all of this, so a delete-and-re-add of a still-qualifying member loses it silently.
    let username: String = sqlx::query_scalar(
        "SELECT username FROM usr WHERE workspace_id = 'test-workspace' AND email = 'multi@example.com'",
    )
    .fetch_one(&db)
    .await?;
    sqlx::query(
        "INSERT INTO favorite (workspace_id, usr, path, favorite_kind)
         VALUES ('test-workspace', $1, 'f/keep/me', 'script')",
    )
    .bind(&username)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO draft (workspace_id, path, typ, value)
         VALUES ('test-workspace', 'u/' || $1 || '/keep', 'script', '{}'::jsonb)",
    )
    .bind(&username)
    .execute(&db)
    .await?;

    let resp = authed(client().delete(format!("{global_base}/delete/igroup_a")))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "delete igroup_a: {}",
        resp.text().await?
    );

    // Still a member, downgraded to igroup_b's role rather than evicted.
    let (is_admin, is_operator, via): (bool, bool, Option<String>) = sqlx::query_as(
        "SELECT is_admin, operator, added_via->>'group' FROM usr
         WHERE workspace_id = 'test-workspace' AND email = 'multi@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert!(!is_admin, "multi@ should lose admin with igroup_a gone");
    assert!(!is_operator, "igroup_b grants developer, not operator");
    assert_eq!(
        via.as_deref(),
        Some("igroup_b"),
        "added_via should re-point at the surviving group"
    );

    // Their workspace state is intact: they were never deleted and re-added.
    let favorites: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM favorite WHERE workspace_id = 'test-workspace' AND path = 'f/keep/me'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        favorites, 1,
        "favorite must survive losing a non-sole group"
    );
    let drafts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM draft WHERE workspace_id = 'test-workspace' AND path LIKE 'u/%/keep'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(drafts, 1, "draft must survive losing a non-sole group");

    // igroup_a was only_a@'s sole path in, so they are removed.
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM usr
         WHERE workspace_id = 'test-workspace' AND email = 'only_a@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(remaining, 0, "only_a@ should be removed with igroup_a");

    // The deleted group leaves no dangling reference in either auto_invite field.
    let (groups, roles): (serde_json::Value, serde_json::Value) = sqlx::query_as(
        "SELECT auto_invite->'instance_groups', auto_invite->'instance_groups_roles'
         FROM workspace_settings WHERE workspace_id = 'test-workspace'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(groups, json!(["igroup_b"]), "igroup_a should be stripped");
    assert_eq!(
        roles,
        json!({ "igroup_b": "developer" }),
        "igroup_a's role entry should be stripped"
    );

    Ok(())
}

/// Removing a member from one instance group must re-derive their role from the groups they
/// still belong to, not leave the privileges the removed group granted.
///
/// Still-qualifying members keep their `usr` row (deleting it would destroy their workspace
/// data), so the removal path must recompute that row's role — otherwise a member dropped
/// from an admin group keeps `is_admin` through the stale row.
#[cfg(feature = "private")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_remove_user_from_instance_group_rederives_role(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let global_base = format!("http://localhost:{port}/api/groups");
    let ws_base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    for g in ["role_a", "role_b"] {
        let resp = authed(client().post(format!("{global_base}/create")))
            .json(&json!({ "name": g }))
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "create {g}");
        let resp = authed(client().post(format!("{global_base}/adduser/{g}")))
            .json(&json!({ "email": "demoted@example.com" }))
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "adduser {g}");
    }

    let resp = authed(client().post(format!("{ws_base}/edit_instance_groups")))
        .json(&json!({
            "groups": ["role_a", "role_b"],
            "roles": { "role_a": "admin", "role_b": "developer" }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "edit: {}", resp.text().await?);

    let is_admin: bool = sqlx::query_scalar(
        "SELECT is_admin FROM usr WHERE workspace_id = 'test-workspace' AND email = 'demoted@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert!(is_admin, "should start admin via role_a");

    // Drop them from the admin group only.
    let resp = authed(client().post(format!("{global_base}/removeuser/role_a")))
        .json(&json!({ "email": "demoted@example.com" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "removeuser: {}", resp.text().await?);

    let (is_admin, is_operator, via): (bool, bool, Option<String>) = sqlx::query_as(
        "SELECT is_admin, operator, added_via->>'group' FROM usr
         WHERE workspace_id = 'test-workspace' AND email = 'demoted@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert!(
        !is_admin,
        "admin granted by role_a must not survive removal from role_a"
    );
    assert!(!is_operator, "role_b grants developer");
    assert_eq!(via.as_deref(), Some("role_b"));

    Ok(())
}

/// An overwrite import that moves a member from a dropped group to a retained one must keep
/// their workspace data.
///
/// Qualification must be judged against the imported membership, not the pre-import state:
/// judged too early, the member's new group is not yet visible, they are deleted, and any
/// re-add creates a fresh row stripped of everything workspace-scoped.
#[cfg(all(feature = "private", feature = "enterprise"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_overwrite_igroups_preserves_moved_member_data(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let global_base = format!("http://localhost:{port}/api/groups");
    let ws_base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    for g in ["move_from", "move_to"] {
        let resp = authed(client().post(format!("{global_base}/create")))
            .json(&json!({ "name": g }))
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "create {g}");
    }
    // Member starts only in move_from.
    let resp = authed(client().post(format!("{global_base}/adduser/move_from")))
        .json(&json!({ "email": "mover@example.com" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    let resp = authed(client().post(format!("{ws_base}/edit_instance_groups")))
        .json(&json!({
            "groups": ["move_from", "move_to"],
            "roles": { "move_from": "developer", "move_to": "developer" }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "edit: {}", resp.text().await?);

    let username: String = sqlx::query_scalar(
        "SELECT username FROM usr WHERE workspace_id = 'test-workspace' AND email = 'mover@example.com'",
    )
    .fetch_one(&db)
    .await?;
    sqlx::query(
        "INSERT INTO favorite (workspace_id, usr, path, favorite_kind)
         VALUES ('test-workspace', $1, 'f/moved/keep', 'script')",
    )
    .bind(&username)
    .execute(&db)
    .await?;

    // Import drops move_from entirely and puts the member in move_to instead.
    let resp = authed(client().post(format!("{global_base}/overwrite")))
        .json(&json!([
            { "name": "move_to", "emails": ["mover@example.com"] }
        ]))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "overwrite: {}", resp.text().await?);

    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM usr WHERE workspace_id = 'test-workspace' AND email = 'mover@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        remaining, 1,
        "member should still be in the workspace via move_to"
    );

    let favorites: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM favorite WHERE workspace_id = 'test-workspace' AND path = 'f/moved/keep'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        favorites, 1,
        "moving between groups in one import must not destroy workspace data"
    );

    Ok(())
}

/// A full-import overwrite must reconcile the membership of retained groups too: a member
/// dropped from a retained group loses the access that group granted, and a member who only
/// lost their highest-precedence group is re-roled in place instead of keeping a stale
/// elevated role.
///
/// Regression: the delta-based cleanup only acted on groups that disappeared from the import,
/// so an import that kept a group but dropped some of its members never cleaned those members
/// up.
#[cfg(all(feature = "private", feature = "enterprise"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_overwrite_igroups_reconciles_retained_group_membership(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let global_base = format!("http://localhost:{port}/api/groups");
    let ws_base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    for g in ["top_admins", "base_devs"] {
        let resp = authed(client().post(format!("{global_base}/create")))
            .json(&json!({ "name": g }))
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "create {g}");
    }

    // demoted@ holds admin via top_admins and developer via base_devs; dropped@ only has
    // base_devs.
    for (g, email) in [
        ("top_admins", "demoted@example.com"),
        ("base_devs", "demoted@example.com"),
        ("base_devs", "dropped@example.com"),
    ] {
        let resp = authed(client().post(format!("{global_base}/adduser/{g}")))
            .json(&json!({ "email": email }))
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "adduser {g}/{email}");
    }

    let resp = authed(client().post(format!("{ws_base}/edit_instance_groups")))
        .json(&json!({
            "groups": ["top_admins", "base_devs"],
            "roles": { "top_admins": "admin", "base_devs": "developer" }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "edit: {}", resp.text().await?);

    let (is_admin, via): (bool, Option<String>) = sqlx::query_as(
        "SELECT is_admin, added_via->>'group' FROM usr
         WHERE workspace_id = 'test-workspace' AND email = 'demoted@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert!(is_admin, "demoted@ should start as admin via top_admins");
    assert_eq!(via.as_deref(), Some("top_admins"));

    // Workspace state that must survive the demotion.
    let username: String = sqlx::query_scalar(
        "SELECT username FROM usr WHERE workspace_id = 'test-workspace' AND email = 'demoted@example.com'",
    )
    .fetch_one(&db)
    .await?;
    sqlx::query(
        "INSERT INTO favorite (workspace_id, usr, path, favorite_kind)
         VALUES ('test-workspace', $1, 'f/lifecycle/keep', 'script')",
    )
    .bind(&username)
    .execute(&db)
    .await?;

    // The import retains both groups but drops demoted@ from top_admins and dropped@ from
    // base_devs.
    let resp = authed(client().post(format!("{global_base}/overwrite")))
        .json(&json!([
            { "name": "top_admins", "emails": [] },
            { "name": "base_devs", "emails": ["demoted@example.com"] }
        ]))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "overwrite: {}", resp.text().await?);

    // demoted@ stays, re-roled to base_devs' developer, with their data intact.
    let (is_admin, is_operator, via): (bool, bool, Option<String>) = sqlx::query_as(
        "SELECT is_admin, operator, added_via->>'group' FROM usr
         WHERE workspace_id = 'test-workspace' AND email = 'demoted@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert!(
        !is_admin,
        "admin from top_admins must not survive being dropped from it"
    );
    assert!(!is_operator, "base_devs grants developer");
    assert_eq!(via.as_deref(), Some("base_devs"));

    let favorites: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM favorite WHERE workspace_id = 'test-workspace' AND path = 'f/lifecycle/keep'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        favorites, 1,
        "re-roling in place must not destroy workspace data"
    );

    // dropped@ lost their only configured group even though the group itself was retained.
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM usr
         WHERE workspace_id = 'test-workspace' AND email = 'dropped@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        remaining, 0,
        "member dropped from a retained group must be removed"
    );

    // Both groups were retained, so the workspace config is untouched.
    let (groups, roles): (serde_json::Value, serde_json::Value) = sqlx::query_as(
        "SELECT auto_invite->'instance_groups', auto_invite->'instance_groups_roles'
         FROM workspace_settings WHERE workspace_id = 'test-workspace'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(groups, json!(["top_admins", "base_devs"]));
    assert_eq!(
        roles,
        json!({ "top_admins": "admin", "base_devs": "developer" })
    );

    Ok(())
}

/// Members whose `added_via` source is not 'instance_group' — manually added users, and the
/// orphaned members the `preserve_orphaned_instance_group_members` migration converted to
/// manual — are invisible to reconciliation: never re-roled and never removed, even when they
/// also appear in a configured group's membership.
#[cfg(feature = "private")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_reconcile_ignores_non_instance_group_members(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let global_base = format!("http://localhost:{port}/api/groups");
    let ws_base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    let resp = authed(client().post(format!("{global_base}/create")))
        .json(&json!({ "name": "visible_grp" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "create");
    for email in ["kept@example.com", "shielded@example.com"] {
        let resp = authed(client().post(format!("{global_base}/adduser/visible_grp")))
            .json(&json!({ "email": email }))
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "adduser {email}");
    }

    // shielded@ is already in the workspace through a non-instance_group source (the shape
    // the migration leaves behind), at a role the group config would not grant. The username
    // deliberately differs from the instance-derived one ('shielded'): an unguarded
    // auto_add_user would then insert a second usr row for the email instead of no-op'ing on
    // a username conflict, so the count assertions below can catch it.
    sqlx::query(
        r#"INSERT INTO usr (workspace_id, username, email, is_admin, operator, added_via)
           VALUES ('test-workspace', 'shielded_legacy', 'shielded@example.com', true, false,
                   '{"source": "manual", "migrated_from_instance_group": "gone_grp"}'::jsonb)"#,
    )
    .execute(&db)
    .await?;

    let resp = authed(client().post(format!("{ws_base}/edit_instance_groups")))
        .json(&json!({
            "groups": ["visible_grp"],
            "roles": { "visible_grp": "developer" }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "edit: {}", resp.text().await?);

    // kept@ was auto-added via the group; shielded@ kept their single manual row untouched.
    let kept: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM usr WHERE workspace_id = 'test-workspace' AND email = 'kept@example.com'
         AND added_via->>'source' = 'instance_group'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(kept, 1, "group member should be auto-added");

    let shielded_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM usr WHERE workspace_id = 'test-workspace' AND email = 'shielded@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        shielded_rows, 1,
        "reconciliation must not create a second usr row for a member already present under a non-instance_group source"
    );

    // Dropping both users from the group removes the instance_group-sourced member but must
    // leave the manual row alone.
    for email in ["kept@example.com", "shielded@example.com"] {
        let resp = authed(client().post(format!("{global_base}/removeuser/visible_grp")))
            .json(&json!({ "email": email }))
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "removeuser {email}");
    }

    let kept: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM usr WHERE workspace_id = 'test-workspace' AND email = 'kept@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        kept, 0,
        "instance_group-sourced member loses access with their only group"
    );

    let (username, is_admin, via_source): (String, bool, Option<String>) = sqlx::query_as(
        "SELECT username, is_admin, added_via->>'source' FROM usr
         WHERE workspace_id = 'test-workspace' AND email = 'shielded@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        username, "shielded_legacy",
        "the original manual row must be the only one"
    );
    assert!(
        is_admin,
        "manual member's role must not be touched by reconciliation"
    );
    assert_eq!(via_source.as_deref(), Some("manual"));

    Ok(())
}

/// The upgrade migration converts every member the reconciler would evict — those whose
/// granting group was deleted and those dropped from a group that still exists — and leaves
/// still-qualifying members alone. The migration has already run against the empty test
/// database by the time this executes, so the test fabricates pre-fix state and re-executes
/// the migration's statements, which are idempotent plain UPDATEs.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_preserve_orphaned_members_migration(db: Pool<Postgres>) -> anyhow::Result<()> {
    // ghost_grp pins the statement order: it is referenced by the workspace and still has a
    // membership row, but no instance_group row. Only when the reference strip runs before
    // the conversion does ghost@ read as unconverted-by-membership nowhere and get preserved;
    // converting first would spare them on the doomed reference and then strand them.
    sqlx::raw_sql(
        r#"
        INSERT INTO workspace (id, name, owner) VALUES ('mig-ws', 'mig-ws', 'admin@windmill.dev');
        INSERT INTO workspace_settings (workspace_id, auto_invite) VALUES
          ('mig-ws', '{"instance_groups": ["gone_grp", "ghost_grp", "live_grp"], "instance_groups_roles": {"gone_grp": "admin", "ghost_grp": "developer", "live_grp": "developer"}}'::jsonb);
        INSERT INTO instance_group (name) VALUES ('live_grp');
        INSERT INTO email_to_igroup (email, igroup) VALUES
          ('live@example.com', 'live_grp'),
          ('ghost@example.com', 'ghost_grp');
        INSERT INTO usr (workspace_id, username, email, is_admin, operator, added_via) VALUES
          ('mig-ws', 'orphan', 'orphan@example.com', true, false, '{"source": "instance_group", "group": "gone_grp"}'::jsonb),
          ('mig-ws', 'droppedu', 'dropped@example.com', false, false, '{"source": "instance_group", "group": "live_grp"}'::jsonb),
          ('mig-ws', 'ghostmember', 'ghost@example.com', false, false, '{"source": "instance_group", "group": "ghost_grp"}'::jsonb),
          ('mig-ws', 'livemember', 'live@example.com', false, false, '{"source": "instance_group", "group": "live_grp"}'::jsonb);
        "#,
    )
    .execute(&db)
    .await?;

    sqlx::raw_sql(include_str!(
        "../../migrations/20260813195023_preserve_orphaned_instance_group_members.up.sql"
    ))
    .execute(&db)
    .await?;

    // Deleted-group orphan and retained-group-dropped orphan both become manual members
    // with the original group recorded; the still-qualifying member is untouched.
    for (email, expected_group) in [
        ("orphan@example.com", "gone_grp"),
        ("dropped@example.com", "live_grp"),
        ("ghost@example.com", "ghost_grp"),
    ] {
        let (source, migrated_from): (Option<String>, Option<String>) = sqlx::query_as(
            "SELECT added_via->>'source', added_via->>'migrated_from_instance_group'
             FROM usr WHERE workspace_id = 'mig-ws' AND email = $1",
        )
        .bind(email)
        .fetch_one(&db)
        .await?;
        assert_eq!(
            source.as_deref(),
            Some("manual"),
            "{email} should be converted"
        );
        assert_eq!(
            migrated_from.as_deref(),
            Some(expected_group),
            "{email} marker"
        );
    }

    let (source, group): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT added_via->>'source', added_via->>'group'
         FROM usr WHERE workspace_id = 'mig-ws' AND email = 'live@example.com'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        source.as_deref(),
        Some("instance_group"),
        "still-qualifying member spared"
    );
    assert_eq!(group.as_deref(), Some("live_grp"));

    // The dangling references are stripped from both auto_invite fields; the live one stays.
    let (groups, roles): (serde_json::Value, serde_json::Value) = sqlx::query_as(
        "SELECT auto_invite->'instance_groups', auto_invite->'instance_groups_roles'
         FROM workspace_settings WHERE workspace_id = 'mig-ws'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(groups, json!(["live_grp"]));
    assert_eq!(roles, json!({ "live_grp": "developer" }));

    Ok(())
}
