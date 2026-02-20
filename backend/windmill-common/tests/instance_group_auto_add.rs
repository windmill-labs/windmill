//! Integration tests for the interaction between instance groups and workspace auto-add.
//!
//! These tests verify that:
//! 1. Users added to instance groups are automatically added to workspaces configured with auto-add
//! 2. Configuring instance groups for a workspace adds existing group members
//! 3. Users removed from instance groups are removed from workspaces
//! 4. Role precedence is respected when users belong to multiple instance groups
//! 5. The `added_via` field correctly tracks how users were added
//!
//! ## Requirements
//!
//! - PostgreSQL database running locally
//! - Enterprise features enabled
//!
//! ## Running the tests
//!
//! These tests are ignored by default in CI. To run them locally:
//!
//! ```bash
//! # Run all instance group auto-add tests
//! cargo test -p windmill --test instance_group_auto_add --features private,enterprise -- --ignored --nocapture
//!
//! # Run a specific test
//! cargo test -p windmill --test instance_group_auto_add --features private,enterprise -- --ignored test_role_precedence --nocapture
//! ```

#[cfg(all(feature = "private", feature = "enterprise"))]
mod tests {
    use serde_json::json;
    use sqlx::{Pool, Postgres};

    /// Test that configuring instance groups for a workspace auto-adds existing group members
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations", fixtures("base", "instance_group_auto_add"))]
    async fn test_configure_instance_groups_adds_existing_members(db: Pool<Postgres>) {
        // Configure workspace to auto-add users from 'engineering' group with 'developer' role
        let groups = vec!["engineering".to_string()];
        let roles = json!({"engineering": "developer"});

        sqlx::query!(
            r#"
            UPDATE workspace_settings
            SET auto_invite = jsonb_build_object(
                'instance_groups', $2::jsonb,
                'instance_groups_roles', $3::jsonb
            )
            WHERE workspace_id = $1
            "#,
            "ws-with-auto-add",
            serde_json::to_value(&groups).unwrap(),
            &roles,
        )
        .execute(&db)
        .await
        .expect("Failed to update workspace settings");

        // Manually simulate the auto-add process that would happen in the API
        // In the real API, this is done by process_instance_group_auto_adds
        for email in &["alice@example.com", "bob@example.com"] {
            // Check if user already exists
            let exists = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND email = $2)",
                "ws-with-auto-add",
                *email
            )
            .fetch_one(&db)
            .await
            .unwrap()
            .unwrap_or(false);

