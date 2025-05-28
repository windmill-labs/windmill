use sqlx::{Pool, Postgres};

pub struct ServiceLogIndexReader;
pub struct ServiceLogIndexWriter;

pub async fn init_index(
    _db: &Pool<Postgres>,
    _index_name: &str,
    _index_location: &str,
    _tantivy_buffer_size_mb: usize,
) -> Result<(ServiceLogIndexReader, ServiceLogIndexWriter), anyhow::Error> {
    crate::service_logs_ee::init_index(_db, _index_name, _index_location, _tantivy_buffer_size_mb).await
}

pub async fn run_indexer(
    _db: Pool<Postgres>,
    _writer: ServiceLogIndexWriter,
    _mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), anyhow::Error> {
    crate::service_logs_ee::run_indexer(_db, _writer, _mut killpill_rx).await
}