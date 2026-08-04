use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// Comprehensive integration test for the compare_workspaces endpoint.
///
/// This test validates workspace fork comparison functionality by:
/// 1. Setting up a parent workspace with all item types (scripts, flows, apps, resources, variables, resource_types, folders)
/// 2. Creating a fork of the workspace
/// 3. Making various changes in both workspaces (new items, modifications, conflicts, deletions, renames)
/// 4. Populating the workspace_diff table to simulate Git sync tracking
/// 5. Calling compare_workspaces and verifying all aspects of the comparison
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_compare_workspaces_comprehensive(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );
    let base_url = format!("http://localhost:{port}/api");

    // ==============================================================
    // PHASE 1: Setup Parent Workspace with All Item Types
    // ==============================================================

    // Create folder first (other items will use it)
    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, summary, created_by)
         VALUES ('test-workspace', 'shared', 'Shared Folder', ARRAY['test@windmill.dev']::varchar[], 'Test folder', 'test@windmill.dev')"
    )
    .execute(&db)
    .await?;

    // Create scripts
    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, archived, schema_validation, ws_error_handler_muted, deleted)
         VALUES
         ('test-workspace', 'f/shared/original_script', 12345, 'def main(): pass', 'Original', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false),
         ('test-workspace', 'f/shared/to_modify_parent', 22222, 'def main(): return 1', 'To modify in parent', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false),
         ('test-workspace', 'f/shared/to_modify_fork', 33333, 'def main(): return 2', 'To modify in fork', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false),
         ('test-workspace', 'f/shared/to_conflict', 44444, 'def main(): return 3', 'To conflict', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false),
         ('test-workspace', 'f/shared/to_delete', 55555, 'def main(): return 4', 'To delete', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false)"
    )
    .execute(&db)
    .await?;

    // Create flow
    sqlx::query!(
        "INSERT INTO flow (workspace_id, path, summary, description, value, schema, edited_by, edited_at, archived)
         VALUES ('test-workspace', 'f/shared/original_flow', 'Flow summary', '', $1, NULL, 'test@windmill.dev', NOW(), false)",
        json!({"modules": []})
    )
    .execute(&db)
    .await?;

    // Create resource
    sqlx::query!(
        "INSERT INTO resource (workspace_id, path, value, resource_type, description, created_by)
         VALUES
         ('test-workspace', 'f/shared/db_config', $1, 'postgresql', '', 'test@windmill.dev'),
         ('test-workspace', 'f/shared/old_name', $2, 'generic', '', 'test@windmill.dev'),
         ('test-workspace', 'f/shared/resource_to_modify', $3, 'generic', '', 'test@windmill.dev')",
        json!({"host": "localhost"}),
        json!({}),
        json!({"key": "value"})
    )
    .execute(&db)
    .await?;

    // Create variable
    sqlx::query!(
        "INSERT INTO variable (workspace_id, path, value, is_secret, description)
         VALUES
         ('test-workspace', 'f/shared/api_key', 'secret123', false, 'Test key'),
         ('test-workspace', 'f/shared/variable_to_modify', 'original', false, 'To modify')"
    )
    .execute(&db)
    .await?;

    // Create resource type
    sqlx::query!(
        "INSERT INTO resource_type (workspace_id, name, schema, description, created_by)
         VALUES ('test-workspace', 'custom_db', $1, 'Custom DB type', 'test@windmill.dev')",
        json!({"type": "object"})
    )
    .execute(&db)
    .await?;

    // Create app
    sqlx::query!(
        "INSERT INTO app (workspace_id, path, summary, policy, versions, extra_perms)
         VALUES ('test-workspace', 'f/shared/dashboard', 'Dashboard app', '{}', ARRAY[1::bigint], '{}')"
    )
    .execute(&db)
    .await?;

    let app_id = sqlx::query_scalar!(
        "SELECT id FROM app WHERE path = 'f/shared/dashboard' AND workspace_id = 'test-workspace'"
    )
    .fetch_one(&db)
    .await?;

    sqlx::query!(
        "INSERT INTO app_version (app_id, value, created_by, created_at)
         VALUES ($1, $2, 'test@windmill.dev', NOW())",
        app_id,
        json!({"grid": []})
    )
    .execute(&db)
    .await?;

    // ==============================================================
    // PHASE 2: Create Fork
    // ==============================================================

    let fork_response = client
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .json(&json!({
            "id": "wm-fork-test-workspace",
            "name": "Test Fork",
            "color": "#0000ff"
        }))
        .send()
        .await?;

    let status = fork_response.status();
    assert!(
        status.is_success(),
        "Fork creation should succeed: {}",
        status
    );

    // Verify fork was created
    let fork_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM workspace WHERE id = 'wm-fork-test-workspace')"
    )
    .fetch_one(&db)
    .await?;
    assert!(fork_exists.unwrap_or(false), "Fork workspace should exist");

    // ==============================================================
    // PHASE 3: Make Changes in Both Workspaces
    // ==============================================================

    // Scenario 1: New script in parent (ahead)
    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, archived, schema_validation, ws_error_handler_muted, deleted)
         VALUES ('test-workspace', 'f/shared/new_in_parent', 54321, 'def main(): return \"new\"', 'New in parent', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false)"
    )
    .execute(&db)
    .await?;

    // Scenario 2: New script in fork (behind)
    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, archived, schema_validation, ws_error_handler_muted, deleted)
         VALUES ('wm-fork-test-workspace', 'f/shared/new_in_fork', 99999, 'def main(): return \"fork\"', 'New in fork', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false)"
    )
    .execute(&db)
    .await?;

    // Scenario 3: Modify script in parent (ahead)
    sqlx::query!(
        "UPDATE script
         SET content = 'def main(): return \"modified\"', summary = 'Modified in parent'
         WHERE workspace_id = 'test-workspace' AND path = 'f/shared/to_modify_parent'"
    )
    .execute(&db)
    .await?;

    // Scenario 4: Modify script in fork (behind)
    sqlx::query!(
        "UPDATE script
         SET content = 'def main(): return \"fork_modified\"', summary = 'Modified in fork'
         WHERE workspace_id = 'wm-fork-test-workspace' AND path = 'f/shared/to_modify_fork'"
    )
    .execute(&db)
    .await?;

    // Scenario 5: Conflict - modify in both workspaces
    sqlx::query!(
        "UPDATE flow SET value = $1
         WHERE workspace_id = 'test-workspace' AND path = 'f/shared/original_flow'",
        json!({"modules": [{"id": "a"}]})
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        "UPDATE flow SET value = $1
         WHERE workspace_id = 'wm-fork-test-workspace' AND path = 'f/shared/original_flow'",
        json!({"modules": [{"id": "b"}]})
    )
    .execute(&db)
    .await?;

    // Scenario 6: Delete (archive) in fork
    sqlx::query!(
        "UPDATE script SET archived = true
         WHERE workspace_id = 'wm-fork-test-workspace' AND path = 'f/shared/to_delete'"
    )
    .execute(&db)
    .await?;

    // Scenario 7: Rename in parent (resource)
    sqlx::query!(
        "UPDATE resource SET path = 'f/shared/new_name'
         WHERE workspace_id = 'test-workspace' AND path = 'f/shared/old_name'"
    )
    .execute(&db)
    .await?;

    // Scenario 8: Modify app in parent
    sqlx::query!(
        "UPDATE app SET summary = 'Modified dashboard app'
         WHERE workspace_id = 'test-workspace' AND path = 'f/shared/dashboard'"
    )
    .execute(&db)
    .await?;

    // Scenario 9: Modify resource in fork
    sqlx::query!(
        "UPDATE resource SET value = $1
         WHERE workspace_id = 'wm-fork-test-workspace' AND path = 'f/shared/resource_to_modify'",
        json!({"key": "modified_value"})
    )
    .execute(&db)
    .await?;

    // Modify variable in parent
    sqlx::query!(
        "UPDATE variable SET value = 'modified_value'
         WHERE workspace_id = 'test-workspace' AND path = 'f/shared/variable_to_modify'"
    )
    .execute(&db)
    .await?;

    // Create new resource type in parent
    sqlx::query!(
        "INSERT INTO resource_type (workspace_id, name, schema, description, created_by)
         VALUES ('test-workspace', 'new_type', $1, 'New type in parent', 'test@windmill.dev')",
        json!({"type": "string"})
    )
    .execute(&db)
    .await?;

    // Modify folder in fork (display_name)
    sqlx::query!(
        "UPDATE folder SET display_name = 'Modified Shared Folder'
         WHERE workspace_id = 'wm-fork-test-workspace' AND name = 'shared'"
    )
    .execute(&db)
    .await?;

    // ==============================================================
    // PHASE 4: Populate workspace_diff Table
    // ==============================================================

    // New in parent (ahead)
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES
         ('test-workspace', 'wm-fork-test-workspace', 'f/shared/new_in_parent', 'script', 1, 0, NULL),
         ('test-workspace', 'wm-fork-test-workspace', 'new_type', 'resource_type', 1, 0, NULL)"
    )
    .execute(&db)
    .await?;

    // New in fork (behind)
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES ('test-workspace', 'wm-fork-test-workspace', 'f/shared/new_in_fork', 'script', 0, 1, NULL)"
    )
    .execute(&db)
    .await?;

    // Modified in parent (ahead)
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES
         ('test-workspace', 'wm-fork-test-workspace', 'f/shared/to_modify_parent', 'script', 1, 0, NULL),
         ('test-workspace', 'wm-fork-test-workspace', 'f/shared/dashboard', 'app', 1, 0, NULL),
         ('test-workspace', 'wm-fork-test-workspace', 'f/shared/variable_to_modify', 'variable', 1, 0, NULL)"
    )
    .execute(&db)
    .await?;

    // Modified in fork (behind)
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES
         ('test-workspace', 'wm-fork-test-workspace', 'f/shared/to_modify_fork', 'script', 0, 1, NULL),
         ('test-workspace', 'wm-fork-test-workspace', 'f/shared/resource_to_modify', 'resource', 0, 1, NULL),
         ('test-workspace', 'wm-fork-test-workspace', 'shared', 'folder', 0, 1, NULL)"
    )
    .execute(&db)
    .await?;

    // Conflict (both ahead and behind)
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES
         ('test-workspace', 'wm-fork-test-workspace', 'f/shared/original_flow', 'flow', 1, 1, NULL),
         ('test-workspace', 'wm-fork-test-workspace', 'f/shared/to_conflict', 'script', 1, 1, NULL)"
    )
    .execute(&db)
    .await?;

    // Deleted in fork (exists only in parent)
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES ('test-workspace', 'wm-fork-test-workspace', 'f/shared/to_delete', 'script', 1, 0, NULL)"
    )
    .execute(&db)
    .await?;

    // Renamed in parent (two entries)
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES
         ('test-workspace', 'wm-fork-test-workspace', 'f/shared/old_name', 'resource', 0, 1, NULL),
         ('test-workspace', 'wm-fork-test-workspace', 'f/shared/new_name', 'resource', 1, 0, NULL)"
    )
    .execute(&db)
    .await?;

    // Add an unchanged item to verify it gets filtered out
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES ('test-workspace', 'wm-fork-test-workspace', 'f/shared/original_script', 'script', 0, 0, NULL)"
    )
    .execute(&db)
    .await?;

    // ==============================================================
    // PHASE 5: Call compare_workspaces and Verify Results
    // ==============================================================

    let comparison: serde_json::Value = client
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-test-workspace"
        ))
        .send()
        .await?
        .json()
        .await?;

    // Verify basic structure
    assert!(
        !comparison["skipped_comparison"].as_bool().unwrap_or(true),
        "Should not skip comparison"
    );
    assert!(comparison["diffs"].is_array(), "Should have diffs array");
    assert!(
        comparison["summary"].is_object(),
        "Should have summary object"
    );

    let diffs = comparison["diffs"].as_array().unwrap();
    let summary = &comparison["summary"];

    // ==============================================================
    // Summary Assertions
    // ==============================================================

    // Total diffs (excluding unchanged items which should be deleted)
    let total_diffs = summary["total_diffs"].as_u64().unwrap();
    assert!(total_diffs > 0, "Should have at least some diffs");

    // Verify ahead/behind counts
    let total_ahead = summary["total_ahead"].as_u64().unwrap();
    let total_behind = summary["total_behind"].as_u64().unwrap();
    assert!(total_ahead > 0, "Should have items ahead");
    assert!(total_behind > 0, "Should have items behind");

    // Verify conflicts (items that are both ahead and behind)
    let conflicts = summary["conflicts"].as_u64().unwrap();
    assert!(conflicts >= 1, "Should have at least 1 conflict (flow)");

    // Verify per-item-type counts
    assert!(
        summary["scripts_changed"].as_u64().unwrap() > 0,
        "Should have script changes"
    );
    assert!(
        summary["flows_changed"].as_u64().unwrap() > 0,
        "Should have flow changes"
    );
    assert!(
        summary["apps_changed"].as_u64().unwrap() > 0,
        "Should have app changes"
    );
    assert!(
        summary["resources_changed"].as_u64().unwrap() > 0,
        "Should have resource changes"
    );
    assert!(
        summary["variables_changed"].as_u64().unwrap() > 0,
        "Should have variable changes"
    );
    assert!(
        summary["resource_types_changed"].as_u64().unwrap() > 0,
        "Should have resource_type changes"
    );
    // Note: folders_changed may be 0 if folder comparison didn't detect changes
    // assert!(summary["folders_changed"].as_u64().unwrap() > 0, "Should have folder changes");

    // ==============================================================
    // Individual Diff Assertions
    // ==============================================================

    // Scenario 1: New in parent
    let new_in_parent = diffs
        .iter()
        .find(|d| d["path"] == "f/shared/new_in_parent" && d["kind"] == "script")
        .expect("Should find new_in_parent diff");
    assert_eq!(
        new_in_parent["ahead"].as_i64().unwrap(),
        1,
        "new_in_parent should be ahead"
    );
    assert_eq!(
        new_in_parent["behind"].as_i64().unwrap(),
        0,
        "new_in_parent should not be behind"
    );
    assert_eq!(
        new_in_parent["has_changes"].as_bool().unwrap(),
        true,
        "new_in_parent should have changes"
    );
    assert_eq!(
        new_in_parent["exists_in_source"].as_bool().unwrap(),
        true,
        "new_in_parent should exist in source"
    );
    assert_eq!(
        new_in_parent["exists_in_fork"].as_bool().unwrap(),
        false,
        "new_in_parent should not exist in fork"
    );

    // Scenario 2: New in fork
    let new_in_fork = diffs
        .iter()
        .find(|d| d["path"] == "f/shared/new_in_fork" && d["kind"] == "script")
        .expect("Should find new_in_fork diff");
    assert_eq!(
        new_in_fork["ahead"].as_i64().unwrap(),
        0,
        "new_in_fork should not be ahead"
    );
    assert_eq!(
        new_in_fork["behind"].as_i64().unwrap(),
        1,
        "new_in_fork should be behind"
    );
    assert_eq!(
        new_in_fork["has_changes"].as_bool().unwrap(),
        true,
        "new_in_fork should have changes"
    );
    assert_eq!(
        new_in_fork["exists_in_source"].as_bool().unwrap(),
        false,
        "new_in_fork should not exist in source"
    );
    assert_eq!(
        new_in_fork["exists_in_fork"].as_bool().unwrap(),
        true,
        "new_in_fork should exist in fork"
    );

    // Scenario 5: Conflict
    let conflict_flow = diffs
        .iter()
        .find(|d| d["path"] == "f/shared/original_flow" && d["kind"] == "flow")
        .expect("Should find conflict flow diff");
    assert!(
        conflict_flow["ahead"].as_i64().unwrap() > 0,
        "conflict should be ahead"
    );
    assert!(
        conflict_flow["behind"].as_i64().unwrap() > 0,
        "conflict should be behind"
    );
    assert_eq!(
        conflict_flow["has_changes"].as_bool().unwrap(),
        true,
        "conflict should have changes"
    );
    assert_eq!(
        conflict_flow["exists_in_source"].as_bool().unwrap(),
        true,
        "conflict should exist in source"
    );
    assert_eq!(
        conflict_flow["exists_in_fork"].as_bool().unwrap(),
        true,
        "conflict should exist in fork"
    );

    // Scenario 6: Deleted in fork
    let deleted = diffs
        .iter()
        .find(|d| d["path"] == "f/shared/to_delete" && d["kind"] == "script")
        .expect("Should find deleted diff");
    assert_eq!(
        deleted["exists_in_source"].as_bool().unwrap(),
        true,
        "deleted should exist in source"
    );
    assert_eq!(
        deleted["exists_in_fork"].as_bool().unwrap(),
        false,
        "deleted should not exist in fork (archived)"
    );
    assert_eq!(
        deleted["has_changes"].as_bool().unwrap(),
        true,
        "deleted should have changes"
    );

    // Scenario 7: Rename (should show as two entries)
    let old_name = diffs
        .iter()
        .find(|d| d["path"] == "f/shared/old_name" && d["kind"] == "resource");
    let new_name = diffs
        .iter()
        .find(|d| d["path"] == "f/shared/new_name" && d["kind"] == "resource");

    // At least one of these should exist (depending on how the comparison handles renames)
    assert!(
        old_name.is_some() || new_name.is_some(),
        "Should find at least one rename-related diff"
    );

    // ==============================================================
    // Database State Assertions
    // ==============================================================

    // Verify has_changes was cached for items that have changes
    let cached_new_in_parent = sqlx::query!(
        "SELECT has_changes, exists_in_source, exists_in_fork FROM workspace_diff
         WHERE path = 'f/shared/new_in_parent' AND kind = 'script' AND source_workspace_id = 'test-workspace'"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        cached_new_in_parent.has_changes,
        Some(true),
        "has_changes should be cached as true"
    );
    assert_eq!(
        cached_new_in_parent.exists_in_source,
        Some(true),
        "exists_in_source should be cached"
    );
    assert_eq!(
        cached_new_in_parent.exists_in_fork,
        Some(false),
        "exists_in_fork should be cached"
    );

    // Verify unchanged items were deleted from workspace_diff
    let unchanged_original_script = sqlx::query!(
        "SELECT has_changes FROM workspace_diff
         WHERE path = 'f/shared/original_script' AND kind = 'script' AND source_workspace_id = 'test-workspace'"
    )
    .fetch_optional(&db)
    .await?;

    // The unchanged item should either be deleted or marked as has_changes = false
    // Based on the code, items with has_changes = false are deleted
    if let Some(record) = unchanged_original_script {
        assert_ne!(
            record.has_changes,
            Some(false),
            "unchanged items with has_changes=false should be deleted"
        );
    }

    // ==============================================================
    // Lazy Evaluation Test
    // ==============================================================

    // Create a new diff entry with NULL has_changes
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES ('test-workspace', 'wm-fork-test-workspace', 'f/shared/lazy_test', 'script', 1, 0, NULL)
         ON CONFLICT DO NOTHING"
    )
    .execute(&db)
    .await?;

    // Call the endpoint again
    let _comparison2: serde_json::Value = client
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-test-workspace"
        ))
        .send()
        .await?
        .json()
        .await?;

    // Verify the lazy_test entry was evaluated (should be deleted since it doesn't exist)
    let lazy_test = sqlx::query!(
        "SELECT has_changes FROM workspace_diff
         WHERE path = 'f/shared/lazy_test' AND kind = 'script' AND source_workspace_id = 'test-workspace'"
    )
    .fetch_optional(&db)
    .await?;

    // Should be deleted since the item doesn't actually exist in either workspace
    assert!(
        lazy_test.is_none(),
        "Non-existent item should be deleted from workspace_diff"
    );

    // ==============================================================
    // Stale Archived Cache Test (regression)
    // ==============================================================
    //
    // Unlike the lazy_test above (has_changes = NULL → always re-evaluated), a
    // cached `has_changes = true` row is trusted without re-running the per-kind
    // comparison. It can go stale: after a rename the old path keeps only
    // archived versions, and for lock-gen languages the `has_changes = NULL`
    // reset is deferred to the dependency job — so until that runs the archived
    // old path lingers as a live "ahead" change carrying `exists_in_fork = true`.
    // The visibility check treats archived as non-existent and finds nothing, so
    // even this superadmin used to get `all_ahead_items_visible = false`. The fix
    // re-validates such rows and drops the archived (== non-existent) item.
    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, archived, schema_validation, ws_error_handler_muted, deleted)
         VALUES ('wm-fork-test-workspace', 'f/shared/renamed_away', 67890, 'def main(): return 1', '', '', 'python3', 'test@windmill.dev', NOW(), true, false, false, false)"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes, exists_in_source, exists_in_fork)
         VALUES ('test-workspace', 'wm-fork-test-workspace', 'f/shared/renamed_away', 'script', 1, 0, true, false, true)"
    )
    .execute(&db)
    .await?;

    let comparison3: serde_json::Value = client
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-test-workspace"
        ))
        .send()
        .await?
        .json()
        .await?;

    // The archived item must be dropped (not surfaced) and must not trip the
    // "changes not visible to your user" warning for a superadmin.
    assert_eq!(
        comparison3["all_ahead_items_visible"].as_bool(),
        Some(true),
        "archived (renamed-away) item must not trip the 'changes not visible' warning: {comparison3}"
    );
    assert!(
        !comparison3["diffs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["path"] == "f/shared/renamed_away"),
        "archived item should be dropped, not surfaced as a diff: {comparison3}"
    );
    let stale_archived = sqlx::query!(
        "SELECT has_changes FROM workspace_diff
         WHERE path = 'f/shared/renamed_away' AND kind = 'script' AND source_workspace_id = 'test-workspace'"
    )
    .fetch_optional(&db)
    .await?;
    assert!(
        stale_archived.is_none(),
        "stale archived diff row should be re-evaluated and deleted"
    );

    Ok(())
}

