#![cfg(feature = "enterprise")]

use std::collections::BTreeMap;

use sqlx::{Pool, Postgres};
use windmill_common::global_settings::WORKSPACE_ROLES_SETTING;
use windmill_common::instance_config::sync_global_settings_declarative;

/// The declarative config sync writes `workspace_roles` without the HTTP settings handler, so it
/// must validate the value and rebuild `role_seat` itself, or billing misprices the new roles.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn declarative_workspace_roles_validates_and_rebuilds_seats(db: Pool<Postgres>) {
    windmill_common::roles::invalidate_all_roles_cache();
    let desired =
        |roles: serde_json::Value| BTreeMap::from([(WORKSPACE_ROLES_SETTING.to_string(), roles)]);

    sync_global_settings_declarative(
        &db,
        &BTreeMap::new(),
        &desired(serde_json::json!([{"name": "x", "capabilities": {"flows": "full"}}])),
        &(),
    )
    .await
    .expect_err("full flows without code must be refused");

    sync_global_settings_declarative(
        &db,
        &BTreeMap::new(),
        &desired(serde_json::json!([
            {"name": "developer"},
            {"name": "builder", "capabilities": {"flows": "compose"}},
            {"name": "operator"}
        ])),
        &(),
    )
    .await
    .unwrap();
    let author: bool = sqlx::query_scalar(
        "SELECT author FROM role_seat WHERE workspace_id = '' AND role = 'builder'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(author, "a role that composes flows costs a full seat");

    // A workspace grants `builder` full flows on top of an instance definition that codes; the
    // instance then withdrawing `code` would leave that workspace with inline code and no code.
    sync_global_settings_declarative(
        &db,
        &BTreeMap::new(),
        &desired(serde_json::json!([
            {"name": "developer"},
            {"name": "builder", "capabilities": {"code": true, "flows": "compose"}},
            {"name": "operator"}
        ])),
        &(),
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE workspace_settings SET role_overrides =
            '{\"builder\": {\"capabilities\": {\"flows\": \"full\"}}}'
         WHERE workspace_id = 'test-workspace'",
    )
    .execute(&db)
    .await
    .unwrap();
    sync_global_settings_declarative(
        &db,
        &BTreeMap::new(),
        &desired(serde_json::json!([
            {"name": "developer"},
            {"name": "builder", "capabilities": {"code": false, "flows": "compose"}},
            {"name": "operator"}
        ])),
        &(),
    )
    .await
    .expect_err("an instance change that invalidates a workspace override must be refused");
}
