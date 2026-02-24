use crate::{
    worker::{write_file, TMP_DIR},
    DB,
};
use serde::Serialize;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::time::Instant;
use uuid::Uuid;

static BENCHMARK_INITIALIZED: AtomicBool = AtomicBool::new(false);

static SHARED_BENCH_ITERS: std::sync::LazyLock<Arc<AtomicU64>> =
    std::sync::LazyLock::new(|| Arc::new(AtomicU64::new(0)));

pub fn shared_bench_iters() -> Arc<AtomicU64> {
    SHARED_BENCH_ITERS.clone()
}

#[derive(Serialize)]
pub struct PoolStats {
    pub peak_active_conns: u32,
    pub pool_saturation_histogram: Vec<u32>,
}

impl PoolStats {
    pub fn new(pool_size: u32) -> Self {
        PoolStats {
            peak_active_conns: 0,
            pool_saturation_histogram: vec![0; pool_size as usize + 1],
        }
    }

    pub fn sample(&mut self, pool_size: u32, num_idle: u32) {
        let active = pool_size.saturating_sub(num_idle);
        if active > self.peak_active_conns {
            self.peak_active_conns = active;
        }
        if let Some(bucket) = self.pool_saturation_histogram.get_mut(active as usize) {
            *bucket += 1;
        }
    }
}

#[derive(Serialize)]
pub struct BenchmarkInfo {
    #[serde(skip)]
    pub start: Instant,
    #[serde(skip)]
    pub iters: u64,
    #[serde(skip)]
    seen_top_level: HashSet<Uuid>,
    #[serde(skip)]
    pub shared_iters: Arc<AtomicU64>,
    timings: Vec<BenchmarkIter>,
    pub iter_durations: Vec<u64>,
    pub total_duration: Option<u64>,
    pub pool_stats: Option<PoolStats>,
}

impl BenchmarkInfo {
    pub fn new(shared_iters: Arc<AtomicU64>) -> Self {
        BenchmarkInfo {
            iters: 0,
            seen_top_level: HashSet::new(),
            shared_iters,
            timings: vec![],
            start: Instant::now(),
            iter_durations: vec![],
            total_duration: None,
            pool_stats: None,
        }
    }

    pub fn init_pool_stats(&mut self, pool_size: u32) {
        self.pool_stats = Some(PoolStats::new(pool_size));
    }

    pub fn sample_pool(&mut self, pool_size: u32, num_idle: u32) {
        if let Some(stats) = self.pool_stats.as_mut() {
            stats.sample(pool_size, num_idle);
        }
    }

    pub fn count_top_level(&mut self, job_id: Uuid) -> bool {
        if self.seen_top_level.insert(job_id) {
            self.iters += 1;
            return true;
        }
        false
    }

    pub fn add_iter(&mut self, bench: BenchmarkIter, job_id: Uuid, is_top_level: bool) -> bool {
        let newly_counted = is_top_level && self.seen_top_level.insert(job_id);
        if newly_counted {
            self.iters += 1;
        }
        let elapsed_total = bench.start.elapsed().as_nanos() as u64;
        self.timings.push(bench);
        self.iter_durations.push(elapsed_total);
        newly_counted
    }

    pub fn write_to_file(&mut self, path: &str) -> anyhow::Result<()> {
        let total_duration = self.start.elapsed().as_millis() as u64;
        self.total_duration = Some(total_duration as u64);

        let pool_info = self.pool_stats.as_ref().map_or(String::new(), |ps| {
            format!(", peak active conns: {}", ps.peak_active_conns)
        });
        println!(
            "Writing benchmark {path}, duration of benchmark: {total_duration}ms and RPS: {}{pool_info}",
            self.iters as f64 / total_duration as f64 * 1000.0
        );
        write_file(TMP_DIR, path, &serde_json::to_string(&self).unwrap()).expect("write profiling");
        Ok(())
    }
}

#[derive(Serialize)]
pub struct BenchmarkIter {
    #[serde(skip)]
    pub start: Instant,
    #[serde(skip)]
    last_instant: Instant,
    last_step: String,
    timings: Vec<(String, u32)>,
}

