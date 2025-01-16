use serde::Serialize;
use serde_json::json;
use tokio::time::Instant;
use windmill_common::{
    worker::{write_file, TMP_DIR},
    DB,
};

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

pub async fn benchmark_init(benchmark_jobs: usize, db: &DB) {
    use std::iter;

    use windmill_common::{jobs::JobKind, scripts::ScriptLang};
    use windmill_queue::RawJob;

    let benchmark_kind = std::env::var("BENCHMARK_KIND").unwrap_or("noop".to_string());
    let uuids = Vec::from_iter(iter::repeat_with(|| ulid::Ulid::new().into()).take(benchmark_jobs));

    if !uuids.is_empty() {
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
                RawJob {
                    runnable_id: Some(hash),
                    runnable_path: Some("f/benchmarks/dedicated"),
                    kind: JobKind::Script,
                    script_lang: Some(ScriptLang::Bun),
                    tag: "admins:f/benchmarks/dedicated",
                    created_by: "admin",
                    permissioned_as: "u/admin",
                    permissioned_as_email: "admin@windmill.dev",
                    ..RawJob::default()
                }
                .push_many(db.begin().await.unwrap(), uuids.as_slice(), "admins", &[])
                .await
                .unwrap_or_else(|_e| panic!("failed to insert dedicated jobs"))
                .commit()
                .await
                .unwrap_or_else(|_e| panic!("failed to commit insert of dedicated jobs"));
            }
            "parallelflow" => {
                // create dedicated script
                sqlx::query!(
                    "INSERT INTO script (
                        summary, description, dedicated_worker, content, workspace_id, path, hash,
                        language, tag, created_by, lock
                    ) VALUES ('', '', true, $1, $2, $3, $4, $5, $6, $7, '')
                    ON CONFLICT (workspace_id, hash) DO NOTHING",
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
                .await
                .unwrap_or_else(|e| panic!("failed to insert parallelflow script {e:#}"));
                RawJob {
                    kind: JobKind::FlowPreview,
                    tag: "flow",
                    created_by: "admin",
                    permissioned_as: "u/admin",
                    permissioned_as_email: "admin@windmill.dev",
                    raw_flow: Some(
                        &serde_json::from_value(json!({
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
                        }))
                        .unwrap(),
                    ),
                    flow_status: Some(
                        &serde_json::from_value(json!({
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
                        }))
                        .unwrap(),
                    ),
                    ..RawJob::default()
                }
                .push_many(db.begin().await.unwrap(), uuids.as_slice(), "admins", &[])
                .await
                .unwrap_or_else(|_e| panic!("failed to insert parallelflow jobs"))
                .commit()
                .await
                .unwrap_or_else(|_e| panic!("failed to commit insert of parallelflow jobs"));
            }
            _ => {
                RawJob {
                    kind: JobKind::Noop,
                    tag: "deno",
                    created_by: "admin",
                    permissioned_as: "u/admin",
                    permissioned_as_email: "admin@windmill.dev",
                    ..RawJob::default()
                }
                .push_many(db.begin().await.unwrap(), uuids.as_slice(), "admins", &[])
                .await
                .unwrap_or_else(|_e| panic!("failed to insert noop jobs"))
                .commit()
                .await
                .unwrap_or_else(|_e| panic!("failed to commit insert of noop jobs"));
            }
        }
    }
}