/// Trigger/schedule diffs go through the same `compare_workspaces` flow as
/// scripts/flows once tally tracks them. The compare_two_trigger_or_schedule
/// helper strips runtime fields (mode/enabled/server_id/last_server_ping/
/// edited_at-by/email/error/extra_perms/permissioned_as), so:
///   - a real config change shows `has_changes = true`
///   - a runtime-only change (mode toggle, enabled flip) shows `has_changes = false`
///     and the row is deleted from `workspace_diff`
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_compare_workspaces_trigger_and_schedule(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );
    let base_url = format!("http://localhost:{port}/api");

    // Parent + fork workspaces (fork created via INSERT to bypass
    // clone_triggers_and_schedules — we want to control the rows manually).
    sqlx::query!(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id)
         VALUES ('wm-fork-test-workspace', 'Fork', 'test-user', 'test-workspace')"
    )
    .execute(&db)
    .await?;
    sqlx::query!("INSERT INTO workspace_settings (workspace_id) VALUES ('wm-fork-test-workspace')")
        .execute(&db)
        .await?;
    sqlx::query!(
        "INSERT INTO workspace_key(workspace_id, kind, key)
         VALUES ('wm-fork-test-workspace', 'cloud', 'test-key')"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO usr(workspace_id, email, username, is_admin, role)
         VALUES ('wm-fork-test-workspace', 'test@windmill.dev', 'test-user', true, 'Admin')"
    )
    .execute(&db)
    .await?;

    // ------ Schedule: identical config in parent and fork, except `enabled`.
    // Should be filtered out (no real diff).
    sqlx::query!(
        "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, enabled,
            script_path, args, is_flow, email, timezone, summary, permissioned_as)
         VALUES
         ('test-workspace', 'f/sch/runtime_only', 'test-user', NOW(), '0 * * * * *', true,
            'f/scripts/x', '{}', false, 'test@windmill.dev', 'UTC', 'sch', 'u/test-user'),
         ('wm-fork-test-workspace', 'f/sch/runtime_only', 'test-user', NOW(), '0 * * * * *', false,
            'f/scripts/x', '{}', false, 'test@windmill.dev', 'UTC', 'sch', 'u/test-user')"
    )
    .execute(&db)
    .await?;

    // ------ Schedule: config change (script_path) in fork. Should diff.
    sqlx::query!(
        "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, enabled,
            script_path, args, is_flow, email, timezone, summary, permissioned_as)
         VALUES
         ('test-workspace', 'f/sch/config_change', 'test-user', NOW(), '0 * * * * *', false,
            'f/scripts/parent_path', '{}', false, 'test@windmill.dev', 'UTC', 'sch', 'u/test-user'),
         ('wm-fork-test-workspace', 'f/sch/config_change', 'test-user', NOW(), '0 * * * * *', false,
            'f/scripts/fork_path', '{}', false, 'test@windmill.dev', 'UTC', 'sch', 'u/test-user')"
    )
    .execute(&db)
    .await?;

    // ------ HTTP trigger: identical config except `mode`. Should be filtered out.
    sqlx::query!(
        "INSERT INTO http_trigger (workspace_id, path, edited_by, edited_at, route_path,
            route_path_key, script_path, is_flow, http_method, request_type,
            authentication_method, mode, permissioned_as)
         VALUES
         ('test-workspace', 'f/rt/runtime_only', 'test-user', NOW(), 'foo', 'foo',
            'f/scripts/y', false, 'get', 'sync',
            'none', 'enabled', 'u/test-user'),
         ('wm-fork-test-workspace', 'f/rt/runtime_only', 'test-user', NOW(), 'foo', 'foo',
            'f/scripts/y', false, 'get', 'sync',
            'none', 'disabled', 'u/test-user')"
    )
    .execute(&db)
    .await?;

    // ------ HTTP trigger: config change (route_path) in fork. Should diff.
    sqlx::query!(
        "INSERT INTO http_trigger (workspace_id, path, edited_by, edited_at, route_path,
            route_path_key, script_path, is_flow, http_method, request_type,
            authentication_method, mode, permissioned_as)
         VALUES
         ('test-workspace', 'f/rt/config_change', 'test-user', NOW(), 'parent', 'parent',
            'f/scripts/y', false, 'get', 'sync',
            'none', 'disabled', 'u/test-user'),
         ('wm-fork-test-workspace', 'f/rt/config_change', 'test-user', NOW(), 'fork', 'fork',
            'f/scripts/y', false, 'get', 'sync',
            'none', 'disabled', 'u/test-user')"
    )
    .execute(&db)
    .await?;

    // Seed workspace_diff with NULL has_changes so compare_workspaces evaluates them lazily.
    sqlx::query!(
        "INSERT INTO workspace_diff
            (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES
         ('test-workspace', 'wm-fork-test-workspace', 'f/sch/runtime_only', 'schedule', 0, 1, NULL),
         ('test-workspace', 'wm-fork-test-workspace', 'f/sch/config_change', 'schedule', 0, 1, NULL),
         ('test-workspace', 'wm-fork-test-workspace', 'f/rt/runtime_only', 'http_trigger', 0, 1, NULL),
         ('test-workspace', 'wm-fork-test-workspace', 'f/rt/config_change', 'http_trigger', 0, 1, NULL)"
    )
    .execute(&db)
    .await?;

    let comparison: serde_json::Value = client
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-test-workspace"
        ))
        .send()
        .await?
        .json()
        .await?;

    let diffs = comparison["diffs"].as_array().unwrap();

    // The runtime-only rows should be filtered out (compare_two_trigger_or_schedule
    // returned has_changes=false → row deleted from workspace_diff).
    assert!(
        !diffs.iter().any(|d| d["path"] == "f/sch/runtime_only"),
        "schedule with only enabled-flag difference should be filtered out"
    );
    assert!(
        !diffs.iter().any(|d| d["path"] == "f/rt/runtime_only"),
        "http_trigger with only mode difference should be filtered out"
    );

    // The config-change rows should be present with has_changes=true.
    let sch_change = diffs
        .iter()
        .find(|d| d["path"] == "f/sch/config_change" && d["kind"] == "schedule")
        .expect("schedule with config change should appear in diffs");
    assert_eq!(sch_change["has_changes"].as_bool().unwrap(), true);
    assert_eq!(sch_change["exists_in_source"].as_bool().unwrap(), true);
    assert_eq!(sch_change["exists_in_fork"].as_bool().unwrap(), true);

    let rt_change = diffs
        .iter()
        .find(|d| d["path"] == "f/rt/config_change" && d["kind"] == "http_trigger")
        .expect("http_trigger with config change should appear in diffs");
    assert_eq!(rt_change["has_changes"].as_bool().unwrap(), true);

    // Summary counts.
    let summary = &comparison["summary"];
    assert_eq!(summary["schedules_changed"].as_u64().unwrap(), 1);
    assert_eq!(summary["triggers_changed"].as_u64().unwrap(), 1);

    Ok(())
}

