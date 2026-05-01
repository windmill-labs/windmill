use serde::de::DeserializeOwned;
use serial_test::serial;

#[cfg(feature = "enterprise")]
use chrono::Timelike;

use serde::Deserialize;
use serde_json::json;
use sqlx::{types::Uuid, Pool, Postgres};

#[cfg(feature = "enterprise")]
use tokio::time::{timeout, Duration};

#[cfg(feature = "python")]
use windmill_api_client::types::{CreateFlowBody, RawScript};
#[cfg(feature = "enterprise")]
use windmill_api_client::types::{EditSchedule, NewSchedule, ScriptArgs};

#[cfg(feature = "deno_core")]
use windmill_common::flows::InputTransform;

#[cfg(any(feature = "python", feature = "deno_core"))]
use windmill_common::flow_status::RestartedFrom;
use windmill_common::{
    flows::FlowValue,
    jobs::{JobPayload, RawCode},
    scripts::ScriptLang,
};
use windmill_test_utils::*;

#[cfg(feature = "enterprise")]
use futures::StreamExt;
use windmill_common::flows::FlowModule;
use windmill_common::flows::FlowModuleValue;

// async fn _print_job(id: Uuid, db: &Pool<Postgres>) -> Result<(), anyhow::Error> {
//     tracing::info!(
//         "{:#?}",
//         get_job_by_id(db.begin().await?, "test-workspace", id)
//             .await?
//             .0
//     );
//     Ok(())
// }

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_iteration(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [{
            "value": {
                "type": "forloopflow",
                "iterator": { "type": "javascript", "expr": "result.items" },
                "skip_failures": false,
                "modules": [{
                    "value": {
                        "input_transforms": {
                            "n": {
                                "type": "javascript",
                                "expr": "flow_input.iter.value",
                            },
                        },
                        "type": "rawscript",
                        "language": "python3",
                        "content": "def main(n):\n    if 1 < n:\n        raise StopIteration(n)",
                    },
                }],
            },
        }],
    }))
    .unwrap();

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("items", json!([]))
            .run_until_complete(&db, false, server.addr.port())
            .await
            .json_result()
            .unwrap();
    assert_eq!(result, serde_json::json!([]));

    /* Don't actually test that this does 257 jobs or that will take forever. */
    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("items", json!((0..257).collect::<Vec<_>>()))
            .run_until_complete(&db, false, server.addr.port())
            .await
            .json_result()
            .unwrap();
    assert!(matches!(result, serde_json::Value::Array(_)));
    assert!(result[2]["error"]
        .as_object()
        .unwrap()
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("2"));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_iteration_parallel(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [{
            "value": {
                "type": "forloopflow",
                "iterator": { "type": "javascript", "expr": "result.items" },
                "skip_failures": false,
                "parallel": true,
                "modules": [{
                    "value": {
                        "input_transforms": {
                            "n": {
                                "type": "javascript",
                                "expr": "flow_input.iter.value",
                            },
                        },
                        "type": "rawscript",
                        "language": "python3",
                        "content": "def main(n):\n    if 1 < n:\n        raise StopIteration(n)",
                    },
                }],
            },
        }],
    }))
    .unwrap();

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("items", json!([]))
            .run_until_complete(&db, false, server.addr.port())
            .await
            .json_result()
            .unwrap();
    assert_eq!(result, serde_json::json!([]));

    /* Don't actually test that this does 257 jobs or that will take forever. */
    let job =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("items", json!((0..50).collect::<Vec<_>>()))
            .run_until_complete(&db, false, server.addr.port())
            .await;
    // println!("{:#?}", job);
    let result = job.json_result().unwrap();
    assert!(matches!(result, serde_json::Value::Array(_)));
    assert!(result[2]["error"]
        .as_object()
        .unwrap()
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("2"));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_deno_flow(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    let numbers = "export function main() { return [1, 2, 3]; }";
    let doubles = "export function main(n) { return n * 2; }";

    let flow = {
        use windmill_common::flows::{FlowModule, FlowModuleValue};

        FlowValue {
            modules: vec![
                FlowModule {
                    id: "a".to_string(),
                    value: FlowModuleValue::RawScript {
                        input_transforms: Default::default(),
                        language: ScriptLang::Deno,
                        content: numbers.to_string(),
                        path: None,
                        lock: None,
                        tag: None,
                        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
                            .into(),
                        is_trigger: None,
                        assets: None,
                    }
                    .into(),
                    stop_after_if: Default::default(),
                    stop_after_all_iters_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    cache_ignore_s3_path: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    delete_after_secs: None,
                    continue_on_error: None,
                    skip_if: None,
                    apply_preprocessor: None,
                    pass_flow_input_directly: None,
                    debouncing: None,
                },
                FlowModule {
                    id: "b".to_string(),
                    value: FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Javascript { expr: "result".to_string() },
                        skip_failures: false,
                        parallel: false,
                        squash: None,
                        parallelism: None,
                        modules: vec![FlowModule {
                            id: "c".to_string(),
                            value: FlowModuleValue::RawScript {
                                input_transforms: [(
                                    "n".to_string(),
                                    InputTransform::Javascript {
                                        expr: "flow_input.iter.value".to_string(),
                                    },
                                )]
                                .into(),
                                language: ScriptLang::Deno,
                                content: doubles.to_string(),
                                path: None,
                                lock: None,
                                tag: None,
                                concurrency_settings:
                                    windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                                is_trigger: None,
                                assets: None,
                            }
                            .into(),
                            stop_after_if: Default::default(),
                            stop_after_all_iters_if: Default::default(),
                            summary: Default::default(),
                            suspend: Default::default(),
                            retry: None,
                            sleep: None,
                            cache_ttl: None,
                            cache_ignore_s3_path: None,
                            mock: None,
                            timeout: None,
                            priority: None,
                            delete_after_use: None,
                    delete_after_secs: None,
                            continue_on_error: None,
                            skip_if: None,
                            apply_preprocessor: None,
                            pass_flow_input_directly: None,
                            debouncing: None,
                        }],
                        modules_node: None,
                    }
                    .into(),
                    stop_after_if: Default::default(),
                    stop_after_all_iters_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    cache_ignore_s3_path: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    delete_after_secs: None,
                    continue_on_error: None,
                    skip_if: None,
                    apply_preprocessor: None,
                    pass_flow_input_directly: None,
                    debouncing: None,
                },
            ],
            same_worker: false,
            ..Default::default()
        }
    };

    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let port = server.addr.port();

    for i in 0..50 {
        println!("deno flow iteration: {}", i);
        let job = run_job_in_new_worker_until_complete(&db, false, job.clone(), port).await;
        // println!("job: {:#?}", job.flow_status);
        let result = job.json_result().unwrap();
        assert_eq!(result, serde_json::json!([2, 4, 6]), "iteration: {}", i);
    }
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_identity(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [{
                "value": {
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(): return 42",
                }}, {
                    "value": {
                        "type": "identity",
                    },
                }, {
                    "value": {
                        "type": "identity",
                    },
                }, {
                    "value": {
                        "type": "identity",
                    },
                }],
    }))
    .unwrap();

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .run_until_complete(&db, false, server.addr.port())
            .await
            .json_result()
            .unwrap();
    assert_eq!(result, serde_json::json!(42));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_deno_flow_same_worker(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    let write_file = r#"export async function main(loop: boolean, i: number, path: string) {
            await Deno.writeTextFile(`./shared/${path}`, `${loop} ${i}`);
        }"#
    .to_string();

    let flow = FlowValue {
            modules: vec![
                FlowModule {
                    id: "a".to_string(),
                    value: FlowModuleValue::RawScript {
                        input_transforms: [
                            (
                                "loop".to_string(),
                                InputTransform::Static { value: windmill_common::worker::to_raw_value(&false) },
                            ),
                            ("i".to_string(), InputTransform::Static { value: windmill_common::worker::to_raw_value(&1) }),
                            (
                                "path".to_string(),
                                InputTransform::Static { value: windmill_common::worker::to_raw_value(&"outer.txt") },
                            ),
                        ]
                        .into(),
                        language: ScriptLang::Deno,
                        content: write_file.clone(),
                        path: None,
                        lock: None,
                        tag: None,
                        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                        is_trigger: None,
                        assets: None,

                    }.into(),
                    stop_after_if: Default::default(),
                    stop_after_all_iters_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    cache_ignore_s3_path: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    delete_after_secs: None,
                    continue_on_error: None,
                    skip_if: None,
                    apply_preprocessor: None,
                    pass_flow_input_directly: None,
                    debouncing: None,
                },
                FlowModule {
                    id: "b".to_string(),
                    value: FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Static { value: windmill_common::worker::to_raw_value(&[1, 2, 3]) },
                        skip_failures: false,
                        parallel: false,
                        squash: None,
                        parallelism: None,
                        modules: vec![
                            FlowModule {
                                id: "d".to_string(),
                                value: FlowModuleValue::RawScript {
                                    input_transforms: [
                                        (
                                            "i".to_string(),
                                            InputTransform::Javascript {
                                                expr: "flow_input.iter.value".to_string(),
                                            },
                                        ),
                                        (
                                            "loop".to_string(),
                                            InputTransform::Static { value: windmill_common::worker::to_raw_value(&true) },
                                        ),
                                        (
                                            "path".to_string(),
                                            InputTransform::Static { value: windmill_common::worker::to_raw_value(&"inner.txt") },
                                        ),
                                    ]
                                    .into(),
                                    language: ScriptLang::Deno,
                                    content: write_file,
                                    path: None,
                                    lock: None,
                                    tag: None,
                                    concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                                    is_trigger: None,
                                    assets: None,
                                }.into(),
                                stop_after_if: Default::default(),
                                stop_after_all_iters_if: Default::default(),
                                summary: Default::default(),
                                suspend: Default::default(),
                                retry: None,
                                sleep: None,
                                cache_ttl: None,
                                cache_ignore_s3_path: None,
                                mock: None,
                                timeout: None,
                                priority: None,
                                delete_after_use: None,
                    delete_after_secs: None,
                                continue_on_error: None,
                                skip_if: None,
                                apply_preprocessor: None,
                                pass_flow_input_directly: None,
                                debouncing: None,
                            },
                            FlowModule {
                                id: "e".to_string(),
                                value: FlowModuleValue::RawScript {
                                    input_transforms: [(
                                        "path".to_string(),
                                        InputTransform::Static { value: windmill_common::worker::to_raw_value(&"inner.txt") },
                                    ), (
                                        "path2".to_string(),
                                        InputTransform::Static { value: windmill_common::worker::to_raw_value(&"outer.txt") },
                                    )]
                                    .into(),
                                    language: ScriptLang::Deno,
                                    content: r#"export async function main(path: string, path2: string) {
                                        return await Deno.readTextFile(`./shared/${path}`) + "," + await Deno.readTextFile(`./shared/${path2}`);
                                    }"#
                                    .to_string(),
                                    path: None,
                                    lock: None,
                                    tag: None,
                                    concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                                    is_trigger: None,
                                    assets: None,

                                }.into(),
                                stop_after_if: Default::default(),
                                stop_after_all_iters_if: Default::default(),
                                summary: Default::default(),
                                suspend: Default::default(),
                                retry: None,
                                sleep: None,
                                cache_ttl: None,
                                cache_ignore_s3_path: None,
                                mock: None,
                                timeout: None,
                                priority: None,
                                delete_after_use: None,
                    delete_after_secs: None,
                                continue_on_error: None,
                                skip_if: None,
                                apply_preprocessor: None,
                                pass_flow_input_directly: None,
                                debouncing: None,
                            },
                        ],
                        modules_node: None,
                    }.into(),
                    stop_after_if: Default::default(),
                    stop_after_all_iters_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    cache_ignore_s3_path: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    delete_after_secs: None,
                    continue_on_error: None,
                    skip_if: None,
                    apply_preprocessor: None,
                    pass_flow_input_directly: None,
                    debouncing: None,
                },
                FlowModule {
                    id: "c".to_string(),
                    value: FlowModuleValue::RawScript {
                        input_transforms: [
                            (
                                "loops".to_string(),
                                InputTransform::Javascript { expr: "results.b".to_string() },
                            ),
                            (
                                "path".to_string(),
                                InputTransform::Static { value: windmill_common::worker::to_raw_value(&"outer.txt") },
                            ),
                            (
                                "path2".to_string(),
                                InputTransform::Static { value: windmill_common::worker::to_raw_value(&"inner.txt") },
                            ),
                        ]
                        .into(),
                        language: ScriptLang::Deno,
                        content: r#"export async function main(path: string, loops: string[], path2: string) {
                            return await Deno.readTextFile(`./shared/${path}`) + "," + loops + "," + await Deno.readTextFile(`./shared/${path2}`);
                        }"#
                        .to_string(),
                        path: None,
                        lock: None,
                        tag: None,
                        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                        is_trigger: None,
                        assets: None,
                    }.into(),
                    stop_after_if: Default::default(),
                    stop_after_all_iters_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    cache_ignore_s3_path: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    delete_after_secs: None,
                    continue_on_error: None,
                    skip_if: None,
                    apply_preprocessor: None,
                    pass_flow_input_directly: None,
                    debouncing: None,
                },
            ],
            same_worker: true,
            ..Default::default()
        };

    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };

    let result = run_job_in_new_worker_until_complete(&db, false, job.clone(), server.addr.port())
        .await
        .json_result()
        .unwrap();
    assert_eq!(
        result,
        serde_json::json!("false 1,true 1,false 1,true 2,false 1,true 3,false 1,true 3")
    );
    Ok(())
}
#[sqlx::test(fixtures("base"))]
async fn test_flow_result_by_id(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server: ApiServer = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
            "modules": [
                {
                    "id": "a",
                    "value": {
                        "type": "rawscript",
                        "language": "deno",
                        "content": "export function main(){ return 42 }",
                    }
                },
                {
                    "value": {
                        "branches": [
                            {
                                "modules": [{
                                    "value": {
                                        "branches": [{"modules": [                {
                                            "id": "d",
                                            "value": {
                                                "input_transforms": {"v": {"type": "javascript", "expr": "results.a"}},
                                                "type": "rawscript",
                                                "language": "deno",
                                                "content": "export function main(v){ return v }",
                                            }

                                        },]}],
                                        "type": "branchall",
                                    }
                                }],
                            }],
                            "type": "branchall",
                        },
                    }
            ],
        }))
        .unwrap();

    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, job.clone(), port)
        .await
        .json_result()
        .unwrap();
    assert_eq!(result, serde_json::json!([[42]]));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_stop_after_if(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    // let server = ApiServer::start(db.clone()).await?;
    // let port = server.addr.port();

    let port = 123;
    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "input_transforms": { "n": { "type": "javascript", "expr": "flow_input.n" } },
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(n): return n",
                },
                "stop_after_if": {
                    "expr": "result < 0",
                    "skip_if_stopped": false,
                },
            },
            {
                "id": "b",
                "value": {
                    "input_transforms": { "n": { "type": "javascript", "expr": "results.a" } },
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(n): return f'last step saw {n}'",
                },
            },
        ],
    }))
    .unwrap();
    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };

    let result = RunJob::from(job.clone())
        .arg("n", json!(123))
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
    assert_eq!(json!("last step saw 123"), result);

    let cjob = RunJob::from(job.clone())
        .arg("n", json!(-123))
        .run_until_complete(&db, false, port)
        .await;

    let result = cjob.json_result().unwrap();
    assert_eq!(json!(-123), result);
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_stop_after_if_nested(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    // let server = ApiServer::start(db.clone()).await?;
    // let port = server.addr.port();

    let port = 123;
    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "branches": [{"modules": [{
                    "id": "b",
                    "value": {
                        "input_transforms": { "n": { "type": "javascript", "expr": "flow_input.n" } },
                        "type": "rawscript",
                        "language": "python3",
                        "content": "def main(n): return n",
                    },
                    "stop_after_if": {
                        "expr": "result < 0",
                        "skip_if_stopped": false,
                    }}]}],
                    "type": "branchall",
                    "parallel": false,
                },
            },
            {
                "id": "c",
                "value": {
                    "input_transforms": { "n": { "type": "javascript", "expr": "results.a" } },
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(n): return f'last step saw {n}'",
                },
            },
        ],
    }))
    .unwrap();
    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };

    let result = RunJob::from(job.clone())
        .arg("n", json!(123))
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
    assert_eq!(json!("last step saw [123]"), result);

    let cjob = RunJob::from(job.clone())
        .arg("n", json!(-123))
        .run_until_complete(&db, false, port)
        .await;

    let result = cjob.json_result().unwrap();
    assert_eq!(json!([-123]), result);
    Ok(())
}

