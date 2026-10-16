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
}
