#[cfg(feature = "deno_core")]
use serde_json::json;
#[cfg(feature = "deno_core")]
use sqlx::{Pool, Postgres};

#[cfg(feature = "deno_core")]
use windmill_test_utils::*;

/// Helper to create a client authenticated as a specific user
#[cfg(feature = "deno_core")]
async fn create_client_for_user(_port: u16, token: &str) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap()
}

/// Test helper to check if a GET request succeeds
#[cfg(feature = "deno_core")]
async fn can_read(client: &reqwest::Client, url: &str) -> bool {
    let resp = client.get(url).send().await.unwrap();
    resp.status().is_success()
}

/// Test helper to check if a POST request succeeds (for write operations)
#[cfg(feature = "deno_core")]
async fn can_write(client: &reqwest::Client, url: &str, body: serde_json::Value) -> bool {
    let resp = client.post(url).json(&body).send().await.unwrap();
    let status = resp.status();
    // 200, 201, 204 are success, 403 is forbidden, 401 is unauthorized
    status.is_success()
}

/// Comprehensive permissions test that verifies:
/// - Non-admin users can only access their user-space (u/username)
/// - Non-admin users can only access folders they have been given access to
/// - Read access when given read permission
/// - Write access when given write permission
/// - Access through direct permissions (u/username)
/// - Access through group memberships (g/groupname)
///
/// This test is ignored in CI because it requires a full Windmill setup
/// and takes longer to run. Run it locally with:
/// `cargo test --features deno_core permissions -- --ignored`
#[ignore]
#[cfg(feature = "deno_core")]
#[sqlx::test(migrations = "../migrations", fixtures("base", "permissions_test"))]
async fn test_permissions_exhaustive(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{}/api", port);

    // Create clients for each user (tokens must be at least 10 chars)
    let admin_client = create_client_for_user(port, "ADMIN_TOKEN_TEST").await;
    let alice_client = create_client_for_user(port, "ALICE_TOKEN_TEST").await;
    let bob_client = create_client_for_user(port, "BOB_TOKEN_TEST12").await;
    let charlie_client = create_client_for_user(port, "CHARLIE_TOKEN_01").await;

    // ============================================
    // TEST 1: User namespace access (u/username)
    // ============================================

    // Alice should be able to read her own scripts
    assert!(
        can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/u/alice/my_script")
        )
        .await,
        "Alice should be able to read her own script"
    );

    // Alice should NOT be able to read Bob's scripts
    assert!(
        !can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/u/bob/my_script")
        )
        .await,
        "Alice should NOT be able to read Bob's script"
    );

    // Bob should be able to read his own scripts
    assert!(
        can_read(
            &bob_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/u/bob/my_script")
        )
        .await,
        "Bob should be able to read his own script"
    );

    // Bob should NOT be able to read Alice's scripts
    assert!(
        !can_read(
            &bob_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/u/alice/my_script")
        )
        .await,
        "Bob should NOT be able to read Alice's script"
    );

    // Admin should be able to read any script
    assert!(
        can_read(
            &admin_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/u/alice/my_script")
        )
        .await,
        "Admin should be able to read Alice's script"
    );
    assert!(
        can_read(
            &admin_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/u/bob/my_script")
        )
        .await,
        "Admin should be able to read Bob's script"
    );

    // ============================================
    // TEST 2: Folder access without permissions
    // ============================================

    // Charlie has no folder permissions, should not be able to access folder items
    assert!(
        !can_read(
            &charlie_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/f/shared/public_script")
        )
        .await,
        "Charlie should NOT be able to read shared folder script without permissions"
    );

    // ============================================
    // TEST 3: Folder read-only access
    // ============================================

    // Alice has read-only access to 'shared' folder
    assert!(
        can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/f/shared/public_script")
        )
        .await,
        "Alice should be able to read shared folder script (read permission)"
    );

    // Alice should NOT be able to create/update scripts in shared folder (read-only)
    let create_script_body = json!({
        "path": "f/shared/alice_new_script",
        "content": "export function main() { return 'test'; }",
        "language": "deno",
        "summary": "Test script",
        "description": ""
    });
    assert!(
        !can_write(
            &alice_client,
            &format!("{base_url}/w/test-workspace/scripts/create"),
            create_script_body.clone()
        )
        .await,
        "Alice should NOT be able to create scripts in shared folder (read-only)"
    );

    // ============================================
    // TEST 4: Folder write access
    // ============================================

    // Bob has write access to 'shared' folder
    assert!(
        can_read(
            &bob_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/f/shared/public_script")
        )
        .await,
        "Bob should be able to read shared folder script (write permission includes read)"
    );

    // Bob should be able to create scripts in shared folder (write access)
    let create_script_body = json!({
        "path": "f/shared/bob_new_script",
        "content": "export function main() { return 'test'; }",
        "language": "deno",
        "summary": "Test script",
        "description": "",
        "schema": {}
    });
    assert!(
        can_write(
            &bob_client,
            &format!("{base_url}/w/test-workspace/scripts/create"),
            create_script_body
        )
        .await,
        "Bob should be able to create scripts in shared folder (write permission)"
    );

    // ============================================
    // TEST 5: Group-based folder access
    // ============================================

    // Charlie is in the 'developers' group which has read access to 'team' folder
    // First, let's verify Charlie has no direct access
    // Then verify he gets access through group membership

    assert!(
        can_read(
            &charlie_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/f/team/team_script")
        )
        .await,
        "Charlie should be able to read team folder script via group membership"
    );

    // Charlie's group only has read access, so he shouldn't be able to write
    let create_team_script = json!({
        "path": "f/team/charlie_script",
        "content": "export function main() { return 'test'; }",
        "language": "deno",
        "summary": "Test script",
        "description": "",
        "schema": {}
    });
    assert!(
        !can_write(
            &charlie_client,
            &format!("{base_url}/w/test-workspace/scripts/create"),
            create_team_script
        )
        .await,
        "Charlie should NOT be able to create scripts in team folder (group has read-only)"
    );

    // ============================================
    // TEST 6: Resource permissions
    // ============================================

    // Alice should be able to read her own resources
    // Resources use /resources/get/{path} (no /p/ prefix)
    assert!(
        can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/resources/get/u/alice/my_resource")
        )
        .await,
        "Alice should be able to read her own resource"
    );

    // Alice should NOT be able to read Bob's resources
    assert!(
        !can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/resources/get/u/bob/my_resource")
        )
        .await,
        "Alice should NOT be able to read Bob's resource"
    );

    // ============================================
    // TEST 7: Variable permissions
    // ============================================

    // Alice should be able to read her own variables
    // Variables use /variables/get/{path} (no /p/ prefix)
    assert!(
        can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/variables/get/u/alice/my_variable")
        )
        .await,
        "Alice should be able to read her own variable"
    );

    // Alice should NOT be able to read Bob's variables
    assert!(
        !can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/variables/get/u/bob/my_variable")
        )
        .await,
        "Alice should NOT be able to read Bob's variable"
    );

    // ============================================
    // TEST 8: Flow permissions
    // ============================================

    // Alice should be able to read flows in folders she has access to
    // Flows use /flows/get/{path} (no /p/ prefix)
    assert!(
        can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/flows/get/f/shared/shared_flow")
        )
        .await,
        "Alice should be able to read shared folder flow"
    );

    // Charlie should NOT be able to read flows in shared folder (no access)
    assert!(
        !can_read(
            &charlie_client,
            &format!("{base_url}/w/test-workspace/flows/get/f/shared/shared_flow")
        )
        .await,
        "Charlie should NOT be able to read shared folder flow (no permission)"
    );

    // ============================================
    // TEST 9: Direct extra_perms grants
    // ============================================

    // Alice has direct read permission on a specific script owned by admin
    assert!(
        can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/f/admin_only/shared_with_alice")
        )
        .await,
        "Alice should be able to read script explicitly shared with her"
    );

    // Bob should NOT be able to read that script (not shared with him)
    assert!(
        !can_read(
            &bob_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/f/admin_only/shared_with_alice")
        )
        .await,
        "Bob should NOT be able to read script only shared with Alice"
    );

    // ============================================
    // TEST 10: Write via group membership
    // ============================================

    // Bob is in the 'editors' group which has write access to 'editable' folder
    let create_editable_script = json!({
        "path": "f/editable/bob_editor_script",
        "content": "export function main() { return 'test'; }",
        "language": "deno",
        "summary": "Test script",
        "description": "",
        "schema": {}
    });
    assert!(
        can_write(
            &bob_client,
            &format!("{base_url}/w/test-workspace/scripts/create"),
            create_editable_script
        )
        .await,
        "Bob should be able to create scripts in editable folder via group write access"
    );

    // ============================================
    // TEST 11: Schedule permissions
    // ============================================

    // Schedules follow the same permission model
    // Schedules use /schedules/get/{path} (no /p/ prefix)
    assert!(
        can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/schedules/get/u/alice/my_schedule")
        )
        .await,
        "Alice should be able to read her own schedule"
    );

    assert!(
        !can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/schedules/get/u/bob/my_schedule")
        )
        .await,
        "Alice should NOT be able to read Bob's schedule"
    );

    // ============================================
    // TEST 12: App permissions
    // ============================================

    // Alice should be able to read apps in shared folder
    assert!(
        can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/apps/get/p/f/shared/shared_app")
        )
        .await,
        "Alice should be able to read shared folder app"
    );

    // Charlie should NOT be able to read apps in shared folder
    assert!(
        !can_read(
            &charlie_client,
            &format!("{base_url}/w/test-workspace/apps/get/p/f/shared/shared_app")
        )
        .await,
        "Charlie should NOT be able to read shared folder app"
    );

    // ============================================
    // TEST 13: Operator permissions
    // ============================================
    // Note: Operators can execute but have limited management permissions
    // This is tested separately if needed

    // ============================================
    // TEST 14: Folder owner permissions
    // ============================================

    // Alice is an owner of the 'alice_owned' folder
    assert!(
        can_read(
            &alice_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/f/alice_owned/owner_script")
        )
        .await,
        "Alice should be able to read scripts in folder she owns"
    );

    // Alice should be able to write to her owned folder
    let create_owned_script = json!({
        "path": "f/alice_owned/new_owner_script",
        "content": "export function main() { return 'test'; }",
        "language": "deno",
        "summary": "Test script",
        "description": "",
        "schema": {}
    });
    assert!(
        can_write(
            &alice_client,
            &format!("{base_url}/w/test-workspace/scripts/create"),
            create_owned_script
        )
        .await,
        "Alice should be able to create scripts in folder she owns"
    );

    // ============================================
    // TEST 15: Cross-user script with extra_perms
    // ============================================

    // Verify that giving someone read-only permission doesn't give write
    // Bob has read-only permission on alice's extra_shared_script
    assert!(
        can_read(
            &bob_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/u/alice/extra_shared_script")
        )
        .await,
        "Bob should be able to read Alice's script shared with him"
    );

    // But Bob should not be able to archive/delete it
    let archive_resp = bob_client
        .post(&format!(
            "{base_url}/w/test-workspace/scripts/archive/p/u/alice/extra_shared_script"
        ))
        .send()
        .await
        .unwrap();
    assert!(
        !archive_resp.status().is_success(),
        "Bob should NOT be able to archive Alice's script (read-only permission)"
    );

    Ok(())
}