#[cfg(all(feature = "deno_core", feature = "python"))]
#[sqlx::test(fixtures("base"))]
async fn test_python_flow(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let numbers = "def main(): return [1, 2, 3]";
    let doubles = "def main(n): return n * 2";

    let flow: FlowValue = serde_json::from_value(serde_json::json!( {
        "modules": [
            {
                "value": {
                    "type": "rawscript",
                    "language": "python3",
                    "content": numbers,
                },
            },
            {
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "result" },
                    "skip_failures": false,
                    "modules": [{
                        "value": {
                            "input_transforms": {
                                "n": {
                                    "type": "javascript",
                                    "expr": "flow_input.iter.value",
                                },
                            },
                            "type": "rawscript",
                            "language": "python3",
                            "content": doubles,
                        },
                    }],
                },
            },
        ],
    }))
    .unwrap();

    for i in 0..10 {
        println!("python flow iteration: {}", i);
        let result = run_job_in_new_worker_until_complete(
            &db,
            false,
            JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None },
            port,
        )
        .await
        .json_result()
        .unwrap();

        assert_eq!(result, serde_json::json!([2, 4, 6]), "iteration: {i}");
    }
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_flow_2(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [
                {
                    "value": {
                        "input_transforms": {},
                        "type": "rawscript",
                        "content": "import wmill\ndef main():  return \"Hello\"",
                        "language": "python3"
                    },
                }
            ]
    }))
    .unwrap();

    for i in 0..10 {
        println!("python flow iteration: {}", i);
        let result = run_job_in_new_worker_until_complete(
            &db,
            false,
            JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None },
            port,
        )
        .await
        .json_result()
        .unwrap();

        assert_eq!(result, serde_json::json!("Hello"), "iteration: {i}");
    }
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_go_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
package inner

import "fmt"

func main(derp string) (string, error) {
	fmt.Println("Hello, 世界")
	return fmt.Sprintf("hello %s", derp), nil
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Go,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("derp", json!("world"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("hello world"));
    Ok(())
}

#[cfg(feature = "rust")]
#[sqlx::test(fixtures("base"))]
async fn test_rust_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
fn main(world: String) -> Result<String, String> {
    println!("Which world to greet today?");
    Ok(format!("Hello {}!", world))
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Rust,
        cache_ignore_s3_path: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        dedicated_worker: None,
        modules: None,
    }))
    .arg("world", json!("Hyrule"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("Hello Hyrule!"));
    Ok(())
}

#[cfg(feature = "csharp")]
#[sqlx::test(fixtures("base"))]
async fn test_csharp_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
using System;

class Script
{
    public static string Main(string world, int b = 2)
    {
        Console.WriteLine($"Hello {world} - {b}. This is a log line");
        return $"Hello {world} - {b}";
    }
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::CSharp,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("world", json!("Arakis"))
    .arg("b", json!(3))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("Hello Arakis - 3"));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_bash_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
msg="$1"
echo "hello $msg"
"#
    .to_owned();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Bash,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("msg", json!("world"))
    .run_until_complete(&db, false, port)
    .await;
    assert_eq!(job.json_result(), Some(json!("hello world")));
    Ok(())
}

#[sqlx::test(fixtures("base", "wmill_cli_test"))]
async fn test_bash_wmill_variable_get(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // The bash script uses wmill CLI to get the variable value.
    // The worker sets WM_TOKEN, WM_WORKSPACE, and BASE_INTERNAL_URL as env vars,
    // and the CLI auto-configures from them when no workspace is explicitly set.
    // We point WMILL_CONFIG_DIR to a clean temp dir so no local active workspace interferes.
    let content = r#"
export WMILL_CONFIG_DIR=$(mktemp -d)
result=$(wmill variable get "u/test-user/test_var" --json | jq -r .value)
echo "$result"
"#
    .to_owned();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Bash,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .run_until_complete(&db, false, port)
    .await;
    assert_eq!(job.json_result(), Some(json!("hello from variable")));
    Ok(())
}

#[sqlx::test(fixtures("base", "wmill_cli_test"))]
async fn test_bash_wmill_resource_get(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // The bash script uses wmill CLI to get the resource value.
    // We point WMILL_CONFIG_DIR to a clean temp dir so no local active workspace interferes.
    let content = r#"
export WMILL_CONFIG_DIR=$(mktemp -d)
result=$(wmill resource get "u/test-user/test_res" --json | jq -c .value)
echo "$result"
"#
    .to_owned();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Bash,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .run_until_complete(&db, false, port)
    .await;
    // Bash echo outputs are returned as strings, so the JSON is a string value
    assert_eq!(
        job.json_result(),
        Some(json!("{\"host\":\"localhost\",\"port\":5432}"))
    );
    Ok(())
}

#[cfg(feature = "rlang")]
#[sqlx::test(fixtures("base"))]
async fn test_r_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
main <- function(msg) {
    return(paste("hello", msg))
}
"#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Rlang,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("msg", json!("world"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, json!("hello world"));
    Ok(())
}

#[cfg(feature = "rlang")]
#[sqlx::test(fixtures("base", "wmill_cli_test"))]
async fn test_r_get_variable(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
main <- function() {
    return(get_variable("u/test-user/test_var"))
}
"#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Rlang,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, json!("hello from variable"));
    Ok(())
}

#[cfg(feature = "rlang")]
#[sqlx::test(fixtures("base", "wmill_cli_test"))]
async fn test_r_get_resource(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
main <- function() {
    return(get_resource("u/test-user/test_res"))
}
"#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Rlang,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, json!({"host": "localhost", "port": 5432}));
    Ok(())
}

#[cfg(feature = "nu")]
#[sqlx::test(fixtures("base"))]
async fn test_nu_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
def main [ msg: string ] {
    "hello " + $msg
}
"#
    .to_owned();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Nu,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("msg", json!("world"))
    .run_until_complete(&db, false, port)
    .await;
    assert_eq!(job.json_result(), Some(json!("hello world")));
    Ok(())
}

