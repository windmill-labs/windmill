//! Integration tests for HashiCorp Vault secret backend.
//!
//! These tests require:
//! 1. A PostgreSQL database (handled by sqlx test framework)
//! 2. A running HashiCorp Vault instance
//! 3. The RUN_VAULT_TESTS=1 environment variable to be set
//!
//! Environment variables:
//! - RUN_VAULT_TESTS=1  - Required to run the tests
//! - VAULT_ADDR         - Vault server address (default: http://127.0.0.1:8200)
//! - VAULT_TOKEN        - Static token for static token tests (default: test-root-token)
//! - BASE_URL           - Windmill instance URL for JWT tests (default: http://localhost:8000)
//!
//! Run tests (static token mode):
//! ```bash
//! RUN_VAULT_TESTS=1 VAULT_TOKEN=your-token cargo test -p windmill \
//!     secret_backend_integration --features private,enterprise -- --nocapture
//! ```
//!
//! Run tests (JWT mode - requires Windmill instance running for JWKS endpoint):
//! ```bash
//! RUN_VAULT_TESTS=1 BASE_URL=http://localhost:8000 cargo test -p windmill \
//!     secret_backend_integration --features private,enterprise,openidconnect -- --nocapture
//! ```

#[cfg(all(feature = "private", feature = "enterprise"))]
mod tests {
    use sqlx::{Pool, Postgres};
    use std::collections::HashMap;
    use windmill_common::secret_backend::{
        migrate_secrets_to_database, migrate_secrets_to_vault, test_vault_connection,
        SecretBackend, VaultBackend, VaultSettings,
    };

    /// Check if vault tests should run (requires RUN_VAULT_TESTS=1 env var)
    fn should_run_vault_tests() -> bool {
        std::env::var("RUN_VAULT_TESTS")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
    }

