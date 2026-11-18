use std::sync::{atomic::Ordering, Mutex, OnceLock};

use windmill_common::{
    otel_oss::{
        otel_add_worker_cpu, otel_record_job_memory_peak, otel_set_worker_memory,
        otel_set_worker_occupancy_rate,
    },
    worker::{get_cpu_stat, get_memory, CpuStat, WORKER_GROUP},
    OTEL_METRICS_ENABLED,
};

use crate::common::OccupancyResult;

#[cfg(feature = "prometheus")]
struct PromMetrics {
    memory_usage: prometheus::IntGaugeVec,
    windmill_memory_usage: prometheus::IntGaugeVec,
    memory_limit: prometheus::IntGaugeVec,
    cpu_usage: prometheus::CounterVec,
    cpu_periods: prometheus::IntCounterVec,
    cpu_throttled_periods: prometheus::IntCounterVec,
    cpu_throttled_seconds: prometheus::CounterVec,
    occupancy_rate: prometheus::GaugeVec,
    job_memory_peak: prometheus::HistogramVec,
}

#[cfg(feature = "prometheus")]
lazy_static::lazy_static! {
    static ref PROM_METRICS: Option<PromMetrics> =
        if windmill_common::METRICS_ENABLED.load(Ordering::Relaxed) {
            const WORKER: &[&str] = &["name", "worker_group"];
            Some(PromMetrics {
                memory_usage: prometheus::register_int_gauge_vec!(
                    "worker_memory_usage_bytes",
                    "Memory used by the worker's container, page cache excluded",
                    WORKER
                )
                .unwrap(),
                windmill_memory_usage: prometheus::register_int_gauge_vec!(
                    "worker_windmill_memory_usage_bytes",
                    "Resident memory of the worker process itself",
                    WORKER
                )
                .unwrap(),
                memory_limit: prometheus::register_int_gauge_vec!(
                    "worker_memory_limit_bytes",
                    "Memory limit of the worker's container",
                    WORKER
                )
                .unwrap(),
                cpu_usage: prometheus::register_counter_vec!(
                    "worker_cpu_usage_seconds_total",
                    "CPU time consumed by the worker's container",
                    WORKER
                )
                .unwrap(),
                cpu_periods: prometheus::register_int_counter_vec!(
                    "worker_cpu_periods_total",
                    "CPU enforcement periods elapsed for the worker's container",
                    WORKER
                )
                .unwrap(),
                cpu_throttled_periods: prometheus::register_int_counter_vec!(
                    "worker_cpu_throttled_periods_total",
                    "CPU enforcement periods in which the worker's container was throttled",
                    WORKER
                )
                .unwrap(),
                cpu_throttled_seconds: prometheus::register_counter_vec!(
                    "worker_cpu_throttled_seconds_total",
                    "Time the worker's container spent throttled by its CPU limit",
                    WORKER
                )
                .unwrap(),
                occupancy_rate: prometheus::register_gauge_vec!(
                    "worker_occupancy_rate",
                    "Share of time the worker spent executing jobs, over the window",
                    &["name", "worker_group", "window"]
                )
                .unwrap(),
                job_memory_peak: prometheus::register_histogram_vec!(
                    "worker_job_memory_peak_bytes",
                    "Peak memory of a job's main process",
                    &["tag"],
                    windmill_common::worker::JOB_MEMORY_PEAK_BUCKETS.to_vec()
                )
                .unwrap(),
            })
        } else {
            None
        };
}

fn metrics_enabled() -> bool {
    #[cfg(feature = "prometheus")]
    if PROM_METRICS.is_some() {
        return true;
    }
    OTEL_METRICS_ENABLED.load(Ordering::Relaxed)
}

// Memory and CPU are read from the cgroup, which every worker of a process shares
// (native mode runs several). Only the first worker to report exports them, so that
// summing the series over workers does not count one container several times.
static CONTAINER_SERIES_OWNER: OnceLock<String> = OnceLock::new();
static MEMORY_LIMIT: Mutex<Option<Option<i64>>> = Mutex::new(None);

/// Replaces the exported memory limit, for the ping that re-reads the cgroup limits.
pub(crate) fn set_memory_limit(limit: Option<i64>) {
    *MEMORY_LIMIT.lock().unwrap() = Some(limit);
}
static LAST_CPU_STAT: Mutex<CpuStat> =
    Mutex::new(CpuStat { usage_usec: 0, nr_periods: 0, nr_throttled: 0, throttled_usec: 0 });

