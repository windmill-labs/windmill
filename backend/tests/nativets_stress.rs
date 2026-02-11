/*
 * Stress test for parallel nativets execution.
 *
 * Spawns 8 workers and pushes hundreds of fast nativets jobs concurrently.
 * Tests: arithmetic, JSON manipulation, string ops, fetch to an internal HTTP server,
 * and async operations. Validates all jobs complete successfully with correct results.
 *
 * IMPORTANT: V8 segfaults when the test harness captures stdout (default behavior).
 * Always run with --nocapture:
 *   cargo test -p windmill --features "deno_core" --test nativets_stress -- --nocapture
 */

#[cfg(feature = "deno_core")]
use windmill_test_utils::*;

#[cfg(feature = "deno_core")]
use std::time::Instant;

#[cfg(feature = "deno_core")]
use futures::StreamExt;
#[cfg(feature = "deno_core")]
use serde_json::json;
#[cfg(feature = "deno_core")]
use sqlx::{Pool, Postgres};
#[cfg(feature = "deno_core")]
use uuid::Uuid;

#[cfg(feature = "deno_core")]
use windmill_common::{
    jobs::{JobPayload, RawCode},
    scripts::ScriptLang,
    worker::{Connection, WORKER_CONFIG},
    KillpillSender,
};
#[cfg(feature = "deno_core")]
use windmill_queue::PushIsolationLevel;

#[cfg(feature = "deno_core")]
const NUM_WORKERS: usize = 8;

/// Various nativets scripts that are fast to execute but cover different features.
#[cfg(feature = "deno_core")]
fn job_scripts() -> Vec<(&'static str, serde_json::Value, serde_json::Value)> {
    vec![
        // (script_code, args_json, expected_result)
        (
            r#"//native

export function main(x: number, y: number): number {
    return x + y;
}
"#,
            json!({"x": 10, "y": 32}),
            json!(42),
        ),
        (
            r#"//native

export function main(s: string): string {
    return s.split('').reverse().join('');
}
"#,
            json!({"s": "hello"}),
            json!("olleh"),
        ),
        (
            r#"//native

export function main(n: number): number[] {
    return Array.from({length: n}, (_, i) => i * i);
}
"#,
            json!({"n": 5}),
            json!([0, 1, 4, 9, 16]),
        ),
        (
            r#"//native

export function main(items: {name: string, value: number}[]): {total: number, names: string[]} {
    return {
        total: items.reduce((sum, i) => sum + i.value, 0),
        names: items.map(i => i.name).sort(),
    };
}
"#,
            json!({"items": [{"name": "c", "value": 30}, {"name": "a", "value": 10}, {"name": "b", "value": 20}]}),
            json!({"total": 60, "names": ["a", "b", "c"]}),
        ),
        (
            r#"//native

export function main(a: number): object {
    const fib = (n: number): number => n <= 1 ? n : fib(n - 1) + fib(n - 2);
    return { input: a, fib: fib(a) };
}
"#,
            json!({"a": 10}),
            json!({"input": 10, "fib": 55}),
        ),
        (
            r#"//native

export function main(text: string, pattern: string): string[] {
    const regex = new RegExp(pattern, 'g');
    return [...text.matchAll(regex)].map(m => m[0]);
}
"#,
            json!({"text": "foo123bar456baz789", "pattern": "\\d+"}),
            json!(["123", "456", "789"]),
        ),
        (
            r#"//native

export function main(obj: Record<string, number>): Record<string, string> {
    return Object.fromEntries(
        Object.entries(obj).map(([k, v]) => [k.toUpperCase(), String(v * 2)])
    );
}
"#,
            json!({"obj": {"a": 1, "b": 2, "c": 3}}),
            json!({"A": "2", "B": "4", "C": "6"}),
        ),
        (
            r#"//native

export function main(): string {
    const data = JSON.stringify({ key: "value", nested: { arr: [1, 2, 3] } });
    const parsed = JSON.parse(data);
    return parsed.nested.arr.map((x: number) => x * 10).join(",");
}
"#,
            json!({}),
            json!("10,20,30"),
        ),
    ]
}

