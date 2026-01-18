/*
 * Integration tests for secret backend
 *
 * These tests require a running Vault instance at http://127.0.0.1:8200
 * with the token "test-root-token" and a KV v2 mount at "windmill".
 *
 * Run Vault dev server:
 * podman run -d --name vault-test --rm -p 8200:8200 \
 *   -e 'VAULT_DEV_ROOT_TOKEN_ID=test-root-token' \
 *   -e 'VAULT_DEV_LISTEN_ADDRESS=0.0.0.0:8200' \
 *   docker.io/hashicorp/vault:latest
 *
 * Then enable the windmill mount:
 * curl --header "X-Vault-Token: test-root-token" \
 *   --request POST \
 *   --data '{"type": "kv", "options": {"version": "2"}}' \
 *   http://127.0.0.1:8200/v1/sys/mounts/windmill
 */

#[cfg(test)]
mod tests {
    use crate::secret_backend::{SecretBackend, VaultBackend, VaultSettings};

    fn test_settings() -> VaultSettings {
        VaultSettings {
            address: "http://127.0.0.1:8200".to_string(),
            mount_path: "windmill".to_string(),
            jwt_role: Some("windmill-secrets".to_string()),
            namespace: None,
            token: Some("test-root-token".to_string()),
        }
    }

    #[tokio::test]
    #[ignore] // Run with --ignored to execute
    async fn test_vault_write_and_read() {
        let settings = test_settings();
        let backend = VaultBackend::new(settings);

        let workspace_id = "test-ws-1";
        let path = "test-secret";
        let value = "my-super-secret-value";

        // Write secret
        backend
            .set_secret(workspace_id, path, value)
            .await
            .expect("Failed to write secret");

        // Read secret
        let read_value = backend
            .get_secret(workspace_id, path)
            .await
            .expect("Failed to read secret");

        assert_eq!(read_value, value);

        // Delete secret
        backend
            .delete_secret(workspace_id, path)
            .await
            .expect("Failed to delete secret");

        // Verify deleted
        let result = backend.get_secret(workspace_id, path).await;
        assert!(result.is_err(), "Secret should be deleted");
    }

    #[tokio::test]
    #[ignore]
    async fn test_vault_multiple_secrets() {
        let settings = test_settings();
        let backend = VaultBackend::new(settings);

        let workspace_id = "test-ws-2";

        // Write multiple secrets
        for i in 0..5 {
            let path = format!("secret-{}", i);
            let value = format!("value-{}", i);
            backend
                .set_secret(workspace_id, &path, &value)
                .await
                .expect(&format!("Failed to write secret-{}", i));
        }

        // Read and verify
        for i in 0..5 {
            let path = format!("secret-{}", i);
            let expected = format!("value-{}", i);
            let value = backend
                .get_secret(workspace_id, &path)
                .await
                .expect(&format!("Failed to read secret-{}", i));
            assert_eq!(value, expected);
        }

        // Cleanup
        for i in 0..5 {
            let path = format!("secret-{}", i);
            backend
                .delete_secret(workspace_id, &path)
                .await
                .expect(&format!("Failed to delete secret-{}", i));
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_vault_overwrite_secret() {
        let settings = test_settings();
        let backend = VaultBackend::new(settings);

        let workspace_id = "test-ws-3";
        let path = "overwrite-test";

        // Write initial value
        backend
            .set_secret(workspace_id, path, "initial-value")
            .await
            .expect("Failed to write initial value");

        // Overwrite
        backend
            .set_secret(workspace_id, path, "new-value")
            .await
            .expect("Failed to overwrite");

        // Read and verify
        let value = backend
            .get_secret(workspace_id, path)
            .await
            .expect("Failed to read");

        assert_eq!(value, "new-value");

        // Cleanup
        backend
            .delete_secret(workspace_id, path)
            .await
            .expect("Failed to delete");
    }
}