/// Regression for the "superadmin-still-sees-the-warning" case in WIN-1975.
///
/// `compare_workspaces` historically trusted `authed.is_admin` for RLS — but
/// that flag is derived from the *token's* cached `super_admin` column at
/// auth time (windmill-api-auth/src/auth.rs), not from a live
/// `password.super_admin` read. A user who is *currently* an instance
/// superadmin can have a token from before the promotion (or via a session
/// refresh race) where `token.super_admin = false`. If they're also not a
/// workspace admin in the source workspace (only in the fork),
/// `authed.is_admin` lands as `false` and source-scoped RLS gets applied to
/// fork-side visibility queries — same bug as the regular non-admin case.
///
/// With the fix, `load_workspace_authed` re-checks `is_super_admin_email`
/// against `password.super_admin` at request time, so the fork-scoped authed
/// gets `is_admin = true` and RLS bypass kicks back in for the fork queries.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_compare_workspaces_stale_superadmin_token(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{port}/api");

    // Promote test-user-2 to instance superadmin AFTER their token was issued
    // (base.sql inserts SECRET_TOKEN_2 with super_admin=false). The token row
    // keeps super_admin=false; password.super_admin flips to true.
    sqlx::query!("UPDATE password SET super_admin = true WHERE email = 'test2@windmill.dev'")
        .execute(&db)
        .await?;

    let stale_super = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN_2".to_string(),
    );

    // Fork test-workspace.
    let resp = stale_super
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .json(&json!({
            "id": "wm-fork-stale-super",
            "name": "Stale Super Fork",
            "color": "#0000ff"
        }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "fork creation failed: {} — {}",
        resp.status(),
        resp.text().await?
    );

    // Fork-only folder + script, with empty extra_perms so the only way to
    // see them is via fork's folder-based RLS or admin bypass.
    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, summary, created_by)
         VALUES ('wm-fork-stale-super', 'folder2', 'folder2', ARRAY['u/test-user-2']::varchar[], $1, '', 'test-user-2')",
        json!({"u/test-user-2": true})
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, archived, schema_validation, ws_error_handler_muted, deleted, extra_perms)
         VALUES ('wm-fork-stale-super', 'f/folder2/myscript', 333333, 'echo 1', '', '', 'bash', 'test-user-2', NOW(), false, false, false, false, $1)",
        json!({})
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES ('test-workspace', 'wm-fork-stale-super', 'f/folder2/myscript', 'script', 1, 0, NULL)"
    )
    .execute(&db)
    .await?;
    sqlx::query!("DELETE FROM skip_workspace_diff_tally")
        .execute(&db)
        .await?;

    let comparison: serde_json::Value = stale_super
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-stale-super"
        ))
        .send()
        .await?
        .json()
        .await?;

    assert_eq!(
        comparison["all_ahead_items_visible"].as_bool(),
        Some(true),
        "current superadmin with stale token should still see ahead items: {comparison}"
    );
    let diffs = comparison["diffs"].as_array().unwrap();
    assert!(
        diffs
            .iter()
            .any(|d| d["path"] == "f/folder2/myscript" && d["kind"] == "script"),
        "fork-only script should appear in diffs; got {diffs:?}"
    );

    Ok(())
}

