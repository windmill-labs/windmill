use std::str::FromStr;

use crate::{
    error::{to_anyhow, Result},
    global_settings::{DISABLE_STATS_SETTING, UNIQUE_ID_SETTING},
    utils::GIT_VERSION,
    DB,
};
use chrono::Utc;
use cron::Schedule;

pub async fn get_disable_stats_setting(db: &DB) -> bool {
    let q = sqlx::query!(
        "SELECT value FROM global_settings WHERE name = $1",
        DISABLE_STATS_SETTING
    )
    .fetch_optional(db)
    .await;

    if let Ok(q) = q {
        if let Some(q) = q {
            if let Ok(v) = serde_json::from_value::<bool>(q.value.clone()) {
                return v;
            } else {
                tracing::error!(
                    "Could not parse DISABLE_STATS_SETTING found: {:#?}",
                    &q.value
                );
            }
        }
    };

    false
}

pub async fn schedule_stats(db: &DB, instance_name: String, http_client: &reqwest::Client) -> () {
    let http_client = http_client.clone();
    let db = db.clone();
    tokio::spawn(async move {
        loop {
            let disabled = get_disable_stats_setting(&db).await;
            if !disabled {
                tracing::info!("Sending stats");
                let result = send_stats(&instance_name, &http_client, &db).await;
                if result.is_err() {
                    tracing::info!("Error sending stats: {}", result.err().unwrap());
                } else {
                    tracing::info!("Stats sent");
                }
            }

            let s = "0 0 */24 * * * *"; // Every 24 hours
            let s = Schedule::from_str(&s);
            if s.is_err() {
                tracing::error!("Invalid schedule for stats");
                return;
            }
            let s = s.unwrap();

            let next_time = s.upcoming(Utc).next();
            if next_time.is_none() {
                tracing::error!("Invalid schedule for stats");
                return;
            }
            let next_time = next_time.unwrap();
            let duration_to_next = next_time - Utc::now();

            tokio::time::sleep(tokio::time::Duration::from_millis(
                duration_to_next.num_milliseconds() as u64,
            ))
            .await;
        }
    });
}

pub async fn send_stats(
    instance_name: &String,
    http_client: &reqwest::Client,
    db: &DB,
) -> Result<()> {
    let uid = sqlx::query_scalar!(
        "SELECT value FROM global_settings WHERE name = $1",
        UNIQUE_ID_SETTING
    )
    .fetch_one(db)
    .await?;

    let uid = serde_json::from_value::<String>(uid).map_err(to_anyhow)?;

    let nb_of_jobs = sqlx::query_scalar!("SELECT COUNT(*) FROM completed_job")
        .fetch_one(db)
        .await?
        .unwrap_or(0);

    let total_duration_of_jobs =
        sqlx::query_scalar!("SELECT SUM(duration_ms)::BIGINT FROM completed_job")
            .fetch_one(db)
            .await?
            .unwrap_or(0);

    let nb_of_users_per_login_type =
        sqlx::query!("SELECT login_type, COUNT(*) FROM password GROUP BY login_type")
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "login_type": r.login_type,
                    "count": r.count.unwrap_or(0),
                })
            })
            .collect::<Vec<serde_json::Value>>();

    let payload = serde_json::json!({
        "uid": uid,
        "version": GIT_VERSION,
        "instance_name": instance_name,
        "nb_of_jobs": nb_of_jobs,
        "total_duration_of_jobs": total_duration_of_jobs,
        "nb_of_users_per_login_type": nb_of_users_per_login_type,
    });

    let request = http_client
        .post("https://hub.windmill.dev/stats")
        .body(serde_json::to_string(&payload).map_err(to_anyhow)?)
        .header("content-type", "application/json");

    request
        .send()
        .await
        .map_err(to_anyhow)?
        .error_for_status()
        .map_err(to_anyhow)?;

    Ok(())
}
