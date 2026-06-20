use reqwest::header::HeaderMap;
use uuid::Uuid;
use windmill_common::{
    agent_workers::QueueInitJob, worker::HttpClient, workspaces::DucklakeWithConnData,
};
use windmill_queue::{JobAndPerms, JobCompleted};

// ---- agent end-to-end profiling (gated behind AGENT_PROF=1, zero-cost when off) ----
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, Once, OnceLock};
use std::time::{Duration, Instant};

static PROF: OnceLock<bool> = OnceLock::new();
fn prof_on() -> bool {
    *PROF.get_or_init(|| std::env::var("AGENT_PROF").as_deref() == Ok("1"))
}

static PULL_NS: AtomicU64 = AtomicU64::new(0); // total ns spent in pull HTTP round-trips
static PULL_CNT: AtomicU64 = AtomicU64::new(0); // pull calls made
static PULL_NONE: AtomicU64 = AtomicU64::new(0); // pulls that returned no job (would nap)
static SEND_NS: AtomicU64 = AtomicU64::new(0); // total ns spent in send_result round-trips
static SEND_CNT: AtomicU64 = AtomicU64::new(0); // jobs completed (= 1 send_result each)
static LAST_PULL: Mutex<Option<Instant>> = Mutex::new(None); // start time of the previous pull
static CYCLE_NS: AtomicU64 = AtomicU64::new(0); // sum of inter-pull intervals = independent cycle
static CYCLE_CNT: AtomicU64 = AtomicU64::new(0);
// Agent-side buffer health: the worker BLOCKS whenever the pull buffer is empty (must sync-fetch)
// or the send buffer is full (must flush inline). For the ~1M j/s single-worker target both must
// be ~0 — pull always has a job ready, push always drains async.
static PULL_EMPTY: AtomicU64 = AtomicU64::new(0); // worker found PULL_BUF empty -> blocking sync refill
static SEND_INLINE: AtomicU64 = AtomicU64::new(0); // worker hit full SEND_BUF -> blocking inline flush
// Split the pull cost: POP = the fast path (buffer had a job) = mutex acquire + pop_front;
// REFILL = the slow path (buffer empty) = the blocking sync HTTP fetch. pop+refill == PULL_NS.
static PULL_POP_NS: AtomicU64 = AtomicU64::new(0);
static PULL_POP_CNT: AtomicU64 = AtomicU64::new(0);
static PULL_REFILL_NS: AtomicU64 = AtomicU64::new(0); // count == PULL_EMPTY
// Directly measure the two phases previously left as a residual, so cycle = pull+exec+send+gap
// EXACTLY (no inference). EXEC = pull_end->send_start (real job processing); GAP = send_end->
// next pull_start (loop overhead + the agent being parked/descheduled / waiting between jobs).
static EXEC_NS: AtomicU64 = AtomicU64::new(0);
static EXEC_CNT: AtomicU64 = AtomicU64::new(0);
static GAP_NS: AtomicU64 = AtomicU64::new(0);
static GAP_CNT: AtomicU64 = AtomicU64::new(0);
// Split the GAP into CPU vs parked: gap_cpu = thread CPU consumed during the gap (real loop work),
// gap_parked = gap_wall − gap_cpu (suspended: I/O await OR descheduled waiting for a core). Only
// valid when the task didn't migrate OS threads mid-gap (CLOCK_THREAD_CPUTIME_ID is per-thread) —
// guarded by comparing ThreadId; migrated gaps are counted but excluded from the cpu split.
static GAP_CPU_NS: AtomicU64 = AtomicU64::new(0);       // thread CPU during same-thread gaps
static GAP_CPU_WALL_NS: AtomicU64 = AtomicU64::new(0);  // wall of those same-thread gaps (denominator)
static GAP_CPU_CNT: AtomicU64 = AtomicU64::new(0);
static GAP_MIG_CNT: AtomicU64 = AtomicU64::new(0);      // gaps where the task migrated threads
static LAST_PULL_END: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_SEND_END: Mutex<Option<(Instant, u64, std::thread::ThreadId)>> = Mutex::new(None);
// Empty-pull NAP: when a pull returns no job the worker tokio::sleeps (sleep_queue()*10 ms). For a
// high-throughput batching agent that momentarily drains its buffer this is catastrophic — it
// sleeps 500ms+ and the whole nap lands in the gap. Count them so the diagram shows it directly.
static NAP_NS: AtomicU64 = AtomicU64::new(0);   // total wall time slept on empty pulls
static NAP_CNT: AtomicU64 = AtomicU64::new(0);  // number of empty-pull naps
static REPORTER: Once = Once::new();

