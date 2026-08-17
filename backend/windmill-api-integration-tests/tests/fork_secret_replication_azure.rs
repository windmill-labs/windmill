//! End-to-end regression test for WIN-2161.
//!
//! Reproduces, through real product code, the state after a database-to-external
//! migration: a secret that was created under the database backend and then
//! *migrated* to an external backend (Azure Key Vault). Migration writes the
//! plaintext to the store but
//! leaves the encrypted ciphertext in `variable.value` (it never rewrites it to
//! a `$azure_kv:` marker). The bug: `clone_variables` only replicated
//! marker-valued secrets, so forking left the migrated secret unreplicated and
//! reads in the fork failed with "not found in Azure Key Vault".
//!
//! This drives the real `/migrate_secrets_to_azure_kv`, `/create_fork` and
//! `variables/get_value` endpoints against a local Azure Key Vault emulator
//! (lowkey-vault), which the `AzureKeyVaultBackend` talks to via its
//! static-token / self-signed-cert emulator mode.
//!
//! Run it:
//! ```bash
//! podman run -d --name lowkey -p 8443:8443 \
//!     -e LOWKEY_ARGS="--LOWKEY_VAULT_NAMES=default" \
//!     docker.io/nagyesta/lowkey-vault:7.3.0
//!
//! RUN_AZURE_KV_TESTS=1 cargo test -p windmill-api-integration-tests \
//!     --features private,enterprise --test fork_secret_replication_azure -- --nocapture
//! ```

#[cfg(all(feature = "private", feature = "enterprise"))]
mod azure_fork {
    use serde_json::json;
    use sqlx::{Pool, Postgres};
    use windmill_common::variables::{build_crypt, encrypt};
    use windmill_test_utils::*;

    fn client() -> reqwest::Client {
        reqwest::Client::new()
    }

    fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder.header("Authorization", "Bearer SECRET_TOKEN")
    }

    fn vault_url() -> String {
        std::env::var("AZURE_KV_URL").unwrap_or_else(|_| "https://localhost:8443".to_string())
    }

    /// The Azure settings for the emulator: a static token switches the backend
    /// into emulator mode (no Entra ID, self-signed certs accepted).
    fn azure_settings() -> serde_json::Value {
        json!({
            "vault_url": vault_url(),
            "tenant_id": "emulator-tenant",
            "client_id": "emulator-client",
            "token": "emulator-token",
        })
    }

    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn migrated_secret_is_replicated_on_fork(db: Pool<Postgres>) -> anyhow::Result<()> {
        if std::env::var("RUN_AZURE_KV_TESTS").as_deref() != Ok("1") {
            eprintln!("skipping: set RUN_AZURE_KV_TESTS=1 and start lowkey-vault to run");
            return Ok(());
        }
        initialize_tracing().await;

        // The Azure KV emulator persists across runs; derive unique names per run
        // so a secret written by a previous run can't mask a regression.
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let short = &suffix[..8];
        let source_ws = "test-workspace";
        let path = format!("u/test-user/db_password_{short}");
        let path = path.as_str();
        let plaintext = "s3cr3t-value";

        let ciphertext = {
            let mc = build_crypt(&db, source_ws).await?;
            encrypt(&mc, plaintext)
        };
        sqlx::query(
            "INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
             VALUES ($1, $2, $3, true, '', '{}')",
        )
        .bind(source_ws)
        .bind(path)
        .bind(&ciphertext)
        .execute(&db)
        .await?;

        sqlx::query(
            "INSERT INTO global_settings (name, value) VALUES ('secret_backend', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(json!({
            "type": "AzureKeyVault",
            "vault_url": vault_url(),
            "tenant_id": "emulator-tenant",
            "client_id": "emulator-client",
            "token": "emulator-token",
        }))
        .execute(&db)
        .await?;

        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let resp = authed(client().post(format!(
            "http://localhost:{port}/api/settings/migrate_secrets_to_azure_kv"
        )))
        .json(&azure_settings())
        .send()
        .await?;
        let status = resp.status();
        let body = resp.text().await?;
        assert_eq!(status, 200, "migrate_secrets_to_azure_kv failed: {body}");
        let report: serde_json::Value = serde_json::from_str(&body)?;
        assert!(
            report["migrated_count"].as_i64().unwrap_or(0) >= 1,
            "expected at least one migrated secret: {report}"
        );

        // Assert the source resolves before forking, so a fork-read failure is
        // attributable to replication rather than a broken seed.
        let resp = authed(client().get(format!(
            "http://localhost:{port}/api/w/{source_ws}/variables/get_value/{path}"
        )))
        .send()
        .await?;
        assert_eq!(resp.status(), 200, "source read: {}", resp.text().await?);
        assert_eq!(resp.json::<String>().await?, plaintext);

        let fork_ws = format!("wm-fork-az{short}");
        let fork_ws = fork_ws.as_str();
        let resp = authed(client().post(format!(
            "http://localhost:{port}/api/w/{source_ws}/workspaces/create_fork"
        )))
        .json(&json!({ "id": fork_ws, "name": "Azure Fork Test" }))
        .send()
        .await?;
        assert_eq!(resp.status(), 200, "create_fork: {}", resp.text().await?);

        // The fork resolves the secret only if it was replicated under the fork's
        // own workspace-id key in the external store.
        let resp = authed(client().get(format!(
            "http://localhost:{port}/api/w/{fork_ws}/variables/get_value/{path}"
        )))
        .send()
        .await?;
        let status = resp.status();
        let body = resp.text().await?;
        assert_eq!(
            status, 200,
            "forked secret must resolve, got {status}: {body}"
        );
        assert_eq!(
            serde_json::from_str::<String>(&body)?,
            plaintext,
            "fork should return the replicated plaintext"
        );

        Ok(())
    }
}
