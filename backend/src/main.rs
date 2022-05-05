/*
* Author & Copyright: Ruben Fiszel 2021
 * This file and its contents are licensed under the AGPL License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::net::SocketAddr;

use dotenv::dotenv;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    windmill::initialize_tracing().await?;

    let db = windmill::connect_db().await?;

    let num_workers = std::env::var("NUM_WORKERS")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(windmill::DEFAULT_NUM_WORKERS as i32);

    let (server_mode, monitor_mode, migrate_db) = (true, true, true);

    if migrate_db {
        windmill::migrate_db(&db).await?;
    }

    let (tx, rx) = tokio::sync::broadcast::channel::<()>(3);
    let shutdown_signal = windmill::shutdown_signal(tx.clone());

    if server_mode || monitor_mode || num_workers > 0 {
        let addr = SocketAddr::from(([0, 0, 0, 0], 8000));

        let timeout = std::env::var("TIMEOUT")
            .ok()
            .and_then(|x| x.parse::<i32>().ok())
            .unwrap_or(windmill::DEFAULT_TIMEOUT);

        let server_f = async {
            if server_mode {
                windmill::run_server(
                    db.clone(),
                    addr,
                    &std::env::var("BASE_URL").unwrap_or("http://localhost".to_string()),
                    windmill::EmailSender {
                        from: "bot@windmill.dev".to_string(),
                        server: "smtp.gmail.com".to_string(),
                        password: std::env::var("SMTP_PASSWORD").unwrap_or("NOPASS".to_string()),
                    },
                    rx,
                )
                .await?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let base_url = std::env::var("BASE_INTERNAL_URL")
            .unwrap_or_else(|_| "http://missing-base-url".to_string());

        let workers_f = async {
            if num_workers > 0 {
                let sleep_queue = std::env::var("SLEEP_QUEUE")
                    .ok()
                    .and_then(|x| x.parse::<u64>().ok())
                    .unwrap_or(windmill::DEFAULT_SLEEP_QUEUE);

                windmill::run_workers(
                    db.clone(),
                    addr,
                    timeout,
                    num_workers,
                    sleep_queue,
                    base_url,
                    tx.clone(),
                )
                .await?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let monitor_f = async {
            if monitor_mode {
                windmill::monitor_db(&db, timeout, tx.clone());
            }
            Ok(()) as anyhow::Result<()>
        };

        futures::try_join!(shutdown_signal, server_f, workers_f, monitor_f)?;
    }

    Ok(())
}
