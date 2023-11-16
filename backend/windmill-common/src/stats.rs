use std::str::FromStr;

use crate::{
    error::{to_anyhow, Result},
    global_settings::DISABLE_STATS_SETTING,
    scripts::ScriptLang,
    utils::{get_uid, Mode, GIT_VERSION},
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

pub async fn schedule_stats(
    instance_name: String,
    mode: Mode,
    db: &DB,
    http_client: &reqwest::Client,
) -> () {
    let http_client = http_client.clone();
    let db = db.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(180)).await;

        loop {
            let disabled = get_disable_stats_setting(&db).await;
            if !disabled {
                tracing::info!("Sending stats");
                let result = send_stats(&instance_name, &mode, &http_client, &db).await;
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

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct JobsUsage {
    language: Option<ScriptLang>,
    total_duration: i64,
    count: i64,
}

pub async fn send_stats(
    instance_name: &String,
    mode: &Mode,
    http_client: &reqwest::Client,
    db: &DB,
) -> Result<()> {
    let uid = get_uid(db).await?;

    let jobs_usage = sqlx::query_as::<_, JobsUsage>(
        "SELECT language, COUNT(*) as count, SUM(duration_ms)::BIGINT as total_duration FROM completed_job GROUP BY language",
    )
    .fetch_all(db)
    .await?;

    let login_type_usage =
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

    let workers_usage = sqlx::query!(
        "SELECT COUNT(*) FROM worker_ping WHERE ping_at > NOW() - INTERVAL '2 minutes'"
    )
    .fetch_one(db)
    .await?
    .count
    .unwrap_or(0);

    let users_count = sqlx::query!("SELECT author.count as author_count, operator.count as operator_count FROM (SELECT count(*)::INT FROM usr where usr.operator IS false) as author, (SELECT count(*)::INT FROM usr where usr.operator IS true) as operator")
        .fetch_one(db)
        .await?;

    let vcpus = std::process::Command::new("cat")
        .args(["/sys/fs/cgroup/cpu.max"])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .to_string()
                .split(" ")
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
                .get(0)
                .map(|s| s.to_string())
        })
        .flatten();

    let payload = serde_json::json!({
        "uid": uid,
        "version": GIT_VERSION,
        "instance_name": instance_name,
        "jobs_usage": jobs_usage,
        "login_type_usage": login_type_usage,
        "workers_usage": workers_usage,
        "users_usage": {
            "author_count": users_count.author_count.unwrap_or(0),
            "operator_count": users_count.operator_count.unwrap_or(0),
        },
        "mode": mode,
        "vcpus": vcpus,
    });

    tracing::info!("Sending stats: {:#?}", payload);

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