/// Exports the worker's resource readings. Meant for the ping ticks, which have already
/// read memory usage and occupancy; the only extra work is the cgroup `cpu.stat` read.
pub(crate) fn record_worker_resources(
    worker_name: &str,
    memory_usage: Option<i64>,
    wm_memory_usage: Option<i64>,
    occupancy: Option<&OccupancyResult>,
) {
    if !metrics_enabled() {
        return;
    }
    let worker_group = WORKER_GROUP.as_str();

    if let Some(occupancy) = occupancy {
        let windows = [
            ("total", Some(occupancy.occupancy_rate)),
            ("15s", occupancy.occupancy_rate_15s),
            ("5m", occupancy.occupancy_rate_5m),
            ("30m", occupancy.occupancy_rate_30m),
        ];
        for (window, rate) in windows {
            let Some(rate) = rate.filter(|r| r.is_finite()) else {
                continue;
            };
            #[cfg(feature = "prometheus")]
            if let Some(m) = PROM_METRICS.as_ref() {
                m.occupancy_rate
                    .with_label_values(&[worker_name, worker_group, window])
                    .set(rate as f64);
            }
            otel_set_worker_occupancy_rate(worker_name, worker_group, window, rate as f64);
        }
    }

    if CONTAINER_SERIES_OWNER.get_or_init(|| worker_name.to_string()) != worker_name {
        return;
    }

    let memory_limit = *MEMORY_LIMIT.lock().unwrap().get_or_insert_with(get_memory);
    #[cfg(feature = "prometheus")]
    if let Some(m) = PROM_METRICS.as_ref() {
        let labels = &[worker_name, worker_group];
        if let Some(v) = memory_usage {
            m.memory_usage.with_label_values(labels).set(v);
        }
        if let Some(v) = wm_memory_usage {
            m.windmill_memory_usage.with_label_values(labels).set(v);
        }
        if let Some(v) = memory_limit {
            m.memory_limit.with_label_values(labels).set(v);
        }
    }
    otel_set_worker_memory(
        worker_name,
        worker_group,
        memory_usage,
        wm_memory_usage,
        memory_limit,
    );

    let Some(stat) = get_cpu_stat() else {
        return;
    };
    let delta = {
        let mut last = LAST_CPU_STAT.lock().unwrap();
        // A reading below the previous one means the cgroup changed under the process:
        // restart from it rather than exporting a negative increase.
        let delta = if stat.usage_usec < last.usage_usec {
            CpuStat::default()
        } else {
            CpuStat {
                usage_usec: stat.usage_usec - last.usage_usec,
                nr_periods: stat.nr_periods.saturating_sub(last.nr_periods),
                nr_throttled: stat.nr_throttled.saturating_sub(last.nr_throttled),
                throttled_usec: stat.throttled_usec.saturating_sub(last.throttled_usec),
            }
        };
        *last = stat;
        delta
    };
    let usage_secs = delta.usage_usec as f64 / 1e6;
    let throttled_secs = delta.throttled_usec as f64 / 1e6;
    #[cfg(feature = "prometheus")]
    if let Some(m) = PROM_METRICS.as_ref() {
        let labels = &[worker_name, worker_group];
        m.cpu_usage.with_label_values(labels).inc_by(usage_secs);
        m.cpu_periods
            .with_label_values(labels)
            .inc_by(delta.nr_periods);
        m.cpu_throttled_periods
            .with_label_values(labels)
            .inc_by(delta.nr_throttled);
        m.cpu_throttled_seconds
            .with_label_values(labels)
            .inc_by(throttled_secs);
    }
    otel_add_worker_cpu(
        worker_name,
        worker_group,
        usage_secs,
        delta.nr_periods,
        delta.nr_throttled,
        throttled_secs,
    );
}

/// `mem_peak_kb` is the job's `mem_peak`: non-positive when the executor had no process
/// to sample or the reading failed, and those are not observations.
pub(crate) fn record_job_memory_peak(tag: &str, mem_peak_kb: i32) {
    if mem_peak_kb <= 0 {
        return;
    }
    let bytes = mem_peak_kb as u64 * 1024;
    #[cfg(feature = "prometheus")]
    if let Some(m) = PROM_METRICS.as_ref() {
        m.job_memory_peak
            .with_label_values(&[tag])
            .observe(bytes as f64);
    }
    otel_record_job_memory_peak(tag, bytes);
}