#[cfg(feature = "nu")]
#[sqlx::test(fixtures("base"))]
async fn test_nu_job_full(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
def main [
  # Required
  ## Primitive
  a
  b: any
  c: bool
  d: float
  e: datetime
  f: string
  j: nothing
  ## Nesting
  g: record
  h: list<string>
  i: table
  # Optional
  m?
  n = "foo"
  o: any = "foo"
  p?: any
  # TODO: ...x
 ] {
    0
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Nu,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("a", json!("3"))
    .arg("b", json!("null"))
    .arg("c", json!(true))
    .arg("d", json!(3.0))
    .arg("e", json!("2024-09-24T10:00:00.000Z"))
    .arg("f", json!("str"))
    .arg("j", json!(null))
    .arg("g", json!({"a": 32}))
    .arg("h", json!(["foo"]))
    .arg(
        "i",
        json!([
            {"a": 1, "b": "foo", "c": true},
            {"a": 2, "b": "baz", "c": false}
        ]),
    )
    .arg("n", json!("baz"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!(0));
    Ok(())
}

#[cfg(feature = "java")]
#[sqlx::test(fixtures("base"))]
async fn test_java_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
public class Main {
  public static Object main(
    // Primitive
    int a,
    float b,
    // Objects
    Integer age,
    Float d
    ){
    return "hello world";
  }
}

"#
    .to_owned();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Java,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("a", json!(3))
    .arg("b", json!(3.0))
    .arg("age", json!(30))
    .arg("d", json!(3.0))
    .run_until_complete(&db, false, port)
    .await;
    assert_eq!(job.json_result(), Some(json!("hello world")));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_nativets_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export async function main(name: string): Promise<string> {
    return `hello ${name}`;
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Nativets,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("name", json!("world"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("hello world"));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_nativets_job_with_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export async function main(a: number, b: number): Promise<number> {
    return a + b;
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Nativets,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("a", json!(3))
    .arg("b", json!(7))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!(10));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_nativets_job_object_return(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export async function main(items: string[]): Promise<{ count: number; items: string[] }> {
    return { count: items.length, items };
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Nativets,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("items", json!(["a", "b", "c"]))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, json!({"count": 3, "items": ["a", "b", "c"]}));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_nativets_job_datetime(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // nativets passes Date-typed args as strings (no auto-conversion unlike Bun/Deno)
    let content = r#"
export async function main(a: Date) {
    return typeof a;
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Nativets,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("a", json!("2024-09-24T10:00:00.000Z"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("string"));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
#[serial(pg_cache)]
async fn test_postgresql_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    windmill_worker::pg_executor::clear_pg_cache().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
-- $1 name
SELECT 'hello ' || $1::text AS result;
"#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Postgresql,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("name", json!("world"))
    .arg(
        "database",
        json!({"host": "localhost", "port": 5432, "dbname": "windmill", "user": "postgres", "password": "changeme"}),
    )
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, json!([{"result": "hello world"}]));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
#[serial(pg_cache)]
async fn test_postgresql_cached_connection_resets_session(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use std::sync::atomic::Ordering;
    use windmill_worker::pg_executor::{clear_pg_cache, CACHE_HITS};

    initialize_tracing().await;

    // Clear stale connections from prior tests.
    clear_pg_cache().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let hits_before = CACHE_HITS.load(Ordering::Relaxed);

    let db_arg = json!({"host": "localhost", "port": 5432, "dbname": "windmill", "user": "postgres", "password": "changeme"});

    let make_pg_job = |content: String| {
        RunJob::from(JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            lock: None,
            language: ScriptLang::Postgresql,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            modules: None,
        }))
        .arg("database", db_arg.clone())
    };

    // Run 3 jobs against the same database resource so that at least the
    // second→third transition exercises the cached-connection path.
    //
    // Job 1: warm up — creates and (if eligible) caches the connection.
    let result1 = make_pg_job("SELECT 1 as n;".into())
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
    assert_eq!(result1, json!([{"n": 1}]));

    // Job 2: mutate session state — set a custom search_path.
    let result2 = make_pg_job(
        "SET search_path TO pg_catalog;\nSELECT current_setting('search_path') as sp;".into(),
    )
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();
    assert_eq!(
        result2,
        json!([{"sp": "pg_catalog"}]),
        "second job should see the custom search_path it just set"
    );

    // Job 3: read search_path — RESET ALL (on cached conn) should have restored
    // the default.
    let result3 = make_pg_job("SELECT current_setting('search_path') as sp;".into())
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
    let sp = result3[0]["sp"].as_str().unwrap();
    assert_ne!(
        sp, "pg_catalog",
        "search_path must be reset between jobs, got: {sp}"
    );

    // Verify the cached-connection path was actually exercised.
    let hits_after = CACHE_HITS.load(Ordering::Relaxed);
    assert!(
        hits_after > hits_before,
        "expected at least one cache hit across the 3 jobs, got {hits_before} -> {hits_after}"
    );

    Ok(())
}

/// Runs multiple PG jobs through a SINGLE worker (like production) to verify
/// that SET ROLE / search_path changes do not leak across jobs.
#[sqlx::test(fixtures("base"))]
#[serial(pg_cache)]
async fn test_postgresql_single_worker_session_isolation(db: Pool<Postgres>) -> anyhow::Result<()> {
    use std::sync::atomic::Ordering;
    use windmill_common::worker::Connection;
    use windmill_worker::pg_executor::{clear_pg_cache, CACHE_HITS};

    initialize_tracing().await;
    clear_pg_cache().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let hits_before = CACHE_HITS.load(Ordering::Relaxed);

    let db_arg = json!({"host": "localhost", "port": 5432, "dbname": "windmill", "user": "postgres", "password": "changeme"});

    let make_pg_job = |content: String| {
        RunJob::from(JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            lock: None,
            language: ScriptLang::Postgresql,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            modules: None,
        }))
        .arg("database", db_arg.clone())
    };

    // Push all 3 jobs into the queue BEFORE starting the worker, so a single
    // worker processes them all sequentially — matching production behaviour.
    let id1 = make_pg_job("SELECT current_user as cu;".into())
        .push(&db)
        .await;
    let id2 = make_pg_job("SET ROLE postgres;\nSET search_path TO pg_catalog;\nSELECT current_user as cu, current_setting('search_path') as sp;".into())
        .push(&db)
        .await;
    let id3 =
        make_pg_job("SELECT current_user as cu, current_setting('search_path') as sp;".into())
            .push(&db)
            .await;

    // Run ONE worker that processes all three jobs.
    let ids = [id1, id2, id3];
    let listener = listen_for_completed_jobs(&db).await;
    in_test_worker(
        Connection::Sql(db.clone()),
        async {
            use futures::StreamExt;
            let mut remaining = ids
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>();
            let mut listener = listener;
            while !remaining.is_empty() {
                if let Some(id) = listener.next().await {
                    remaining.remove(&id);
                }
            }
        },
        port,
    )
    .await;

    let r1 = completed_job(id1, &db).await.json_result().unwrap();
    let r2 = completed_job(id2, &db).await.json_result().unwrap();
    let r3 = completed_job(id3, &db).await.json_result().unwrap();

    // Job 1: baseline — should be the connecting user (postgres)
    assert_eq!(r1[0]["cu"], "postgres");

    // Job 2: SET ROLE + SET search_path took effect within the job
    assert_eq!(r2[0]["sp"], "pg_catalog");

    // Job 3: neither the role nor the search_path should leak from job 2
    let cu3 = r3[0]["cu"].as_str().unwrap();
    let sp3 = r3[0]["sp"].as_str().unwrap();
    assert_eq!(
        cu3, "postgres",
        "SET ROLE must not leak across jobs, got: {cu3}"
    );
    assert_ne!(
        sp3, "pg_catalog",
        "search_path must not leak across jobs, got: {sp3}"
    );

    // Verify caching actually happened (jobs 2 and 3 should hit the cache).
    let hits_after = CACHE_HITS.load(Ordering::Relaxed);
    assert!(
        hits_after > hits_before,
        "expected cache hits, got {hits_before} -> {hits_after}"
    );

    Ok(())
}

/// Runs 100 varied PG jobs through a single worker, verifying every job
/// succeeds, the cache is used for 99 of them, and session state never leaks.
#[sqlx::test(fixtures("base"))]
#[serial(pg_cache)]
async fn test_postgresql_100_jobs_cached(db: Pool<Postgres>) -> anyhow::Result<()> {
    use std::sync::atomic::Ordering;
    use windmill_common::worker::Connection;
    use windmill_worker::pg_executor::{clear_pg_cache, CACHE_HITS};

    initialize_tracing().await;
    clear_pg_cache().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let hits_before = CACHE_HITS.load(Ordering::Relaxed);

    let db_arg = json!({"host": "localhost", "port": 5432, "dbname": "windmill", "user": "postgres", "password": "changeme"});

    let make_pg_job = |content: String| {
        RunJob::from(JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            lock: None,
            language: ScriptLang::Postgresql,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            modules: None,
        }))
        .arg("database", db_arg.clone())
    };

    const N: usize = 100;
    let mut ids = Vec::with_capacity(N);

    // Push 100 varied jobs — mix of session-mutating and read-only queries.
    for i in 0..N {
        let content = match i % 5 {
            // Plain SELECT with different values each time
            0 => format!("SELECT {} as n, current_user as cu, current_setting('search_path') as sp;", i),
            // SET search_path then read it
            1 => format!("SET search_path TO pg_catalog;\nSELECT {} as n, current_setting('search_path') as sp;", i),
            // SET ROLE then read it
            2 => "SET ROLE postgres;\nSELECT current_user as cu;".to_string(),
            // Multi-statement
            3 => format!("SELECT 1;\nSELECT {} as n;", i),
            // Read-only with math
            _ => format!("SELECT {} + {} as n;", i, i * 2),
        };
        ids.push(make_pg_job(content).push(&db).await);
    }

    // Run ONE worker for all 100 jobs.
    let listener = listen_for_completed_jobs(&db).await;
    let id_set = ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    in_test_worker(
        Connection::Sql(db.clone()),
        async {
            use futures::StreamExt;
            let mut remaining = id_set;
            let mut listener = listener;
            while !remaining.is_empty() {
                if let Some(id) = listener.next().await {
                    remaining.remove(&id);
                }
            }
        },
        port,
    )
    .await;

    // Verify all 100 jobs succeeded.
    for (i, id) in ids.iter().enumerate() {
        let cjob = completed_job(*id, &db).await;
        assert!(cjob.success, "job {i} (id={id}) failed: {:?}", cjob.result);
    }

    // Spot-check: jobs that followed a SET search_path should NOT see pg_catalog.
    // Pattern: job i%5==1 sets search_path, job i%5==2 follows — should be clean.
    for i in (2..N).step_by(5) {
        let cjob = completed_job(ids[i], &db).await;
        let result = cjob.json_result().unwrap();
        let cu = result[0]["cu"].as_str().unwrap_or("N/A");
        assert_eq!(cu, "postgres", "job {i}: SET ROLE leaked, got {cu}");
    }
    for i in (0..N).step_by(5) {
        let cjob = completed_job(ids[i], &db).await;
        let result = cjob.json_result().unwrap();
        let sp = result[0]["sp"].as_str().unwrap_or("N/A");
        assert_ne!(sp, "pg_catalog", "job {i}: search_path leaked, got {sp}");
    }

    // Verify caching: first job creates the connection, remaining 99 should hit cache.
    let hits_after = CACHE_HITS.load(Ordering::Relaxed);
    let new_hits = hits_after - hits_before;
    assert!(
        new_hits >= (N as u64 - 1),
        "expected at least {} cache hits, got {new_hits}",
        N - 1
    );

    Ok(())
}

/// Cover the (Value × arg_t) combinations that #8988 broke. Each shape mirrors
/// what the windmill-client SDK or a hand-written PG script can emit:
///
/// - bare `$N` with no inline cast and no `-- $N name (type)` declaration
///   (parser defaults the otyp to "text"). The user's value can be any JSON
///   shape; the eventual column type is whatever the SQL context implies.
/// - inline `$N::TYPE` casts (the SDK's default for bare `${value}`).
/// - `CAST($N AS T)` syntax (the SDK strips its own cast when this pattern
///   surrounds the value, so the parser sees a bare `$N`).
/// - explicit declaration: `-- $N name (type)`.
///
/// Pre-fix, the dispatch asserted `Type::TEXT` for parser-defaulted args, so
/// e.g. a `Value::Bool` bound to `Box<bool>` failed at the encoder with
/// "cannot convert between the Rust type `bool` and the Postgres type `text`"
/// before the query ever reached the server.
#[sqlx::test(fixtures("base"))]
#[serial(pg_cache)]
async fn test_postgresql_arg_type_combinations(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_worker::pg_executor::clear_pg_cache;

    initialize_tracing().await;
    clear_pg_cache().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let db_arg = json!({"host": "localhost", "port": 5432, "dbname": "windmill", "user": "postgres", "password": "changeme"});

    // Build a fresh schema for this test so we don't trip over prior runs.
    let setup = r#"
DROP SCHEMA IF EXISTS wm_pg_arg_combo_test CASCADE;
CREATE SCHEMA wm_pg_arg_combo_test;
CREATE TABLE wm_pg_arg_combo_test.bugbool (flag bool);
CREATE TABLE wm_pg_arg_combo_test.sdkbug (n int, f double precision);
CREATE TABLE wm_pg_arg_combo_test.bugmix (id int, name text, payload jsonb, tags text[]);
CREATE TABLE wm_pg_arg_combo_test.allcols (
  c_bool bool,
  c_int2 smallint,
  c_int4 int,
  c_int8 bigint,
  c_float4 real,
  c_float8 double precision,
  c_numeric numeric,
  c_text text,
  c_varchar varchar(64),
  c_uuid uuid,
  c_date date,
  c_time time,
  c_ts timestamp,
  c_tstz timestamptz,
  c_json json,
  c_jsonb jsonb,
  c_int_arr int[],
  c_text_arr text[]
);
CREATE TYPE wm_pg_arg_combo_test.color AS ENUM ('red','green','blue');
CREATE TABLE wm_pg_arg_combo_test.enumtbl (c wm_pg_arg_combo_test.color);
"#;
    RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: setup.to_owned(),
        path: None,
        lock: None,
        language: ScriptLang::Postgresql,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("database", db_arg.clone())
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    // Each case: (name, content, args, expected_result)
    let cases: Vec<(&str, String, serde_json::Value, serde_json::Value)> = vec![
        // === parser-default "text" otyp (bare $N), value drives the binding ===
        (
            "bool via CAST AS bool (parser default text)",
            "-- $1 arg1\nINSERT INTO wm_pg_arg_combo_test.bugbool VALUES (CAST($1 AS bool)) RETURNING flag".to_owned(),
            json!({"arg1": true}),
            json!([{"flag": true}]),
        ),
        (
            "bare $1 with bool into bool col",
            "-- $1 arg1\nINSERT INTO wm_pg_arg_combo_test.bugbool VALUES ($1) RETURNING flag".to_owned(),
            json!({"arg1": false}),
            json!([{"flag": false}]),
        ),
        (
            "bare $1 with bool into text via implicit cast bool->text",
            "-- $1 arg1\nSELECT $1::text AS s".to_owned(),
            json!({"arg1": true}),
            json!([{"s": "true"}]),
        ),
        (
            "bare $1 with int into text via implicit cast int->text",
            "-- $1 arg1\nSELECT $1::text AS s".to_owned(),
            json!({"arg1": 42}),
            json!([{"s": "42"}]),
        ),
        (
            "object via CAST AS jsonb (parser default text)",
            "-- $1 arg1\nSELECT CAST($1 AS jsonb) AS v".to_owned(),
            json!({"arg1": {"k": 1}}),
            json!([{"v": {"k": 1}}]),
        ),
        (
            "string '42' via CAST AS int (parser default text)",
            "-- $1 arg1\nSELECT CAST($1 AS int) AS v".to_owned(),
            json!({"arg1": "42"}),
            json!([{"v": 42}]),
        ),
        (
            "NULL into bool col via CAST",
            "-- $1 arg1\nINSERT INTO wm_pg_arg_combo_test.bugbool VALUES (CAST($1 AS bool)) RETURNING flag".to_owned(),
            json!({"arg1": null}),
            json!([{"flag": null}]),
        ),
        // === SDK happy path — inline ::TYPE injected by the client ===
        (
            "SDK shape: $1::BIGINT, $2::DOUBLE PRECISION",
            "-- $1 arg1\n-- $2 arg2\nINSERT INTO wm_pg_arg_combo_test.sdkbug VALUES ($1::BIGINT, $2::DOUBLE PRECISION) RETURNING n, f".to_owned(),
            json!({"arg1": 42, "arg2": 3.14}),
            json!([{"n": 42, "f": 3.14}]),
        ),
        (
            "SDK shape: $1::TEXT with int target via explicit ::int",
            "-- $1 arg1\nSELECT $1::TEXT::int AS v".to_owned(),
            json!({"arg1": "7"}),
            json!([{"v": 7}]),
        ),
        // === explicit declaration -- $N name (type) ===
        (
            "explicit decl (text)",
            "-- $1 arg1 (text)\nSELECT $1 AS v".to_owned(),
            json!({"arg1": "hello"}),
            json!([{"v": "hello"}]),
        ),
        (
            "explicit decl (jsonb) with object",
            "-- $1 arg1 (jsonb)\nSELECT $1 AS v".to_owned(),
            json!({"arg1": {"k": [1, 2]}}),
            json!([{"v": {"k": [1, 2]}}]),
        ),
        // === mixed args: int, text, jsonb, text[] ===
        (
            "mixed: int + text + jsonb + text[]",
            "-- $1 arg1\n-- $2 arg2\n-- $3 arg3\n-- $4 arg4\nINSERT INTO wm_pg_arg_combo_test.bugmix VALUES ($1::int, $2::text, $3::jsonb, $4::text[]) RETURNING *".to_owned(),
            json!({"arg1": 7, "arg2": "hello", "arg3": {"k": 1}, "arg4": ["a", "b"]}),
            json!([{"id": 7, "name": "hello", "payload": {"k": 1}, "tags": ["a", "b"]}]),
        ),

        // === Every PG type, SDK happy path (inline ::TYPE) ===
        (
            "all PG types via inline casts",
            r#"-- $1 arg1
-- $2 arg2
-- $3 arg3
-- $4 arg4
-- $5 arg5
-- $6 arg6
-- $7 arg7
-- $8 arg8
-- $9 arg9
-- $10 arg10
-- $11 arg11
-- $12 arg12
-- $13 arg13
-- $14 arg14
-- $15 arg15
-- $16 arg16
-- $17 arg17
-- $18 arg18
INSERT INTO wm_pg_arg_combo_test.allcols VALUES (
  $1::bool, $2::int2, $3::int4, $4::int8,
  $5::real, $6::double precision, $7::numeric,
  $8::text, $9::varchar,
  $10::uuid, $11::date, $12::time, $13::timestamp, $14::timestamptz,
  $15::json, $16::jsonb,
  $17::int[], $18::text[]
) RETURNING c_bool, c_int4, c_int8, c_text, c_uuid, c_int_arr"#.to_owned(),
            json!({
                "arg1": true, "arg2": 1, "arg3": 2, "arg4": 3,
                "arg5": 1.5, "arg6": 2.5, "arg7": 3.14,
                "arg8": "hello", "arg9": "varhello",
                "arg10": "550e8400-e29b-41d4-a716-446655440000",
                "arg11": "2024-01-15", "arg12": "10:30:00",
                "arg13": "2024-01-15T10:30:00", "arg14": "2024-01-15T10:30:00Z",
                "arg15": {"k": 1}, "arg16": {"k": 2},
                "arg17": [1,2,3], "arg18": ["a","b"]
            }),
            json!([{"c_bool": true, "c_int4": 2, "c_int8": 3, "c_text": "hello",
                    "c_uuid": "550e8400-e29b-41d4-a716-446655440000",
                    "c_int_arr": [1,2,3]}]),
        ),

        // === Every PG type, declaration-style otyp (Python SDK shape) ===
        (
            "all PG types via -- $N name (TYPE) decls",
            r#"-- $1 arg1 (bool)
-- $2 arg2 (int2)
-- $3 arg3 (int4)
-- $4 arg4 (int8)
-- $5 arg5 (real)
-- $6 arg6 (float8)
-- $7 arg7 (numeric)
-- $8 arg8 (text)
-- $9 arg9 (varchar)
-- $10 arg10 (uuid)
-- $11 arg11 (date)
-- $12 arg12 (time)
-- $13 arg13 (timestamp)
-- $14 arg14 (timestamptz)
-- $15 arg15 (json)
-- $16 arg16 (jsonb)
-- $17 arg17 (int[])
-- $18 arg18 (text[])
INSERT INTO wm_pg_arg_combo_test.allcols VALUES (
  $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18
) RETURNING c_bool, c_int4, c_int8, c_text, c_int_arr, c_text_arr"#.to_owned(),
            json!({
                "arg1": false, "arg2": 4, "arg3": 5, "arg4": 6,
                "arg5": 1.5, "arg6": 2.5, "arg7": 3.14,
                "arg8": "world", "arg9": "varworld",
                "arg10": "550e8400-e29b-41d4-a716-446655440000",
                "arg11": "2024-01-15", "arg12": "10:30:00",
                "arg13": "2024-01-15T10:30:00", "arg14": "2024-01-15T10:30:00Z",
                "arg15": [1,2], "arg16": [3,4],
                "arg17": [10,20], "arg18": ["x","y"]
            }),
            json!([{"c_bool": false, "c_int4": 5, "c_int8": 6, "c_text": "world",
                    "c_int_arr": [10,20], "c_text_arr": ["x","y"]}]),
        ),

        // === Edge values per type ===
        ("int8 negative", "-- $1 arg1\nSELECT $1::int8 AS v".to_owned(), json!({"arg1": -42}), json!([{"v": -42}])),
        ("int8 zero",     "-- $1 arg1\nSELECT $1::int8 AS v".to_owned(), json!({"arg1": 0}), json!([{"v": 0}])),
        ("int4 max",      "-- $1 arg1\nSELECT $1::int4 AS v".to_owned(), json!({"arg1": 2147483647i64}), json!([{"v": 2147483647i64}])),
        ("int8 max",      "-- $1 arg1\nSELECT $1::int8 AS v".to_owned(), json!({"arg1": 9223372036854775807i64}), json!([{"v": 9223372036854775807i64}])),
        ("float8 fraction", "-- $1 arg1\nSELECT $1::float8 AS v".to_owned(), json!({"arg1": 0.1 + 0.2}), json!([{"v": 0.1 + 0.2}])),
        ("empty string", "-- $1 arg1\nSELECT $1::text AS v".to_owned(), json!({"arg1": ""}), json!([{"v": ""}])),
        ("empty array",  "-- $1 arg1\nSELECT $1::int[] AS v".to_owned(), json!({"arg1": []}), json!([{"v": []}])),
        ("empty object", "-- $1 arg1\nSELECT $1::jsonb AS v".to_owned(), json!({"arg1": {}}), json!([{"v": {}}])),

        // === Bool roundtrip across every shape ===
        ("Bool/bare $1 → bool col", "-- $1 arg1\nSELECT $1 AS v".to_owned(), json!({"arg1": true}), json!([{"v": true}])),
        ("Bool/inline ::bool",      "-- $1 arg1\nSELECT $1::bool AS v".to_owned(), json!({"arg1": true}), json!([{"v": true}])),
        ("Bool/decl (bool)",        "-- $1 arg1 (bool)\nSELECT $1 AS v".to_owned(), json!({"arg1": true}), json!([{"v": true}])),
        ("Bool/CAST AS bool",       "-- $1 arg1\nSELECT CAST($1 AS bool) AS v".to_owned(), json!({"arg1": false}), json!([{"v": false}])),
        ("Bool/CAST AS bool inside SELECT","-- $1 arg1\nSELECT CAST($1 AS bool) AS v WHERE CAST($1 AS bool) IS NOT NULL".to_owned(), json!({"arg1": true}), json!([{"v": true}])),

        // === Object/Array via CAST AS jsonb (regression: non-text otyp via CAST) ===
        ("Object via CAST AS jsonb", "-- $1 arg1\nSELECT CAST($1 AS jsonb) AS v".to_owned(), json!({"arg1": {"a":1,"b":[2,3]}}), json!([{"v": {"a":1,"b":[2,3]}}])),
        ("Array via CAST AS jsonb",  "-- $1 arg1\nSELECT CAST($1 AS jsonb) AS v".to_owned(), json!({"arg1": [1,"two",{"three":3}]}), json!([{"v": [1,"two",{"three":3}]}])),
        ("Object inline ::json",     "-- $1 arg1\nSELECT $1::json AS v".to_owned(), json!({"arg1": {"k":1}}), json!([{"v": {"k":1}}])),
        ("Object decl (jsonb)",      "-- $1 arg1 (jsonb)\nSELECT $1 AS v".to_owned(), json!({"arg1": {"k":1}}), json!([{"v": {"k":1}}])),

        // === Numbers: implicit/explicit casts to text ===
        ("Number/CAST AS text",    "-- $1 arg1\nSELECT CAST($1 AS text) AS v".to_owned(), json!({"arg1": 42}), json!([{"v": "42"}])),
        ("Number/inline ::text",   "-- $1 arg1\nSELECT $1::text AS v".to_owned(), json!({"arg1": 42}), json!([{"v": "42"}])),
        ("Float/CAST AS text",     "-- $1 arg1\nSELECT CAST($1 AS text) AS v".to_owned(), json!({"arg1": 3.14}), json!([{"v": "3.14"}])),
        ("Negative/CAST AS int8",  "-- $1 arg1\nSELECT CAST($1 AS int8) AS v".to_owned(), json!({"arg1": -1}), json!([{"v": -1}])),

        // === Strings parsed into typed targets ===
        ("String '42' as int",     "-- $1 arg1\nSELECT $1::int AS v".to_owned(), json!({"arg1": "42"}), json!([{"v": 42}])),
        ("String '42' as bigint",  "-- $1 arg1\nSELECT $1::bigint AS v".to_owned(), json!({"arg1": "42"}), json!([{"v": 42}])),
        ("String '1.5' as real",   "-- $1 arg1\nSELECT $1::real AS v".to_owned(), json!({"arg1": "1.5"}), json!([{"v": 1.5}])),
        ("String uuid",            "-- $1 arg1\nSELECT $1::uuid AS v".to_owned(), json!({"arg1": "550e8400-e29b-41d4-a716-446655440000"}), json!([{"v": "550e8400-e29b-41d4-a716-446655440000"}])),

        // === NULL handling for every type ===
        ("Null/bool",      "-- $1 arg1\nSELECT CAST($1 AS bool) AS v".to_owned(),      json!({"arg1": null}), json!([{"v": null}])),
        ("Null/int",       "-- $1 arg1\nSELECT CAST($1 AS int) AS v".to_owned(),       json!({"arg1": null}), json!([{"v": null}])),
        ("Null/bigint",    "-- $1 arg1\nSELECT CAST($1 AS bigint) AS v".to_owned(),    json!({"arg1": null}), json!([{"v": null}])),
        ("Null/text",      "-- $1 arg1\nSELECT $1::text AS v".to_owned(),              json!({"arg1": null}), json!([{"v": null}])),
        ("Null/jsonb",     "-- $1 arg1\nSELECT CAST($1 AS jsonb) AS v".to_owned(),     json!({"arg1": null}), json!([{"v": null}])),
        ("Null/uuid",      "-- $1 arg1\nSELECT CAST($1 AS uuid) AS v".to_owned(),      json!({"arg1": null}), json!([{"v": null}])),
        ("Null/timestamp", "-- $1 arg1\nSELECT CAST($1 AS timestamp) AS v".to_owned(), json!({"arg1": null}), json!([{"v": null}])),
        ("Null/date",      "-- $1 arg1\nSELECT CAST($1 AS date) AS v".to_owned(),      json!({"arg1": null}), json!([{"v": null}])),

        // === Multi-statement (datatable scripts often DROP/CREATE/INSERT) ===
        (
            "multi-statement: DROP/CREATE/INSERT",
            r#"DROP TABLE IF EXISTS wm_pg_arg_combo_test.tmp_multi;
CREATE TABLE wm_pg_arg_combo_test.tmp_multi (n int, b bool);
-- $1 arg1
-- $2 arg2
INSERT INTO wm_pg_arg_combo_test.tmp_multi VALUES ($1::int, $2::bool) RETURNING *"#.to_owned(),
            json!({"arg1": 10, "arg2": true}),
            json!([{"n": 10, "b": true}]),
        ),

        // === Same arg used in multiple positions (parser reorders) ===
        (
            "same arg twice",
            "-- $1 arg1\nSELECT $1::int + $1::int AS v".to_owned(),
            json!({"arg1": 5}),
            json!([{"v": 10}]),
        ),

        // === Custom enum (unrecognised arg_t → prepare fallback path) ===
        // The unrecognised-arg_t fallback works when the user formats the
        // value as the enum's text representation themselves, so the binding
        // never has to encode a Rust String *as* an enum (which the
        // tokio-postgres `ToSql` impls don't support):
        (
            "custom enum: text representation cast in SQL",
            r#"-- $1 arg1
INSERT INTO wm_pg_arg_combo_test.enumtbl
VALUES (CAST($1::text AS wm_pg_arg_combo_test.color))
RETURNING c::text AS c"#
                .to_owned(),
            json!({"arg1": "green"}),
            json!([{"c": "green"}]),
        ),

        // === Sparse positional placeholders ($5, $50) ===
        // The pre-fix `String::replace($5 → $1)` chain mangled `$50` into
        // `$10`, breaking sparse-index queries. The regex-based renumbering
        // handles them as distinct units.
        (
            "sparse $5 / $50 renumbering",
            "-- $5 arg5\n-- $50 arg50\nSELECT $5::int AS a, $50::int AS b".to_owned(),
            json!({"arg5": 5, "arg50": 50}),
            json!([{"a": 5, "b": 50}]),
        ),
        (
            "sparse $5 / $50 reversed in SQL",
            "-- $5 arg5\n-- $50 arg50\nSELECT $50::int AS a, $5::int AS b".to_owned(),
            json!({"arg5": 5, "arg50": 50}),
            json!([{"a": 50, "b": 5}]),
        ),
        (
            "sparse same arg used twice + sparse",
            "-- $5 arg5\n-- $50 arg50\nSELECT $5::int + $5::int AS a, $50::int AS b".to_owned(),
            json!({"arg5": 7, "arg50": 50}),
            json!([{"a": 14, "b": 50}]),
        ),

        // === Explicit (text) decl + non-string value: should coerce ===
        // Without the otyp_inferred flag, the executor would bind the value's
        // natural type (INT8/BOOL) and the WHERE comparison `text = int8` /
        // `text = bool` would fail with "operator does not exist". With the
        // flag, the parser tells the executor "user committed to text" and
        // the value is JSON-stringified so `text = text` works.
        (
            "decl (text) + Number used in WHERE text comparison",
            r#"-- $1 arg1 (text)
SELECT name FROM (VALUES ('42'::text)) AS t(name) WHERE name = $1"#
                .to_owned(),
            json!({"arg1": 42}),
            json!([{"name": "42"}]),
        ),
        (
            "decl (text) + Bool used in WHERE text comparison",
            r#"-- $1 arg1 (text)
SELECT name FROM (VALUES ('true'::text)) AS t(name) WHERE name = $1"#
                .to_owned(),
            json!({"arg1": true}),
            json!([{"name": "true"}]),
        ),
        (
            "decl (varchar) + Number used in WHERE",
            r#"-- $1 arg1 (varchar)
SELECT name FROM (VALUES ('99'::varchar)) AS t(name) WHERE name = $1"#
                .to_owned(),
            json!({"arg1": 99}),
            json!([{"name": "99"}]),
        ),

        // === Bare $N (parser-default text) + non-string value: bind native ===
        // The user wrote no annotation — we bind the value's natural type so
        // it works against whatever column the SQL eventually targets.
        (
            "bare $1 + Bool into bool col",
            "-- $1 arg1\nSELECT $1 = true AS v".to_owned(),
            json!({"arg1": true}),
            json!([{"v": true}]),
        ),

        // === Custom enum (Kind::Enum) — round-trip via AnyTextValue ===
        // Pre-fix this failed at the encoder ("cannot convert String → color")
        // because vanilla tokio_postgres' ToSql/FromSql for String reject
        // Kind::Enum. The wrapper accepts enum kinds in both directions.
        (
            "enum: explicit ::wm_pg_arg_combo_test.color cast",
            r#"-- $1 arg1
INSERT INTO wm_pg_arg_combo_test.enumtbl
VALUES ($1::wm_pg_arg_combo_test.color) RETURNING c"#
                .to_owned(),
            json!({"arg1": "blue"}),
            json!([{"c": "blue"}]),
        ),
        (
            "enum: SELECT a literal value cast to enum",
            "-- $1 arg1\nSELECT $1::wm_pg_arg_combo_test.color AS c".to_owned(),
            json!({"arg1": "red"}),
            json!([{"c": "red"}]),
        ),

        // === Extended String→numeric/real/double/oid/bool arms (#10) ===
        (
            "String '3.14' → numeric",
            "-- $1 arg1\nSELECT $1::numeric AS v".to_owned(),
            json!({"arg1": "3.14"}),
            json!([{"v": 3.14}]),
        ),
        (
            "String '1.5' → real",
            "-- $1 arg1\nSELECT $1::real AS v".to_owned(),
            json!({"arg1": "1.5"}),
            json!([{"v": 1.5}]),
        ),
        (
            "String '2.5' → double",
            "-- $1 arg1\nSELECT $1::double precision AS v".to_owned(),
            json!({"arg1": "2.5"}),
            json!([{"v": 2.5}]),
        ),
        (
            "String 'true' → bool",
            "-- $1 arg1\nSELECT $1::bool AS v".to_owned(),
            json!({"arg1": "true"}),
            json!([{"v": true}]),
        ),
        (
            "String 't' → bool",
            "-- $1 arg1\nSELECT $1::bool AS v".to_owned(),
            json!({"arg1": "t"}),
            json!([{"v": true}]),
        ),
        (
            "String '0' → bool false",
            "-- $1 arg1\nSELECT $1::bool AS v".to_owned(),
            json!({"arg1": "0"}),
            json!([{"v": false}]),
        ),
        (
            "String '42' → oid",
            "-- $1 arg1\nSELECT $1::oid AS v".to_owned(),
            json!({"arg1": "42"}),
            json!([{"v": 42}]),
        ),

        // === String literals containing $N must NOT be renumbered ===
        // Sparse positional args force a rewrite pass; the literal
        // `'price: $5'` and the comment `-- mention $5` must survive intact.
        (
            "renumber must skip $N inside string literal",
            r#"-- $5 arg5
-- $50 arg50
SELECT 'price: $5' AS lbl, $5::int + $50::int AS sum"#
                .to_owned(),
            json!({"arg5": 1, "arg50": 2}),
            json!([{"lbl": "price: $5", "sum": 3}]),
        ),

        // === Multi-word PG type names with [] array suffix ===
        // Pre-fix: `transform_types_with_spaces` returned a `&str` alias
        // ("double" / "varchar" / "timestamptz" / …) and dropped the trailing
        // `[]`, so the dispatch routed `Value::Array` through `Type::JSONB`
        // and the server failed with "cannot cast type jsonb to <T>[]".
        (
            "multi-word array: double precision[]",
            "-- $1 a\nSELECT $1::double precision[] AS v".to_owned(),
            json!({"a": [1.5, 2.5]}),
            json!([{"v": [1.5, 2.5]}]),
        ),
        (
            "multi-word array: character varying[]",
            "-- $1 a\nSELECT $1::character varying[] AS v".to_owned(),
            json!({"a": ["x", "y"]}),
            json!([{"v": ["x", "y"]}]),
        ),
        (
            "multi-word array: timestamp without time zone[]",
            "-- $1 a\nSELECT $1::timestamp without time zone[] AS v".to_owned(),
            json!({"a": ["2024-01-15T10:30:00"]}),
            json!([{"v": ["2024-01-15T10:30:00"]}]),
        ),

        // === Stringified primitives in array args ===
        // Mirror the scalar `Value::String → <numeric type>` arms so values
        // sent as e.g. `["1.5", "2.5"]` for `numeric[]` (typical for bulk-
        // loading via `unnest` or BigInt-stringified arrays) round-trip
        // instead of erroring with "Mixed types in array".
        (
            "numeric[] from stringified decimals",
            "-- $1 a\nSELECT $1::numeric[] AS v".to_owned(),
            json!({"a": ["1.5", "2.5"]}),
            json!([{"v": [1.5, 2.5]}]),
        ),
        (
            "int[] from stringified ints",
            "-- $1 a\nSELECT $1::int[] AS v".to_owned(),
            json!({"a": ["1", "2", "3"]}),
            json!([{"v": [1, 2, 3]}]),
        ),
        (
            "bool[] from stringified bools",
            "-- $1 a\nSELECT $1::bool[] AS v".to_owned(),
            json!({"a": ["true", "f", "yes"]}),
            json!([{"v": [true, false, true]}]),
        ),
    ];

    for (name, content, args, expected) in cases {
        let mut job = RunJob::from(JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            lock: None,
            language: ScriptLang::Postgresql,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            modules: None,
        }))
        .arg("database", db_arg.clone());
        for (k, v) in args.as_object().unwrap() {
            job = job.arg(k, v.clone());
        }
        let result = job
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap_or_else(|| panic!("case '{name}': no json result"));
        assert_eq!(result, expected, "case '{name}' mismatch");
    }

    Ok(())
}