/// Additional test for verifying group permission inheritance
#[ignore]
#[cfg(feature = "deno_core")]
#[sqlx::test(migrations = "../migrations", fixtures("base", "permissions_test"))]
async fn test_group_permission_inheritance(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{}/api", port);

    let charlie_client = create_client_for_user(port, "CHARLIE_TOKEN_01").await;

    // Charlie is in 'developers' group
    // 'developers' group has read access to 'team' folder

    // Test 1: Charlie can read from team folder via group
    // Scripts use /scripts/get/p/{path}
    assert!(
        can_read(
            &charlie_client,
            &format!("{base_url}/w/test-workspace/scripts/get/p/f/team/team_script")
        )
        .await,
        "Charlie should read team folder via group"
    );

    // Test 2: Add Charlie to 'editors' group (which has write to 'editable')
    // This should give him write access
    sqlx::query!(
        "INSERT INTO usr_to_group (workspace_id, group_, usr) VALUES ('test-workspace', 'editors', 'charlie')"
    )
    .execute(&db)
    .await?;

    // Note: The AUTH_CACHE in windmill_api::auth caches authentication results
    // for (workspace_id, token) tuples. Since we can't easily clear it from tests,
    // we use a different token or wait for cache expiry. For this test, we create
    // a new token for Charlie.
    sqlx::query!(
        "INSERT INTO token (token, email, label, super_admin, owner, workspace_id)
         VALUES ('CHARLIE_TOKEN_NEW', 'charlie@windmill.dev', 'Charlie new token', false, 'u/charlie', 'test-workspace')"
    )
    .execute(&db)
    .await?;

    // Use the new token to get fresh permissions
    let charlie_client_fresh = create_client_for_user(port, "CHARLIE_TOKEN_NEW").await;

    let create_script = json!({
        "path": "f/editable/charlie_editor_script",
        "content": "export function main() { return 'from charlie'; }",
        "language": "deno",
        "summary": "Charlie's editor script",
        "description": "",
        "schema": {}
    });

    assert!(
        can_write(
            &charlie_client_fresh,
            &format!("{base_url}/w/test-workspace/scripts/create"),
            create_script
        )
        .await,
        "Charlie should be able to write to editable folder after joining editors group"
    );

    Ok(())
}