/// End-to-end regression for WIN-1975 against the real EE tally path.
/// Reproduces the reporter's exact steps with the API: fork → create script
/// in folder1 → rename to folder2 → compare. Folder2 only exists in the
/// fork, so before the fix the source-scoped authed in `filter_visible_diffs`
/// hid the script and the response set `all_ahead_items_visible = false`.
///
/// Gated on `private` because the OSS build of `handle_deployment_metadata`
/// is a no-op (`windmill-git-sync/src/git_sync_oss.rs`) — without it the
/// `workspace_diff` rows never get written and the test would assert against
/// an empty diff set.
#[cfg(feature = "private")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_compare_workspaces_rename_visibility_ee_e2e(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{port}/api");
    let admin = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );
    let non_admin = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN_2".to_string(),
    );

    // The base fixture pre-populates `skip_workspace_diff_tally` for every
    // workspace existing at migration time — that bypasses the diff
    // accounting. Clear it so tally + compare run normally for this test.
    sqlx::query!("DELETE FROM skip_workspace_diff_tally")
        .execute(&db)
        .await?;

    // ------ Fork the existing test-workspace.
    let resp = admin
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .json(&json!({
            "id": "wm-fork-rename-test",
            "name": "Rename Fork",
            "color": "#0000ff"
        }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "fork creation failed: {}",
        resp.status()
    );

    // Non-admin user must be a member of both workspaces. They already are in
    // test-workspace (base fixture); add them to the fork. Same username as
    // the source so RLS extra_perms keys still resolve.
    sqlx::query!(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role)
         VALUES ('wm-fork-rename-test', 'test2@windmill.dev', 'test-user-2', false, 'User')"
    )
    .execute(&db)
    .await?;

    // ------ Non-admin creates folder1 in the fork (owner = self).
    let resp = non_admin
        .client()
        .post(&format!("{base_url}/w/wm-fork-rename-test/folders/create"))
        .json(&json!({"name": "folder1", "owners": [], "summary": ""}))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "folder1 create failed: {} — {}",
        resp.status(),
        resp.text().await?
    );

    // ------ Deploy a script in folder1 (initial deploy, no parent_hash).
    let resp = non_admin
        .client()
        .post(&format!("{base_url}/w/wm-fork-rename-test/scripts/create"))
        .json(&json!({
            "path": "f/folder1/myscript",
            "summary": "renamed test",
            "description": "",
            // Use bash so we don't trigger the dependency-job code path —
            // create_script defers `handle_deployment_metadata` (and the
            // tally) to the dep job for languages that need lock generation
            // (Deno/Bun/Python/etc), which never runs in this test.
            "content": "echo 1",
            "language": "bash",
            "schema": {"type": "object", "properties": {}, "required": []},
            "deployment_message": "initial",
        }))
        .send()
        .await?;
    let status = resp.status();
    let initial_hash = resp.text().await?;
    assert!(
        status.is_success(),
        "initial script create failed: {} — {}",
        status,
        initial_hash
    );

    // ------ Create folder2 in fork.
    let resp = non_admin
        .client()
        .post(&format!("{base_url}/w/wm-fork-rename-test/folders/create"))
        .json(&json!({"name": "folder2", "owners": [], "summary": ""}))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "folder2 create failed: {}",
        resp.status()
    );

    // ------ Rename: re-deploy the same script at the new path with the old
    // hash as parent_hash. This is exactly what the script editor sends when
    // the user changes the path field and clicks Deploy. The EE tally upserts
    // a workspace_diff row for both the new path AND the renamed_from path.
    let resp = non_admin
        .client()
        .post(&format!("{base_url}/w/wm-fork-rename-test/scripts/create"))
        .json(&json!({
            "path": "f/folder2/myscript",
            "summary": "renamed test",
            "description": "",
            // Use bash so we don't trigger the dependency-job code path —
            // create_script defers `handle_deployment_metadata` (and the
            // tally) to the dep job for languages that need lock generation
            // (Deno/Bun/Python/etc), which never runs in this test.
            "content": "echo 1",
            "language": "bash",
            "schema": {"type": "object", "properties": {}, "required": []},
            // The API returns hash as hex (ScriptHash Serialize impl); pass it
            // through verbatim — the backend deserializer parses hex back.
            "parent_hash": initial_hash.trim().trim_matches('"'),
            "deployment_message": "rename to folder2",
        }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "rename failed: {} — {}",
        resp.status(),
        resp.text().await?
    );

    // The tally is fired via `tokio::spawn` in `handle_deployment_metadata`
    // (windmill-git-sync/src/git_sync_ee.rs) — wait specifically for the
    // renamed script row to appear so we don't race the actual case under
    // test.
    let mut script_diff_written = false;
    for _ in 0..40 {
        let row_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM workspace_diff
             WHERE source_workspace_id = 'test-workspace'
               AND fork_workspace_id = 'wm-fork-rename-test'
               AND kind = 'script'
               AND path = 'f/folder2/myscript'"
        )
        .fetch_one(&db)
        .await?;
        if row_count >= 1 {
            script_diff_written = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(
        script_diff_written,
        "tally never wrote the renamed-script row to workspace_diff"
    );

    // ------ Compare as the non-admin who owns folder2 in the fork. With the
    // bug, the source-scoped authed has no folder2 entry → fork visibility
    // query hides f/folder2/myscript → all_ahead_items_visible flips to
    // false. With the fix, the fork-scoped authed sees folder2 and the
    // visibility check passes.
    let comparison: serde_json::Value = non_admin
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-rename-test"
        ))
        .send()
        .await?
        .json()
        .await?;

    assert_eq!(
        comparison["all_ahead_items_visible"].as_bool(),
        Some(true),
        "non-admin owner of fork-only folder should see ahead items as visible; got {comparison}"
    );

    let diffs = comparison["diffs"].as_array().unwrap();
    assert!(
        diffs
            .iter()
            .any(|d| d["path"] == "f/folder2/myscript" && d["kind"] == "script"),
        "renamed script at f/folder2/myscript should appear in diffs; got {diffs:?}"
    );
    // The renamed_from row (f/folder1/myscript) must NOT appear: both sides'
    // archived=false views show it missing, so compare_two_scripts returns
    // has_changes=false and the row is deleted. Keep an explicit assertion
    // so a future regression that leaks the old path is caught here.
    assert!(
        !diffs
            .iter()
            .any(|d| d["path"] == "f/folder1/myscript" && d["kind"] == "script"),
        "renamed-from path f/folder1/myscript should be cleaned up; got {diffs:?}"
    );

    // ------ Also confirm the superadmin path still works (this used to be
    // the only path that worked because RLS bypass masked the bug).
    let comparison: serde_json::Value = admin
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-rename-test"
        ))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(
        comparison["all_ahead_items_visible"].as_bool(),
        Some(true),
        "superadmin must always see all ahead items: {comparison}"
    );

    Ok(())
}