impl BenchmarkIter {
    pub fn new() -> Self {
        BenchmarkIter {
            last_instant: Instant::now(),
            timings: vec![],
            start: Instant::now(),
            last_step: String::new(),
        }
    }

    pub fn add_timing(&mut self, name: &str) {
        let elapsed = self.last_instant.elapsed().as_nanos() as u32;
        self.timings
            .push((format!("{}->{}", self.last_step, name), elapsed));
        self.last_instant = Instant::now();
        self.last_step = name.to_string();
    }
}

pub async fn benchmark_verify(benchmark_jobs: i32, db: &DB) {
    let benchmark_kind = std::env::var("BENCHMARK_KIND").unwrap_or("noop".to_string());

    if benchmark_jobs <= 0 || benchmark_kind == "none" {
        return;
    }

    // For flows, child jobs are created dynamically so only check top-level (parent_job IS NULL).
    // "parallelflow" inserts only 1 top-level flow regardless of benchmark_jobs.
    let expected_top_level = match benchmark_kind.as_str() {
        "parallelflow" => 1i64,
        _ => benchmark_jobs as i64,
    };

    let row = sqlx::query!(
        "SELECT
            COUNT(*) FILTER (WHERE status = 'success') AS succeeded,
            COUNT(*) FILTER (WHERE status = 'failure') AS failed,
            COUNT(*) FILTER (WHERE status = 'canceled') AS canceled
         FROM v2_job_completed
         JOIN v2_job USING (id)
         WHERE v2_job.workspace_id = 'admins' AND v2_job.parent_job IS NULL",
    )
    .fetch_one(db)
    .await
    .expect("benchmark verify query failed");

    let succeeded = row.succeeded.unwrap_or(0);
    let failed = row.failed.unwrap_or(0);
    let canceled = row.canceled.unwrap_or(0);
    let total = succeeded + failed + canceled;

    let remaining_in_queue =
        sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue WHERE workspace_id = 'admins'",)
            .fetch_one(db)
            .await
            .expect("benchmark verify queue query failed")
            .unwrap_or(0);

    println!("=== BENCHMARK VERIFICATION ===");
    println!("  kind:              {benchmark_kind}");
    println!("  expected top-level: {expected_top_level}");
    println!("  completed total:   {total}");
    println!("  succeeded:         {succeeded}");
    println!("  failed:            {failed}");
    println!("  canceled:          {canceled}");
    println!("  still in queue:    {remaining_in_queue}");

    if failed > 0 || canceled > 0 {
        tracing::error!(
            "BENCHMARK VERIFICATION FAILED: {failed} failed, {canceled} canceled out of {total} completed"
        );
    }
    if remaining_in_queue > 0 {
        tracing::warn!(
            "BENCHMARK VERIFICATION: {remaining_in_queue} jobs still in queue after benchmark"
        );
    }
    if succeeded != expected_top_level {
        tracing::error!(
            "BENCHMARK VERIFICATION FAILED: expected {expected_top_level} succeeded top-level jobs, got {succeeded}"
        );
    } else if failed == 0 && canceled == 0 && remaining_in_queue == 0 {
        println!("  result:            ALL PASSED");
    }
    println!("==============================");
}