/// Test that permissions work correctly for all item types
#[ignore]
#[cfg(feature = "deno_core")]
#[sqlx::test(migrations = "../migrations", fixtures("base", "permissions_test"))]
async fn test_all_item_types_permissions(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{}/api", port);

    let alice_client = create_client_for_user(port, "ALICE_TOKEN_TEST").await;
    let bob_client = create_client_for_user(port, "BOB_TOKEN_TEST12").await;

    // Test Scripts - uses /scripts/get/p/{path}
    assert!(can_read(&alice_client, &format!("{base_url}/w/test-workspace/scripts/get/p/u/alice/my_script")).await);
    assert!(!can_read(&bob_client, &format!("{base_url}/w/test-workspace/scripts/get/p/u/alice/my_script")).await);

    // Test Flows - uses /flows/get/{path} (no /p/)
    assert!(can_read(&alice_client, &format!("{base_url}/w/test-workspace/flows/get/u/alice/my_flow")).await);
    assert!(!can_read(&bob_client, &format!("{base_url}/w/test-workspace/flows/get/u/alice/my_flow")).await);

    // Test Resources - uses /resources/get/{path} (no /p/)
    assert!(can_read(&alice_client, &format!("{base_url}/w/test-workspace/resources/get/u/alice/my_resource")).await);
    assert!(!can_read(&bob_client, &format!("{base_url}/w/test-workspace/resources/get/u/alice/my_resource")).await);

    // Test Variables - uses /variables/get/{path} (no /p/)
    assert!(can_read(&alice_client, &format!("{base_url}/w/test-workspace/variables/get/u/alice/my_variable")).await);
    assert!(!can_read(&bob_client, &format!("{base_url}/w/test-workspace/variables/get/u/alice/my_variable")).await);

    // Test Schedules - uses /schedules/get/{path} (no /p/)
    assert!(can_read(&alice_client, &format!("{base_url}/w/test-workspace/schedules/get/u/alice/my_schedule")).await);
    assert!(!can_read(&bob_client, &format!("{base_url}/w/test-workspace/schedules/get/u/alice/my_schedule")).await);

    // Test Apps - uses /apps/get/p/{path}
    assert!(can_read(&alice_client, &format!("{base_url}/w/test-workspace/apps/get/p/u/alice/my_app")).await);
    assert!(!can_read(&bob_client, &format!("{base_url}/w/test-workspace/apps/get/p/u/alice/my_app")).await);

    Ok(())
}