/// Pooler-safety regression test for #8988: when every arg has a resolvable
/// otyp (the common SDK case), the dispatch must use unnamed prepared
/// statements (`query_typed_raw`) and *must not* leak named statements
/// (`s0, s1, ...`) on the cached connection. Behind a transaction-mode pooler
/// (PgBouncer / Supabase pooler / RDS Proxy), accumulated names get dropped
/// when the prepare and execute land on different backend connections, which
/// is what produced the original "prepared statement \"sN\" does not exist"
/// errors.
#[sqlx::test(fixtures("base"))]
#[serial(pg_cache)]
async fn test_postgresql_no_named_statements_after_typed_args(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use windmill_worker::pg_executor::clear_pg_cache;

    initialize_tracing().await;
    clear_pg_cache().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let db_arg = json!({"host": "localhost", "port": 5432, "dbname": "windmill", "user": "postgres", "password": "changeme"});

    let make_pg_job = |content: String| {
        RunJob::from(JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            lock: None,
            language: ScriptLang::Postgresql,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            modules: None,
        }))
        .arg("database", db_arg.clone())
    };

    // Run several SDK-shape queries (parameterised, all args resolvable) on the
    // same cached connection. None of them should land in `pg_prepared_statements`.
    for i in 0..5 {
        let result = make_pg_job(format!(
            "-- $1 arg1\n-- $2 arg2\nSELECT $1::BIGINT AS a, $2::TEXT AS b, {} AS i;",
            i
        ))
        .arg("arg1", json!(i))
        .arg("arg2", json!(format!("v{i}")))
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
        assert_eq!(
            result,
            json!([{"a": i, "b": format!("v{i}"), "i": i}]),
            "iteration {i}"
        );
    }

    // Use the *same* connection (cached one) to peek at pg_prepared_statements.
    // Anything matching the tokio-postgres "s\d+" naming would mean the
    // pooler-unsafe `prepare + query_raw` path was taken.
    let probe = make_pg_job(
        "SELECT count(*)::int AS n FROM pg_prepared_statements WHERE name ~ '^s[0-9]+$'".to_owned(),
    )
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();
    let n = probe[0]["n"].as_i64().unwrap_or(-1);
    assert_eq!(
        n, 0,
        "expected no leaked named prepared statements, found {n}"
    );

    Ok(())
}

