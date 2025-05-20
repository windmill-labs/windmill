#[cfg(feature = "private")]
use crate::completed_runs_ee;

use anyhow::anyhow;
use sqlx::{Pool, Postgres};
use windmill_common::error::Error;

#[derive(Clone)]
pub struct IndexReader;

#[derive(Clone)]
pub struct IndexWriter;

pub async fn init_index(db: &Pool<Postgres>) -> Result<(IndexReader, IndexWriter), Error> {
    #[cfg(feature = "private")]
    {
        return completed_runs_ee::init_index(db).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = db;
        Err(anyhow!("Cannot initialize index: not in EE").into())
    }
}

pub async fn run_indexer(
    db: Pool<Postgres>,
    mut index_writer: IndexWriter,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), Error> {
    #[cfg(feature = "private")]
    {
        return completed_runs_ee::run_indexer(db, index_writer, killpill_rx).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (db, index_writer, killpill_rx);
        tracing::error!("Cannot run indexer: not in EE");
        Err(anyhow!("Cannot run indexer: not in EE").into())
    }
}
