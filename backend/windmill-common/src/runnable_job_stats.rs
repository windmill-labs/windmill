//! Hourly resource rollup per runnable and worker group (`runnable_job_stats`).
//!
//! Workers accumulate completed jobs in memory and upsert periodically, so that the
//! "heaviest scripts" view never has to aggregate `v2_job_completed`.
//!
//! What a row leaves out: executors that run in the worker process (SQL, native
//! TypeScript, ...) have no memory or CPU reading, memory is the peak of the job's main
//! process only, and CPU counts that process and the children it waited for, and nothing
//! for a job that was killed.
//!
//! Nothing here checks authorization: these functions write and prune every workspace's
//! rows, and are for the worker's completion path and the server's monitor only. Reads
//! go through the devops-gated API handler.

use std::{collections::HashMap, sync::Mutex};

use sqlx::{Pool, Postgres};

use crate::{jobs::JobKind, worker_group_job_stats::get_current_hour};

/// Every run without a stable path is counted under this one, per workspace and worker
/// group: previews and other ad-hoc runs carry caller-chosen paths that would otherwise
/// grow the table without bound.
pub const ADHOC_RUNNABLE_PATH: &str = "<adhoc>";

/// Also the furthest back the API reads.
pub const RETENTION_DAYS: i64 = 30;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct RunnableStats {
    job_count: i32,
    total_duration_ms: i64,
    max_memory_peak: i32,
    sum_memory_peak: i64,
    memory_sample_count: i32,
    total_cpu_ms: i64,
}

impl RunnableStats {
    /// `mem_peak` is in kB and non-positive when the job had no reading.
    fn add_job(&mut self, duration_ms: i64, mem_peak: i32, cpu_time_ms: Option<i64>) {
        self.job_count += 1;
        self.total_duration_ms += duration_ms;
        if mem_peak > 0 {
            self.max_memory_peak = self.max_memory_peak.max(mem_peak);
            self.sum_memory_peak += mem_peak as i64;
            self.memory_sample_count += 1;
        }
        self.total_cpu_ms += cpu_time_ms.unwrap_or(0).max(0);
    }

    fn merge(&mut self, other: &RunnableStats) {
        self.job_count += other.job_count;
        self.total_duration_ms += other.total_duration_ms;
        self.max_memory_peak = self.max_memory_peak.max(other.max_memory_peak);
        self.sum_memory_peak += other.sum_memory_peak;
        self.memory_sample_count += other.memory_sample_count;
        self.total_cpu_ms += other.total_cpu_ms;
    }
}

/// (hour, workspace_id, runnable_path, worker_group)
type StatsKey = (i64, String, String, String);

lazy_static::lazy_static! {
    static ref STATS: Mutex<HashMap<StatsKey, RunnableStats>> = Mutex::new(HashMap::new());
}

/// The path a job is rolled up under: its own for the kinds whose path names something
/// deployed (a flow step's is `<flow path>/<step id>`), the ad-hoc bucket otherwise.
pub fn stats_path(kind: JobKind, runnable_path: Option<&str>) -> &str {
    match (kind, runnable_path) {
        (
            JobKind::Script
            | JobKind::Script_Hub
            | JobKind::FlowScript
            | JobKind::FlowNode
            | JobKind::AIAgent,
            Some(path),
        ) if !path.is_empty() => path,
        _ => ADHOC_RUNNABLE_PATH,
    }
}

pub fn accumulate_runnable_job_stats(
    workspace_id: &str,
    kind: JobKind,
    runnable_path: Option<&str>,
    worker_group: &str,
    duration_ms: i64,
    mem_peak: i32,
    cpu_time_ms: Option<i64>,
) {
    let key = (
        get_current_hour(),
        workspace_id.to_string(),
        stats_path(kind, runnable_path).to_string(),
        worker_group.to_string(),
    );
    STATS
        .lock()
        .unwrap()
        .entry(key)
        .or_default()
        .add_job(duration_ms, mem_peak, cpu_time_ms);
}

