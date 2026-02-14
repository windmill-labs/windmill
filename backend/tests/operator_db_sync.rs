/*!
 * Integration tests for windmill-operator db_sync module.
 *
 * Tests verify full declarative sync of global_settings and worker configs:
 * - Upsert desired settings into DB
 * - Delete settings present in DB but absent from desired state
 * - Protect certain internal settings from deletion
 */

#[cfg(feature = "operator")]
mod tests {
    use std::collections::BTreeMap;

    use sqlx::{Pool, Postgres};

    // ========================================================================
    // Helpers
    // ========================================================================

    async fn get_global_setting(db: &Pool<Postgres>, name: &str) -> Option<serde_json::Value> {
        sqlx::query_as::<_, (serde_json::Value,)>(
            "SELECT value FROM global_settings WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(db)
        .await
        .expect("query should succeed")
        .map(|(v,)| v)
    }

    async fn get_config(db: &Pool<Postgres>, name: &str) -> Option<serde_json::Value> {
        sqlx::query_as::<_, (serde_json::Value,)>("SELECT config FROM config WHERE name = $1")
            .bind(name)
            .fetch_optional(db)
            .await
            .expect("query should succeed")
            .map(|(v,)| v)
    }

    async fn insert_global_setting(db: &Pool<Postgres>, name: &str, value: serde_json::Value) {
        sqlx::query(
            "INSERT INTO global_settings (name, value) VALUES ($1, $2) \
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(name)
        .bind(&value)
        .execute(db)
        .await
        .expect("insert should succeed");
    }

    async fn insert_config(db: &Pool<Postgres>, name: &str, config: serde_json::Value) {
        sqlx::query(
            "INSERT INTO config (name, config) VALUES ($1, $2) \
             ON CONFLICT (name) DO UPDATE SET config = EXCLUDED.config",
        )
        .bind(name)
        .bind(&config)
        .execute(db)
        .await
        .expect("insert should succeed");
    }

    // ========================================================================
    // sync_global_settings tests
    // ========================================================================

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_global_settings_upserts(db: Pool<Postgres>) {
        let mut desired = BTreeMap::new();
        desired.insert(
            "test_op_setting_a".to_string(),
            serde_json::json!("value_a"),
        );
        desired.insert("test_op_setting_b".to_string(), serde_json::json!(42));

        windmill_operator::db_sync::sync_global_settings(&db, &desired)
            .await
            .expect("sync should succeed");

        assert_eq!(
            get_global_setting(&db, "test_op_setting_a").await,
            Some(serde_json::json!("value_a"))
        );
        assert_eq!(
            get_global_setting(&db, "test_op_setting_b").await,
            Some(serde_json::json!(42))
        );
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_global_settings_updates_existing(db: Pool<Postgres>) {
        // Pre-populate a setting
        insert_global_setting(&db, "test_op_existing", serde_json::json!("old")).await;

        // Sync with new value
        let mut desired = BTreeMap::new();
        desired.insert("test_op_existing".to_string(), serde_json::json!("new"));

        windmill_operator::db_sync::sync_global_settings(&db, &desired)
            .await
            .expect("sync should succeed");

        assert_eq!(
            get_global_setting(&db, "test_op_existing").await,
            Some(serde_json::json!("new"))
        );
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_global_settings_deletes_absent(db: Pool<Postgres>) {
        // Pre-populate settings
        insert_global_setting(&db, "test_op_keep", serde_json::json!("keep")).await;
        insert_global_setting(&db, "test_op_remove", serde_json::json!("remove")).await;

        // Sync with only one of them
        let mut desired = BTreeMap::new();
        desired.insert("test_op_keep".to_string(), serde_json::json!("keep"));

        windmill_operator::db_sync::sync_global_settings(&db, &desired)
            .await
            .expect("sync should succeed");

        assert!(get_global_setting(&db, "test_op_keep").await.is_some());
        assert!(
            get_global_setting(&db, "test_op_remove").await.is_none(),
            "Setting absent from desired should be deleted"
        );
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_global_settings_protects_ducklake(db: Pool<Postgres>) {
        // Pre-populate a protected setting
        insert_global_setting(
            &db,
            "ducklake_settings",
            serde_json::json!({"protected": true}),
        )
        .await;

        // Sync with empty desired — protected key should survive
        let desired = BTreeMap::new();

        windmill_operator::db_sync::sync_global_settings(&db, &desired)
            .await
            .expect("sync should succeed");

        assert!(
            get_global_setting(&db, "ducklake_settings").await.is_some(),
            "Protected setting ducklake_settings should not be deleted"
        );
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_global_settings_protects_all_protected_keys(db: Pool<Postgres>) {
        let protected_keys = [
            "ducklake_user_pg_pwd",
            "ducklake_settings",
            "custom_instance_pg_databases",
        ];

        for key in &protected_keys {
            insert_global_setting(&db, key, serde_json::json!("protected_value")).await;
        }

        // Sync with empty desired
        let desired = BTreeMap::new();
        windmill_operator::db_sync::sync_global_settings(&db, &desired)
            .await
            .expect("sync should succeed");

        for key in &protected_keys {
            assert!(
                get_global_setting(&db, key).await.is_some(),
                "Protected key {key} should not be deleted"
            );
        }
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_global_settings_empty_desired(db: Pool<Postgres>) {
        // Pre-populate a non-protected setting
        insert_global_setting(&db, "test_op_ephemeral", serde_json::json!("gone")).await;

        let desired = BTreeMap::new();
        windmill_operator::db_sync::sync_global_settings(&db, &desired)
            .await
            .expect("sync should succeed");

        assert!(
            get_global_setting(&db, "test_op_ephemeral").await.is_none(),
            "Non-protected settings should be deleted when desired is empty"
        );
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_global_settings_complex_json_values(db: Pool<Postgres>) {
        let mut desired = BTreeMap::new();
        desired.insert(
            "test_op_complex".to_string(),
            serde_json::json!({
                "host": "smtp.example.com",
                "port": 587,
                "tls": true,
                "nested": {"array": [1, 2, 3]}
            }),
        );

        windmill_operator::db_sync::sync_global_settings(&db, &desired)
            .await
            .expect("sync should succeed");

        let stored = get_global_setting(&db, "test_op_complex")
            .await
            .expect("Setting should exist");
        assert_eq!(stored["host"], "smtp.example.com");
        assert_eq!(stored["port"], 587);
        assert_eq!(stored["nested"]["array"][1], 2);
    }

    // ========================================================================
    // sync_worker_configs tests
    // ========================================================================

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_worker_configs_upserts_with_prefix(db: Pool<Postgres>) {
        let mut desired = BTreeMap::new();
        desired.insert(
            "test_group".to_string(),
            serde_json::json!({"dedicated_worker": false}),
        );

        windmill_operator::db_sync::sync_worker_configs(&db, &desired)
            .await
            .expect("sync should succeed");

        let config = get_config(&db, "worker__test_group")
            .await
            .expect("Config should exist with worker__ prefix");
        assert_eq!(config["dedicated_worker"], false);
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_worker_configs_updates_existing(db: Pool<Postgres>) {
        // Pre-populate
        insert_config(
            &db,
            "worker__test_wc_existing",
            serde_json::json!({"old": true}),
        )
        .await;

        let mut desired = BTreeMap::new();
        desired.insert(
            "test_wc_existing".to_string(),
            serde_json::json!({"new": true}),
        );

        windmill_operator::db_sync::sync_worker_configs(&db, &desired)
            .await
            .expect("sync should succeed");

        let config = get_config(&db, "worker__test_wc_existing")
            .await
            .expect("Config should exist");
        assert_eq!(config["new"], true);
        assert!(config.get("old").is_none());
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_worker_configs_deletes_absent(db: Pool<Postgres>) {
        // Pre-populate two worker configs
        insert_config(
            &db,
            "worker__test_wc_keep",
            serde_json::json!({"keep": true}),
        )
        .await;
        insert_config(
            &db,
            "worker__test_wc_remove",
            serde_json::json!({"remove": true}),
        )
        .await;

        // Sync with only one
        let mut desired = BTreeMap::new();
        desired.insert(
            "test_wc_keep".to_string(),
            serde_json::json!({"keep": true}),
        );

        windmill_operator::db_sync::sync_worker_configs(&db, &desired)
            .await
            .expect("sync should succeed");

        assert!(get_config(&db, "worker__test_wc_keep").await.is_some());
        assert!(
            get_config(&db, "worker__test_wc_remove").await.is_none(),
            "Worker config absent from desired should be deleted"
        );
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_worker_configs_does_not_touch_non_worker_configs(db: Pool<Postgres>) {
        // Insert a non-worker config (no worker__ prefix)
        insert_config(&db, "server_config", serde_json::json!({"important": true})).await;

        // Sync with empty worker configs
        let desired = BTreeMap::new();
        windmill_operator::db_sync::sync_worker_configs(&db, &desired)
            .await
            .expect("sync should succeed");

        assert!(
            get_config(&db, "server_config").await.is_some(),
            "Non-worker configs should not be touched"
        );
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_worker_configs_multiple_groups(db: Pool<Postgres>) {
        let mut desired = BTreeMap::new();
        desired.insert(
            "default".to_string(),
            serde_json::json!({"init_bash": "echo default"}),
        );
        desired.insert(
            "gpu".to_string(),
            serde_json::json!({"dedicated_worker": true}),
        );
        desired.insert(
            "native".to_string(),
            serde_json::json!({"init_bash": "echo native"}),
        );

        windmill_operator::db_sync::sync_worker_configs(&db, &desired)
            .await
            .expect("sync should succeed");

        assert!(get_config(&db, "worker__default").await.is_some());
        assert!(get_config(&db, "worker__gpu").await.is_some());
        assert!(get_config(&db, "worker__native").await.is_some());
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_sync_worker_configs_empty_desired(db: Pool<Postgres>) {
        insert_config(
            &db,
            "worker__test_wc_gone",
            serde_json::json!({"ephemeral": true}),
        )
        .await;

        let desired = BTreeMap::new();
        windmill_operator::db_sync::sync_worker_configs(&db, &desired)
            .await
            .expect("sync should succeed");

        assert!(
            get_config(&db, "worker__test_wc_gone").await.is_none(),
            "Worker config should be deleted when desired is empty"
        );
    }

    // ========================================================================
    // End-to-end: both syncs together
    // ========================================================================

    #[sqlx::test(fixtures("base"))]
    async fn test_full_declarative_sync(db: Pool<Postgres>) {
        // Pre-populate some existing state
        insert_global_setting(&db, "test_op_stale_setting", serde_json::json!("stale")).await;
        insert_config(
            &db,
            "worker__test_stale_group",
            serde_json::json!({"stale": true}),
        )
        .await;

        // Define desired state
        let mut global_settings = BTreeMap::new();
        global_settings.insert(
            "test_op_base_url".to_string(),
            serde_json::json!("https://windmill.example.com"),
        );
        global_settings.insert(
            "test_op_license_key".to_string(),
            serde_json::json!("my-license"),
        );

        let mut worker_configs = BTreeMap::new();
        worker_configs.insert(
            "default".to_string(),
            serde_json::json!({"init_bash": "echo hello"}),
        );

        // Sync both
        windmill_operator::db_sync::sync_global_settings(&db, &global_settings)
            .await
            .expect("global sync should succeed");
        windmill_operator::db_sync::sync_worker_configs(&db, &worker_configs)
            .await
            .expect("worker sync should succeed");

        // Verify desired state is present
        assert_eq!(
            get_global_setting(&db, "test_op_base_url").await,
            Some(serde_json::json!("https://windmill.example.com"))
        );
        assert_eq!(
            get_global_setting(&db, "test_op_license_key").await,
            Some(serde_json::json!("my-license"))
        );
        assert!(get_config(&db, "worker__default").await.is_some());

        // Verify stale state is removed
        assert!(
            get_global_setting(&db, "test_op_stale_setting")
                .await
                .is_none(),
            "Stale global setting should be removed"
        );
        assert!(
            get_config(&db, "worker__test_stale_group").await.is_none(),
            "Stale worker config should be removed"
        );
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_idempotent_sync(db: Pool<Postgres>) {
        let mut desired = BTreeMap::new();
        desired.insert("test_op_idempotent".to_string(), serde_json::json!("value"));

        // Run sync twice — should be idempotent
        windmill_operator::db_sync::sync_global_settings(&db, &desired)
            .await
            .expect("first sync should succeed");
        windmill_operator::db_sync::sync_global_settings(&db, &desired)
            .await
            .expect("second sync should succeed");

        assert_eq!(
            get_global_setting(&db, "test_op_idempotent").await,
            Some(serde_json::json!("value"))
        );
    }
}