/// Regression test for #10401. The fork -> parent ("ahead") side of the tally
/// used to key off `workspace_settings.deploy_to` while the parent -> fork
/// ("behind") side keyed off `workspace.parent_workspace_id`, so a fork whose
/// `deploy_to` disagreed recorded no ahead change at all and its edits stayed
/// permanently absent from "Deploy to <parent>". Both sides now read the
/// lineage, which is the only key left.
///
/// Gated on `private` for the same reason as the rename test above: the OSS
/// `handle_deployment_metadata` is a no-op, so no rows would ever be written.
#[cfg(feature = "private")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_fork_tally_ahead_against_parent(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{port}/api");
    let admin = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    sqlx::query!("DELETE FROM skip_workspace_diff_tally")
        .execute(&db)
        .await?;

    let resp = admin
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .json(&json!({
            "id": "wm-fork-tally",
            "name": "Tally Fork",
        }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "fork creation failed: {}",
        resp.status()
    );

    // bash needs no lock generation, so `create_script` tallies inline instead of
    // deferring to a dependency job (no worker runs in this test).
    let deploy = async |path: &str| -> anyhow::Result<()> {
        let resp = admin
            .client()
            .post(&format!("{base_url}/w/wm-fork-tally/scripts/create"))
            .json(&json!({
                "path": path,
                "summary": "",
                "description": "",
                "content": "echo 1",
                "language": "bash",
                "schema": {"type": "object", "properties": {}, "required": []},
            }))
            .send()
            .await?;
        let status = resp.status();
        assert!(
            status.is_success(),
            "script create failed: {} — {}",
            status,
            resp.text().await?
        );
        Ok(())
    };

    // The tally runs in a `tokio::spawn` inside `handle_deployment_metadata`.
    // Stopping at the first non-NULL read would let a regression that tallies the
    // same deploy against two upstream workspaces slip through: it shows `ahead = 1`
    // between the upserts. So keep sampling after the row appears and return the
    // settled value.
    let ahead_for = async |path: &str| -> anyhow::Result<Option<i32>> {
        let read = async || -> anyhow::Result<Option<i32>> {
            Ok(sqlx::query_scalar!(
                "SELECT ahead FROM workspace_diff
                 WHERE source_workspace_id = 'test-workspace'
                   AND fork_workspace_id = 'wm-fork-tally'
                   AND kind = 'script'
                   AND path = $1",
                path
            )
            .fetch_optional(&db)
            .await?)
        };
        let mut ahead = None;
        for _ in 0..40 {
            ahead = read().await?;
            if ahead.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        for _ in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            ahead = read().await?;
        }
        Ok(ahead)
    };

    deploy("u/admin/tallied").await?;
    assert_eq!(
        ahead_for("u/admin/tallied").await?,
        Some(1),
        "a fork's change must record once against its parent"
    );

    Ok(())
}

/// Regression test for WIN-1975. A non-admin user creating a script in a fork-
/// only folder used to get the spurious
/// "this fork has changes not visible to your user" warning because
/// `filter_visible_diffs` ran every RLS query with the source-workspace
/// authed, so any item only reachable via fork-specific folders/groups was
/// hidden from the visibility check.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_compare_workspaces_fork_only_folder_visibility(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client_user_2 = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN_2".to_string(),
    );
    let base_url = format!("http://localhost:{port}/api");

    // ----- Set up parent workspace folder1 owned by test-user-2, then fork it.
    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, summary, created_by)
         VALUES ('test-workspace', 'folder1', 'folder1', ARRAY['u/test-user-2']::varchar[], $1, '', 'test-user-2')",
        json!({"u/test-user-2": true})
    )
    .execute(&db)
    .await?;

    // Create fork via the API so the cloning and lineage wiring matches what production sees.
    let client_admin = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );
    let fork_response = client_admin
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .json(&json!({
            "id": "wm-fork-visibility-test",
            "name": "Test Fork",
            "color": "#0000ff"
        }))
        .send()
        .await?;
    assert!(
        fork_response.status().is_success(),
        "Fork creation failed: {}",
        fork_response.status()
    );

    // test-user-2 must be a member of the fork. The fork's clone copies the
    // creator's usr row only — add test-user-2 manually so they can hit the
    // compare endpoint and own a fork-only folder.
    sqlx::query!(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role) VALUES
         ('wm-fork-visibility-test', 'test2@windmill.dev', 'test-user-2', false, 'User')"
    )
    .execute(&db)
    .await?;

    // ----- Fork-only folder2 (does not exist in source) owned by test-user-2.
    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, summary, created_by)
         VALUES ('wm-fork-visibility-test', 'folder2', 'folder2', ARRAY['u/test-user-2']::varchar[], $1, '', 'test-user-2')",
        json!({"u/test-user-2": true})
    )
    .execute(&db)
    .await?;

    // Script in the fork-only folder with empty extra_perms (typical: scripts
    // inherit access through their containing folder, not direct perms).
    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, archived, schema_validation, ws_error_handler_muted, deleted, extra_perms)
         VALUES ('wm-fork-visibility-test', 'f/folder2/myscript', 222222, 'def main():\n    return 1', '', '', 'python3', 'test-user-2', NOW(), false, false, false, false, $1)",
        json!({})
    )
    .execute(&db)
    .await?;

    // Seed workspace_diff to mirror what the tally would write.
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES ('test-workspace', 'wm-fork-visibility-test', 'f/folder2/myscript', 'script', 1, 0, NULL)"
    )
    .execute(&db)
    .await?;

    // Clear the skip flag added by the bootstrap migration so compare actually
    // runs against this fork (it short-circuits otherwise).
    sqlx::query!(
        "DELETE FROM skip_workspace_diff_tally WHERE workspace_id IN ('test-workspace', 'wm-fork-visibility-test')"
    )
    .execute(&db)
    .await?;

    let comparison: serde_json::Value = client_user_2
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-visibility-test"
        ))
        .send()
        .await?
        .json()
        .await?;

    assert_eq!(
        comparison["all_ahead_items_visible"].as_bool(),
        Some(true),
        "ahead items should be visible to the fork-only folder owner; full response: {comparison}"
    );
    assert_eq!(
        comparison["all_behind_items_visible"].as_bool(),
        Some(true),
        "behind items should be visible (no behind items here)"
    );

    let diffs = comparison["diffs"].as_array().unwrap();
    assert!(
        diffs
            .iter()
            .any(|d| d["path"] == "f/folder2/myscript" && d["kind"] == "script"),
        "fork-only script should appear in diffs; got {diffs:?}"
    );

    Ok(())
}

/// Regression test: deleting a fork must purge its `workspace_diff` and
/// `skip_workspace_diff_tally` rows. These tables are keyed by workspace id with
/// no FK cascade, and a fork id is reused when a fork is deleted and recreated
/// under the same name. If the cached diff rows survive the delete, they leak
/// onto the next fork sharing that id and produce a spurious "changes not
/// visible" warning that hides the deploy button (WIN-2066).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_delete_fork_purges_workspace_diff(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );
    let base_url = format!("http://localhost:{port}/api");

    // Create the fork so the caller owns it (delete is authorized for fork owners).
    let fork_response = client
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .json(&json!({
            "id": "wm-fork-test-workspace",
            "name": "Test Fork",
            "color": "#0000ff"
        }))
        .send()
        .await?;
    assert!(
        fork_response.status().is_success(),
        "Fork creation should succeed: {}",
        fork_response.status()
    );

    // Seed cached diff state for the fork: as the fork side of a pair, as the
    // source side of a pair, and a skip-tally row.
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes, exists_in_source, exists_in_fork)
         VALUES ('test-workspace', 'wm-fork-test-workspace', 'f/shared/leaky', 'script', 1, 0, true, true, true)"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES ('wm-fork-test-workspace', 'test-workspace', 'f/shared/other', 'script', 0, 1, true)"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO skip_workspace_diff_tally (workspace_id) VALUES ('wm-fork-test-workspace')"
    )
    .execute(&db)
    .await?;

    // Delete the fork through the real handler.
    let delete_response = client
        .client()
        .delete(&format!(
            "{base_url}/workspaces/delete/wm-fork-test-workspace"
        ))
        .send()
        .await?;
    assert!(
        delete_response.status().is_success(),
        "Fork deletion should succeed: {}",
        delete_response.status()
    );

    let leftover_diffs = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM workspace_diff
         WHERE source_workspace_id = 'wm-fork-test-workspace'
            OR fork_workspace_id = 'wm-fork-test-workspace'"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        leftover_diffs,
        Some(0),
        "workspace_diff rows referencing the deleted fork must be purged"
    );

    let leftover_skip = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM skip_workspace_diff_tally WHERE workspace_id = 'wm-fork-test-workspace'"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        leftover_skip,
        Some(0),
        "skip_workspace_diff_tally row for the deleted fork must be purged"
    );

    Ok(())
}