#[cfg(feature = "deno_core")]
async fn push_job(db: &Pool<Postgres>, content: &str, args: &serde_json::Value) -> Uuid {
    let mut hm_args = std::collections::HashMap::new();
    if let Some(obj) = args.as_object() {
        for (k, v) in obj {
            hm_args.insert(k.clone(), windmill_common::worker::to_raw_value(v));
        }
    }

    let job = JobPayload::Code(RawCode {
        hash: None,
        content: content.to_string(),
        path: None,
        language: ScriptLang::Bun, // //native annotation → nativets
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let tx = PushIsolationLevel::IsolatedRoot(db.clone());
    let (uuid, tx) = windmill_queue::push(
        db,
        tx,
        "test-workspace",
        job,
        windmill_queue::PushArgs::from(&hm_args),
        /* user */ "test-user",
        /* email */ "test@windmill.dev",
        /* permissioned_as */ "u/test-user".to_string(),
        /* token_prefix */ None,
        /* scheduled_for */ None,
        /* schedule_path */ None,
        /* parent_job */ None,
        /* root_job */ None,
        /* flow_innermost_root_job */ None,
        /* job_id */ None,
        /* is_flow_step */ false,
        /* same_worker */ false,
        None,
        true,
        None,
        None,
        None,
        None,
        None,
        false,
        None,
        None,
        None,
    )
    .await
    .expect("push must succeed");
    tx.commit().await.unwrap();
    uuid
}

#[cfg(feature = "deno_core")]
fn spawn_workers(
    conn: &Connection,
    port: u16,
    n: usize,
) -> (KillpillSender, Vec<tokio::task::JoinHandle<()>>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static WORKER_ID: AtomicUsize = AtomicUsize::new(0);

    std::fs::DirBuilder::new()
        .recursive(true)
        .create(windmill_worker::GO_BIN_CACHE_DIR)
        .expect("could not create initial worker dir");

    let (tx, _) = KillpillSender::new(n + 1);
    let mut handles = Vec::with_capacity(n);

    for i in 0..n {
        let rx = tx.subscribe();
        let conn = conn.clone();
        let tx2 = tx.clone();
        let id = WORKER_ID.fetch_add(1, Ordering::SeqCst);
        let worker_name = format!("{id}/stress-w{i}");

        let future = async move {
            let base_internal_url = format!("http://localhost:{}", port);
            {
                let mut wc = WORKER_CONFIG.write().await;
                wc.worker_tags = windmill_common::worker::DEFAULT_TAGS.clone();
                wc.priority_tags_sorted = vec![windmill_common::worker::PriorityTags {
                    priority: 0,
                    tags: wc.worker_tags.clone(),
                }];
                windmill_common::worker::store_suspended_pull_query(&wc).await;
                windmill_common::worker::store_pull_query(&wc).await;
            }
            windmill_worker::run_worker(
                &conn,
                "test-host",
                worker_name,
                i as u64,
                n as u32,
                "127.0.0.1",
                rx,
                tx2,
                &base_internal_url,
            )
            .await;
        };

        handles.push(tokio::task::spawn(future));
    }

    (tx, handles)
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_parallel_nativets_stress(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let conn = Connection::Sql(db.clone());

    // Set up completed job listener
    let mut listener = listen_for_completed_jobs(&db).await;

    // Prepare job scripts
    let scripts = job_scripts();

    // Also prepare fetch-based scripts that hit the internal API server
    let fetch_script_template = |port: u16| -> Vec<(String, serde_json::Value)> {
        vec![
            (
                format!(
                    r#"//native

export async function main(): Promise<object> {{
    const resp = await fetch("http://localhost:{port}/api/version");
    return {{ status: resp.status, ok: resp.ok }};
}}
"#
                ),
                json!({}),
            ),
            (
                format!(
                    r#"//native

export async function main(): Promise<string> {{
    const resp = await fetch("http://localhost:{port}/api/version");
    const text = await resp.text();
    return typeof text;
}}
"#
                ),
                json!({}),
            ),
        ]
    };

    let fetch_scripts = fetch_script_template(port);
    let total_jobs = 200;

    let start = Instant::now();
    let mut expected_results: Vec<(Uuid, Option<serde_json::Value>)> =
        Vec::with_capacity(total_jobs);

    for i in 0..total_jobs {
        let (uuid, expected) = if i % 10 < 8 {
            // 80% non-fetch jobs
            let idx = i % scripts.len();
            let (code, args, expected) = &scripts[idx];
            let uuid = push_job(&db, code, args).await;
            (uuid, Some(expected.clone()))
        } else {
            // 20% fetch jobs (we don't check exact result since version string varies)
            let idx = i % fetch_scripts.len();
            let (code, args) = &fetch_scripts[idx];
            let uuid = push_job(&db, code, args).await;
            (uuid, None) // Don't check exact value, just success
        };
        expected_results.push((uuid, expected));
    }

    let push_duration = start.elapsed();
    tracing::info!(
        "Pushed {} jobs in {:?} ({:?}/job)",
        total_jobs,
        push_duration,
        push_duration / total_jobs as u32
    );

    // Spawn 8 workers
    let (killpill, worker_handles) = spawn_workers(&conn, port, NUM_WORKERS);

    // Wait for all jobs to complete
    let mut completed: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    let timeout_dur = tokio::time::Duration::from_secs(120);
    let deadline = Instant::now() + timeout_dur;

    while completed.len() < total_jobs {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            panic!(
                "Timed out waiting for jobs: {}/{} completed",
                completed.len(),
                total_jobs
            );
        }

        match tokio::time::timeout(remaining, listener.next()).await {
            Ok(Some(uuid)) => {
                completed.insert(uuid);
                if completed.len() % 50 == 0 {
                    tracing::info!(
                        "Progress: {}/{} jobs completed",
                        completed.len(),
                        total_jobs
                    );
                }
            }
            Ok(None) => panic!("Listener stream ended"),
            Err(_) => panic!("Timed out: {}/{} completed", completed.len(), total_jobs),
        }
    }

    let exec_duration = start.elapsed();
    tracing::info!(
        "All {} jobs completed in {:?} with {} workers ({:?}/job avg)",
        total_jobs,
        exec_duration,
        NUM_WORKERS,
        exec_duration / total_jobs as u32
    );

    // Kill workers
    killpill.send();
    for handle in worker_handles {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(10), handle).await;
    }

    // Verify results
    let mut successes = 0;
    let mut failures = 0;

    for (uuid, expected) in &expected_results {
        let job = completed_job(*uuid, &db).await;
        if !job.success {
            failures += 1;
            tracing::error!("Job {} FAILED: {:?}", uuid, job.result);
            continue;
        }
        successes += 1;

        if let Some(expected_val) = expected {
            let result = job
                .json_result()
                .expect("successful job should have result");
            assert_eq!(
                result, *expected_val,
                "Job {} produced wrong result.\nExpected: {}\nGot: {}",
                uuid, expected_val, result
            );
        }
    }

    tracing::info!(
        "Results: {} successes, {} failures out of {} total",
        successes,
        failures,
        total_jobs
    );

    assert_eq!(failures, 0, "All jobs should succeed");
    assert_eq!(successes, total_jobs, "All jobs should be verified");

    Ok(())
}
