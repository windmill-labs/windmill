use anyhow::anyhow;
use const_format::concatcp;
use gethostname::gethostname;
use std::{str::FromStr, sync::{Arc, Mutex}};
use tantivy::{directory::MmapDirectory, schema::Schema};
use windmill_common::{
    indexer::TantivyIndexerSettings, utils::{rd_string, Mode}, worker::TMP_DIR,
};
use windmill_indexer::{
    completed_runs_ee::{
        fill_schema, IndexWriter, IndexedJobTrackerData, INDEXED_JOB_TRACKER_FILENAME,
    },
    indexer_ee::{clean_dir_all_if_exists, IndexedDBTracker, S3BackedMMapDirectory},
};

lazy_static::lazy_static! {
    static ref HOSTNAME :String = std::env::var("FORCE_HOSTNAME").unwrap_or_else(|_| {
        gethostname()
            .to_str()
            .map(|x| x.to_string())
            .unwrap_or_else(|| rd_string(5))
    });
}

const S3_JOB_INDEX_PATH: &str = "search-dbg/completed_job_index";
const JOB_INDEX_DIRECTORY_PATH: &str = concatcp!(TMP_DIR, "/search-dbg/completed_jobs_index");

fn init_index() -> anyhow::Result<IndexWriter> {
    let mut schema_builder = Schema::builder();
    let fields = fill_schema(&mut schema_builder);
    let schema = schema_builder.build();

    std::fs::create_dir_all(JOB_INDEX_DIRECTORY_PATH)?;

    let idx_tantivy_dir = MmapDirectory::open(JOB_INDEX_DIRECTORY_PATH)
        .map_err(|e| anyhow!("Failed to create MMapDirectory: {e}"))?;

    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    let idx_tantivy_dir = S3BackedMMapDirectory::new(
        std::path::PathBuf::from_str(JOB_INDEX_DIRECTORY_PATH).map_err(|e| anyhow!("{}", e))?,
        idx_tantivy_dir,
        S3_JOB_INDEX_PATH.to_string(),
        tokio::runtime::Handle::current(),
    );

    clean_dir_all_if_exists(JOB_INDEX_DIRECTORY_PATH)?;
    let index = tantivy::Index::open_or_create(idx_tantivy_dir, schema)
        .map_err(|e| anyhow!("Failed to open index {e}"))?;

    let job_tracker: IndexedDBTracker<IndexedJobTrackerData> = IndexedDBTracker::new(
        std::path::PathBuf::from_str(INDEXED_JOB_TRACKER_FILENAME)
            .map_err(|e| anyhow!("Failed to read {}: {e}", INDEXED_JOB_TRACKER_FILENAME))?,
        index.directory().clone(),
    );

    Ok(IndexWriter { index, fields, job_tracker })
}
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let hostname = HOSTNAME.to_owned();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }

    let (_guard, _jaja) = windmill_common::tracing_init::initialize_tracing(&hostname, &Mode::Indexer, "");

    let db = windmill_common::initial_connection().await?;

    let mut idx_writer = init_index()?;

    let max_created_at = std::env::var("INDEX_STARTING_FROM")
        .ok()
        .map(|start_from| chrono::DateTime::parse_from_rfc3339(&start_from))
        .transpose()
        .map_err(|e| anyhow!("Failed to parse date for INDEX_STARTING_FROM"))?;

    if let Some(max_created_at) = max_created_at {
        tracing::info!("Date to start indexing aka INDEX_STARTING_FROM is set to {:?}", max_created_at);
        idx_writer.job_tracker.data =
            Some(IndexedJobTrackerData { max_created_at: max_created_at.into(), queued_uuids_remainder: vec![] });
    } else {
        tracing::info!("INDEX_STARTING_FROM env var not found, indexing from the very first job");
    }

    let writer = idx_writer
        .index
        .writer(300 * 1024 * 1024)?;

    let w = Arc::new(Mutex::new(writer));
    idx_writer.refresh_jobs(w.clone(), &db, &TantivyIndexerSettings::default()).await?;

    tokio::task::spawn_blocking(move || {
        if let Err(e) = Arc::try_unwrap(w)
            .map_err(|_| {
                anyhow!(
                    "There was more than 1 refrence to the writer. This should not be possible."
                )
            })
            .unwrap()
            .into_inner()
            .unwrap()
            .wait_merging_threads()
        {
            tracing::error!("Error while waiting for index writer merging threads: {e}");
        }
    })
    .await?;

    tracing::info!("All merging threads completed, releasing lock");

    Ok(())
}

// let (index_reader, index_writer) = {
//     let mut indexer_rx = killpill_rx.resubscribe();
//
//     let (mut reader, mut writer) = (None, None);
//     tokio::select! {
//     _ = indexer_rx.recv() => {
//         tracing::info!("Received killpill, aborting index initialization");
//     },
//     res = windmill_indexer::completed_runs_oss::init_index(&db) => {
//             let res = res?;
//             reader = Some(res.0);
//             writer = Some(res.1);
//     }
//
//     }
//     (reader, writer)
// };
