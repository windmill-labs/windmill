//! End-to-end reproduction for WIN-2161: forking a workspace must replicate
//! *all* secrets to the external backend, not only the ones whose DB `value`
//! column holds a `$vault:`/`$azure_kv:`/`$aws_sm:` marker.
//!
//! This drives the real `AzureKeyVaultBackend` and the real `get_secret_value`
//! resolver against a local emulator; the fork replication loop itself is
//! mirrored inline (parameterized by the old/new behavior) since the production
//! `clone_variables` lives in another crate and is not public. It verifies the
//! external round-trip and the fix's effect, not the exact call site.
//!
//! Root cause: `migrate_secrets_to_*` writes the plaintext to the external store
//! but leaves the encrypted ciphertext in `variable.value` (it never rewrites it
//! to a marker). The old fork replication filtered on
//! `is_external_stored_value(value)`, so every migrated secret was skipped and
//! never copied to the fork's workspace id — reads in the fork then 404 with
//! "Secret ... not found in Azure Key Vault".
//!
//! This test runs against a local Azure Key Vault emulator (lowkey-vault), which
//! the real `AzureKeyVaultBackend` talks to via its static-token / self-signed
//! cert "emulator mode".
//!
//! Run it:
//! ```bash
//! podman run -d --name lowkey -p 8443:8443 \
//!     -e LOWKEY_ARGS="--LOWKEY_VAULT_NAMES=default" \
//!     docker.io/nagyesta/lowkey-vault:7.3.0
//!
//! RUN_AZURE_KV_TESTS=1 cargo test -p windmill-common \
//!     --features private,enterprise --test fork_secret_replication_azure -- --nocapture
//! ```

#[cfg(all(feature = "private", feature = "enterprise"))]
mod tests {
    use sqlx::{Pool, Postgres};
    use windmill_common::{
        global_settings::SECRET_BACKEND_SETTING,
        secret_backend::{
            get_secret_backend, get_secret_value, is_external_stored_value, AzureKeyVaultSettings,
            SecretBackendConfig,
        },
        variables::{build_crypt, encrypt},
    };

    fn should_run() -> bool {
        std::env::var("RUN_AZURE_KV_TESTS").as_deref() == Ok("1")
    }

    fn azure_settings() -> AzureKeyVaultSettings {
        AzureKeyVaultSettings {
            vault_url: std::env::var("AZURE_KV_URL")
                .unwrap_or_else(|_| "https://localhost:8443".to_string()),
            tenant_id: "emulator-tenant".to_string(),
            client_id: "emulator-client".to_string(),
            client_secret: None,
            // Static token => emulator mode (bypasses Entra ID, accepts self-signed certs).
            token: Some("emulator-token".to_string()),
        }
    }

