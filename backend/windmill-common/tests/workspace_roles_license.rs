#![cfg(feature = "enterprise")]
//! Flips the process-wide `LICENSE_KEY_VALID`, so it keeps a test binary of its own.

use std::{collections::BTreeMap, sync::atomic::Ordering};

use sqlx::{Pool, Postgres};
use windmill_common::ee_oss::LICENSE_KEY_VALID;
use windmill_common::instance_config::sync_global_settings_declarative;
use windmill_common::roles::{
    self, check_frozen_overrides, instance_roles, load_role_overrides,
    validate_instance_hub_access_write, validate_instance_roles_write, RoleOverrides,
};

/// A lapsed license freezes the configured roles: falling back to the defaults would give a
/// restricted `developer` everything and strand the members of workspace-only roles.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn lapsed_license_freezes_roles(db: Pool<Postgres>) {
    let restricted = serde_json::json!([
        {"name": "developer", "capabilities": {"code": false, "flows": "none", "apps": "none"}}
    ]);
    sqlx::query("INSERT INTO global_settings (name, value) VALUES ('workspace_roles', $1)")
        .bind(&restricted)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE workspace_settings SET role_overrides = '{\"auditor\": {\"label\": \"Auditor\"}}'
         WHERE workspace_id = 'test-workspace'",
    )
    .execute(&db)
    .await
    .unwrap();

    LICENSE_KEY_VALID.store(false, Ordering::Relaxed);
    roles::invalidate_all_roles_cache();

    let ws = roles::workspace_roles(&db, "test-workspace").await.unwrap();
    let dev = ws.iter().find(|r| r.name == "developer").unwrap();
    assert!(
        !dev.capabilities.code,
        "the restricted developer must not widen"
    );
    assert!(
        ws.iter().any(|r| r.name == "auditor"),
        "workspace-only roles keep applying"
    );

    let changed = serde_json::json!([{"name": "developer"}, {"name": "scripter"}]);
    validate_instance_roles_write(&db, Some(&changed))
        .await
        .expect_err("changing instance roles needs a license");
    validate_instance_roles_write(&db, Some(&restricted))
        .await
        .expect("resaving the stored roles is a no-op");
    validate_instance_roles_write(&db, None)
        .await
        .expect("resetting to the defaults stays open");

    let instance = instance_roles(&db).await.unwrap();
    let previous = load_role_overrides(&db, "test-workspace").await.unwrap();
    let mut relabelled = previous.clone();
    relabelled.get_mut("auditor").unwrap().label = Some("Reader".to_string());
    check_frozen_overrides(&instance, &previous, &relabelled)
        .expect_err("editing a workspace-only role needs a license");
    check_frozen_overrides(&instance, &previous, &previous).expect("unchanged entries pass");
    check_frozen_overrides(&instance, &previous, &RoleOverrides::new())
        .expect("removing a workspace-only role stays open");

    // The instance's Hub access list is frozen the same way, and can always be cleared.
    let slack = BTreeMap::from([(
        "hub_access".to_string(),
        serde_json::json!({"slack": "read"}),
    )]);
    sync_global_settings_declarative(&db, &BTreeMap::new(), &slack, &())
        .await
        .expect_err("changing the instance's Hub scripts needs a license");
    validate_instance_hub_access_write(&db, &serde_json::json!({}))
        .await
        .expect("clearing stays open");

    LICENSE_KEY_VALID.store(true, Ordering::Relaxed);
    sync_global_settings_declarative(&db, &BTreeMap::new(), &slack, &())
        .await
        .expect("licensed again");
    validate_instance_hub_access_write(&db, &serde_json::json!({"slack/x": "read"}))
        .await
        .expect_err("a license does not let a malformed list through");
    LICENSE_KEY_VALID.store(false, Ordering::Relaxed);
    validate_instance_hub_access_write(&db, &serde_json::json!({"Slack": "read"}))
        .await
        .expect("writing the stored list back unchanged stays open");
    validate_instance_hub_access_write(&db, &serde_json::json!({"slack": "write"}))
        .await
        .expect_err("changing the stored list still needs a license");
    roles::invalidate_all_roles_cache();
    let ws = roles::workspace_roles(&db, "test-workspace").await.unwrap();
    let dev = ws.iter().find(|r| r.name == "developer").unwrap();
    assert!(
        dev.capabilities.hub.contains_key("slack"),
        "the stored list keeps applying"
    );
    LICENSE_KEY_VALID.store(true, Ordering::Relaxed);
    check_frozen_overrides(&instance, &previous, &relabelled).expect("licensed again");
}