/// Record an empty-pull nap (called from the worker loop's Ok(None) arm). Counts only under AGENT_PROF.
pub(crate) fn record_nap(elapsed: Duration) {
    if prof_on() {
        NAP_NS.fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
        NAP_CNT.fetch_add(1, Ordering::Relaxed);
    }
}

// Per-thread consumed CPU time (nanoseconds). Wall − this = time the thread was NOT running.
fn thread_cpu_nanos() -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
}

/// Spawn the 2s rolling reporter once. For a single synchronous agent (NUM_WORKERS=1)
/// the loop is serial, so cycle = wall/jobs is exact; pull+send come from the timers;
/// gap = cycle - pull - send = local per-job work (job_dir, perms, exec) + any nap.
fn start_reporter() {
    REPORTER.call_once(|| {
        tokio::spawn(async {
            let (mut p_ns, mut p_c, mut p_n, mut s_ns, mut s_c, mut cy_ns, mut cy_c) =
                (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
            let (mut prev_pe, mut prev_si) = (0u64, 0u64);
            // prev trackers so the per-job phase costs can be emitted WINDOWED (per-interval delta)
            // instead of cumulative lifetime — stable on spiky runs.
            let (mut prev_pop_ns, mut prev_ref_ns, mut prev_exec_ns, mut prev_exec_cnt) = (0u64, 0u64, 0u64, 0u64);
            let (mut prev_gap_ns, mut prev_gap_cnt) = (0u64, 0u64);
            let (mut prev_gcpu_ns, mut prev_gcpu_wall, mut prev_gcpu_cnt, mut prev_gmig) = (0u64, 0u64, 0u64, 0u64);
            let mut prev_nap_ns = 0u64;
            let start = Instant::now(); // for lifetime/cumulative aggregation
            let mut t = Instant::now();
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let now = Instant::now();
                let dt = now.duration_since(t).as_secs_f64();
                let (np, cp, nn, ns, cs, ncy, ccy) = (
                    PULL_NS.load(Ordering::Relaxed),
                    PULL_CNT.load(Ordering::Relaxed),
                    PULL_NONE.load(Ordering::Relaxed),
                    SEND_NS.load(Ordering::Relaxed),
                    SEND_CNT.load(Ordering::Relaxed),
                    CYCLE_NS.load(Ordering::Relaxed),
                    CYCLE_CNT.load(Ordering::Relaxed),
                );
                let d_np = np - p_ns;
                let (d_cp, d_nn, d_ns, d_cs, d_ncy, d_ccy) =
                    (cp - p_c, nn - p_n, ns - s_ns, cs - s_c, ncy - cy_ns, ccy - cy_c);
                if d_cs > 0 {
                    let thr = d_cs as f64 / dt; // independent #1: completed / wall
                    let pull_ms = if d_cp > 0 { d_np as f64 / d_cp as f64 / 1e6 } else { 0.0 };
                    let send_ms = d_ns as f64 / d_cs as f64 / 1e6;
                    let cycle_thr = 1000.0 / thr; // cycle implied by throughput
                    let cycle_meas = if d_ccy > 0 { d_ncy as f64 / d_ccy as f64 / 1e6 } else { 0.0 }; // independent #2: inter-pull interval
                    let local_ms = cycle_meas - pull_ms - send_ms;
                    eprintln!(
                        "[AGENTPROF] thr={:.0} j/s | cycle: meas={:.3}ms thr-implied={:.3}ms (ratio {:.2}) | pull={:.3} send={:.3} local={:.3} ms | HTTP={:.0}% of cycle | empty_pulls/s={:.0}",
                        thr, cycle_meas, cycle_thr, cycle_meas / cycle_thr,
                        pull_ms, send_ms, local_ms,
                        (pull_ms + send_ms) / cycle_meas * 100.0, d_nn as f64 / dt
                    );
                    // Cumulative / lifetime means accumulated over the whole run (not a
                    // single 2s window): total ns / total count since pulling began.
                    eprintln!(
                        "[AGENTPROF-CUM] t={:.0}s served={} pull={:.3}ms send={:.3}ms cycle={:.3}ms (lifetime means)",
                        start.elapsed().as_secs_f64(),
                        cs,
                        if cp > 0 { np as f64 / cp as f64 / 1e6 } else { 0.0 },
                        if cs > 0 { ns as f64 / cs as f64 / 1e6 } else { 0.0 },
                        if ccy > 0 { ncy as f64 / ccy as f64 / 1e6 } else { 0.0 }
                    );
                    // Agent-side buffer health. depth sampled now (try_lock; -1 = momentarily
                    // locked). pull_empty/s = worker BLOCKED on a sync refill (pull buffer ran
                    // dry); inline_flush/s = worker BLOCKED flushing a full send buffer. Both
                    // must trend to 0 for pull/push to be free.
                    let pe = PULL_EMPTY.load(Ordering::Relaxed);
                    let si = SEND_INLINE.load(Ordering::Relaxed);
                    let pd = pull_buf().try_lock().map(|b| b.len() as i64).unwrap_or(-1);
                    let sd = send_buf().try_lock().map(|b| b.len() as i64).unwrap_or(-1);
                    // WINDOWED per-job phase costs: per-interval delta (Δns / Δcount over THIS 2s
                    // window), NOT cumulative lifetime — so a spiky run's dips don't smear across the
                    // whole run, and the plot's steady-window aggregation reconciles with throughput.
                    let r = Ordering::Relaxed;
                    let pop_ns = PULL_POP_NS.load(r); let ref_ns = PULL_REFILL_NS.load(r);
                    let exec_ns = EXEC_NS.load(r); let exec_cnt = EXEC_CNT.load(r);
                    let gap_ns = GAP_NS.load(r); let gap_cnt = GAP_CNT.load(r);
                    let gcpu_ns = GAP_CPU_NS.load(r); let gcpu_wall = GAP_CPU_WALL_NS.load(r);
                    let gcpu_cnt = GAP_CPU_CNT.load(r); let gmig = GAP_MIG_CNT.load(r);
                    let d_cpm = d_cp.max(1) as f64;                                   // ΔPULL_CNT this window
                    let d_exec_c = (exec_cnt - prev_exec_cnt).max(1) as f64;
                    let d_gap_c = (gap_cnt - prev_gap_cnt).max(1) as f64;
                    let d_gcc = (gcpu_cnt - prev_gcpu_cnt).max(1) as f64;
                    let d_gmig = (gmig - prev_gmig) as f64;
                    let pop_us = (pop_ns - prev_pop_ns) as f64 / d_cpm / 1000.0;       // amortized over jobs
                    let ref_us = (ref_ns - prev_ref_ns) as f64 / d_cpm / 1000.0;
                    let exec_us = (exec_ns - prev_exec_ns) as f64 / d_exec_c / 1000.0;
                    let gap_us = (gap_ns - prev_gap_ns) as f64 / d_gap_c / 1000.0;
                    let gap_cpu_us = (gcpu_ns - prev_gcpu_ns) as f64 / d_gcc / 1000.0;
                    let gap_parked_us = ((gcpu_wall - prev_gcpu_wall).saturating_sub(gcpu_ns - prev_gcpu_ns) as f64 / d_gcc / 1000.0).max(0.0);
                    let gap_mig_pct = d_gmig / (d_gcc + d_gmig).max(1.0) * 100.0;
                    // NAP: the empty-pull sleep (part of the gap). nap_us = windowed per-job; total
                    // count + total ms are cumulative so the diagram can show "N naps = X ms slept".
                    let nap_ns = NAP_NS.load(r); let nap_cnt = NAP_CNT.load(r);
                    let nap_us = (nap_ns - prev_nap_ns) as f64 / d_gap_c / 1000.0;
                    eprintln!(
                        "[AGENTBUF] pull_depth={} send_depth={} pull_empty/s={:.0} inline_flush/s={:.0} pull_pop_us={:.2} pull_refill_us={:.2} exec_us={:.2} gap_us={:.2} gap_cpu_us={:.2} gap_parked_us={:.2} gap_mig_pct={:.0} nap_us={:.2} nap_cnt={} nap_total_ms={:.0} pull_empty_total={} send_inline_total={}",
                        pd, sd, (pe - prev_pe) as f64 / dt, (si - prev_si) as f64 / dt, pop_us, ref_us, exec_us, gap_us, gap_cpu_us, gap_parked_us, gap_mig_pct, nap_us, nap_cnt, nap_ns as f64 / 1e6, pe, si
                    );
                    prev_pe = pe; prev_si = si;
                    prev_pop_ns = pop_ns; prev_ref_ns = ref_ns;
                    prev_exec_ns = exec_ns; prev_exec_cnt = exec_cnt;
                    prev_gap_ns = gap_ns; prev_gap_cnt = gap_cnt;
                    prev_gcpu_ns = gcpu_ns; prev_gcpu_wall = gcpu_wall; prev_gcpu_cnt = gcpu_cnt; prev_gmig = gmig;
                    prev_nap_ns = nap_ns;
                }
                t = now;
                p_ns = np; p_c = cp; p_n = nn; s_ns = ns; s_c = cs; cy_ns = ncy; cy_c = ccy;
            }
        });
    });
}

