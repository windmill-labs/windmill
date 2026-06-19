/*!
 * Tests that the asset producer-writes cache invalidation
 * (`notify_asset_producer_change`) is emitted only when a deploy actually
 * changes the set of script write-producers ('w'/'rw' asset usage), not on
 * every deploy. The cache (asset_dispatch::ASSET_PRODUCER_WRITES_CACHE) only
 * tracks script rows with write access, so read-only usage, flow usage, and
 * deploys touching no write asset must NOT emit an event.
 */

use sqlx::{Pool, Postgres};
use windmill_common::assets::{
    clear_static_asset_usage, clear_static_asset_usage_by_script_hash, insert_static_asset_usage,
    AssetKind, AssetUsageAccessType, AssetUsageKind, AssetWithAltAccessType,
};
use windmill_common::scripts::ScriptHash;

const WS: &str = "test-workspace";

async fn producer_notify_count(db: &Pool<Postgres>) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notify_event WHERE channel = 'notify_asset_producer_change'",
    )
    .fetch_one(db)
    .await
    .expect("count notify events")
}

async fn reset_notify(db: &Pool<Postgres>) {
    sqlx::query("DELETE FROM notify_event WHERE channel = 'notify_asset_producer_change'")
        .execute(db)
        .await
        .expect("reset notify events");
}

fn asset(access: Option<AssetUsageAccessType>) -> AssetWithAltAccessType {
    AssetWithAltAccessType {
        path: "u/test-user/res".to_string(),
        kind: AssetKind::Resource,
        access_type: access,
        alt_access_type: None,
        columns: None,
    }
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn insert_write_script_asset_emits(db: Pool<Postgres>) {
    insert_static_asset_usage(
        &db,
        WS,
        &asset(Some(AssetUsageAccessType::W)),
        "u/test-user/script",
        AssetUsageKind::Script,
    )
    .await
    .unwrap();
    assert_eq!(producer_notify_count(&db).await, 1, "write asset must emit");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn insert_readonly_script_asset_does_not_emit(db: Pool<Postgres>) {
    insert_static_asset_usage(
        &db,
        WS,
        &asset(Some(AssetUsageAccessType::R)),
        "u/test-user/script",
        AssetUsageKind::Script,
    )
    .await
    .unwrap();
    assert_eq!(
        producer_notify_count(&db).await,
        0,
        "read-only asset is not a write producer"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn insert_write_flow_asset_does_not_emit(db: Pool<Postgres>) {
    insert_static_asset_usage(
        &db,
        WS,
        &asset(Some(AssetUsageAccessType::W)),
        "u/test-user/flow",
        AssetUsageKind::Flow,
    )
    .await
    .unwrap();
    assert_eq!(
        producer_notify_count(&db).await,
        0,
        "flow usage does not affect the script producer cache"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn clear_plain_script_does_not_emit(db: Pool<Postgres>) {
    // No asset rows for this path: a plain script deploy must not emit.
    clear_static_asset_usage(&db, WS, "u/test-user/plain", AssetUsageKind::Script)
        .await
        .unwrap();
    assert_eq!(
        producer_notify_count(&db).await,
        0,
        "plain script deploy must not emit"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn clear_removes_write_producer_emits(db: Pool<Postgres>) {
    insert_static_asset_usage(
        &db,
        WS,
        &asset(Some(AssetUsageAccessType::RW)),
        "u/test-user/script",
        AssetUsageKind::Script,
    )
    .await
    .unwrap();
    reset_notify(&db).await; // ignore the insert-side event; isolate the clear

    clear_static_asset_usage(&db, WS, "u/test-user/script", AssetUsageKind::Script)
        .await
        .unwrap();
    assert_eq!(
        producer_notify_count(&db).await,
        1,
        "removing a write producer must emit"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn clear_removes_only_readonly_does_not_emit(db: Pool<Postgres>) {
    insert_static_asset_usage(
        &db,
        WS,
        &asset(Some(AssetUsageAccessType::R)),
        "u/test-user/script",
        AssetUsageKind::Script,
    )
    .await
    .unwrap();
    reset_notify(&db).await;

    clear_static_asset_usage(&db, WS, "u/test-user/script", AssetUsageKind::Script)
        .await
        .unwrap();
    assert_eq!(
        producer_notify_count(&db).await,
        0,
        "removing only read-only usage must not emit"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn clear_flow_usage_does_not_emit(db: Pool<Postgres>) {
    insert_static_asset_usage(
        &db,
        WS,
        &asset(Some(AssetUsageAccessType::W)),
        "u/test-user/flow",
        AssetUsageKind::Flow,
    )
    .await
    .unwrap();
    reset_notify(&db).await;

    clear_static_asset_usage(&db, WS, "u/test-user/flow", AssetUsageKind::Flow)
        .await
        .unwrap();
    assert_eq!(
        producer_notify_count(&db).await,
        0,
        "clearing flow usage must not emit"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn clear_by_script_hash_gated_on_write(db: Pool<Postgres>) {
    let hash = 123456789_i64;
    sqlx::query(
        r#"INSERT INTO script (workspace_id, hash, path, summary, description, content,
            created_by, language, lock)
           VALUES ($1, $2, 'u/test-user/byhash', '', '', '', 'test-user', 'deno', '')"#,
    )
    .bind(WS)
    .bind(hash)
    .execute(&db)
    .await
    .unwrap();

    // Read-only usage on that script path → clear must not emit.
    insert_static_asset_usage(
        &db,
        WS,
        &asset(Some(AssetUsageAccessType::R)),
        "u/test-user/byhash",
        AssetUsageKind::Script,
    )
    .await
    .unwrap();
    reset_notify(&db).await;

    clear_static_asset_usage_by_script_hash(&db, WS, ScriptHash(hash))
        .await
        .unwrap();
    assert_eq!(
        producer_notify_count(&db).await,
        0,
        "by-hash clear of read-only usage must not emit"
    );

    // Now a write usage → by-hash clear must emit.
    insert_static_asset_usage(
        &db,
        WS,
        &asset(Some(AssetUsageAccessType::W)),
        "u/test-user/byhash",
        AssetUsageKind::Script,
    )
    .await
    .unwrap();
    reset_notify(&db).await;

    clear_static_asset_usage_by_script_hash(&db, WS, ScriptHash(hash))
        .await
        .unwrap();
    assert_eq!(
        producer_notify_count(&db).await,
        1,
        "by-hash clear of write usage must emit"
    );
}
