/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::net::SocketAddr;

use anyhow::Ok;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    windmill_common::tracing_init::initialize_tracing();

    let db = windmill_common::connect_db(true).await?;

    let num_workers = std::env::var("NUM_WORKERS")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(windmill_common::DEFAULT_NUM_WORKERS as i32);

    let metrics_addr: Option<SocketAddr> = std::env::var("METRICS_ADDR")
        .ok()
        .map(|s| {
            s.parse::<bool>()
                .map(|b| b.then(|| SocketAddr::from(([0, 0, 0, 0], 8001))))
                .or_else(|_| s.parse::<SocketAddr>().map(Some))
        })
        .transpose()?
        .flatten();

    let server_mode = !std::env::var("DISABLE_SERVER")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false);

    if server_mode {
        windmill_api::migrate_db(&db).await?;
    }

    let (tx, rx) = tokio::sync::broadcast::channel::<()>(3);
    let shutdown_signal = windmill_common::shutdown_signal(tx);

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost".to_string());

    if server_mode || num_workers > 0 {
        let addr = SocketAddr::from(([0, 0, 0, 0], 8000));

        let server_f = async {
            if server_mode {
                windmill_api::run_server(db.clone(), addr, base_url, rx.resubscribe()).await?;
            }
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

        futures::try_join!(shutdown_signal, server_f, metrics_f)?;
    }
    Ok(())
}
