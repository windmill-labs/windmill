//! Per-job cost profiler: allocation bytes, allocation sites and SQL statements per job, for
//! jobs run end to end through the real queue, worker and database in this process.
//!
//! Not a regression test: every test is `#[ignore]`d and asserts only that the jobs succeeded.
//! How to run it, read it and A/B a change: `docs/perf-profiling.md`.
//!
//! Each workload warms up, measures the idle background rate, pushes `PERF_N` jobs scheduled a
//! few seconds ahead (so the push is measured alone), then measures the worker until all of them
//! complete. Reports land in `$PERF_OUT/<workload>.txt`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use futures::StreamExt;
use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::flows::FlowValue;
use windmill_common::jobs::JobPayload;
use windmill_common::runnable_settings::{ConcurrencySettings, DebouncingSettings};
use windmill_common::scripts::{ScriptHash, ScriptLang};
use windmill_common::worker::Connection;
use windmill_test_utils::*;

// ---------------------------------------------------------------- allocator

struct Profiler;

#[global_allocator]
static GLOBAL: Profiler = Profiler;

static BYTES: AtomicU64 = AtomicU64::new(0);
static COUNT: AtomicU64 = AtomicU64::new(0);
static SAMPLING: AtomicBool = AtomicBool::new(false);

const BYTE_PERIOD: u64 = 128 * 1024;
const COUNT_PERIOD: u64 = 2048;

thread_local! {
    // Set while a sample is being captured: the backtrace's own allocations are neither
    // counted nor sampled, or they would dominate every number.
    static IN_SAMPLER: Cell<bool> = const { Cell::new(false) };
    static BYTES_LEFT: Cell<u64> = const { Cell::new(BYTE_PERIOD) };
    static COUNT_LEFT: Cell<u64> = const { Cell::new(COUNT_PERIOD) };
}

#[derive(Default, Clone, Copy)]
struct Site {
    byte_samples: u64,
    count_samples: u64,
}

static SITES_LEAF: Mutex<Option<HashMap<String, Site>>> = Mutex::new(None);
static SITES_CHAIN: Mutex<Option<HashMap<String, Site>>> = Mutex::new(None);

