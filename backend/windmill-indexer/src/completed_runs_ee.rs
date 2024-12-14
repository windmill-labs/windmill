use anyhow::anyhow;
use sqlx::{Pool, Postgres};
use windmill_common::error::Error;

#[derive(Clone)]
pub struct IndexReader;

#[derive(Clone)]
pub struct IndexWriter;

pub async fn init_index(_db: &Pool<Postgres>) -> Result<(IndexReader, IndexWriter), Error> {
    Err(anyhow!("Cannot initialize index: not in EE").into())
}

pub async fn run_indexer(
    _db: Pool<Postgres>,
    mut _index_writer: IndexWriter,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Error> {
    tracing::error!("Cannot run indexer: not in EE");
    Err(anyhow!("Cannot run indexer: not in EE").into())
}