pub async fn queue_init_job(client: &HttpClient, content: &str) -> anyhow::Result<Uuid> {
    client
        .post(
            "/api/agent_workers/queue_init_job",
            None,
            &QueueInitJob { content: content.to_string() },
        )
        .await
        .and_then(|x: String| Uuid::parse_str(&x).map_err(|e| anyhow::anyhow!(e)))
}

pub async fn queue_periodic_job(client: &HttpClient, content: &str) -> anyhow::Result<Uuid> {
    client
        .post(
            "/api/agent_workers/queue_periodic_job",
            None,
            &QueueInitJob { content: content.to_string() },
        )
        .await
        .and_then(|x: String| Uuid::parse_str(&x).map_err(|e| anyhow::anyhow!(e)))
}

// ---- EXPERIMENT 3: agent-side batch pull + batch complete (gated behind AGENT_CLIENT_BATCH>1) ----
// The agent pulls N jobs per HTTP and buffers completions, flushing N-at-a-time (or on a
// timeout), so the ~2 HTTP round-trips per job collapse to ~2 per N jobs — amortizing the wire.
// For a single agent worker (NUM_WORKERS=1) the process-wide buffers below are single-consumer.
static CLIENT_BATCH: OnceLock<usize> = OnceLock::new();
fn client_batch() -> usize {
    *CLIENT_BATCH.get_or_init(|| {
        std::env::var("AGENT_CLIENT_BATCH").ok().and_then(|v| v.parse().ok()).unwrap_or(0)
    })
}
static CLIENT_BATCH_MS: OnceLock<u64> = OnceLock::new();
fn client_batch_ms() -> u64 {
    *CLIENT_BATCH_MS.get_or_init(|| {
        std::env::var("AGENT_CLIENT_BATCH_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(20)
    })
}
static PULL_BUF: OnceLock<tokio::sync::Mutex<std::collections::VecDeque<JobAndPerms>>> = OnceLock::new();
fn pull_buf() -> &'static tokio::sync::Mutex<std::collections::VecDeque<JobAndPerms>> {
    PULL_BUF.get_or_init(|| tokio::sync::Mutex::new(std::collections::VecDeque::new()))
}
static SEND_BUF: OnceLock<tokio::sync::Mutex<Vec<JobCompleted>>> = OnceLock::new();
fn send_buf() -> &'static tokio::sync::Mutex<Vec<JobCompleted>> {
    SEND_BUF.get_or_init(|| tokio::sync::Mutex::new(Vec::new()))
}
static FLUSHER: Once = Once::new();
static REFILL_INFLIGHT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// Eager (background) refill of the local pull buffer (gated by AGENT_CLIENT_EAGER=1): top it up
// when it drops below AGENT_CLIENT_REFILL_PCT% of the batch, so pop() never waits on a fetch.
static CLIENT_EAGER: OnceLock<bool> = OnceLock::new();
fn client_eager() -> bool {
    *CLIENT_EAGER.get_or_init(|| {
        std::env::var("AGENT_CLIENT_EAGER").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false)
    })
}
static CLIENT_REFILL_PCT: OnceLock<usize> = OnceLock::new();
#[allow(dead_code)]
fn client_refill_pct() -> usize {
    *CLIENT_REFILL_PCT.get_or_init(|| {
        std::env::var("AGENT_CLIENT_REFILL_PCT").ok().and_then(|v| v.parse().ok()).filter(|n| *n <= 100).unwrap_or(50)
    })
}
// Keep this many BATCHES of runway buffered (refill whenever below). Each refill takes ~5ms to
// land, so the buffer must be deep enough that the worker can't drain it in that time.
static CLIENT_PREFETCH: OnceLock<usize> = OnceLock::new();
fn client_prefetch_batches() -> usize {
    *CLIENT_PREFETCH.get_or_init(|| {
        std::env::var("AGENT_CLIENT_PREFETCH_BATCHES").ok().and_then(|v| v.parse().ok()).filter(|n| *n >= 1).unwrap_or(2)
    })
}
// Allow this many concurrent refills in flight (one isn't enough when refill latency > drain time).
static CLIENT_MAX_INFLIGHT: OnceLock<usize> = OnceLock::new();
fn client_max_inflight() -> usize {
    *CLIENT_MAX_INFLIGHT.get_or_init(|| {
        std::env::var("AGENT_CLIENT_MAX_INFLIGHT").ok().and_then(|v| v.parse().ok()).filter(|n| *n >= 1).unwrap_or(3)
    })
}

