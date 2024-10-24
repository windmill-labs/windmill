use std::collections::HashMap;

use sqlx::{Pool, Postgres};

pub async fn get_queue_counts(db: &Pool<Postgres>) -> HashMap<String, i64> {
    sqlx::query_as::<_, (String, i64)>(
        "SELECT tag, count(*) as count FROM queue WHERE
            scheduled_for <= now() - ('3 seconds')::interval AND running = false
            GROUP BY tag",
    )
    .fetch_all(db)
    .await
    .ok()
    .map(|v| v.into_iter().collect())
    .unwrap_or_else(|| HashMap::new())
}
