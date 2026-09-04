/*!
 * One rule, two spellings: "is dbt the sole producer of this warehouse
 * relation". `sole_dbt_producer` decides whether a `// on dbt://…` subscription
 * is refused at deploy; `dormant_dbt_subscriptions` names the edges a dbt deploy
 * retroactively leaves unwakeable. Both must answer "yes, dormant" only when
 * every script writing the relation is a dbt one — a dbt run does not dispatch —
 * and "no" both when a native `// materialize manual dbt://…` producer exists and
 * when nothing produces the relation yet, which is the ordinary deploy-order
 * case. Every way of getting this wrong is silent: a dormant edge on the canvas,
 * a refused deploy of a valid pipeline, or a warning that stops appearing.
 */

use sqlx::{Pool, Postgres};
use windmill_common::assets::{dormant_dbt_subscriptions, sole_dbt_producer};

const WS: &str = "test-workspace";
const RELATION: &str = "main/analytics/orders";
const SUBSCRIBER: &str = "u/test-user/consumer";

async fn plant_producer(db: &Pool<Postgres>, path: &str, language: &str, hash: i64) {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by,
                             language)
         VALUES ($1, $2, $3, '', '', '', 'test-user', $4::text::script_lang)",
    )
    .bind(WS)
    .bind(hash)
    .bind(path)
    .bind(language)
    .execute(db)
    .await
    .expect("insert script");
    sqlx::query(
        "INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind)
         VALUES ($1, $2, 'dbt', 'w', $3, 'script') ON CONFLICT DO NOTHING",
    )
    .bind(WS)
    .bind(RELATION)
    .bind(path)
    .execute(db)
    .await
    .expect("insert asset");
}

async fn plant_subscriber(db: &Pool<Postgres>, path: &str) {
    sqlx::query(
        "INSERT INTO script_trigger (workspace_id, runnable_kind, runnable_path, trigger_kind,
                                     trigger_ref)
         VALUES ($1, 'script', $2, 'asset', 'dbt://' || $3)",
    )
    .bind(WS)
    .bind(path)
    .bind(RELATION)
    .execute(db)
    .await
    .expect("insert script_trigger");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn no_producer_is_not_dormant(db: Pool<Postgres>) {
    assert_eq!(
        sole_dbt_producer(&db, WS, RELATION, SUBSCRIBER)
            .await
            .unwrap(),
        None,
        "a relation nothing produces yet must not refuse the subscription"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn dbt_only_producer_is_dormant(db: Pool<Postgres>) {
    plant_producer(&db, "u/test-user/project", "dbt", 1).await;
    assert_eq!(
        sole_dbt_producer(&db, WS, RELATION, SUBSCRIBER)
            .await
            .unwrap(),
        Some("u/test-user/project".to_string())
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_native_producer_beside_dbt_is_not_dormant(db: Pool<Postgres>) {
    plant_producer(&db, "u/test-user/project", "dbt", 1).await;
    plant_producer(&db, "u/test-user/ingest", "postgresql", 2).await;
    assert_eq!(
        sole_dbt_producer(&db, WS, RELATION, SUBSCRIBER)
            .await
            .unwrap(),
        None
    );
}

/// `asset` is keyed by path while `script` holds every version of it, so the
/// language has to be read off the live one: a path converted to dbt still has
/// its old native versions sitting in `script`.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_superseded_native_version_does_not_count(db: Pool<Postgres>) {
    plant_producer(&db, "u/test-user/project", "postgresql", 1).await;
    sqlx::query("UPDATE script SET archived = true WHERE hash = 1")
        .execute(&db)
        .await
        .expect("archive the old version");
    plant_producer(&db, "u/test-user/project", "dbt", 2).await;
    assert_eq!(
        sole_dbt_producer(&db, WS, RELATION, SUBSCRIBER)
            .await
            .unwrap(),
        Some("u/test-user/project".to_string())
    );
}

/// The rows of the script being deployed describe the version it replaces, so a
/// script dropping its `// materialize` while adding a subscription would
/// otherwise count itself as the producer that wakes it — and commit a dormant
/// edge. It can never be that producer anyway: the dispatcher skips self-loops.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn the_subscriber_is_never_its_own_producer(db: Pool<Postgres>) {
    plant_producer(&db, "u/test-user/project", "dbt", 1).await;
    plant_producer(&db, SUBSCRIBER, "postgresql", 2).await;
    assert_eq!(
        sole_dbt_producer(&db, WS, RELATION, SUBSCRIBER)
            .await
            .unwrap(),
        Some("u/test-user/project".to_string())
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn the_set_form_agrees_with_the_singular_one(db: Pool<Postgres>) {
    let relations = vec![RELATION.to_string()];
    plant_subscriber(&db, "u/test-user/consumer").await;
    plant_producer(&db, "u/test-user/project", "dbt", 1).await;
    assert_eq!(
        dormant_dbt_subscriptions(&db, WS, &relations)
            .await
            .unwrap(),
        vec![format!("dbt://{RELATION} → u/test-user/consumer")],
        "dbt alone builds it, so the subscription can never be woken"
    );

    plant_producer(&db, "u/test-user/ingest", "postgresql", 2).await;
    assert!(
        dormant_dbt_subscriptions(&db, WS, &relations)
            .await
            .unwrap()
            .is_empty(),
        "a native producer wakes it, so the edge is live"
    );
}

/// The two ways the set form could stop meaning what the singular one means: a
/// relation nothing produces is deploy order rather than a dormant edge, and a
/// subscriber's own write is not a producer that can wake it — the dispatcher
/// skips self-loops, so that edge is dormant and has to be named.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn the_set_form_matches_on_the_edge_cases_too(db: Pool<Postgres>) {
    let relations = vec![RELATION.to_string()];
    plant_subscriber(&db, SUBSCRIBER).await;
    assert!(
        dormant_dbt_subscriptions(&db, WS, &relations)
            .await
            .unwrap()
            .is_empty(),
        "nothing produces it yet, so nothing is dormant"
    );

    plant_producer(&db, "u/test-user/project", "dbt", 1).await;
    plant_producer(&db, SUBSCRIBER, "postgresql", 2).await;
    assert_eq!(
        dormant_dbt_subscriptions(&db, WS, &relations)
            .await
            .unwrap(),
        vec![format!("dbt://{RELATION} → {SUBSCRIBER}")],
        "the subscriber's own write cannot wake it, so dbt is still the sole producer"
    );
}