/// Exercise the `prepare + query_raw` fallback path. The dispatch takes this
/// path when `otyp_to_pg_type` doesn't recognise the parser-derived arg_t —
/// typical for custom enums, domains, and extension types.
///
/// Vanilla `tokio_postgres`'s `ToSql for String` doesn't actually accept
/// `Kind::Enum` / `Kind::Domain`, so an end-to-end happy-path test of an
/// arbitrary custom type isn't possible without `postgres-derive`. What we
/// CAN lock in here is *which dispatch path runs*: when the arg_t is
/// unrecognised, the prepare path must be taken (the server resolves the
/// param type from the cast and the binding then errors at the encoder).
/// If a regression accidentally routes unrecognised types through
/// `query_typed_raw`, the failure mode flips: instead of "cannot convert
/// `String` to `<custom_type>`" we'd see "cannot convert `String` to
/// `text`" (because we'd assert TEXT). The error-text check below catches
/// that flip.
#[sqlx::test(fixtures("base"))]
#[serial(pg_cache)]
async fn test_postgresql_prepare_fallback_for_unrecognised_arg_t(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use windmill_worker::pg_executor::clear_pg_cache;

    initialize_tracing().await;
    clear_pg_cache().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let db_arg = json!({"host": "localhost", "port": 5432, "dbname": "windmill", "user": "postgres", "password": "changeme"});

    let make_pg_job = |content: String| {
        RunJob::from(JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            lock: None,
            language: ScriptLang::Postgresql,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            modules: None,
        }))
        .arg("database", db_arg.clone())
    };

    // Set up a custom enum in a dedicated schema so the test is self-contained.
    let setup = r#"
DROP SCHEMA IF EXISTS fallback_test CASCADE;
CREATE SCHEMA fallback_test;
CREATE TYPE fallback_test.color AS ENUM ('red', 'green', 'blue');
"#;
    make_pg_job(setup.to_owned())
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

    // Inline cast `::fallback_test.color` — the parser only captures
    // `fallback_test` (regex stops at the dot), which otyp_to_pg_type won't
    // recognise. Dispatch must take the prepare fallback path.
    //
    // Thanks to the AnyTextValue ToSql/FromSql wrapper, this case now
    // round-trips end-to-end (the wrapper accepts Kind::Enum on both
    // directions). Pre-fix, vanilla tokio_postgres rejected String → color
    // and the user had to write `CAST($1::text AS color)` as a workaround.
    let result = make_pg_job("-- $1 arg1\nSELECT $1::fallback_test.color AS c".to_owned())
        .arg("arg1", json!("red"))
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
    assert_eq!(result, json!([{"c": "red"}]));

    // Reading enum columns also goes through AnyTextValue's FromSql impl on
    // the result side, so the value comes back as a JSON string.
    let result = make_pg_job(
        r#"-- $1 arg1
SELECT $1::fallback_test.color AS c1,
       'green'::fallback_test.color AS c2"#
            .to_owned(),
    )
    .arg("arg1", json!("blue"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();
    assert_eq!(result, json!([{"c1": "blue", "c2": "green"}]));

    Ok(())
}

/// Regression: custom enum / domain queries must not error with `prepared
/// statement "sN" does not exist` when run on a cached connection.
///
/// Root cause: when we used `DISCARD ALL` to reset the cached connection
/// between jobs, the included `DEALLOCATE ALL` deallocated *every* prepared
/// statement server-side — including the typeinfo statements that
/// tokio_postgres caches per-client to resolve custom-type Oids. The Rust
/// client still held `Statement` objects whose names the server had
/// forgotten, so the next custom-type query failed.
///
/// Fix: switched the cached-connection probe to `RESET ALL; UNLISTEN *;
/// CLOSE ALL;` which covers windmill's session-isolation needs (GUC reset,
/// listen channels, open cursors) without nuking the prepared-statement
/// cache. This test runs the failing pattern (enum query → domain query on
/// the same cached connection, several times) to lock the behaviour in.
#[sqlx::test(fixtures("base"))]
#[serial(pg_cache)]
async fn test_postgresql_custom_types_on_cached_connection(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use windmill_worker::pg_executor::clear_pg_cache;

    initialize_tracing().await;
    clear_pg_cache().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let db_arg = json!({"host": "localhost", "port": 5432, "dbname": "windmill", "user": "postgres", "password": "changeme"});

    let make_pg_job = |content: String| {
        RunJob::from(JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            lock: None,
            language: ScriptLang::Postgresql,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            modules: None,
        }))
        .arg("database", db_arg.clone())
    };

    // Set up custom enum + domain types in a dedicated schema (avoid the
    // `pg_*` reserved prefix for user schemas).
    let setup = r#"
DROP SCHEMA IF EXISTS wm_pg_cached_test CASCADE;
CREATE SCHEMA wm_pg_cached_test;
CREATE TYPE wm_pg_cached_test.color AS ENUM ('red', 'green', 'blue');
CREATE DOMAIN wm_pg_cached_test.short_name AS TEXT CHECK (length(VALUE) BETWEEN 1 AND 10);
"#;
    make_pg_job(setup.to_owned())
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

    // Run a long alternating sequence of enum + domain queries on the same
    // cached connection. Each query goes through the prepare-fallback path
    // (otyp_to_pg_type doesn't recognise these custom-type names) and needs
    // tokio_postgres' typeinfo cache to resolve the Oids server-side. Pre-fix
    // this would fail intermittently with `prepared statement "sN" does not
    // exist` after the first cached-conn reuse.
    for i in 0..10 {
        let r = make_pg_job("-- $1 arg1\nSELECT $1::wm_pg_cached_test.color AS c".to_owned())
            .arg("arg1", json!(if i % 2 == 0 { "red" } else { "blue" }))
            .run_until_complete(&db, false, port)
            .await;
        assert!(
            r.success,
            "iter {i} enum query failed (was the DISCARD ALL bug); result: {:?}",
            r.result
        );

        let r = make_pg_job("-- $1 arg1\nSELECT $1::wm_pg_cached_test.short_name AS s".to_owned())
            .arg("arg1", json!(format!("v{i}")))
            .run_until_complete(&db, false, port)
            .await;
        assert!(
            r.success,
            "iter {i} domain query failed (was the DISCARD ALL bug); result: {:?}",
            r.result
        );
    }

    Ok(())
}

/// Security regression: a previous job that did `SET ROLE` or `SET SESSION
/// AUTHORIZATION` to a different role must not leak that role into the next
/// job that reuses the cached connection.
///
/// This is the case `RESET ALL` alone does *not* cover — neither SET ROLE
/// nor SET SESSION AUTHORIZATION are GUC parameters, so they survive
/// `RESET ALL`. We rely on `RESET SESSION AUTHORIZATION` (which subsumes
/// `RESET ROLE`) explicitly being part of the cached-connection probe.
///
/// The pre-existing `test_postgresql_single_worker_session_isolation` test
/// only did `SET ROLE postgres` (the connecting user), so the leak was
/// invisible — this test catches it by switching to a *different* role.
#[sqlx::test(fixtures("base"))]
#[serial(pg_cache)]
async fn test_postgresql_set_role_does_not_leak_across_cached_connection(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use windmill_worker::pg_executor::clear_pg_cache;

    initialize_tracing().await;
    clear_pg_cache().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let db_arg = json!({"host": "localhost", "port": 5432, "dbname": "windmill", "user": "postgres", "password": "changeme"});

    let make_pg_job = |content: String| {
        RunJob::from(JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            lock: None,
            language: ScriptLang::Postgresql,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            modules: None,
        }))
        .arg("database", db_arg.clone())
    };

    // Create a non-postgres role to switch to. Idempotent so the test survives
    // re-runs against the same DB.
    make_pg_job(
        "DO $$ BEGIN \
           IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'wm_isolation_test_role') THEN \
             CREATE ROLE wm_isolation_test_role; \
           END IF; \
         END $$"
            .to_owned(),
    )
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    // Job 1: SET ROLE to a different role.
    let r1 = make_pg_job(
        "SET ROLE wm_isolation_test_role; SELECT current_user AS cu, session_user AS su".to_owned(),
    )
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();
    assert_eq!(r1[0]["cu"], "wm_isolation_test_role");
    assert_eq!(r1[0]["su"], "postgres");

    // Job 2 (cached connection reuse): role MUST be back to the connecting
    // user. Pre-fix with `RESET ALL` alone, this would still see
    // `wm_isolation_test_role` because RESET ALL doesn't cover SET ROLE.
    let r2 = make_pg_job("SELECT current_user AS cu, session_user AS su".to_owned())
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
    assert_eq!(
        r2[0]["cu"], "postgres",
        "SET ROLE leaked across cached connection: current_user is {}",
        r2[0]["cu"]
    );

    // Job 3: SET SESSION AUTHORIZATION (changes both current_user and
    // session_user — RESET ALL does NOT touch this either).
    let r3 = make_pg_job(
        "SET SESSION AUTHORIZATION wm_isolation_test_role; \
         SELECT current_user AS cu, session_user AS su"
            .to_owned(),
    )
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();
    assert_eq!(r3[0]["cu"], "wm_isolation_test_role");
    assert_eq!(r3[0]["su"], "wm_isolation_test_role");

    // Job 4 (cached): both must be restored to the connecting user.
    let r4 = make_pg_job("SELECT current_user AS cu, session_user AS su".to_owned())
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
    assert_eq!(
        r4[0]["cu"], "postgres",
        "SET SESSION AUTHORIZATION leaked across cached connection: current_user is {}",
        r4[0]["cu"]
    );
    assert_eq!(
        r4[0]["su"], "postgres",
        "SET SESSION AUTHORIZATION leaked across cached connection: session_user is {}",
        r4[0]["su"]
    );

    Ok(())
}

