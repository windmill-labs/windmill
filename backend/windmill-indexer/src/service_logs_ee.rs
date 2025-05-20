#[cfg(feature = "private")]
use crate::service_logs_ee;

use anyhow::anyhow;
use sqlx::{Pool, Postgres};
use windmill_common::error::Error;
use windmill_common::KillpillSender;

#[derive(Clone)]
pub struct ServiceLogIndexReader; // Stays in OSS

#[derive(Clone)]
pub struct ServiceLogIndexWriter; // Stays in OSS

pub async fn init_index(
    db: &Pool<Postgres>,
    mut killpill_tx: KillpillSender,
) -> Result<(ServiceLogIndexReader, ServiceLogIndexWriter), Error> {
    #[cfg(feature = "private")]
    {
        return service_logs_ee::init_index(db, killpill_tx).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (db, killpill_tx);
        Err(anyhow!("Cannot initialize index: not in EE").into())
    }
}

pub async fn run_indexer(
    db: Pool<Postgres>,
    mut index_writer: ServiceLogIndexWriter,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Error> {
    #[cfg(feature = "private")]
    {
        return service_logs_ee::run_indexer(db, index_writer, killpill_rx).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (db, index_writer, killpill_rx);
        tracing::error!("Cannot run indexer: not in EE");
        Err(anyhow!("Cannot run indexer: not in EE").into())
    }
}
