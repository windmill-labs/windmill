use sqlx::{Pool, Postgres};
use windmill_common::error::Error;

#[derive(Clone)]
pub struct IndexReader;

#[derive(Clone)]
pub struct IndexWriter;

pub fn init_index() -> Result<(IndexReader, IndexWriter), Error> {
    panic!("Cannot initialize index: not in EE")
}

pub async fn run_indexer(
    _db: Pool<Postgres>,
    mut _index_writer: IndexWriter,
    mut _killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    panic!("Cannot run indexer: not in EE");
}
