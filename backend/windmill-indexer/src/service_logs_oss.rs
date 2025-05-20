use anyhow::anyhow;
use sqlx::{Pool, Postgres};
use windmill_common::error::Error;
use windmill_common::KillpillSender;
#[derive(Clone)]
pub struct ServiceLogIndexReader;

#[derive(Clone)]
pub struct ServiceLogIndexWriter;

pub async fn init_index(
    _db: &Pool<Postgres>,
    mut _killpill_tx: KillpillSender,
) -> Result<(ServiceLogIndexReader, ServiceLogIndexWriter), Error> {
    Err(anyhow!("Cannot initialize index: not in EE").into())
}

pub async fn run_indexer(
    _db: Pool<Postgres>,
    mut _index_writer: ServiceLogIndexWriter,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Error> {
    tracing::error!("Cannot run indexer: not in EE");
    Err(anyhow!("Cannot run indexer: not in EE").into())
}