// Record the per-job cycle (interval between pull_job calls) into the AGENT_PROF counters.
fn prof_cycle() {
    use std::sync::atomic::Ordering::Relaxed;
    let now = Instant::now();
    let mut lp = LAST_PULL.lock().unwrap();
    if let Some(prev) = *lp {
        CYCLE_NS.fetch_add(now.duration_since(prev).as_nanos() as u64, Relaxed);
        CYCLE_CNT.fetch_add(1, Relaxed);
    }
    *lp = Some(now);
}

// Keep the pull buffer DEEP: refill whenever it's below `prefetch_batches` worth of jobs, and
// allow up to `max_inflight` concurrent refills — so the buffer never drains to 0 even though
// each refill takes ~5ms to land (otherwise pop() falls into the blocking sync fetch).
fn maybe_spawn_refill(client: &HttpClient) {
    use std::sync::atomic::Ordering::{AcqRel, Relaxed};
    let n = client_batch();
    let target = (n * client_prefetch_batches()).max(1);
    let max_inflight = client_max_inflight();
    let low = match pull_buf().try_lock() {
        Ok(b) => b.len() < target,
        Err(_) => false,
    };
    if !low {
        return;
    }
    // claim an in-flight slot (cap at max_inflight)
    let cur = REFILL_INFLIGHT.load(Relaxed);
    if cur >= max_inflight
        || REFILL_INFLIGHT.compare_exchange(cur, cur + 1, AcqRel, Relaxed).is_err()
    {
        return;
    }
    let client = clone_client(client);
    tokio::spawn(async move {
        let jobs: Vec<JobAndPerms> = client
            .post("/api/agent_workers/pull_jobs_batch", None, &n)
            .await
            .unwrap_or_default();
        if !jobs.is_empty() {
            pull_buf().lock().await.extend(jobs);
        }
        REFILL_INFLIGHT.fetch_sub(1, Relaxed);
    });
}