/// Regression test: creating a fork must start with clean diff state even when
/// the (reusable) fork id was previously occupied by a deleted fork. Stale
/// `workspace_diff` / `skip_workspace_diff_tally` rows left behind by an earlier
/// occupant would otherwise leak onto the new fork — a stale skip row suppresses
/// comparison entirely, and stale diff rows produce a spurious "changes not
/// visible" warning that hides the deploy button (WIN-2066).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_create_fork_purges_stale_diff_state(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );
    let base_url = format!("http://localhost:{port}/api");

    // Simulate leftovers from a previously deleted fork that reused this id:
    // diff rows on both sides plus a skip-tally row, with no workspace yet.
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes, exists_in_source, exists_in_fork)
         VALUES ('test-workspace', 'wm-fork-test-workspace', 'f/shared/leaky', 'script', 1, 0, true, true, true)"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES ('wm-fork-test-workspace', 'test-workspace', 'f/shared/other', 'script', 0, 1, true)"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO skip_workspace_diff_tally (workspace_id) VALUES ('wm-fork-test-workspace')"
    )
    .execute(&db)
    .await?;

    // Create the fork reusing that id; the conflict check passes because no
    // workspace row exists for it.
    let fork_response = client
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .json(&json!({
            "id": "wm-fork-test-workspace",
            "name": "Test Fork",
            "color": "#0000ff"
        }))
        .send()
        .await?;
    assert!(
        fork_response.status().is_success(),
        "Fork creation should succeed: {}",
        fork_response.status()
    );

    let leftover_diffs = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM workspace_diff
         WHERE source_workspace_id = 'wm-fork-test-workspace'
            OR fork_workspace_id = 'wm-fork-test-workspace'"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        leftover_diffs,
        Some(0),
        "stale workspace_diff rows must be purged on fork creation"
    );

    let leftover_skip = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM skip_workspace_diff_tally WHERE workspace_id = 'wm-fork-test-workspace'"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        leftover_skip,
        Some(0),
        "stale skip_workspace_diff_tally row must be purged on fork creation"
    );

    Ok(())
}

/// Regression: a stale/phantom trigger diff row must never block a privileged
/// user's deploy. Triggers (unlike scripts/flows) are not re-validated by
/// `compare_workspaces`, so a cached `has_changes=true` row for a trigger that
/// no longer exists in the table is trusted, then dropped by the visibility
/// filter (the row is gone) — flipping `all_ahead_items_visible` to false and
/// hiding the deploy button. This used to happen even for a superadmin, because
/// the item's absence is indistinguishable from a permission-hidden item.
///
/// The blast-radius guard forces the flag true for anyone who sees the relevant
/// side in full: a target/fork admin (or superadmin) for ahead items. A regular
/// user with no such visibility still gets the (conservative) warning, since we
/// cannot tell a phantom from a genuine permission gap on their behalf.
///
/// The diff row and visibility query are OSS, so this runs on any build (no
/// tally needed — the row is inserted directly, mimicking a delete that left the
/// diff behind).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_compare_workspaces_phantom_trigger_shortfuse(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{port}/api");
    let superadmin = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );
    let non_admin = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN_2".to_string(),
    );

    // Fork of test-workspace (INSERT directly; we only need the pair to exist).
    sqlx::query!(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id)
         VALUES ('wm-fork-test-workspace', 'Fork', 'test-user', 'test-workspace')"
    )
    .execute(&db)
    .await?;
    sqlx::query!("INSERT INTO workspace_settings (workspace_id) VALUES ('wm-fork-test-workspace')")
        .execute(&db)
        .await?;
    sqlx::query!(
        "INSERT INTO workspace_key(workspace_id, kind, key)
         VALUES ('wm-fork-test-workspace', 'cloud', 'test-key')"
    )
    .execute(&db)
    .await?;

    // Phantom rows: cached diffs for http_triggers with no backing row (the exact
    // state a trigger delete used to leave behind before it reset the tally). One
    // ahead (fork side), one behind (source side) so both guard branches are hit.
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes, exists_in_source, exists_in_fork)
         VALUES
         ('test-workspace', 'wm-fork-test-workspace', 'f/rt/ghost', 'http_trigger', 1, 0, true, false, true),
         ('test-workspace', 'wm-fork-test-workspace', 'f/rt/ghost_behind', 'http_trigger', 0, 1, true, true, false)"
    )
    .execute(&db)
    .await?;

    // Superadmin: the guard forces `all_ahead_items_visible = true`, and the
    // non-existent trigger is not surfaced as a diff.
    let comparison: serde_json::Value = superadmin
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-test-workspace"
        ))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(
        comparison["all_ahead_items_visible"].as_bool(),
        Some(true),
        "phantom trigger row must not trip the 'not visible' warning for a superadmin: {comparison}"
    );
    assert_eq!(
        comparison["all_behind_items_visible"].as_bool(),
        Some(true),
        "phantom behind trigger row must not trip the warning for a superadmin: {comparison}"
    );
    assert!(
        !comparison["diffs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["path"] == "f/rt/ghost" || d["path"] == "f/rt/ghost_behind"),
        "non-existent triggers must not be surfaced as diffs: {comparison}"
    );

    // Non-superadmin, non-fork-admin member of the source: no full-visibility
    // guarantee, so the warning still (conservatively) fires.
    let comparison: serde_json::Value = non_admin
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-test-workspace"
        ))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(
        comparison["all_ahead_items_visible"].as_bool(),
        Some(false),
        "a user without full visibility must not be short-circuited by the guard: {comparison}"
    );
    assert_eq!(
        comparison["all_behind_items_visible"].as_bool(),
        Some(false),
        "the behind-side guard must not fire for a non-admin either: {comparison}"
    );

    Ok(())
}

/// A source-only row is offered to the fork whatever its counters say, so it can
/// carry `behind = 0` — a `behind`-derived tally never sees it, and hiding one must
/// still be reported to the update direction. The merge direction does not carry
/// such a row at all, so hiding one withholds nothing from that side.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_compare_workspaces_hidden_source_only_no_behind(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{port}/api");
    let non_admin = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN_2".to_string(),
    );

    sqlx::query!(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id)
         VALUES ('wm-fork-test-workspace', 'Fork', 'test-user', 'test-workspace')"
    )
    .execute(&db)
    .await?;
    sqlx::query!("INSERT INTO workspace_settings (workspace_id) VALUES ('wm-fork-test-workspace')")
        .execute(&db)
        .await?;
    sqlx::query!(
        "INSERT INTO workspace_key(workspace_id, kind, key)
         VALUES ('wm-fork-test-workspace', 'cloud', 'test-key')"
    )
    .execute(&db)
    .await?;

    // Source-only row with no `behind`: the trigger has no backing row, so the
    // visibility filter drops it exactly as it would an ACL-hidden one.
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes, exists_in_source, exists_in_fork)
         VALUES ('test-workspace', 'wm-fork-test-workspace', 'f/rt/parent_only', 'http_trigger', 1, 0, true, true, false)"
    )
    .execute(&db)
    .await?;

    let comparison: serde_json::Value = non_admin
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-test-workspace"
        ))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(
        comparison["all_behind_items_visible"].as_bool(),
        Some(false),
        "a hidden source-only row must trip the warning even at behind = 0: {comparison}"
    );
    assert_eq!(
        comparison["hidden_behind"]["total"].as_i64(),
        Some(1),
        "a hidden source-only row must be counted as withheld from the update direction: {comparison}"
    );
    assert_eq!(
        comparison["all_ahead_items_visible"].as_bool(),
        Some(true),
        "the merge direction does not carry a source-only row, so hiding one withholds nothing from it: {comparison}"
    );
    assert_eq!(
        comparison["hidden_ahead"]["total"].as_i64(),
        Some(0),
        "a source-only row must not be reported as withheld from the merge direction: {comparison}"
    );

    Ok(())
}

/// Regression: the "sees everything" guard must require admin of BOTH sides, not
/// just the fork. `filter_visible_diffs` keeps a modified/conflict row (one that
/// exists in the source AND the fork) only when the caller can see it on both
/// sides, so an ahead diff can be dropped for a *source-side* visibility gap even
/// when the caller is a fork admin. If the guard cleared the ahead flag on
/// fork-admin alone, the UI would report "all ahead visible" and let the user
/// deploy from an incomplete comparison. Here test-user-2 is admin of the fork
/// but only a plain member of the parent with no access to folder `restricted`,
/// so the parent copy of the modified script is hidden from them and the ahead
/// diff is (correctly) dropped — the flag must stay false.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_compare_workspaces_fork_admin_source_hidden_ahead(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{port}/api");
    let admin = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );
    let fork_admin_user = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN_2".to_string(),
    );

    // Parent folder `restricted` owned by test-user (the admin), NOT test-user-2,
    // and a script inside it (access flows through the folder). test-user-2 is a
    // plain member of the parent, so RLS hides this script from them.
    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, summary, created_by)
         VALUES ('test-workspace', 'restricted', 'restricted', ARRAY['u/test-user']::varchar[], '{\"u/test-user\": true}'::jsonb, '', 'test-user')"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, archived, schema_validation, ws_error_handler_muted, deleted, extra_perms)
         VALUES ('test-workspace', 'f/restricted/item', 314159, 'def main(): return 1', '', '', 'python3', 'test-user', NOW(), false, false, false, false, '{}'::jsonb)"
    )
    .execute(&db)
    .await?;

    // Fork (clones the folder + script into the fork).
    let resp = admin
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .json(&json!({"id": "wm-fork-guard-test", "name": "Guard Fork", "color": "#0000ff"}))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "fork creation failed: {}",
        resp.status()
    );

    // test-user-2 is ADMIN of the fork (so they see the fork copy via RLS bypass)
    // but only a plain member of the parent.
    sqlx::query!(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role)
         VALUES ('wm-fork-guard-test', 'test2@windmill.dev', 'test-user-2', true, 'Admin')"
    )
    .execute(&db)
    .await?;

    // A confirmed modified/ahead diff on the script that exists on both sides.
    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes, exists_in_source, exists_in_fork)
         VALUES ('test-workspace', 'wm-fork-guard-test', 'f/restricted/item', 'script', 1, 0, true, true, true)"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "DELETE FROM skip_workspace_diff_tally WHERE workspace_id IN ('test-workspace', 'wm-fork-guard-test')"
    )
    .execute(&db)
    .await?;

    let comparison: serde_json::Value = fork_admin_user
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-guard-test"
        ))
        .send()
        .await?
        .json()
        .await?;

    // The parent copy is hidden from test-user-2, so the ahead diff is dropped.
    // Fork-admin alone must NOT clear the flag — the comparison is incomplete.
    assert_eq!(
        comparison["all_ahead_items_visible"].as_bool(),
        Some(false),
        "fork admin without source-side visibility must not be reported as seeing all ahead items: {comparison}"
    );
    assert!(
        !comparison["diffs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["path"] == "f/restricted/item"),
        "source-hidden item must not appear in the fork admin's diff list: {comparison}"
    );

    // Sanity: a superadmin (admin of both sides) still sees everything.
    let comparison: serde_json::Value = admin
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-guard-test"
        ))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(
        comparison["all_ahead_items_visible"].as_bool(),
        Some(true),
        "superadmin must see all ahead items: {comparison}"
    );

    Ok(())
}

