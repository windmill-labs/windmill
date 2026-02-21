use serde::de::DeserializeOwned;

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
                    continue_on_error: None,
                    skip_if: None,
                    apply_preprocessor: None,
                    pass_flow_input_directly: None,
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
                            continue_on_error: None,
                            skip_if: None,
                            apply_preprocessor: None,
                            pass_flow_input_directly: None,
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
                    continue_on_error: None,
                    skip_if: None,
                    apply_preprocessor: None,
                    pass_flow_input_directly: None,
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
                    continue_on_error: None,
                    skip_if: None,
                    apply_preprocessor: None,
                    pass_flow_input_directly: None,
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
                                continue_on_error: None,
                                skip_if: None,
                                apply_preprocessor: None,
                                pass_flow_input_directly: None,
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
                                continue_on_error: None,
                                skip_if: None,
                                apply_preprocessor: None,
                                pass_flow_input_directly: None,
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
                    continue_on_error: None,
                    skip_if: None,
                    apply_preprocessor: None,
                    pass_flow_input_directly: None,
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
                    continue_on_error: None,
                    skip_if: None,
                    apply_preprocessor: None,
                    pass_flow_input_directly: None,
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
    }))
    .arg("msg", json!("world"))
    .run_until_complete(&db, false, port)
    .await;
    assert_eq!(job.json_result(), Some(json!("hello world")));
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
async fn test_postgresql_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
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
    }))
    .arg("msg", json!("world"))
    .run_until_complete(&db, false, port)
    .await;
    assert_eq!(job.json_result(), Some(json!("hello world")));
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
    {
        let mut custom_tags = CUSTOM_TAGS_PER_WORKSPACE.write().await;
        *custom_tags = CustomTags {
            global: vec![],
            specific: std::collections::HashMap::from([(
                "restricted-tag".to_string(),
                SpecificTagData {
                    tag_type: SpecificTagType::NoneExcept,
                    workspaces: vec!["other-workspace".to_string()],
                },
            )]),
        };
    }

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
    {
        let mut custom_tags = CUSTOM_TAGS_PER_WORKSPACE.write().await;
        *custom_tags = CustomTags::default();
    }

    Ok(())
}