fn clone_client(client: &HttpClient) -> HttpClient {
    HttpClient { client: client.client.clone(), base_internal_url: client.base_internal_url.clone() }
}

async fn flush_completes(client: &HttpClient, batch: Vec<JobCompleted>) -> anyhow::Result<()> {
    if batch.is_empty() {
        return Ok(());
    }
    let w_id = batch[0].job.workspace_id.clone();
    let _: String = client
        .post(&format!("/api/w/{}/agent_workers/send_results_batch", w_id), None, &batch)
        .await?;
    Ok(())
}

// Background flush so buffered completions (e.g. the tail) don't sit unsent past the timeout.
fn ensure_flusher(client: &HttpClient) {
    FLUSHER.call_once(|| {
        let client = clone_client(client);
        let ms = client_batch_ms();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(ms)).await;
                let batch = {
                    let mut b = send_buf().lock().await;
                    if b.is_empty() {
                        continue;
                    }
                    std::mem::take(&mut *b)
                };
                let _ = flush_completes(&client, batch).await;
            }
        });
    });
}

async fn pull_job_batched(client: &HttpClient) -> anyhow::Result<Option<JobAndPerms>> {
    use std::sync::atomic::Ordering::Relaxed;
    let prof = prof_on();
    if prof {
        start_reporter();
        prof_cycle();
        // GAP: between the previous send returning and this pull starting (loop + suspension).
        if let Some((se, se_cpu, se_tid)) = *LAST_SEND_END.lock().unwrap() {
            let now = Instant::now();
            let wall = now.duration_since(se).as_nanos() as u64;
            GAP_NS.fetch_add(wall, Relaxed);
            GAP_CNT.fetch_add(1, Relaxed);
            // CPU split is only valid if the worker task stayed on the same OS thread for the gap.
            if std::thread::current().id() == se_tid {
                GAP_CPU_NS.fetch_add(thread_cpu_nanos().saturating_sub(se_cpu), Relaxed);
                GAP_CPU_WALL_NS.fetch_add(wall, Relaxed);
                GAP_CPU_CNT.fetch_add(1, Relaxed);
            } else {
                GAP_MIG_CNT.fetch_add(1, Relaxed);
            }
        }
    }
    let t = Instant::now();
    {
        let mut b = pull_buf().lock().await;
        if let Some(j) = b.pop_front() {
            drop(b);
            if prof {
                let e = t.elapsed().as_nanos() as u64;
                PULL_NS.fetch_add(e, Relaxed);
                PULL_CNT.fetch_add(1, Relaxed);
                PULL_POP_NS.fetch_add(e, Relaxed); // fast path = mutex acquire + pop_front
                PULL_POP_CNT.fetch_add(1, Relaxed);
                *LAST_PULL_END.lock().unwrap() = Some(Instant::now()); // exec starts after this
            }
            if client_eager() {
                maybe_spawn_refill(client);
            }
            return Ok(Some(j));
        }
    }
    // buffer empty (reactive path, or eager fell behind): fetch a batch synchronously.
    // This is the BLOCKING refill — the worker stalls here until the HTTP fetch returns.
    if prof {
        PULL_EMPTY.fetch_add(1, Relaxed);
    }
    let n = client_batch();
    let jobs: Vec<JobAndPerms> = client
        .post("/api/agent_workers/pull_jobs_batch", None, &n)
        .await?;
    if prof {
        let e = t.elapsed().as_nanos() as u64;
        PULL_NS.fetch_add(e, Relaxed);
        PULL_CNT.fetch_add(1, Relaxed);
        PULL_REFILL_NS.fetch_add(e, Relaxed); // slow path = blocking sync HTTP fetch
        if jobs.is_empty() {
            PULL_NONE.fetch_add(1, Relaxed);
        }
    }
    if jobs.is_empty() {
        return Ok(None);
    }
    let mut b = pull_buf().lock().await;
    b.extend(jobs);
    let j = b.pop_front();
    drop(b);
    if prof && j.is_some() {
        *LAST_PULL_END.lock().unwrap() = Some(Instant::now()); // exec starts after this
    }
    if client_eager() {
        maybe_spawn_refill(client);
    }
    Ok(j)
}

