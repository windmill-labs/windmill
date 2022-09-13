/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::net::SocketAddr;

use dotenv::dotenv;
use windmill::WorkerConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    windmill::initialize_tracing();

    let db = windmill::connect_db().await?;

    let num_workers = std::env::var("NUM_WORKERS")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(windmill::DEFAULT_NUM_WORKERS as i32);

    let metrics_addr: Option<SocketAddr> = std::env::var("METRICS_ADDR")
        .ok()
        .map(|s| {
            s.parse::<bool>()
                .map(|b| b.then(|| SocketAddr::from(([0, 0, 0, 0], 8001))))
                .or_else(|_| s.parse::<SocketAddr>().map(Some))
        })
        .transpose()?
        .flatten();

    let (server_mode, monitor_mode, migrate_db) = (true, true, true);

    if migrate_db {
        windmill::migrate_db(&db).await?;
    }

    let (tx, rx) = tokio::sync::broadcast::channel::<()>(3);
    let shutdown_signal = windmill::shutdown_signal(tx);

    let base_internal_url = std::env::var("BASE_INTERNAL_URL")
        .unwrap_or_else(|_| "http://missing-base-url".to_string());

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost".to_string());

    if server_mode || monitor_mode || num_workers > 0 {
        let addr = SocketAddr::from(([0, 0, 0, 0], 8000));

        let timeout = std::env::var("TIMEOUT")
            .ok()
            .and_then(|x| x.parse::<i32>().ok())
            .unwrap_or(windmill::DEFAULT_TIMEOUT);

        let base_url_2 = base_url.clone();
        let server_f = async {
            if server_mode {
                windmill::run_server(
                    db.clone(),
                    addr,
                    &base_url_2,
                    windmill::EmailSender {
                        from: "bot@windmill.dev".to_string(),
                        server: "smtp.gmail.com".to_string(),
                        password: std::env::var("SMTP_PASSWORD").unwrap_or("NOPASS".to_string()),
                    },
                    rx.resubscribe(),
                )
                .await?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let workers_f = async {
            if num_workers > 0 {
                let sleep_queue = std::env::var("SLEEP_QUEUE")
                    .ok()
                    .and_then(|x| x.parse::<u64>().ok())
                    .unwrap_or(windmill::DEFAULT_SLEEP_QUEUE);
                let disable_nuser = std::env::var("DISABLE_NUSER")
                    .ok()
                    .and_then(|x| x.parse::<bool>().ok())
                    .unwrap_or(false);
                let disable_nsjail = std::env::var("DISABLE_NSJAIL")
                    .ok()
                    .and_then(|x| x.parse::<bool>().ok())
                    .unwrap_or(false);
                let keep_job_dir = std::env::var("KEEP_JOB_DIR")
                    .ok()
                    .and_then(|x| x.parse::<bool>().ok())
                    .unwrap_or(false);

                tracing::info!(
                    "DISABLE_NSJAIL: {disable_nsjail}, DISABLE_NUSER: {disable_nuser}, BASE_URL: \
                     {base_url}, SLEEP_QUEUE: {sleep_queue}, NUM_WORKERS: {num_workers}, TIMEOUT: \
                     {timeout}, KEEP_JOB_DIR: {keep_job_dir}"
                );
                windmill::run_workers(
                    db.clone(),
                    addr,
                    timeout,
                    num_workers,
                    sleep_queue,
                    WorkerConfig {
                        disable_nsjail,
                        disable_nuser,
                        base_internal_url,
                        base_url,
                        keep_job_dir,
                    },
                    rx.resubscribe(),
                )
                .await?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let monitor_f = async {
            if monitor_mode {
                windmill::monitor_db(&db, timeout, rx.resubscribe());
            }
            Ok(()) as anyhow::Result<()>
        };

        let metrics_f = async {
            match metrics_addr {
                Some(addr) => windmill::serve_metrics(addr, rx.resubscribe())
                    .await
                    .map_err(anyhow::Error::from),
                None => Ok(()),
            }
        };

        futures::try_join!(shutdown_signal, server_f, workers_f, monitor_f, metrics_f)?;
    }

    Ok(())
}
