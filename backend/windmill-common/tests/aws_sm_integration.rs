//! Integration tests for AWS Secrets Manager secret backend.
//!
//! These tests require a running LocalStack instance with the `secretsmanager` service.
//!
//! ## Setup
//!
//! 1. Start LocalStack:
//!
//!    ```bash
//!    docker run -d --name localstack -p 4566:4566 \
//!      -e SERVICES=secretsmanager \
//!      localstack/localstack:3.8
//!    ```
//!
//! 2. Run the tests (they are `#[ignore]`d, so a plain `cargo test` skips them):
//!
//!    ```bash
//!    cargo test -p windmill-common --features private,enterprise \
//!      --test aws_sm_integration -- --ignored --nocapture
//!    ```
//!
//! ## Environment variables
//!
//! - `AWS_SM_ENDPOINT`        - LocalStack endpoint (default: http://localhost:4566)
//! - `AWS_SM_REGION`          - AWS region (default: us-east-1)

#[cfg(all(feature = "private", feature = "enterprise"))]
mod tests {
    use windmill_common::secret_backend::{
        test_aws_sm_connection, AwsSecretsManagerBackend, AwsSecretsManagerSettings, SecretBackend,
    };

    fn test_settings() -> AwsSecretsManagerSettings {
        AwsSecretsManagerSettings {
            region: std::env::var("AWS_SM_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            access_key_id: Some("test".to_string()),
            secret_access_key: Some("test".to_string()),
            endpoint_url: Some(
                std::env::var("AWS_SM_ENDPOINT")
                    .unwrap_or_else(|_| "http://localhost:4566".to_string()),
            ),
            prefix: Some("windmill-test/".to_string()),
        }
    }

    #[tokio::test]
    #[ignore = "requires LocalStack secretsmanager at AWS_SM_ENDPOINT"]
    async fn test_connection() {
        let result = test_aws_sm_connection(&test_settings()).await;
        assert!(result.is_ok(), "Connection test failed: {:?}", result.err());
        println!("  ✓ Connection test passed");
    }

    #[tokio::test]
    #[ignore = "requires LocalStack secretsmanager at AWS_SM_ENDPOINT"]
    async fn test_create_and_get_secret() {
        let backend = AwsSecretsManagerBackend::new_with_client(test_settings())
            .await
            .unwrap();

        backend
            .set_secret("test-ws", "u/admin/my_secret", "super-secret-value")
            .await
            .unwrap();
        let value = backend
            .get_secret("test-ws", "u/admin/my_secret")
            .await
            .unwrap();
        assert_eq!(value, "super-secret-value");

        backend
            .delete_secret("test-ws", "u/admin/my_secret")
            .await
            .unwrap();
        println!("  ✓ Create + Get + Delete roundtrip passed");
    }

    #[tokio::test]
    #[ignore = "requires LocalStack secretsmanager at AWS_SM_ENDPOINT"]
    async fn test_update_secret() {
        let backend = AwsSecretsManagerBackend::new_with_client(test_settings())
            .await
            .unwrap();

        backend
            .set_secret("test-ws", "u/admin/update_test", "original")
            .await
            .unwrap();
        backend
            .set_secret("test-ws", "u/admin/update_test", "updated-value")
            .await
            .unwrap();
        let value = backend
            .get_secret("test-ws", "u/admin/update_test")
            .await
            .unwrap();
        assert_eq!(value, "updated-value");

        backend
            .delete_secret("test-ws", "u/admin/update_test")
            .await
            .unwrap();
        println!("  ✓ Update secret passed");
    }

    #[tokio::test]
    #[ignore = "requires LocalStack secretsmanager at AWS_SM_ENDPOINT"]
    async fn test_delete_and_verify_gone() {
        let backend = AwsSecretsManagerBackend::new_with_client(test_settings())
            .await
            .unwrap();

        backend
            .set_secret("test-ws", "u/admin/delete_test", "to-be-deleted")
            .await
            .unwrap();
        backend
            .delete_secret("test-ws", "u/admin/delete_test")
            .await
            .unwrap();

        let result = backend.get_secret("test-ws", "u/admin/delete_test").await;
        assert!(
            matches!(result, Err(windmill_common::error::Error::NotFound(_))),
            "Expected NotFound after delete, got: {:?}",
            result
        );
        println!("  ✓ Delete + verify gone passed");
    }

    #[tokio::test]
    #[ignore = "requires LocalStack secretsmanager at AWS_SM_ENDPOINT"]
    async fn test_delete_nonexistent_is_ok() {
        let backend = AwsSecretsManagerBackend::new_with_client(test_settings())
            .await
            .unwrap();

        let result = backend
            .delete_secret("test-ws", "u/admin/never_existed")
            .await;
        assert!(
            result.is_ok(),
            "Delete of nonexistent should be Ok, got: {:?}",
            result.err()
        );
        println!("  ✓ Delete nonexistent is Ok");
    }

    #[tokio::test]
    #[ignore = "requires LocalStack secretsmanager at AWS_SM_ENDPOINT"]
    async fn test_get_nonexistent_returns_not_found() {
        let backend = AwsSecretsManagerBackend::new_with_client(test_settings())
            .await
            .unwrap();

        let result = backend
            .get_secret("test-ws", "u/admin/does_not_exist")
            .await;
        assert!(
            matches!(result, Err(windmill_common::error::Error::NotFound(_))),
            "Expected NotFound, got: {:?}",
            result
        );
        println!("  ✓ Get nonexistent returns NotFound");
    }

    #[tokio::test]
    #[ignore = "requires LocalStack secretsmanager at AWS_SM_ENDPOINT"]
    async fn test_unicode_and_special_chars() {
        let backend = AwsSecretsManagerBackend::new_with_client(test_settings())
            .await
            .unwrap();

        let secret_value = "p@$$w0rd with unicode: \u{1F512}\u{1F511} and <xml>&\"quotes\"";
        backend
            .set_secret("test-ws", "u/admin/unicode_test", secret_value)
            .await
            .unwrap();
        let retrieved = backend
            .get_secret("test-ws", "u/admin/unicode_test")
            .await
            .unwrap();
        assert_eq!(retrieved, secret_value);

        backend
            .delete_secret("test-ws", "u/admin/unicode_test")
            .await
            .unwrap();
        println!("  ✓ Unicode and special chars passed");
    }

    #[tokio::test]
    #[ignore = "requires LocalStack secretsmanager at AWS_SM_ENDPOINT"]
    async fn test_workspace_isolation() {
        let backend = AwsSecretsManagerBackend::new_with_client(test_settings())
            .await
            .unwrap();

        // Create secrets in different workspaces with the same path
        backend
            .set_secret("workspace-a", "u/admin/shared_name", "value-a")
            .await
            .unwrap();
        backend
            .set_secret("workspace-b", "u/admin/shared_name", "value-b")
            .await
            .unwrap();

        // Each workspace should see its own value
        let a = backend
            .get_secret("workspace-a", "u/admin/shared_name")
            .await
            .unwrap();
        let b = backend
            .get_secret("workspace-b", "u/admin/shared_name")
            .await
            .unwrap();
        assert_eq!(a, "value-a");
        assert_eq!(b, "value-b");

        // Cleanup
        backend
            .delete_secret("workspace-a", "u/admin/shared_name")
            .await
            .unwrap();
        backend
            .delete_secret("workspace-b", "u/admin/shared_name")
            .await
            .unwrap();
        println!("  ✓ Workspace isolation passed");
    }

    #[tokio::test]
    #[ignore = "requires LocalStack secretsmanager at AWS_SM_ENDPOINT"]
    async fn test_backend_name() {
        let backend = AwsSecretsManagerBackend::new_with_client(test_settings())
            .await
            .unwrap();
        assert_eq!(backend.backend_name(), "aws_secrets_manager");
        println!("  ✓ Backend name is correct");
    }
}

#[cfg(not(all(feature = "private", feature = "enterprise")))]
mod tests {
    #[test]
    fn test_aws_sm_requires_enterprise() {
        println!("AWS Secrets Manager integration tests require Enterprise Edition features");
        println!("Run with: cargo test -p windmill-common --features private,enterprise");
    }
}