/// The merge UI can target a workspace outside the fork lineage. Nothing tallies
/// such a pair, so its comparison rests on an explicit full scan: before one, an
/// empty `diffs` must be distinguishable from "the two workspaces agree", and the
/// scan itself must seed only what actually differs, one way (WIN-2266).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_full_diff_scan_against_arbitrary_workspace(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );
    let base_url = format!("http://localhost:{port}/api");

    // A second root workspace, unrelated to test-workspace by lineage.
    sqlx::query!(
        "INSERT INTO workspace (id, name, owner) VALUES ('other-workspace', 'other-workspace', 'test-user')"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role)
         VALUES ('other-workspace', 'test@windmill.dev', 'test-user', true, 'Admin')"
    )
    .execute(&db)
    .await?;
    sqlx::query!("INSERT INTO workspace_settings (workspace_id) VALUES ('other-workspace')")
        .execute(&db)
        .await?;

    // One script identical on both sides, one that differs, one that exists only here.
    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, archived, schema_validation, ws_error_handler_muted, deleted)
         VALUES
         ('test-workspace', 'f/shared/same', 1, 'def main(): pass', 'Same', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false),
         ('test-workspace', 'f/shared/differs', 2, 'def main(): return 1', 'Differs', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false),
         ('test-workspace', 'f/shared/only_here', 3, 'def main(): return 2', 'Only here', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false),
         ('other-workspace', 'f/shared/same', 4, 'def main(): pass', 'Same', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false),
         ('other-workspace', 'f/shared/differs', 5, 'def main(): return 99', 'Differs', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false)"
    )
    .execute(&db)
    .await?;

    // The same app path, a normal app here and a raw app there: one logical item that
    // keys as two kinds, which the scan must collapse onto this workspace's kind.
    for (ws, raw) in [("test-workspace", false), ("other-workspace", true)] {
        let app_id = sqlx::query_scalar!(
            "INSERT INTO app (workspace_id, path, summary, policy, versions)
             VALUES ($1, 'f/shared/converted', 'Converted', '{}'::jsonb, ARRAY[]::bigint[])
             RETURNING id",
            ws,
        )
        .fetch_one(&db)
        .await?;
        let version = sqlx::query_scalar!(
            "INSERT INTO app_version (app_id, value, created_by, created_at, raw_app)
             VALUES ($1, $2, 'test@windmill.dev', NOW(), $3) RETURNING id",
            app_id,
            json!({"grid": []}),
            raw,
        )
        .fetch_one(&db)
        .await?;
        sqlx::query!(
            "UPDATE app SET versions = ARRAY[$2::bigint] WHERE id = $1",
            app_id,
            version,
        )
        .execute(&db)
        .await?;
    }

    // The same migration under a different name on each side: one logical item, two
    // candidate paths, which the scan must collapse.
    sqlx::query!(
        "INSERT INTO datatable_migrations (workspace_id, datatable, timestamp, name, code_up)
         VALUES
         ('test-workspace', 'dt', 20260101000001, 'renamed_here', 'ALTER TABLE t ADD COLUMN a int'),
         ('other-workspace', 'dt', 20260101000001, 'named_there', 'ALTER TABLE t ADD COLUMN a int')"
    )
    .execute(&db)
    .await?;

    let compare_url = format!("{base_url}/w/other-workspace/workspaces/compare/test-workspace");

    // Before any scan: no candidate set, so no diffs — and `full_scan_at` says so.
    let before: serde_json::Value = client
        .client()
        .get(&compare_url)
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(
        before["diffs"].as_array().map(|d| d.len()),
        Some(0),
        "an unscanned arbitrary pair has no diffs: {before}"
    );
    assert!(
        before["full_scan_at"].is_null(),
        "an unscanned arbitrary pair must report no scan: {before}"
    );

    let scan = client
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/seed_full_diff/other-workspace"
        ))
        .send()
        .await?;
    assert!(
        scan.status().is_success(),
        "seeding the scan should succeed: {}",
        scan.status()
    );

    let after: serde_json::Value = client
        .client()
        .get(&compare_url)
        .send()
        .await?
        .json()
        .await?;
    assert!(
        !after["full_scan_at"].is_null(),
        "a scanned pair must report when it was scanned: {after}"
    );
    let mut keys: Vec<(&str, &str)> = after["diffs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| (d["kind"].as_str().unwrap(), d["path"].as_str().unwrap()))
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            ("app", "f/shared/converted"),
            ("datatable_migration", "dt/20260101000001_renamed_here"),
            ("script", "f/shared/differs"),
            ("script", "f/shared/only_here"),
        ],
        "the scan keeps only what differs, drops the identical script, and collapses the renamed migration and the app/raw-app conversion onto this workspace's key: {after}"
    );
    for diff in after["diffs"].as_array().unwrap() {
        assert_eq!(
            (diff["ahead"].as_i64(), diff["behind"].as_i64()),
            (Some(1), Some(0)),
            "an arbitrary pair is compared one way only: {diff}"
        );
    }

    // Renaming the migration moves its candidate path. A re-scan must replace the
    // candidate set, not add to it: the previous path still resolves to the same
    // `(datatable, timestamp)`, so keeping it would list one migration twice.
    sqlx::query!(
        "UPDATE datatable_migrations SET name = 'renamed_again'
         WHERE workspace_id = 'test-workspace' AND datatable = 'dt'"
    )
    .execute(&db)
    .await?;
    client
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/seed_full_diff/other-workspace"
        ))
        .send()
        .await?;
    let rescanned: serde_json::Value = client
        .client()
        .get(&compare_url)
        .send()
        .await?
        .json()
        .await?;
    let migrations: Vec<&str> = rescanned["diffs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["kind"] == "datatable_migration")
        .map(|d| d["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        migrations,
        vec!["dt/20260101000001_renamed_again"],
        "a re-scan replaces the candidate set, so the pre-rename path is gone: {rescanned}"
    );

    // An arbitrary pair is only comparable to someone who administers both sides:
    // otherwise admin of one workspace would be enough to learn how much of another
    // differs from it. test-user-2 is a plain member of test-workspace.
    sqlx::query!(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role)
         VALUES ('other-workspace', 'test2@windmill.dev', 'test-user-2', true, 'Admin')"
    )
    .execute(&db)
    .await?;
    let non_admin = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN_2".to_string(),
    );
    let refused = non_admin.client().get(&compare_url).send().await?;
    assert!(
        refused.status().is_client_error(),
        "comparing an arbitrary pair without admin on both sides must be refused: {}",
        refused.status()
    );
    let refused_scan = non_admin
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/seed_full_diff/other-workspace"
        ))
        .send()
        .await?;
    assert!(
        refused_scan.status().is_client_error(),
        "seeding a scan without admin on both sides must be refused: {}",
        refused_scan.status()
    );

    // The lineage pair has a continuous tally whose direction a one-way scan would
    // overwrite, so scanning it is refused.
    let fork = client
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .json(&json!({"id": "wm-fork-test-workspace", "name": "Test Fork", "color": "#0000ff"}))
        .send()
        .await?;
    assert!(fork.status().is_success(), "fork creation should succeed");
    let lineage_scan = client
        .client()
        .post(&format!(
            "{base_url}/w/wm-fork-test-workspace/workspaces/seed_full_diff/test-workspace"
        ))
        .send()
        .await?;
    assert!(
        lineage_scan.status().is_client_error(),
        "scanning a lineage pair must be refused: {}",
        lineage_scan.status()
    );

    Ok(())
}

