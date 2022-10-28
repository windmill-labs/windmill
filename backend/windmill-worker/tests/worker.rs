use futures::stream;
use futures::Stream;
use futures::StreamExt;
use serde_json::json;
use sqlx::Pool;
use sqlx::Postgres;
use sqlx::{postgres::PgListener, query_scalar};
use uuid::Uuid;
use windmill_common::flows::FlowModule;
use windmill_common::flows::FlowModuleValue;
use windmill_common::flows::FlowValue;
use windmill_common::flows::InputTransform;
use windmill_common::scripts::ScriptLang;
use windmill_common::DEFAULT_SLEEP_QUEUE;
use windmill_queue::JobPayload;
use windmill_queue::RawCode;
use windmill_worker::WorkerConfig;
async fn initialize_tracing() {
    use std::sync::Once;

    static ONCE: Once = Once::new();
    ONCE.call_once(windmill_common::tracing_init::initialize_tracing);
}

#[sqlx::test(fixtures("base"))]
async fn test_deno_flow(db: Pool<Postgres>) {
    initialize_tracing().await;

    let numbers = "export function main() { return [1, 2, 3]; }";
    let doubles = "export function main(n) { return n * 2; }";

    let flow = {
        FlowValue {
            modules: vec![
                FlowModule {
                    id: "a".to_string(),
                    value: FlowModuleValue::RawScript {
                        input_transforms: Default::default(),
                        language: ScriptLang::Deno,
                        content: numbers.to_string(),
                        path: None,
                    },
                    input_transforms: Default::default(),
                    stop_after_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                },
                FlowModule {
                    id: "b".to_string(),
                    value: FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Javascript { expr: "result".to_string() },
                        skip_failures: false,
                        modules: vec![FlowModule {
                            id: "c".to_string(),
                            value: FlowModuleValue::RawScript {
                                input_transforms: [(
                                    "n".to_string(),
                                    InputTransform::Javascript {
                                        expr: "previous_result.iter.value".to_string(),
                                    },
                                )]
                                .into(),
                                language: ScriptLang::Deno,
                                content: doubles.to_string(),
                                path: None,
                            },
                            input_transforms: Default::default(),
                            stop_after_if: Default::default(),
                            summary: Default::default(),
                            suspend: Default::default(),
                            retry: None,
                            sleep: None,
                        }],
                    },
                    input_transforms: Default::default(),
                    stop_after_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                },
            ],
            same_worker: false,
            ..Default::default()
        }
    };

    let job = JobPayload::RawFlow { value: flow, path: None };

    for i in 0..50 {
        println!("deno flow iteration: {}", i);
        let result = run_job_in_new_worker_until_complete(&db, job.clone(), None).await;
        assert_eq!(result, serde_json::json!([2, 4, 6]), "iteration: {}", i);
    }
}

