use std::collections::HashMap;

use futures::future::join_all;
use sqlx::{Pool, Postgres};

use crate::{db::get_shard_db_from_shard_id, SHARD_ID_TO_DB_INSTANCE, SHARD_MODE};

pub async fn get_queue_counts(db: &Pool<Postgres>) -> HashMap<String, u32> {
    if *SHARD_MODE {
        return get_queue_counts_from_shards().await;
    }
    get_queue_counts_single_db(db).await
}

pub async fn get_queue_running_counts(db: &Pool<Postgres>) -> HashMap<String, u32> {
    if *SHARD_MODE {
        return get_queue_running_counts_from_shards().await;
    }
    get_queue_running_counts_single_db(db).await
}

async fn get_queue_counts_single_db(db: &Pool<Postgres>) -> HashMap<String, u32> {
    sqlx::query!(
        "SELECT tag AS \"tag!\", count(*) AS \"count!\" FROM v2_job_queue WHERE
            scheduled_for <= now() - ('3 seconds')::interval AND running = false
            GROUP BY tag",
    )
    .fetch_all(db)
    .await
    .ok()
    .map(|v| v.into_iter().map(|x| (x.tag, x.count as u32)).collect())
    .unwrap_or_else(|| HashMap::new())
}

async fn get_queue_running_counts_single_db(db: &Pool<Postgres>) -> HashMap<String, u32> {
    sqlx::query!(
        "SELECT tag AS \"tag!\", count(*) AS \"count!\" FROM v2_job_queue WHERE
            running = true
            GROUP BY tag",
    )
    .fetch_all(db)
    .await
    .ok()
    .map(|v| v.into_iter().map(|x| (x.tag, x.count as u32)).collect())
    .unwrap_or_else(|| HashMap::new())
}

async fn get_queue_counts_from_shards() -> HashMap<String, u32> {
    let shard_db_map = match SHARD_ID_TO_DB_INSTANCE.get() {
        Some(map) => map,
        None => return HashMap::new(),
    };
    let shard_count = shard_db_map.len();

    if shard_count == 0 {
        return HashMap::new();
    }

    let mut futures = Vec::new();

    for shard_id in 0..shard_count {
        if let Some(shard_db) = get_shard_db_from_shard_id(shard_id) {
            let future = async move { get_queue_counts_single_db(shard_db).await };
            futures.push(future);
        }
    }

    let shard_results = join_all(futures).await;

    let mut aggregated_counts = HashMap::new();

    for shard_counts in shard_results {
        for (tag, count) in shard_counts {
            *aggregated_counts.entry(tag).or_insert(0) += count;
        }
    }

    aggregated_counts
}

async fn get_queue_running_counts_from_shards() -> HashMap<String, u32> {
    let shard_db_map = match SHARD_ID_TO_DB_INSTANCE.get() {
        Some(map) => map,
        None => return HashMap::new(),
    };
    let shard_count = shard_db_map.len();

    if shard_count == 0 {
        return HashMap::new();
    }

    let mut futures = Vec::new();

    for shard_id in 0..shard_count {
        if let Some(shard_db) = get_shard_db_from_shard_id(shard_id) {
            let future = async move { get_queue_running_counts_single_db(shard_db).await };
            futures.push(future);
        }
    }

    let shard_results = join_all(futures).await;

    let mut aggregated_counts = HashMap::new();

    for shard_counts in shard_results {
        for (tag, count) in shard_counts {
            *aggregated_counts.entry(tag).or_insert(0) += count;
        }
    }

    aggregated_counts
}
