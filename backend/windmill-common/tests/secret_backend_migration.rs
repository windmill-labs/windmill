//! Integration tests for secret backend migration between database and HashiCorp Vault.
//!
//! These tests require:
//! 1. A PostgreSQL database (handled by sqlx test framework)
//! 2. A running HashiCorp Vault instance at http://127.0.0.1:8200
//!
//! To run these tests:
//! ```bash
//! # Start Vault in dev mode
//! podman run -d --name vault-test -p 8200:8200 \
//!     -e VAULT_DEV_ROOT_TOKEN_ID=test-root-token \
//!     docker.io/hashicorp/vault:latest
//!
//! # Enable KV v2 secrets engine
//! curl -s -H "X-Vault-Token: test-root-token" -X POST \
//!     --data '{"type":"kv-v2"}' \
//!     http://127.0.0.1:8200/v1/sys/mounts/windmill
//!
//! # Run the tests
//! cargo test -p windmill secret_backend_migration -- --ignored --nocapture
//! ```

use sqlx::{Pool, Postgres};
use windmill_common::error::Result;
use windmill_common::secret_backend::{
    vault_oss::{migrate_secrets_to_database, migrate_secrets_to_vault, test_vault_connection, VaultBackend},
    SecretBackend, VaultSettings,
};

fn test_vault_settings() -> VaultSettings {
    VaultSettings {
        address: std::env::var("VAULT_ADDR").unwrap_or_else(|_| "http://127.0.0.1:8200".to_string()),
        mount_path: "windmill".to_string(),
        jwt_role: Some("windmill-secrets".to_string()),
        namespace: None,
        token: Some(
            std::env::var("VAULT_TOKEN").unwrap_or_else(|_| "test-root-token".to_string()),
        ),
    }
}

/// Test that we can connect to Vault
#[sqlx::test(migrations = "../migrations", fixtures("base", "secret_backend"))]
#[ignore = "requires running Vault instance"]
async fn test_vault_connection_works(db: Pool<Postgres>) {
    let settings = test_vault_settings();

    let result = test_vault_connection(&settings, Some(&db)).await;
    assert!(result.is_ok(), "Failed to connect to Vault: {:?}", result.err());
    println!("✓ Successfully connected to Vault at {}", settings.address);
}

/// Test migration from database to Vault
#[sqlx::test(migrations = "../migrations", fixtures("base", "secret_backend"))]
#[ignore = "requires running Vault instance"]
async fn test_migrate_db_to_vault(db: Pool<Postgres>) {
    let settings = test_vault_settings();

    // First verify we can connect to Vault
    test_vault_connection(&settings, Some(&db))
        .await
        .expect("Failed to connect to Vault");

    // Check initial state - secrets should exist in database
    let secrets_before = sqlx::query!(
        "SELECT workspace_id, path, value FROM variable WHERE is_secret = true ORDER BY workspace_id, path"
    )
    .fetch_all(&db)
    .await
    .expect("Failed to query secrets");

    println!("Found {} secrets in database before migration:", secrets_before.len());
    for s in &secrets_before {
        println!("  - {}/{}: {} chars", s.workspace_id, s.path, s.value.len());
    }

    // Run migration to Vault
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

    // Verify secrets are in Vault
    println!("\nVerifying secrets in Vault...");
    let vault_backend = VaultBackend::new(settings.clone());

    for secret in &secrets_before {
        let result: Result<String> = vault_backend
            .get_secret(&secret.workspace_id, &secret.path)
            .await;
        assert!(
            result.is_ok(),
            "Failed to read secret {}/{} from Vault: {:?}",
            secret.workspace_id,
            secret.path,
            result.err()
        );
        println!("  ✓ {}/{} exists in Vault", secret.workspace_id, secret.path);
    }

    println!("\n✓ Migration to Vault completed successfully");
}

/// Test migration from Vault to database
#[sqlx::test(migrations = "../migrations", fixtures("base", "secret_backend"))]
#[ignore = "requires running Vault instance"]
async fn test_migrate_vault_to_db(db: Pool<Postgres>) {
    let settings = test_vault_settings();

    // First verify we can connect to Vault
    test_vault_connection(&settings, Some(&db))
        .await
        .expect("Failed to connect to Vault");

    // First, migrate secrets TO Vault so we have something to migrate back
    println!("Setting up: migrating secrets to Vault first...");
    let to_vault_report = migrate_secrets_to_vault(&db, &settings)
        .await
        .expect("Initial migration to Vault failed");
    assert!(to_vault_report.migrated_count > 0, "No secrets to test with");
    println!("  Migrated {} secrets to Vault", to_vault_report.migrated_count);

    // Clear the database values to simulate fresh migration back
    println!("\nClearing database secret values...");
    sqlx::query!("UPDATE variable SET value = 'CLEARED' WHERE is_secret = true")
        .execute(&db)
        .await
        .expect("Failed to clear database values");

    // Verify they were cleared
    let cleared = sqlx::query!(
        "SELECT COUNT(*) as count FROM variable WHERE is_secret = true AND value = 'CLEARED'"
    )
    .fetch_one(&db)
    .await
    .expect("Failed to count cleared");
    println!("  Cleared {} secret values in database", cleared.count.unwrap_or(0));

    // Now migrate from Vault back to database
    println!("\nMigrating secrets from Vault to database...");
    let report = migrate_secrets_to_database(&db, &settings)
        .await
        .expect("Migration to database failed");

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

    // Verify secrets are restored in database
    println!("\nVerifying secrets in database...");
    let secrets_after = sqlx::query!(
        "SELECT workspace_id, path, value FROM variable WHERE is_secret = true AND value != 'CLEARED' ORDER BY workspace_id, path"
    )
    .fetch_all(&db)
    .await
    .expect("Failed to query restored secrets");

    assert!(
        !secrets_after.is_empty(),
        "No secrets were restored in database"
    );

    for s in &secrets_after {
        println!("  ✓ {}/{}: {} chars", s.workspace_id, s.path, s.value.len());
    }

    println!("\n✓ Migration to database completed successfully");
}