#[sqlx::test(fixtures("base"))]
async fn test_deno_flow_same_worker(db: Pool<Postgres>) {
    initialize_tracing().await;

    let write_file = r#"export async function main(loop: boolean, i: number, path: string) {  
            await Deno.writeTextFile(`/shared/${path}`, `${loop} ${i}`);
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
                            InputTransform::Static { value: json!(false) },
                        ),
                        ("i".to_string(), InputTransform::Static { value: json!(1) }),
                        (
                            "path".to_string(),
                            InputTransform::Static { value: json!("outer.txt") },
                        ),
                    ]
                    .into(),
                        language: ScriptLang::Deno,
                        content: write_file.clone(),
                        path: None,
                    },
                    input_transforms: Default::default(),
                    stop_after_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                },
                FlowModule {
                    id: "b".to_string(),
                    value: FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Static { value: json!([1, 2, 3]) },
                        skip_failures: false,
                        modules: vec![
                            FlowModule {
                                id: "d".to_string(),
                                input_transforms: [
                                (
                                    "i".to_string(),
                                    InputTransform::Javascript {
                                        expr: "previous_result.iter.value".to_string(),
                                    },
                                ),
                                (
                                    "loop".to_string(),
                                    InputTransform::Static { value: json!(true) },
                                ),
                                (
                                    "path".to_string(),
                                    InputTransform::Static { value: json!("inner.txt") },
                                ),
                            ]
                            .into(),
                                value: FlowModuleValue::RawScript {
                                    input_transforms: [].into(),
                                    language: ScriptLang::Deno,
                                    content: write_file,
                                    path: None,
                                },
                                stop_after_if: Default::default(),
                                summary: Default::default(),
                                suspend: Default::default(),
                                retry: None,
                                sleep: None,
                            },
                            FlowModule {
                                id: "e".to_string(),
                                value: FlowModuleValue::RawScript {
                                    input_transforms: [(
                                        "path".to_string(),
                                        InputTransform::Static { value: json!("inner.txt") },
                                    ), (
                                        "path2".to_string(),
                                        InputTransform::Static { value: json!("outer.txt") },
                                    )]
                                    .into(),
                                    language: ScriptLang::Deno,
                                    content: r#"export async function main(path: string, path2: string) {  
                                        return await Deno.readTextFile(`/shared/${path}`) + "," + await Deno.readTextFile(`/shared/${path2}`);
                                    }"#
                                    .to_string(),
                                    path: None,
                                },
                                input_transforms: [].into(),
                                stop_after_if: Default::default(),
                                summary: Default::default(),
                                suspend: Default::default(),
                                retry: None,
                                sleep: None,
                            },
                        ],
                    },
                    input_transforms: Default::default(),
                    stop_after_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,

                },
                FlowModule {
                    id: "c".to_string(),
                    value: FlowModuleValue::RawScript {
                        input_transforms: [
                        (
                            "loops".to_string(),
                            InputTransform::Javascript { expr: "previous_result".to_string() },
                        ),
                        (
                            "path".to_string(),
                            InputTransform::Static { value: json!("outer.txt") },
                        ),
                        (
                            "path2".to_string(),
                            InputTransform::Static { value: json!("inner.txt") },
                        ),
                    ]
                    .into(),
                        language: ScriptLang::Deno,
                        content: r#"export async function main(path: string, loops: string[], path2: string) {
                            return await Deno.readTextFile(`/shared/${path}`) + "," + loops + "," + await Deno.readTextFile(`/shared/${path2}`);
                        }"#
                        .to_string(),
                        path: None,
                    },
                    input_transforms: [].into(),
                    stop_after_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                },
            ],
            same_worker: true,
            ..Default::default()
        };

    let job = JobPayload::RawFlow { value: flow, path: None };

    let result = run_job_in_new_worker_until_complete(&db, job.clone(), None).await;
    assert_eq!(
        result,
        serde_json::json!("false 1,true 1,false 1,true 2,false 1,true 3,false 1,true 3")
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_flow_result_by_id(db: Pool<Postgres>) {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await;
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
                                                "input_transforms": {"v": {"type": "javascript", "expr": "result_by_id(\"a\")"}},
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

    let job = JobPayload::RawFlow { value: flow, path: None };
    let result = run_job_in_new_worker_until_complete(&db, job.clone(), Some(port)).await;
    assert_eq!(result, serde_json::json!([[42]]));
}
#[sqlx::test(fixtures("base"))]
async fn test_stop_after_if(db: Pool<Postgres>) {
    initialize_tracing().await;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "input_transforms": { "n": { "type": "javascript", "expr": "flow_input.n" } },
                "value": {
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
                "input_transforms": { "n": { "type": "javascript", "expr": "previous_result" } },
                "value": {
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(n): return f'last step saw {n}'",
                },
            },
        ],
    }))
    .unwrap();
    let job = JobPayload::RawFlow { value: flow, path: None };

    let result = RunJob::from(job.clone())
        .arg("n", json!(123))
        .run_until_complete(&db, None)
        .await;
    assert_eq!(json!("last step saw 123"), result);

    let result = RunJob::from(job.clone())
        .arg("n", json!(-123))
        .run_until_complete(&db, None)
        .await;
    assert_eq!(json!(-123), result);
}