unsafe impl GlobalAlloc for Profiler {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        record(l.size());
        System.alloc(l)
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        record(l.size());
        System.alloc_zeroed(l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        record(n);
        System.realloc(p, l, n)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
}

#[inline]
fn record(size: usize) {
    if IN_SAMPLER.try_with(|i| i.get()).unwrap_or(true) {
        return;
    }
    BYTES.fetch_add(size as u64, Relaxed);
    COUNT.fetch_add(1, Relaxed);
    if !SAMPLING.load(Relaxed) {
        return;
    }
    let size = size as u64;
    let byte_hits = BYTES_LEFT.with(|b| {
        let left = b.get();
        if size >= left {
            let over = size - left;
            b.set(BYTE_PERIOD - over % BYTE_PERIOD);
            1 + over / BYTE_PERIOD
        } else {
            b.set(left - size);
            0
        }
    });
    let count_hit = COUNT_LEFT.with(|c| {
        let left = c.get();
        c.set(if left <= 1 { COUNT_PERIOD } else { left - 1 });
        left <= 1
    });
    if byte_hits > 0 || count_hit {
        IN_SAMPLER.with(|i| i.set(true));
        capture(byte_hits, count_hit);
        IN_SAMPLER.with(|i| i.set(false));
    }
}

/// Attributes a sample to the deepest frame in a Windmill crate. Dev builds use split DWARF and
/// inline aggressively, so frame *names* are often bare (`poll<…>`); the relative
/// `./windmill-*/src/…:line` location is what identifies the site.
fn capture(byte_hits: u64, count_hit: bool) {
    let bt = std::backtrace::Backtrace::force_capture().to_string();
    let mut frames: Vec<(String, String)> = vec![];
    let mut lines = bt.lines().peekable();
    while let Some(l) = lines.next() {
        let Some((num, name)) = l.trim_start().split_once(": ") else {
            continue;
        };
        if !num.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let loc = match lines.peek() {
            Some(n) if n.trim_start().starts_with("at ") => {
                lines.next().unwrap().trim_start()[3..].to_string()
            }
            _ => String::new(),
        };
        frames.push((name.to_string(), loc));
    }
    let is_windmill = |(_, loc): &(String, String)| {
        (loc.starts_with("./windmill-") || loc.starts_with("./src/"))
            && !loc.starts_with("./windmill-test-utils")
    };
    let Some(first) = frames.iter().position(is_windmill) else {
        return;
    };
    let short_loc = |l: &str| -> String {
        let l = l.trim_start_matches("./");
        l.rfind(':').map_or(l, |i| &l[..i]).to_string()
    };
    const PLUMBING: &[&str] = &[
        "alloc::",
        "<alloc::",
        "__rust",
        "perf_profile",
        "<perf_profile",
        "try_with",
        "capture",
        "record",
        "reserve",
        "finish_grow",
        "grow",
        "do_reserve",
        "with_capacity",
        "allocate",
        "new_uninit",
        "exchange_malloc",
        "box_new",
        "realloc",
    ];
    let callee: String = frames[..first]
        .iter()
        .rev()
        .map(|f| f.0.as_str())
        .find(|n| !PLUMBING.iter().any(|p| n.starts_with(p)))
        .unwrap_or_default()
        .chars()
        .take(100)
        .collect();
    let leaf = format!("{}  [-> {callee}]", short_loc(&frames[first].1));
    let chain = frames[first..]
        .iter()
        .filter(|f| is_windmill(f))
        .map(|f| short_loc(&f.1))
        .take(4)
        .collect::<Vec<_>>()
        .join(" < ");
    let chain = format!("{chain}  [-> {callee}]");
    for (map, key) in [(&SITES_LEAF, leaf), (&SITES_CHAIN, chain)] {
        let mut g = map.lock().unwrap();
        let e = g.get_or_insert_with(HashMap::new).entry(key).or_default();
        e.byte_samples += byte_hits;
        e.count_samples += count_hit as u64;
    }
}

// ---------------------------------------------------------------- sql counting

static SQL_ON: AtomicBool = AtomicBool::new(false);
static SQL: Mutex<Option<HashMap<String, (u64, f64)>>> = Mutex::new(None);

struct SqlLayer;

#[derive(Default)]
struct SqlVisit {
    statement: String,
    summary: String,
    secs: f64,
}

impl tracing::field::Visit for SqlVisit {
    fn record_str(&mut self, f: &tracing::field::Field, v: &str) {
        match f.name() {
            "db.statement" => self.statement = v.to_string(),
            "summary" => self.summary = v.to_string(),
            _ => {}
        }
    }
    fn record_f64(&mut self, f: &tracing::field::Field, v: f64) {
        if f.name() == "elapsed_secs" {
            self.secs = v;
        }
    }
    fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
        match f.name() {
            "db.statement" => self.statement = format!("{v:?}"),
            "summary" => self.summary = format!("{v:?}"),
            _ => {}
        }
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for SqlLayer {
    fn on_event(&self, ev: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        if !SQL_ON.load(Relaxed) {
            return;
        }
        let mut v = SqlVisit::default();
        ev.record(&mut v);
        // sqlx leaves `db.statement` empty when the summary already is the whole statement
        let raw = if v.statement.trim().is_empty() {
            v.summary
        } else {
            v.statement
        };
        let key: String = raw
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(200)
            .collect();
        let mut g = SQL.lock().unwrap();
        let e = g.get_or_insert_with(HashMap::new).entry(key).or_default();
        e.0 += 1;
        e.1 += v.secs;
    }
}

/// SQL counting is a separate mode (`PERF_SQL=1`): sqlx formats a query summary for every
/// statement once its event is enabled, which inflates the allocation numbers.
fn init_sql_counting() {
    use tracing_subscriber::prelude::*;
    if std::env::var("PERF_SQL").is_err() {
        return;
    }
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let filter = tracing_subscriber::filter::Targets::new()
            .with_target("sqlx::query", tracing::Level::TRACE);
        let _ = tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(SqlLayer.with_filter(filter)),
        );
    });
}

