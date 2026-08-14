//! The scoping is what makes `with_global_settings_snapshot` safe to batch with: widening it
//! into a process-wide cache would silently stop `notify_global_setting_change` reloads from
//! reaching a running worker.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::global_settings::{
    load_value_from_global_settings, with_global_settings_snapshot,
};

async fn set_setting(db: &Pool<Postgres>, name: &str, value: serde_json::Value) {
    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES ($1, $2) \
         ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(name)
    .bind(value)
    .execute(db)
    .await
    .expect("failed to write global setting");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn snapshot_serves_reads_and_does_not_outlive_its_scope(db: Pool<Postgres>) {
    set_setting(&db, "wm_test_setting", json!("before")).await;

    with_global_settings_snapshot(&db, async {
        assert_eq!(
            load_value_from_global_settings(&db, "wm_test_setting")
                .await
                .unwrap(),
            Some(json!("before"))
        );
        // A key with no row reads as unset, exactly like the per-setting query does.
        assert_eq!(
            load_value_from_global_settings(&db, "wm_test_absent")
                .await
                .unwrap(),
            None
        );

        set_setting(&db, "wm_test_setting", json!("after")).await;
        assert_eq!(
            load_value_from_global_settings(&db, "wm_test_setting")
                .await
                .unwrap(),
            Some(json!("before")),
            "a snapshot must keep serving the instant it was taken"
        );
    })
    .await;

    assert_eq!(
        load_value_from_global_settings(&db, "wm_test_setting")
            .await
            .unwrap(),
        Some(json!("after")),
        "outside the scope, reads must reach the database"
    );
}