    /// Set up BASE_URL for JWT tests (required for OIDC issuer URL generation)
    async fn setup_base_url() {
        let base_url = std::env::var("BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());
        let mut url = windmill_common::BASE_URL.write().await;
        *url = base_url;
    }

    /// Skip test if RUN_VAULT_TESTS is not set
    macro_rules! skip_if_no_vault {
        () => {
            if !should_run_vault_tests() {
                println!("Skipping test: RUN_VAULT_TESTS=1 not set");
                println!("To run vault tests: RUN_VAULT_TESTS=1 cargo test ...");
                return;
            }
        };
    }

    fn vault_settings_static_token() -> VaultSettings {
        VaultSettings {
            address: std::env::var("VAULT_ADDR")
                .unwrap_or_else(|_| "http://127.0.0.1:8200".to_string()),
            mount_path: "windmill".to_string(),
            jwt_role: None, // Static token mode
            namespace: None,
            token: Some(
                std::env::var("VAULT_TOKEN").unwrap_or_else(|_| "test-root-token".to_string()),
            ),
        }
    }

    fn vault_settings_jwt() -> VaultSettings {
        VaultSettings {
            address: std::env::var("VAULT_ADDR")
                .unwrap_or_else(|_| "http://127.0.0.1:8200".to_string()),
            mount_path: "windmill".to_string(),
            jwt_role: Some("windmill-secrets".to_string()), // JWT mode
            namespace: None,
            token: None, // No static token - use JWT
        }
    }

    // ==================== Static Token Tests ====================

    /// Test Vault connection with static token
    #[sqlx::test(fixtures("base", "secret_backend"))]
    async fn test_vault_connection_static_token(db: Pool<Postgres>) {
        skip_if_no_vault!();

        let settings = vault_settings_static_token();
        println!("Testing Vault connection with static token...");
        println!("  Address: {}", settings.address);

        let result = test_vault_connection(&settings, Some(&db)).await;
        assert!(
            result.is_ok(),
            "Failed to connect to Vault: {:?}",
            result.err()
        );
        println!("✓ Successfully connected to Vault with static token");
    }

    /// Test basic CRUD operations with static token
    #[sqlx::test(fixtures("base", "secret_backend"))]
    async fn test_vault_crud_static_token(_db: Pool<Postgres>) {
        skip_if_no_vault!();

        let settings = vault_settings_static_token();
        let backend = VaultBackend::new(settings);

        let workspace_id = "test-crud-static";
        let path = "test-secret";
        let value = "my-super-secret-value-123";

        println!("Testing CRUD with static token...");

        // Create
        println!("  Creating secret...");
        backend
            .set_secret(workspace_id, path, value)
            .await
            .expect("Failed to create secret");
        println!("  ✓ Created");

        // Read
        println!("  Reading secret...");
        let read_value = backend
            .get_secret(workspace_id, path)
            .await
            .expect("Failed to read secret");
        assert_eq!(read_value, value);
        println!("  ✓ Read (value matches)");

        // Update
        println!("  Updating secret...");
        let new_value = "updated-secret-value-456";
        backend
            .set_secret(workspace_id, path, new_value)
            .await
            .expect("Failed to update secret");
        let updated = backend
            .get_secret(workspace_id, path)
            .await
            .expect("Failed to read updated secret");
        assert_eq!(updated, new_value);
        println!("  ✓ Updated");

        // Delete
        println!("  Deleting secret...");
        backend
            .delete_secret(workspace_id, path)
            .await
            .expect("Failed to delete secret");
        let result = backend.get_secret(workspace_id, path).await;
        assert!(result.is_err(), "Secret should be deleted");
        println!("  ✓ Deleted");

        println!("✓ CRUD operations successful with static token");
    }

    // ==================== JWT Auth Tests ====================

    /// Test Vault connection with JWT authentication
    #[sqlx::test(fixtures("base", "secret_backend"))]
    async fn test_vault_connection_jwt(db: Pool<Postgres>) {
        skip_if_no_vault!();
        setup_base_url().await;

        let settings = vault_settings_jwt();
        println!("Testing Vault connection with JWT auth...");
        println!("  Address: {}", settings.address);
        println!("  JWT Role: {:?}", settings.jwt_role);
        println!("  BASE_URL: {}", windmill_common::BASE_URL.read().await.clone());

        let result = test_vault_connection(&settings, Some(&db)).await;
        assert!(
            result.is_ok(),
            "Failed to connect to Vault with JWT: {:?}",
            result.err()
        );
        println!("✓ Successfully connected to Vault with JWT auth");
    }

    /// Test basic CRUD operations with JWT authentication
    #[cfg(feature = "openidconnect")]
    #[sqlx::test(fixtures("base", "secret_backend"))]
    async fn test_vault_crud_jwt(db: Pool<Postgres>) {
        skip_if_no_vault!();
        setup_base_url().await;

        let settings = vault_settings_jwt();
        let backend = VaultBackend::new_with_db(settings, db.clone());

        let workspace_id = "test-crud-jwt";
        let path = "jwt-test-secret";
        let value = "jwt-authenticated-secret-value";

        println!("Testing CRUD with JWT auth...");

        // Create
        println!("  Creating secret...");
        backend
            .set_secret(workspace_id, path, value)
            .await
            .expect("Failed to create secret with JWT");
        println!("  ✓ Created");

        // Read
        println!("  Reading secret...");
        let read_value = backend
            .get_secret(workspace_id, path)
            .await
            .expect("Failed to read secret with JWT");
        assert_eq!(read_value, value);
        println!("  ✓ Read (value matches)");

        // Delete (cleanup)
        println!("  Deleting secret...");
        backend
            .delete_secret(workspace_id, path)
            .await
            .expect("Failed to delete secret with JWT");
        println!("  ✓ Deleted");

        println!("✓ CRUD operations successful with JWT auth");
    }

    // ==================== Migration Tests ====================

    /// Test migration from database to Vault
    #[sqlx::test(fixtures("base", "secret_backend"))]
    async fn test_migrate_db_to_vault(db: Pool<Postgres>) {
        skip_if_no_vault!();

        let settings = vault_settings_static_token();

        // Verify Vault connection
        test_vault_connection(&settings, Some(&db))
            .await
            .expect("Failed to connect to Vault");

        // Check initial state
        let secrets_before = sqlx::query!(
            "SELECT workspace_id, path, value FROM variable WHERE is_secret = true ORDER BY workspace_id, path"
        )
        .fetch_all(&db)
        .await
        .expect("Failed to query secrets");

        println!(
            "Found {} secrets in database before migration:",
            secrets_before.len()
        );
        for s in &secrets_before {
            println!("  - {}/{}: {} chars", s.workspace_id, s.path, s.value.len());
        }

        // Run migration
        println!("\nMigrating secrets to Vault...");
        let report = migrate_secrets_to_vault(&db, &settings)
            .await
            .expect("Migration to Vault failed");

        println!("Migration report:");
        println!("  Total secrets: {}", report.total_secrets);
        println!("  Migrated: {}", report.migrated_count);
        println!("  Failed: {}", report.failed_count);

        if !report.failures.is_empty() {
            println!("  Failures:");
            for f in &report.failures {
                println!("    - {}/{}: {}", f.workspace_id, f.path, f.error);
            }
        }

        assert_eq!(report.failed_count, 0, "Migration had failures");
        assert!(report.migrated_count > 0, "No secrets were migrated");

        // Verify secrets in Vault
        println!("\nVerifying secrets in Vault...");
        let vault_backend = VaultBackend::new(settings.clone());

        for secret in &secrets_before {
            let result = vault_backend
                .get_secret(&secret.workspace_id, &secret.path)
                .await;
            assert!(
                result.is_ok(),
                "Failed to read secret {}/{} from Vault: {:?}",
                secret.workspace_id,
                secret.path,
                result.err()
            );
            println!(
                "  ✓ {}/{} exists in Vault",
                secret.workspace_id, secret.path
            );
        }

        println!("\n✓ Migration to Vault completed successfully");
    }

    /// Test migration from Vault back to database
    #[sqlx::test(fixtures("base", "secret_backend"))]
    async fn test_migrate_vault_to_db(db: Pool<Postgres>) {
        skip_if_no_vault!();

        let settings = vault_settings_static_token();

        test_vault_connection(&settings, Some(&db))
            .await
            .expect("Failed to connect to Vault");

        // First migrate TO Vault
        println!("Setting up: migrating secrets to Vault first...");
        let to_vault = migrate_secrets_to_vault(&db, &settings)
            .await
            .expect("Initial migration to Vault failed");
        assert!(to_vault.migrated_count > 0, "No secrets to test with");
        println!("  Migrated {} secrets to Vault", to_vault.migrated_count);

        // Clear database values
        println!("\nClearing database secret values...");
        sqlx::query!("UPDATE variable SET value = 'CLEARED' WHERE is_secret = true")
            .execute(&db)
            .await
            .expect("Failed to clear values");

        // Migrate back from Vault
        println!("\nMigrating secrets from Vault to database...");
        let report = migrate_secrets_to_database(&db, &settings)
            .await
            .expect("Migration to database failed");

        println!("Migration report:");
        println!("  Total secrets: {}", report.total_secrets);
        println!("  Migrated: {}", report.migrated_count);
        println!("  Failed: {}", report.failed_count);

        assert_eq!(report.failed_count, 0, "Migration had failures");
        assert!(report.migrated_count > 0, "No secrets were migrated");

        // Verify restored
        let restored = sqlx::query!(
            "SELECT COUNT(*) as count FROM variable WHERE is_secret = true AND value != 'CLEARED'"
        )
        .fetch_one(&db)
        .await
        .expect("Failed to count restored");

        assert!(
            restored.count.unwrap_or(0) > 0,
            "No secrets were restored in database"
        );

        println!("\n✓ Migration to database completed successfully");
    }

    // ==================== Variable Rename Tests ====================

    /// Test renaming a variable path in Vault
    #[sqlx::test(fixtures("base", "secret_backend"))]
    async fn test_variable_rename(db: Pool<Postgres>) {
        skip_if_no_vault!();
        let _ = &db; // suppress unused warning

        let settings = vault_settings_static_token();
        let backend = VaultBackend::new(settings);

        let workspace_id = "test-workspace";
        let old_path = "u/test-user/old_secret_name";
        let new_path = "u/test-user/new_secret_name";
        let value = "secret-value-for-rename-test";

        println!("Testing variable rename in Vault...");

        // Create secret at old path
        println!("  Creating secret at old path: {}", old_path);
        backend
            .set_secret(workspace_id, old_path, value)
            .await
            .expect("Failed to create secret");

        // Verify it exists
        let read_value = backend
            .get_secret(workspace_id, old_path)
            .await
            .expect("Failed to read secret at old path");
        assert_eq!(read_value, value);
        println!("  ✓ Secret exists at old path");

        // Simulate rename: read from old, write to new, delete old
        println!("  Renaming: {} -> {}", old_path, new_path);
        let secret_value = backend
            .get_secret(workspace_id, old_path)
            .await
            .expect("Failed to read for rename");

        backend
            .set_secret(workspace_id, new_path, &secret_value)
            .await
            .expect("Failed to write to new path");

        backend
            .delete_secret(workspace_id, old_path)
            .await
            .expect("Failed to delete old path");

        // Verify old path is gone
        let old_result = backend.get_secret(workspace_id, old_path).await;
        assert!(old_result.is_err(), "Old path should not exist");
        println!("  ✓ Old path deleted");

        // Verify new path exists with correct value
        let new_value = backend
            .get_secret(workspace_id, new_path)
            .await
            .expect("Failed to read new path");
        assert_eq!(new_value, value);
        println!("  ✓ New path exists with correct value");

        // Cleanup
        backend
            .delete_secret(workspace_id, new_path)
            .await
            .expect("Failed to cleanup");

        println!("\n✓ Variable rename completed successfully");
    }

    // ==================== Full Round Trip Test ====================

    /// Test full round-trip: DB -> Vault -> DB with verification
    #[sqlx::test(fixtures("base", "secret_backend"))]
    async fn test_full_round_trip(db: Pool<Postgres>) {
        skip_if_no_vault!();

        let settings = vault_settings_static_token();

        test_vault_connection(&settings, Some(&db))
            .await
            .expect("Failed to connect to Vault");

        // Get original secrets
        let original: HashMap<(String, String), String> = sqlx::query!(
            "SELECT workspace_id, path, value FROM variable WHERE is_secret = true"
        )
        .fetch_all(&db)
        .await
        .expect("Failed to query")
        .into_iter()
        .map(|r| ((r.workspace_id, r.path), r.value))
        .collect();

        println!("Original secrets: {} entries", original.len());

        // Step 1: DB -> Vault
        println!("\n=== Step 1: Migrate DB -> Vault ===");
        let to_vault = migrate_secrets_to_vault(&db, &settings)
            .await
            .expect("Migration to Vault failed");
        println!("Migrated {} secrets to Vault", to_vault.migrated_count);
        assert_eq!(to_vault.failed_count, 0);

        // Step 2: Clear DB
        println!("\n=== Step 2: Clear database values ===");
        sqlx::query!("UPDATE variable SET value = 'ROUND_TRIP_CLEARED' WHERE is_secret = true")
            .execute(&db)
            .await
            .expect("Failed to clear");

        // Step 3: Vault -> DB
        println!("\n=== Step 3: Migrate Vault -> DB ===");
        let to_db = migrate_secrets_to_database(&db, &settings)
            .await
            .expect("Migration to database failed");
        println!("Migrated {} secrets to database", to_db.migrated_count);
        assert_eq!(to_db.failed_count, 0);

        // Step 4: Verify
        println!("\n=== Step 4: Verify round-trip integrity ===");
        let restored: HashMap<(String, String), String> = sqlx::query!(
            "SELECT workspace_id, path, value FROM variable WHERE is_secret = true"
        )
        .fetch_all(&db)
        .await
        .expect("Failed to query")
        .into_iter()
        .map(|r| ((r.workspace_id, r.path), r.value))
        .collect();

        for ((ws, path), _) in &original {
            let restored_value = restored
                .get(&(ws.clone(), path.clone()))
                .expect(&format!("Secret {}/{} not found after round-trip", ws, path));

            assert_ne!(
                restored_value, "ROUND_TRIP_CLEARED",
                "Secret {}/{} was not restored",
                ws, path
            );
            println!("  ✓ {}/{}: restored", ws, path);
        }

        println!("\n✓ Full round-trip completed successfully!");
    }

    // ==================== Workspace Isolation Test ====================

    /// Test that workspace isolation is maintained
    #[sqlx::test(fixtures("base", "secret_backend"))]
    async fn test_workspace_isolation(db: Pool<Postgres>) {
        skip_if_no_vault!();

        let settings = vault_settings_static_token();
        let backend = VaultBackend::new(settings.clone());

        // First migrate secrets to Vault
        migrate_secrets_to_vault(&db, &settings)
            .await
            .expect("Migration failed");

        println!("Testing workspace isolation...");

        // Try to access workspace-2 secret from workspace-1 path (should fail)
        let cross_access = backend
            .get_secret("test-workspace", "u/test-user/other_secret")
            .await;

        assert!(
            cross_access.is_err(),
            "Cross-workspace access should fail!"
        );
        println!("✓ Cross-workspace access correctly denied");

        // Verify own workspace access works
        let ws1 = backend
            .get_secret("test-workspace", "u/test-user/db_password")
            .await;
        assert!(ws1.is_ok(), "Same-workspace access should work");
        println!("✓ Same-workspace access works");

        println!("\n✓ Workspace isolation verified!");
    }
}

// OSS version - just a placeholder to avoid compilation errors
#[cfg(not(all(feature = "private", feature = "enterprise")))]
mod tests {
    #[test]
    fn test_vault_requires_enterprise() {
        println!("Vault integration tests require Enterprise Edition features");
        println!("Run with: cargo test --features private,enterprise,openidconnect");
    }
}
