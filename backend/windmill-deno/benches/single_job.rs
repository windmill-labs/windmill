use criterion::{black_box, criterion_group, criterion_main, Criterion};

async fn run_job() -> i32 {
    let job_dir = "/tmp/windmill_bench";
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
    windmill_deno::run_deno_cli(args, job_dir, "/tmp/windmill_bench_cache")
        .await
        .unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    std::fs::create_dir_all("/tmp/windmill_bench").unwrap();
    std::fs::write("/tmp/windmill_bench/main.ts", b"").unwrap();
    std::fs::write("/tmp/windmill_bench/importmap.json", b"{}").unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("run job", |b| b.to_async(&runtime).iter(|| run_job()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
