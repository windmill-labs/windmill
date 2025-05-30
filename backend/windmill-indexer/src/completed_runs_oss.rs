#[cfg(not(feature = "private"))]
use anyhow::anyhow;
#[cfg(not(feature = "private"))]
use sqlx::{Pool, Postgres};
#[cfg(not(feature = "private"))]
use windmill_common::error::Error;

#[cfg(not(feature = "private"))]
#[derive(Clone)]
pub struct IndexReader;

#[cfg(not(feature = "private"))]
#[derive(Clone)]
pub struct IndexWriter;

#[cfg(not(feature = "private"))]
pub async fn init_index(_db: &Pool<Postgres>) -> Result<(IndexReader, IndexWriter), Error> {
    Err(anyhow!("Cannot initialize index: not in EE").into())
}

#[cfg(not(feature = "private"))]
pub async fn run_indexer(
    _db: Pool<Postgres>,
    mut _index_writer: IndexWriter,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Error> {
    tracing::error!("Cannot run indexer: not in EE");
    Err(anyhow!("Cannot run indexer: not in EE").into())
}
