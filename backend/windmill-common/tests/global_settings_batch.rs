//! `load_values_from_global_settings` is what a settings pass fetches with, so the difference
//! between "no row" and "the read failed" has to survive it: several settings reset to a
//! default when they read as unset, and would clobber a known-good value on a transient error.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::global_settings::{
    load_values_from_global_settings, BASE_URL_SETTING, SCIM_TOKEN_SETTING,
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
async fn batch_returns_only_rows_that_exist(db: Pool<Postgres>) {
    set_setting(&db, BASE_URL_SETTING, json!("set")).await;
    // A dynamically named row, of which the table holds one per workspace with no cleanup
    // path. Asking by name is what keeps a pass from scaling with how many of them exist.
    set_setting(&db, "wm_test_dynamic:some_workspace", json!({})).await;

    let values = load_values_from_global_settings(&db, &[BASE_URL_SETTING, SCIM_TOKEN_SETTING])
        .await
        .unwrap();

    assert_eq!(values.get(BASE_URL_SETTING), Some(&json!("set")));
    assert_eq!(
        values.get(SCIM_TOKEN_SETTING),
        None,
        "a name with no row must be absent, which the caller reads as unset"
    );
    assert_eq!(values.len(), 1, "unrequested names must not come back");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn batch_reports_failure_rather_than_an_empty_result(db: Pool<Postgres>) {
    set_setting(&db, BASE_URL_SETTING, json!("set")).await;

    // Closing a pool makes every query on it fail. It has to be a separate pool built from the
    // same options — `Pool` is a handle, so closing a clone of `db` would take `db` down too.
    let unusable = sqlx::postgres::PgPoolOptions::new()
        .connect_with((*db.connect_options()).clone())
        .await
        .expect("failed to open second pool");
    unusable.close().await;

    assert!(
        load_values_from_global_settings(&unusable, &[BASE_URL_SETTING])
            .await
            .is_err(),
        "a failed read must be an error, not an empty map that reads as every setting unset"
    );
}