pub async fn flush_runnable_job_stats(db: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    let drained: Vec<(StatsKey, RunnableStats)> = {
        let mut stats = STATS.lock().unwrap();
        if stats.is_empty() {
            return Ok(());
        }
        stats.drain().collect()
    };

    let mut hours = Vec::with_capacity(drained.len());
    let mut workspace_ids = Vec::with_capacity(drained.len());
    let mut runnable_paths = Vec::with_capacity(drained.len());
    let mut worker_groups = Vec::with_capacity(drained.len());
    let mut job_counts = Vec::with_capacity(drained.len());
    let mut durations = Vec::with_capacity(drained.len());
    let mut max_peaks = Vec::with_capacity(drained.len());
    let mut sum_peaks = Vec::with_capacity(drained.len());
    let mut sample_counts = Vec::with_capacity(drained.len());
    let mut cpus = Vec::with_capacity(drained.len());
    for ((hour, workspace_id, runnable_path, worker_group), s) in &drained {
        hours.push(*hour);
        workspace_ids.push(workspace_id.clone());
        runnable_paths.push(runnable_path.clone());
        worker_groups.push(worker_group.clone());
        job_counts.push(s.job_count);
        durations.push(s.total_duration_ms);
        max_peaks.push(s.max_memory_peak);
        sum_peaks.push(s.sum_memory_peak);
        sample_counts.push(s.memory_sample_count);
        cpus.push(s.total_cpu_ms);
    }

    // The join drops rows of a workspace deleted since its jobs ran: without it their
    // foreign key violation would fail the whole batch, and every retry after it.
    let res = sqlx::query!(
        r#"
        INSERT INTO runnable_job_stats
            (hour, workspace_id, runnable_path, worker_group, job_count, total_duration_ms,
             max_memory_peak, sum_memory_peak, memory_sample_count, total_cpu_ms)
        SELECT s.hour, s.workspace_id, s.runnable_path, s.worker_group, s.job_count,
               s.total_duration_ms, s.max_memory_peak, s.sum_memory_peak,
               s.memory_sample_count, s.total_cpu_ms
        FROM UNNEST($1::bigint[], $2::text[], $3::text[], $4::text[], $5::int[], $6::bigint[],
                    $7::int[], $8::bigint[], $9::int[], $10::bigint[])
            AS s(hour, workspace_id, runnable_path, worker_group, job_count, total_duration_ms,
                 max_memory_peak, sum_memory_peak, memory_sample_count, total_cpu_ms)
        JOIN workspace w ON w.id = s.workspace_id
        ON CONFLICT (hour, workspace_id, runnable_path, worker_group) DO UPDATE SET
            job_count = runnable_job_stats.job_count + EXCLUDED.job_count,
            total_duration_ms = runnable_job_stats.total_duration_ms + EXCLUDED.total_duration_ms,
            max_memory_peak = GREATEST(runnable_job_stats.max_memory_peak, EXCLUDED.max_memory_peak),
            sum_memory_peak = runnable_job_stats.sum_memory_peak + EXCLUDED.sum_memory_peak,
            memory_sample_count = runnable_job_stats.memory_sample_count + EXCLUDED.memory_sample_count,
            total_cpu_ms = runnable_job_stats.total_cpu_ms + EXCLUDED.total_cpu_ms
        "#,
        &hours,
        &workspace_ids,
        &runnable_paths,
        &worker_groups,
        &job_counts,
        &durations,
        &max_peaks,
        &sum_peaks,
        &sample_counts,
        &cpus,
    )
    .execute(db)
    .await;

    if let Err(e) = res {
        let mut stats = STATS.lock().unwrap();
        for (key, s) in drained {
            stats.entry(key).or_default().merge(&s);
        }
        return Err(e);
    }
    Ok(())
}

pub async fn cleanup_old_runnable_job_stats(
    db: &Pool<Postgres>,
    retention_days: i64,
) -> Result<u64, sqlx::Error> {
    let cutoff = get_current_hour() - retention_days * 24 * 3600;
    let result = sqlx::query!("DELETE FROM runnable_job_stats WHERE hour < $1", cutoff)
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_deployed_runnables_keep_their_path() {
        assert_eq!(
            stats_path(JobKind::Script, Some("f/etl/load")),
            "f/etl/load"
        );
        assert_eq!(
            stats_path(JobKind::FlowScript, Some("f/etl/flow/a")),
            "f/etl/flow/a"
        );
        // A preview carries the path being edited, which is not a stable identity.
        assert_eq!(
            stats_path(JobKind::Preview, Some("f/etl/load")),
            ADHOC_RUNNABLE_PATH
        );
        assert_eq!(
            stats_path(JobKind::FlowPreview, Some("f/etl/flow")),
            ADHOC_RUNNABLE_PATH
        );
        assert_eq!(
            stats_path(JobKind::Dependencies, Some("f/etl/load")),
            ADHOC_RUNNABLE_PATH
        );
        assert_eq!(stats_path(JobKind::Script, None), ADHOC_RUNNABLE_PATH);
    }

    #[test]
    fn jobs_without_a_memory_reading_do_not_dilute_it() {
        let mut s = RunnableStats::default();
        s.add_job(1000, 2048, Some(700));
        s.add_job(500, -3, None);
        s.add_job(250, 0, Some(-5));
        s.add_job(250, 1024, Some(50));
        assert_eq!(
            s,
            RunnableStats {
                job_count: 4,
                total_duration_ms: 2000,
                max_memory_peak: 2048,
                sum_memory_peak: 3072,
                memory_sample_count: 2,
                total_cpu_ms: 750,
            }
        );
    }
}
