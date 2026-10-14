/*!
 * The integration tests clear the rights cache in-process, so they pass whether or not this
 * trigger fires. Only the emitting half is covered here: `process_notify_event` lives in the
 * server binary.
 */

use sqlx::{Pool, Postgres};

const WS: &str = "test-workspace";

async fn emitted_payloads(db: &Pool<Postgres>) -> Vec<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT payload FROM notify_event WHERE channel = 'notify_operator_settings_change'",
    )
    .fetch_all(db)
    .await
    .expect("read notify events")
}

async fn set_operator_settings(db: &Pool<Postgres>, settings: &str) {
    sqlx::query(
        "UPDATE workspace_settings SET operator_settings = $2::jsonb WHERE workspace_id = $1",
    )
    .bind(WS)
    .bind(settings)
    .execute(db)
    .await
    .expect("update operator settings");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn withdrawing_a_right_emits_the_cache_invalidation(db: Pool<Postgres>) {
    set_operator_settings(&db, r#"{"manage_schedules": false}"#).await;
    assert_eq!(
        emitted_payloads(&db).await,
        vec![WS.to_string()],
        "the payload must be the workspace id: `process_notify_event` invalidates by it"
    );

    // Rewriting the same settings must stay silent, or every unrelated save of the settings row
    // would drop the entry on every replica and send them all back to the database.
    set_operator_settings(&db, r#"{"manage_schedules": false}"#).await;
    assert_eq!(
        emitted_payloads(&db).await.len(),
        1,
        "an update that leaves operator_settings unchanged must not emit"
    );
}
