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
fn test_step_in_try_catch() {
    let code = r#"
import { workflow, task } from "windmill-client";

const extract_data = task(async () => {});
const handle_error = task(async (e: any) => {});

export default workflow(async () => {
  try {
    await extract_data();
  } catch (e) {
    await handle_error(e);
  }
});
"#;

    let dag = parse_ts_workflow(code).expect("should parse try/catch");
    // Branch(try/catch), extract_data, handle_error = 3
    assert_eq!(dag.nodes.len(), 3);
    assert!(matches!(dag.nodes[0].node_type, DagNodeType::Branch { .. }));
    assert_eq!(dag.nodes[0].label, "try");
    assert!(matches!(dag.nodes[1].node_type, DagNodeType::Step { .. }));
    assert!(matches!(dag.nodes[2].node_type, DagNodeType::Step { .. }));
}

#[test]
fn test_step_in_while_ts() {
    let code = r#"
import { workflow, task } from "windmill-client";

const poll_status = task(async () => {});

export default workflow(async () => {
  while (true) {
    await poll_status();
  }
});
"#;

    let dag = parse_ts_workflow(code).expect("should parse while loop");
    // LoopStart, poll_status, LoopEnd = 3
    assert_eq!(dag.nodes.len(), 3);
    assert!(matches!(
        dag.nodes[0].node_type,
        DagNodeType::LoopStart { .. }
    ));
    assert_eq!(dag.nodes[0].label, "while");
    assert!(matches!(dag.nodes[1].node_type, DagNodeType::Step { .. }));
    assert!(matches!(dag.nodes[2].node_type, DagNodeType::LoopEnd));
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

#[test]
fn test_task_script_and_task_flow() {
    let code = r#"
import { workflow, task, taskScript, taskFlow } from "windmill-client";

const helper = taskScript("./helper.ts");
const pipeline = taskFlow("f/etl/pipeline");
const process = task(async (x: string) => {});

export default workflow(async (x: string) => {
  const a = await process(x);
  const b = await helper({ a });
  const c = await pipeline({ b });
  return { a, b, c };
});
"#;

    let dag = parse_ts_workflow(code).expect("should parse");
    assert_eq!(dag.nodes.len(), 4); // 3 steps + 1 return

    match &dag.nodes[0].node_type {
        DagNodeType::Step { name, script } => {
            assert_eq!(name, "process");
            assert_eq!(script, "process");
        }
        _ => panic!("expected Step node"),
    }

    match &dag.nodes[1].node_type {
        DagNodeType::Step { name, script } => {
            assert_eq!(name, "helper");
            assert_eq!(script, "./helper.ts");
        }
        _ => panic!("expected Step node for taskScript"),
    }

    match &dag.nodes[2].node_type {
        DagNodeType::Step { name, script } => {
            assert_eq!(name, "pipeline");
            assert_eq!(script, "f/etl/pipeline");
        }
        _ => panic!("expected Step node for taskFlow"),
    }
}

#[test]
fn test_full_template_with_sdk_calls() {
    let code = r#"
import { task, taskScript, step, sleep, waitForApproval, getResumeUrls, workflow } from "windmill-client";

const helper = taskScript("./helper.ts");
const process = task(async (x: string): Promise<string> => {
  return `processed: ${x}`;
});

export const main = workflow(async (x: string) => {
  const a = await process(x);
  const b = await helper({ a });
  const urls = await step("get_urls", () => getResumeUrls());
  await sleep(1);
  const approval = await waitForApproval({ timeout: 3600 });
  return { processed: a, helper_result: b, approval };
});
"#;

    let dag = parse_ts_workflow(code).expect("should parse");
    // process, helper, step("get_urls"), sleep(1), waitForApproval, return = 6
    assert_eq!(dag.nodes.len(), 6);
    assert_eq!(dag.edges.len(), 5);

    assert!(matches!(dag.nodes[0].node_type, DagNodeType::Step { .. }));

    match &dag.nodes[1].node_type {
        DagNodeType::Step { name, script } => {
            assert_eq!(name, "helper");
            assert_eq!(script, "./helper.ts");
        }
        _ => panic!("expected Step node"),
    }

    match &dag.nodes[2].node_type {
        DagNodeType::InlineStep { name } => {
            assert_eq!(name, "get_urls");
        }
        _ => panic!("expected InlineStep node, got {:?}", dag.nodes[2].node_type),
    }

    match &dag.nodes[3].node_type {
        DagNodeType::Sleep { seconds } => {
            assert_eq!(seconds, "1");
        }
        _ => panic!("expected Sleep node, got {:?}", dag.nodes[3].node_type),
    }

    assert!(matches!(
        dag.nodes[4].node_type,
        DagNodeType::WaitForApproval
    ));
    assert!(matches!(dag.nodes[5].node_type, DagNodeType::Return));
}

#[test]
fn test_complex_mixed_workflow() {
    let code = r#"
import { workflow, task, step, sleep } from "windmill-client";

const validate = task(async (data: any) => {});
const process_csv = task(async (data: any) => {});
const process_json = task(async (data: any) => {});
const enrich = task(async (item: any) => {});
const store = task(async (data: any) => {});

export default workflow(async (data: any) => {
  const validated = await validate(data);
  if (validated.format === "csv") {
    const parsed = await process_csv(validated);
    for (const row of parsed.rows) {
      await enrich(row);
    }
  } else {
    await process_json(validated);
  }
  await sleep(5);
  const ts = await step("timestamp", () => new Date().toISOString());
  await store(validated);
  return { done: true };
});
"#;

    let dag = parse_ts_workflow(code).expect("should parse");

    // validate, Branch, process_csv, LoopStart, enrich, LoopEnd, process_json,
    // sleep(5), step("timestamp"), store, return = 11
    assert_eq!(dag.nodes.len(), 11);

    assert!(matches!(dag.nodes[0].node_type, DagNodeType::Step { .. }));
    assert!(matches!(dag.nodes[1].node_type, DagNodeType::Branch { .. }));
    assert!(matches!(dag.nodes[2].node_type, DagNodeType::Step { .. })); // process_csv
    assert!(matches!(
        dag.nodes[3].node_type,
        DagNodeType::LoopStart { .. }
    ));
    assert!(matches!(dag.nodes[4].node_type, DagNodeType::Step { .. })); // enrich
    assert!(matches!(dag.nodes[5].node_type, DagNodeType::LoopEnd));
    assert!(matches!(dag.nodes[6].node_type, DagNodeType::Step { .. })); // process_json
    assert!(matches!(dag.nodes[7].node_type, DagNodeType::Sleep { .. }));
    assert!(matches!(
        dag.nodes[8].node_type,
        DagNodeType::InlineStep { .. }
    )); // timestamp
    assert!(matches!(dag.nodes[9].node_type, DagNodeType::Step { .. })); // store
    assert!(matches!(dag.nodes[10].node_type, DagNodeType::Return));
}