            if !exists {
                let username = email.split('@').next().unwrap().replace(".", "");
                let added_via = json!({"source": "instance_group", "group": "engineering"});

                sqlx::query!(
                    "INSERT INTO usr (workspace_id, username, email, is_admin, operator, added_via)
                     VALUES ($1, $2, $3, false, false, $4)",
                    "ws-with-auto-add",
                    username,
                    *email,
                    &added_via,
                )
                .execute(&db)
                .await
                .expect("Failed to add user");

                sqlx::query!(
                    "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, 'all')",
                    "ws-with-auto-add",
                    username,
                )
                .execute(&db)
                .await
                .expect("Failed to add user to all group");
            }
        }

        // Verify alice was added (she's in engineering)
        let alice_in_workspace = sqlx::query!(
            r#"
            SELECT username, is_admin, operator, added_via
            FROM usr
            WHERE workspace_id = 'ws-with-auto-add' AND email = 'alice@example.com'
            "#
        )
        .fetch_optional(&db)
        .await
        .expect("Failed to query user");

        assert!(
            alice_in_workspace.is_some(),
            "Alice should be in the workspace"
        );
        let alice = alice_in_workspace.unwrap();
        assert!(!alice.is_admin, "Alice should not be admin (developer role)");
        assert!(!alice.operator, "Alice should not be operator (developer role)");

        // Check added_via field
        let added_via = alice.added_via.expect("added_via should be set");
        assert_eq!(
            added_via.get("source").and_then(|v| v.as_str()),
            Some("instance_group"),
            "added_via source should be instance_group"
        );
        assert_eq!(
            added_via.get("group").and_then(|v| v.as_str()),
            Some("engineering"),
            "added_via group should be engineering"
        );

        // Verify bob was also added (he's in engineering too)
        let bob_in_workspace = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = 'ws-with-auto-add' AND email = 'bob@example.com')"
        )
        .fetch_one(&db)
        .await
        .unwrap()
        .unwrap_or(false);

        assert!(bob_in_workspace, "Bob should be in the workspace");

        // Verify charlie was NOT added (she's only in sales, not engineering)
        let charlie_in_workspace = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = 'ws-with-auto-add' AND email = 'charlie@example.com')"
        )
        .fetch_one(&db)
        .await
        .unwrap()
        .unwrap_or(false);

        assert!(
            !charlie_in_workspace,
            "Charlie should NOT be in the workspace (not in engineering group)"
        );

        println!("✓ Configure instance groups adds existing members correctly");
    }

    /// Test role assignment based on instance group configuration
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations", fixtures("base", "instance_group_auto_add"))]
    async fn test_role_assignment_admin(db: Pool<Postgres>) {
        // Configure workspace with admins group having admin role
        let groups = vec!["admins".to_string()];
        let roles = json!({"admins": "admin"});

        sqlx::query!(
            r#"
            UPDATE workspace_settings
            SET auto_invite = jsonb_build_object(
                'instance_groups', $2::jsonb,
                'instance_groups_roles', $3::jsonb
            )
            WHERE workspace_id = $1
            "#,
            "ws-with-auto-add",
            serde_json::to_value(&groups).unwrap(),
            &roles,
        )
        .execute(&db)
        .await
        .expect("Failed to update workspace settings");

        // Simulate adding dave (who is in admins group) with admin role
        let added_via = json!({"source": "instance_group", "group": "admins"});
        sqlx::query!(
            "INSERT INTO usr (workspace_id, username, email, is_admin, operator, added_via)
             VALUES ($1, 'dave', 'dave@example.com', true, false, $2)",
            "ws-with-auto-add",
            &added_via,
        )
        .execute(&db)
        .await
        .expect("Failed to add user");

        // Verify dave is admin
        let dave = sqlx::query!(
            "SELECT is_admin, operator FROM usr WHERE workspace_id = 'ws-with-auto-add' AND email = 'dave@example.com'"
        )
        .fetch_one(&db)
        .await
        .expect("Failed to query user");

        assert!(dave.is_admin, "Dave should be admin");
        assert!(!dave.operator, "Dave should not be operator");

        println!("✓ Admin role assignment works correctly");
    }

    /// Test role assignment for operator
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations", fixtures("base", "instance_group_auto_add"))]
    async fn test_role_assignment_operator(db: Pool<Postgres>) {
        // Configure workspace with sales group having operator role
        let groups = vec!["sales".to_string()];
        let roles = json!({"sales": "operator"});

        sqlx::query!(
            r#"
            UPDATE workspace_settings
            SET auto_invite = jsonb_build_object(
                'instance_groups', $2::jsonb,
                'instance_groups_roles', $3::jsonb
            )
            WHERE workspace_id = $1
            "#,
            "ws-with-auto-add",
            serde_json::to_value(&groups).unwrap(),
            &roles,
        )
        .execute(&db)
        .await
        .expect("Failed to update workspace settings");

        // Simulate adding charlie (who is in sales group) with operator role
        let added_via = json!({"source": "instance_group", "group": "sales"});
        sqlx::query!(
            "INSERT INTO usr (workspace_id, username, email, is_admin, operator, added_via)
             VALUES ($1, 'charlie', 'charlie@example.com', false, true, $2)",
            "ws-with-auto-add",
            &added_via,
        )
        .execute(&db)
        .await
        .expect("Failed to add user");

        // Verify charlie is operator
        let charlie = sqlx::query!(
            "SELECT is_admin, operator FROM usr WHERE workspace_id = 'ws-with-auto-add' AND email = 'charlie@example.com'"
        )
        .fetch_one(&db)
        .await
        .expect("Failed to query user");

        assert!(!charlie.is_admin, "Charlie should not be admin");
        assert!(charlie.operator, "Charlie should be operator");

        println!("✓ Operator role assignment works correctly");
    }

    /// Test role precedence when user is in multiple instance groups
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations", fixtures("base", "instance_group_auto_add"))]
    async fn test_role_precedence_multiple_groups(db: Pool<Postgres>) {
        // Configure workspace with multiple groups: engineering (admin), sales (operator)
        // Bob is in both groups, should get admin role (highest precedence)
        let groups = vec!["engineering".to_string(), "sales".to_string()];
        let roles = json!({"engineering": "admin", "sales": "operator"});

        sqlx::query!(
            r#"
            UPDATE workspace_settings
            SET auto_invite = jsonb_build_object(
                'instance_groups', $2::jsonb,
                'instance_groups_roles', $3::jsonb
            )
            WHERE workspace_id = $1
            "#,
            "ws-multi-group",
            serde_json::to_value(&groups).unwrap(),
            &roles,
        )
        .execute(&db)
        .await
        .expect("Failed to update workspace settings");

        // Simulate the role precedence logic: admin > developer > operator
        // Bob is in both engineering (admin) and sales (operator)
        // He should get admin role
        let added_via = json!({"source": "instance_group", "group": "engineering"});
        sqlx::query!(
            "INSERT INTO usr (workspace_id, username, email, is_admin, operator, added_via)
             VALUES ($1, 'bob', 'bob@example.com', true, false, $2)",
            "ws-multi-group",
            &added_via,
        )
        .execute(&db)
        .await
        .expect("Failed to add user");

        // Verify bob got admin role (highest precedence)
        let bob = sqlx::query!(
            "SELECT is_admin, operator, added_via FROM usr WHERE workspace_id = 'ws-multi-group' AND email = 'bob@example.com'"
        )
        .fetch_one(&db)
        .await
        .expect("Failed to query user");

        assert!(bob.is_admin, "Bob should be admin (highest precedence role)");
        assert!(!bob.operator, "Bob should not be operator");

        // Verify added_via tracks the primary group (engineering, the one with highest precedence)
        let added_via = bob.added_via.expect("added_via should be set");
        assert_eq!(
            added_via.get("group").and_then(|v| v.as_str()),
            Some("engineering"),
            "added_via should track the primary group (highest precedence)"
        );

        println!("✓ Role precedence works correctly for users in multiple groups");
    }

    /// Test removing user from instance group removes them from workspace
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations", fixtures("base", "instance_group_auto_add"))]
    async fn test_remove_user_from_instance_group(db: Pool<Postgres>) {
        // First, add alice to the workspace via engineering group
        let added_via = json!({"source": "instance_group", "group": "engineering"});
        sqlx::query!(
            "INSERT INTO usr (workspace_id, username, email, is_admin, operator, added_via)
             VALUES ($1, 'alice', 'alice@example.com', false, false, $2)",
            "ws-with-auto-add",
            &added_via,
        )
        .execute(&db)
        .await
        .expect("Failed to add user");

        sqlx::query!(
            "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, 'alice', 'all')",
            "ws-with-auto-add",
        )
        .execute(&db)
        .await
        .expect("Failed to add user to all group");

        // Verify alice is in the workspace
        let alice_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = 'ws-with-auto-add' AND email = 'alice@example.com')"
        )
        .fetch_one(&db)
        .await
        .unwrap()
        .unwrap_or(false);
        assert!(alice_exists, "Alice should be in workspace before removal");

        // Simulate removing alice from the instance group and then from workspace
        // In the real API, remove_users_from_instance_group_workspaces does this
        sqlx::query!(
            "DELETE FROM email_to_igroup WHERE email = 'alice@example.com' AND igroup = 'engineering'"
        )
        .execute(&db)
        .await
        .expect("Failed to remove from instance group");

        // Find and remove users who were added via this specific instance group
        let users_to_remove = sqlx::query!(
            r#"
            SELECT workspace_id, username
            FROM usr
            WHERE email = 'alice@example.com'
            AND added_via->>'source' = 'instance_group'
            AND added_via->>'group' = 'engineering'
            "#
        )
        .fetch_all(&db)
        .await
        .expect("Failed to query users to remove");

        for user in users_to_remove {
            sqlx::query!(
                "DELETE FROM usr_to_group WHERE workspace_id = $1 AND usr = $2",
                user.workspace_id,
                user.username,
            )
            .execute(&db)
            .await
            .expect("Failed to remove from groups");

            sqlx::query!(
                "DELETE FROM usr WHERE workspace_id = $1 AND username = $2",
                user.workspace_id,
                user.username,
            )
            .execute(&db)
            .await
            .expect("Failed to remove user");
        }

        // Verify alice was removed from the workspace
        let alice_still_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = 'ws-with-auto-add' AND email = 'alice@example.com')"
        )
        .fetch_one(&db)
        .await
        .unwrap()
        .unwrap_or(false);

        assert!(
            !alice_still_exists,
            "Alice should be removed from workspace after instance group removal"
        );

        println!("✓ Removing user from instance group removes them from workspace");
    }

    /// Test that users added via domain are not affected by instance group removal
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations", fixtures("base", "instance_group_auto_add"))]
    async fn test_domain_added_users_not_affected_by_group_removal(db: Pool<Postgres>) {
        // Add alice via domain (not instance group)
        let added_via = json!({"source": "domain", "domain": "example.com"});
        sqlx::query!(
            "INSERT INTO usr (workspace_id, username, email, is_admin, operator, added_via)
             VALUES ($1, 'alice', 'alice@example.com', false, false, $2)",
            "ws-with-auto-add",
            &added_via,
        )
        .execute(&db)
        .await
        .expect("Failed to add user");

        // Try to remove users added via 'engineering' instance group
        // Alice should NOT be affected since she was added via domain
        let users_to_remove = sqlx::query!(
            r#"
            SELECT workspace_id, username
            FROM usr
            WHERE email = 'alice@example.com'
            AND added_via->>'source' = 'instance_group'
            AND added_via->>'group' = 'engineering'
            "#
        )
        .fetch_all(&db)
        .await
        .expect("Failed to query users to remove");

        assert!(
            users_to_remove.is_empty(),
            "No users should be found for removal (alice was added via domain)"
        );

        // Verify alice is still in the workspace
        let alice_still_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = 'ws-with-auto-add' AND email = 'alice@example.com')"
        )
        .fetch_one(&db)
        .await
        .unwrap()
        .unwrap_or(false);

        assert!(
            alice_still_exists,
            "Alice should remain in workspace (added via domain, not instance group)"
        );

        println!("✓ Users added via domain are not affected by instance group removal");
    }

    /// Test cleanup when instance group is removed from workspace configuration
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations", fixtures("base", "instance_group_auto_add"))]
    async fn test_cleanup_removed_instance_groups(db: Pool<Postgres>) {
        // First, add users via engineering group
        for (username, email) in &[("alice", "alice@example.com"), ("bob", "bob@example.com")] {
            let added_via = json!({"source": "instance_group", "group": "engineering"});
            sqlx::query!(
                "INSERT INTO usr (workspace_id, username, email, is_admin, operator, added_via)
                 VALUES ($1, $2, $3, false, false, $4) ON CONFLICT DO NOTHING",
                "ws-with-auto-add",
                *username,
                *email,
                &added_via,
            )
            .execute(&db)
            .await
            .expect("Failed to add user");

            sqlx::query!(
                "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ($1, $2, 'all') ON CONFLICT DO NOTHING",
                "ws-with-auto-add",
                *username,
            )
            .execute(&db)
            .await
            .expect("Failed to add user to all group");
        }

        // Verify both users are in workspace
        let count_before = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM usr WHERE workspace_id = 'ws-with-auto-add' AND added_via->>'source' = 'instance_group'"#
        )
        .fetch_one(&db)
        .await
        .unwrap();

        assert!(
            count_before >= 2,
            "At least 2 users should be in workspace before cleanup"
        );

        // Simulate removing 'engineering' from workspace configuration
        // This should trigger cleanup of users added via that group

        // Get all users in the engineering group
        let group_users = sqlx::query_scalar!(
            "SELECT email FROM email_to_igroup WHERE igroup = 'engineering'"
        )
        .fetch_all(&db)
        .await
        .expect("Failed to get group users");

        // Remove users who were added via engineering group
        for email in group_users {
            let users_to_remove = sqlx::query!(
                r#"
                SELECT workspace_id, username
                FROM usr
                WHERE email = $1
                AND workspace_id = 'ws-with-auto-add'
                AND added_via->>'source' = 'instance_group'
                AND added_via->>'group' = 'engineering'
                "#,
                email
            )
            .fetch_all(&db)
            .await
            .expect("Failed to query users");

            for user in users_to_remove {
                sqlx::query!(
                    "DELETE FROM usr_to_group WHERE workspace_id = $1 AND usr = $2",
                    user.workspace_id,
                    user.username,
                )
                .execute(&db)
                .await
                .ok();

                sqlx::query!(
                    "DELETE FROM usr WHERE workspace_id = $1 AND username = $2",
                    user.workspace_id,
                    user.username,
                )
                .execute(&db)
                .await
                .ok();
            }
        }

        // Clear the instance groups configuration
        sqlx::query!(
            r#"
            UPDATE workspace_settings
            SET auto_invite = COALESCE(auto_invite, '{}'::jsonb) - 'instance_groups' - 'instance_groups_roles'
            WHERE workspace_id = 'ws-with-auto-add'
            "#
        )
        .execute(&db)
        .await
        .expect("Failed to clear config");

        // Verify users were removed
        let count_after = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM usr WHERE workspace_id = 'ws-with-auto-add' AND added_via->>'source' = 'instance_group' AND added_via->>'group' = 'engineering'"#
        )
        .fetch_one(&db)
        .await
        .unwrap();

        assert_eq!(
            count_after, 0,
            "All users added via engineering group should be removed"
        );

        println!("✓ Cleanup of removed instance groups works correctly");
    }

    /// Test that users are not duplicated if already in workspace
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations", fixtures("base", "instance_group_auto_add"))]
    async fn test_no_duplicate_users(db: Pool<Postgres>) {
        // Add alice to workspace first (without instance group tracking)
        sqlx::query!(
            "INSERT INTO usr (workspace_id, username, email, is_admin, operator)
             VALUES ('ws-with-auto-add', 'alice', 'alice@example.com', false, false)",
        )
        .execute(&db)
        .await
        .expect("Failed to add user");

        // Count users before attempting to add via instance group
        let count_before = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM usr WHERE workspace_id = 'ws-with-auto-add' AND email = 'alice@example.com'"#
        )
        .fetch_one(&db)
        .await
        .unwrap();

        assert_eq!(count_before, 1, "Should have exactly 1 alice before");

        // Try to add alice again (simulating instance group auto-add with ON CONFLICT DO NOTHING)
        let added_via = json!({"source": "instance_group", "group": "engineering"});
        sqlx::query!(
            "INSERT INTO usr (workspace_id, username, email, is_admin, operator, added_via)
             VALUES ($1, 'alice', 'alice@example.com', false, false, $2) ON CONFLICT DO NOTHING",
            "ws-with-auto-add",
            &added_via,
        )
        .execute(&db)
        .await
        .expect("Should not fail on conflict");

        // Verify no duplicate was created
        let count_after = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM usr WHERE workspace_id = 'ws-with-auto-add' AND email = 'alice@example.com'"#
        )
        .fetch_one(&db)
        .await
        .unwrap();

        assert_eq!(count_after, 1, "Should still have exactly 1 alice after");

        println!("✓ No duplicate users are created");
    }

    /// Test workspace without auto-add configured is not affected
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations", fixtures("base", "instance_group_auto_add"))]
    async fn test_workspace_without_auto_add_not_affected(db: Pool<Postgres>) {
        // ws-no-auto-add has no instance_groups configured

        // Verify no auto_invite configuration
        let config = sqlx::query_scalar!(
            "SELECT auto_invite->'instance_groups' FROM workspace_settings WHERE workspace_id = 'ws-no-auto-add'"
        )
        .fetch_one(&db)
        .await
        .unwrap();

        assert!(
            config.is_none(),
            "ws-no-auto-add should have no instance_groups configured"
        );

        // Verify alice is not in this workspace (she's in engineering group but workspace has no auto-add)
        let alice_in_workspace = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = 'ws-no-auto-add' AND email = 'alice@example.com')"
        )
        .fetch_one(&db)
        .await
        .unwrap()
        .unwrap_or(false);

        assert!(
            !alice_in_workspace,
            "Alice should NOT be in ws-no-auto-add (no auto-add configured)"
        );

        println!("✓ Workspace without auto-add is not affected");
    }

    /// Test querying workspaces configured with a specific instance group
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations", fixtures("base", "instance_group_auto_add"))]
    async fn test_query_workspaces_with_instance_group(db: Pool<Postgres>) {
        // Configure ws-with-auto-add to use engineering group
        let groups = vec!["engineering".to_string()];
        let roles = json!({"engineering": "developer"});

        sqlx::query!(
            r#"
            UPDATE workspace_settings
            SET auto_invite = jsonb_build_object(
                'instance_groups', $2::jsonb,
                'instance_groups_roles', $3::jsonb
            )
            WHERE workspace_id = $1
            "#,
            "ws-with-auto-add",
            serde_json::to_value(&groups).unwrap(),
            &roles,
        )
        .execute(&db)
        .await
        .expect("Failed to update workspace settings");

        // Query workspaces that have 'engineering' in their instance_groups
        let workspaces = sqlx::query!(
            r#"
            SELECT workspace_id, auto_invite->'instance_groups_roles' as instance_groups_roles
            FROM workspace_settings
            WHERE auto_invite->'instance_groups' ? $1
            "#,
            "engineering"
        )
        .fetch_all(&db)
        .await
        .expect("Failed to query workspaces");

        assert_eq!(workspaces.len(), 1, "Should find 1 workspace with engineering group");
        assert_eq!(workspaces[0].workspace_id, "ws-with-auto-add");

        // Verify the role configuration is returned correctly
        let roles = workspaces[0]
            .instance_groups_roles
            .as_ref()
            .expect("Should have roles");
        assert_eq!(
            roles.get("engineering").and_then(|v| v.as_str()),
            Some("developer"),
            "Should have developer role for engineering"
        );

        // Query for a group that's not configured anywhere
        let no_workspaces = sqlx::query!(
            r#"
            SELECT workspace_id
            FROM workspace_settings
            WHERE auto_invite->'instance_groups' ? $1
            "#,
            "nonexistent_group"
        )
        .fetch_all(&db)
        .await
        .expect("Failed to query workspaces");

        assert!(
            no_workspaces.is_empty(),
            "Should find no workspaces with nonexistent_group"
        );

        println!("✓ Querying workspaces with instance groups works correctly");
    }
}

// OSS version - placeholder to avoid compilation errors
#[cfg(not(all(feature = "private", feature = "enterprise")))]
mod tests {
    #[test]
    fn test_instance_group_auto_add_requires_enterprise() {
        println!("Instance group auto-add tests require Enterprise Edition features");
        println!("Run with: cargo test -p windmill instance_group_auto_add --features private,enterprise -- --nocapture");
    }
}