#[sqlx::test(fixtures("base"))]
async fn test_python_flow(db: Pool<Postgres>) {
    initialize_tracing().await;

    let numbers = "def main(): return [1, 2, 3]";
    let doubles = "def main(n): return n * 2";

    let flow: FlowValue = serde_json::from_value(serde_json::json!( {
        "input_transform": {},
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
                            "type": "rawscript",
                            "language": "python3",
                            "content": doubles,
                        },
                        "input_transform": {
                            "n": {
                                "type": "javascript",
                                "expr": "previous_result.iter.value",
                            },
                        },
                    }],
                },
            },
        ],
    }))
    .unwrap();

    for i in 0..50 {
        println!("python flow iteration: {}", i);
        let result = run_job_in_new_worker_until_complete(
            &db,
            JobPayload::RawFlow { value: flow.clone(), path: None },
            None,
        )
        .await;

        assert_eq!(result, serde_json::json!([2, 4, 6]), "iteration: {i}");
    }
}

#[sqlx::test(fixtures("base"))]
async fn test_python_flow_2(db: Pool<Postgres>) {
    initialize_tracing().await;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [
                {
                    "value": {
                        "type": "rawscript",
                        "content": "import wmill\ndef main():  return \"Hello\"",
                        "language": "python3"
                    },
                    "input_transform": {}
                }
            ]
    }))
    .unwrap();

    for i in 0..10 {
        println!("python flow iteration: {}", i);
        let result = run_job_in_new_worker_until_complete(
            &db,
            JobPayload::RawFlow { value: flow.clone(), path: None },
            None,
        )
        .await;

        assert_eq!(result, serde_json::json!("Hello"), "iteration: {i}");
    }
}

#[sqlx::test(fixtures("base"))]
async fn test_go_job(db: Pool<Postgres>) {
    initialize_tracing().await;

    let content = r#"
import "fmt"

func main(derp string) (string, error) {
	fmt.Println("Hello, 世界")
	return fmt.Sprintf("hello %s", derp), nil
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        content,
        path: None,
        language: ScriptLang::Go,
    }))
    .arg("derp", json!("world"))
    .run_until_complete(&db, None)
    .await;

    assert_eq!(result, serde_json::json!("hello world"));
}

#[sqlx::test(fixtures("base"))]
async fn test_python_job(db: Pool<Postgres>) {
    initialize_tracing().await;

    let content = r#"
def main():
    return "hello world"
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode { content, path: None, language: ScriptLang::Python3 });

    let result = run_job_in_new_worker_until_complete(&db, job, None).await;

    assert_eq!(result, serde_json::json!("hello world"));
}

#[sqlx::test(fixtures("base"))]
async fn test_python_job_heavy_dep(db: Pool<Postgres>) {
    initialize_tracing().await;

    let content = r#"
import numpy as np

def main():
    a = np.arange(15).reshape(3, 5)
    return len(a)
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode { content, path: None, language: ScriptLang::Python3 });

    let result = run_job_in_new_worker_until_complete(&db, job, None).await;

    assert_eq!(result, serde_json::json!(3));
}

#[sqlx::test(fixtures("base"))]
async fn test_python_job_with_imports(db: Pool<Postgres>) {
    initialize_tracing().await;

    let content = r#"
import wmill

def main():
    return wmill.get_workspace()
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode { content, path: None, language: ScriptLang::Python3 });

    let result = run_job_in_new_worker_until_complete(&db, job, None).await;

    assert_eq!(result, serde_json::json!("test-workspace"));
}

