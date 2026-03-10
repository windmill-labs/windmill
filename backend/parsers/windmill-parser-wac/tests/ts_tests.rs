use windmill_parser_wac::dag::DagNodeType;
use windmill_parser_wac::typescript::parse_ts_workflow;

#[test]
fn test_simple_sequential_ts_workflow() {
    let code = r#"
import { workflow, task } from "windmill-client";

const extract_data = task(async (url: string) => {});
const load_data = task(async (data: any) => {});

export default workflow(async (url: string) => {
  const raw = await extract_data(url);
  await load_data(raw);
  return { status: "done" };
});
"#;

    let dag = parse_ts_workflow(code).expect("should parse");
    assert_eq!(dag.nodes.len(), 3); // 2 steps + 1 return
    assert_eq!(dag.edges.len(), 2);

    // Check params (url — no ctx to skip)
    assert_eq!(dag.params.len(), 1);
    assert_eq!(dag.params[0].name, "url");
    assert_eq!(dag.params[0].typ.as_deref(), Some("string"));

    match &dag.nodes[0].node_type {
        DagNodeType::Step { name, script } => {
            assert_eq!(name, "extract_data");
            assert_eq!(script, "extract_data");
        }
        _ => panic!("expected Step node"),
    }

    match &dag.nodes[1].node_type {
        DagNodeType::Step { name, script } => {
            assert_eq!(name, "load_data");
            assert_eq!(script, "load_data");
        }
        _ => panic!("expected Step node"),
    }

    assert!(matches!(dag.nodes[2].node_type, DagNodeType::Return));
    assert!(!dag.source_hash.is_empty());
}

#[test]
fn test_parallel_ts_workflow() {
    let code = r#"
import { workflow, task } from "windmill-client";

const extract_data = task(async (url: string) => {});
const clean_data = task(async (data: any) => {});
const compute_stats = task(async (data: any) => {});
const load_to_warehouse = task(async (rows: any) => {});

export default workflow(async (url: string) => {
  const raw = await extract_data(url);

  const [cleaned, stats] = await Promise.all([
    clean_data(raw),
    compute_stats(raw),
  ]);

  await load_to_warehouse(cleaned);
  return { status: "done", rows: stats.rowCount };
});
"#;

    let dag = parse_ts_workflow(code).expect("should parse");
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
fn test_conditional_ts_workflow() {
    let code = r#"
import { workflow, task } from "windmill-client";

const send_alert = task(async (msg: string) => {});
const load_data = task(async () => {});

export default workflow(async (count: number) => {
  if (count > 100) {
    await send_alert("large");
  }
  await load_data();
  return { done: true };
});
"#;

    let dag = parse_ts_workflow(code).expect("should parse");
    // Branch, notify, load, return = 4
    assert_eq!(dag.nodes.len(), 4);
    assert!(matches!(dag.nodes[0].node_type, DagNodeType::Branch { .. }));
}

#[test]
fn test_for_of_ts_workflow() {
    let code = r#"
import { workflow, task } from "windmill-client";

const process_item = task(async (item: string) => {});

export default workflow(async (items: string[]) => {
  for (const item of items) {
    await process_item(item);
  }
  return { done: true };
});
"#;

    let dag = parse_ts_workflow(code).expect("should parse");
    // LoopStart, step, LoopEnd, return = 4
    assert_eq!(dag.nodes.len(), 4);
    assert!(matches!(
        dag.nodes[0].node_type,
        DagNodeType::LoopStart { .. }
    ));
}

#[test]
fn test_reject_step_in_try_catch() {
    let code = r#"
import { workflow, task } from "windmill-client";

const extract_data = task(async () => {});

export default workflow(async () => {
  try {
    await extract_data();
  } catch (e) {
    console.log(e);
  }
});
"#;

    let result = parse_ts_workflow(code);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("catch"));
}

#[test]
fn test_reject_step_in_while_ts() {
    let code = r#"
import { workflow, task } from "windmill-client";

const extract_data = task(async () => {});

export default workflow(async () => {
  while (true) {
    await extract_data();
  }
});
"#;

    let result = parse_ts_workflow(code);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("while"));
}

#[test]
fn test_reject_missing_await_ts() {
    let code = r#"
import { workflow, task } from "windmill-client";

const extract_data = task(async () => {});

export default workflow(async () => {
  extract_data();
});
"#;

    let result = parse_ts_workflow(code);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("awaited"));
}

#[test]
fn test_no_workflow_wrapper() {
    let code = r#"
export default async function main(ctx: any) {
  return {};
}
"#;

    let result = parse_ts_workflow(code);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("No workflow()"));
}

#[test]
fn test_variable_declaration_with_step() {
    let code = r#"
import { workflow, task } from "windmill-client";

const compute = task(async () => {});

export default workflow(async () => {
  const result = await compute();
  return result;
});
"#;

    let dag = parse_ts_workflow(code).expect("should parse");
    assert_eq!(dag.nodes.len(), 2); // step + return
    assert!(matches!(dag.nodes[0].node_type, DagNodeType::Step { .. }));
}

#[test]
fn test_task_with_external_path() {
    let code = r#"
import { workflow, task } from "windmill-client";

const run_external = task("f/external_script", async (x: number) => {});

export default workflow(async (x: number) => {
  const result = await run_external(x);
  return result;
});
"#;

    let dag = parse_ts_workflow(code).expect("should parse");
    assert_eq!(dag.nodes.len(), 2); // step + return
    match &dag.nodes[0].node_type {
        DagNodeType::Step { name, script } => {
            assert_eq!(name, "run_external");
            assert_eq!(script, "f/external_script");
        }
        _ => panic!("expected Step node"),
    }
}