#[cfg(feature = "mysql")]
#[sqlx::test(fixtures("base"))]
async fn test_mysql_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
-- ? name (varchar)
SELECT ? AS result;
"#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Mysql,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("name", json!("world"))
    .arg(
        "database",
        json!({"host": "localhost", "port": 3306, "user": "root", "password": "changeme", "database": "windmill_test"}),
    )
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, json!([{"result": "world"}]));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_bunnative_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export async function main(name: string): Promise<string> {
    return `hello ${name}`;
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Bunnative,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("name", json!("world"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("hello world"));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_powershell_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
param($msg)
Write-Output "hello $msg"
"#
    .to_owned();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Powershell,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("msg", json!("world"))
    .run_until_complete(&db, false, port)
    .await;
    assert_eq!(job.json_result(), Some(json!("hello world")));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_powershell_param_block_with_attributes(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
param(
    [Parameter(Mandatory=$true)]
    [string]$Name,
    [int]$Count = 3
)
Write-Output "$Name-$Count"
"#
    .to_owned();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Powershell,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("Name", json!("test"))
    .arg("Count", json!(7))
    .run_until_complete(&db, false, port)
    .await;
    assert_eq!(job.json_result(), Some(json!("test-7")));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_powershell_error_caught(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Script with param block that throws an error — verifies the catch block works
    let content = r#"
param($x)
throw "intentional error"
"#
    .to_owned();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Powershell,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("x", json!(1))
    .run_until_complete(&db, false, port)
    .await;
    assert!(!job.success, "job should fail on thrown error");
    let result_str = serde_json::to_string(&job.result).unwrap_or_default();
    assert!(
        result_str.contains("An error occurred:"),
        "catch block should output 'An error occurred:', got: {result_str}"
    );
    assert!(
        result_str.contains("intentional error"),
        "catch block should output the error message, got: {result_str}"
    );
    // Verify the catch block doesn't leak "Write-Output" as literal text
    // (regression from the old broken line continuation in strict_termination_end)
    let after_marker = result_str.split("An error occurred:").nth(1).unwrap_or("");
    assert!(
        !after_marker.starts_with("\\nWrite-Output"),
        "catch block should not output literal 'Write-Output' text, got: {result_str}"
    );
    Ok(())
}

#[cfg(feature = "php")]
#[sqlx::test(fixtures("base"))]
async fn test_php_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
<?php

function main(string $name): string {
    return "hello " . $name;
}
"#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Php,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("name", json!("world"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("hello world"));
    Ok(())
}

#[cfg(feature = "ruby")]
#[sqlx::test(fixtures("base"))]
async fn test_ruby_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
def main(name)
  "hello #{name}"
end
"#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Ruby,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("name", json!("world"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("hello world"));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_datetime(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export async function main(a: Date) {
    return typeof a;
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Bun,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("a", json!("2024-09-24T10:00:00.000Z"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("object"));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_deno_job_datetime(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export async function main(a: Date) {
    return typeof a;
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Deno,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("a", json!("2024-09-24T10:00:00.000Z"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("object"));
    Ok(())
}

/// Test that full .npmrc content works for deno jobs with private registries.
/// Requires:
/// - `TEST_NPMRC` environment variable set to the full .npmrc content
#[cfg(feature = "private_registry_test")]
#[sqlx::test(fixtures("base"))]
async fn test_deno_job_private_npmrc(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_worker::NPMRC;

    let npmrc_content = std::env::var("TEST_NPMRC")
        .expect("TEST_NPMRC must be set when running private_registry_test");

    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    {
        let mut npmrc = NPMRC.write().await;
        *npmrc = Some(npmrc_content.clone());
    }

    let content = r#"
import { greet } from "npm:@windmill-test/private-pkg";

export function main(name: string) {
    return greet(name);
}
"#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Deno,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        modules: None,
    }))
    .arg("name", json!("World"))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    {
        let mut npmrc = NPMRC.write().await;
        *npmrc = None;
    }

    assert_eq!(
        result,
        serde_json::json!("Hello from private package, World!")
    );
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_job_datetime_and_bytes(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
from datetime import datetime
def main(a: datetime, b: bytes):
    return (isinstance(a, datetime), isinstance(b, bytes))
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Python3,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .arg("a", json!("2024-09-24T10:00:00.000Z"))
    .arg("b", json!("dGVzdA=="))
    .run_until_complete(&db, false, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!([true, true]));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_empty_loop_1(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "static", "value": [] },
                    "modules": [
                        {
                            "value": {
                                "input_transforms": {
                                    "n": {
                                        "type": "javascript",
                                        "expr": "flow_input.iter.value",
                                    },
                                },
                                "type": "rawscript",
                                "language": "python3",
                                "content": "def main(n): return n",
                            },
                        }
                    ],
                },
            },
            {
                "value": {
                    "input_transforms": {
                        "items": {
                            "type": "javascript",
                            "expr": "results.a",
                        },
                    },
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(items): return sum(items)",
                },
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!(0));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_invalid_first_step(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "flow_input" },
                    "modules": [
                        {
                            "value": {
                                "type": "identity",
                            },
                        }
                    ],
                },
            },
            {
                "value": {
                    "type": "identity",
                },
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let job = run_job_in_new_worker_until_complete(&db, false, flow, port).await;

    assert!(
        serde_json::to_string(&job.json_result().unwrap()).unwrap().contains("Expected an array value in the iterator expression, found: invalid type: map, expected a sequence at line 1 column 0")
    );
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_empty_loop_2(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "static", "value": [] },
                    "modules": [
                        {
                            "value": {
                                "input_transforms": {
                                    "n": {
                                        "type": "javascript",
                                        "expr": "flow_input.iter.value",
                                    },
                                },
                                "type": "rawscript",
                                "language": "python3",
                                "content": "def main(n): return n",
                            },
                        }
                    ],
                },
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([]));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_step_after_loop(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "static", "value": [2,3,4] },
                    "modules": [
                        {
                            "value": {
                                "input_transforms": {
                                    "n": {
                                        "type": "javascript",
                                        "expr": "flow_input.iter.value",
                                    },
                                },
                                "type": "rawscript",
                                "language": "python3",
                                "content": "def main(n): return n",
                            } ,
                        }
                    ],
                },
            },
            {
                "value": {
                    "input_transforms": {
                        "items": {
                            "type": "javascript",
                            "expr": "results.a",
                        },
                    },
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(items): return sum(items)",
                },
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!(9));
    Ok(())
}

fn module_add_item_to_list(i: i32, id: &str) -> serde_json::Value {
    json!({
        "id": format!("id_{}", i.to_string().replace("-", "_")),
        "value": {
            "input_transforms": {
                "array": {
                    "type": "javascript",
                    "expr": format!("results.{id}"),
                },
                "i": {
                    "type": "static",
                    "value": json!(i),
                }
            },
            "type": "rawscript",
            "language": "deno",
            "content": "export function main(array, i){ array.push(i); return array }",
        }
    })
}

fn module_failure() -> serde_json::Value {
    json!({
        "value": {
            "input_transforms": {},
            "type": "rawscript",
            "language": "deno",
            "content": "export function main(){ throw Error('failure') }",
        }
    })
}

#[sqlx::test(fixtures("base"))]
async fn test_branchone_simple(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [],
                    "default": [module_add_item_to_list(2, "a")],
                    "type": "branchone",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([1, 2]));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_branchone_with_cond(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [{"expr": "results.a[0] == 1", "modules": [module_add_item_to_list(3, "a")]}],
                    "default": [module_add_item_to_list(2, "a")],
                    "type": "branchone",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([1, 3]));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_branchall_sequential(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [
                        {"modules": [module_add_item_to_list(2, "a")]},
                        {"modules": [module_add_item_to_list(3, "a")]}],
                    "type": "branchall",
                    "parallel": true,
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([[1, 2], [1, 3]]));
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_branchall_simple(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [
                        {"modules": [module_add_item_to_list(2, "a")]},
                        {"modules": [module_add_item_to_list(3, "a")]}],
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([[1, 2], [1, 3]]));
    Ok(())
}

#[derive(Deserialize)]
struct ErrorResult {
    error: NamedError,
}

#[derive(Deserialize)]
struct NamedError {
    name: String,
}

#[sqlx::test(fixtures("base"))]
async fn test_branchall_skip_failure(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [
                        {"modules": [module_failure()], "skip_failure": false},
                        {"modules": [module_add_item_to_list(3, "a")]}],
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(
        serde_json::from_value::<ErrorResult>(result.get(0).unwrap().clone())
            .unwrap()
            .error
            .name,
        "Error"
    );

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [
                        {"modules": [module_failure()], "skip_failure": true},
                        {"modules": [module_add_item_to_list(2, "a")]}
                ],
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(
        serde_json::from_value::<ErrorResult>(result.get(0).unwrap().clone())
            .unwrap()
            .error
            .name,
        "Error"
    );
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_branchone_nested(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [] }",
                }
            },
            module_add_item_to_list(1, "a"),
            {
                "id": "b",
                "value": {
                    "branches": [
                        {
                            "expr": "false",
                            "modules": []
                        },
                        {
                            "expr": "true",
                            "modules": [                {
                                "value": {
                                    "branches": [
                                        {
                                            "expr": "false",
                                            "modules": []
                                        }],
                                    "default": [module_add_item_to_list(2, "id_1")],
                                    "type": "branchone",
                                }
                            }]
                        },
                    ],
                    "default": [module_add_item_to_list(-4, "id_1")],
                    "type": "branchone",
                }
            },
            module_add_item_to_list(3, "b"),
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([1, 2, 3]));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_branchall_nested(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [
                        {
                            "modules": [
                                    {
                                "id": "b",
                                "value": {
                                    "branches": [
                                        {"modules": [module_add_item_to_list(2, "a")]},
                                        {"modules": [module_add_item_to_list(3, "a")]}],
                                    "type": "branchall",
                                }
                            }, {
                                "value": {
                                    "branches": [
                                        {"modules": [module_add_item_to_list(4, "b")]},
                                        {"modules": [module_add_item_to_list(5, "b")]}],
                                    "type": "branchall",
                                }
                            }
                                    ]
                        },
                        {"modules": [module_add_item_to_list(6, "a")]}],
                        // "parallel": false,
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, false, flow, port)
        .await
        .json_result()
        .unwrap();

    println!("{:#?}", result);
    assert_eq!(
        result,
        serde_json::json!([[[[1, 2], [1, 3], 4], [[1, 2], [1, 3], 5]], [1, 6]])
    );
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_failure_module(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [{
                "id": "a",
                "value": {
                    "input_transforms": {
                        "l": { "type": "javascript", "expr": "[]", },
                        "n": { "type": "javascript", "expr": "flow_input.n", },
                    },
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 0) throw Error(JSON.stringify(l)); return { l: [...l, 0] } }",
                },
            }, {
                "id": "b",
                "value": {
                    "input_transforms": {
                        "l": { "type": "javascript", "expr": "results.a.l", },
                        "n": { "type": "javascript", "expr": "flow_input.n", },
                    },
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 1) throw Error(JSON.stringify(l)); return { l: [...l, 1] } }",
                },
            }, {
                "value": {
                    "input_transforms": {
                        "l": { "type": "javascript", "expr": "results.b.l", },
                        "n": { "type": "javascript", "expr": "flow_input.n", },
                    },
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 2) throw Error(JSON.stringify(l)); return { l: [...l, 2] } }",
                },
            }],
            "failure_module": {
                "value": {
                    "input_transforms": { "error": { "type": "javascript", "expr": "previous_result", } },
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(error) { return { 'from failure module': error } }",
                }
            },
        }))
        .unwrap();

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("n", json!(0))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

    assert!(result["from failure module"]["error"]
        .as_object()
        .unwrap()
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("[]"));

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("n", json!(1))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

    assert!(result["from failure module"]["error"]
        .as_object()
        .unwrap()
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("[0]"));

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("n", json!(2))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

    assert!(result["from failure module"]["error"]
        .as_object()
        .unwrap()
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("[0,1]"));

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("n", json!(3))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();
    assert_eq!(json!({ "l": [0, 1, 2] }), result);
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_lock_all(db: Pool<Postgres>) -> anyhow::Result<()> {
    use futures::StreamExt;
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: windmill_api_client::types::OpenFlow = serde_json::from_value(serde_json::json!({
        "summary": "",
        "description": "",
        "value": {
            "modules": [
                {
                    "id": "a",
                    "value": {
                        "lock": null,
                        "path": null,
                        "type": "rawscript",
                        "content": "import wmill\n\ndef main():\n  return \"Test\"\n",
                        "language": "python3",
                        "input_transforms": {}
                    },
                    "summary": null,
                    "stop_after_if": null,
                    "input_transforms": {}
                },
                {
                    "id": "b",
                    "value": {
                        "lock": null,
                        "path": null,
                        "type": "rawscript",
                        "content": "import * as wmill from \"https://deno.land/x/windmill@v1.50.0/mod.ts\"\n\nexport async function main() {\n  return wmill\n}\n",
                        "language": "deno",
                        "input_transforms": {}
                    },
                    "summary": null,
                    "stop_after_if": null,
                    "input_transforms": {}
                },
                {
                    "id": "c",
                    "value": {
                        "lock": null,
                        "path": null,
                        "type": "rawscript",
                        "content": "package inner\n\nimport (\n\t\"fmt\"\n\t\"rsc.io/quote\"\n  wmill \"github.com/windmill-labs/windmill-go-client\"\n)\n\n// the main must return (interface{}, error)\n\nfunc main() (interface{}, error) {\n\tfmt.Println(\"Hello, World\")\n  // v, _ := wmill.GetVariable(\"g/all/pretty_secret\")\n  return \"Test\"\n}\n",
                        "language": "go",
                        "input_transforms": {}
                    },
                    "summary": null,
                    "stop_after_if": null,
                    "input_transforms": {}
                },
                {
                    "id": "d",
                    "value": {
                        "lock": null,
                        "path": null,
                        "type": "rawscript",
                        "content": "\n# the last line of the stdout is the return value\necho \"Hello $msg\"\n",
                        "language": "bash",
                        "input_transforms": {}
                    },
                    "summary": null,
                    "stop_after_if": null,
                    "input_transforms": {}
                }
            ],
            "failure_module": null
        },
        "schema": {
            "type": "object",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "required": [],
            "properties": {}
        }
    }))
    .unwrap();

    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_owned(),
    );
    client
        .create_flow(
            "test-workspace",
            &CreateFlowBody {
                open_flow_w_path: windmill_api_client::types::OpenFlowWPath {
                    open_flow: flow,
                    path: "g/all/flow_lock_all".to_owned(),
                    tag: None,
                    ws_error_handler_muted: None,
                    priority: None,
                    dedicated_worker: None,
                    timeout: None,
                    visible_to_runner_only: None,
                    on_behalf_of_email: None,
                },
                draft_only: None,
                deployment_message: None,
            },
        )
        .await
        .unwrap();
    let mut str = listen_for_completed_jobs(&db).await;
    let listen_first_job = str.next();
    in_test_worker(&db, listen_first_job, port).await;

    let modules = client
        .get_flow_by_path("test-workspace", "g/all/flow_lock_all", None)
        .await
        .unwrap()
        .open_flow
        .value
        .modules;
    modules.into_iter()
        .for_each(|m| {
            assert!(matches!(
                m.value,
                windmill_api_client::types::FlowModuleValue::RawScript(RawScript {
                    language: windmill_api_client::types::RawScriptLanguage::Bash,
                    lock: Some(ref lock),
                    ..
                }) if lock.is_empty())
                || matches!(
                m.value,
                windmill_api_client::types::FlowModuleValue::RawScript(RawScript{
                    language: windmill_api_client::types::RawScriptLanguage::Go | windmill_api_client::types::RawScriptLanguage::Python3 | windmill_api_client::types::RawScriptLanguage::Deno,
                    lock: Some(ref lock),
                    ..
                }) if !lock.is_empty()),
            "{:?}", m.value
            );
        });
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]

