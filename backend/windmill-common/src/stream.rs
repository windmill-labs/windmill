use futures::{Stream, TryStreamExt};

pub async fn collect_stream_with_limits<T, S>(
    mut row_stream: S,
    max_size_megabytes: usize,
    estimate_row_size: fn(&T) -> anyhow::Result<usize>,
    cutoff_time: chrono::TimeDelta,
) -> Result<Vec<T>, anyhow::Error>
where
    S: Stream<Item = Result<T, sqlx::Error>> + Unpin,
{
    let started_time = chrono::Utc::now();
    let mut results = Vec::new();
    let mut total_size = 0;
    let max_size_bytes = max_size_megabytes * 1024 * 1024;

    while let Some(row) = row_stream.try_next().await? {
        let row_size = estimate_row_size(&row)?;

        results.push(row);
        total_size += row_size;
        if total_size + row_size > max_size_bytes {
            tracing::info!(
                "Stream was cutoff at {} elements because the collected size reached the treshold of {} MB",
                results.len(),
                max_size_megabytes
            );
            break;
        }

        if chrono::Utc::now() - started_time > cutoff_time {
            tracing::info!(
                "Stream was cutoff at {} elements because collecting it was taking longer than {}s",
                results.len(),
                cutoff_time.num_seconds()
            );
            break;
        }
    }

    Ok(results)
}
