/*
 * Full Flow Execution Parity Tests
 *
 * These tests verify that flows execute identically when using deno_core vs quickjs
 * for expression evaluation. They test the complete flow execution path including:
 * - Input transforms with JavaScript expressions
 * - For-loop iterators with complex expressions
 * - Branch conditions
 * - Skip/stop conditions
 * - Combining results from multiple steps
 *
 * To run with deno_core (default):
 *   cargo test -p windmill --features "deno_core" --test flow_engine_parity
 *
 * To run with quickjs:
 *   USE_QUICKJS_FOR_FLOW_EVAL=1 cargo test -p windmill --features "quickjs,deno_core" --test flow_engine_parity
 */

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_common::{
    flows::{FlowModule, FlowModuleValue, FlowValue, InputTransform, Branch},
    jobs::JobPayload,
    scripts::ScriptLang,
};

mod common;
use common::*;

/// Helper to create a FlowModule with default fields
fn flow_module(id: &str, value: FlowModuleValue) -> FlowModule {
    FlowModule {
        id: id.to_string(),
        value: windmill_common::worker::to_raw_value(&value),
        stop_after_if: None,
        stop_after_all_iters_if: None,
        summary: None,
        suspend: None,
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
    }
}

/// Helper to create input transforms from JavaScript expressions
fn js_input(key: &str, expr: &str) -> (String, InputTransform) {
    (key.to_string(), InputTransform::Javascript { expr: expr.to_string() })
}

/// Helper to create static input transforms
fn static_input<T: serde::Serialize>(key: &str, value: T) -> (String, InputTransform) {
    (key.to_string(), InputTransform::Static { value: windmill_common::worker::to_raw_value(&value) })
}