async fn test_complex_flow_restart(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "go",
                    "content": "package inner\nimport (\n\t\"fmt\"\n\t\"math/rand\"\n)\nfunc main(max int) (interface{}, error) {\n\tresult := rand.Intn(max) + 1\n\tfmt.Printf(\"Number generated: '%d'\", result)\n\treturn result, nil\n}",
                    "input_transforms": {
                        "max": {
                            "type": "static",
                            "value": json!(20),
                        },
                    }
                },
                "summary": "Generate random number in [1, 20]"
            },
            {
                "id": "b",
                "value":
                {
                    "type": "branchall",
                    "branches":
                    [
                        {
                            "modules":
                            [
                                {
                                    "id": "d",
                                    "value":
                                    {
                                        "type": "branchone",
                                        "default":
                                        [
                                            {
                                                "id": "f",
                                                "value":
                                                {
                                                    "type": "rawscript",
                                                    "content": "package inner\nimport \"math/rand\"\nfunc main(x int) (interface{}, error) {\n\treturn rand.Intn(x) + 1, nil\n}",
                                                    "language": "go",
                                                    "input_transforms":
                                                    {
                                                        "x":
                                                        {
                                                            "expr": "results.a",
                                                            "type": "javascript"
                                                        }
                                                    }
                                                },
                                                "summary": "Rand N in [1; x]"
                                            }
                                        ],
                                        "branches":
                                        [
                                            {
                                                "expr": "results.a < flow_input.max / 2",
                                                "modules":
                                                [
                                                    {
                                                        "id": "e",
                                                        "value":
                                                        {
                                                            "type": "rawscript",
                                                            "content": "package inner\nimport \"math/rand\"\nfunc main(x int) (interface{}, error) {\n\treturn rand.Intn(x * 2) + 1, nil\n}\n",
                                                            "language": "go",
                                                            "input_transforms":
                                                            {
                                                                "x":
                                                                {
                                                                    "expr": "results.a",
                                                                    "type": "javascript"
                                                                }
                                                            }
                                                        },
                                                        "summary": "Rand N in [1; x*2]"
                                                    }
                                                ],
                                                "summary": "N in first half"
                                            }
                                        ]
                                    },
                                    "summary": ""
                                }
                            ],
                            "summary": "Process x",
                            "parallel": true,
                            "skip_failure": false
                        },
                        {
                            "modules":
                            [
                                {
                                    "id": "c",
                                    "value":
                                    {
                                        "type": "rawscript",
                                        "content": "package inner\nfunc main(x int) (interface{}, error) {\n\treturn x, nil\n}",
                                        "language": "go",
                                        "input_transforms":
                                        {
                                            "x":
                                            {
                                                "expr": "results.a",
                                                "type": "javascript"
                                            }
                                        }
                                    },
                                    "summary": "Identity"
                                }
                            ],
                            "summary": "Do nothing",
                            "parallel": true,
                            "skip_failure": false
                        }
                    ],
                    "parallel": false
                },
                "summary": ""
            },
            {
                "id": "g",
                "value":
                {
                    "tag": "",
                    "type": "rawscript",
                    "content": "package inner\nimport \"fmt\"\nfunc main(x []int) (interface{}, error) {\n\tfmt.Printf(\"Results: %v\", x)\n\treturn x, nil\n}\n",
                    "language": "go",
                    "input_transforms":
                    {
                        "x":
                        {
                            "expr": "results.b",
                            "type": "javascript"
                        }
                    }
                },
                "summary": "Print results - This will get the results from the prior step directly"
            },
            {
                "id": "h",
                "value":
                {
                    "tag": "",
                    "type": "rawscript",
                    "content": "package inner\nimport (\n\t\"fmt\"\n\t\"slices\"\n)\nfunc main(x []int) (interface{}, error) {\n\tresult := slices.Max(x)\n\tfmt.Printf(\"Result is %d\", result)\n\treturn result, nil\n}",
                    "language": "go",
                    "input_transforms":
                    {
                        "x":
                        {
                            "expr": "results.b",
                            "type": "javascript"
                        }
                    }
                },
                "summary": "Choose max - this will get results.b querying get_result_by_id on the backend"
            }
        ],
    }))
    .unwrap();

    let first_run_result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .run_until_complete(&db, false, port)
            .await;

    let restarted_flow_result = RunJob::from(JobPayload::RawFlow {
        value: flow.clone(),
        path: None,
        restarted_from: Some(RestartedFrom {
            flow_job_id: first_run_result.id,
            step_id: "h".to_owned(),
            branch_or_iteration_n: None,
            flow_version: None,
            ..Default::default()
        }),
    })
    .run_until_complete(&db, false, port)
    .await;

    let first_run_result_int =
        serde_json::from_value::<i32>(first_run_result.json_result().unwrap())
            .expect("first_run_result was not an int");
    let restarted_flow_result_int =
        serde_json::from_value::<i32>(restarted_flow_result.json_result().unwrap())
            .expect("restarted_flow_result was not an int");
    assert_eq!(first_run_result_int, restarted_flow_result_int);
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_rust_client(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    )
    .list_workspaces()
    .await
    .unwrap();
    Ok(())
}

#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(fixtures("base", "schedule"))]
async fn test_script_schedule_handlers(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    let mut args = std::collections::HashMap::new();
    args.insert("fail".to_string(), json!(true));

    let now = chrono::Utc::now();
    // add 5 seconds to now
    let then = now
        .checked_add_signed(chrono::Duration::try_seconds(5).unwrap())
        .unwrap();

    let schedule = NewSchedule {
        args: ScriptArgs::from(args),
        enabled: Some(true),
        is_flow: false,
        on_failure: Some("script/f/system/schedule_error_handler".to_string()),
        on_failure_times: None,
        on_failure_exact: None,
        on_failure_extra_args: None,
        on_recovery: Some("script/f/system/schedule_recovery_handler".to_string()),
        on_recovery_times: None,
        on_recovery_extra_args: None,
        on_success: None,
        on_success_extra_args: None,
        path: "f/system/failing_script_schedule".to_string(),
        script_path: "f/system/failing_script".to_string(),
        timezone: "UTC".to_string(),
        schedule: format!("{} {} * * * *", then.second(), then.minute()).to_string(),
        ws_error_handler_muted: None,
        retry: None,
        no_flow_overlap: None,
        summary: None,
        tag: None,
        cron_version: None,
        description: None,
    };

    let _ = client.create_schedule("test-workspace", &schedule).await;

    let mut str = listen_for_completed_jobs(&db).await;

    let db2 = db.clone();
    in_test_worker(
        &db,
        async move {
            str.next().await; // completed error job

            let uuid = timeout(Duration::from_millis(5000), str.next()).await; // error handler

            if uuid.is_err() {
                panic!("schedule error handler was not run within 5 s");
            }

            let uuid = uuid.unwrap().unwrap();

            let completed_job = sqlx::query!(
                "SELECT runnable_path as script_path FROM v2_job WHERE id = $1",
                uuid
            )
            .fetch_one(&db2)
            .await
            .unwrap();

            if completed_job.script_path.is_none()
                || completed_job.script_path != Some("f/system/schedule_error_handler".to_string())
            {
                panic!(
                    "a script was run after main job execution but was not schedule error handler"
                );
            }
        },
        port,
    )
    .await;

    let mut args = std::collections::HashMap::new();
    args.insert("fail".to_string(), json!(false));
    let now = chrono::Utc::now();
    let then = now
        .checked_add_signed(chrono::Duration::try_seconds(5).unwrap())
        .unwrap();
    client
        .update_schedule(
            "test-workspace",
            "f/system/failing_script_schedule",
            &EditSchedule {
                args: ScriptArgs::from(args),
                on_failure: Some("script/f/system/schedule_error_handler".to_string()),
                on_failure_times: None,
                on_failure_exact: None,
                on_failure_extra_args: None,
                on_recovery: Some("script/f/system/schedule_recovery_handler".to_string()),
                on_recovery_times: None,
                on_recovery_extra_args: None,
                on_success: None,
                on_success_extra_args: None,
                timezone: "UTC".to_string(),
                schedule: format!("{} {} * * * *", then.second(), then.minute()).to_string(),
                ws_error_handler_muted: None,
                retry: None,
                summary: None,
                no_flow_overlap: None,
                tag: None,
                cron_version: None,
                description: None,
            },
        )
        .await
        .unwrap();

    let mut str = listen_for_completed_jobs(&db).await;

    let db2 = db.clone();
    in_test_worker(
        &db,
        async move {
            str.next().await; // completed working job
            let uuid = timeout(Duration::from_millis(5000), str.next()).await; // recovery handler

            if uuid.is_err() {
                panic!("schedule recovery handler was not run within 5 s");
            }

            let uuid = uuid.unwrap().unwrap();

            let completed_job =
                sqlx::query!("SELECT runnable_path as script_path FROM v2_job WHERE id = $1", uuid)
                    .fetch_one(&db2)
                    .await
                    .unwrap();

            if completed_job.script_path.is_none()
                || completed_job.script_path
                    != Some("f/system/schedule_recovery_handler".to_string())
            {
                panic!("a script was run after main job execution but was not schedule recovery handler");
            }
        },
        port,
    )
    .await;

    Ok(())
}

#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(fixtures("base", "schedule"))]
async fn test_flow_schedule_handlers(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    let mut args = std::collections::HashMap::new();
    args.insert("fail".to_string(), json!(true));

    let now = chrono::Utc::now();
    // add 5 seconds to now
    let then = now
        .checked_add_signed(chrono::Duration::try_seconds(5).unwrap())
        .unwrap();

    let schedule = NewSchedule {
        args: ScriptArgs::from(args),
        enabled: Some(true),
        is_flow: true,
        on_failure: Some("script/f/system/schedule_error_handler".to_string()),
        on_failure_times: None,
        on_failure_exact: None,
        on_failure_extra_args: None,
        on_recovery: Some("script/f/system/schedule_recovery_handler".to_string()),
        on_recovery_times: None,
        on_recovery_extra_args: None,
        on_success: None,
        on_success_extra_args: None,
        path: "f/system/failing_flow_schedule".to_string(),
        script_path: "f/system/failing_flow".to_string(),
        timezone: "UTC".to_string(),
        schedule: format!("{} {} * * * *", then.second(), then.minute()).to_string(),
        ws_error_handler_muted: None,
        retry: None,
        no_flow_overlap: None,
        summary: None,
        tag: None,
        cron_version: None,
        description: None,
    };

    let _ = client.create_schedule("test-workspace", &schedule).await;

    let mut str = listen_for_completed_jobs(&db).await;

    let db2 = db.clone();
    in_test_worker(
        &db,
        async move {
            str.next().await; // completed error step
            str.next().await; // completed error flow

            let uuid = timeout(Duration::from_millis(5000), str.next()).await; // error handler

            if uuid.is_err() {
                panic!("schedule error handler was not run within 5 s");
            }

            let uuid = uuid.unwrap().unwrap();

            let completed_job = sqlx::query!(
                "SELECT runnable_path as script_path FROM v2_job WHERE id = $1",
                uuid
            )
            .fetch_one(&db2)
            .await
            .unwrap();

            if completed_job.script_path.is_none()
                || completed_job.script_path != Some("f/system/schedule_error_handler".to_string())
            {
                panic!(
                    "a script was run after main job execution but was not schedule error handler"
                );
            }
        },
        port,
    )
    .await;

    let mut args = std::collections::HashMap::new();
    args.insert("fail".to_string(), json!(false));
    let now = chrono::Utc::now();
    let then = now
        .checked_add_signed(chrono::Duration::try_seconds(5).unwrap())
        .unwrap();
    client
        .update_schedule(
            "test-workspace",
            "f/system/failing_flow_schedule",
            &EditSchedule {
                args: ScriptArgs::from(args),
                on_failure: Some("script/f/system/schedule_error_handler".to_string()),
                on_failure_times: None,
                on_failure_exact: None,
                on_failure_extra_args: None,
                on_recovery: Some("script/f/system/schedule_recovery_handler".to_string()),
                on_recovery_times: None,
                on_recovery_extra_args: None,
                on_success: None,
                on_success_extra_args: None,
                timezone: "UTC".to_string(),
                schedule: format!("{} {} * * * *", then.second(), then.minute()).to_string(),
                ws_error_handler_muted: None,
                retry: None,
                summary: None,
                no_flow_overlap: None,
                tag: None,
                cron_version: None,
                description: None,
            },
        )
        .await
        .unwrap();

    let mut str = listen_for_completed_jobs(&db).await;

    let db2 = db.clone();
    in_test_worker(
        &db,
        async move {
            str.next().await; // completed working step
            str.next().await; // completed working flow
            let uuid = timeout(Duration::from_millis(5000), str.next()).await; // recovery handler

            if uuid.is_err() {
                panic!("schedule recovery handler was not run within 5 s");
            }

            let uuid = uuid.unwrap().unwrap();

            let completed_job =
                sqlx::query!("SELECT runnable_path as script_path FROM v2_job WHERE id = $1", uuid)
                    .fetch_one(&db2)
                    .await
                    .unwrap();

            if completed_job.script_path.is_none()
                || completed_job.script_path
                    != Some("f/system/schedule_recovery_handler".to_string())
            {
                panic!("a script was run after main job execution but was not schedule recovery handler");
            }
        },
        port,
    )
    .await;

    Ok(())
}

