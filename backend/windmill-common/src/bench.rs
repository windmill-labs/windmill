use crate::{
    worker::{write_file, TMP_DIR},
    DB,
};
use serde::Serialize;
use tokio::time::Instant;

#[derive(Serialize)]
pub struct BenchmarkInfo {
    #[serde(skip)]
    pub start: Instant,
    #[serde(skip)]
    pub iters: u64,
    timings: Vec<BenchmarkIter>,
    pub iter_durations: Vec<u64>,
    pub total_duration: Option<u64>,
}

impl BenchmarkInfo {
    pub fn new() -> Self {
        BenchmarkInfo {
            iters: 0,
            timings: vec![],
            start: Instant::now(),
            iter_durations: vec![],
            total_duration: None,
        }
    }

    pub fn add_iter(&mut self, bench: BenchmarkIter, inc_iters: bool) {
        if inc_iters {
            self.iters += 1;
        }
        let elapsed_total = bench.start.elapsed().as_nanos() as u64;
        self.timings.push(bench);
        self.iter_durations.push(elapsed_total);
    }

    pub fn write_to_file(&mut self, path: &str) -> anyhow::Result<()> {
        let total_duration = self.start.elapsed().as_millis() as u64;
        self.total_duration = Some(total_duration as u64);

        println!(
            "Writing benchmark {path}, duration of benchmark: {total_duration}s and RPS: {}",
            self.iters as f64 / total_duration as f64
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

pub async fn benchmark_init(benchmark_jobs: i32, db: &DB) {
    use crate::{jobs::JobKind, scripts::ScriptLang};

    let benchmark_kind = std::env::var("BENCHMARK_KIND").unwrap_or("noop".to_string());

    if benchmark_jobs > 0 {
        match benchmark_kind.as_str() {
            "dedicated" => {
                // you need to create the script first, check https://github.com/windmill-labs/windmill/blob/b76a92cfe454c686f005c65f534e29e039f3c706/benchmarks/lib.ts#L47
                let hash = sqlx::query_scalar!(
                    "SELECT hash FROM script WHERE path = $1 AND workspace_id = $2",
                    "f/benchmarks/dedicated",
                    "admins"
                )
                .fetch_one(db)
                .await
                .unwrap_or_else(|_e| panic!("failed to insert dedicated jobs"));
                sqlx::query!("INSERT INTO queue (id, script_hash, script_path, job_kind, language, tag, created_by, permissioned_as, email, scheduled_for, workspace_id) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 FROM generate_series(1, $11))",
                    hash,
                    "f/benchmarks/dedicated",
                    JobKind::Script as JobKind,
                    ScriptLang::Bun as ScriptLang,
                    "admins:f/benchmarks/dedicated",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    chrono::Utc::now(),
                    "admins",
                    benchmark_jobs
                )
                .execute(db)
                .await.unwrap_or_else(|_e| panic!("failed to insert dedicated jobs"));
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
                .execute(db)
                .await.unwrap_or_else(|_e| panic!("failed to insert parallelflow jobs {_e:#}"));
                sqlx::query!("INSERT INTO queue (id, script_hash, script_path, job_kind, language, tag, created_by, permissioned_as, email, scheduled_for, workspace_id, raw_flow, flow_status) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12 FROM generate_series(1, 1))",
                    None::<i64>,
                    None::<String>,
                    JobKind::FlowPreview as JobKind,
                    ScriptLang::Deno as ScriptLang,
                    "flow",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    chrono::Utc::now(),
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
                    serde_json::from_str::<serde_json::Value>(r#"
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

                "#).unwrap()
                )
                .execute(db)
                .await.unwrap_or_else(|_e| panic!("failed to insert parallelflow jobs"));
            }
            _ => {
                sqlx::query!("INSERT INTO queue (id, script_hash, script_path, job_kind, language, tag, created_by, permissioned_as, email, scheduled_for, workspace_id) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 FROM generate_series(1, $11))",
                    None::<i64>,
                    None::<String>,
                    JobKind::Noop as JobKind,
                    ScriptLang::Deno as ScriptLang,
                    "deno",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    chrono::Utc::now(),
                    "admins",
                    benchmark_jobs
                )
                .execute(db)
                .await.unwrap_or_else(|_e| panic!("failed to insert noop jobs"));
            }
        }
    }
}