async fn send_result_batched(client: &HttpClient, jc: JobCompleted) -> anyhow::Result<String> {
    use std::sync::atomic::Ordering::Relaxed;
    let prof = prof_on();
    ensure_flusher(client);
    if prof {
        // EXEC: pull_end -> send_start = the real job processing the worker did this cycle.
        if let Some(pe) = *LAST_PULL_END.lock().unwrap() {
            EXEC_NS.fetch_add(Instant::now().duration_since(pe).as_nanos() as u64, Relaxed);
            EXEC_CNT.fetch_add(1, Relaxed);
        }
    }
    let t = Instant::now();
    let n = client_batch();
    let flush = {
        let mut b = send_buf().lock().await;
        b.push(jc);
        if b.len() >= n {
            Some(std::mem::take(&mut *b))
        } else {
            None
        }
    };
    let did_flush = flush.is_some();
    if let Some(batch) = flush {
        // BLOCKING inline flush — the worker stalls here sending the batch over HTTP.
        flush_completes(client, batch).await?;
    }
    if prof {
        if did_flush {
            SEND_INLINE.fetch_add(1, Relaxed);
        }
        SEND_NS.fetch_add(t.elapsed().as_nanos() as u64, Relaxed);
        SEND_CNT.fetch_add(1, Relaxed);
        // stamp wall + thread-CPU + thread-id so the next pull can split the gap into cpu vs parked.
        *LAST_SEND_END.lock().unwrap() =
            Some((Instant::now(), thread_cpu_nanos(), std::thread::current().id()));
    }
    Ok("ok".to_string())
}

