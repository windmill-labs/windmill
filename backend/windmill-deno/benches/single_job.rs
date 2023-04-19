use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

async fn run_job(runner: &windmill_deno::runner::DenoRunnerPool) -> i32 {
    let job_dir = "/tmp/windmill_bench".to_owned();
    let args = vec![
        "deno".to_owned(),
        "run".to_owned(),
        "--import-map".to_owned(),
        "./importmap.json".to_owned(),
        "--reload=http://127.0.0.1/".to_owned(),
        "--allow-net".to_owned(),
        format!("--allow-read={job_dir}"),
        format!("--allow-write={job_dir}"),
        "--allow-env".to_owned(),
        "./main.ts".to_owned(),
    ];
    runner
        .run_job(args, job_dir, "/tmp/windmill_bench_cache".to_owned())
        .await
        .unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    std::fs::create_dir_all("/tmp/windmill_bench").unwrap();
    std::fs::write("/tmp/windmill_bench/main.ts", b"").unwrap();
    std::fs::write("/tmp/windmill_bench/importmap.json", b"{}").unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let runner = runtime.block_on(async move { windmill_deno::runner::DenoRunnerPool::new() });

    c.bench_function("run job", |b| {
        b.to_async(&runtime).iter(|| run_job(&runner))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(60));
    targets = criterion_benchmark
);
criterion_main!(benches);
