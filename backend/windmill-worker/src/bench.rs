use serde::Serialize;
use tokio::time::Instant;
use windmill_common::{
    worker::{write_file, TMP_DIR},
    DB,
};

pub struct BenchmarkInfo {
    iters: u64,
    timings: Vec<BenchmarkIter>,
}

impl Serialize for BenchmarkInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let timings: Vec<Vec<(String, u32)>> = self
            .timings
            .iter()
            .map(|x| x.timings.clone())
            .collect::<Vec<Vec<(String, u32)>>>();
        //serialize timings as vec of vec of tuples
        timings.serialize(serializer)
    }
}

impl BenchmarkInfo {
    pub fn new() -> Self {
        BenchmarkInfo { iters: 0, timings: vec![] }
    }

    pub fn add_iter(&mut self, bench: BenchmarkIter) {
        self.iters += 1;
        self.timings.push(bench);
    }

    pub fn write_to_file(&self, path: &str) -> anyhow::Result<()> {
        println!("Writing benchmark {path}");
        write_file(TMP_DIR, path, &serde_json::to_string(&self).unwrap()).expect("write profiling");
        Ok(())
    }
}

pub struct BenchmarkIter {
    last_instant: Instant,
    timings: Vec<(String, u32)>,
}

impl BenchmarkIter {
    pub fn new() -> Self {
        BenchmarkIter { last_instant: Instant::now(), timings: vec![] }
    }

    pub fn add_timing(&mut self, name: &str) {
        let elapsed = self.last_instant.elapsed().as_nanos() as u32;
        self.timings.push((name.to_string(), elapsed));
        self.last_instant = Instant::now();
    }
}

pub async fn benchmark_init(is_dedicated_worker: bool, db: &DB) {
    use windmill_common::{jobs::JobKind, scripts::ScriptLang};

    let benchmark_jobs: i32 = std::env::var("BENCHMARK_JOBS_AT_INIT")
        .unwrap_or("5000".to_string())
        .parse::<i32>()
        .unwrap();
    if is_dedicated_worker {
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
    } else {
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