// ---------------------------------------------------------------- measurement

struct Snap {
    bytes: u64,
    count: u64,
    at: Instant,
}

fn start_phase() -> Snap {
    *SITES_LEAF.lock().unwrap() = None;
    *SITES_CHAIN.lock().unwrap() = None;
    *SQL.lock().unwrap() = None;
    SAMPLING.store(true, Relaxed);
    SQL_ON.store(true, Relaxed);
    Snap { bytes: BYTES.load(Relaxed), count: COUNT.load(Relaxed), at: Instant::now() }
}

fn end_phase() -> Snap {
    let snap = Snap { bytes: BYTES.load(Relaxed), count: COUNT.load(Relaxed), at: Instant::now() };
    SAMPLING.store(false, Relaxed);
    SQL_ON.store(false, Relaxed);
    snap
}

struct Idle {
    bytes_per_sec: f64,
    count_per_sec: f64,
}

fn report(out: &mut String, title: &str, s0: &Snap, s1: &Snap, n: u64, idle: &Idle) {
    use std::fmt::Write;
    let secs = s1.at.duration_since(s0.at).as_secs_f64();
    let n_f = n as f64;
    let bytes = (s1.bytes - s0.bytes) as f64;
    let count = (s1.count - s0.count) as f64;
    let adj_bytes = (bytes - idle.bytes_per_sec * secs).max(0.0);
    let adj_count = (count - idle.count_per_sec * secs).max(0.0);
    let _ = writeln!(
        out,
        "\n=== {title}: {n} jobs in {secs:.2}s | raw {:.1} KiB/job {:.0} allocs/job | idle-adjusted {:.1} KiB/job {:.0} allocs/job",
        bytes / n_f / 1024.0,
        count / n_f,
        adj_bytes / n_f / 1024.0,
        adj_count / n_f
    );
    for (name, map) in [("LEAF", &SITES_LEAF), ("CHAIN", &SITES_CHAIN)] {
        let g = map.lock().unwrap();
        let Some(map) = g.as_ref() else { continue };
        let mut sites: Vec<_> = map.iter().map(|(k, s)| (k.clone(), *s)).collect();
        sites.sort_by_key(|(_, s)| std::cmp::Reverse(s.byte_samples));
        let _ = writeln!(out, "--- {name} by bytes (KiB/job est.)");
        for (k, s) in sites.iter().take(30).filter(|(_, s)| s.byte_samples > 0) {
            let kib = (s.byte_samples * BYTE_PERIOD) as f64 / n_f / 1024.0;
            let _ = writeln!(out, "{kib:>9.1}  {k}");
        }
        sites.sort_by_key(|(_, s)| std::cmp::Reverse(s.count_samples));
        let _ = writeln!(out, "--- {name} by alloc count (allocs/job est.)");
        for (k, s) in sites.iter().take(30).filter(|(_, s)| s.count_samples > 0) {
            let allocs = (s.count_samples * COUNT_PERIOD) as f64 / n_f;
            let _ = writeln!(out, "{allocs:>9.0}  {k}");
        }
    }
    let g = SQL.lock().unwrap();
    if let Some(map) = g.as_ref() {
        let mut stmts: Vec<_> = map.iter().collect();
        stmts.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
        let total: u64 = stmts.iter().map(|(_, (c, _))| c).sum();
        let total_secs: f64 = stmts.iter().map(|(_, (_, s))| s).sum();
        let _ = writeln!(
            out,
            "--- SQL: {:.1} statements/job, {:.2} ms/job db time",
            total as f64 / n_f,
            total_secs * 1000.0 / n_f
        );
        for (k, (c, s)) in stmts.iter().take(60) {
            let _ = writeln!(
                out,
                "{:>7.2}/job {:>7.3}ms/job  {k}",
                *c as f64 / n_f,
                s * 1000.0 / n_f
            );
        }
    }
}

