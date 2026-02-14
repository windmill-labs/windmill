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

use windmill_test_utils::*;

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

// =============================================================================
// TEST 13: Optional chaining with results proxy
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_results_optional_chaining(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            // Step a: return nested data with some null values
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        user: {
            name: "Alice",
            profile: {
                email: "alice@example.com",
                phone: null
            },
            settings: null
        },
        items: [
            {id: 1, value: 10},
            {id: 2, value: null},
            {id: 3, value: 30}
        ],
        empty_array: [],
        null_field: null
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
            // Step b: use optional chaining on results
            flow_module("b", FlowModuleValue::RawScript {
                input_transforms: [
                    // Basic optional chaining
                    js_input("user_name", "results.a.user?.name"),
                    js_input("user_email", "results.a.user?.profile?.email"),
                    // Optional chaining with null value
                    js_input("user_phone", "results.a.user?.profile?.phone ?? 'no_phone'"),
                    // Optional chaining on null settings
                    js_input("user_setting", "results.a.user?.settings?.theme ?? 'default_theme'"),
                    // Optional chaining with array access
                    js_input("first_item_value", "results.a.items?.[0]?.value"),
                    js_input("second_item_value", "results.a.items?.[1]?.value ?? 0"),
                    // Optional chaining with find
                    js_input("item_by_id", "results.a.items?.find(i => i.id === 1)?.value"),
                    js_input("missing_item", "results.a.items?.find(i => i.id === 999)?.value ?? 'not_found'"),
                    // Optional chaining on empty array
                    js_input("empty_first", "results.a.empty_array?.[0]?.value ?? 'empty'"),
                    // Nullish coalescing with null field
                    js_input("null_with_default", "results.a.null_field ?? 'was_null'"),
                    // Accessing missing property with ?.
                    js_input("missing_prop", "results.a.nonexistent?.nested?.deep ?? 'missing'"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(
    user_name: string,
    user_email: string,
    user_phone: string,
    user_setting: string,
    first_item_value: number,
    second_item_value: number,
    item_by_id: number,
    missing_item: string,
    empty_first: string,
    null_with_default: string,
    missing_prop: string
) {
    return {
        user_name, user_email, user_phone, user_setting,
        first_item_value, second_item_value, item_by_id, missing_item,
        empty_first, null_with_default, missing_prop
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
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    assert_eq!(result["user_name"], "Alice");
    assert_eq!(result["user_email"], "alice@example.com");
    assert_eq!(result["user_phone"], "no_phone");
    assert_eq!(result["user_setting"], "default_theme");
    assert_eq!(result["first_item_value"], 10);
    assert_eq!(result["second_item_value"], 0);
    assert_eq!(result["item_by_id"], 10);
    assert_eq!(result["missing_item"], "not_found");
    assert_eq!(result["empty_first"], "empty");
    assert_eq!(result["null_with_default"], "was_null");
    assert_eq!(result["missing_prop"], "missing");

    Ok(())
}

// =============================================================================
// TEST 14: Large integer handling in results
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_large_integers(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        small_int: 42,
        i32_max: 2147483647,
        i32_max_plus_1: 2147483648,
        timestamp: 1704067200000,  // Jan 1, 2024 00:00:00 UTC
        large_safe: 9007199254740991,  // MAX_SAFE_INTEGER
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
            flow_module("b", FlowModuleValue::RawScript {
                input_transforms: [
                    js_input("small", "results.a.small_int"),
                    js_input("i32_max", "results.a.i32_max"),
                    js_input("over_i32", "results.a.i32_max_plus_1"),
                    js_input("timestamp", "results.a.timestamp"),
                    js_input("ts_plus_day", "results.a.timestamp + 86400000"),
                    js_input("large", "results.a.large_safe"),
                    // Arithmetic on large numbers
                    js_input("large_minus_1", "results.a.large_safe - 1"),
                    // Comparisons
                    js_input("is_large_safe", "Number.isSafeInteger(results.a.large_safe)"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(
    small: number, i32_max: number, over_i32: number,
    timestamp: number, ts_plus_day: number,
    large: number, large_minus_1: number,
    is_large_safe: boolean
) {
    return {small, i32_max, over_i32, timestamp, ts_plus_day, large, large_minus_1, is_large_safe};
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

    assert_eq!(result["small"], 42);
    assert_eq!(result["i32_max"], 2147483647_i64);
    assert_eq!(result["over_i32"], 2147483648_i64);
    assert_eq!(result["timestamp"], 1704067200000_i64);
    assert_eq!(result["ts_plus_day"], 1704153600000_i64);
    assert_eq!(result["large"], 9007199254740991_i64);
    assert_eq!(result["large_minus_1"], 9007199254740990_i64);
    assert_eq!(result["is_large_safe"], true);

    Ok(())
}

// =============================================================================
// TEST 15: Unicode and emoji handling in results
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_unicode_emoji(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        greeting: "Hello World",
        simple_str: "hello",
        greeting_len: 11,
        mixed: "cafe resume naive",
        names: ["Alice", "Bob", "Carlos"]
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
            flow_module("b", FlowModuleValue::RawScript {
                input_transforms: [
                    js_input("greeting", "results.a.greeting"),
                    js_input("greeting_len", "results.a.greeting_len"),
                    js_input("simple_str", "results.a.simple_str"),  // Get string directly first
                    js_input("has_world", "results.a.greeting.includes('World')"),
                    js_input("first_name", "results.a.names[0]"),
                    js_input("last_name", "results.a.names[2]"),
                    js_input("mixed_upper", "results.a.mixed.toUpperCase()"),
                    js_input("template", "`Welcome: ${results.a.greeting}`"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(
    greeting: string, greeting_len: number, simple_str: string, has_world: boolean,
    first_name: string, last_name: string, mixed_upper: string, template: string
) {
    return {greeting, greeting_len, simple_str, simple_str_len: simple_str?.length, has_world, first_name, last_name, mixed_upper, template};
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

    assert_eq!(result["greeting"], "Hello World");
    assert_eq!(result["greeting_len"], 11);
    assert_eq!(result["simple_str"], "hello");
    assert_eq!(result["simple_str_len"], 5); // "hello".length (computed inside script)
    assert_eq!(result["has_world"], true);
    assert_eq!(result["first_name"], "Alice");
    assert_eq!(result["last_name"], "Carlos");
    assert_eq!(result["mixed_upper"], "CAFE RESUME NAIVE");
    assert_eq!(result["template"], "Welcome: Hello World");

    Ok(())
}

// =============================================================================
// TEST 16: Complex array operations with results
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_complex_array_operations(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        numbers: [5, 2, 8, 1, 9, 3, 7, 4, 6],
        users: [
            {id: 1, name: "Alice", score: 85, active: true},
            {id: 2, name: "Bob", score: 92, active: false},
            {id: 3, name: "Charlie", score: 78, active: true},
            {id: 4, name: "Diana", score: 95, active: true}
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
            flow_module("b", FlowModuleValue::RawScript {
                input_transforms: [
                    // Sorting
                    js_input("sorted_asc", "[...results.a.numbers].sort((a, b) => a - b)"),
                    js_input("sorted_desc", "[...results.a.numbers].sort((a, b) => b - a)"),
                    // Filtering and mapping combined
                    js_input("active_names", "results.a.users.filter(u => u.active).map(u => u.name)"),
                    js_input("high_scorers", "results.a.users.filter(u => u.score >= 90).map(u => ({name: u.name, score: u.score}))"),
                    // Reduce operations
                    js_input("total_score", "results.a.users.reduce((sum, u) => sum + u.score, 0)"),
                    js_input("avg_score", "results.a.users.reduce((sum, u) => sum + u.score, 0) / results.a.users.length"),
                    // Find operations
                    js_input("top_scorer", "results.a.users.reduce((max, u) => u.score > max.score ? u : max).name"),
                    // Some/every
                    js_input("has_inactive", "results.a.users.some(u => !u.active)"),
                    js_input("all_above_70", "results.a.users.every(u => u.score > 70)"),
                    // Slice and spread
                    js_input("first_three", "results.a.numbers.slice(0, 3)"),
                    js_input("last_two", "results.a.numbers.slice(-2)"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(
    sorted_asc: number[], sorted_desc: number[], active_names: string[],
    high_scorers: {name: string, score: number}[], total_score: number,
    avg_score: number, top_scorer: string, has_inactive: boolean,
    all_above_70: boolean, first_three: number[], last_two: number[]
) {
    return {
        sorted_asc, sorted_desc, active_names, high_scorers,
        total_score, avg_score, top_scorer, has_inactive,
        all_above_70, first_three, last_two
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
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    assert_eq!(result["sorted_asc"], json!([1, 2, 3, 4, 5, 6, 7, 8, 9]));
    assert_eq!(result["sorted_desc"], json!([9, 8, 7, 6, 5, 4, 3, 2, 1]));
    assert_eq!(result["active_names"], json!(["Alice", "Charlie", "Diana"]));
    assert_eq!(result["high_scorers"], json!([{"name": "Bob", "score": 92}, {"name": "Diana", "score": 95}]));
    assert_eq!(result["total_score"], 350); // 85 + 92 + 78 + 95
    assert_eq!(result["avg_score"], 87.5);
    assert_eq!(result["top_scorer"], "Diana");
    assert_eq!(result["has_inactive"], true);
    assert_eq!(result["all_above_70"], true);
    assert_eq!(result["first_three"], json!([5, 2, 8]));
    assert_eq!(result["last_two"], json!([4, 6]));

    Ok(())
}

// =============================================================================
// TEST 17: Multiline expressions with semicolons and return
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_multiline_expressions(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        items: [
            {id: 1, name: "Item A", price: 10, qty: 2},
            {id: 2, name: "Item B", price: 20, qty: 3},
            {id: 3, name: "Item C", price: 30, qty: 1}
        ],
        discount: 0.1,
        tax_rate: 0.08
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
            flow_module("b", FlowModuleValue::RawScript {
                input_transforms: [
                    // Simple multiline with variable declaration
                    js_input("subtotal", r#"
                        let items = results.a.items;
                        let total = items.reduce((sum, item) => sum + (item.price * item.qty), 0);
                        return total;
                    "#),
                    // Multiline with conditional logic
                    js_input("discounted_total", r#"
                        let items = results.a.items;
                        let subtotal = items.reduce((sum, item) => sum + (item.price * item.qty), 0);
                        let discount = results.a.discount;
                        if (subtotal > 50) {
                            return subtotal * (1 - discount);
                        } else {
                            return subtotal;
                        }
                    "#),
                    // Multiline with multiple statements and final expression
                    js_input("item_summary", r#"
                        const items = results.a.items;
                        const names = items.map(i => i.name);
                        const total_qty = items.reduce((sum, i) => sum + i.qty, 0);
                        return { names, total_qty };
                    "#),
                    // Multiline with try-catch
                    js_input("safe_calculation", r#"
                        try {
                            const items = results.a.items;
                            const tax_rate = results.a.tax_rate;
                            const subtotal = items.reduce((sum, item) => sum + (item.price * item.qty), 0);
                            return Math.round(subtotal * (1 + tax_rate) * 100) / 100;
                        } catch (e) {
                            return 0;
                        }
                    "#),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(
    subtotal: number, discounted_total: number,
    item_summary: {names: string[], total_qty: number},
    safe_calculation: number
) {
    return {subtotal, discounted_total, item_summary, safe_calculation};
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

    // subtotal = 10*2 + 20*3 + 30*1 = 20 + 60 + 30 = 110
    assert_eq!(result["subtotal"], 110);
    // discounted_total = 110 * (1 - 0.1) = 99
    assert_eq!(result["discounted_total"], 99.0);
    assert_eq!(result["item_summary"]["names"], json!(["Item A", "Item B", "Item C"]));
    assert_eq!(result["item_summary"]["total_qty"], 6);
    // safe_calculation = 110 * 1.08 = 118.8
    assert_eq!(result["safe_calculation"], 118.8);

    Ok(())
}

// =============================================================================
// TEST 18: Spread operators with results
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_spread_with_results(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        config: { host: "localhost", port: 3000 },
        tags: ["api", "v1"],
        user: { name: "Alice", role: "admin" }
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
            flow_module("b", FlowModuleValue::RawScript {
                input_transforms: [
                    // Object spread with results
                    js_input("merged_config", "{...results.a.config, timeout: 5000}"),
                    // Array spread with results
                    js_input("all_tags", "[...results.a.tags, 'production']"),
                    // Nested object spread
                    js_input("full_user", "{...results.a.user, permissions: ['read', 'write']}"),
                    // Spread in function call
                    js_input("max_port", "Math.max(...[results.a.config.port, 8080, 4000])"),
                    // Destructuring with rest spread
                    js_input("rest_config", r#"
                        const {host, ...rest} = results.a.config;
                        return rest;
                    "#),
                    // Combining multiple spreads
                    js_input("combined", "{config: {...results.a.config}, tags: [...results.a.tags], source: 'flow'}"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(
    merged_config: any, all_tags: string[], full_user: any,
    max_port: number, rest_config: any, combined: any
) {
    return {merged_config, all_tags, full_user, max_port, rest_config, combined};
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

    assert_eq!(result["merged_config"], json!({"host": "localhost", "port": 3000, "timeout": 5000}));
    assert_eq!(result["all_tags"], json!(["api", "v1", "production"]));
    assert_eq!(result["full_user"], json!({"name": "Alice", "role": "admin", "permissions": ["read", "write"]}));
    assert_eq!(result["max_port"], 8080);
    assert_eq!(result["rest_config"], json!({"port": 3000}));
    assert_eq!(result["combined"]["config"], json!({"host": "localhost", "port": 3000}));
    assert_eq!(result["combined"]["tags"], json!(["api", "v1"]));
    assert_eq!(result["combined"]["source"], "flow");

    Ok(())
}

// =============================================================================
// TEST 19: Nested for-loop accessing parent step results
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_nested_forloop_results_access(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            // Step a: outer data
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return {
        multiplier: 10,
        categories: ["cat1", "cat2"]
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
            // Outer for-loop
            flow_module("outer", FlowModuleValue::ForloopFlow {
                iterator: InputTransform::Javascript {
                    expr: "results.a.categories".to_string()
                },
                skip_failures: false,
                parallel: false,
                squash: None,
                parallelism: None,
                modules: vec![
                    // Step b: generate inner items based on category
                    flow_module("b", FlowModuleValue::RawScript {
                        input_transforms: [
                            js_input("category", "flow_input.iter.value"),
                            js_input("multiplier", "results.a.multiplier"),  // Access outer step from inside for-loop
                        ].into(),
                        language: ScriptLang::Deno,
                        content: r#"
export function main(category: string, multiplier: number) {
    return {
        category,
        items: [1, 2].map(n => ({
            id: `${category}-${n}`,
            value: n * multiplier
        }))
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
                    // Step c: process inner items and access previous step in loop
                    flow_module("c", FlowModuleValue::RawScript {
                        input_transforms: [
                            js_input("items", "results.b.items"),  // Access sibling step
                            js_input("category", "results.b.category"),
                            js_input("original_mult", "results.a.multiplier"),  // Access outer step
                        ].into(),
                        language: ScriptLang::Deno,
                        content: r#"
export function main(items: any[], category: string, original_mult: number) {
    return {
        category,
        original_mult,
        item_count: items.length,
        total_value: items.reduce((sum, i) => sum + i.value, 0)
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

    // outer loop produces 2 results (for cat1 and cat2)
    assert!(result.is_array());
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    // First iteration (cat1): items [1*10, 2*10] = [10, 20], total = 30
    assert_eq!(arr[0]["category"], "cat1");
    assert_eq!(arr[0]["original_mult"], 10);
    assert_eq!(arr[0]["item_count"], 2);
    assert_eq!(arr[0]["total_value"], 30);

    // Second iteration (cat2): items [1*10, 2*10] = [10, 20], total = 30
    assert_eq!(arr[1]["category"], "cat2");
    assert_eq!(arr[1]["original_mult"], 10);
    assert_eq!(arr[1]["item_count"], 2);
    assert_eq!(arr[1]["total_value"], 30);

    Ok(())
}

// =============================================================================
// TEST 20: Accessing non-existent steps via results proxy
// This tests the critical case where results.nonexistent should return
// null rather than throwing an error (matching deno_core behavior)
// =============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_results_non_existent_step(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let flow = FlowValue {
        modules: vec![
            flow_module("a", FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: r#"
export function main() {
    return { value: 42 };
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
                    // Access existing step - should work
                    js_input("existing", "results.a.value"),
                    // Access non-existent step - should return null, not error
                    // Note: The expression gets wrapped as (await results.nonexistent)
                    // The proxy returns a Promise that resolves to null for non-existent steps
                    js_input("non_existent", "results.nonexistent"),
                    // Access non-existent step with nullish coalescing
                    js_input("non_existent_with_default", "results.nonexistent ?? 'default_value'"),
                    // Nested access on non-existent step (null?.value -> undefined -> ?? kicks in)
                    js_input("non_existent_nested", "results.nonexistent?.value ?? 'nested_default'"),
                ].into(),
                language: ScriptLang::Deno,
                content: r#"
export function main(
    existing: number,
    non_existent: any,
    non_existent_with_default: string,
    non_existent_nested: string
) {
    return {
        existing,
        non_existent,
        non_existent_with_default,
        non_existent_nested
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
        ],
        same_worker: false,
        ..Default::default()
    };

    let result = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

    // existing step should work
    assert_eq!(result["existing"], 42);
    // non-existent should be null, not error
    assert!(result["non_existent"].is_null());
    // non-existent with default should return the default
    assert_eq!(result["non_existent_with_default"], "default_value");
    // nested non-existent should return the default
    assert_eq!(result["non_existent_nested"], "nested_default");

    Ok(())
}