/// Test that operators cannot create or update scripts, flows, and apps
/// Operators have limited permissions - they can execute but cannot manage resources
#[ignore]
#[cfg(feature = "deno_core")]
#[sqlx::test(migrations = "../migrations", fixtures("base", "permissions_test"))]
async fn test_operator_cannot_create_update(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{}/api", port);

    // Create operator client
    let operator_client = create_client_for_user(port, "OPERATOR_TOKEN_1").await;

    // ============================================
    // TEST: Operator cannot create scripts
    // ============================================
    let create_script = json!({
        "path": "u/operator/test_script",
        "content": "export function main() { return 'test'; }",
        "language": "deno",
        "summary": "Test script",
        "description": "",
        "schema": {}
    });
    assert!(
        !can_write(
            &operator_client,
            &format!("{base_url}/w/test-workspace/scripts/create"),
            create_script
        )
        .await,
        "Operator should NOT be able to create scripts"
    );

    // ============================================
    // TEST: Operator cannot create flows
    // ============================================
    let create_flow = json!({
        "path": "u/operator/test_flow",
        "summary": "Test flow",
        "description": "",
        "value": {"modules": []},
        "schema": {}
    });
    assert!(
        !can_write(
            &operator_client,
            &format!("{base_url}/w/test-workspace/flows/create"),
            create_flow
        )
        .await,
        "Operator should NOT be able to create flows"
    );

    // ============================================
    // TEST: Operator cannot create apps
    // ============================================
    let create_app = json!({
        "path": "u/operator/test_app",
        "summary": "Test app",
        "value": {"grid": []},
        "policy": {
            "on_behalf_of": "u/operator",
            "on_behalf_of_email": "operator@windmill.dev",
            "execution_mode": "viewer"
        }
    });
    assert!(
        !can_write(
            &operator_client,
            &format!("{base_url}/w/test-workspace/apps/create"),
            create_app
        )
        .await,
        "Operator should NOT be able to create apps"
    );

    // ============================================
    // TEST: Operator cannot archive scripts
    // ============================================

    // Insert a script in operator's namespace using direct DB access
    sqlx::query!(
        r#"INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, extra_perms)
           VALUES ('test-workspace', 3001, 'u/operator/existing_script', 'export function main() { return "original"; }', 'deno', 'script', 'admin', '{}', 'Existing script', '', '', '{}')"#
    )
    .execute(&db)
    .await?;

    // Operator tries to archive this script
    let archive_resp = operator_client
        .post(&format!(
            "{base_url}/w/test-workspace/scripts/archive/p/u/operator/existing_script"
        ))
        .send()
        .await
        .unwrap();
    assert!(
        !archive_resp.status().is_success(),
        "Operator should NOT be able to archive scripts"
    );

    // ============================================
    // TEST: Operator cannot update flows
    // ============================================

    // Insert a flow in operator's namespace
    sqlx::query!(
        r#"INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, schema, extra_perms)
           VALUES ('test-workspace', 'u/operator/existing_flow', 'Existing flow', '', '{"modules": []}', 'admin', NOW(), '{}', '{}')"#
    )
    .execute(&db)
    .await?;

    let update_flow = json!({
        "path": "u/operator/existing_flow",
        "summary": "Updated flow",
        "description": "Updated",
        "value": {"modules": []},
        "schema": {}
    });

    let update_flow_resp = operator_client
        .post(&format!(
            "{base_url}/w/test-workspace/flows/update/u/operator/existing_flow"
        ))
        .json(&update_flow)
        .send()
        .await
        .unwrap();
    assert!(
        !update_flow_resp.status().is_success(),
        "Operator should NOT be able to update flows"
    );

    // ============================================
    // TEST: Operator cannot update apps
    // ============================================

    // Insert an app in operator's namespace
    sqlx::query!(
        r#"INSERT INTO app (id, workspace_id, path, summary, versions, policy, extra_perms)
           VALUES (3001, 'test-workspace', 'u/operator/existing_app', 'Existing app', '{}',
                   '{"on_behalf_of": "u/admin", "on_behalf_of_email": "admin@windmill.dev", "execution_mode": "viewer"}', '{}')"#
    )
    .execute(&db)
    .await?;

    // Create an app version
    sqlx::query!(
        r#"INSERT INTO app_version (id, app_id, value, created_by, created_at)
           VALUES (3001, 3001, '{"grid": []}', 'admin', NOW())"#
    )
    .execute(&db)
    .await?;

    // Update app versions
    sqlx::query!(
        "UPDATE app SET versions = ARRAY[3001::bigint] WHERE id = 3001"
    )
    .execute(&db)
    .await?;

    let update_app = json!({
        "path": "u/operator/existing_app",
        "summary": "Updated app",
        "value": {"grid": []},
        "policy": {
            "on_behalf_of": "u/operator",
            "on_behalf_of_email": "operator@windmill.dev",
            "execution_mode": "viewer"
        }
    });

    let update_app_resp = operator_client
        .post(&format!(
            "{base_url}/w/test-workspace/apps/update/u/operator/existing_app"
        ))
        .json(&update_app)
        .send()
        .await
        .unwrap();
    assert!(
        !update_app_resp.status().is_success(),
        "Operator should NOT be able to update apps"
    );

    Ok(())
}