    /// Point the instance's secret backend at the Azure KV emulator.
    async fn configure_azure_backend(db: &Pool<Postgres>) {
        let config = serde_json::to_value(SecretBackendConfig::AzureKeyVault(azure_settings()))
            .expect("serialize config");
        sqlx::query(
            "INSERT INTO global_settings (name, value) VALUES ($1, $2)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(SECRET_BACKEND_SETTING)
        .bind(config)
        .execute(db)
        .await
        .expect("insert secret_backend global setting");
    }

    async fn create_workspace(db: &Pool<Postgres>, id: &str) {
        sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ($1, $1, 'test-user') ON CONFLICT DO NOTHING")
            .bind(id)
            .execute(db)
            .await
            .expect("insert workspace");
        sqlx::query(
            "INSERT INTO workspace_settings (workspace_id) VALUES ($1) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .execute(db)
        .await
        .expect("insert workspace_settings");
        sqlx::query("INSERT INTO workspace_key (workspace_id, kind, key) VALUES ($1, 'cloud', $2) ON CONFLICT DO NOTHING")
            .bind(id)
            .bind(format!("key-{id}"))
            .execute(db)
            .await
            .expect("insert workspace_key");
    }

    /// Reproduces exactly what `clone_variables` does for external backends,
    /// parameterized by whether the old `is_external_stored_value` filter is
    /// applied. This mirrors backend/windmill-api-workspaces/src/workspaces.rs.
    async fn replicate_secrets_to_fork(
        db: &Pool<Postgres>,
        source_ws: &str,
        fork_ws: &str,
        apply_old_filter: bool,
    ) {
        let backend = get_secret_backend(db).await.expect("get backend");

        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT path, value FROM variable
             WHERE workspace_id = $1 AND is_secret = true AND value != ''",
        )
        .bind(fork_ws)
        .fetch_all(db)
        .await
        .expect("select secret variables");

        for (path, value) in rows {
            if apply_old_filter && !is_external_stored_value(&value) {
                continue; // old behavior: skip non-marker values (the bug)
            }
            match backend.get_secret(source_ws, &path).await {
                Ok(plain) => backend
                    .set_secret(fork_ws, &path, &plain)
                    .await
                    .expect("replicate secret"),
                Err(_) => { /* source unreadable: fork does not block */ }
            }
        }
    }

    /// Full before/after reproduction of WIN-2161 against the Azure KV emulator.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn migrated_secret_is_replicated_on_fork(db: Pool<Postgres>) {
        if !should_run() {
            eprintln!("skipping: set RUN_AZURE_KV_TESTS=1 and start lowkey-vault to run");
            return;
        }

        let source_ws = "test-workspace";
        let path = "u/test-user/db_password";
        let plaintext = "s3cr3t-value";

        configure_azure_backend(&db).await;
        let backend = get_secret_backend(&db).await.expect("get backend");

        // --- Simulate the post-migration state of a secret ---------------------
        // migrate_secrets_to_azure_kv writes the plaintext to the external store
        // but leaves the encrypted ciphertext in variable.value (NOT a marker).
        backend
            .set_secret(source_ws, path, plaintext)
            .await
            .expect("seed secret in Azure KV emulator");
        let ciphertext = {
            let mc = build_crypt(&db, source_ws).await.expect("build_crypt");
            encrypt(&mc, plaintext)
        };
        assert!(
            !is_external_stored_value(&ciphertext),
            "post-migration value is ciphertext, not a marker (this is the crux of the bug)"
        );
        sqlx::query(
            "INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
             VALUES ($1, $2, $3, true, '', '{}')",
        )
        .bind(source_ws)
        .bind(path)
        .bind(&ciphertext)
        .execute(&db)
        .await
        .expect("insert source secret variable");

        // Sanity: the source workspace resolves the secret through the backend.
        let src = get_secret_value(&db, source_ws, path, &ciphertext)
            .await
            .expect("source secret resolves");
        assert_eq!(src, plaintext, "source read should return the plaintext");

        // --- Fork: copy the variable row (clone_variables' INSERT..SELECT) -----
        let fork_before = "forked-ws-oldbehavior";
        create_workspace(&db, fork_before).await;
        sqlx::query(
            "INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
             VALUES ($1, $2, $3, true, '', '{}')",
        )
        .bind(fork_before)
        .bind(path)
        .bind(&ciphertext)
        .execute(&db)
        .await
        .unwrap();

        // BEFORE FIX: old filter skips the ciphertext-valued secret.
        replicate_secrets_to_fork(
            &db,
            source_ws,
            fork_before,
            /* apply_old_filter */ true,
        )
        .await;
        let before = get_secret_value(&db, fork_before, path, &ciphertext).await;
        eprintln!("[before fix] fork read => {before:?}");
        let err = before.expect_err("BUG: fork read should fail before the fix");
        assert!(
            err.to_string().contains("not found in Azure Key Vault"),
            "expected the customer-reported error, got: {err}"
        );

        // --- Fork again with the fix ------------------------------------------
        let fork_after = "forked-ws-newbehavior";
        create_workspace(&db, fork_after).await;
        sqlx::query(
            "INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
             VALUES ($1, $2, $3, true, '', '{}')",
        )
        .bind(fork_after)
        .bind(path)
        .bind(&ciphertext)
        .execute(&db)
        .await
        .unwrap();

        // AFTER FIX: no filter, every secret is replicated.
        replicate_secrets_to_fork(
            &db, source_ws, fork_after, /* apply_old_filter */ false,
        )
        .await;
        let after = get_secret_value(&db, fork_after, path, &ciphertext)
            .await
            .expect("AFTER FIX: fork read should resolve the secret");
        eprintln!("[after fix]  fork read => Ok({after:?})");
        assert_eq!(
            after, plaintext,
            "fork should return the replicated plaintext"
        );
    }
}
