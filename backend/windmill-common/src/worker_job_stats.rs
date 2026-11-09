//! Per-worker job outcome counters, kept in the worker's memory and written to
//! `worker_ping.job_stats` with every ping.

use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};

pub const RECENT_JOBS_WINDOW: u32 = 100;
const WINDOW_HOURS: i64 = 24;

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct HourBucket {
    /// Unix timestamp divided by 3600.
    pub hour: i64,
    pub executed: u32,
    pub failed: u32,
}

/// What a worker persists. The API turns it into a [`JobStatsSummary`] relative to the
/// time of the request, so a worker that stopped pinging does not keep reporting
/// failures from more than 24h ago.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct WorkerJobStats {
    pub executed: u64,
    pub failed: u64,
    /// Oldest first, at most `WINDOW_HOURS` buckets, only hours with jobs.
    #[serde(default)]
    pub hours: VecDeque<HourBucket>,
    /// Outcome of the last `recent_len` jobs, most recent in bit 0, a set bit for a failure.
    /// A string because a JSON number loses precision past 2^53 in most readers.
    #[serde(default, with = "u128_as_string")]
    pub recent: u128,
    #[serde(default)]
    pub recent_len: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq)]
pub struct JobCounts {
    pub executed: u64,
    pub failed: u64,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq)]
pub struct JobStatsSummary {
    pub total: JobCounts,
    pub last_24h: JobCounts,
    pub last_100: JobCounts,
}

impl WorkerJobStats {
    pub fn record(&mut self, failed: bool, now_hour: i64) {
        self.executed += 1;
        self.failed += failed as u64;

        match self.hours.back_mut() {
            Some(b) if b.hour == now_hour => {
                b.executed += 1;
                b.failed += failed as u32;
            }
            _ => self.hours.push_back(HourBucket {
                hour: now_hour,
                executed: 1,
                failed: failed as u32,
            }),
        }
        self.prune(now_hour);

        self.recent = (self.recent << 1) | failed as u128;
        self.recent_len = (self.recent_len + 1).min(RECENT_JOBS_WINDOW);
    }

    fn prune(&mut self, now_hour: i64) {
        while self
            .hours
            .front()
            .is_some_and(|b| b.hour <= now_hour - WINDOW_HOURS)
        {
            self.hours.pop_front();
        }
    }

    pub fn summary(&self, now_hour: i64) -> JobStatsSummary {
        let last_24h = self
            .hours
            .iter()
            .filter(|b| b.hour > now_hour - WINDOW_HOURS)
            .fold(JobCounts::default(), |acc, b| JobCounts {
                executed: acc.executed + b.executed as u64,
                failed: acc.failed + b.failed as u64,
            });
        let len = self.recent_len.min(RECENT_JOBS_WINDOW);
        let mask = if len == 0 {
            0
        } else {
            u128::MAX >> (128 - len)
        };
        JobStatsSummary {
            total: JobCounts { executed: self.executed, failed: self.failed },
            last_24h,
            last_100: JobCounts {
                executed: len as u64,
                failed: (self.recent & mask).count_ones() as u64,
            },
        }
    }
}

pub fn current_hour() -> i64 {
    chrono::Utc::now().timestamp().div_euclid(3600)
}

/// Keyed by worker name: one process can run several workers, and the result processor
/// and the ping loop of a worker only share its name.
static WORKER_JOB_STATS: LazyLock<Mutex<HashMap<String, WorkerJobStats>>> =
    LazyLock::new(Default::default);

pub fn record_job_outcome(worker_name: &str, failed: bool) {
    let hour = current_hour();
    let mut map = WORKER_JOB_STATS.lock().unwrap_or_else(|e| e.into_inner());
    match map.get_mut(worker_name) {
        Some(s) => s.record(failed, hour),
        None => map
            .entry(worker_name.to_string())
            .or_default()
            .record(failed, hour),
    }
}

/// Seeds the counters from the row a restarted worker reclaims, so they survive restarts
/// under the same name as `jobs_executed` does.
pub fn restore_job_stats(worker_name: &str, stats: WorkerJobStats) {
    let mut map = WORKER_JOB_STATS.lock().unwrap_or_else(|e| e.into_inner());
    let entry = map.entry(worker_name.to_string()).or_default();
    // Jobs recorded before the restore (none in practice) are dropped rather than merged.
    *entry = stats;
}

pub fn job_stats_snapshot(worker_name: &str) -> Option<WorkerJobStats> {
    let mut map = WORKER_JOB_STATS.lock().unwrap_or_else(|e| e.into_inner());
    map.get_mut(worker_name).map(|s| {
        s.prune(current_hour());
        s.clone()
    })
}

mod u128_as_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &u128, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u128, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_roll_over() {
        let mut s = WorkerJobStats::default();
        // 150 jobs at hour 0, every third one failing: 50 failures in total.
        for i in 0..150 {
            s.record(i % 3 == 0, 0);
        }
        let sum = s.summary(0);
        assert_eq!(sum.total, JobCounts { executed: 150, failed: 50 });
        assert_eq!(sum.last_24h, JobCounts { executed: 150, failed: 50 });
        // jobs 50..150 hold i % 3 == 0 for 51, 54, ..., 147: 33 of them.
        assert_eq!(sum.last_100, JobCounts { executed: 100, failed: 33 });

        // 24 hours later the hour-0 bucket has left the window; the last 100 have not.
        s.record(true, 24);
        let sum = s.summary(24);
        assert_eq!(sum.total, JobCounts { executed: 151, failed: 51 });
        assert_eq!(sum.last_24h, JobCounts { executed: 1, failed: 1 });
        assert_eq!(sum.last_100.executed, 100);

        // Read later without new jobs: the stale bucket no longer counts.
        assert_eq!(s.summary(48).last_24h, JobCounts::default());

        let round_trip: WorkerJobStats =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(round_trip, s);
    }
}
