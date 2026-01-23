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
        let deno_result = eval_timeout(
            expr.to_string(),
            transform_context.clone(),
            flow_input.clone(),
            None,
            None,
            None,
            None,
        )
        .await?;

        let quickjs_result = eval_timeout_quickjs(
            expr.to_string(),
            transform_context,
            flow_input,
            None,
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
