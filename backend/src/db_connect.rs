use windmill_common::{
    error::{self, Error},
    get_database_url, DatabaseUrl,
};

pub const DEFAULT_MAX_CONNECTIONS_SERVER: u32 = 50;
pub const DEFAULT_MAX_CONNECTIONS_WORKER: u32 = 5;
pub const DEFAULT_MAX_CONNECTIONS_INDEXER: u32 = 5;

pub async fn initial_connection() -> Result<sqlx::Pool<sqlx::Postgres>, error::Error> {
    let connect_options = get_database_url().await?.connect_options().await?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with(connect_options)
        .await
        .map_err(|err| Error::ConnectingToDatabase(err.to_string()))
}

pub async fn connect_db(
    server_mode: bool,
    indexer_mode: bool,
    worker_mode: bool,
    #[cfg(feature = "private")] mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<sqlx::Pool<sqlx::Postgres>> {
    use anyhow::Context;

    let database_url = get_database_url().await?;

    let max_connections = match std::env::var("DATABASE_CONNECTIONS") {
        Ok(n) => n.parse::<u32>().context("invalid DATABASE_CONNECTIONS")?,
        Err(_) => {
            if server_mode {
                DEFAULT_MAX_CONNECTIONS_SERVER
            } else if indexer_mode {
                DEFAULT_MAX_CONNECTIONS_INDEXER
            } else {
                DEFAULT_MAX_CONNECTIONS_WORKER
                    + std::env::var("NUM_WORKERS")
                        .ok()
                        .map(|x| x.parse().ok())
                        .flatten()
                        .unwrap_or(1)
                    - 1
            }
        }
    };

    let pool = connect(database_url.clone(), max_connections, worker_mode).await?;
    #[cfg(all(feature = "enterprise", feature = "private"))]
    let pool2 = pool.clone();
    #[cfg(all(feature = "enterprise", feature = "private"))]
    if let DatabaseUrl::IamRds(database_url) = database_url {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = killpill_rx.recv() => {
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                        let needs_refresh = {
                            let read_guard = database_url.read().await;
                            read_guard.needs_refresh()
                        };
                        if needs_refresh {
                            let new_url = tokio::time::timeout(std::time::Duration::from_secs(10), get_database_url()).await;
                            match new_url {
                                Ok(Ok(new_url)) => {
                                    match new_url.connect_options().await {
                                        Ok(connect_options) => {
                                            pool2.set_connect_options(connect_options);
                                            tracing::info!("Refreshed IAM RDS URL successfully");
                                        }
                                        Err(e) => {
                                            tracing::error!("Error getting IAM RDS connect options, retrying in 10s: {}", e);
                                            continue;
                                        }
                                    }
                                }
                                Ok(Err(e)) => {
                                    tracing::error!("Error refreshing IAM RDS URL, trying again in 10s: {}", e);
                                    continue;
                                }
                                Err(e) => {
                                    tracing::error!("Timeout after 10s refreshing IAM RDS URL, trying again in 10 seconds: {}", e);
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    Ok(pool)
}

pub async fn connect(
    database_url: DatabaseUrl,
    max_connections: u32,
    worker_mode: bool,
) -> Result<sqlx::Pool<sqlx::Postgres>, error::Error> {
    use sqlx::Executor;
    use std::time::Duration;
    let mut pool_options = sqlx::postgres::PgPoolOptions::new()
        .min_connections((max_connections / 5).clamp(1, max_connections))
        .max_connections(max_connections)
        .max_lifetime(Duration::from_secs(30 * 60)); // 30 mins
    if worker_mode {
        pool_options = pool_options.idle_timeout(Duration::from_secs(60));
    }
    pool_options
        .after_connect(move |conn, _| {
            if worker_mode {
                Box::pin(async move {
                    if let Err(e) = conn
                        .execute(
                            r#"
        SET enable_seqscan = OFF;
        SET statement_timeout = '5min';
        SET idle_in_transaction_session_timeout = '10min';
        SET tcp_keepalives_idle = 300;
        SET tcp_keepalives_interval = 60;
        SET tcp_keepalives_count = 10;"#,
                        )
                        .await
                    {
                        tracing::error!("Error setting postgres settings: {}", e);
                    }
                    Ok(())
                })
            } else {
                Box::pin(async move {
                    if let Err(e) = conn
                        .execute(
                            r#"
        SET statement_timeout = '5min';
        SET idle_in_transaction_session_timeout = '10min';
        SET tcp_keepalives_idle = 300;
        SET tcp_keepalives_interval = 60;
        SET tcp_keepalives_count = 10;"#,
                        )
                        .await
                    {
                        tracing::error!("Error setting postgres settings: {}", e);
                    }
                    Ok(())
                })
            }
        })
        .connect_with(
            database_url
                .connect_options()
                .await?
                .statement_cache_capacity(400),
        )
        .await
        .map_err(|err| Error::ConnectingToDatabase(err.to_string()))
}