pub async fn benchmark_init(benchmark_jobs: i32, db: &DB) {
    use crate::{jobs::JobKind, scripts::ScriptLang};

    // Only the first worker to reach this point runs init
    if BENCHMARK_INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }

    let benchmark_kind = std::env::var("BENCHMARK_KIND").unwrap_or("noop".to_string());

    if benchmark_jobs > 0 {
        // Clean up data from previous benchmark runs
        sqlx::query!("DELETE FROM v2_job_completed WHERE workspace_id = 'admins'")
            .execute(db)
            .await
            .unwrap_or_else(|e| panic!("failed to clean up v2_job_completed: {e:#}"));
        sqlx::query!("DELETE FROM v2_job_queue WHERE workspace_id = 'admins'")
            .execute(db)
            .await
            .unwrap_or_else(|e| panic!("failed to clean up v2_job_queue: {e:#}"));
        sqlx::query!("DELETE FROM v2_job_status WHERE id IN (SELECT id FROM v2_job WHERE workspace_id = 'admins')")
            .execute(db)
            .await
            .unwrap_or_else(|e| panic!("failed to clean up v2_job_status: {e:#}"));
        sqlx::query!("DELETE FROM v2_job_runtime WHERE id IN (SELECT id FROM v2_job WHERE workspace_id = 'admins')")
            .execute(db)
            .await
            .unwrap_or_else(|e| panic!("failed to clean up v2_job_runtime: {e:#}"));
        sqlx::query("DELETE FROM job_perms WHERE workspace_id = 'admins'")
            .execute(db)
            .await
            .unwrap_or_else(|e| panic!("failed to clean up job_perms: {e:#}"));
        sqlx::query!(
            "DELETE FROM concurrency_key WHERE key LIKE 'bench_%' OR key LIKE 'u/admin/bench_%'"
        )
        .execute(db)
        .await
        .unwrap_or_else(|e| panic!("failed to clean up concurrency_key: {e:#}"));
        sqlx::query!("DELETE FROM concurrency_counter WHERE concurrency_id LIKE 'bench_%' OR concurrency_id LIKE 'u/admin/bench_%'")
            .execute(db)
            .await
            .unwrap_or_else(|e| panic!("failed to clean up concurrency_counter: {e:#}"));
        sqlx::query!("DELETE FROM v2_job WHERE workspace_id = 'admins'")
            .execute(db)
            .await
            .unwrap_or_else(|e| panic!("failed to clean up v2_job: {e:#}"));

        let mut tx = db.begin().await.unwrap();
        match benchmark_kind.as_str() {
            "dedicated" => {
                // you need to create the script first, check https://github.com/windmill-labs/windmill/blob/b76a92cfe454c686f005c65f534e29e039f3c706/benchmarks/lib.ts#L47
                let hash = sqlx::query_scalar!(
                    "SELECT hash FROM script WHERE path = $1 AND workspace_id = $2",
                    "f/benchmarks/dedicated",
                    "admins"
                )
                .fetch_one(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert dedicated jobs"));
                let uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9 FROM generate_series(1, $10)) RETURNING id",
                    hash,
                    "f/benchmarks/dedicated",
                    JobKind::Script as JobKind,
                    ScriptLang::Bun as ScriptLang,
                    "admins:f/benchmarks/dedicated",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    "admins",
                    benchmark_jobs
                )
                .fetch_all(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert dedicated jobs (1)"));
                sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &uuids, "admins", "admins:f/benchmarks/dedicated")
                .execute(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert dedicated jobs (2)"));
                sqlx::query!(
                    "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                    &uuids
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert dedicated jobs (3)"));
            }
            "parallelflow" => {
                //create dedicated script
                sqlx::query!("INSERT INTO script (summary, description, dedicated_worker, content, workspace_id, path, hash, language, tag, created_by, lock) VALUES ('', '', true, $1, $2, $3, $4, $5, $6, $7, '') ON CONFLICT (workspace_id, hash) DO NOTHING",
                "export async function main() {
                    console.log('hello world');
                }",
                "admins",
                "u/admin/parallelflow",
                1234567890,
                ScriptLang::Deno as ScriptLang,
                "flow",
                "admin",
                )
                .execute(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert parallelflow jobs {_e:#}"));
                let uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, raw_flow) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 FROM generate_series(1, 1)) RETURNING id",
                    None::<i64>,
                    None::<String>,
                    JobKind::FlowPreview as JobKind,
                    ScriptLang::Deno as ScriptLang,
                    "flow",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    "admins",
                    serde_json::from_str::<serde_json::Value>(r#"
{
  "modules": [
    {
      "id": "a",
      "value": {
        "type": "forloopflow",
        "modules": [
          {
            "id": "b",
            "value": {
              "path": "u/admin/parallelflow",
              "type": "script",
              "tag_override": "",
              "input_transforms": {}
            },
            "summary": "calctest"
          }
        ],
        "iterator": {
          "expr": "[...new Array(300)]",
          "type": "javascript"
        },
        "parallel": true,
        "parallelism": 10,
        "skip_failures": true
      }
    }
  ],
  "preprocessor_module": null
}
                    "#).unwrap(),
                )
                .fetch_all(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert parallelflow jobs (1)"));
                sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &uuids, "admins", "flow")
                .execute(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert parallelflow jobs (2)"));
                sqlx::query!(
                    "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                    &uuids
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert parallelflow jobs (3)"));
                sqlx::query!(
                    "INSERT INTO v2_job_status (id, flow_status) SELECT unnest($1::uuid[]), $2",
                    &uuids,
                    serde_json::from_str::<serde_json::Value>(
                        r#"
{
		"step": 0,
		"modules": [
			{
				"id": "a",
				"type": "WaitingForPriorSteps"
			}
		],
		"cleanup_module": {},
		"failure_module": {
			"id": "failure",
			"type": "WaitingForPriorSteps"
		},
		"preprocessor_module": null
	}

                "#
                    )
                    .unwrap()
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert parallelflow jobs (4)"));
            }
            "sequentialflow" => {
                let uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, raw_flow) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 FROM generate_series(1, $11)) RETURNING id",
                    None::<i64>,
                    None::<String>,
                    JobKind::FlowPreview as JobKind,
                    ScriptLang::Deno as ScriptLang,
                    "flow",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    "admins",
                    serde_json::from_str::<serde_json::Value>(r#"
{
  "modules": [
    {
      "id": "a",
      "value": {
        "type": "rawscript",
        "content": "export async function main() { return 'a'; }",
        "language": "deno",
        "input_transforms": {}
      }
    },
    {
      "id": "b",
      "value": {
        "type": "rawscript",
        "content": "export async function main() { return 'b'; }",
        "language": "deno",
        "input_transforms": {}
      }
    },
    {
      "id": "c",
      "value": {
        "type": "rawscript",
        "content": "export async function main() { return 'c'; }",
        "language": "deno",
        "input_transforms": {}
      }
    },
    {
      "id": "d",
      "value": {
        "type": "rawscript",
        "content": "export async function main() { return 'd'; }",
        "language": "deno",
        "input_transforms": {}
      }
    }
  ],
  "preprocessor_module": null
}
                    "#).unwrap(),
                    benchmark_jobs,
                )
                .fetch_all(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert sequentialflow jobs (1)"));
                sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &uuids, "admins", "flow")
                .execute(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert sequentialflow jobs (2)"));
                sqlx::query!(
                    "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                    &uuids
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert sequentialflow jobs (3)"));
                sqlx::query!(
                    "INSERT INTO v2_job_status (id, flow_status) SELECT unnest($1::uuid[]), $2",
                    &uuids,
                    serde_json::from_str::<serde_json::Value>(
                        r#"
{
    "step": 0,
    "modules": [
        { "id": "a", "type": "WaitingForPriorSteps" },
        { "id": "b", "type": "WaitingForPriorSteps" },
        { "id": "c", "type": "WaitingForPriorSteps" },
        { "id": "d", "type": "WaitingForPriorSteps" }
    ],
    "cleanup_module": {},
    "failure_module": {
        "id": "failure",
        "type": "WaitingForPriorSteps"
    },
    "preprocessor_module": null
}
                "#
                    )
                    .unwrap()
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert sequentialflow jobs (4)"));
            }
            "scriptlogs" => {
                let uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, raw_code) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 FROM generate_series(1, $11)) RETURNING id",
                    None::<i64>,
                    None::<String>,
                    JobKind::Preview as JobKind,
                    ScriptLang::Deno as ScriptLang,
                    "deno",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    "admins",
                    "export async function main() { for (let i = 0; i < 1000; i++) { console.log('benchmark log line ' + i); } return 'done'; }",
                    benchmark_jobs,
                )
                .fetch_all(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert scriptlogs jobs (1)"));
                sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &uuids, "admins", "deno")
                .execute(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert scriptlogs jobs (2)"));
                sqlx::query!(
                    "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                    &uuids
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert scriptlogs jobs (3)"));
            }
            "concurrencylimit" => {
                let uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, raw_code, concurrent_limit, concurrency_time_window_s) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12 FROM generate_series(1, $13)) RETURNING id",
                    None::<i64>,
                    Some("u/admin/bench_conclimit"),
                    JobKind::Preview as JobKind,
                    ScriptLang::Deno as ScriptLang,
                    "deno",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    "admins",
                    "export async function main() { return 'done'; }",
                    2i32,
                    0i32,
                    benchmark_jobs,
                )
                .fetch_all(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert concurrencylimit jobs (1)"));
                sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &uuids, "admins", "deno")
                .execute(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert concurrencylimit jobs (2)"));
                sqlx::query!(
                    "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                    &uuids
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert concurrencylimit jobs (3)"));
                let concurrency_id = "u/admin/bench_conclimit";
                sqlx::query!(
                    "INSERT INTO concurrency_counter (concurrency_id, job_uuids) VALUES ($1, '{}'::jsonb) ON CONFLICT (concurrency_id) DO NOTHING",
                    concurrency_id,
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert concurrencylimit counter"));
                for uuid in &uuids {
                    sqlx::query!(
                        "INSERT INTO concurrency_key (key, ended_at, job_id) VALUES ($1, now() - INTERVAL '1 hour', $2)",
                        concurrency_id,
                        uuid,
                    )
                    .execute(&mut *tx)
                    .await
                    .unwrap_or_else(|_e| panic!("failed to insert concurrencylimit key"));
                }
            }
            "concurrencykey" => {
                let uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, raw_code, concurrent_limit, concurrency_time_window_s) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12 FROM generate_series(1, $13)) RETURNING id",
                    None::<i64>,
                    Some("u/admin/bench_conckey"),
                    JobKind::Preview as JobKind,
                    ScriptLang::Deno as ScriptLang,
                    "deno",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    "admins",
                    "export async function main() { return 'done'; }",
                    1i32,
                    0i32,
                    benchmark_jobs,
                )
                .fetch_all(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert concurrencykey jobs (1)"));
                sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &uuids, "admins", "deno")
                .execute(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert concurrencykey jobs (2)"));
                sqlx::query!(
                    "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                    &uuids
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert concurrencykey jobs (3)"));
                let concurrency_id = "bench_shared_concurrency_key";
                sqlx::query!(
                    "INSERT INTO concurrency_counter (concurrency_id, job_uuids) VALUES ($1, '{}'::jsonb) ON CONFLICT (concurrency_id) DO NOTHING",
                    concurrency_id,
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert concurrencykey counter"));
                for uuid in &uuids {
                    sqlx::query!(
                        "INSERT INTO concurrency_key (key, ended_at, job_id) VALUES ($1, now() - INTERVAL '1 hour', $2)",
                        concurrency_id,
                        uuid,
                    )
                    .execute(&mut *tx)
                    .await
                    .unwrap_or_else(|_e| panic!("failed to insert concurrencykey key"));
                }
            }
            "mixed" => {
                let portion = benchmark_jobs / 5;
                let remainder = benchmark_jobs % 5;

                // 1) noop jobs
                let noop_count = portion + remainder;
                let noop_uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9 FROM generate_series(1, $10)) RETURNING id",
                    None::<i64>,
                    None::<String>,
                    JobKind::Noop as JobKind,
                    ScriptLang::Deno as ScriptLang,
                    "deno",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    "admins",
                    noop_count,
                )
                .fetch_all(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert mixed noop jobs"));
                sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &noop_uuids, "admins", "deno")
                .execute(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert mixed noop queue"));
                sqlx::query!(
                    "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                    &noop_uuids
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert mixed noop runtime"));

                // 2) sequentialflow jobs
                if portion > 0 {
                    let sf_uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, raw_flow) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 FROM generate_series(1, $11)) RETURNING id",
                        None::<i64>,
                        None::<String>,
                        JobKind::FlowPreview as JobKind,
                        ScriptLang::Deno as ScriptLang,
                        "flow",
                        "admin",
                        "u/admin",
                        "admin@windmill.dev",
                        "admins",
                        serde_json::from_str::<serde_json::Value>(r#"{"modules":[{"id":"a","value":{"type":"rawscript","content":"export async function main() { return 'a'; }","language":"deno","input_transforms":{}}},{"id":"b","value":{"type":"rawscript","content":"export async function main() { return 'b'; }","language":"deno","input_transforms":{}}},{"id":"c","value":{"type":"rawscript","content":"export async function main() { return 'c'; }","language":"deno","input_transforms":{}}},{"id":"d","value":{"type":"rawscript","content":"export async function main() { return 'd'; }","language":"deno","input_transforms":{}}}],"preprocessor_module":null}"#).unwrap(),
                        portion,
                    )
                    .fetch_all(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed sequentialflow jobs"));
                    sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &sf_uuids, "admins", "flow")
                    .execute(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed sequentialflow queue"));
                    sqlx::query!(
                        "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                        &sf_uuids
                    )
                    .execute(&mut *tx)
                    .await
                    .unwrap_or_else(|_e| panic!("failed to insert mixed sequentialflow runtime"));
                    sqlx::query!(
                        "INSERT INTO v2_job_status (id, flow_status) SELECT unnest($1::uuid[]), $2",
                        &sf_uuids,
                        serde_json::from_str::<serde_json::Value>(r#"{"step":0,"modules":[{"id":"a","type":"WaitingForPriorSteps"},{"id":"b","type":"WaitingForPriorSteps"},{"id":"c","type":"WaitingForPriorSteps"},{"id":"d","type":"WaitingForPriorSteps"}],"cleanup_module":{},"failure_module":{"id":"failure","type":"WaitingForPriorSteps"},"preprocessor_module":null}"#).unwrap()
                    )
                    .execute(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed sequentialflow status"));
                }

                // 3) scriptlogs jobs
                if portion > 0 {
                    let sl_uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, raw_code) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 FROM generate_series(1, $11)) RETURNING id",
                        None::<i64>,
                        None::<String>,
                        JobKind::Preview as JobKind,
                        ScriptLang::Deno as ScriptLang,
                        "deno",
                        "admin",
                        "u/admin",
                        "admin@windmill.dev",
                        "admins",
                        "export async function main() { for (let i = 0; i < 1000; i++) { console.log('benchmark log line ' + i); } return 'done'; }",
                        portion,
                    )
                    .fetch_all(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed scriptlogs jobs"));
                    sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &sl_uuids, "admins", "deno")
                    .execute(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed scriptlogs queue"));
                    sqlx::query!(
                        "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                        &sl_uuids
                    )
                    .execute(&mut *tx)
                    .await
                    .unwrap_or_else(|_e| panic!("failed to insert mixed scriptlogs runtime"));
                }

                // 4) concurrencylimit jobs
                if portion > 0 {
                    let cl_uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, raw_code, concurrent_limit, concurrency_time_window_s) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12 FROM generate_series(1, $13)) RETURNING id",
                        None::<i64>,
                        Some("u/admin/bench_conclimit"),
                        JobKind::Preview as JobKind,
                        ScriptLang::Deno as ScriptLang,
                        "deno",
                        "admin",
                        "u/admin",
                        "admin@windmill.dev",
                        "admins",
                        "export async function main() { return 'done'; }",
                        2i32,
                        0i32,
                        portion,
                    )
                    .fetch_all(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed concurrencylimit jobs"));
                    sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &cl_uuids, "admins", "deno")
                    .execute(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed concurrencylimit queue"));
                    sqlx::query!(
                        "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                        &cl_uuids
                    )
                    .execute(&mut *tx)
                    .await
                    .unwrap_or_else(|_e| panic!("failed to insert mixed concurrencylimit runtime"));
                    let cl_concurrency_id = "u/admin/bench_conclimit";
                    sqlx::query!(
                        "INSERT INTO concurrency_counter (concurrency_id, job_uuids) VALUES ($1, '{}'::jsonb) ON CONFLICT (concurrency_id) DO NOTHING",
                        cl_concurrency_id,
                    )
                    .execute(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed concurrencylimit counter"));
                    for uuid in &cl_uuids {
                        sqlx::query!(
                            "INSERT INTO concurrency_key (key, ended_at, job_id) VALUES ($1, now() - INTERVAL '1 hour', $2)",
                            cl_concurrency_id,
                            uuid,
                        )
                        .execute(&mut *tx)
                        .await.unwrap_or_else(|_e| panic!("failed to insert mixed concurrencylimit key"));
                    }
                }

                // 5) concurrencykey jobs
                if portion > 0 {
                    let ck_uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, raw_code, concurrent_limit, concurrency_time_window_s) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12 FROM generate_series(1, $13)) RETURNING id",
                        None::<i64>,
                        Some("u/admin/bench_conckey"),
                        JobKind::Preview as JobKind,
                        ScriptLang::Deno as ScriptLang,
                        "deno",
                        "admin",
                        "u/admin",
                        "admin@windmill.dev",
                        "admins",
                        "export async function main() { return 'done'; }",
                        1i32,
                        0i32,
                        portion,
                    )
                    .fetch_all(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed concurrencykey jobs"));
                    sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &ck_uuids, "admins", "deno")
                    .execute(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed concurrencykey queue"));
                    sqlx::query!(
                        "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                        &ck_uuids
                    )
                    .execute(&mut *tx)
                    .await
                    .unwrap_or_else(|_e| panic!("failed to insert mixed concurrencykey runtime"));
                    let ck_concurrency_id = "bench_shared_concurrency_key";
                    sqlx::query!(
                        "INSERT INTO concurrency_counter (concurrency_id, job_uuids) VALUES ($1, '{}'::jsonb) ON CONFLICT (concurrency_id) DO NOTHING",
                        ck_concurrency_id,
                    )
                    .execute(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed concurrencykey counter"));
                    for uuid in &ck_uuids {
                        sqlx::query!(
                            "INSERT INTO concurrency_key (key, ended_at, job_id) VALUES ($1, now() - INTERVAL '1 hour', $2)",
                            ck_concurrency_id,
                            uuid,
                        )
                        .execute(&mut *tx)
                        .await.unwrap_or_else(|_e| panic!("failed to insert mixed concurrencykey key"));
                    }
                }
            }
            "mixed_no_cc" => {
                let portion = benchmark_jobs / 3;
                let remainder = benchmark_jobs % 3;

                // 1) noop jobs
                let noop_count = portion + remainder;
                let noop_uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9 FROM generate_series(1, $10)) RETURNING id",
                    None::<i64>,
                    None::<String>,
                    JobKind::Noop as JobKind,
                    ScriptLang::Deno as ScriptLang,
                    "deno",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    "admins",
                    noop_count,
                )
                .fetch_all(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert mixed_no_cc noop jobs"));
                sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &noop_uuids, "admins", "deno")
                .execute(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert mixed_no_cc noop queue"));
                sqlx::query!(
                    "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                    &noop_uuids
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert mixed_no_cc noop runtime"));

                // 2) sequentialflow jobs
                if portion > 0 {
                    let sf_uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, raw_flow) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 FROM generate_series(1, $11)) RETURNING id",
                        None::<i64>,
                        None::<String>,
                        JobKind::FlowPreview as JobKind,
                        ScriptLang::Deno as ScriptLang,
                        "flow",
                        "admin",
                        "u/admin",
                        "admin@windmill.dev",
                        "admins",
                        serde_json::from_str::<serde_json::Value>(r#"{"modules":[{"id":"a","value":{"type":"rawscript","content":"export async function main() { return 'a'; }","language":"deno","input_transforms":{}}},{"id":"b","value":{"type":"rawscript","content":"export async function main() { return 'b'; }","language":"deno","input_transforms":{}}},{"id":"c","value":{"type":"rawscript","content":"export async function main() { return 'c'; }","language":"deno","input_transforms":{}}},{"id":"d","value":{"type":"rawscript","content":"export async function main() { return 'd'; }","language":"deno","input_transforms":{}}}],"preprocessor_module":null}"#).unwrap(),
                        portion,
                    )
                    .fetch_all(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed_no_cc sequentialflow jobs"));
                    sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &sf_uuids, "admins", "flow")
                    .execute(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed_no_cc sequentialflow queue"));
                    sqlx::query!(
                        "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                        &sf_uuids
                    )
                    .execute(&mut *tx)
                    .await
                    .unwrap_or_else(|_e| {
                        panic!("failed to insert mixed_no_cc sequentialflow runtime")
                    });
                    sqlx::query!(
                        "INSERT INTO v2_job_status (id, flow_status) SELECT unnest($1::uuid[]), $2",
                        &sf_uuids,
                        serde_json::from_str::<serde_json::Value>(r#"{"step":0,"modules":[{"id":"a","type":"WaitingForPriorSteps"},{"id":"b","type":"WaitingForPriorSteps"},{"id":"c","type":"WaitingForPriorSteps"},{"id":"d","type":"WaitingForPriorSteps"}],"cleanup_module":{},"failure_module":{"id":"failure","type":"WaitingForPriorSteps"},"preprocessor_module":null}"#).unwrap()
                    )
                    .execute(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed_no_cc sequentialflow status"));
                }

                // 3) scriptlogs jobs
                if portion > 0 {
                    let sl_uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, raw_code) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 FROM generate_series(1, $11)) RETURNING id",
                        None::<i64>,
                        None::<String>,
                        JobKind::Preview as JobKind,
                        ScriptLang::Deno as ScriptLang,
                        "deno",
                        "admin",
                        "u/admin",
                        "admin@windmill.dev",
                        "admins",
                        "export async function main() { for (let i = 0; i < 1000; i++) { console.log('benchmark log line ' + i); } return 'done'; }",
                        portion,
                    )
                    .fetch_all(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed_no_cc scriptlogs jobs"));
                    sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &sl_uuids, "admins", "deno")
                    .execute(&mut *tx)
                    .await.unwrap_or_else(|_e| panic!("failed to insert mixed_no_cc scriptlogs queue"));
                    sqlx::query!(
                        "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                        &sl_uuids
                    )
                    .execute(&mut *tx)
                    .await
                    .unwrap_or_else(|_e| panic!("failed to insert mixed_no_cc scriptlogs runtime"));
                }
            }
            "none" => {}
            _ => {
                let uuids = sqlx::query_scalar!("INSERT INTO v2_job (id, runnable_id, runnable_path, kind, script_lang, tag, created_by, permissioned_as, permissioned_as_email, workspace_id) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9 FROM generate_series(1, $10)) RETURNING id",
                    None::<i64>,
                    None::<String>,
                    JobKind::Noop as JobKind,
                    ScriptLang::Deno as ScriptLang,
                    "deno",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    "admins",
                    benchmark_jobs
                )
                .fetch_all(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert noop jobs (1)"));
                sqlx::query!("INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) SELECT unnest($1::uuid[]), $2, now(), $3", &uuids, "admins", "deno")
                .execute(&mut *tx)
                .await.unwrap_or_else(|_e| panic!("failed to insert noop jobs (2)"));
                sqlx::query!(
                    "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                    &uuids
                )
                .execute(&mut *tx)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert noop jobs (3)"));
            }
        }
        // Insert job_perms for all benchmark jobs so workers don't fall back to slow permission lookups
        sqlx::query(
            "INSERT INTO job_perms (job_id, email, username, is_admin, is_operator, groups, folders, workspace_id)
             SELECT id, 'admin@windmill.dev', 'admin', true, false, ARRAY['all']::text[], ARRAY[]::jsonb[], 'admins'
             FROM v2_job WHERE workspace_id = 'admins'"
        )
        .execute(&mut *tx)
        .await
        .unwrap_or_else(|e| panic!("failed to insert job_perms: {e:#}"));

        tx.commit().await.unwrap();
    }
}