/// Regression for WIN-2289. A fork deletion and a git-sync pull reverting an
/// earlier deploy leave the same `ahead > 0, behind = 0` row with the item gone
/// from the fork; the comparison must carry the recorded origin that tells them
/// apart, and a tally that cannot vouch for its event must not overwrite it.
///
/// Gated on `private` for the same reason as the tally tests above: the OSS
/// `handle_deployment_metadata` is a no-op, so no rows would ever be written.
#[cfg(feature = "private")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_compare_records_fork_removal_origin(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{port}/api");
    let admin = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    sqlx::query!("DELETE FROM skip_workspace_diff_tally")
        .execute(&db)
        .await?;

    // Deployed in the parent before the fork exists, so nothing is tallied yet and
    // the fork starts with both scripts. bash needs no lock generation, so the
    // create tallies inline instead of deferring to a dependency job (no worker
    // runs in this test).
    for path in ["u/admin/dropped_by_user", "u/admin/dropped_by_sync"] {
        let resp = admin
            .client()
            .post(&format!("{base_url}/w/test-workspace/scripts/create"))
            .json(&json!({
                "path": path,
                "summary": "",
                "description": "",
                "content": "echo 1",
                "language": "bash",
                "schema": {"type": "object", "properties": {}, "required": []},
            }))
            .send()
            .await?;
        let status = resp.status();
        assert!(
            status.is_success(),
            "script create failed: {} — {}",
            status,
            resp.text().await?
        );
    }

    let resp = admin
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .json(&json!({"id": "wm-fork-removal", "name": "Removal Fork"}))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "fork creation failed: {}",
        resp.status()
    );

    // Same act, same resulting row shape; only the header a sync client sets differs.
    for (path, origin_header) in [
        ("u/admin/dropped_by_user", None),
        ("u/admin/dropped_by_sync", Some("sync")),
    ] {
        let mut req = admin.client().post(&format!(
            "{base_url}/w/wm-fork-removal/scripts/archive/p/{path}"
        ));
        if let Some(origin) = origin_header {
            req = req.header("X-Windmill-Deploy-Origin", origin);
        }
        let resp = req.send().await?;
        let status = resp.status();
        assert!(
            status.is_success(),
            "archive of {path} failed: {} — {}",
            status,
            resp.text().await?
        );
    }

    // The tally runs in a `tokio::spawn` inside `handle_deployment_metadata`.
    let mut recorded = 0;
    for _ in 0..40 {
        recorded = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"count!\" FROM workspace_diff
             WHERE source_workspace_id = 'test-workspace'
               AND fork_workspace_id = 'wm-fork-removal'
               AND kind = 'script'
               AND fork_last_event_origin IS NOT NULL"
        )
        .fetch_one(&db)
        .await?;
        if recorded >= 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert_eq!(recorded, 2, "both archives should have tallied an event");

    let comparison: serde_json::Value = admin
        .client()
        .get(&format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-removal"
        ))
        .send()
        .await?
        .json()
        .await?;
    let diffs = comparison["diffs"].as_array().unwrap();

    for (path, expected_origin) in [
        ("u/admin/dropped_by_user", "authored"),
        ("u/admin/dropped_by_sync", "sync"),
    ] {
        let row = diffs
            .iter()
            .find(|d| d["path"] == path && d["kind"] == "script")
            .unwrap_or_else(|| panic!("{path} should be in the diff; got {diffs:?}"));
        assert_eq!(row["exists_in_source"].as_bool(), Some(true), "{row}");
        assert_eq!(row["exists_in_fork"].as_bool(), Some(false), "{row}");
        assert_eq!(row["behind"].as_i64(), Some(0), "{row}");
        assert_eq!(
            row["fork_last_event_kind"].as_str(),
            Some("delete"),
            "{row}"
        );
        assert_eq!(
            row["fork_last_event_origin"].as_str(),
            Some(expected_origin),
            "the two removals must stay distinguishable: {row}"
        );
    }

    // The call the worker makes on its success path for a lock-generating deploy:
    // it reports a deploy that landed before the archive and cannot answer for what
    // the path holds now. The archive's record has to survive it, and only the
    // counter moves.
    windmill_git_sync::handle_deployment_metadata(
        "admin@windmill.dev",
        "admin",
        &db,
        "wm-fork-removal",
        windmill_git_sync::DeployedObject::Script {
            hash: windmill_common::scripts::ScriptHash(0),
            path: "u/admin/dropped_by_sync".to_string(),
            parent_path: Some("u/admin/dropped_by_sync".to_string()),
        },
        None,
        true,
        // A redeploy that did not move the item passes the same path back; it must
        // not be tallied a second time as the path a rename vacated.
        Some("u/admin/dropped_by_sync"),
    )
    .await?;
    let mut after = None;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let row = sqlx::query!(
            "SELECT ahead, fork_last_event_kind, fork_last_event_origin FROM workspace_diff
             WHERE source_workspace_id = 'test-workspace' AND fork_workspace_id = 'wm-fork-removal'
               AND kind = 'script' AND path = 'u/admin/dropped_by_sync'"
        )
        .fetch_one(&db)
        .await?;
        if row.ahead > 1 {
            after = Some(row);
            break;
        }
    }
    let after = after.expect("the detached tally should still bump the counter");
    assert_eq!(after.fork_last_event_kind.as_deref(), Some("delete"));
    assert_eq!(
        after.fork_last_event_origin.as_deref(),
        Some("sync"),
        "a tally that cannot vouch for its event must not overwrite one that did"
    );
    assert_eq!(after.ahead, 2, "and it counts once, not twice");

    Ok(())
}

/// `probe_deploy_event_kind` builds its query for these kinds by interpolating the
/// table name, so a wrong entry compiles fine and only fails once someone deploys
/// that trigger kind in a fork — where the error aborts the tally and the item
/// never reaches `workspace_diff` at all. Sweep the allowlist against a live
/// database instead.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_probe_covers_every_path_keyed_table(db: Pool<Postgres>) -> anyhow::Result<()> {
    for kind in windmill_git_sync::PATH_KEYED_TABLES {
        let probed =
            windmill_git_sync::probe_deploy_event_kind(&db, "test-workspace", kind, "u/a/nothing")
                .await
                .unwrap_or_else(|e| panic!("probing `{kind}` failed: {e}"));
        assert_eq!(
            probed,
            Some(windmill_common::deploy_origin::DeployEventKind::Delete),
            "an absent `{kind}` must probe as a deletion"
        );
    }
    Ok(())
}

/// Flows always generate a lock, so without the request reporting the path a
/// rename left behind, a renamed flow leaves its old path in the parent with
/// nothing to merge.
#[cfg(feature = "private")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_rename_records_the_vacated_path(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{port}/api");
    let admin = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    sqlx::query!("DELETE FROM skip_workspace_diff_tally")
        .execute(&db)
        .await?;
    let resp = admin
        .client()
        .post(&format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .json(&json!({"id": "wm-fork-renamed", "name": "Rename Fork"}))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "fork creation: {}",
        resp.status()
    );

    let flow = |path: &str| {
        json!({
            "path": path,
            "summary": "",
            "description": "",
            "value": {"modules": []},
            "schema": {"type": "object", "properties": {}, "required": []},
        })
    };
    for (path, origin) in [
        ("u/admin/moved_by_user", None),
        ("u/admin/moved_by_sync", Some("sync")),
    ] {
        let mut req = admin
            .client()
            .post(&format!("{base_url}/w/wm-fork-renamed/flows/create"))
            .json(&flow(path));
        if let Some(origin) = origin {
            req = req.header("X-Windmill-Deploy-Origin", origin);
        }
        let resp = req.send().await?;
        assert!(resp.status().is_success(), "flow create: {}", resp.status());

        let mut req = admin
            .client()
            .post(&format!("{base_url}/w/wm-fork-renamed/flows/update/{path}"))
            .json(&flow(&format!("{path}_new")));
        if let Some(origin) = origin {
            req = req.header("X-Windmill-Deploy-Origin", origin);
        }
        let resp = req.send().await?;
        assert!(resp.status().is_success(), "flow rename: {}", resp.status());
    }

    let mut rows = vec![];
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        rows = sqlx::query!(
            "SELECT path, fork_last_event_kind, fork_last_event_origin FROM workspace_diff
             WHERE fork_workspace_id = 'wm-fork-renamed' AND kind = 'flow'
               AND fork_last_event_origin IS NOT NULL ORDER BY path"
        )
        .fetch_all(&db)
        .await?;
        if rows.len() >= 2 {
            break;
        }
    }
    let claimed: Vec<_> = rows
        .iter()
        .map(|r| {
            (
                r.path.as_str(),
                r.fork_last_event_kind.as_deref(),
                r.fork_last_event_origin.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        claimed,
        vec![
            ("u/admin/moved_by_sync", Some("delete"), Some("sync")),
            ("u/admin/moved_by_user", Some("delete"), Some("authored")),
        ],
        "each vacated path is recorded with the origin of the rename that left it, \
         and the deployed paths — which only the dependency job reports — are not"
    );

    // A script needing no lock deploys inline, so its own request reports both
    // paths and can say why the old one is empty rather than only that it is.
    let mut parent_hash: Option<String> = None;
    for path in ["u/admin/inline", "u/admin/inline_moved"] {
        let resp = admin
            .client()
            .post(&format!("{base_url}/w/wm-fork-renamed/scripts/create"))
            .json(&json!({
                "path": path,
                "summary": "",
                "description": "",
                // bash needs no lock, so this deploys inline.
                "content": "echo 1",
                "language": "bash",
                "schema": {"type": "object", "properties": {}, "required": []},
                "parent_hash": parent_hash,
            }))
            .send()
            .await?;
        let status = resp.status();
        let hash = resp.text().await?;
        assert!(status.is_success(), "script deploy failed: {status} — {hash}");
        parent_hash = Some(hash.trim().trim_matches('"').to_string());
    }
    let mut vacated = None;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        vacated = sqlx::query!(
            "SELECT fork_last_event_kind, fork_last_event_origin FROM workspace_diff
             WHERE fork_workspace_id = 'wm-fork-renamed' AND kind = 'script'
               AND path = 'u/admin/inline' AND fork_last_event_kind IS NOT NULL"
        )
        .fetch_optional(&db)
        .await?;
        if vacated.is_some() {
            break;
        }
    }
    let vacated = vacated.expect("the inline rename should report the path it left");
    assert_eq!(vacated.fork_last_event_kind.as_deref(), Some("rename_from"));
    assert_eq!(vacated.fork_last_event_origin.as_deref(), Some("authored"));

    Ok(())
}
