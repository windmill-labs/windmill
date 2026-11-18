/*!
 * Tests the schema-capture versioning contract (gap #2a):
 * `record_asset_schema` inserts a new `materialized_asset_schema` version only
 * when the captured column set changes, re-affirms the latest row in place when
 * it doesn't, and `list_asset_schemas` returns the evolution history newest
 * first.
 */

use sqlx::{Pool, Postgres};
use windmill_common::assets::AssetKind;
use windmill_common::materialization::{list_asset_schemas, record_asset_schema, SchemaColumn};

const WS: &str = "test-workspace";
const PATH: &str = "analytics/orders";

fn col(name: &str, ty: &str) -> SchemaColumn {
    SchemaColumn { name: name.to_string(), data_type: ty.to_string() }
}

async fn record(db: &Pool<Postgres>, cols: &[SchemaColumn], snapshot_id: i64) -> bool {
    let mut tx = db.begin().await.expect("begin");
    let inserted = record_asset_schema(
        &mut tx,
        WS,
        AssetKind::Ducklake,
        PATH,
        cols,
        Some(snapshot_id),
        None,
    )
    .await
    .expect("record asset schema");
    tx.commit().await.expect("commit");
    inserted
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn first_capture_inserts_version_one(db: Pool<Postgres>) {
    let cols = [col("order_id", "BIGINT"), col("status", "VARCHAR")];
    assert!(
        record(&db, &cols, 10).await,
        "first capture inserts a version"
    );

    let versions = list_asset_schemas(&db, WS, AssetKind::Ducklake, PATH)
        .await
        .unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].version, 1);
    assert_eq!(versions[0].snapshot_id, Some(10));
    assert_eq!(versions[0].columns.0, cols);
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn unchanged_schema_reaffirms_without_new_version(db: Pool<Postgres>) {
    let cols = [col("order_id", "BIGINT")];
    record(&db, &cols, 10).await;
    // Identical column set on a later snapshot: no new version, but the latest
    // row's snapshot_id advances.
    assert!(
        !record(&db, &cols, 20).await,
        "unchanged schema must not insert a new version"
    );

    let versions = list_asset_schemas(&db, WS, AssetKind::Ducklake, PATH)
        .await
        .unwrap();
    assert_eq!(versions.len(), 1, "still a single version");
    assert_eq!(versions[0].version, 1);
    assert_eq!(versions[0].snapshot_id, Some(20), "snapshot re-affirmed");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn changed_schema_bumps_version_newest_first(db: Pool<Postgres>) {
    record(&db, &[col("order_id", "BIGINT")], 10).await;
    // A column added → schema changed → new version.
    let evolved = [col("order_id", "BIGINT"), col("amount", "DOUBLE")];
    assert!(record(&db, &evolved, 20).await, "changed schema inserts v2");

    let versions = list_asset_schemas(&db, WS, AssetKind::Ducklake, PATH)
        .await
        .unwrap();
    assert_eq!(versions.len(), 2);
    // Newest first.
    assert_eq!(versions[0].version, 2);
    assert_eq!(versions[0].columns.0, evolved);
    assert_eq!(versions[1].version, 1);
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn column_order_change_is_a_new_version(db: Pool<Postgres>) {
    let a = [col("a", "BIGINT"), col("b", "VARCHAR")];
    let reordered = [col("b", "VARCHAR"), col("a", "BIGINT")];
    record(&db, &a, 10).await;
    // Same columns, different physical order: the captured list is ordered, so a
    // reorder is a real schema change (downstream `SELECT *` consumers see it).
    assert!(
        record(&db, &reordered, 20).await,
        "column reorder is a distinct schema version"
    );
    let versions = list_asset_schemas(&db, WS, AssetKind::Ducklake, PATH)
        .await
        .unwrap();
    assert_eq!(versions.len(), 2);
}
