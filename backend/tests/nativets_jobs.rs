/*
 * Integration tests for nativets (//native) job execution.
 *
 * These tests ensure that the V8 runtime is properly initialized and that
 * nativets jobs execute correctly without segfaults. They cover sync functions,
 * async functions, and fetch operations.
 *
 * All nativets tests run inside a single #[sqlx::test] with a single long-lived
 * worker, because V8 isolates cannot be safely created/destroyed/recreated
 * across different tokio runtimes or worker lifecycles.
 *
 * Run with:
 *   cargo test -p windmill --features "deno_core" --test nativets_jobs -- --nocapture
 */

#[cfg(feature = "deno_core")]
use windmill_test_utils::*;

#[cfg(feature = "deno_core")]
use futures::StreamExt;
#[cfg(feature = "deno_core")]
use sqlx::{Pool, Postgres};
#[cfg(feature = "deno_core")]
use windmill_common::jobs::{JobPayload, RawCode};
#[cfg(feature = "deno_core")]
use windmill_common::scripts::ScriptLang;

#[cfg(feature = "deno_core")]
fn nativets_code(content: &str) -> JobPayload {
    JobPayload::Code(RawCode {
        hash: None,
        content: content.to_string(),
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    })
}

#[cfg(feature = "deno_core")]
async fn push_and_wait(
    db: &Pool<Postgres>,
    job: RunJob,
    listener: &mut (impl StreamExt<Item = uuid::Uuid> + Unpin),
) -> CompletedJob {
    let uuid = job.push(db).await;
    let deadline = std::time::Instant::now() + tokio::time::Duration::from_secs(60);
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            panic!("Timed out waiting for job {uuid}");
        }
        match tokio::time::timeout(remaining, listener.next()).await {
            Ok(Some(completed_uuid)) if completed_uuid == uuid => break,
            Ok(Some(_)) => continue,
            Ok(None) => panic!("Listener ended while waiting for {uuid}"),
            Err(_) => panic!("Timed out waiting for job {uuid}"),
        }
    }
    completed_job(uuid, db).await
}

/// All nativets tests share a single worker to avoid V8 isolate lifecycle issues.
#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_nativets_jobs(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let mut listener = listen_for_completed_jobs(&db).await;
    let conn = windmill_common::worker::Connection::Sql(db.clone());
    let (killpill, _worker_handle) = spawn_test_worker(&conn, port);

    // -- simple string return --
    {
        let result = push_and_wait(
            &db,
            RunJob::from(nativets_code(
                r#"//native

export function main(): string {
    return "hello from nativets";
}
"#,
            )),
            &mut listener,
        )
        .await;
        assert!(result.success, "simple_return failed: {:?}", result.result);
        assert_eq!(
            result.json_result().unwrap(),
            serde_json::json!("hello from nativets")
        );
    }

    // -- arithmetic with args --
    {
        let result = push_and_wait(
            &db,
            RunJob::from(nativets_code(
                r#"//native

export function main(x: number, y: number): number {
    return x + y;
}
"#,
            ))
            .arg("x", serde_json::json!(17))
            .arg("y", serde_json::json!(25)),
            &mut listener,
        )
        .await;
        assert!(result.success, "with_args failed: {:?}", result.result);
        assert_eq!(result.json_result().unwrap(), serde_json::json!(42));
    }

    // -- complex object return --
    {
        let result = push_and_wait(
            &db,
            RunJob::from(nativets_code(
                r#"//native

export function main(items: {name: string, value: number}[]): {total: number, names: string[]} {
    return {
        total: items.reduce((sum, i) => sum + i.value, 0),
        names: items.map(i => i.name).sort(),
    };
}
"#,
            ))
            .arg(
                "items",
                serde_json::json!([
                    {"name": "c", "value": 30},
                    {"name": "a", "value": 10},
                    {"name": "b", "value": 20}
                ]),
            ),
            &mut listener,
        )
        .await;
        assert!(result.success, "object_return failed: {:?}", result.result);
        assert_eq!(
            result.json_result().unwrap(),
            serde_json::json!({"total": 60, "names": ["a", "b", "c"]})
        );
    }

    // -- async function (Promise) --
    {
        let result = push_and_wait(
            &db,
            RunJob::from(nativets_code(
                r#"//native

export async function main(): Promise<number> {
    const result = await Promise.resolve(42);
    return result;
}
"#,
            )),
            &mut listener,
        )
        .await;
        assert!(result.success, "async failed: {:?}", result.result);
        assert_eq!(result.json_result().unwrap(), serde_json::json!(42));
    }

    // -- fetch hitting the internal API server --
    {
        let code = format!(
            r#"//native

export async function main(): Promise<object> {{
    const resp = await fetch("http://localhost:{port}/api/version");
    return {{ status: resp.status, ok: resp.ok }};
}}
"#
        );
        let result = push_and_wait(&db, RunJob::from(nativets_code(&code)), &mut listener).await;
        assert!(result.success, "fetch failed: {:?}", result.result);
        let val = result.json_result().unwrap();
        assert_eq!(val["ok"], serde_json::json!(true));
        assert_eq!(val["status"], serde_json::json!(200));
    }

    // -- regex --
    {
        let result = push_and_wait(
            &db,
            RunJob::from(nativets_code(
                r#"//native

export function main(text: string, pattern: string): string[] {
    const regex = new RegExp(pattern, 'g');
    return [...text.matchAll(regex)].map(m => m[0]);
}
"#,
            ))
            .arg("text", serde_json::json!("foo123bar456baz789"))
            .arg("pattern", serde_json::json!("\\d+")),
            &mut listener,
        )
        .await;
        assert!(result.success, "regex failed: {:?}", result.result);
        assert_eq!(
            result.json_result().unwrap(),
            serde_json::json!(["123", "456", "789"])
        );
    }

    // -- JSON roundtrip --
    {
        let result = push_and_wait(
            &db,
            RunJob::from(nativets_code(
                r#"//native

export function main(): string {
    const data = JSON.stringify({ key: "value", nested: { arr: [1, 2, 3] } });
    const parsed = JSON.parse(data);
    return parsed.nested.arr.map((x: number) => x * 10).join(",");
}
"#,
            )),
            &mut listener,
        )
        .await;
        assert!(result.success, "json_roundtrip failed: {:?}", result.result);
        assert_eq!(result.json_result().unwrap(), serde_json::json!("10,20,30"));
    }

    // -- no args, array return --
    {
        let result = push_and_wait(
            &db,
            RunJob::from(nativets_code(
                r#"//native

export function main(): number[] {
    return Array.from({length: 5}, (_, i) => i * i);
}
"#,
            )),
            &mut listener,
        )
        .await;
        assert!(result.success, "no_args failed: {:?}", result.result);
        assert_eq!(
            result.json_result().unwrap(),
            serde_json::json!([0, 1, 4, 9, 16])
        );
    }

    killpill.send();
    Ok(())
}
