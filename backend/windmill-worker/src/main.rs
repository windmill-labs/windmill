/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{net::SocketAddr, time::Duration};

use anyhow::Context;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use windmill_common::{
    error::{self, Error},
    utils::rd_string,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // dotenv().ok();

    windmill_common::tracing_init::initialize_tracing();

    let db = async {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| Error::BadConfig("DATABASE_URL env var is missing".to_string()))?;

        let max_connections = match std::env::var("DATABASE_CONNECTIONS") {
            Ok(n) => n.parse::<u32>().context("invalid DATABASE_CONNECTIONS")?,
            Err(_) => 10,
        };

        Ok::<Pool<Postgres>, error::Error>(
            PgPoolOptions::new()
                .max_connections(max_connections)
                .max_lifetime(Duration::from_secs(30 * 60)) // 30 mins
                .connect(&database_url)
                .await
                .map_err(|err| Error::ConnectingToDatabase(err.to_string()))?,
        )
    }
    .await?;

    let metrics_addr: Option<SocketAddr> = std::env::var("METRICS_ADDR")
        .ok()
        .map(|s| {
            s.parse::<bool>()
                .map(|b| b.then(|| SocketAddr::from(([0, 0, 0, 0], 8001))))
                .or_else(|_| s.parse::<SocketAddr>().map(Some))
        })
        .transpose()?
        .flatten();

    let profiling_addr: Option<SocketAddr> = std::env::var("PROFILING_ADDR")
        .ok()
        .map(|s| {
            s.parse::<bool>()
                .map(|b| b.then(|| SocketAddr::from(([0, 0, 0, 0], 4001))))
                .or_else(|_| s.parse::<SocketAddr>().map(Some))
        })
        .transpose()?
        .flatten();

    let (tx, rx) = tokio::sync::broadcast::channel::<()>(3);
    let shutdown_signal = windmill_common::shutdown_signal(tx);

    let base_internal_url =
        std::env::var("BASE_INTERNAL_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost".to_string());

    let timeout = std::env::var("TIMEOUT")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(windmill_common::DEFAULT_TIMEOUT);

    let workers_f = async {
        let sleep_queue = std::env::var("SLEEP_QUEUE")
            .ok()
            .and_then(|x| x.parse::<u64>().ok())
            .unwrap_or(windmill_common::DEFAULT_SLEEP_QUEUE);
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
                     {base_url}, SLEEP_QUEUE: {sleep_queue}, TIMEOUT: \
                     {timeout}, KEEP_JOB_DIR: {keep_job_dir}"
        );
        let instance_name = rd_string(5);

        let ip = windmill_common::external_ip::get_ip()
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = e.to_string(), "failed to get external IP");
                "unretrievable IP".to_string()
            });
        let worker_name = format!("dt-worker-{}-{}", &instance_name, rd_string(5));
        windmill_worker::run_worker(
            &db.clone(),
            timeout,
            &instance_name,
            worker_name,
            1,
            1,
            &ip,
            sleep_queue,
            windmill_worker::WorkerConfig {
                disable_nsjail,
                disable_nuser,
                base_internal_url,
                base_url,
                keep_job_dir,
            },
            rx.resubscribe(),
        )
        .await;
        Ok(()) as anyhow::Result<()>
    };

    let metrics_f = async {
        match metrics_addr {
            Some(addr) => windmill_common::serve_metrics(addr, rx.resubscribe())
                .await
                .map_err(anyhow::Error::from),
            None => Ok(()),
        }
    };

    let profiling_f = async {
        match profiling_addr {
            Some(addr) => windmill_common::serve_profiling(addr, rx.resubscribe())
                .await
                .map_err(anyhow::Error::from),
            None => Ok(()),
        }
    };

    futures::try_join!(shutdown_signal, workers_f, metrics_f, profiling_f)?;

    Ok(())
}