#[sqlx::test(fixtures("base"))]
async fn test_empty_loop(db: Pool<Postgres>) {
    initialize_tracing().await;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "static", "value": [] },
                    "modules": [
                        {
                            "input_transform": {
                                "n": {
                                    "type": "javascript",
                                    "expr": "previous_result.iter.value",
                                },
                            },
                            "value": {
                                "type": "rawscript",
                                "language": "python3",
                                "content": "def main(n): return n",
                            },
                        }
                    ],
                },
            },
            {
                "input_transform": {
                    "items": {
                        "type": "javascript",
                        "expr": "previous_result",
                    },
                },
                "value": {
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(items): return sum(items)",
                },
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, None).await;

    assert_eq!(result, serde_json::json!(0));
}

#[sqlx::test(fixtures("base"))]
async fn test_empty_loop_2(db: Pool<Postgres>) {
    initialize_tracing().await;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "static", "value": [] },
                    "modules": [
                        {
                            "input_transform": {
                                "n": {
                                    "type": "javascript",
                                    "expr": "previous_result.iter.value",
                                },
                            },
                            "value": {
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

    let flow = JobPayload::RawFlow { value: flow, path: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, None).await;

    assert_eq!(result, serde_json::json!([]));
}

#[sqlx::test(fixtures("base"))]
async fn test_step_after_loop(db: Pool<Postgres>) {
    initialize_tracing().await;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "static", "value": [2,3,4] },
                    "modules": [
                        {
                            "input_transform": {
                                "n": {
                                    "type": "javascript",
                                    "expr": "previous_result.iter.value",
                                },
                            },
                            "value": {
                                "type": "rawscript",
                                "language": "python3",
                                "content": "def main(n): return n",
                            } ,
                        }
                    ],
                },
            },
            {
                "input_transform": {
                    "items": {
                        "type": "javascript",
                        "expr": "previous_result",
                    },
                },
                "value": {
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(items): return sum(items)",
                },
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, None).await;

    assert_eq!(result, serde_json::json!(9));
}

fn module_add_item_to_list(i: i32) -> serde_json::Value {
    json!({
        "input_transform": {
            "array": {
                "type": "javascript",
                "expr": "previous_result",
            },
            "i": {
                "type": "static",
                "value": json!(i),
            }
        },
        "value": {
            "type": "rawscript",
            "language": "deno",
            "content": "export function main(array, i){ array.push(i); return array }",
        }
    })
}

fn module_failure() -> serde_json::Value {
    json!({
        "input_transform": {},
        "value": {
            "type": "rawscript",
            "language": "deno",
            "content": "export function main(){ throw Error('failure') }",
        }
    })
}

#[sqlx::test(fixtures("base"))]
async fn test_branchone_simple(db: Pool<Postgres>) {
    initialize_tracing().await;

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [],
                    "default": [module_add_item_to_list(2)],
                    "type": "branchone",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, None).await;

    assert_eq!(result, serde_json::json!([1, 2]));
}

#[sqlx::test(fixtures("base"))]
async fn test_branchall_simple(db: Pool<Postgres>) {
    initialize_tracing().await;

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [
                        {"modules": [module_add_item_to_list(2)]},
                        {"modules": [module_add_item_to_list(3)]}],
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, None).await;

    assert_eq!(result, serde_json::json!([[1, 2], [1, 3]]));
}