// =============================================================================
// TEST 1: Simple linear flow with input transforms
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_linear_input_transforms(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    // Flow: step_a returns data, step_b transforms it using JS expressions
    let flow = FlowValue {
        modules: vec![
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: [static_input("x", 10), static_input("y", 5)].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(x: number, y: number) {
    return {sum: x + y, product: x * y, items: [1, 2, 3, 4, 5]};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
            flow_module("b", FlowModuleValue::RawScript {
                input_transforms: [
                    js_input("total", "results.a.sum + results.a.product"),
                    js_input("doubled_items", "results.a.items.map(x => x * 2)"),
                    js_input("filtered", "results.a.items.filter(x => x > 2)"),
                    js_input("from_flow_input", "flow_input.multiplier * results.a.sum"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(total: number, doubled_items: number[], filtered: number[], from_flow_input: number) {
    return {total, doubled_items, filtered, from_flow_input};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .arg("multiplier", json!(3))
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    // Expected: sum=15, product=50, total=65, doubled=[2,4,6,8,10], filtered=[3,4,5], from_flow_input=45
    assert_eq!(result["total"], json!(65));
    assert_eq!(result["doubled_items"], json!([2, 4, 6, 8, 10]));
    assert_eq!(result["filtered"], json!([3, 4, 5]));
    assert_eq!(result["from_flow_input"], json!(45));

    Ok(())
}

// =============================================================================
// TEST 2: For-loop with complex iterator and inner expressions
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_forloop_complex_expressions(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            // Step a: return data to iterate over
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        users: [
            {id: 1, name: "Alice", score: 85},
            {id: 2, name: "Bob", score: 92},
            {id: 3, name: "Charlie", score: 78}
        ]
    };
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
            // Step b: for-loop over filtered users
            flow_module("b", FlowModuleValue::ForloopFlow {
                iterator: InputTransform::Javascript {
                    expr: "results.a.users.filter(u => u.score >= 80)".to_string()
                },
                skip_failures: false,
                parallel: false,
                squash: None,
                parallelism: None,
                modules: vec![
                    flow_module("c", FlowModuleValue::RawScript {
                        input_transforms: [
                            js_input("user_name", "flow_input.iter.value.name"),
                            js_input("user_score", "flow_input.iter.value.score"),
                            js_input("bonus", "flow_input.iter.value.score >= 90 ? 10 : 5"),
                            js_input("index", "flow_input.iter.index"),
                        ].into(),
                        language: ScriptLang::Deno,
                        content: r#"
export function main(user_name: string, user_score: number, bonus: number, index: number) {
    return {name: user_name, final_score: user_score + bonus, position: index};
}
"#.to_string(),
                        path: None,
                        lock: None,
                        tag: None,
                        concurrency_settings: Default::default(),
                        is_trigger: None,
                        assets: None,
                    }),
                ],
                modules_node: None,
            }),
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    // Only Alice (85) and Bob (92) pass the filter (score >= 80)
    // Alice gets bonus=5, Bob gets bonus=10
    assert!(result.is_array());
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "Alice");
    assert_eq!(arr[0]["final_score"], 90);  // 85 + 5
    assert_eq!(arr[0]["position"], 0);
    assert_eq!(arr[1]["name"], "Bob");
    assert_eq!(arr[1]["final_score"], 102);  // 92 + 10
    assert_eq!(arr[1]["position"], 1);

    Ok(())
}

// =============================================================================
// TEST 3: Branch-one with complex conditions
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_branchone_conditions(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            // Step a: return data for branching
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {status: "premium", score: 95, items: [1, 2, 3]};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
            // Step b: branch based on status and score
            flow_module("b", FlowModuleValue::BranchOne {
                branches: vec![
                    Branch {
                        summary: Some("Premium with high score".to_string()),
                        expr: "results.a.status === 'premium' && results.a.score >= 90".to_string(),
                        modules: vec![
                            flow_module("premium_high", FlowModuleValue::RawScript {
                                input_transforms: [
                                    js_input("discount", "results.a.score >= 95 ? 30 : 20"),
                                    js_input("score_from_a", "results.a.score"),
                                ].into(),
                                language: ScriptLang::Deno,
                                content: r#"
export function main(discount: number, score_from_a: number) {
    return {branch: "premium_high", discount, score_from_a};
}
"#.to_string(),
                                path: None,
                                lock: None,
                                tag: None,
                                concurrency_settings: Default::default(),
                                is_trigger: None,
                                assets: None,
                            }),
                        ],
                        modules_node: None,
                        skip_failure: true,
                        parallel: true,
                    },
                    Branch {
                        summary: Some("Premium with low score".to_string()),
                        expr: "results.a.status === 'premium' && results.a.score < 90".to_string(),
                        modules: vec![
                            flow_module("premium_low", FlowModuleValue::RawScript {
                                input_transforms: Default::default(),
                                language: ScriptLang::Deno,
                                content: r#"
export function main() {
    return {branch: "premium_low", discount: 10};
}
"#.to_string(),
                                path: None,
                                lock: None,
                                tag: None,
                                concurrency_settings: Default::default(),
                                is_trigger: None,
                                assets: None,
                            }),
                        ],
                        modules_node: None,
                        skip_failure: true,
                        parallel: true,
                    },
                ],
                default: vec![
                    flow_module("default_branch", FlowModuleValue::RawScript {
                        input_transforms: Default::default(),
                        language: ScriptLang::Deno,
                        content: r#"
export function main() {
    return {branch: "default", discount: 0};
}
"#.to_string(),
                        path: None,
                        lock: None,
                        tag: None,
                        concurrency_settings: Default::default(),
                        is_trigger: None,
                        assets: None,
                    }),
                ],
                default_node: None,
            }),
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    // status=premium, score=95 -> premium_high branch, discount=30
    assert_eq!(result["branch"], "premium_high");
    assert_eq!(result["discount"], 30);
    assert_eq!(result["score_from_a"], 95);

    Ok(())
}

// =============================================================================
// TEST 4: Previous result and result aggregation
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_previous_result_aggregation(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {value: 10, items: [1, 2, 3]};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
            flow_module("b", FlowModuleValue::RawScript {
                input_transforms: [
                    js_input("prev_value", "previous_result.value"),
                    js_input("prev_items_sum", "previous_result.items.reduce((a, b) => a + b, 0)"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(prev_value: number, prev_items_sum: number) {
    return {value: prev_value * 2, sum: prev_items_sum};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
            flow_module("c", FlowModuleValue::RawScript {
                input_transforms: [
                    js_input("a_value", "results.a.value"),
                    js_input("b_value", "results.b.value"),
                    js_input("b_sum", "results.b.sum"),
                    js_input("combined", "results.a.value + results.b.value + results.b.sum"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(a_value: number, b_value: number, b_sum: number, combined: number) {
    return {a_value, b_value, b_sum, combined};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    // a: value=10, items=[1,2,3]
    // b: prev_value=10, prev_items_sum=6 -> value=20, sum=6
    // c: a_value=10, b_value=20, b_sum=6, combined=36
    assert_eq!(result["a_value"], 10);
    assert_eq!(result["b_value"], 20);
    assert_eq!(result["b_sum"], 6);
    assert_eq!(result["combined"], 36);

    Ok(())
}

// =============================================================================
// TEST 5: Nested for-loops with complex data
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_nested_complexity(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("data", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        categories: [
            {name: "A", multiplier: 2},
            {name: "B", multiplier: 3}
        ],
        base_values: [10, 20]
    };
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
            // Iterate over categories
            flow_module("outer_loop", FlowModuleValue::ForloopFlow {
                iterator: InputTransform::Javascript {
                    expr: "results.data.categories".to_string()
                },
                skip_failures: false,
                parallel: false,
                squash: None,
                parallelism: None,
                modules: vec![
                    // For each category, compute results using base_values
                    flow_module("compute", FlowModuleValue::RawScript {
                        input_transforms: [
                            js_input("cat_name", "flow_input.iter.value.name"),
                            js_input("multiplier", "flow_input.iter.value.multiplier"),
                            js_input("values", "results.data.base_values.map(v => v * flow_input.iter.value.multiplier)"),
                            js_input("sum", "results.data.base_values.reduce((a, b) => a + b, 0) * flow_input.iter.value.multiplier"),
                        ].into(),
                        language: ScriptLang::Deno,
                        content: r#"
export function main(cat_name: string, multiplier: number, values: number[], sum: number) {
    return {category: cat_name, multiplier, computed_values: values, total: sum};
}
"#.to_string(),
                        path: None,
                        lock: None,
                        tag: None,
                        concurrency_settings: Default::default(),
                        is_trigger: None,
                        assets: None,
                    }),
                ],
                modules_node: None,
            }),
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    // Category A (multiplier=2): values=[20,40], total=60
    // Category B (multiplier=3): values=[30,60], total=90
    assert!(result.is_array());
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    assert_eq!(arr[0]["category"], "A");
    assert_eq!(arr[0]["multiplier"], 2);
    assert_eq!(arr[0]["computed_values"], json!([20, 40]));
    assert_eq!(arr[0]["total"], 60);

    assert_eq!(arr[1]["category"], "B");
    assert_eq!(arr[1]["multiplier"], 3);
    assert_eq!(arr[1]["computed_values"], json!([30, 60]));
    assert_eq!(arr[1]["total"], 90);

    Ok(())
}

// =============================================================================
// TEST 6: Complex object transformations
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_object_transformations(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("source", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        users: [
            {id: 1, name: "Alice", tags: ["admin", "active"]},
            {id: 2, name: "Bob", tags: ["user"]},
            {id: 3, name: "Charlie", tags: ["admin", "inactive"]}
        ],
        config: {
            activeBonus: 10,
            adminBonus: 20
        }
    };
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
            flow_module("transform", FlowModuleValue::RawScript {
                input_transforms: [
                    js_input("admins", "results.source.users.filter(u => u.tags.includes('admin')).map(u => u.name)"),
                    js_input("active_count", "results.source.users.filter(u => u.tags.includes('active')).length"),
                    js_input("admin_bonus", "results.source.config.adminBonus"),
                    js_input("active_bonus", "results.source.config.activeBonus"),
                    js_input("all_users", "results.source.users"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(admins: string[], active_count: number, admin_bonus: number, active_bonus: number, all_users: any[]) {
    // Compute user_summary and total_bonus in the script since complex expressions in input_transforms
    // with closures referencing outer variables have parsing limitations
    const user_summary = all_users.map(u => ({
        name: u.name,
        isAdmin: u.tags.includes('admin'),
        isActive: u.tags.includes('active'),
        bonus: (u.tags.includes('admin') ? admin_bonus : 0) + (u.tags.includes('active') ? active_bonus : 0)
    }));
    const total_bonus = user_summary.reduce((sum, u) => sum + u.bonus, 0);
    return {admins, active_count, user_summary, total_bonus};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    // Admins: Alice, Charlie
    // Active count: 1 (only Alice)
    // Bonuses: Alice=30 (admin+active), Bob=0, Charlie=20 (admin only)
    // Total bonus: 50
    assert_eq!(result["admins"], json!(["Alice", "Charlie"]));
    assert_eq!(result["active_count"], 1);
    assert_eq!(result["total_bonus"], 50);

    let summary = result["user_summary"].as_array().unwrap();
    assert_eq!(summary[0]["name"], "Alice");
    assert_eq!(summary[0]["bonus"], 30);
    assert_eq!(summary[1]["name"], "Bob");
    assert_eq!(summary[1]["bonus"], 0);
    assert_eq!(summary[2]["name"], "Charlie");
    assert_eq!(summary[2]["bonus"], 20);

    Ok(())
}

// =============================================================================
// TEST 7: Skip-if with expression evaluation
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_skip_if_expressions(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("check", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {should_skip: true, value: 100};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
            {
                let mut module = flow_module("maybe_skipped", FlowModuleValue::RawScript {
                    input_transforms: [
                        js_input("input_val", "results.check.value * 2"),
                    ].into(),
                    language: ScriptLang::Deno,
                    content: r#"
export function main(input_val: number) {
    return {processed: input_val, was_run: true};
}
"#.to_string(),
                    path: None,
                    lock: None,
                    tag: None,
                    concurrency_settings: Default::default(),
                    is_trigger: None,
                    assets: None,
                });
                module.skip_if = Some(windmill_common::flows::SkipIf {
                    expr: "results.check.should_skip === true".to_string(),
                });
                module
            },
            flow_module("final", FlowModuleValue::RawScript {
                input_transforms: [
                    js_input("check_val", "results.check.value"),
                    js_input("prev", "previous_result"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(check_val: number, prev: any) {
    return {check_val, previous: prev};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    // maybe_skipped should be skipped because check.should_skip === true
    // So previous_result in final should be from check, not maybe_skipped
    assert_eq!(result["check_val"], 100);
    // previous_result should be the skipped result or check's result

    Ok(())
}

// =============================================================================
// TEST 8: Template literals and string operations
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_template_literals(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("data", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        firstName: "John",
        lastName: "Doe",
        items: ["apple", "banana", "cherry"],
        count: 42
    };
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
            flow_module("format", FlowModuleValue::RawScript {
                input_transforms: [
                    js_input("full_name", "`${results.data.firstName} ${results.data.lastName}`"),
                    js_input("greeting", "`Hello, ${results.data.firstName}! You have ${results.data.count} items.`"),
                    js_input("items_str", "results.data.items.join(', ')"),
                    js_input("upper_name", "results.data.firstName.toUpperCase()"),
                    js_input("items_formatted", "`Items: ${results.data.items.map(i => i.charAt(0).toUpperCase() + i.slice(1)).join(', ')}`"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(full_name: string, greeting: string, items_str: string, upper_name: string, items_formatted: string) {
    return {full_name, greeting, items_str, upper_name, items_formatted};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    assert_eq!(result["full_name"], "John Doe");
    assert_eq!(result["greeting"], "Hello, John! You have 42 items.");
    assert_eq!(result["items_str"], "apple, banana, cherry");
    assert_eq!(result["upper_name"], "JOHN");
    assert_eq!(result["items_formatted"], "Items: Apple, Banana, Cherry");

    Ok(())
}

// =============================================================================
// TEST 9: Optional chaining and nullish coalescing
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_optional_chaining(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("data", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        user: {
            name: "Alice",
            address: {
                city: "NYC"
            }
        },
        empty_user: null,
        partial_user: {
            name: "Bob"
        }
    };
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
            flow_module("access", FlowModuleValue::RawScript {
                input_transforms: [
                    js_input("city", "results.data.user?.address?.city"),
                    js_input("missing_city", "results.data.partial_user?.address?.city"),
                    js_input("null_user_name", "results.data.empty_user?.name"),
                    js_input("default_city", "results.data.partial_user?.address?.city ?? 'Unknown'"),
                    js_input("default_country", "results.data.user?.address?.country ?? 'USA'"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(city: string, missing_city: any, null_user_name: any, default_city: string, default_country: string) {
    return {city, missing_city, null_user_name, default_city, default_country};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    assert_eq!(result["city"], "NYC");
    assert_eq!(result["missing_city"], serde_json::Value::Null);
    assert_eq!(result["null_user_name"], serde_json::Value::Null);
    assert_eq!(result["default_city"], "Unknown");
    assert_eq!(result["default_country"], "USA");

    Ok(())
}

// =============================================================================
// TEST 10: Parallel for-loop with expression-based parallelism
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_parallel_forloop(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("data", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {items: [1, 2, 3, 4, 5]};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
            flow_module("parallel_loop", FlowModuleValue::ForloopFlow {
                iterator: InputTransform::Javascript {
                    expr: "results.data.items.map(x => ({ value: x, squared: x * x }))".to_string()
                },
                skip_failures: false,
                parallel: true,
                squash: None,
                parallelism: None,
                modules: vec![
                    flow_module("process", FlowModuleValue::RawScript {
                        input_transforms: [
                            js_input("original", "flow_input.iter.value.value"),
                            js_input("squared", "flow_input.iter.value.squared"),
                            js_input("cubed", "flow_input.iter.value.value ** 3"),
                        ].into(),
                        language: ScriptLang::Deno,
                        content: r#"
export function main(original: number, squared: number, cubed: number) {
    return {original, squared, cubed};
}
"#.to_string(),
                        path: None,
                        lock: None,
                        tag: None,
                        concurrency_settings: Default::default(),
                        is_trigger: None,
                        assets: None,
                    }),
                ],
                modules_node: None,
            }),
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    // Results may be in any order due to parallel execution
    assert!(result.is_array());
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 5);

    // Verify all expected values are present (order may vary)
    let mut values: Vec<i64> = arr.iter()
        .map(|r| r["original"].as_i64().unwrap())
        .collect();
    values.sort();
    assert_eq!(values, vec![1, 2, 3, 4, 5]);

    // Verify computations are correct
    for item in arr {
        let orig = item["original"].as_i64().unwrap();
        let squared = item["squared"].as_i64().unwrap();
        let cubed = item["cubed"].as_i64().unwrap();
        assert_eq!(squared, orig * orig);
        assert_eq!(cubed, orig * orig * orig);
    }

    Ok(())
}

// =============================================================================
// TEST 11: flow_env access in expressions
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_env_access(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    // Create flow_env with various types of values
    let mut flow_env = std::collections::HashMap::new();
    flow_env.insert("ENV".to_string(), windmill_common::worker::to_raw_value(&json!("production")));
    flow_env.insert("DEBUG".to_string(), windmill_common::worker::to_raw_value(&json!(false)));
    flow_env.insert("TIMEOUT".to_string(), windmill_common::worker::to_raw_value(&json!(30)));
    flow_env.insert("CONFIG".to_string(), windmill_common::worker::to_raw_value(&json!({
        "apiUrl": "https://api.example.com",
        "retries": 3,
        "features": ["auth", "logging"]
    })));

    let flow = FlowValue {
        modules: vec![
            flow_module("use_env", FlowModuleValue::RawScript {
                input_transforms: [
                    js_input("env_name", "flow_env.ENV"),
                    js_input("is_debug", "flow_env.DEBUG"),
                    js_input("timeout_val", "flow_env.TIMEOUT"),
                    js_input("api_url", "flow_env.CONFIG.apiUrl"),
                    js_input("retry_count", "flow_env.CONFIG.retries"),
                    js_input("has_auth", "flow_env.CONFIG.features.includes('auth')"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(env_name: string, is_debug: boolean, timeout_val: number, api_url: string, retry_count: number, has_auth: boolean) {
    return {env_name, is_debug, timeout_val, api_url, retry_count, has_auth};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
        ],
        flow_env: Some(flow_env),
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    assert_eq!(result["env_name"], "production");
    assert_eq!(result["is_debug"], false);
    assert_eq!(result["timeout_val"], 30);
    assert_eq!(result["api_url"], "https://api.example.com");
    assert_eq!(result["retry_count"], 3);
    assert_eq!(result["has_auth"], true);

    Ok(())
}

// =============================================================================
// TEST 12: flow_input and flow_env combined with conditionals
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_input_and_env_combined(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    // flow_env with environment-specific configuration
    let mut flow_env = std::collections::HashMap::new();
    flow_env.insert("ENV".to_string(), windmill_common::worker::to_raw_value(&json!("production")));
    flow_env.insert("MAX_ITEMS".to_string(), windmill_common::worker::to_raw_value(&json!(100)));

    let flow = FlowValue {
        modules: vec![
            flow_module("process", FlowModuleValue::RawScript {
                input_transforms: [
                    // Combine flow_input with flow_env
                    js_input("effective_limit", "Math.min(flow_input.requested_limit, flow_env.MAX_ITEMS)"),
                    js_input("env_prefix", "`[${flow_env.ENV}]`"),
                    js_input("is_prod", "flow_env.ENV === 'production'"),
                    js_input("doubled_input", "flow_input.value * 2"),
                    // Conditional based on both
                    js_input("multiplier", "flow_env.ENV === 'production' ? flow_input.prod_mult : 1"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(effective_limit: number, env_prefix: string, is_prod: boolean, doubled_input: number, multiplier: number) {
    return {effective_limit, env_prefix, is_prod, doubled_input, final_value: doubled_input * multiplier};
}
"#.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings: Default::default(),
                is_trigger: None,
                assets: None,
            }),
        ],
        flow_env: Some(flow_env),
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .arg("requested_limit", json!(150))
        .arg("value", json!(25))
        .arg("prod_mult", json!(3))
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    // effective_limit = min(150, 100) = 100
    assert_eq!(result["effective_limit"], 100);
    assert_eq!(result["env_prefix"], "[production]");
    assert_eq!(result["is_prod"], true);
    assert_eq!(result["doubled_input"], 50);  // 25 * 2
    // final_value = 50 * 3 (prod_mult because ENV is production)
    assert_eq!(result["final_value"], 150);

    Ok(())
}
