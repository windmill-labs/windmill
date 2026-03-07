use windmill_parser_wac::dag::DagNodeType;
use windmill_parser_wac::python::parse_python_workflow;

#[test]
fn test_simple_sequential_workflow() {
    let code = r#"
import asyncio
from wmill import workflow, task

@task
async def extract_data(url: str): ...
@task
async def load_data(data: list): ...

@workflow
async def my_etl(url: str):
    raw = await extract_data(url=url)
    await load_data(data=raw)
    return {"status": "done"}
"#;

    let dag = parse_python_workflow(code).expect("should parse");
    assert_eq!(dag.nodes.len(), 3); // 2 steps + 1 return
    assert_eq!(dag.edges.len(), 2); // step0->step1, step1->return

    // Check params (url — no ctx to skip)
    assert_eq!(dag.params.len(), 1);
    assert_eq!(dag.params[0].name, "url");
    assert_eq!(dag.params[0].typ.as_deref(), Some("str"));

    // Check first step
    match &dag.nodes[0].node_type {
        DagNodeType::Step { name, script } => {
            assert_eq!(name, "extract_data");
            assert_eq!(script, "extract_data");
        }
        _ => panic!("expected Step node"),
    }

    // Check second step
    match &dag.nodes[1].node_type {
        DagNodeType::Step { name, script } => {
            assert_eq!(name, "load_data");
            assert_eq!(script, "load_data");
        }
        _ => panic!("expected Step node"),
    }

    // Check return
    assert!(matches!(dag.nodes[2].node_type, DagNodeType::Return));

    // Check source hash is non-empty
    assert!(!dag.source_hash.is_empty());
}

#[test]
fn test_parallel_workflow() {
    let code = r#"
import asyncio
from wmill import workflow, task

@task
async def extract_data(url: str): ...
@task
async def clean_data(data: list): ...
@task
async def compute_stats(data: list): ...
@task
async def load_to_warehouse(rows: list): ...

@workflow
async def my_etl(url: str):
    raw = await extract_data(url=url)
    cleaned, stats = await asyncio.gather(
        clean_data(data=raw),
        compute_stats(data=raw),
    )
    await load_to_warehouse(rows=cleaned)
    return {"status": "done"}
"#;

    let dag = parse_python_workflow(code).expect("should parse");

    // extract, ParallelStart, clean, stats, ParallelEnd, load, return = 7
    assert_eq!(dag.nodes.len(), 7);

    assert!(matches!(dag.nodes[0].node_type, DagNodeType::Step { .. }));
    assert!(matches!(dag.nodes[1].node_type, DagNodeType::ParallelStart));
    assert!(matches!(dag.nodes[2].node_type, DagNodeType::Step { .. }));
    assert!(matches!(dag.nodes[3].node_type, DagNodeType::Step { .. }));
    assert!(matches!(dag.nodes[4].node_type, DagNodeType::ParallelEnd));
    assert!(matches!(dag.nodes[5].node_type, DagNodeType::Step { .. }));
    assert!(matches!(dag.nodes[6].node_type, DagNodeType::Return));
}

#[test]
fn test_conditional_workflow() {
    let code = r#"
import asyncio
from wmill import workflow, task

@task
async def send_alert(msg: str): ...
@task
async def load_data(): ...

@workflow
async def my_etl(count: int):
    if count > 100:
        await send_alert(msg="large")
    await load_data()
    return {"done": True}
"#;

    let dag = parse_python_workflow(code).expect("should parse");
    // Branch, notify step, load step, return = 4
    assert_eq!(dag.nodes.len(), 4);
    assert!(matches!(dag.nodes[0].node_type, DagNodeType::Branch { .. }));
    assert!(matches!(dag.nodes[1].node_type, DagNodeType::Step { .. }));
}

#[test]
fn test_for_loop_workflow() {
    let code = r#"
import asyncio
from wmill import workflow, task

@task
async def process_item(item: str): ...

@workflow
async def my_etl(items: list):
    for item in items:
        await process_item(item=item)
    return {"done": True}
"#;

    let dag = parse_python_workflow(code).expect("should parse");
    // LoopStart, step, LoopEnd, return = 4
    assert_eq!(dag.nodes.len(), 4);
    assert!(matches!(
        dag.nodes[0].node_type,
        DagNodeType::LoopStart { .. }
    ));
    assert!(matches!(dag.nodes[1].node_type, DagNodeType::Step { .. }));
    assert!(matches!(dag.nodes[2].node_type, DagNodeType::LoopEnd));
}

#[test]
fn test_reject_step_in_try() {
    let code = r#"
import asyncio
from wmill import workflow, task

@task
async def extract_data(): ...

@workflow
async def my_etl():
    try:
        await extract_data()
    except Exception:
        pass
"#;

    let result = parse_python_workflow(code);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("try/except"));
}

#[test]
fn test_reject_step_in_while() {
    let code = r#"
import asyncio
from wmill import workflow, task

@task
async def extract_data(): ...

@workflow
async def my_etl():
    while True:
        await extract_data()
"#;

    let result = parse_python_workflow(code);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("while"));
}

#[test]
fn test_reject_non_async() {
    let code = r#"
from wmill import workflow

@workflow
def my_etl():
    pass
"#;

    let result = parse_python_workflow(code);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("async"));
}

#[test]
fn test_reject_missing_await() {
    let code = r#"
import asyncio
from wmill import workflow, task

@task
async def extract_data(): ...

@workflow
async def my_etl():
    extract_data()
"#;

    let result = parse_python_workflow(code);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("awaited"));
}

#[test]
fn test_no_workflow_function() {
    let code = r#"
async def my_func():
    pass
"#;

    let result = parse_python_workflow(code);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("No @workflow"));
}

#[test]
fn test_task_with_external_path() {
    let code = r#"
import asyncio
from wmill import workflow, task

@task(path="f/external_script")
async def run_external(x: int): ...

@workflow
async def my_wf(x: int):
    result = await run_external(x=x)
    return result
"#;

    let dag = parse_python_workflow(code).expect("should parse");
    assert_eq!(dag.nodes.len(), 2); // 1 step + 1 return (bare `return` is not a step node but walk_return creates one)
    match &dag.nodes[0].node_type {
        DagNodeType::Step { name, script } => {
            assert_eq!(name, "run_external");
            assert_eq!(script, "f/external_script");
        }
        _ => panic!("expected Step node"),
    }
}