pub async fn pull_job(
    client: &HttpClient,
    headers: Option<HeaderMap>,
    body: Option<bool>,
) -> anyhow::Result<Option<JobAndPerms>> {
    if client_batch() > 1 {
        return pull_job_batched(client).await;
    }
    if !prof_on() {
        return client
            .post("/api/agent_workers/pull_job", headers, &body)
            .await;
    }
    start_reporter();
    {
        // independent cycle: time between consecutive pull starts (serial agent => true cycle)
        let now = Instant::now();
        let mut lp = LAST_PULL.lock().unwrap();
        if let Some(prev) = *lp {
            CYCLE_NS.fetch_add(now.duration_since(prev).as_nanos() as u64, Ordering::Relaxed);
            CYCLE_CNT.fetch_add(1, Ordering::Relaxed);
        }
        *lp = Some(now);
    }
    let t = Instant::now();
    let r: anyhow::Result<Option<JobAndPerms>> = client
        .post("/api/agent_workers/pull_job", headers, &body)
        .await;
    PULL_NS.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
    PULL_CNT.fetch_add(1, Ordering::Relaxed);
    if matches!(&r, Ok(None)) {
        PULL_NONE.fetch_add(1, Ordering::Relaxed);
    }
    r
}

pub async fn send_result(client: &HttpClient, jc: JobCompleted) -> anyhow::Result<String> {
    if client_batch() > 1 {
        return send_result_batched(client, jc).await;
    }
    let url = format!(
        "/api/w/{}/agent_workers/send_result/{}",
        jc.job.workspace_id, jc.job.id
    );
    if !prof_on() {
        return client.post(&url, None, &jc).await;
    }
    let t = Instant::now();
    let r: anyhow::Result<String> = client.post(&url, None, &jc).await;
    SEND_NS.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
    SEND_CNT.fetch_add(1, Ordering::Relaxed);
    r
}

#[allow(dead_code)]
pub async fn get_ducklake_from_agent_http(
    client: &HttpClient,
    name: &str,
    w_id: &str,
) -> anyhow::Result<DucklakeWithConnData> {
    client
        .get(&format!(
            "/api/w/{}/agent_workers/get_ducklake/{}",
            w_id, &name
        ))
        .await
}

#[allow(dead_code)]
pub async fn get_datatable_resource_from_agent_http(
    client: &HttpClient,
    name: &str,
    w_id: &str,
) -> anyhow::Result<serde_json::Value> {
    client
        .get(&format!(
            "/api/w/{}/agent_workers/get_datatable_resource/{}",
            w_id, &name
        ))
        .await
}

pub const UPDATE_PING_URL: &str = "/api/agent_workers/update_ping";