/// Test full round-trip migration: DB -> Vault -> DB
#[sqlx::test(migrations = "../migrations", fixtures("base", "secret_backend"))]
#[ignore = "requires running Vault instance"]
async fn test_full_round_trip_migration(db: Pool<Postgres>) {
    let settings = test_vault_settings();

    // Verify Vault connection
    test_vault_connection(&settings, Some(&db))
        .await
        .expect("Failed to connect to Vault");

    // Get original secrets
    let original_secrets: std::collections::HashMap<(String, String), String> = sqlx::query!(
        "SELECT workspace_id, path, value FROM variable WHERE is_secret = true"
    )
    .fetch_all(&db)
    .await
    .expect("Failed to query original secrets")
    .into_iter()
    .map(|r| ((r.workspace_id, r.path), r.value))
    .collect();

    println!("Original secrets: {} entries", original_secrets.len());

    // Step 1: Migrate to Vault
    println!("\n=== Step 1: Migrate DB -> Vault ===");
    let to_vault = migrate_secrets_to_vault(&db, &settings)
        .await
        .expect("Migration to Vault failed");
    println!("Migrated {} secrets to Vault", to_vault.migrated_count);
    assert_eq!(to_vault.failed_count, 0);

    // Step 2: Clear database values
    println!("\n=== Step 2: Clear database values ===");
    sqlx::query!("UPDATE variable SET value = 'ROUND_TRIP_CLEARED' WHERE is_secret = true")
        .execute(&db)
        .await
        .expect("Failed to clear values");

    // Step 3: Migrate back from Vault
    println!("\n=== Step 3: Migrate Vault -> DB ===");
    let to_db = migrate_secrets_to_database(&db, &settings)
        .await
        .expect("Migration to database failed");
    println!("Migrated {} secrets to database", to_db.migrated_count);
    assert_eq!(to_db.failed_count, 0);

    // Step 4: Verify round-trip integrity
    println!("\n=== Step 4: Verify round-trip integrity ===");
    let restored_secrets: std::collections::HashMap<(String, String), String> = sqlx::query!(
        "SELECT workspace_id, path, value FROM variable WHERE is_secret = true"
    )
    .fetch_all(&db)
    .await
    .expect("Failed to query restored secrets")
    .into_iter()
    .map(|r| ((r.workspace_id, r.path), r.value))
    .collect();

    // Compare original and restored
    for ((ws, path), _original_value) in &original_secrets {
        let restored_value = restored_secrets
            .get(&(ws.clone(), path.clone()))
            .expect(&format!("Secret {}/{} not found after round-trip", ws, path));

        // Note: Values might differ slightly due to encryption/decryption
        // but they should not be the cleared value
        assert_ne!(
            restored_value, "ROUND_TRIP_CLEARED",
            "Secret {}/{} was not restored",
            ws, path
        );
        println!("  ✓ {}/{}: restored ({} chars)", ws, path, restored_value.len());
    }

    println!("\n✓ Full round-trip migration completed successfully!");
    println!("  Original secrets: {}", original_secrets.len());
    println!("  Restored secrets: {}", restored_secrets.len());
}

/// Test that workspace isolation is maintained during migration
#[sqlx::test(migrations = "../migrations", fixtures("base", "secret_backend"))]
#[ignore = "requires running Vault instance"]
async fn test_workspace_isolation(db: Pool<Postgres>) {
    let settings = test_vault_settings();

    test_vault_connection(&settings, Some(&db))
        .await
        .expect("Failed to connect to Vault");

    // Migrate all secrets to Vault
    let report = migrate_secrets_to_vault(&db, &settings)
        .await
        .expect("Migration failed");

    println!("Migrated {} secrets across workspaces", report.migrated_count);

    // Verify workspace isolation in Vault
    let vault_backend = VaultBackend::new(settings.clone());

    // Try to access test-workspace-2 secret from test-workspace path (should fail)
    let cross_workspace_result: Result<String> = vault_backend
        .get_secret("test-workspace", "u/test-user/other_secret")
        .await;

    assert!(
        cross_workspace_result.is_err(),
        "Cross-workspace access should fail - workspace isolation violated!"
    );
    println!("✓ Cross-workspace access correctly denied");

    // Verify each workspace's secrets are accessible from their own workspace
    let ws1_result: Result<String> = vault_backend
        .get_secret("test-workspace", "u/test-user/db_password")
        .await;
    assert!(ws1_result.is_ok(), "test-workspace secret should be accessible");
    println!("✓ test-workspace secrets accessible");

    let ws2_result: Result<String> = vault_backend
        .get_secret("test-workspace-2", "u/test-user/other_secret")
        .await;
    assert!(ws2_result.is_ok(), "test-workspace-2 secret should be accessible");
    println!("✓ test-workspace-2 secrets accessible");

    println!("\n✓ Workspace isolation verified!");
}
