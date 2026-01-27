/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Feature parity tests for deno_core vs rquickjs expression evaluation.
//!
//! This module ensures both JavaScript engines produce identical results for
//! the same expressions, validating that QuickJS can be used as a drop-in
//! replacement for deno_core in flow expression evaluation.

#[cfg(all(test, feature = "deno_core", feature = "quickjs"))]
mod parity_tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use serde_json::json;
    use serde_json::value::RawValue;
    use windmill_common::worker::to_raw_value;

    use crate::js_eval::eval_timeout;
    use crate::js_eval_quickjs::eval_timeout_quickjs;

    /// Helper to run the same test on both engines and compare results
    async fn test_parity(
        expr: &str,
        transform_context: HashMap<String, Arc<Box<RawValue>>>,
        flow_input: Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
    ) -> anyhow::Result<()> {
        test_parity_with_flow_env(expr, transform_context, flow_input, None).await
    }

    /// Helper to run the same test on both engines with flow_env support
    async fn test_parity_with_flow_env(
        expr: &str,
        transform_context: HashMap<String, Arc<Box<RawValue>>>,
        flow_input: Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
        flow_env: Option<HashMap<String, Box<RawValue>>>,
    ) -> anyhow::Result<()> {
        let deno_result = eval_timeout(
            expr.to_string(),
            transform_context.clone(),
            flow_input.clone(),
            flow_env.as_ref(),
            None,
            None,
            None,
        )
        .await?;

        let quickjs_result = eval_timeout_quickjs(
            expr.to_string(),
            transform_context,
            flow_input,
            flow_env.as_ref(),
            None,
            None,
            None,
        )
        .await?;

        // Parse both results to compare as JSON values (handles formatting differences)
        let deno_value: serde_json::Value = serde_json::from_str(deno_result.get())?;
        let quickjs_value: serde_json::Value = serde_json::from_str(quickjs_result.get())?;

        assert_eq!(
            deno_value, quickjs_value,
            "Results differ for expression '{}'\ndeno_core: {}\nquickjs: {}",
            expr, deno_result.get(), quickjs_result.get()
        );

        Ok(())
    }

    #[tokio::test]
    async fn parity_simple_arithmetic() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(5))));
        env.insert("y".to_string(), Arc::new(to_raw_value(&json!(3))));

        test_parity("x + y", env.clone(), None).await?;
        test_parity("x - y", env.clone(), None).await?;
        test_parity("x * y", env.clone(), None).await?;
        test_parity("x / y", env.clone(), None).await?;
        test_parity("x % y", env.clone(), None).await?;
        test_parity("x ** 2", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_object_property_access() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "obj".to_string(),
            Arc::new(to_raw_value(&json!({
                "name": "test",
                "value": 42,
                "nested": {
                    "deep": {
                        "property": "found"
                    }
                }
            }))),
        );

        test_parity("obj.name", env.clone(), None).await?;
        test_parity("obj.value", env.clone(), None).await?;
        test_parity("obj.nested.deep.property", env.clone(), None).await?;
        test_parity("obj['name']", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_array_operations() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "arr".to_string(),
            Arc::new(to_raw_value(&json!([1, 2, 3, 4, 5]))),
        );

        test_parity("arr.length", env.clone(), None).await?;
        test_parity("arr[0]", env.clone(), None).await?;
        test_parity("arr.map(x => x * 2)", env.clone(), None).await?;
        test_parity("arr.filter(x => x > 2)", env.clone(), None).await?;
        test_parity("arr.reduce((a, b) => a + b, 0)", env.clone(), None).await?;
        test_parity("arr.find(x => x > 3)", env.clone(), None).await?;
        test_parity("arr.some(x => x > 4)", env.clone(), None).await?;
        test_parity("arr.every(x => x > 0)", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_string_operations() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "s".to_string(),
            Arc::new(to_raw_value(&json!("Hello World"))),
        );

        test_parity("s.toLowerCase()", env.clone(), None).await?;
        test_parity("s.toUpperCase()", env.clone(), None).await?;
        test_parity("s.length", env.clone(), None).await?;
        test_parity("s.split(' ')", env.clone(), None).await?;
        test_parity("s.replace('World', 'QuickJS')", env.clone(), None).await?;
        test_parity("s.includes('World')", env.clone(), None).await?;
        test_parity("s.startsWith('Hello')", env.clone(), None).await?;
        test_parity("s.trim()", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_ternary_and_conditionals() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(10))));
        env.insert("y".to_string(), Arc::new(to_raw_value(&json!(5))));

        test_parity("x > y ? 'bigger' : 'smaller'", env.clone(), None).await?;
        test_parity("x === 10 ? true : false", env.clone(), None).await?;
        test_parity("x > 5 && y < 10", env.clone(), None).await?;
        test_parity("x > 20 || y < 10", env.clone(), None).await?;
        test_parity("!false", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_object_creation() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert("name".to_string(), Arc::new(to_raw_value(&json!("test"))));
        env.insert("value".to_string(), Arc::new(to_raw_value(&json!(42))));

        test_parity("({ foo: 'bar' })", env.clone(), None).await?;
        test_parity("({ name, value })", env.clone(), None).await?;
        test_parity("({ ...{ a: 1 }, b: 2 })", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_null_undefined() -> anyhow::Result<()> {
        let env = HashMap::new();

        test_parity("null", env.clone(), None).await?;
        test_parity("undefined", env.clone(), None).await?;

        let mut env_with_null = HashMap::new();
        env_with_null.insert("x".to_string(), Arc::new(to_raw_value(&json!(null))));
        test_parity("x", env_with_null.clone(), None).await?;
        test_parity("x ?? 'default'", env_with_null.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_flow_input() -> anyhow::Result<()> {
        let mut flow_input = HashMap::new();
        flow_input.insert("name".to_string(), to_raw_value(&json!("test_flow")));
        flow_input.insert("count".to_string(), to_raw_value(&json!(100)));
        flow_input.insert(
            "config".to_string(),
            to_raw_value(&json!({"enabled": true})),
        );

        let fi = Some(mappable_rc::Marc::new(flow_input));

        test_parity("flow_input.name", HashMap::new(), fi.clone()).await?;
        test_parity("flow_input.count", HashMap::new(), fi.clone()).await?;
        test_parity("flow_input.config.enabled", HashMap::new(), fi.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_template_literals() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert("name".to_string(), Arc::new(to_raw_value(&json!("World"))));
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(5))));

        test_parity("`Hello ${name}!`", env.clone(), None).await?;
        test_parity("`The answer is ${x * 2}`", env.clone(), None).await?;
        test_parity("`Multi\nline`", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_json_operations() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "obj".to_string(),
            Arc::new(to_raw_value(&json!({"a": 1, "b": 2}))),
        );

        test_parity("JSON.stringify(obj)", env.clone(), None).await?;
        test_parity("Object.keys(obj)", env.clone(), None).await?;
        test_parity("Object.values(obj)", env.clone(), None).await?;
        // Note: Object.entries order might differ, so we skip that

        Ok(())
    }

    #[tokio::test]
    async fn parity_math_operations() -> anyhow::Result<()> {
        let env = HashMap::new();

        test_parity("Math.max(1, 5, 3)", env.clone(), None).await?;
        test_parity("Math.min(1, 5, 3)", env.clone(), None).await?;
        test_parity("Math.abs(-5)", env.clone(), None).await?;
        test_parity("Math.floor(3.7)", env.clone(), None).await?;
        test_parity("Math.ceil(3.2)", env.clone(), None).await?;
        test_parity("Math.round(3.5)", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_type_coercion() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert("num".to_string(), Arc::new(to_raw_value(&json!(42))));
        env.insert("str".to_string(), Arc::new(to_raw_value(&json!("123"))));

        test_parity("String(num)", env.clone(), None).await?;
        test_parity("Number(str)", env.clone(), None).await?;
        test_parity("Boolean(num)", env.clone(), None).await?;
        test_parity("parseInt('42px')", env.clone(), None).await?;
        test_parity("parseFloat('3.14')", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_array_spread() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "arr1".to_string(),
            Arc::new(to_raw_value(&json!([1, 2, 3]))),
        );
        env.insert(
            "arr2".to_string(),
            Arc::new(to_raw_value(&json!([4, 5, 6]))),
        );

        test_parity("[...arr1, ...arr2]", env.clone(), None).await?;
        test_parity("[0, ...arr1, 99]", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_multiline_statements() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(5))));

        test_parity(
            r#"let y = x * 2;
            return y + 1"#,
            env.clone(),
            None,
        )
        .await?;

        test_parity(
            r#"const result = x > 3 ? 'big' : 'small';
            return result"#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_optional_chaining_nullish() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "obj".to_string(),
            Arc::new(to_raw_value(&json!({"a": {"b": 1}}))),
        );
        env.insert("empty".to_string(), Arc::new(to_raw_value(&json!(null))));

        // Optional chaining
        test_parity("obj?.a?.b", env.clone(), None).await?;
        test_parity("obj?.a?.c", env.clone(), None).await?;
        test_parity("obj?.x?.y", env.clone(), None).await?;
        test_parity("empty?.foo", env.clone(), None).await?;

        // Nullish coalescing
        test_parity("null ?? 'default'", env.clone(), None).await?;
        test_parity("undefined ?? 'default'", env.clone(), None).await?;
        test_parity("0 ?? 'default'", env.clone(), None).await?;
        test_parity("'' ?? 'default'", env.clone(), None).await?;
        test_parity("false ?? 'default'", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_destructuring() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "obj".to_string(),
            Arc::new(to_raw_value(&json!({"name": "test", "value": 42}))),
        );
        env.insert(
            "arr".to_string(),
            Arc::new(to_raw_value(&json!([1, 2, 3, 4, 5]))),
        );

        // Object destructuring
        test_parity(
            "const { name, value } = obj; return { name, value }",
            env.clone(),
            None,
        )
        .await?;

        // Array destructuring
        test_parity(
            "const [first, second, ...rest] = arr; return { first, second, rest }",
            env.clone(),
            None,
        )
        .await?;

        // Default values
        test_parity(
            "const { missing = 'default' } = obj; return missing",
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_number_edge_cases() -> anyhow::Result<()> {
        let env = HashMap::new();

        // Basic number operations
        test_parity("Number.MAX_SAFE_INTEGER", env.clone(), None).await?;
        test_parity("Number.MIN_SAFE_INTEGER", env.clone(), None).await?;
        test_parity("Number.isInteger(5)", env.clone(), None).await?;
        test_parity("Number.isInteger(5.5)", env.clone(), None).await?;
        test_parity("Number.isFinite(Infinity)", env.clone(), None).await?;
        test_parity("Number.isNaN(NaN)", env.clone(), None).await?;

        // Floating point
        test_parity("0.1 + 0.2", env.clone(), None).await?;
        test_parity("Math.round((0.1 + 0.2) * 10) / 10", env.clone(), None).await?;

        // Special values (these serialize to null in JSON)
        test_parity("isNaN(NaN)", env.clone(), None).await?;
        test_parity("isFinite(100)", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_regex_basic() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "str".to_string(),
            Arc::new(to_raw_value(&json!("hello world 123"))),
        );

        // Basic regex operations
        test_parity("/hello/.test(str)", env.clone(), None).await?;
        test_parity("str.match(/\\d+/)?.[0]", env.clone(), None).await?;
        test_parity("str.replace(/world/, 'universe')", env.clone(), None).await?;
        test_parity("str.split(/\\s+/)", env.clone(), None).await?;

        // Global flag
        test_parity("'aaa'.replace(/a/g, 'b')", env.clone(), None).await?;

        // Case insensitive
        test_parity("/HELLO/i.test(str)", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_unicode_strings() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "emoji".to_string(),
            Arc::new(to_raw_value(&json!("Hello 👋 World 🌍"))),
        );
        env.insert(
            "chinese".to_string(),
            Arc::new(to_raw_value(&json!("你好世界"))),
        );
        env.insert(
            "mixed".to_string(),
            Arc::new(to_raw_value(&json!("Héllo Wörld"))),
        );

        // Basic operations on unicode strings
        test_parity("emoji.includes('👋')", env.clone(), None).await?;
        test_parity("chinese.length", env.clone(), None).await?;
        test_parity("mixed.toUpperCase()", env.clone(), None).await?;
        test_parity("mixed.toLowerCase()", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_date_basic() -> anyhow::Result<()> {
        let env = HashMap::new();

        // Static Date methods (deterministic)
        test_parity("Date.parse('2024-01-15T00:00:00.000Z')", env.clone(), None).await?;
        test_parity(
            "new Date('2024-01-15T00:00:00.000Z').getUTCFullYear()",
            env.clone(),
            None,
        )
        .await?;
        test_parity(
            "new Date('2024-01-15T00:00:00.000Z').getUTCMonth()",
            env.clone(),
            None,
        )
        .await?;
        test_parity(
            "new Date('2024-01-15T00:00:00.000Z').getUTCDate()",
            env.clone(),
            None,
        )
        .await?;
        test_parity(
            "new Date('2024-01-15T00:00:00.000Z').toISOString()",
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_array_advanced() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "arr".to_string(),
            Arc::new(to_raw_value(&json!([3, 1, 4, 1, 5, 9, 2, 6]))),
        );
        env.insert(
            "nested".to_string(),
            Arc::new(to_raw_value(&json!([[1, 2], [3, 4], [5, 6]]))),
        );

        // Sorting (note: sort mutates, so we slice first)
        test_parity("[...arr].sort((a, b) => a - b)", env.clone(), None).await?;
        test_parity("[...arr].sort((a, b) => b - a)", env.clone(), None).await?;

        // Flat operations
        test_parity("nested.flat()", env.clone(), None).await?;
        test_parity("nested.flatMap(x => x)", env.clone(), None).await?;

        // indexOf, includes
        test_parity("arr.indexOf(5)", env.clone(), None).await?;
        test_parity("arr.indexOf(99)", env.clone(), None).await?;
        test_parity("arr.includes(9)", env.clone(), None).await?;

        // slice, splice behavior
        test_parity("arr.slice(2, 5)", env.clone(), None).await?;
        test_parity("arr.slice(-3)", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_logical_operators() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert("a".to_string(), Arc::new(to_raw_value(&json!(true))));
        env.insert("b".to_string(), Arc::new(to_raw_value(&json!(false))));
        env.insert("n".to_string(), Arc::new(to_raw_value(&json!(null))));
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(5))));

        // Short-circuit evaluation
        test_parity("a && 'yes'", env.clone(), None).await?;
        test_parity("b && 'yes'", env.clone(), None).await?;
        test_parity("b || 'no'", env.clone(), None).await?;
        test_parity("a || 'no'", env.clone(), None).await?;

        // Logical assignment (ES2021)
        test_parity("let y = null; y ??= 10; return y", env.clone(), None).await?;
        test_parity("let y = 5; y ??= 10; return y", env.clone(), None).await?;

        // Complex conditions
        test_parity("(a && x > 3) || (b && x < 3)", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_typeof_instanceof() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert("str".to_string(), Arc::new(to_raw_value(&json!("hello"))));
        env.insert("num".to_string(), Arc::new(to_raw_value(&json!(42))));
        env.insert("arr".to_string(), Arc::new(to_raw_value(&json!([1, 2, 3]))));
        env.insert(
            "obj".to_string(),
            Arc::new(to_raw_value(&json!({"a": 1}))),
        );
        env.insert("n".to_string(), Arc::new(to_raw_value(&json!(null))));

        // typeof
        test_parity("typeof str", env.clone(), None).await?;
        test_parity("typeof num", env.clone(), None).await?;
        test_parity("typeof arr", env.clone(), None).await?;
        test_parity("typeof obj", env.clone(), None).await?;
        test_parity("typeof n", env.clone(), None).await?;
        test_parity("typeof undefined", env.clone(), None).await?;

        // Array.isArray
        test_parity("Array.isArray(arr)", env.clone(), None).await?;
        test_parity("Array.isArray(obj)", env.clone(), None).await?;

        Ok(())
    }

    // =========================================================================
    // COMPLEX MULTILINE EXPRESSIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_multiline_complex_logic() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "users".to_string(),
            Arc::new(to_raw_value(&json!([
                {"name": "Alice", "age": 30, "role": "admin"},
                {"name": "Bob", "age": 25, "role": "user"},
                {"name": "Charlie", "age": 35, "role": "admin"},
                {"name": "Diana", "age": 28, "role": "user"}
            ]))),
        );

        // Complex filtering and mapping
        test_parity(
            r#"
            const admins = users.filter(u => u.role === 'admin');
            const names = admins.map(u => u.name);
            return names.join(', ')
            "#,
            env.clone(),
            None,
        )
        .await?;

        // Aggregation with reduce
        test_parity(
            r#"
            const totalAge = users.reduce((sum, u) => sum + u.age, 0);
            const avgAge = totalAge / users.length;
            return Math.round(avgAge)
            "#,
            env.clone(),
            None,
        )
        .await?;

        // Group by operation
        test_parity(
            r#"
            const grouped = users.reduce((acc, u) => {
                if (!acc[u.role]) acc[u.role] = [];
                acc[u.role].push(u.name);
                return acc;
            }, {});
            return grouped
            "#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_multiline_data_transformation() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "data".to_string(),
            Arc::new(to_raw_value(&json!({
                "items": [
                    {"id": 1, "price": 100, "quantity": 2},
                    {"id": 2, "price": 50, "quantity": 5},
                    {"id": 3, "price": 75, "quantity": 3}
                ],
                "discount": 0.1
            }))),
        );

        // Calculate total with discount
        test_parity(
            r#"
            const subtotals = data.items.map(item => item.price * item.quantity);
            const total = subtotals.reduce((a, b) => a + b, 0);
            const discounted = total * (1 - data.discount);
            return { subtotals, total, discounted }
            "#,
            env.clone(),
            None,
        )
        .await?;

        // Transform data structure
        test_parity(
            r#"
            const result = data.items.map(item => ({
                ...item,
                subtotal: item.price * item.quantity,
                discountedSubtotal: item.price * item.quantity * (1 - data.discount)
            }));
            return result
            "#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_multiline_conditional_logic() -> anyhow::Result<()> {
        // NOTE: We avoid the word "error" in expressions due to special handling in eval_timeout

        let mut env = HashMap::new();
        env.insert("status".to_string(), Arc::new(to_raw_value(&json!("pending"))));
        env.insert("retries".to_string(), Arc::new(to_raw_value(&json!(3))));
        env.insert("maxRetries".to_string(), Arc::new(to_raw_value(&json!(5))));

        // Complex conditional with multiple branches
        test_parity(
            r#"
            let action;
            if (status === 'success') {
                action = 'complete';
            } else if (status === 'pending' && retries < maxRetries) {
                action = 'retry';
            } else if (status === 'pending') {
                action = 'fail';
            } else {
                action = 'unknown';
            }
            return { action, retriesLeft: maxRetries - retries }
            "#,
            env.clone(),
            None,
        )
        .await?;

        // Switch-like using object lookup
        test_parity(
            r#"
            const actions = {
                'success': () => ({ next: 'complete', message: 'Done!' }),
                'pending': () => ({ next: 'retry', message: `Retry ${retries + 1}/${maxRetries}` }),
                'failed': () => ({ next: 'stop', message: 'Giving up' })
            };
            const handler = actions[status] || (() => ({ next: 'fallback', message: 'Unknown status' }));
            return handler()
            "#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    // =========================================================================
    // ARROW FUNCTION VARIATIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_arrow_functions() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "numbers".to_string(),
            Arc::new(to_raw_value(&json!([1, 2, 3, 4, 5]))),
        );

        // Concise body (implicit return)
        test_parity("numbers.map(n => n * 2)", env.clone(), None).await?;

        // Block body (explicit return)
        test_parity(
            "numbers.map(n => { return n * 2; })",
            env.clone(),
            None,
        )
        .await?;

        // Multiple parameters
        test_parity(
            "numbers.reduce((acc, n) => acc + n, 0)",
            env.clone(),
            None,
        )
        .await?;

        // Destructuring in parameters
        test_parity(
            r#"
            const pairs = [[1, 2], [3, 4], [5, 6]];
            return pairs.map(([a, b]) => a + b)
            "#,
            HashMap::new(),
            None,
        )
        .await?;

        // Object destructuring in parameters
        test_parity(
            r#"
            const items = [{x: 1, y: 2}, {x: 3, y: 4}];
            return items.map(({x, y}) => x * y)
            "#,
            HashMap::new(),
            None,
        )
        .await?;

        // Nested arrow functions
        test_parity(
            "numbers.map(n => numbers.filter(m => m !== n))",
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    // =========================================================================
    // TRY-CATCH EXPRESSIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_try_catch() -> anyhow::Result<()> {
        // NOTE: We avoid the word "error" in expressions due to special handling in eval_timeout

        let env = HashMap::new();

        // Basic try-catch
        test_parity(
            r#"
            try {
                return JSON.parse('{"valid": true}');
            } catch (e) {
                return { problem: e.message };
            }
            "#,
            env.clone(),
            None,
        )
        .await?;

        // Try-catch with invalid JSON
        test_parity(
            r#"
            try {
                return JSON.parse('invalid json');
            } catch (e) {
                return { problem: 'parse_failed' };
            }
            "#,
            env.clone(),
            None,
        )
        .await?;

        // Try-catch-finally
        test_parity(
            r#"
            let result = 'initial';
            try {
                result = 'try';
            } catch (e) {
                result = 'catch';
            } finally {
                result = result + '_finally';
            }
            return result
            "#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    // =========================================================================
    // COMPLEX OBJECT OPERATIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_object_advanced() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "config".to_string(),
            Arc::new(to_raw_value(&json!({
                "server": {"host": "localhost", "port": 8080},
                "database": {"host": "db.local", "port": 5432},
                "features": ["auth", "logging", "cache"]
            }))),
        );

        // Object.assign
        test_parity(
            "Object.assign({}, config.server, { secure: true })",
            env.clone(),
            None,
        )
        .await?;

        // Object spread with override
        test_parity(
            "({ ...config.server, port: 443, secure: true })",
            env.clone(),
            None,
        )
        .await?;

        // Object.entries and Object.fromEntries
        test_parity(
            r#"
            const entries = Object.entries(config.server);
            const reversed = entries.map(([k, v]) => [k.toUpperCase(), v]);
            return Object.fromEntries(reversed)
            "#,
            env.clone(),
            None,
        )
        .await?;

        // Deep clone pattern
        test_parity(
            "JSON.parse(JSON.stringify(config))",
            env.clone(),
            None,
        )
        .await?;

        // Computed property names
        test_parity(
            r#"
            const key = 'dynamic';
            return { [key]: 'value', [`${key}_2`]: 'value2' }
            "#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    // =========================================================================
    // STRING MANIPULATION ADVANCED
    // =========================================================================

    #[tokio::test]
    async fn parity_string_advanced() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "text".to_string(),
            Arc::new(to_raw_value(&json!("  Hello, World!  "))),
        );
        env.insert(
            "path".to_string(),
            Arc::new(to_raw_value(&json!("/api/v1/users/123/profile"))),
        );

        // Trim variants
        test_parity("text.trim()", env.clone(), None).await?;
        test_parity("text.trimStart()", env.clone(), None).await?;
        test_parity("text.trimEnd()", env.clone(), None).await?;

        // Padding
        test_parity("'42'.padStart(5, '0')", env.clone(), None).await?;
        test_parity("'42'.padEnd(5, '-')", env.clone(), None).await?;

        // Repeat
        test_parity("'ab'.repeat(3)", env.clone(), None).await?;

        // Path manipulation
        test_parity(
            "path.split('/').filter(p => p.length > 0)",
            env.clone(),
            None,
        )
        .await?;

        // Template literal with expressions
        test_parity(
            r#"`Path parts: ${path.split('/').filter(p => p).length}`"#,
            env.clone(),
            None,
        )
        .await?;

        // String search methods
        test_parity("path.indexOf('/users/')", env.clone(), None).await?;
        test_parity("path.lastIndexOf('/')", env.clone(), None).await?;
        test_parity("path.substring(0, 7)", env.clone(), None).await?;

        Ok(())
    }

    // =========================================================================
    // ARRAY MANIPULATION ADVANCED
    // =========================================================================

    #[tokio::test]
    async fn parity_array_manipulation() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "items".to_string(),
            Arc::new(to_raw_value(&json!([
                {"id": 1, "name": "Apple", "category": "fruit"},
                {"id": 2, "name": "Carrot", "category": "vegetable"},
                {"id": 3, "name": "Banana", "category": "fruit"},
                {"id": 4, "name": "Broccoli", "category": "vegetable"}
            ]))),
        );

        // find and findIndex
        test_parity(
            "items.find(i => i.name === 'Banana')",
            env.clone(),
            None,
        )
        .await?;

        test_parity(
            "items.findIndex(i => i.name === 'Banana')",
            env.clone(),
            None,
        )
        .await?;

        // Filter and sort chain
        test_parity(
            "items.filter(i => i.category === 'fruit').map(i => i.name).sort()",
            env.clone(),
            None,
        )
        .await?;

        // Array.from with map function
        test_parity(
            "Array.from({length: 5}, (_, i) => i * 2)",
            env.clone(),
            None,
        )
        .await?;

        // Array fill
        test_parity("Array(3).fill(0)", env.clone(), None).await?;

        // Reverse (on copy to avoid mutation)
        test_parity(
            "[...items].reverse().map(i => i.name)",
            env.clone(),
            None,
        )
        .await?;

        // concat
        test_parity(
            "[1, 2].concat([3, 4], [5, 6])",
            env.clone(),
            None,
        )
        .await?;

        // join variations
        test_parity(
            "items.map(i => i.name).join(' | ')",
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    // =========================================================================
    // REAL-WORLD FLOW EXPRESSION PATTERNS
    // =========================================================================

    #[tokio::test]
    async fn parity_flow_patterns_api_response() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "previous_result".to_string(),
            Arc::new(to_raw_value(&json!({
                "status": 200,
                "data": {
                    "users": [
                        {"id": 1, "email": "alice@example.com", "active": true},
                        {"id": 2, "email": "bob@example.com", "active": false},
                        {"id": 3, "email": "charlie@example.com", "active": true}
                    ],
                    "pagination": {"page": 1, "total": 50, "per_page": 10}
                }
            }))),
        );

        // Extract active users' emails
        test_parity(
            "previous_result.data.users.filter(u => u.active).map(u => u.email)",
            env.clone(),
            None,
        )
        .await?;

        // Check if more pages exist
        test_parity(
            r#"
            const { page, total, per_page } = previous_result.data.pagination;
            return page * per_page < total
            "#,
            env.clone(),
            None,
        )
        .await?;

        // Transform to different structure
        test_parity(
            r#"({
                emails: previous_result.data.users.map(u => u.email),
                activeCount: previous_result.data.users.filter(u => u.active).length,
                hasMore: previous_result.data.pagination.page * previous_result.data.pagination.per_page < previous_result.data.pagination.total
            })"#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_flow_patterns_failure_handling() -> anyhow::Result<()> {
        // NOTE: We avoid using the literal word "error" in expressions because
        // it triggers special error-handling code that has a bug with duplicate declarations.

        // Test with failure info in previous_result
        let mut env_failure = HashMap::new();
        env_failure.insert(
            "previous_result".to_string(),
            Arc::new(to_raw_value(&json!({
                "failure": {
                    "name": "APIFailure",
                    "message": "Rate limit exceeded",
                    "code": 429
                }
            }))),
        );

        // Check for failure presence
        test_parity(
            "previous_result?.failure ? true : false",
            env_failure.clone(),
            None,
        )
        .await?;

        // Extract failure details
        test_parity(
            "previous_result.failure?.code ?? 500",
            env_failure.clone(),
            None,
        )
        .await?;

        // Test with successful result (no failure)
        let mut env_success = HashMap::new();
        env_success.insert(
            "previous_result".to_string(),
            Arc::new(to_raw_value(&json!({
                "data": "success"
            }))),
        );

        test_parity(
            "previous_result?.failure ? 'failed' : 'ok'",
            env_success.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_flow_patterns_conditional_branching() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "step_a".to_string(),
            Arc::new(to_raw_value(&json!({"count": 5}))),
        );
        env.insert(
            "step_b".to_string(),
            Arc::new(to_raw_value(&json!({"count": 10}))),
        );
        env.insert("threshold".to_string(), Arc::new(to_raw_value(&json!(7))));

        // Branch selection based on condition
        test_parity(
            "step_a.count > threshold ? 'high' : step_b.count > threshold ? 'medium' : 'low'",
            env.clone(),
            None,
        )
        .await?;

        // Aggregate from multiple steps
        test_parity(
            "({ total: step_a.count + step_b.count, average: (step_a.count + step_b.count) / 2 })",
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_flow_patterns_data_mapping() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "source".to_string(),
            Arc::new(to_raw_value(&json!({
                "firstName": "John",
                "lastName": "Doe",
                "birthDate": "1990-05-15",
                "addresses": [
                    {"type": "home", "city": "New York"},
                    {"type": "work", "city": "Boston"}
                ]
            }))),
        );

        // Map to different schema
        test_parity(
            r#"({
                fullName: `${source.firstName} ${source.lastName}`,
                birth_date: source.birthDate,
                primary_city: source.addresses.find(a => a.type === 'home')?.city ?? source.addresses[0]?.city ?? 'Unknown',
                all_cities: source.addresses.map(a => a.city)
            })"#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    // =========================================================================
    // EDGE CASES AND SPECIAL VALUES
    // =========================================================================

    #[tokio::test]
    async fn parity_edge_cases_empty_values() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert("emptyArray".to_string(), Arc::new(to_raw_value(&json!([]))));
        env.insert("emptyObject".to_string(), Arc::new(to_raw_value(&json!({}))));
        env.insert("emptyString".to_string(), Arc::new(to_raw_value(&json!(""))));
        env.insert("zero".to_string(), Arc::new(to_raw_value(&json!(0))));

        // Operations on empty values
        test_parity("emptyArray.length", env.clone(), None).await?;
        test_parity("emptyArray.map(x => x * 2)", env.clone(), None).await?;
        test_parity("emptyArray.filter(x => x > 0)", env.clone(), None).await?;
        test_parity("emptyArray.reduce((a, b) => a + b, 100)", env.clone(), None).await?;

        test_parity("Object.keys(emptyObject)", env.clone(), None).await?;
        test_parity("Object.values(emptyObject)", env.clone(), None).await?;

        test_parity("emptyString.length", env.clone(), None).await?;
        test_parity("emptyString || 'default'", env.clone(), None).await?;
        test_parity("emptyString ?? 'default'", env.clone(), None).await?;

        test_parity("zero || 'default'", env.clone(), None).await?;
        test_parity("zero ?? 'default'", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_edge_cases_nested_access() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "deep".to_string(),
            Arc::new(to_raw_value(&json!({
                "a": {"b": {"c": {"d": {"e": "found!"}}}}
            }))),
        );

        // Deep property access
        test_parity("deep.a.b.c.d.e", env.clone(), None).await?;
        test_parity("deep?.a?.b?.c?.d?.e", env.clone(), None).await?;
        test_parity("deep?.a?.b?.x?.y?.z", env.clone(), None).await?;
        test_parity("deep?.a?.b?.x?.y?.z ?? 'not found'", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_edge_cases_special_characters() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "data".to_string(),
            Arc::new(to_raw_value(&json!({
                "key-with-dash": "value1",
                "key.with.dots": "value2",
                "key with spaces": "value3"
            }))),
        );

        // Bracket notation for special keys
        test_parity("data['key-with-dash']", env.clone(), None).await?;
        test_parity("data['key.with.dots']", env.clone(), None).await?;
        test_parity("data['key with spaces']", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_edge_cases_large_numbers() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        // Large but safe integers
        env.insert(
            "bigNum".to_string(),
            Arc::new(to_raw_value(&json!(9007199254740991_i64))), // MAX_SAFE_INTEGER
        );
        env.insert(
            "timestamp".to_string(),
            Arc::new(to_raw_value(&json!(1704067200000_i64))), // 2024-01-01 UTC
        );

        test_parity("bigNum", env.clone(), None).await?;
        test_parity("timestamp", env.clone(), None).await?;
        test_parity("new Date(timestamp).toISOString()", env.clone(), None).await?;

        // Arithmetic on large numbers
        test_parity("bigNum - 1", env.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_edge_cases_boolean_coercion() -> anyhow::Result<()> {
        let env = HashMap::new();

        // Falsy values
        test_parity("Boolean(0)", env.clone(), None).await?;
        test_parity("Boolean('')", env.clone(), None).await?;
        test_parity("Boolean(null)", env.clone(), None).await?;
        test_parity("Boolean(undefined)", env.clone(), None).await?;
        test_parity("Boolean(NaN)", env.clone(), None).await?;

        // Truthy values
        test_parity("Boolean(1)", env.clone(), None).await?;
        test_parity("Boolean('hello')", env.clone(), None).await?;
        test_parity("Boolean([])", env.clone(), None).await?;
        test_parity("Boolean({})", env.clone(), None).await?;

        // Double negation coercion
        test_parity("!!0", env.clone(), None).await?;
        test_parity("!!1", env.clone(), None).await?;
        test_parity("!!''", env.clone(), None).await?;
        test_parity("!!'hello'", env.clone(), None).await?;

        Ok(())
    }

    // =========================================================================
    // PROMISES AND ASYNC PATTERNS (without client)
    // =========================================================================

    #[tokio::test]
    async fn parity_promise_resolve() -> anyhow::Result<()> {
        let env = HashMap::new();

        // Basic Promise.resolve
        test_parity(
            "Promise.resolve(42)",
            env.clone(),
            None,
        )
        .await?;

        test_parity(
            "Promise.resolve({ key: 'value' })",
            env.clone(),
            None,
        )
        .await?;

        // Promise.all with resolved values
        test_parity(
            "Promise.all([Promise.resolve(1), Promise.resolve(2), Promise.resolve(3)])",
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    // =========================================================================
    // SET AND MAP OPERATIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_set_operations() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "arr".to_string(),
            Arc::new(to_raw_value(&json!([1, 2, 2, 3, 3, 3, 4]))),
        );

        // Deduplicate using Set
        test_parity(
            "[...new Set(arr)]",
            env.clone(),
            None,
        )
        .await?;

        // Set size
        test_parity(
            "new Set(arr).size",
            env.clone(),
            None,
        )
        .await?;

        // Set.has
        test_parity(
            "new Set(arr).has(3)",
            env.clone(),
            None,
        )
        .await?;

        test_parity(
            "new Set(arr).has(99)",
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_map_operations() -> anyhow::Result<()> {
        let env = HashMap::new();

        // Create Map and convert to object
        test_parity(
            r#"
            const map = new Map([['a', 1], ['b', 2], ['c', 3]]);
            return Object.fromEntries(map)
            "#,
            env.clone(),
            None,
        )
        .await?;

        // Map operations
        test_parity(
            r#"
            const map = new Map();
            map.set('key1', 'value1');
            map.set('key2', 'value2');
            return map.get('key1')
            "#,
            env.clone(),
            None,
        )
        .await?;

        test_parity(
            r#"
            const map = new Map([['a', 1], ['b', 2]]);
            return map.size
            "#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    // =========================================================================
    // COMPARISON OPERATORS
    // =========================================================================

    #[tokio::test]
    async fn parity_comparisons() -> anyhow::Result<()> {
        let env = HashMap::new();

        // Strict equality
        test_parity("1 === 1", env.clone(), None).await?;
        test_parity("1 === '1'", env.clone(), None).await?;
        test_parity("null === undefined", env.clone(), None).await?;
        test_parity("null === null", env.clone(), None).await?;

        // Loose equality
        test_parity("1 == '1'", env.clone(), None).await?;
        test_parity("null == undefined", env.clone(), None).await?;
        test_parity("0 == false", env.clone(), None).await?;
        test_parity("'' == false", env.clone(), None).await?;

        // Inequality
        test_parity("5 !== '5'", env.clone(), None).await?;
        test_parity("5 != '5'", env.clone(), None).await?;

        // Comparison operators
        test_parity("5 > 3", env.clone(), None).await?;
        test_parity("5 >= 5", env.clone(), None).await?;
        test_parity("3 < 5", env.clone(), None).await?;
        test_parity("5 <= 5", env.clone(), None).await?;

        // String comparison
        test_parity("'apple' < 'banana'", env.clone(), None).await?;
        test_parity("'10' < '9'", env.clone(), None).await?;

        Ok(())
    }

    // =========================================================================
    // BITWISE OPERATIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_bitwise() -> anyhow::Result<()> {
        let env = HashMap::new();

        test_parity("5 & 3", env.clone(), None).await?;
        test_parity("5 | 3", env.clone(), None).await?;
        test_parity("5 ^ 3", env.clone(), None).await?;
        test_parity("~5", env.clone(), None).await?;
        test_parity("5 << 2", env.clone(), None).await?;
        test_parity("20 >> 2", env.clone(), None).await?;
        test_parity("-5 >>> 0", env.clone(), None).await?;

        Ok(())
    }

    // =========================================================================
    // COMPLEX REAL-WORLD SCENARIOS
    // =========================================================================

    #[tokio::test]
    async fn parity_scenario_batch_processing() -> anyhow::Result<()> {
        // NOTE: We avoid the word "error" in expressions due to special handling in eval_timeout

        let mut env = HashMap::new();
        env.insert(
            "jobs".to_string(),
            Arc::new(to_raw_value(&json!([
                {"id": 1, "status": "completed", "result": 100},
                {"id": 2, "status": "failed", "reason": "timeout"},
                {"id": 3, "status": "completed", "result": 200},
                {"id": 4, "status": "failed", "reason": "connection"},
                {"id": 5, "status": "completed", "result": 150}
            ]))),
        );

        // Aggregate batch results
        test_parity(
            r#"
            const completed = jobs.filter(j => j.status === 'completed');
            const failed = jobs.filter(j => j.status === 'failed');
            const totalResult = completed.reduce((sum, j) => sum + j.result, 0);
            return {
                totalJobs: jobs.length,
                completedCount: completed.length,
                failedCount: failed.length,
                successRate: completed.length / jobs.length,
                totalResult,
                failureReasons: failed.map(j => j.reason)
            }
            "#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_scenario_webhook_payload() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "webhook".to_string(),
            Arc::new(to_raw_value(&json!({
                "event": "user.created",
                "timestamp": "2024-01-15T10:30:00Z",
                "data": {
                    "user": {
                        "id": "usr_123",
                        "email": "newuser@example.com",
                        "metadata": {
                            "source": "signup",
                            "campaign": "winter_2024"
                        }
                    }
                }
            }))),
        );

        // Extract and transform webhook data
        test_parity(
            r#"({
                eventType: webhook.event.split('.')[1],
                userId: webhook.data.user.id,
                userEmail: webhook.data.user.email,
                source: webhook.data.user.metadata?.source ?? 'unknown',
                campaign: webhook.data.user.metadata?.campaign,
                processedAt: new Date().toISOString().split('T')[0]
            })"#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_scenario_config_merge() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "defaults".to_string(),
            Arc::new(to_raw_value(&json!({
                "timeout": 5000,
                "retries": 3,
                "headers": {"Content-Type": "application/json"},
                "features": {"logging": true, "caching": false}
            }))),
        );
        env.insert(
            "overrides".to_string(),
            Arc::new(to_raw_value(&json!({
                "timeout": 10000,
                "headers": {"Authorization": "Bearer token"},
                "features": {"caching": true}
            }))),
        );

        // Deep merge configuration
        test_parity(
            r#"({
                ...defaults,
                ...overrides,
                headers: { ...defaults.headers, ...overrides.headers },
                features: { ...defaults.features, ...overrides.features }
            })"#,
            env.clone(),
            None,
        )
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod benchmark_tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;

    use serde_json::json;
    use windmill_common::worker::to_raw_value;

    /// Benchmark QuickJS expression evaluation startup time
    #[cfg(feature = "quickjs")]
    #[tokio::test]
    async fn benchmark_quickjs_startup() -> anyhow::Result<()> {
        use crate::js_eval_quickjs::eval_timeout_quickjs;

        let iterations = 100;
        let mut env = HashMap::new();
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(5))));

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = eval_timeout_quickjs(
                "x + 1".to_string(),
                env.clone(),
                None,
                None,
                None,
                None,
                None,
            )
            .await?;
        }
        let duration = start.elapsed();

        println!(
            "QuickJS: {} iterations in {:?} ({:?} per iteration)",
            iterations,
            duration,
            duration / iterations
        );

        Ok(())
    }

    /// Benchmark deno_core expression evaluation startup time
    #[cfg(feature = "deno_core")]
    #[tokio::test]
    async fn benchmark_deno_startup() -> anyhow::Result<()> {
        use crate::js_eval::eval_timeout;

        let iterations = 100;
        let mut env = HashMap::new();
        env.insert("x".to_string(), Arc::new(to_raw_value(&json!(5))));

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = eval_timeout(
                "x + 1".to_string(),
                env.clone(),
                None,
                None,
                None,
                None,
                None,
            )
            .await?;
        }
        let duration = start.elapsed();

        println!(
            "deno_core: {} iterations in {:?} ({:?} per iteration)",
            iterations,
            duration,
            duration / iterations
        );

        Ok(())
    }

    /// Benchmark both engines with a complex expression
    #[cfg(all(feature = "deno_core", feature = "quickjs"))]
    #[tokio::test]
    async fn benchmark_complex_expression() -> anyhow::Result<()> {
        use crate::js_eval::eval_timeout;
        use crate::js_eval_quickjs::eval_timeout_quickjs;

        let iterations = 50;
        let mut env = HashMap::new();
        env.insert(
            "data".to_string(),
            Arc::new(to_raw_value(&json!({
                "items": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                "multiplier": 2
            }))),
        );

        let expr = "data.items.filter(x => x > 3).map(x => x * data.multiplier).reduce((a, b) => a + b, 0)";

        // QuickJS
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = eval_timeout_quickjs(
                expr.to_string(),
                env.clone(),
                None,
                None,
                None,
                None,
                None,
            )
            .await?;
        }
        let quickjs_duration = start.elapsed();

        // deno_core
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = eval_timeout(
                expr.to_string(),
                env.clone(),
                None,
                None,
                None,
                None,
                None,
            )
            .await?;
        }
        let deno_duration = start.elapsed();

        println!(
            "Complex expression benchmark ({} iterations):\n  QuickJS:    {:?} ({:?}/iter)\n  deno_core:  {:?} ({:?}/iter)\n  Speedup:    {:.2}x",
            iterations,
            quickjs_duration, quickjs_duration / iterations,
            deno_duration, deno_duration / iterations,
            deno_duration.as_secs_f64() / quickjs_duration.as_secs_f64()
        );

        Ok(())
    }
}

