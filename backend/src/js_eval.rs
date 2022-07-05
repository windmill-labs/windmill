/*
 * Author & Copyright: Ruben Fiszel 2021
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use deno_core::op;
use deno_core::serde_v8;
use deno_core::v8;
use deno_core::v8::IsolateHandle;
use deno_core::Extension;
use deno_core::JsRuntime;
use deno_core::RuntimeOptions;
use itertools::Itertools;
use regex::Regex;
use serde_json::Value;
use tokio::sync::oneshot;
use tokio::time::timeout;

use crate::client;
use crate::error::Error;

pub async fn eval_timeout(
    expr: String,
    env: Vec<(String, serde_json::Value)>,
    workspace: &str,
    token: &str,
    steps: Vec<String>,
) -> anyhow::Result<serde_json::Value> {
    let expr2 = expr.clone();
    let (sender, mut receiver) = oneshot::channel::<IsolateHandle>();
    let (workspace, token) = (workspace.to_string().clone(), token.to_string().clone());
    timeout(
        std::time::Duration::from_millis(2000),
        tokio::task::spawn_blocking(move || {

            let mut ops = vec![
                // An op for summing an array of numbers
                // The op-layer automatically deserializes inputs
                // and serializes the returned Result & value
                op_variable::decl(),
                op_resource::decl(),
            ];

            if !steps.is_empty() {
                ops.push(op_get_result::decl())
            }

            let ext = Extension::builder().ops(ops).build();
            // Use our snapshot to provision our new runtime
            let options = RuntimeOptions {
                extensions: vec![ext],
                //                startup_snapshot: Some(Snapshot::Static(buffer)),
                ..Default::default()
            };

            let mut js_runtime = JsRuntime::new(options);

            sender
                .send(js_runtime.v8_isolate().thread_safe_handle())
                .map_err(|_| Error::ExecutionErr("impossible to send v8 isolate".to_string()))?;

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;

            let re = Regex::new(r"import (.*)\n").unwrap();
            let expr = re.replace_all(&expr, "").to_string();
            // pretty frail but this it to make the expr more user friendly and not require the user to write await
            let expr = ["variable", "step"]
                .into_iter()
                .fold(expr, replace_with_await);

            let r =
                runtime.block_on(eval(&mut js_runtime, &expr, env, &workspace, &token, steps))?;

            Ok(r) as anyhow::Result<Value>
        }),
    )
    .await
    .map_err(|_| {
        if let Ok(isolate) = receiver.try_recv() {
            isolate.terminate_execution();
        };
        Error::ExecutionErr(format!(
            "The expression of evaluation `{expr2}` took too long to execute (>2000ms)"
        ))
    })??
}

fn replace_with_await(expr: String, fn_name: &str) -> String {
    let sep = format!("{}(", fn_name);
    let mut split = expr.split(&sep);
    let mut s = split.next().unwrap_or_else(|| "").to_string();
    for x in split {
        s.push_str(&format!("(await {}({}", fn_name, add_closing_bracket(x)))
    }
    s
}

fn add_closing_bracket(s: &str) -> String {
    let mut s = s.to_string();
    let mut level = 1;
    let mut idx = 0;
    for c in s.chars() {
        match c {
            '(' => level += 1,
            ')' => level -= 1,
            _ => (),
        };
        if level == 0 {
            break;
        }
        idx += 1;
    }
    s.insert_str(idx, ")");
    s
}

const SPLIT_PAT: &str = ";\n";
async fn eval(
    context: &mut JsRuntime,
    expr: &str,
    env: Vec<(String, serde_json::Value)>,
    workspace: &str,
    token: &str,
    steps: Vec<String>,
) -> anyhow::Result<serde_json::Value> {
    let expr = expr.trim();
    let expr = format!(
        "{}\nreturn {};",
        expr.split(SPLIT_PAT)
            .take(expr.split(SPLIT_PAT).count() - 1)
            .join("\n"),
        expr.split(SPLIT_PAT).last().unwrap_or_else(|| "")
    );
    let steps_code = if !steps.is_empty() {
        format!(
            r#"
let steps = [{}];
async function step(n) {{
    if (n == -1) {{
        return previous_result;
    }}
    let token = "{token}";
    if (n < 0) {{
        let steps_length = steps.length;
        n = n % steps.length + steps.length;
    }}
    let id = steps[n];
    return await Deno.core.opAsync("op_get_result", [workspace, id, token, base_url]);
}}"#,
            steps.into_iter().map(|x| format!("\"{x}\"")).join(",")
        )
    } else {
        "".to_string()
    };

    let code = format!(
        r#"
let workspace = "{workspace}";
let base_url = "{}";
async function variable(path) {{
    let token = "{token}";
    return await Deno.core.opAsync("op_variable", [workspace, path, token, base_url]);
}}
async function resource(path) {{
    let token = "{token}";
    return await Deno.core.opAsync("op_resource", [workspace, path, token, base_url]);
}}
{}
{steps_code}
(async () => {{ 
    {expr} 
}})()
        "#,
        std::env::var("BASE_INTERNAL_URL")
            .unwrap_or_else(|_| "http://missing-base-url".to_string()),
        env.into_iter()
            .map(|(a, b)| format!(
                "let {a} = {};\n",
                serde_json::to_string(&b)
                    .unwrap_or_else(|_| "\"error serializing value\"".to_string())
            ))
            .join(""),
    );
    let global = context.execute_script("<anon>", &code)?;
    let global = context.resolve_value(global).await?;

    let scope = &mut context.handle_scope();
    let local = v8::Local::new(scope, global);
    // Deserialize a `v8` object into a Rust type using `serde_v8`,
    // in this case deserialize to a JSON `Value`.
    Ok(serde_v8::from_v8::<serde_json::Value>(scope, local)?)
}

// #[warn(dead_code)]
// async fn op_test(
//     _state: Rc<RefCell<OpState>>,
//     path: String,
//     _buf: Option<ZeroCopyBuf>,
// ) -> Result<String, anyhow::Error> {
//     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//     Ok(path)
// }

#[op]
async fn op_variable(args: Vec<String>) -> Result<String, anyhow::Error> {
    let workspace = &args[0];
    let path = &args[1];
    let token = &args[2];
    let base_url = &args[3];
    client::get_variable(workspace, path, token, &base_url).await
}

#[op]
async fn op_get_result(args: Vec<String>) -> Result<Option<serde_json::Value>, anyhow::Error> {
    let workspace = &args[0];
    let id = &args[1];
    let token = &args[2];
    let base_url = &args[3];
    let client = reqwest::Client::new();
    let result = client
        .get(format!(
            "{base_url}/api/w/{workspace}/jobs/completed/get_result/{id}"
        ))
        .bearer_auth(token)
        .send()
        .await?
        .json::<Option<serde_json::Value>>()
        .await?;
    Ok(result)
}

#[op]
async fn op_resource(args: Vec<String>) -> Result<Option<serde_json::Value>, anyhow::Error> {
    let workspace = &args[0];
    let path = &args[1];
    let token = &args[2];
    let base_url = &args[3];
    client::get_resource(workspace, path, token, &base_url).await
}

#[cfg(test)]
mod tests {

    use serde_json::json;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[tokio::test]
    async fn test_eval() -> anyhow::Result<()> {
        let env = vec![
            ("params".to_string(), json!({"test": 2})),
            ("value".to_string(), json!({"test": 2})),
        ];
        let code = "value.test + params.test";

        let mut runtime = JsRuntime::new(RuntimeOptions::default());
        let res = eval(&mut runtime, code, env, "workspace", "token", vec![]).await?;
        assert_eq!(res, json!(4));
        Ok(())
    }

    #[tokio::test]
    async fn test_eval_multiline() -> anyhow::Result<()> {
        let env = vec![];
        let code = "let x = 5;
`my ${x}
multiline template`";

        let mut runtime = JsRuntime::new(RuntimeOptions::default());
        let res = eval(&mut runtime, code, env, "workspace", "token", vec![]).await?;
        assert_eq!(res, json!("my 5\nmultiline template"));
        Ok(())
    }

    #[tokio::test]
    async fn test_eval_timeout() -> anyhow::Result<()> {
        let env = vec![
            ("params".to_string(), json!({"test": 2})),
            ("value".to_string(), json!({"test": 2})),
        ];
        let code = r#"variable("test")"#;

        let res = eval_timeout(code.to_string(), env, "workspace", "token", vec![]).await?;
        assert_eq!(res, json!("test"));
        Ok(())
    }
}