#[sqlx::test(fixtures("base"))]
async fn test_branchall_skip_failure(db: Pool<Postgres>) {
    initialize_tracing().await;

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
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
                        {"modules": [module_add_item_to_list(3)]}],
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, None).await;

    assert_eq!(
        result,
        serde_json::json!({"error": "Error during execution of the script:\n\nerror: Uncaught (in promise) Error: failure\nexport function main(){ throw Error('failure') }\n                              ^\n    at main (file:///tmp/inner.ts:1:31)\n    at run (file:///tmp/main.ts:9:26)\n    at file:///tmp/main.ts:14:1"})
    );

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
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
                        {"modules": [module_add_item_to_list(2)]}
                ],
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, None).await;

    assert_eq!(
        result,
        serde_json::json!([{"error": "Error during execution of the script:\n\nerror: Uncaught (in promise) Error: failure\nexport function main(){ throw Error('failure') }\n                              ^\n    at main (file:///tmp/inner.ts:1:31)\n    at run (file:///tmp/main.ts:9:26)\n    at file:///tmp/main.ts:14:1"}, [1, 2]])
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_branchone_nested(db: Pool<Postgres>) {
    initialize_tracing().await;

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [] }",
                }
            },
            module_add_item_to_list(1),
            {
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
                                    "default": [module_add_item_to_list(2)],
                                    "type": "branchone",
                                }
                            }]
                        },
                    ],
                    "default": [module_add_item_to_list(-4)],
                    "type": "branchone",
                }
            },
            module_add_item_to_list(3),
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, None).await;

    assert_eq!(result, serde_json::json!([1, 2, 3]));
}

#[sqlx::test(fixtures("base"))]
async fn test_branchall_nested(db: Pool<Postgres>) {
    initialize_tracing().await;

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
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
                            "modules": [                {
                                "value": {
                                    "branches": [
                                        {"modules": [module_add_item_to_list(2)]},
                                        {"modules": [module_add_item_to_list(3)]}],
                                    "type": "branchall",
                                }
                            }, {
                                "value": {
                                    "branches": [
                                        {"modules": [module_add_item_to_list(4)]},
                                        {"modules": [module_add_item_to_list(5)]}],
                                    "type": "branchall",
                                }
                            }
                                    ]
                        },
                        {"modules": [module_add_item_to_list(6)]}],
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, None).await;

    assert_eq!(
        result,
        serde_json::json!([[[[1, 2], [1, 3], 4], [[1, 2], [1, 3], 5]], [1, 6]])
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_failure_module(db: Pool<Postgres>) {
    initialize_tracing().await;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [{
                "input_transform": {
                    "l": { "type": "javascript", "expr": "[]", },
                    "n": { "type": "javascript", "expr": "flow_input.n", },
                },
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 0) throw l; return { l: [...l, 0] } }",
                },
            }, {
                "input_transform": {
                    "l": { "type": "javascript", "expr": "previous_result.l", },
                    "n": { "type": "javascript", "expr": "flow_input.n", },
                },
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 1) throw l; return { l: [...l, 1] } }",
                },
            }, {
                "input_transform": {
                    "l": { "type": "javascript", "expr": "previous_result.l", },
                    "n": { "type": "javascript", "expr": "flow_input.n", },
                },
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 2) throw l; return { l: [...l, 2] } }",
                },
            }],
            "failure_module": {
                "input_transform": { "error": { "type": "javascript", "expr": "previous_result", } },
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(error) { return { 'from failure module': error } }",
                }
            },
        }))
        .unwrap();

    let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
        .arg("n", json!(0))
        .run_until_complete(&db, None)
        .await;
    assert!(result["from failure module"]["error"]
        .as_str()
        .unwrap()
        .contains("Uncaught (in promise) []"));

    let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
        .arg("n", json!(1))
        .run_until_complete(&db, None)
        .await;
    assert!(result["from failure module"]["error"]
        .as_str()
        .unwrap()
        .contains("Uncaught (in promise) [ 0 ]"));

    let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
        .arg("n", json!(2))
        .run_until_complete(&db, None)
        .await;
    assert!(result["from failure module"]["error"]
        .as_str()
        .unwrap()
        .contains("Uncaught (in promise) [ 0, 1 ]"));

    let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
        .arg("n", json!(3))
        .run_until_complete(&db, None)
        .await;
    assert_eq!(json!({ "l": [0, 1, 2] }), result);
}