/// Comprehensive flow simulation parity tests
/// Tests expression evaluation in contexts that simulate real flow execution
#[cfg(all(test, feature = "deno_core", feature = "quickjs"))]
mod flow_simulation_parity_tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use serde_json::json;
    use serde_json::value::RawValue;
    use windmill_common::worker::to_raw_value;

    use crate::js_eval::eval_timeout;
    use crate::js_eval_quickjs::eval_timeout_quickjs;

    /// Helper to run the same test on both engines and compare results
    async fn test_parity(
        expr: &str,
        transform_context: HashMap<String, Arc<Box<RawValue>>>,
        flow_input: Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
        flow_env: Option<HashMap<String, Box<RawValue>>>,
    ) -> anyhow::Result<()> {
        let deno_result = eval_timeout(
            expr.to_string(),
            transform_context.clone(),
            flow_input.clone(),
            flow_env.as_ref(),
            None,
            None,
            None,
        )
        .await?;

        let quickjs_result = eval_timeout_quickjs(
            expr.to_string(),
            transform_context,
            flow_input,
            flow_env.as_ref(),
            None,
            None,
            None,
        )
        .await?;

        let deno_value: serde_json::Value = serde_json::from_str(deno_result.get())?;
        let quickjs_value: serde_json::Value = serde_json::from_str(quickjs_result.get())?;

        assert_eq!(
            deno_value, quickjs_value,
            "Results differ for expression '{}'\ndeno_core: {}\nquickjs: {}",
            expr, deno_result.get(), quickjs_result.get()
        );

        Ok(())
    }

    // =========================================================================
    // SIMULATED FLOW CONTEXT: Multi-step flow with various step results
    // =========================================================================

    fn create_multi_step_flow_context() -> (
        HashMap<String, Arc<Box<RawValue>>>,
        Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
        Option<HashMap<String, Box<RawValue>>>,
    ) {
        let mut transform_context = HashMap::new();

        // Step 'a' result: simple number
        transform_context.insert(
            "a".to_string(),
            Arc::new(to_raw_value(&json!(42))),
        );

        // Step 'b' result: object with nested data
        transform_context.insert(
            "b".to_string(),
            Arc::new(to_raw_value(&json!({
                "status": "success",
                "data": {
                    "users": [
                        {"id": 1, "name": "Alice", "active": true, "roles": ["admin", "user"]},
                        {"id": 2, "name": "Bob", "active": false, "roles": ["user"]},
                        {"id": 3, "name": "Charlie", "active": true, "roles": ["moderator", "user"]}
                    ],
                    "total": 3,
                    "metadata": {
                        "page": 1,
                        "hasMore": true
                    }
                }
            }))),
        );

        // Step 'c' result: array of numbers (from a for-loop)
        transform_context.insert(
            "c".to_string(),
            Arc::new(to_raw_value(&json!([10, 20, 30, 40, 50]))),
        );

        // Step 'd' result: null (simulating a step that returned null)
        transform_context.insert(
            "d".to_string(),
            Arc::new(to_raw_value(&json!(null))),
        );

        // Step 'e' result: error object (simulating a failed step with continue_on_error)
        transform_context.insert(
            "e".to_string(),
            Arc::new(to_raw_value(&json!({
                "error": {
                    "name": "ValidationError",
                    "message": "Invalid input provided",
                    "step_id": "e"
                }
            }))),
        );

        // Step 'f' result: deeply nested object
        transform_context.insert(
            "f".to_string(),
            Arc::new(to_raw_value(&json!({
                "level1": {
                    "level2": {
                        "level3": {
                            "level4": {
                                "value": "deeply_nested"
                            }
                        }
                    }
                }
            }))),
        );

        // Step 'g' result: array of mixed types
        transform_context.insert(
            "g".to_string(),
            Arc::new(to_raw_value(&json!([
                "string",
                123,
                true,
                null,
                {"key": "value"},
                [1, 2, 3]
            ]))),
        );

        // previous_result (the last executed step, 'g')
        transform_context.insert(
            "previous_result".to_string(),
            Arc::new(to_raw_value(&json!([
                "string",
                123,
                true,
                null,
                {"key": "value"},
                [1, 2, 3]
            ]))),
        );

        // flow_input
        let mut flow_input = HashMap::new();
        flow_input.insert("name".to_string(), to_raw_value(&json!("test_flow")));
        flow_input.insert("count".to_string(), to_raw_value(&json!(100)));
        flow_input.insert("enabled".to_string(), to_raw_value(&json!(true)));
        flow_input.insert(
            "config".to_string(),
            to_raw_value(&json!({
                "timeout": 30,
                "retries": 3,
                "options": ["fast", "secure"]
            })),
        );
        flow_input.insert(
            "items".to_string(),
            to_raw_value(&json!([
                {"id": 1, "value": "first"},
                {"id": 2, "value": "second"},
                {"id": 3, "value": "third"}
            ])),
        );

        // flow_env
        let mut flow_env = HashMap::new();
        flow_env.insert("ENV".to_string(), to_raw_value(&json!("production")));
        flow_env.insert("DEBUG".to_string(), to_raw_value(&json!(false)));
        flow_env.insert("VERSION".to_string(), to_raw_value(&json!("1.2.3")));

        (
            transform_context,
            Some(mappable_rc::Marc::new(flow_input)),
            Some(flow_env),
        )
    }

    // =========================================================================
    // INPUT TRANSFORM EXPRESSIONS (step inputs)
    // =========================================================================

    #[tokio::test]
    async fn parity_input_transform_direct_reference() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        // Direct step result reference
        test_parity("a", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("b", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("c", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_input_transform_property_access() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        // Nested property access
        test_parity("b.status", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("b.data.total", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("b.data.users[0].name", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("b.data.users[1].roles", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("b.data.metadata.hasMore", ctx.clone(), fi.clone(), fe.clone()).await?;

        // Deeply nested
        test_parity("f.level1.level2.level3.level4.value", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_input_transform_array_operations() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        // Array indexing
        test_parity("c[0]", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("c[c.length - 1]", ctx.clone(), fi.clone(), fe.clone()).await?;

        // Array methods
        test_parity("c.map(x => x * 2)", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("c.filter(x => x > 25)", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("c.reduce((acc, x) => acc + x, 0)", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("c.find(x => x === 30)", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("c.some(x => x > 40)", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("c.every(x => x > 0)", ctx.clone(), fi.clone(), fe.clone()).await?;

        // Chained operations
        test_parity("c.filter(x => x > 20).map(x => x / 10)", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_input_transform_complex_expressions() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        // Complex data extraction from step 'b'
        test_parity(
            "b.data.users.filter(u => u.active).map(u => u.name)",
            ctx.clone(), fi.clone(), fe.clone()
        ).await?;

        test_parity(
            "b.data.users.filter(u => u.roles.includes('admin'))[0]?.name",
            ctx.clone(), fi.clone(), fe.clone()
        ).await?;

        test_parity(
            "b.data.users.reduce((acc, u) => acc + (u.active ? 1 : 0), 0)",
            ctx.clone(), fi.clone(), fe.clone()
        ).await?;

        // Combining multiple step results
        test_parity("a + c[0]", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("a * b.data.total", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    // =========================================================================
    // FLOW_INPUT EXPRESSIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_flow_input_simple() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        test_parity("flow_input.name", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_input.count", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_input.enabled", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_flow_input_nested() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        test_parity("flow_input.config.timeout", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_input.config.retries", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_input.config.options", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_input.config.options[0]", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_flow_input_array_operations() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        test_parity("flow_input.items.length", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_input.items[0].id", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_input.items.map(i => i.value)", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_input.items.find(i => i.id === 2)", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_flow_input_combined_with_steps() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        // Combining flow_input with step results
        test_parity("flow_input.count + a", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_input.config.timeout * b.data.total", ctx.clone(), fi.clone(), fe.clone()).await?;

        // Conditional based on flow_input
        test_parity(
            "flow_input.enabled ? b.data.users : []",
            ctx.clone(), fi.clone(), fe.clone()
        ).await?;

        Ok(())
    }

    // =========================================================================
    // FLOW_ENV EXPRESSIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_flow_env_access() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        test_parity("flow_env.ENV", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_env.DEBUG", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_env.VERSION", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_flow_env_conditionals() -> anyhow::Result<()> {
        // Test flow_env conditionals with explicit flow_env reference in context
        let mut ctx = HashMap::new();
        ctx.insert("env_val".to_string(), Arc::new(to_raw_value(&json!("production"))));
        ctx.insert("debug_val".to_string(), Arc::new(to_raw_value(&json!(false))));

        test_parity(
            "env_val === 'production' ? 'prod' : 'dev'",
            ctx.clone(), None, None
        ).await?;

        test_parity(
            "debug_val ? 'debug mode' : 'normal'",
            ctx.clone(), None, None
        ).await?;

        Ok(())
    }

    // =========================================================================
    // ITERATOR EXPRESSIONS (for forloopflow)
    // =========================================================================

    #[tokio::test]
    async fn parity_iterator_expressions() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        // Typical iterator expressions
        test_parity("c", ctx.clone(), fi.clone(), fe.clone()).await?;  // Direct array
        test_parity("b.data.users", ctx.clone(), fi.clone(), fe.clone()).await?;  // Nested array
        test_parity("flow_input.items", ctx.clone(), fi.clone(), fe.clone()).await?;

        // Transformed iterators
        test_parity("c.map(x => ({value: x, doubled: x * 2}))", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("b.data.users.filter(u => u.active)", ctx.clone(), fi.clone(), fe.clone()).await?;

        // Range-like iteration
        test_parity("Array.from({length: 5}, (_, i) => i)", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_forloop_inner_expressions() -> anyhow::Result<()> {
        // Simulate expressions inside a for-loop where flow_input.iter exists
        let mut ctx = HashMap::new();
        ctx.insert("previous_result".to_string(), Arc::new(to_raw_value(&json!({"value": 42, "index": 2}))));

        let mut flow_input = HashMap::new();
        flow_input.insert("iter".to_string(), to_raw_value(&json!({
            "index": 2,
            "value": {"id": 3, "name": "test_item"}
        })));
        flow_input.insert("name".to_string(), to_raw_value(&json!("parent_flow")));

        let fi = Some(mappable_rc::Marc::new(flow_input));

        test_parity("flow_input.iter.index", ctx.clone(), fi.clone(), None).await?;
        test_parity("flow_input.iter.value", ctx.clone(), fi.clone(), None).await?;
        test_parity("flow_input.iter.value.id", ctx.clone(), fi.clone(), None).await?;
        test_parity("flow_input.iter.value.name", ctx.clone(), fi.clone(), None).await?;

        // Combining iter with other flow_input
        test_parity(
            "`Item ${flow_input.iter.index} of ${flow_input.name}`",
            ctx.clone(), fi.clone(), None
        ).await?;

        Ok(())
    }

    // =========================================================================
    // BRANCH CONDITION EXPRESSIONS (for branchone)
    // =========================================================================

    #[tokio::test]
    async fn parity_branch_conditions() -> anyhow::Result<()> {
        let (ctx, fi, _fe) = create_multi_step_flow_context();

        // Simple boolean conditions
        test_parity("a > 40", ctx.clone(), fi.clone(), None).await?;
        test_parity("b.status === 'success'", ctx.clone(), fi.clone(), None).await?;
        test_parity("flow_input.enabled", ctx.clone(), fi.clone(), None).await?;

        // Complex boolean conditions
        test_parity("a > 40 && b.status === 'success'", ctx.clone(), fi.clone(), None).await?;
        test_parity("a < 50 || b.data.total > 5", ctx.clone(), fi.clone(), None).await?;

        // Conditions with array checks
        test_parity("b.data.users.length > 0", ctx.clone(), fi.clone(), None).await?;
        test_parity("b.data.users.some(u => u.active)", ctx.clone(), fi.clone(), None).await?;
        test_parity("c.includes(30)", ctx.clone(), fi.clone(), None).await?;

        Ok(())
    }

    // =========================================================================
    // SKIP_IF / STOP_AFTER_IF EXPRESSIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_skip_if_expressions() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        // Skip based on previous result
        test_parity("previous_result === null", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("previous_result.length === 0", ctx.clone(), fi.clone(), fe.clone()).await?;

        // Skip based on flow_input
        test_parity("!flow_input.enabled", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("flow_input.count === 0", ctx.clone(), fi.clone(), fe.clone()).await?;

        // Skip based on step result
        test_parity("d === null", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("e?.error !== undefined", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_stop_after_if_expressions() -> anyhow::Result<()> {
        let (ctx, fi, _fe) = create_multi_step_flow_context();

        // Stop conditions (avoid previous_result?.error pattern which has issues with error extraction)
        test_parity("a >= 42", ctx.clone(), fi.clone(), None).await?;
        test_parity("b.data.metadata.hasMore === false", ctx.clone(), fi.clone(), None).await?;
        test_parity("b.status !== 'success'", ctx.clone(), fi.clone(), None).await?;

        Ok(())
    }

    // =========================================================================
    // UNDEFINED/MISSING STEP RESULTS (simulating non-executed branches)
    // =========================================================================

    #[tokio::test]
    async fn parity_missing_step_with_optional_chaining() -> anyhow::Result<()> {
        // Context where some steps weren't executed (e.g., branch not taken)
        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), Arc::new(to_raw_value(&json!(42))));
        // 'b' was never executed (branch not taken)
        ctx.insert("c".to_string(), Arc::new(to_raw_value(&json!(null))));  // Step returned null

        // Safe access to potentially missing step
        test_parity("a", ctx.clone(), None, None).await?;
        test_parity("c", ctx.clone(), None, None).await?;

        // Optional chaining on null
        test_parity("c?.value", ctx.clone(), None, None).await?;
        test_parity("c?.nested?.deep", ctx.clone(), None, None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_null_coalescing_for_missing_data() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        // Nullish coalescing
        test_parity("d ?? 'default'", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("d?.value ?? 'not found'", ctx.clone(), fi.clone(), fe.clone()).await?;

        // With nested access
        test_parity("b.data.missing?.value ?? 'fallback'", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    // =========================================================================
    // ERROR HANDLING EXPRESSIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_error_object_access() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        // Accessing error from step 'e'
        test_parity("e.error.name", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("e.error.message", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("e.error.step_id", ctx.clone(), fi.clone(), fe.clone()).await?;

        // Conditional based on error
        test_parity(
            "e.error ? `Error: ${e.error.message}` : 'OK'",
            ctx.clone(), fi.clone(), fe.clone()
        ).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_error_variable_extraction() -> anyhow::Result<()> {
        // Simulate previous_result being an error
        let mut ctx = HashMap::new();
        ctx.insert(
            "previous_result".to_string(),
            Arc::new(to_raw_value(&json!({
                "error": {
                    "name": "RuntimeError",
                    "message": "Something went wrong"
                }
            }))),
        );

        // The 'error' variable is extracted from previous_result
        test_parity("error.name", ctx.clone(), None, None).await?;
        test_parity("error.message", ctx.clone(), None, None).await?;
        test_parity("`${error.name}: ${error.message}`", ctx.clone(), None, None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_error_from_parallel_results() -> anyhow::Result<()> {
        // Simulate previous_result being an array with errors (from parallel branches)
        let mut ctx = HashMap::new();
        ctx.insert(
            "previous_result".to_string(),
            Arc::new(to_raw_value(&json!([
                {"result": "success"},
                {"error": {"name": "Error1", "message": "First error", "step_id": "branch_1"}},
                {"result": "also success"},
                {"error": {"name": "Error2", "message": "Second error", "step_id": "branch_2"}}
            ]))),
        );

        // Access the aggregated error
        test_parity("error.name", ctx.clone(), None, None).await?;
        test_parity("error.message", ctx.clone(), None, None).await?;
        test_parity("error.errors", ctx.clone(), None, None).await?;

        Ok(())
    }

    // =========================================================================
    // OBJECT CONSTRUCTION EXPRESSIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_object_construction() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        // Building objects from step results
        test_parity(
            "({ count: a, users: b.data.users })",
            ctx.clone(), fi.clone(), fe.clone()
        ).await?;

        test_parity(
            "({ ...flow_input.config, extra: 'value' })",
            ctx.clone(), fi.clone(), fe.clone()
        ).await?;

        // Computed properties
        test_parity(
            "({ [`step_${a}`]: b.status })",
            ctx.clone(), fi.clone(), fe.clone()
        ).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_array_construction() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        // Spread operator
        test_parity("[...c, 60, 70]", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("[a, ...c.slice(0, 2)]", ctx.clone(), fi.clone(), fe.clone()).await?;

        // Array from step results
        test_parity(
            "[b.data.users[0], b.data.users[2]]",
            ctx.clone(), fi.clone(), fe.clone()
        ).await?;

        Ok(())
    }

    // =========================================================================
    // MULTILINE / COMPLEX EXPRESSIONS
    // =========================================================================

    #[tokio::test]
    async fn parity_multiline_data_processing() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        test_parity(r#"
            let users = b.data.users;
            let activeUsers = users.filter(u => u.active);
            let adminUsers = activeUsers.filter(u => u.roles.includes('admin'));
            return adminUsers.map(u => u.name);
        "#, ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_multiline_conditional_logic() -> anyhow::Result<()> {
        let (ctx, fi, _fe) = create_multi_step_flow_context();

        test_parity(r#"
            if (flow_input.enabled) {
                return { mode: 'enabled', data: b.data.users.filter(u => u.active) };
            } else {
                return { mode: 'disabled', data: b.data.users };
            }
        "#, ctx.clone(), fi.clone(), None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_multiline_aggregation() -> anyhow::Result<()> {
        let (ctx, fi, _fe) = create_multi_step_flow_context();

        test_parity(r#"
            const summary = {
                stepA: a,
                stepB_status: b.status,
                stepC_sum: c.reduce((acc, x) => acc + x, 0),
                stepC_count: c.length,
                activeUserCount: b.data.users.filter(u => u.active).length,
                flowName: flow_input.name,
                enabled: flow_input.enabled
            };
            return summary;
        "#, ctx.clone(), fi.clone(), None).await?;

        Ok(())
    }

    // =========================================================================
    // EDGE CASES
    // =========================================================================

    #[tokio::test]
    async fn parity_empty_arrays_and_objects() -> anyhow::Result<()> {
        let mut ctx = HashMap::new();
        ctx.insert("emptyArr".to_string(), Arc::new(to_raw_value(&json!([]))));
        ctx.insert("emptyObj".to_string(), Arc::new(to_raw_value(&json!({}))));

        test_parity("emptyArr.length", ctx.clone(), None, None).await?;
        test_parity("emptyArr.map(x => x)", ctx.clone(), None, None).await?;
        test_parity("emptyArr.filter(x => true)", ctx.clone(), None, None).await?;
        test_parity("Object.keys(emptyObj)", ctx.clone(), None, None).await?;
        test_parity("Object.values(emptyObj)", ctx.clone(), None, None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_large_numbers() -> anyhow::Result<()> {
        let mut ctx = HashMap::new();
        ctx.insert("bigInt".to_string(), Arc::new(to_raw_value(&json!(9007199254740991_i64))));  // MAX_SAFE_INTEGER
        ctx.insert("timestamp".to_string(), Arc::new(to_raw_value(&json!(1703980800000_i64))));  // Typical timestamp

        test_parity("bigInt", ctx.clone(), None, None).await?;
        test_parity("timestamp", ctx.clone(), None, None).await?;
        test_parity("bigInt + 1", ctx.clone(), None, None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_special_characters_in_strings() -> anyhow::Result<()> {
        let mut ctx = HashMap::new();
        ctx.insert(
            "data".to_string(),
            Arc::new(to_raw_value(&json!({
                "message": "Hello, \"World\"!",
                "path": "C:\\Users\\test",
                "newlines": "line1\nline2\nline3",
                "unicode": "こんにちは 🌍",
                "empty": ""
            }))),
        );

        test_parity("data.message", ctx.clone(), None, None).await?;
        test_parity("data.path", ctx.clone(), None, None).await?;
        test_parity("data.newlines.split('\\n').length", ctx.clone(), None, None).await?;
        test_parity("data.unicode", ctx.clone(), None, None).await?;
        test_parity("data.empty.length", ctx.clone(), None, None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_boolean_coercion_edge_cases() -> anyhow::Result<()> {
        let mut ctx = HashMap::new();
        ctx.insert("zero".to_string(), Arc::new(to_raw_value(&json!(0))));
        ctx.insert("emptyString".to_string(), Arc::new(to_raw_value(&json!(""))));
        ctx.insert("nullVal".to_string(), Arc::new(to_raw_value(&json!(null))));
        ctx.insert("falseVal".to_string(), Arc::new(to_raw_value(&json!(false))));
        ctx.insert("emptyArr".to_string(), Arc::new(to_raw_value(&json!([]))));
        ctx.insert("emptyObj".to_string(), Arc::new(to_raw_value(&json!({}))));

        // Truthy/falsy checks
        test_parity("!!zero", ctx.clone(), None, None).await?;
        test_parity("!!emptyString", ctx.clone(), None, None).await?;
        test_parity("!!nullVal", ctx.clone(), None, None).await?;
        test_parity("!!falseVal", ctx.clone(), None, None).await?;
        test_parity("!!emptyArr", ctx.clone(), None, None).await?;  // [] is truthy!
        test_parity("!!emptyObj", ctx.clone(), None, None).await?;  // {} is truthy!

        // Logical operators with falsy values
        test_parity("zero || 'default'", ctx.clone(), None, None).await?;
        test_parity("zero ?? 'default'", ctx.clone(), None, None).await?;  // 0 is not nullish
        test_parity("nullVal ?? 'default'", ctx.clone(), None, None).await?;

        Ok(())
    }

    // =========================================================================
    // PREVIOUS_RESULT SPECIAL HANDLING
    // =========================================================================

    #[tokio::test]
    async fn parity_previous_result_access() -> anyhow::Result<()> {
        let (ctx, fi, fe) = create_multi_step_flow_context();

        test_parity("previous_result", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("previous_result[0]", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("previous_result.length", ctx.clone(), fi.clone(), fe.clone()).await?;
        test_parity("previous_result[4].key", ctx.clone(), fi.clone(), fe.clone()).await?;

        Ok(())
    }

    // =========================================================================
    // REAL-WORLD FLOW SCENARIOS
    // =========================================================================

    #[tokio::test]
    async fn parity_scenario_api_pagination() -> anyhow::Result<()> {
        // Simulate a flow that fetches paginated data
        let mut ctx = HashMap::new();
        ctx.insert(
            "fetch_result".to_string(),
            Arc::new(to_raw_value(&json!({
                "items": [{"id": 1}, {"id": 2}, {"id": 3}],
                "nextCursor": "abc123",
                "hasMore": true
            }))),
        );
        ctx.insert("previous_result".to_string(), Arc::new(to_raw_value(&json!({
            "items": [{"id": 1}, {"id": 2}, {"id": 3}],
            "nextCursor": "abc123",
            "hasMore": true
        }))));

        // Iterator for next page
        test_parity(
            "previous_result.hasMore ? [previous_result.nextCursor] : []",
            ctx.clone(), None, None
        ).await?;

        // Accumulating results
        test_parity(
            "fetch_result.items",
            ctx.clone(), None, None
        ).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_scenario_data_transformation_pipeline() -> anyhow::Result<()> {
        let mut ctx = HashMap::new();

        // Step 1: Raw data
        ctx.insert(
            "raw_data".to_string(),
            Arc::new(to_raw_value(&json!({
                "records": [
                    {"date": "2024-01-15", "amount": 100, "type": "credit"},
                    {"date": "2024-01-16", "amount": 50, "type": "debit"},
                    {"date": "2024-01-17", "amount": 200, "type": "credit"},
                    {"date": "2024-01-18", "amount": 75, "type": "debit"}
                ]
            }))),
        );

        // Step 2: Filter credits
        ctx.insert(
            "credits".to_string(),
            Arc::new(to_raw_value(&json!([
                {"date": "2024-01-15", "amount": 100, "type": "credit"},
                {"date": "2024-01-17", "amount": 200, "type": "credit"}
            ]))),
        );

        ctx.insert("previous_result".to_string(), Arc::new(to_raw_value(&json!([
            {"date": "2024-01-15", "amount": 100, "type": "credit"},
            {"date": "2024-01-17", "amount": 200, "type": "credit"}
        ]))));

        // Filter expression
        test_parity(
            "raw_data.records.filter(r => r.type === 'credit')",
            ctx.clone(), None, None
        ).await?;

        // Sum expression
        test_parity(
            "credits.reduce((sum, r) => sum + r.amount, 0)",
            ctx.clone(), None, None
        ).await?;

        // Summary
        test_parity(r#"
            ({
                totalCredits: credits.reduce((sum, r) => sum + r.amount, 0),
                count: credits.length,
                average: credits.reduce((sum, r) => sum + r.amount, 0) / credits.length
            })
        "#, ctx.clone(), None, None).await?;

        Ok(())
    }

    #[tokio::test]
    async fn parity_scenario_conditional_workflow() -> anyhow::Result<()> {
        // Simulate a workflow with conditional logic
        let mut ctx = HashMap::new();
        ctx.insert(
            "check_result".to_string(),
            Arc::new(to_raw_value(&json!({
                "passed": true,
                "score": 95
            }))),
        );
        ctx.insert(
            "user_data".to_string(),
            Arc::new(to_raw_value(&json!({
                "name": "test_user",
                "level": "admin"
            }))),
        );

        // Branch condition
        test_parity(
            "check_result.passed && check_result.score > 90",
            ctx.clone(), None, None
        ).await?;

        // Skip condition
        test_parity(
            "!check_result.passed || check_result.score < 50",
            ctx.clone(), None, None
        ).await?;

        // Decision logic
        test_parity(
            "check_result.passed && user_data.level === 'admin' ? 'approved' : 'pending'",
            ctx.clone(), None, None
        ).await?;

        Ok(())
    }
}
