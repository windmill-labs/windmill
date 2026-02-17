/*!
 * Integration tests for windmill-common instance_config module.
 *
 * Tests verify the DB-level operations:
 * - `InstanceConfig::from_db()` reads global_settings + worker configs
 * - `apply_settings_diff()` applies upserts and deletes to global_settings
 * - `apply_configs_diff()` applies upserts and deletes to config table
 * - Full roundtrip: write → read → modify → diff → apply → read → verify
 *
 * Note: the test DB is created from migrations which seed default settings
 * and worker configs. Tests either clean up first or assert on specific keys
 * rather than exact counts.
 */

use std::collections::BTreeMap;

use sqlx::{Pool, Postgres};
use windmill_common::instance_config::{
    apply_configs_diff, apply_settings_diff, diff_global_settings, diff_worker_configs, ApplyMode,
    ConfigsDiff, InstanceConfig, SettingsDiff,
};

// ========================================================================
// Helpers
// ========================================================================

async fn get_global_setting(db: &Pool<Postgres>, name: &str) -> Option<serde_json::Value> {
    sqlx::query_as::<_, (serde_json::Value,)>("SELECT value FROM global_settings WHERE name = $1")
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

async fn count_global_settings(db: &Pool<Postgres>) -> i64 {
    sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM global_settings")
        .fetch_one(db)
        .await
        .expect("count query should succeed")
        .0
}

/// Clear all migration-seeded data so tests start from a clean slate.
async fn clear_settings_and_configs(db: &Pool<Postgres>) {
    sqlx::query("DELETE FROM global_settings")
        .execute(db)
        .await
        .expect("clear global_settings should succeed");
    sqlx::query("DELETE FROM config WHERE name LIKE 'worker__%'")
        .execute(db)
        .await
        .expect("clear worker configs should succeed");
}

// ========================================================================
// InstanceConfig::from_db() tests
// ========================================================================

#[sqlx::test(fixtures("base"))]
async fn test_from_db_empty(db: Pool<Postgres>) {
    clear_settings_and_configs(&db).await;

    let config = InstanceConfig::from_db(&db)
        .await
        .expect("from_db should succeed on empty DB");
    assert!(config.global_settings.base_url.is_none());
    assert!(config.global_settings.license_key.is_none());
    assert!(config.worker_configs.is_empty());
}

#[sqlx::test(fixtures("base"))]
async fn test_from_db_with_typed_global_settings(db: Pool<Postgres>) {
    insert_global_setting(&db, "base_url", serde_json::json!("https://windmill.test")).await;
    insert_global_setting(&db, "retention_period_secs", serde_json::json!(86400)).await;
    insert_global_setting(&db, "expose_metrics", serde_json::json!(true)).await;

    let config = InstanceConfig::from_db(&db).await.unwrap();
    assert_eq!(
        config.global_settings.base_url.as_deref(),
        Some("https://windmill.test")
    );
    assert_eq!(config.global_settings.retention_period_secs, Some(86400));
    assert_eq!(config.global_settings.expose_metrics, Some(true));
}

#[sqlx::test(fixtures("base"))]
async fn test_from_db_with_structured_settings(db: Pool<Postgres>) {
    insert_global_setting(
        &db,
        "smtp_settings",
        serde_json::json!({
            "smtp_host": "mail.test.com",
            "smtp_port": 587,
            "smtp_tls_implicit": false
        }),
    )
    .await;
    insert_global_setting(
        &db,
        "otel",
        serde_json::json!({
            "metrics_enabled": true,
            "logs_enabled": false,
            "otel_exporter_otlp_endpoint": "http://otel:4317"
        }),
    )
    .await;

    let config = InstanceConfig::from_db(&db).await.unwrap();

    let smtp = config.global_settings.smtp_settings.as_ref().unwrap();
    assert_eq!(smtp.smtp_host.as_deref(), Some("mail.test.com"));
    assert_eq!(smtp.smtp_port, Some(587));
    assert_eq!(smtp.smtp_tls_implicit, Some(false));

    let otel = config.global_settings.otel.as_ref().unwrap();
    assert_eq!(otel.metrics_enabled, Some(true));
    assert_eq!(otel.logs_enabled, Some(false));
    assert_eq!(
        otel.otel_exporter_otlp_endpoint.as_deref(),
        Some("http://otel:4317")
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_from_db_unknown_settings_go_to_extra(db: Pool<Postgres>) {
    insert_global_setting(&db, "future_setting_xyz", serde_json::json!({"nested": 42})).await;
    insert_global_setting(&db, "another_unknown", serde_json::json!("hello")).await;

    let config = InstanceConfig::from_db(&db).await.unwrap();
    assert_eq!(
        config.global_settings.extra["future_setting_xyz"],
        serde_json::json!({"nested": 42})
    );
    assert_eq!(
        config.global_settings.extra["another_unknown"],
        serde_json::json!("hello")
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_from_db_with_worker_configs(db: Pool<Postgres>) {
    clear_settings_and_configs(&db).await;

    insert_config(
        &db,
        "worker__default",
        serde_json::json!({"init_bash": "echo default"}),
    )
    .await;
    insert_config(
        &db,
        "worker__gpu",
        serde_json::json!({
            "dedicated_worker": "ws:f/gpu_script",
            "worker_tags": ["gpu", "cuda"]
        }),
    )
    .await;

    let config = InstanceConfig::from_db(&db).await.unwrap();
    assert_eq!(config.worker_configs.len(), 2);
    assert_eq!(
        config.worker_configs["default"].init_bash.as_deref(),
        Some("echo default")
    );
    assert_eq!(
        config.worker_configs["gpu"].dedicated_worker.as_deref(),
        Some("ws:f/gpu_script")
    );
    assert_eq!(
        config.worker_configs["gpu"].worker_tags.as_ref().unwrap(),
        &["gpu", "cuda"]
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_from_db_ignores_non_worker_configs(db: Pool<Postgres>) {
    clear_settings_and_configs(&db).await;

    // Insert a config without worker__ prefix — should not appear
    insert_config(&db, "server_config", serde_json::json!({"important": true})).await;
    insert_config(
        &db,
        "worker__actual",
        serde_json::json!({"init_bash": "echo hi"}),
    )
    .await;

    let config = InstanceConfig::from_db(&db).await.unwrap();
    assert_eq!(config.worker_configs.len(), 1);
    assert!(config.worker_configs.contains_key("actual"));
}

#[sqlx::test(fixtures("base"))]
async fn test_from_db_worker_config_prefix_stripping(db: Pool<Postgres>) {
    insert_config(
        &db,
        "worker__my_group_name",
        serde_json::json!({"cache_clear": 5}),
    )
    .await;

    let config = InstanceConfig::from_db(&db).await.unwrap();
    assert!(
        config.worker_configs.contains_key("my_group_name"),
        "worker__ prefix should be stripped"
    );
    assert_eq!(config.worker_configs["my_group_name"].extra["cache_clear"], serde_json::json!(5));
}

#[sqlx::test(fixtures("base"))]
async fn test_from_db_worker_config_unknown_fields_in_extra(db: Pool<Postgres>) {
    insert_config(
        &db,
        "worker__test_extra",
        serde_json::json!({
            "init_bash": "echo hello",
            "future_field": 999,
            "another_future": {"nested": true}
        }),
    )
    .await;

    let config = InstanceConfig::from_db(&db).await.unwrap();
    let wc = &config.worker_configs["test_extra"];
    assert_eq!(wc.init_bash.as_deref(), Some("echo hello"));
    assert_eq!(wc.extra["future_field"], serde_json::json!(999));
    assert_eq!(
        wc.extra["another_future"],
        serde_json::json!({"nested": true})
    );
}

// ========================================================================
// apply_settings_diff() tests
// ========================================================================

#[sqlx::test(fixtures("base"))]
async fn test_apply_settings_diff_upserts_only(db: Pool<Postgres>) {
    let diff = SettingsDiff {
        upserts: {
            let mut m = BTreeMap::new();
            m.insert("key_a".to_string(), serde_json::json!("val_a"));
            m.insert("key_b".to_string(), serde_json::json!(123));
            m
        },
        deletes: vec![],
        ..Default::default()
    };

    apply_settings_diff(&db, &diff).await.unwrap();

    assert_eq!(
        get_global_setting(&db, "key_a").await,
        Some(serde_json::json!("val_a"))
    );
    assert_eq!(
        get_global_setting(&db, "key_b").await,
        Some(serde_json::json!(123))
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_apply_settings_diff_deletes_only(db: Pool<Postgres>) {
    insert_global_setting(&db, "to_delete_1", serde_json::json!("bye")).await;
    insert_global_setting(&db, "to_delete_2", serde_json::json!("gone")).await;
    insert_global_setting(&db, "to_keep", serde_json::json!("stay")).await;

    let diff = SettingsDiff {
        upserts: BTreeMap::new(),
        deletes: vec!["to_delete_1".to_string(), "to_delete_2".to_string()],
        ..Default::default()
    };

    apply_settings_diff(&db, &diff).await.unwrap();

    assert!(get_global_setting(&db, "to_delete_1").await.is_none());
    assert!(get_global_setting(&db, "to_delete_2").await.is_none());
    assert!(get_global_setting(&db, "to_keep").await.is_some());
}

#[sqlx::test(fixtures("base"))]
async fn test_apply_settings_diff_upserts_and_deletes(db: Pool<Postgres>) {
    insert_global_setting(&db, "old_key", serde_json::json!("old")).await;

    let diff = SettingsDiff {
        upserts: {
            let mut m = BTreeMap::new();
            m.insert("new_key".to_string(), serde_json::json!("new"));
            m
        },
        deletes: vec!["old_key".to_string()],
        ..Default::default()
    };

    apply_settings_diff(&db, &diff).await.unwrap();

    assert!(get_global_setting(&db, "old_key").await.is_none());
    assert_eq!(
        get_global_setting(&db, "new_key").await,
        Some(serde_json::json!("new"))
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_apply_settings_diff_empty_noop(db: Pool<Postgres>) {
    insert_global_setting(&db, "preexisting", serde_json::json!("value")).await;

    let diff = SettingsDiff::default();
    apply_settings_diff(&db, &diff).await.unwrap();

    assert_eq!(
        get_global_setting(&db, "preexisting").await,
        Some(serde_json::json!("value"))
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_apply_settings_diff_upsert_overwrites(db: Pool<Postgres>) {
    insert_global_setting(&db, "overwrite_me", serde_json::json!("old_value")).await;

    let diff = SettingsDiff {
        upserts: {
            let mut m = BTreeMap::new();
            m.insert("overwrite_me".to_string(), serde_json::json!("new_value"));
            m
        },
        deletes: vec![],
        ..Default::default()
    };

    apply_settings_diff(&db, &diff).await.unwrap();

    assert_eq!(
        get_global_setting(&db, "overwrite_me").await,
        Some(serde_json::json!("new_value"))
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_apply_settings_diff_complex_json(db: Pool<Postgres>) {
    let complex_value = serde_json::json!({
        "host": "smtp.example.com",
        "port": 587,
        "auth": {"user": "admin", "pass": "secret"},
        "tags": [1, 2, 3],
        "nested": {"deep": {"value": null}}
    });

    let diff = SettingsDiff {
        upserts: {
            let mut m = BTreeMap::new();
            m.insert("complex_setting".to_string(), complex_value.clone());
            m
        },
        deletes: vec![],
        ..Default::default()
    };

    apply_settings_diff(&db, &diff).await.unwrap();

    let stored = get_global_setting(&db, "complex_setting").await.unwrap();
    assert_eq!(stored, complex_value);
}

#[sqlx::test(fixtures("base"))]
async fn test_apply_settings_diff_delete_nonexistent_is_noop(db: Pool<Postgres>) {
    let diff =
        SettingsDiff {
            upserts: BTreeMap::new(),
            deletes: vec!["does_not_exist".to_string()],
            ..Default::default()
        };

    // Should not error
    apply_settings_diff(&db, &diff).await.unwrap();
}

// ========================================================================
// apply_configs_diff() tests
// ========================================================================

#[sqlx::test(fixtures("base"))]
async fn test_apply_configs_diff_upserts_with_prefix(db: Pool<Postgres>) {
    let diff = ConfigsDiff {
        upserts: {
            let mut m = BTreeMap::new();
            m.insert(
                "mygroup".to_string(),
                serde_json::json!({"init_bash": "echo hi"}),
            );
            m
        },
        deletes: vec![],
    };

    apply_configs_diff(&db, &diff).await.unwrap();

    let stored = get_config(&db, "worker__mygroup").await.unwrap();
    assert_eq!(stored["init_bash"], "echo hi");
}

#[sqlx::test(fixtures("base"))]
async fn test_apply_configs_diff_deletes_with_prefix(db: Pool<Postgres>) {
    insert_config(&db, "worker__to_remove", serde_json::json!({"a": 1})).await;

    let diff = ConfigsDiff { upserts: BTreeMap::new(), deletes: vec!["to_remove".to_string()] };

    apply_configs_diff(&db, &diff).await.unwrap();

    assert!(get_config(&db, "worker__to_remove").await.is_none());
}

#[sqlx::test(fixtures("base"))]
async fn test_apply_configs_diff_upsert_overwrites(db: Pool<Postgres>) {
    insert_config(&db, "worker__grp", serde_json::json!({"old": true})).await;

    let diff = ConfigsDiff {
        upserts: {
            let mut m = BTreeMap::new();
            m.insert("grp".to_string(), serde_json::json!({"new": true}));
            m
        },
        deletes: vec![],
    };

    apply_configs_diff(&db, &diff).await.unwrap();

    let stored = get_config(&db, "worker__grp").await.unwrap();
    assert_eq!(stored, serde_json::json!({"new": true}));
}

#[sqlx::test(fixtures("base"))]
async fn test_apply_configs_diff_empty_noop(db: Pool<Postgres>) {
    insert_config(&db, "worker__keep", serde_json::json!({"keep": true})).await;

    let diff = ConfigsDiff::default();
    apply_configs_diff(&db, &diff).await.unwrap();

    assert!(get_config(&db, "worker__keep").await.is_some());
}

#[sqlx::test(fixtures("base"))]
async fn test_apply_configs_diff_does_not_touch_non_worker_configs(db: Pool<Postgres>) {
    insert_config(&db, "server_config", serde_json::json!({"x": 1})).await;

    let diff = ConfigsDiff { upserts: BTreeMap::new(), deletes: vec!["server_config".to_string()] };

    apply_configs_diff(&db, &diff).await.unwrap();

    // The delete targets "worker__server_config", not "server_config"
    assert!(
        get_config(&db, "server_config").await.is_some(),
        "Non-worker config should not be affected"
    );
}

// ========================================================================
// Full roundtrip tests
// ========================================================================

#[sqlx::test(fixtures("base"))]
async fn test_roundtrip_write_read_modify_apply(db: Pool<Postgres>) {
    clear_settings_and_configs(&db).await;

    // Step 1: Seed initial state
    insert_global_setting(&db, "base_url", serde_json::json!("https://v1.test")).await;
    insert_global_setting(&db, "retention_period_secs", serde_json::json!(3600)).await;
    insert_config(
        &db,
        "worker__default",
        serde_json::json!({"init_bash": "echo v1"}),
    )
    .await;

    // Step 2: Read via from_db
    let config = InstanceConfig::from_db(&db).await.unwrap();
    assert_eq!(
        config.global_settings.base_url.as_deref(),
        Some("https://v1.test")
    );
    assert_eq!(config.global_settings.retention_period_secs, Some(3600));
    assert_eq!(config.worker_configs.len(), 1);

    // Step 3: Modify — change base_url, add a new setting, remove retention
    let mut desired_settings = config.global_settings.clone();
    desired_settings.base_url = Some("https://v2.test".to_string());
    desired_settings.expose_metrics = Some(true);
    desired_settings.retention_period_secs = None;

    let current_map = config.global_settings.to_settings_map();
    let desired_map = desired_settings.to_settings_map();

    // Step 4: Diff + apply (Merge mode — no deletes)
    let diff = diff_global_settings(&current_map, &desired_map, ApplyMode::Merge);
    assert!(diff.upserts.contains_key("base_url"));
    assert!(diff.upserts.contains_key("expose_metrics"));
    assert!(diff.deletes.is_empty());

    apply_settings_diff(&db, &diff).await.unwrap();

    // Step 5: Read back and verify
    let config2 = InstanceConfig::from_db(&db).await.unwrap();
    assert_eq!(
        config2.global_settings.base_url.as_deref(),
        Some("https://v2.test")
    );
    assert_eq!(config2.global_settings.expose_metrics, Some(true));
    // retention_period_secs still in DB because Merge mode doesn't delete
    assert_eq!(config2.global_settings.retention_period_secs, Some(3600));
}

#[sqlx::test(fixtures("base"))]
async fn test_roundtrip_replace_mode_deletes(db: Pool<Postgres>) {
    clear_settings_and_configs(&db).await;

    // Seed
    insert_global_setting(&db, "base_url", serde_json::json!("https://old.test")).await;
    insert_global_setting(&db, "retention_period_secs", serde_json::json!(7200)).await;
    insert_global_setting(&db, "expose_metrics", serde_json::json!(false)).await;

    let config = InstanceConfig::from_db(&db).await.unwrap();
    let current_map = config.global_settings.to_settings_map();

    // Desired: only base_url — retention and expose_metrics should be deleted
    let mut desired_map = BTreeMap::new();
    desired_map.insert(
        "base_url".to_string(),
        serde_json::json!("https://old.test"),
    );

    let diff = diff_global_settings(&current_map, &desired_map, ApplyMode::Replace);
    assert!(diff.upserts.is_empty());
    assert!(diff.deletes.contains(&"retention_period_secs".to_string()));
    assert!(diff.deletes.contains(&"expose_metrics".to_string()));

    apply_settings_diff(&db, &diff).await.unwrap();

    assert!(get_global_setting(&db, "retention_period_secs")
        .await
        .is_none());
    assert!(get_global_setting(&db, "expose_metrics").await.is_none());
    assert!(get_global_setting(&db, "base_url").await.is_some());
}

#[sqlx::test(fixtures("base"))]
async fn test_roundtrip_worker_configs(db: Pool<Postgres>) {
    clear_settings_and_configs(&db).await;

    // Seed two worker configs
    insert_config(
        &db,
        "worker__default",
        serde_json::json!({"init_bash": "echo default"}),
    )
    .await;
    insert_config(
        &db,
        "worker__legacy",
        serde_json::json!({"init_bash": "echo legacy"}),
    )
    .await;

    // Read
    let config = InstanceConfig::from_db(&db).await.unwrap();
    assert_eq!(config.worker_configs.len(), 2);

    // Desired: replace default, add gpu, remove legacy
    let current_map: BTreeMap<String, serde_json::Value> = config
        .worker_configs
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::to_value(v).unwrap()))
        .collect();

    let mut desired_map = BTreeMap::new();
    desired_map.insert(
        "default".to_string(),
        serde_json::json!({"init_bash": "echo default v2"}),
    );
    desired_map.insert(
        "gpu".to_string(),
        serde_json::json!({"dedicated_worker": "ws:f/gpu"}),
    );

    let diff = diff_worker_configs(&current_map, &desired_map, ApplyMode::Replace);
    assert!(diff.upserts.contains_key("default")); // changed
    assert!(diff.upserts.contains_key("gpu")); // new
    assert_eq!(diff.deletes, vec!["legacy".to_string()]);

    apply_configs_diff(&db, &diff).await.unwrap();

    // Verify
    let config2 = InstanceConfig::from_db(&db).await.unwrap();
    assert_eq!(config2.worker_configs.len(), 2);
    assert_eq!(
        config2.worker_configs["default"].init_bash.as_deref(),
        Some("echo default v2")
    );
    assert_eq!(
        config2.worker_configs["gpu"].dedicated_worker.as_deref(),
        Some("ws:f/gpu")
    );
    assert!(!config2.worker_configs.contains_key("legacy"));
}

#[sqlx::test(fixtures("base"))]
async fn test_roundtrip_to_settings_map_from_db_consistency(db: Pool<Postgres>) {
    clear_settings_and_configs(&db).await;

    // Write a GlobalSettings via to_settings_map + apply, then read back via from_db
    let original = windmill_common::instance_config::GlobalSettings {
        base_url: Some("https://roundtrip.test".to_string()),
        retention_period_secs: Some(43200),
        expose_metrics: Some(true),
        smtp_settings: Some(windmill_common::instance_config::SmtpSettings {
            smtp_host: Some("smtp.roundtrip.test".to_string()),
            smtp_port: Some(465),
            ..Default::default()
        }),
        custom_tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
        ..Default::default()
    };

    let map = original.to_settings_map();
    let diff = SettingsDiff {
        upserts: map.into_iter().collect(),
        deletes: vec![],
        ..Default::default()
    };

    apply_settings_diff(&db, &diff).await.unwrap();

    let config = InstanceConfig::from_db(&db).await.unwrap();
    assert_eq!(config.global_settings.base_url, original.base_url);
    assert_eq!(
        config.global_settings.retention_period_secs,
        original.retention_period_secs
    );
    assert_eq!(
        config.global_settings.expose_metrics,
        original.expose_metrics
    );
    assert_eq!(
        config
            .global_settings
            .smtp_settings
            .as_ref()
            .unwrap()
            .smtp_host,
        original.smtp_settings.as_ref().unwrap().smtp_host
    );
    assert_eq!(
        config
            .global_settings
            .smtp_settings
            .as_ref()
            .unwrap()
            .smtp_port,
        original.smtp_settings.as_ref().unwrap().smtp_port
    );
    assert_eq!(config.global_settings.custom_tags, original.custom_tags);
}

#[sqlx::test(fixtures("base"))]
async fn test_idempotent_apply(db: Pool<Postgres>) {
    clear_settings_and_configs(&db).await;

    let diff = SettingsDiff {
        upserts: {
            let mut m = BTreeMap::new();
            m.insert("idem_key".to_string(), serde_json::json!("idem_value"));
            m
        },
        deletes: vec![],
        ..Default::default()
    };

    // Apply twice
    apply_settings_diff(&db, &diff).await.unwrap();
    apply_settings_diff(&db, &diff).await.unwrap();

    assert_eq!(
        get_global_setting(&db, "idem_key").await,
        Some(serde_json::json!("idem_value"))
    );
    assert_eq!(count_global_settings(&db).await, 1);
}

#[sqlx::test(fixtures("base"))]
async fn test_from_db_mixed_typed_and_extra(db: Pool<Postgres>) {
    clear_settings_and_configs(&db).await;

    // Insert both typed and untyped settings
    insert_global_setting(&db, "base_url", serde_json::json!("https://mixed.test")).await;
    insert_global_setting(&db, "expose_metrics", serde_json::json!(true)).await;
    insert_global_setting(
        &db,
        "unknown_future_setting",
        serde_json::json!({"key": "val"}),
    )
    .await;
    insert_global_setting(&db, "another_custom", serde_json::json!([1, 2, 3])).await;

    let config = InstanceConfig::from_db(&db).await.unwrap();

    // Typed fields
    assert_eq!(
        config.global_settings.base_url.as_deref(),
        Some("https://mixed.test")
    );
    assert_eq!(config.global_settings.expose_metrics, Some(true));

    // Extra fields
    assert_eq!(
        config.global_settings.extra["unknown_future_setting"],
        serde_json::json!({"key": "val"})
    );
    assert_eq!(
        config.global_settings.extra["another_custom"],
        serde_json::json!([1, 2, 3])
    );

    // Roundtrip: to_settings_map should include everything
    let map = config.global_settings.to_settings_map();
    assert!(map.contains_key("base_url"));
    assert!(map.contains_key("expose_metrics"));
    assert!(map.contains_key("unknown_future_setting"));
    assert!(map.contains_key("another_custom"));
    assert_eq!(map.len(), 4);
}

#[sqlx::test(fixtures("base"))]
async fn test_full_config_roundtrip(db: Pool<Postgres>) {
    clear_settings_and_configs(&db).await;

    // Seed a realistic configuration
    insert_global_setting(
        &db,
        "base_url",
        serde_json::json!("https://prod.windmill.dev"),
    )
    .await;
    insert_global_setting(&db, "license_key", serde_json::json!("prod-license-key")).await;
    insert_global_setting(&db, "retention_period_secs", serde_json::json!(2592000)).await;
    insert_global_setting(
        &db,
        "smtp_settings",
        serde_json::json!({
            "smtp_host": "smtp.prod.com",
            "smtp_port": 587,
            "smtp_from": "noreply@prod.com",
            "smtp_tls_implicit": true
        }),
    )
    .await;
    insert_global_setting(
        &db,
        "critical_error_channels",
        serde_json::json!([
            {"email": "admin@prod.com"},
            {"slack_channel": "#prod-alerts"}
        ]),
    )
    .await;
    insert_global_setting(
        &db,
        "otel",
        serde_json::json!({
            "metrics_enabled": true,
            "tracing_enabled": true,
            "otel_exporter_otlp_endpoint": "http://otel-collector:4317"
        }),
    )
    .await;

    insert_config(
        &db,
        "worker__default",
        serde_json::json!({
            "init_bash": "apt-get update",
            "worker_tags": ["default", "deno", "bun"],
            "cache_clear": 7
        }),
    )
    .await;
    insert_config(
        &db,
        "worker__gpu",
        serde_json::json!({
            "dedicated_worker": "ws:f/gpu_inference",
            "autoscaling": {
                "enabled": true,
                "min_workers": 0,
                "max_workers": 4,
                "integration": {"type": "kubernetes"}
            }
        }),
    )
    .await;

    // Read full config
    let config = InstanceConfig::from_db(&db).await.unwrap();

    assert_eq!(
        config.global_settings.base_url.as_deref(),
        Some("https://prod.windmill.dev")
    );
    assert_eq!(
        config
            .global_settings
            .license_key
            .as_ref()
            .and_then(|v| v.as_literal()),
        Some("prod-license-key")
    );
    assert_eq!(config.global_settings.retention_period_secs, Some(2592000));

    let smtp = config.global_settings.smtp_settings.as_ref().unwrap();
    assert_eq!(smtp.smtp_host.as_deref(), Some("smtp.prod.com"));
    assert_eq!(smtp.smtp_from.as_deref(), Some("noreply@prod.com"));

    let channels = config
        .global_settings
        .critical_error_channels
        .as_ref()
        .unwrap();
    assert_eq!(channels.len(), 2);

    let otel = config.global_settings.otel.as_ref().unwrap();
    assert_eq!(otel.metrics_enabled, Some(true));
    assert_eq!(otel.tracing_enabled, Some(true));

    assert_eq!(config.worker_configs.len(), 2);
    assert_eq!(config.worker_configs["default"].extra["cache_clear"], serde_json::json!(7));
    let gpu_auto = config.worker_configs["gpu"].autoscaling.as_ref().unwrap();
    assert!(gpu_auto.enabled);
    assert_eq!(gpu_auto.min_workers, Some(0));
    assert_eq!(gpu_auto.max_workers, Some(4));

    // Verify settings count matches
    let settings_map = config.global_settings.to_settings_map();
    let db_count = count_global_settings(&db).await;
    assert_eq!(
        settings_map.len() as i64,
        db_count,
        "to_settings_map should produce same count as DB rows"
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_diff_apply_only_touches_changed_rows(db: Pool<Postgres>) {
    clear_settings_and_configs(&db).await;

    // Seed 3 settings
    insert_global_setting(&db, "unchanged_1", serde_json::json!("val1")).await;
    insert_global_setting(&db, "unchanged_2", serde_json::json!("val2")).await;
    insert_global_setting(&db, "to_change", serde_json::json!("old")).await;

    let mut current = BTreeMap::new();
    current.insert("unchanged_1".to_string(), serde_json::json!("val1"));
    current.insert("unchanged_2".to_string(), serde_json::json!("val2"));
    current.insert("to_change".to_string(), serde_json::json!("old"));

    let mut desired = current.clone();
    desired.insert("to_change".to_string(), serde_json::json!("new"));
    desired.insert("added".to_string(), serde_json::json!("fresh"));

    let diff = diff_global_settings(&current, &desired, ApplyMode::Merge);

    // Only "to_change" and "added" should be in upserts
    assert_eq!(diff.upserts.len(), 2);
    assert!(diff.upserts.contains_key("to_change"));
    assert!(diff.upserts.contains_key("added"));
    assert!(!diff.upserts.contains_key("unchanged_1"));
    assert!(!diff.upserts.contains_key("unchanged_2"));

    apply_settings_diff(&db, &diff).await.unwrap();

    // Verify all 4 settings present
    assert_eq!(count_global_settings(&db).await, 4);
    assert_eq!(
        get_global_setting(&db, "to_change").await,
        Some(serde_json::json!("new"))
    );
    assert_eq!(
        get_global_setting(&db, "added").await,
        Some(serde_json::json!("fresh"))
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_from_db_reads_migration_defaults(db: Pool<Postgres>) {
    // Verify that from_db correctly reads the migration-seeded state
    // without clearing — tests that pre-existing data is properly deserialized
    let config = InstanceConfig::from_db(&db).await.unwrap();

    // Migrations seed at least base_url, license_key, etc.
    // Just verify from_db doesn't error and returns a populated struct
    let map = config.global_settings.to_settings_map();
    assert!(
        !map.is_empty(),
        "Migration-seeded DB should produce non-empty settings"
    );
    assert!(
        !config.worker_configs.is_empty(),
        "Migration-seeded DB should have worker configs"
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_replace_mode_protects_settings_in_integration(db: Pool<Postgres>) {
    // Seed a protected setting
    insert_global_setting(
        &db,
        "ducklake_settings",
        serde_json::json!({"ducklakes": {}}),
    )
    .await;
    insert_global_setting(&db, "ducklake_user_pg_pwd", serde_json::json!("secret_pwd")).await;
    insert_global_setting(
        &db,
        "custom_instance_pg_databases",
        serde_json::json!({"databases": {}}),
    )
    .await;
    insert_global_setting(&db, "normal_setting", serde_json::json!("will_be_deleted")).await;

    // Read current state
    let config = InstanceConfig::from_db(&db).await.unwrap();
    let current_map = config.global_settings.to_settings_map();

    // Desired: only keep_me — everything else should be deleted except protected
    let mut desired_map = BTreeMap::new();
    desired_map.insert("keep_me".to_string(), serde_json::json!("yes"));

    let diff = diff_global_settings(&current_map, &desired_map, ApplyMode::Replace);

    // Protected keys should NOT be in deletes
    assert!(
        !diff.deletes.contains(&"ducklake_settings".to_string()),
        "ducklake_settings is protected"
    );
    assert!(
        !diff.deletes.contains(&"ducklake_user_pg_pwd".to_string()),
        "ducklake_user_pg_pwd is protected"
    );
    assert!(
        !diff
            .deletes
            .contains(&"custom_instance_pg_databases".to_string()),
        "custom_instance_pg_databases is protected"
    );
    // But normal_setting should be deleted
    assert!(diff.deletes.contains(&"normal_setting".to_string()));

    apply_settings_diff(&db, &diff).await.unwrap();

    // Verify protected settings survived
    assert!(get_global_setting(&db, "ducklake_settings").await.is_some());
    assert!(get_global_setting(&db, "ducklake_user_pg_pwd")
        .await
        .is_some());
    assert!(get_global_setting(&db, "custom_instance_pg_databases")
        .await
        .is_some());
    // Normal setting is gone
    assert!(get_global_setting(&db, "normal_setting").await.is_none());
    // New setting is present
    assert_eq!(
        get_global_setting(&db, "keep_me").await,
        Some(serde_json::json!("yes"))
    );
}