async fn wait_all(
    completed: &mut (impl futures::Stream<Item = Uuid> + Unpin),
    ids: &HashSet<Uuid>,
) {
    let mut left = ids.clone();
    while !left.is_empty() {
        let id = tokio::time::timeout(Duration::from_secs(120), completed.next())
            .await
            .expect("timed out waiting for completions")
            .expect("completion stream ended");
        left.remove(&id);
    }
}

async fn assert_succeeded(db: &Pool<Postgres>, ids: impl IntoIterator<Item = &Uuid>) {
    for id in ids {
        let job = completed_job(*id, db).await;
        assert!(job.success, "job {id} failed: {:?}", job.json_result());
    }
}

async fn profile_workload(db: Pool<Postgres>, name: &str, payload: impl Fn() -> JobPayload) {
    let n: usize = std::env::var("PERF_N")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    // `capture` takes std's non-reentrant backtrace lock. With either variable enabled, any
    // `anyhow::Error` captures a backtrace too, and an allocation it makes under that lock can
    // land on a sample point and deadlock the thread.
    for var in ["RUST_BACKTRACE", "RUST_LIB_BACKTRACE"] {
        let enabled = std::env::var(var).is_ok_and(|v| v != "0");
        assert!(
            !enabled,
            "unset {var}: it can deadlock the allocation sampler"
        );
    }
    init_sql_counting();
    set_jwt_secret().await;
    let server = ApiServer::start(db.clone()).await.unwrap();
    let mut completed = listen_for_completed_jobs(&db).await;
    let (quit, worker) = spawn_test_worker(&Connection::Sql(db.clone()), server.addr.port());

    // Warm caches (script/flow lookups, runtime caches) so they do not count against the
    // measured jobs.
    let mut warm = HashSet::new();
    for _ in 0..3 {
        warm.insert(RunJob::from(payload()).push(&db).await);
    }
    wait_all(&mut completed, &warm).await;
    assert_succeeded(&db, &warm).await;

    let mut out = String::new();

    let i0 = start_phase();
    tokio::time::sleep(Duration::from_secs(4)).await;
    let i1 = end_phase();
    let idle_secs = i1.at.duration_since(i0.at).as_secs_f64();
    let idle = Idle {
        bytes_per_sec: (i1.bytes - i0.bytes) as f64 / idle_secs,
        count_per_sec: (i1.count - i0.count) as f64 / idle_secs,
    };
    let no_idle = Idle { bytes_per_sec: 0.0, count_per_sec: 0.0 };
    report(
        &mut out,
        &format!("{name} IDLE (per second)"),
        &i0,
        &i1,
        idle_secs.round() as u64,
        &no_idle,
    );

    // Scheduled ahead so the worker cannot start on them while the push is being measured.
    let run_at = chrono::Utc::now() + chrono::Duration::seconds(12);
    let p0 = start_phase();
    let mut ids = HashSet::new();
    for _ in 0..n {
        ids.insert(
            RunJob::from(payload())
                .push_arg_scheduled_for_o(Some(run_at))
                .push(&db)
                .await,
        );
    }
    let p1 = end_phase();
    report(&mut out, &format!("{name} PUSH"), &p0, &p1, n as u64, &idle);
    let wait = (run_at - chrono::Utc::now())
        .to_std()
        .expect("the push phase outlasted the schedule offset; lower PERF_N");
    tokio::time::sleep(wait).await;

    let w0 = start_phase();
    wait_all(&mut completed, &ids).await;
    let w1 = end_phase();
    report(
        &mut out,
        &format!("{name} WORKER"),
        &w0,
        &w1,
        n as u64,
        &idle,
    );
    assert_succeeded(&db, &ids).await;

    quit.send();
    let _ = tokio::time::timeout(Duration::from_secs(30), worker).await;
    let _ = server.close().await;

    let dir: std::path::PathBuf =
        std::env::var("PERF_OUT").map_or_else(|_| std::env::temp_dir(), Into::into);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{name}.txt")), &out).unwrap();
    println!("{out}");
}

