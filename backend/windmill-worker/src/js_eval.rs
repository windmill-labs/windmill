/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use deno_core::{op, serde_v8, v8, v8::IsolateHandle, Extension, JsRuntime, RuntimeOptions};
use itertools::Itertools;
use regex::Regex;
use serde_json::Value;
use tokio::{sync::oneshot, time::timeout};
use uuid::Uuid;
use windmill_common::error::Error;

pub struct EvalCreds {
    pub workspace: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct IdContext(pub Uuid, pub HashMap<String, Uuid>);

pub async fn eval_timeout(
    expr: String,
    env: Vec<(String, serde_json::Value)>,
    creds: Option<EvalCreds>,
    steps: Vec<Uuid>,
    by_id: Option<IdContext>,
    base_internal_url: String,
) -> anyhow::Result<serde_json::Value> {
    let expr2 = expr.clone();
    let (sender, mut receiver) = oneshot::channel::<IsolateHandle>();
    timeout(
        std::time::Duration::from_millis(2000),
        tokio::task::spawn_blocking(move || {
            let mut ops = vec![];

            if creds.is_some() {
                ops.extend([
                    // An op for summing an array of numbers
                    // The op-layer automatically deserializes inputs
                    // and serializes the returned Result & value
                    op_variable::decl(),
                    op_resource::decl(),
                ])
            }

            if !steps.is_empty() || by_id.is_some() {
                ops.push(op_get_result::decl())
            }

            if by_id.is_some() {
                ops.push(op_get_id::decl())
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
            let expr = ["variable", "step", "resource", "result_by_id"]
                .into_iter()
                .fold(expr, replace_with_await);

            let r = runtime.block_on(eval(
                &mut js_runtime,
                &expr,
                env,
                creds,
                steps,
                by_id,
                &base_internal_url,
            ))?;

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
    creds: Option<EvalCreds>,
    steps: Vec<Uuid>,
    by_id: Option<IdContext>,
    base_internal_url: &str,
) -> anyhow::Result<serde_json::Value> {
    let expr = expr.trim();
    let expr = format!(
        "{}\nreturn {};",
        expr.split(SPLIT_PAT)
            .take(expr.split(SPLIT_PAT).count() - 1)
            .join("\n"),
        expr.split(SPLIT_PAT).last().unwrap_or_else(|| "")
    );
    let (steps_code, api_code, by_id_code) = if let Some(EvalCreds { workspace, token }) = creds {
        let steps_code = if !steps.is_empty() {
            format!(
                r#"
let steps = [{}];
async function step(n) {{
    if (n == -1) {{
        return previous_result;
    }}
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
            String::new()
        };

        let by_id_code = if let Some(by_id) = by_id {
            format!(
                r#"
async function result_by_id(node_id) {{
    let id_map = {{ {} }};
    let id = id_map[node_id];
    if (id) {{
        return await Deno.core.opAsync("op_get_result", [workspace, id, token, base_url]);
    }} else {{
        let flow_job_id = "{}";
        return await Deno.core.opAsync("op_get_id", [workspace, flow_job_id, token, base_url, node_id]);
    }}
}}"#,
                by_id
                    .1
                    .into_iter()
                    .map(|(k, v)| format!("\"{k}\": \"{v}\""))
                    .join(","),
                by_id.0,
            )
        } else {
            String::new()
        };

        let api_code = format!(
            r#"
let workspace = "{workspace}";
let base_url = "{}";
let token = "{token}";
async function variable(path) {{
    return await Deno.core.opAsync("op_variable", [workspace, path, token, base_url]);
}}
async function resource(path) {{
    return await Deno.core.opAsync("op_resource", [workspace, path, token, base_url]);
}}
        "#,
            base_internal_url,
        );
        (steps_code, api_code, by_id_code)
    } else {
        (String::new(), String::new(), String::new())
    };

    let code = format!(
        r#"
{api_code}
{}
{steps_code}
{by_id_code}
(async () => {{ 
    {expr} 
}})()
        "#,
        env.into_iter()
            .map(|(a, b)| {
                format!(
                    "let {a} = {};\n",
                    serde_json::to_string(&b)
                        .unwrap_or_else(|_| "\"error serializing value\"".to_string())
                )
            })
            .join(""),
    );
    tracing::debug!("{}", code);
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

// TODO: Can we a) share the api configuration here somehow or b) just implement this natively in deno, via the deno client?
#[op]
async fn op_variable(args: Vec<String>) -> Result<String, anyhow::Error> {
    let workspace = &args[0];
    let path = &args[1];
    let token = &args[2];
    let base_url = &args[3];
    let client = windmill_api_client::create_client(base_url, token.clone());
    let result = client.get_variable(workspace, path, None).await?;
    Ok(result.into_inner().value.unwrap_or_else(|| "".to_owned()))
}

#[op]
async fn op_get_result(args: Vec<String>) -> Result<serde_json::Value, anyhow::Error> {
    let workspace = &args[0];
    let id = &args[1];
    let token = &args[2];
    let base_url = &args[3];
    let client = windmill_api_client::create_client(base_url, token.clone());
    let result = client
        .get_completed_job(workspace, &id.parse()?)
        .await?
        .result
        .clone();
    Ok(serde_json::json!(result))
}

#[op]
async fn op_get_id(args: Vec<String>) -> Result<Option<serde_json::Value>, anyhow::Error> {
    let workspace = &args[0];
    let flow_job_id = &args[1];
    let token = &args[2];
    let base_url = &args[3];
    let node_id = &args[4];

    let client = windmill_api_client::create_client(base_url, token.clone());
    let result = client
        .result_by_id(workspace, flow_job_id, node_id, Some(true))
        .await
        .map_or(None, |e| Some(e.into_inner()));

    Ok(result)
}

#[op]
async fn op_resource(
    args: Vec<String>,
) -> Result<windmill_api_client::types::Resource, anyhow::Error> {
    let workspace = &args[0];
    let path = &args[1];
    let token = &args[2];
    let base_url = &args[3];
    let client = windmill_api_client::create_client(base_url, token.clone());
    let result = client.get_resource(workspace, path).await?;
    // TODO: verify this works. Previously this returned Option<serde_jons::Value>, now it's statically typed.
    Ok(result.into_inner())
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
        let res = eval(&mut runtime, code, env, None, vec![], None, "").await?;
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
        let res = eval(&mut runtime, code, env, None, vec![], None, "").await?;
        assert_eq!(res, json!("my 5\nmultiline template"));
        Ok(())
    }

    #[tokio::test]
    async fn test_eval_timeout() -> anyhow::Result<()> {
        let env = vec![
            ("params".to_string(), json!({"test": 2})),
            ("value".to_string(), json!({"test": 2})),
        ];
        let code = r#"params.test"#;

        let res = eval_timeout(code.to_string(), env, None, vec![], None, "".to_string()).await?;
        assert_eq!(res, json!(2));
        Ok(())
    }
}