#[sqlx::test(fixtures("base", "relative_bun"))]
async fn test_relative_imports_bun(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"
import { main as test1 } from "/f/system/same_folder_script.ts";
import { main as test2 } from "./same_folder_script.ts";
import { main as test3 } from "/f/system_relative/different_folder_script.ts";
import { main as test4 } from "../system_relative/different_folder_script.ts";

export async function main() {
  return [test1(), test2(), test3(), test4()];
}
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Bun).await?;
    run_preview_relative_imports(&db, content, ScriptLang::Bun).await?;
    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base", "relative_bun"))]
async fn test_nested_imports_bun(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"
import { main as test } from "/f/system_relative/nested_script.ts";

export async function main() {
  return test();
}
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Bun).await?;
    run_preview_relative_imports(&db, content, ScriptLang::Bun).await?;
    Ok(())
}

#[sqlx::test(fixtures("base", "relative_deno"))]
async fn test_relative_imports_deno(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"
import { main as test1 } from "/f/system/same_folder_script.ts";
import { main as test2 } from "./same_folder_script.ts";
import { main as test3 } from "/f/system_relative/different_folder_script.ts";
import { main as test4 } from "../system_relative/different_folder_script.ts";

export async function main() {
  return [test1(), test2(), test3(), test4()];
}
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Deno).await?;
    run_preview_relative_imports(&db, content, ScriptLang::Deno).await?;
    Ok(())
}

#[sqlx::test(fixtures("base", "relative_deno"))]
async fn test_nested_imports_deno(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"
import { main as test } from "/f/system_relative/nested_script.ts";

export async function main() {
  return test();
}
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Deno).await?;
    run_preview_relative_imports(&db, content, ScriptLang::Deno).await?;
    Ok(())
}

#[sqlx::test(fixtures("base", "result_format"))]
async fn test_result_format(db: Pool<Postgres>) -> anyhow::Result<()> {
    let ordered_result_job_id = "1eecb96a-c8b0-4a3d-b1b6-087878c55e41";

    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await?;

    let port = server.addr.port();

    let token = windmill_common::auth::create_token_for_owner(
        &db,
        "test-workspace",
        "u/test-user",
        "",
        100,
        "",
        &Uuid::nil(),
        None,
        None,
    )
    .await
    .unwrap();

    #[derive(Debug, Deserialize)]
    struct JobResponse {
        result: Option<Box<serde_json::value::RawValue>>,
    }

    async fn get_result<T: DeserializeOwned>(url: String) -> T {
        reqwest::get(url)
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    let correct_result = r#"[{"b":"first","a":"second"}]"#;

    let job_response: JobResponse = get_result(format!("http://localhost:{port}/api/w/test-workspace/jobs_u/get/{ordered_result_job_id}?token={token}&no_logs=true")).await;
    assert_eq!(job_response.result.unwrap().get(), correct_result);

    let job_response: JobResponse = get_result(format!("http://localhost:{port}/api/w/test-workspace/jobs_u/completed/get_result_maybe/{ordered_result_job_id}?token={token}&no_logs=true")).await;
    assert_eq!(job_response.result.unwrap().get(), correct_result);

    let job_result: Box<serde_json::value::RawValue> = get_result(format!("http://localhost:{port}/api/w/test-workspace/jobs_u/completed/get_result/{ordered_result_job_id}?token={token}&no_logs=true")).await;
    assert_eq!(job_result.get(), correct_result);

    let response = windmill_api::jobs::run_wait_result(
        &db,
        Uuid::parse_str(ordered_result_job_id).unwrap(),
        "test-workspace",
        None,
        "test-user",
    )
    .await
    .unwrap();
    let result: Box<serde_json::value::RawValue> = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(result.get(), correct_result);
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_job_labels(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let db = &db;
    let test = |original_labels: &'static [&'static str]| async move {
        let job = RunJob::from(JobPayload::RawFlow {
            value: serde_json::from_value(json!({
                "modules": [{
                    "id": "a",
                    "value": {
                        "type": "rawscript",
                        "content": r#"export function main(world: string) {
                        const greet = `Hello ${world}!`;
                        console.log(greet)
                        return { greet, wm_labels: ["yolo", "greet", "greet", world] };
                    }"#,
                        "language": "deno",
                        "input_transforms": {
                            "world": { "type": "javascript", "expr": "flow_input.world" }
                        }
                    }
                }],
                "schema": {
                    "$schema": "https://json-schema.org/draft/2020-12/schema",
                    "properties": { "world": { "type": "string" } },
                    "type": "object",
                    "order": [ "world" ]
                }
            }))
            .unwrap(),
            path: None,
            restarted_from: None,
        })
        .arg("world", json!("you"))
        .run_until_complete_with(db, false, port, |id| async move {
            sqlx::query!(
                "UPDATE v2_job SET labels = $2 WHERE id = $1 AND $2::TEXT[] IS NOT NULL",
                id,
                original_labels as &[&str],
            )
            .execute(db)
            .await
            .unwrap();
        })
        .await;

        let result = job.json_result().unwrap();
        assert_eq!(result.get("greet"), Some(&json!("Hello you!")));
        let labels = sqlx::query_scalar!("SELECT labels FROM v2_job WHERE id = $1", job.id)
            .fetch_one(db)
            .await
            .unwrap();
        let mut expected_labels = original_labels
            .iter()
            .chain(&["yolo", "greet", "you"])
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        expected_labels.sort();
        assert_eq!(labels, Some(expected_labels));
    };
    test(&[]).await;
    test(&["z", "a", "x"]).await;
    Ok(())
}

#[cfg(feature = "python")]
const WORKFLOW_AS_CODE: &str = r#"
from wmill import task

import pandas as pd
import numpy as np

@task()
def heavy_compute(n: int):
    df = pd.DataFrame(np.random.randn(100, 4), columns=list('ABCD'))
    return df.sum().sum()

@task
def send_result(res: int, email: str):
    print(f"Sending result {res} to {email}")
    return "OK"

def main(n: int):
    l = []
    for i in range(n):
        l.append(heavy_compute(i))
    print(l)
    return [send_result(sum(l), "example@example.com"), n]


"#;

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "hello"))]
async fn test_workflow_as_code(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // workflow as code require at least 2 workers:
    let db = &db;
    in_test_worker(
        db,
        async move {
            let job = Box::pin(
                RunJob::from(JobPayload::Code(RawCode {
                    language: ScriptLang::Python3,
                    content: WORKFLOW_AS_CODE.into(),
                    ..RawCode::default()
                }))
                .arg("n", json!(3))
                .run_until_complete(db, false, port),
            )
            .await;

            assert_eq!(job.json_result().unwrap(), json!(["OK", 3]));

            let workflow_as_code_status = sqlx::query_scalar!(
                "SELECT workflow_as_code_status FROM v2_job_completed WHERE id = $1",
                job.id
            )
            .fetch_one(db)
            .await
            .unwrap()
            .unwrap();

            #[derive(Deserialize)]
            #[allow(dead_code)]
            struct WorkflowJobStatus {
                name: String,
                started_at: String,
                scheduled_for: String,
                duration_ms: i64,
            }

            let workflow_as_code_status: std::collections::HashMap<String, WorkflowJobStatus> =
                serde_json::from_value(workflow_as_code_status).unwrap();

            let uuids = sqlx::query_scalar!("SELECT id FROM v2_job WHERE parent_job = $1", job.id)
                .fetch_all(db)
                .await
                .unwrap();

            assert_eq!(uuids.len(), 4);
            for uuid in uuids {
                let status = workflow_as_code_status.get(&uuid.to_string());
                assert!(status.is_some());
                assert!(
                    status.unwrap().name == "send_result"
                        || status.unwrap().name == "heavy_compute"
                );
            }
        },
        port,
    )
    .await;
    Ok(())
}

#[cfg(feature = "duckdb")]
#[sqlx::test(fixtures("base"))]
async fn test_duckdb_ffi(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    let content = "-- result_collection=last_statement_first_row_scalar\nSELECT 'Hello world!';";

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [{
            "value": {
                "type": "rawscript",
                "language": "duckdb",
                "content": content,
            },
        }],
    }))
    .unwrap();

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .run_until_complete(&db, false, server.addr.port())
            .await
            .json_result()
            .unwrap();
    assert_eq!(result, serde_json::json!("Hello world!"));
    Ok(())
}

/// Test that flow substeps with tags that are not available for the workspace fail.
/// This validates that `check_tag_available_for_workspace_internal` is properly called
/// when pushing jobs from worker_flow.
#[sqlx::test(fixtures("base"))]
async fn test_flow_substep_tag_availability_check(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_common::worker::{
        CustomTags, SpecificTagData, SpecificTagType, CUSTOM_TAGS_PER_WORKSPACE,
    };

    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    // Set up a restricted tag that is only available to "other-workspace" (not "test-workspace")
    CUSTOM_TAGS_PER_WORKSPACE.store(std::sync::Arc::new(CustomTags {
        global: vec![],
        specific: std::collections::HashMap::from([(
            "restricted-tag".to_string(),
            SpecificTagData {
                tag_type: SpecificTagType::NoneExcept,
                workspaces: vec!["other-workspace".to_string()],
            },
        )]),
    }));

    // Create a flow with a substep that uses the restricted tag
    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [{
            "id": "a",
            "value": {
                "type": "rawscript",
                "language": "deno",
                "content": "export function main() { return 42; }",
                "tag": "restricted-tag",
            },
        }],
    }))
    .unwrap();

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .email("test2@windmill.dev")
            .run_until_complete(&db, false, server.addr.port())
            .await;

    // The flow should fail because the tag is not available for test-workspace
    assert!(
        !result.success,
        "Flow should have failed due to unavailable tag"
    );

    let result_json = result.json_result();
    assert!(result_json.is_some(), "Result should have error details");

    let error_result = result_json.unwrap();
    let error_message = error_result["error"]["message"].as_str().unwrap_or("");

    // Verify the error is about tag availability
    assert!(
        error_message.contains("restricted-tag") || error_message.contains("tag"),
        "Error message should mention the tag issue: {}",
        error_message
    );

    // Clean up: reset custom tags
    CUSTOM_TAGS_PER_WORKSPACE.store(std::sync::Arc::new(CustomTags::default()));

    Ok(())
}

#[cfg(all(feature = "quickjs", feature = "python"))]
#[sqlx::test(fixtures("base"))]
async fn test_stop_after_all_iters_if_bad_expr_parallel_branchall(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let port = 123;
    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "branches": [
                        {"modules": [{
                            "id": "b",
                            "value": {
                                "input_transforms": { "n": { "type": "javascript", "expr": "flow_input.n" } },
                                "type": "rawscript",
                                "language": "python3",
                                "content": "def main(n): return n",
                            },
                        }]}
                    ],
                    "type": "branchall",
                    "parallel": true,
                },
                "stop_after_all_iters_if": {
                    "expr": "invalid!!!syntax",
                    "skip_if_stopped": false,
                },
            },
        ],
    }))
    .unwrap();
    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };

    let cjob = RunJob::from(job)
        .arg("n", json!(42))
        .run_until_complete(&db, false, port)
        .await;

    assert!(
        !cjob.success,
        "flow should fail when stop_after_all_iters_if has bad expression"
    );

    let result = cjob.json_result().unwrap();
    let error_msg = result["error"]["message"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("stop_after_all_iters_if"),
        "error should mention stop_after_all_iters_if, got: {error_msg}"
    );

    Ok(())
}

#[cfg(all(feature = "quickjs", feature = "python"))]
#[sqlx::test(fixtures("base"))]
async fn test_stop_after_all_iters_if_bad_expr_parallel_forloop(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let port = 123;
    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "result.items" },
                    "skip_failures": false,
                    "parallel": true,
                    "modules": [{
                        "value": {
                            "input_transforms": {
                                "n": { "type": "javascript", "expr": "flow_input.iter.value" },
                            },
                            "type": "rawscript",
                            "language": "python3",
                            "content": "def main(n): return n",
                        },
                    }],
                },
                "stop_after_all_iters_if": {
                    "expr": "invalid!!!syntax",
                    "skip_if_stopped": false,
                },
            },
        ],
    }))
    .unwrap();
    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };

    let cjob = RunJob::from(job)
        .arg("items", json!([1, 2, 3]))
        .run_until_complete(&db, false, port)
        .await;

    assert!(
        !cjob.success,
        "flow should fail when stop_after_all_iters_if has bad expression"
    );

    let result = cjob.json_result().unwrap();
    let error_msg = result["error"]["message"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("stop_after_all_iters_if"),
        "error should mention stop_after_all_iters_if, got: {error_msg}"
    );

    Ok(())
}

#[cfg(all(feature = "quickjs", feature = "python"))]
#[sqlx::test(fixtures("base"))]
async fn test_results_length_in_input_transform(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Step a returns a list, step b accesses results.a.length via input transform.
    // This tests that the handle_full_regex fast path falls through to QuickJS
    // when the SQL JSON path operator can't resolve JS properties like .length.
    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(): return [10, 20, 30]",
                },
            },
            {
                "id": "b",
                "value": {
                    "input_transforms": {
                        "v": { "type": "javascript", "expr": "results.a.length" },
                    },
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(v): return v",
                },
            },
        ],
    }))
    .unwrap();

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

    assert_eq!(
        result,
        json!(3),
        "results.a.length should resolve to 3, not null"
    );

    Ok(())
}
