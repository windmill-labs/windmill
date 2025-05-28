use sqlx::{Pool, Postgres};

pub struct IndexReader;
pub struct IndexWriter;

pub async fn init_index(_db: &Pool<Postgres>) -> Result<(IndexReader, IndexWriter), anyhow::Error> {
    crate::completed_runs_ee::init_index(_db).await
}

pub async fn run_indexer(
    _db: Pool<Postgres>,
    _writer: IndexWriter,
    _mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), anyhow::Error> {
    crate::completed_runs_ee::run_indexer(_db, _writer, _mut killpill_rx).await
}