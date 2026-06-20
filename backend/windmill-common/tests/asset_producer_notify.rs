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
    replace_static_asset_usage, AssetKind, AssetUsageAccessType, AssetUsageKind,
    AssetWithAltAccessType,
};
use windmill_common::scripts::ScriptHash;

const WS: &str = "test-workspace";

/// Run the deploy-time clear+reinsert against `usage_path` in its own tx,
/// mirroring how create_script_internal calls it.
async fn replace(db: &Pool<Postgres>, usage_path: &str, assets: &[AssetWithAltAccessType]) {
    let mut tx = db.begin().await.expect("begin");
    replace_static_asset_usage(&mut tx, WS, usage_path, assets)
        .await
        .expect("replace static asset usage");
    tx.commit().await.expect("commit");
}

fn asset_at(
    path: &str,
    kind: AssetKind,
    access: Option<AssetUsageAccessType>,
) -> AssetWithAltAccessType {
    AssetWithAltAccessType {
        path: path.to_string(),
        kind,
        access_type: access,
        alt_access_type: None,
        columns: None,
    }
}

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

const SP: &str = "u/test-user/script";

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn replace_plain_deploy_does_not_emit(db: Pool<Postgres>) {
    replace(&db, SP, &[]).await;
    assert_eq!(
        producer_notify_count(&db).await,
        0,
        "deploy with no assets must not emit"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn replace_gaining_write_emits_once(db: Pool<Postgres>) {
    replace(
        &db,
        SP,
        &[asset_at(
            "u/test-user/res",
            AssetKind::Resource,
            Some(AssetUsageAccessType::W),
        )],
    )
    .await;
    assert_eq!(
        producer_notify_count(&db).await,
        1,
        "gaining a write producer must emit exactly once"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn replace_unchanged_write_set_does_not_emit(db: Pool<Postgres>) {
    let assets = [asset_at(
        "u/test-user/res",
        AssetKind::Resource,
        Some(AssetUsageAccessType::W),
    )];
    replace(&db, SP, &assets).await;
    reset_notify(&db).await; // isolate the redeploy

    // Redeploy with the identical write-producer set: clear removes the row and
    // the reinsert adds it back, but the cache value is unchanged → no emit.
    replace(&db, SP, &assets).await;
    assert_eq!(
        producer_notify_count(&db).await,
        0,
        "redeploy keeping the same write producers must not emit"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn replace_dropping_write_emits(db: Pool<Postgres>) {
    let assets = [asset_at(
        "u/test-user/res",
        AssetKind::Resource,
        Some(AssetUsageAccessType::RW),
    )];
    replace(&db, SP, &assets).await;
    reset_notify(&db).await;

    // Redeploy with no assets: the write producer disappears → emit.
    replace(&db, SP, &[]).await;
    assert_eq!(
        producer_notify_count(&db).await,
        1,
        "dropping the last write producer must emit"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn replace_readonly_to_write_emits(db: Pool<Postgres>) {
    replace(
        &db,
        SP,
        &[asset_at(
            "u/test-user/res",
            AssetKind::Resource,
            Some(AssetUsageAccessType::R),
        )],
    )
    .await;
    reset_notify(&db).await;

    // Same asset path, access flips read-only → write: cache gains a row → emit.
    replace(
        &db,
        SP,
        &[asset_at(
            "u/test-user/res",
            AssetKind::Resource,
            Some(AssetUsageAccessType::W),
        )],
    )
    .await;
    assert_eq!(
        producer_notify_count(&db).await,
        1,
        "read-only → write transition must emit"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn replace_duplicate_entries_use_persisted_state(db: Pool<Postgres>) {
    replace(
        &db,
        SP,
        &[asset_at(
            "u/test-user/res",
            AssetKind::Resource,
            Some(AssetUsageAccessType::W),
        )],
    )
    .await;
    reset_notify(&db).await;

    // Redeploy with conflicting duplicates for the same (kind, path), read-only
    // first: ON CONFLICT DO NOTHING persists the read-only row and drops the
    // write one, so the write producer is really gone → must emit (the diff must
    // follow the persisted state, not the requested slice).
    replace(
        &db,
        SP,
        &[
            asset_at(
                "u/test-user/res",
                AssetKind::Resource,
                Some(AssetUsageAccessType::R),
            ),
            asset_at(
                "u/test-user/res",
                AssetKind::Resource,
                Some(AssetUsageAccessType::W),
            ),
        ],
    )
    .await;
    assert_eq!(
        producer_notify_count(&db).await,
        1,
        "duplicate entries losing the persisted write producer must emit"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn replace_duplicate_entries_keeping_write_does_not_emit(db: Pool<Postgres>) {
    replace(
        &db,
        SP,
        &[asset_at(
            "u/test-user/res",
            AssetKind::Resource,
            Some(AssetUsageAccessType::W),
        )],
    )
    .await;
    reset_notify(&db).await;

    // Same duplicates but write first: the write row persists, so the write set
    // is unchanged → no emit.
    replace(
        &db,
        SP,
        &[
            asset_at(
                "u/test-user/res",
                AssetKind::Resource,
                Some(AssetUsageAccessType::W),
            ),
            asset_at(
                "u/test-user/res",
                AssetKind::Resource,
                Some(AssetUsageAccessType::R),
            ),
        ],
    )
    .await;
    assert_eq!(
        producer_notify_count(&db).await,
        0,
        "duplicate entries keeping the persisted write producer must not emit"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn replace_changing_only_readonly_does_not_emit(db: Pool<Postgres>) {
    replace(
        &db,
        SP,
        &[asset_at(
            "u/test-user/res",
            AssetKind::Resource,
            Some(AssetUsageAccessType::R),
        )],
    )
    .await;
    reset_notify(&db).await;

    // Swap one read-only producer for another: no write producer either side, so
    // the write-set cache is untouched → no emit.
    replace(
        &db,
        SP,
        &[asset_at(
            "u/test-user/other",
            AssetKind::Resource,
            Some(AssetUsageAccessType::R),
        )],
    )
    .await;
    assert_eq!(
        producer_notify_count(&db).await,
        0,
        "changes confined to read-only producers must not emit"
    );
}