async fn insert_script(
    db: &Pool<Postgres>,
    path: &str,
    hash: i64,
    lang: &str,
    content: &str,
    lock: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO script (workspace_id, created_by, content, schema, summary, description, path, hash, language, lock)
         VALUES ('test-workspace', 'test-user', $1, '{}', '', '', $2, $3, $4::script_lang, $5)",
    )
    .bind(content)
    .bind(path)
    .bind(hash)
    .bind(lang)
    .bind(lock)
    .execute(db)
    .await
    .unwrap();
}

fn script_hash_payload(path: &str, hash: i64, language: ScriptLang) -> JobPayload {
    JobPayload::ScriptHash {
        hash: ScriptHash(hash),
        path: path.to_string(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        language,
        priority: None,
        apply_preprocessor: false,
        concurrency_settings: ConcurrencySettings::default().into(),
        debouncing_settings: DebouncingSettings::default(),
        labels: None,
        job_token_scopes: None,
    }
}

async fn insert_flow(db: &Pool<Postgres>, path: &str, version: i64, value: serde_json::Value) {
    let _: FlowValue = serde_json::from_value(value.clone()).expect("invalid flow value");
    sqlx::query(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, schema, extra_perms, versions)
         VALUES ('test-workspace', $1, '', '', $2, 'test-user', NOW(), '{}', '{}', ARRAY[$3::bigint])",
    )
    .bind(path)
    .bind(&value)
    .bind(version)
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by, created_at)
         VALUES ($1, 'test-workspace', $2, $3, '{}', 'test-user', NOW())",
    )
    .bind(version)
    .bind(path)
    .bind(&value)
    .execute(db)
    .await
    .unwrap();
}

fn flow_payload(path: &str, version: i64) -> JobPayload {
    JobPayload::Flow {
        path: path.to_string(),
        dedicated_worker: None,
        apply_preprocessor: false,
        version,
        labels: None,
        job_token_scopes: None,
    }
}

// ---------------------------------------------------------------- workloads

#[sqlx::test(fixtures("base"))]
#[ignore = "profiling harness, see docs/perf-profiling.md"]
async fn perf_bash_hash(db: Pool<Postgres>) {
    insert_script(&db, "f/perf/bash", 900001, "bash", "echo 42\n", Some("")).await;
    profile_workload(db, "bash_hash", || {
        script_hash_payload("f/perf/bash", 900001, ScriptLang::Bash)
    })
    .await;
}

#[sqlx::test(fixtures("base"))]
#[ignore = "profiling harness, see docs/perf-profiling.md"]
async fn perf_bun_hash(db: Pool<Postgres>) {
    // An empty bun lock is rejected at run time ("expected //bun.lock split pattern").
    let content = "export function main() { return { a: 1, b: [1, 2, 3] }; }\n";
    insert_script(&db, "f/perf/bun", 900002, "bun", content, None).await;
    profile_workload(db, "bun_hash", || {
        script_hash_payload("f/perf/bun", 900002, ScriptLang::Bun)
    })
    .await;
}

#[sqlx::test(fixtures("base"))]
#[ignore = "profiling harness, see docs/perf-profiling.md"]
async fn perf_flow_identity(db: Pool<Postgres>) {
    let step = |id: &str| json!({"id": id, "value": {"type": "identity"}});
    let value = json!({"modules": [step("a"), step("b"), step("c")]});
    insert_flow(&db, "f/perf/identity", 910001, value).await;
    profile_workload(db, "flow_identity", || {
        flow_payload("f/perf/identity", 910001)
    })
    .await;
}

#[sqlx::test(fixtures("base"))]
#[ignore = "profiling harness, see docs/perf-profiling.md"]
async fn perf_flow_bash(db: Pool<Postgres>) {
    let step = |id: &str| {
        json!({"id": id, "value": {"type": "rawscript", "language": "bash",
            "content": "echo 1\n", "input_transforms": {}}})
    };
    let value = json!({"modules": [step("a"), step("b"), step("c")]});
    insert_flow(&db, "f/perf/bashflow", 910002, value).await;
    profile_workload(db, "flow_bash", || flow_payload("f/perf/bashflow", 910002)).await;
}
