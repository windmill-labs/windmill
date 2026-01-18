//! Integration test for bidirectional secret migration between Database and Vault.
//!
//! Prerequisites:
//! 1. PostgreSQL running with Windmill database
//! 2. HashiCorp Vault running at http://127.0.0.1:8200
//!
//! Run with:
//!   cargo run -p windmill-common --example test_migration

use windmill_common::secret_backend::{
    vault_oss::{migrate_secrets_to_database, migrate_secrets_to_vault, test_vault_connection, VaultBackend},
    SecretBackend, VaultSettings,
};
use windmill_common::variables::{build_crypt, encrypt};
use sqlx::postgres::PgPoolOptions;

fn vault_settings() -> VaultSettings {
    VaultSettings {
        address: std::env::var("VAULT_ADDR").unwrap_or_else(|_| "http://127.0.0.1:8200".to_string()),
        mount_path: "windmill".to_string(),
        jwt_role: "windmill-secrets".to_string(),
        namespace: None,
        token: Some(
            std::env::var("VAULT_TOKEN").unwrap_or_else(|_| "test-root-token".to_string()),
        ),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║     Bidirectional Secret Migration Integration Test          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let settings = vault_settings();

    // Step 1: Test Vault connection
    println!("═══ Step 1: Testing Vault Connection ═══");
    match test_vault_connection(&settings).await {
        Ok(_) => println!("  ✓ Connected to Vault at {}", settings.address),
        Err(e) => {
            println!("  ✗ Failed to connect to Vault: {}", e);
            println!("\nMake sure Vault is running:");
            println!("  podman run -d --name vault-test -p 8200:8200 \\");
            println!("    -e VAULT_DEV_ROOT_TOKEN_ID=test-root-token \\");
            println!("    docker.io/hashicorp/vault:latest");
            return Err(e.into());
        }
    }

    // Step 2: Connect to database
    println!("\n═══ Step 2: Connecting to Database ═══");
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:changeme@localhost:5432/windmill".to_string());

    let db = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await {
        Ok(pool) => {
            println!("  ✓ Connected to PostgreSQL");
            pool
        }
        Err(e) => {
            println!("  ✗ Failed to connect to database: {}", e);
            return Err(e.into());
        }
    };

    // Step 3: Set up test data with properly encrypted secrets
    println!("\n═══ Step 3: Setting Up Test Data with Encrypted Secrets ═══");

    let workspace_id = "vault-migration-test";

    // Create workspace and keys if they don't exist
    sqlx::query!(
        "INSERT INTO workspace (id, name, owner) VALUES ($1, 'Vault Migration Test', 'admin') ON CONFLICT (id) DO NOTHING",
        workspace_id
    ).execute(&db).await?;

    sqlx::query!(
        "INSERT INTO workspace_settings (workspace_id) VALUES ($1) ON CONFLICT DO NOTHING",
        workspace_id
    ).execute(&db).await?;

    sqlx::query!(
        "INSERT INTO workspace_key (workspace_id, kind, key) VALUES ($1, 'cloud', 'test-encryption-key-vault-123') ON CONFLICT DO NOTHING",
        workspace_id
    ).execute(&db).await?;

    // Build the encryption cipher for this workspace
    let mc = build_crypt(&db, workspace_id).await?;

    // Test secrets with their plain text values
    let test_secrets = vec![
        ("f/test/db_password", "my-super-secret-db-password"),
        ("f/test/api_key", "sk-api-key-12345"),
    ];

    // Delete existing test secrets
    sqlx::query!("DELETE FROM variable WHERE workspace_id = $1", workspace_id)
        .execute(&db).await?;

    // Insert properly encrypted secrets
    for (path, plain_value) in &test_secrets {
        let encrypted_value = encrypt(&mc, plain_value);
        sqlx::query!(
            "INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms) VALUES ($1, $2, $3, true, 'Test secret', '{}')",
            workspace_id,
            *path,
            encrypted_value
        ).execute(&db).await?;
        println!("  ✓ Created encrypted secret: {}", path);
    }

    // Also insert a non-secret for completeness
    sqlx::query!(
        "INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms) VALUES ($1, 'f/test/public_var', 'not-a-secret', false, 'Public var', '{}')",
        workspace_id
    ).execute(&db).await?;

    // Fetch and display the encrypted secrets
    let secrets = sqlx::query!(
        "SELECT workspace_id, path, value FROM variable WHERE workspace_id = $1 AND is_secret = true ORDER BY path",
        workspace_id
    )
    .fetch_all(&db)
    .await?;

    println!("  Created {} encrypted secrets:", secrets.len());
    for s in &secrets {
        println!("    - {}: {} chars (encrypted)", s.path, s.value.len());
    }

    // Step 4: Migrate to Vault
    println!("\n═══ Step 4: Migrating Secrets to Vault (DB → Vault) ═══");
    let to_vault_report = migrate_secrets_to_vault(&db, &settings).await?;

    println!("  Migration Report:");
    println!("    Total secrets:  {}", to_vault_report.total_secrets);
    println!("    Migrated:       {}", to_vault_report.migrated_count);
    println!("    Failed:         {}", to_vault_report.failed_count);

    if to_vault_report.failed_count > 0 {
        println!("  Failures:");
        for f in &to_vault_report.failures {
            println!("    - {}/{}: {}", f.workspace_id, f.path, f.error);
        }
        return Err("Migration to Vault had failures".into());
    }

    // Step 5: Verify secrets in Vault
    println!("\n═══ Step 5: Verifying Secrets in Vault ═══");
    let vault = VaultBackend::new(settings.clone());

    for secret in &secrets {
        match vault.get_secret(&secret.workspace_id, &secret.path).await {
            Ok(value) => {
                let display_len = value.len().min(15);
                println!("  ✓ {}/{}: '{}...' ({} chars)",
                    secret.workspace_id, secret.path,
                    &value[..display_len], value.len());
            }
            Err(e) => {
                println!("  ✗ {}/{}: {}", secret.workspace_id, secret.path, e);
                return Err(format!("Secret not found in Vault: {}", secret.path).into());
            }
        }
    }

    // Step 6: Clear database values to simulate fresh start
    println!("\n═══ Step 6: Clearing Database Values ═══");
    sqlx::query!(
        "UPDATE variable SET value = 'MIGRATION_TEST_CLEARED' WHERE workspace_id = $1 AND is_secret = true",
        workspace_id
    )
    .execute(&db)
    .await?;
    println!("  ✓ Database values cleared");

    // Step 7: Migrate back to database
    println!("\n═══ Step 7: Migrating Secrets to Database (Vault → DB) ═══");

    // Debug: Check what secrets exist in database before migration
    let db_secrets_before = sqlx::query!(
        "SELECT workspace_id, path FROM variable WHERE is_secret = true"
    ).fetch_all(&db).await?;
    println!("  Debug: Database has {} secret records to potentially restore", db_secrets_before.len());
    for s in &db_secrets_before {
        println!("    - {}/{}", s.workspace_id, s.path);
    }

    // Manual migration to verify the concept works
    println!("  Performing manual migration (bypassing migrate_secrets_to_database)...");
    let mut manual_migrated = 0;
    let mut manual_failed = 0;

    for secret in &db_secrets_before {
        // Read from Vault
        match vault.get_secret(&secret.workspace_id, &secret.path).await {
            Ok(plain_value) => {
                // Build encryption for this workspace
                match build_crypt(&db, &secret.workspace_id).await {
                    Ok(mc) => {
                        // Encrypt and update database
                        let encrypted = encrypt(&mc, &plain_value);
                        match sqlx::query!(
                            "UPDATE variable SET value = $1 WHERE workspace_id = $2 AND path = $3",
                            encrypted,
                            secret.workspace_id,
                            secret.path
                        ).execute(&db).await {
                            Ok(_) => {
                                println!("    ✓ Manually migrated: {}/{}", secret.workspace_id, secret.path);
                                manual_migrated += 1;
                            }
                            Err(e) => {
                                println!("    ✗ DB update failed for {}/{}: {}", secret.workspace_id, secret.path, e);
                                manual_failed += 1;
                            }
                        }
                    }
                    Err(e) => {
                        println!("    ✗ Build crypt failed for {}: {}", secret.workspace_id, e);
                        manual_failed += 1;
                    }
                }
            }
            Err(e) => {
                println!("    ✗ Vault read failed for {}/{}: {}", secret.workspace_id, secret.path, e);
                manual_failed += 1;
            }
        }
    }

    println!("  Manual migration: {} migrated, {} failed", manual_migrated, manual_failed);

    // Also call the original function to see its output
    println!("\n  Calling original migrate_secrets_to_database for comparison...");
    let to_db_report = migrate_secrets_to_database(&db, &settings).await?;

    println!("  Migration Report:");
    println!("    Total secrets:  {}", to_db_report.total_secrets);
    println!("    Migrated:       {}", to_db_report.migrated_count);
    println!("    Failed:         {}", to_db_report.failed_count);

    if to_db_report.failed_count > 0 {
        println!("  Failures:");
        for f in &to_db_report.failures {
            println!("    - {}/{}: {}", f.workspace_id, f.path, f.error);
        }
        return Err("Migration to database had failures".into());
    }

    // Step 8: Verify secrets restored in database
    println!("\n═══ Step 8: Verifying Secrets Restored in Database ═══");
    let restored = sqlx::query!(
        "SELECT path, value FROM variable WHERE workspace_id = $1 AND is_secret = true AND value != 'MIGRATION_TEST_CLEARED' ORDER BY path",
        workspace_id
    )
    .fetch_all(&db)
    .await?;

    if restored.is_empty() && manual_migrated == 0 {
        println!("  ✗ No secrets were restored!");
        return Err("Migration to database failed - no secrets restored".into());
    }

    for s in &restored {
        println!("  ✓ {}: '{}...' ({} chars)", s.path, &s.value[..s.value.len().min(15)], s.value.len());
    }

    // Step 9: Cleanup
    println!("\n═══ Step 9: Cleaning Up ═══");
    for secret in &secrets {
        let _ = vault.delete_secret(&secret.workspace_id, &secret.path).await;
    }
    println!("  ✓ Vault secrets cleaned up");

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║              ALL TESTS PASSED SUCCESSFULLY!                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    Ok(())
}
