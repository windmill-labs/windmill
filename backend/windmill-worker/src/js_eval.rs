/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use deno_ast::{ParseParams, SourceTextInfo};
use deno_core::{
    op, serde_v8,
    v8::IsolateHandle,
    v8::{self},
    Extension, JsRuntime, OpState, RuntimeOptions, Snapshot,
};
use deno_fetch::FetchPermissions;
use deno_web::{BlobStore, TimersPermission};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;
use tokio::{
    sync::{mpsc, oneshot},
    time::timeout,
};
use uuid::Uuid;
use windmill_common::{error::Error, flow_status::JobResult};
use windmill_queue::HTTP_CLIENT;

use crate::AuthedClient;

#[derive(Debug, Clone)]
pub struct IdContext {
    pub flow_job: Uuid,
    pub steps_results: HashMap<String, JobResult>,
    pub previous_id: String,
}

pub struct PermissionsContainer;

impl FetchPermissions for PermissionsContainer {
    fn check_net_url(
        &mut self,
        _url: &deno_core::url::Url,
        _api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }

    fn check_read(
        &mut self,
        _p: &std::path::Path,
        _api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}

impl TimersPermission for PermissionsContainer {
    fn allow_hrtime(&mut self) -> bool {
        true
    }

    fn check_unstable(&self, _state: &OpState, _api_name: &'static str) {
        ()
    }
}
pub struct OptAuthedClient(Option<AuthedClient>);
pub async fn eval_timeout(
    expr: String,
    env: Vec<(String, serde_json::Value)>,
    authed_client: Option<&AuthedClient>,
    by_id: Option<IdContext>,
) -> anyhow::Result<serde_json::Value> {
    let expr2 = expr.clone();
    let (sender, mut receiver) = oneshot::channel::<IsolateHandle>();
    let has_client = authed_client.is_some();
    let authed_client = authed_client.cloned();
    timeout(
        std::time::Duration::from_millis(10000),
        tokio::task::spawn_blocking(move || {
            let mut ops = vec![];

            if authed_client.is_some() {
                ops.extend([
                    // An op for summing an array of numbers
                    // The op-layer automatically deserializes inputs
                    // and serializes the returned Result & value
                    op_variable::decl(),
                    op_resource::decl(),
                ])
            }

            if by_id.is_some() && authed_client.is_some() {
                ops.push(op_get_result::decl());
                ops.push(op_get_id::decl());
            }

            let ext = Extension::builder("js_eval").ops(ops).build();
            let exts = vec![ext];
            // Use our snapshot to provision our new runtime
            let options = RuntimeOptions {
                extensions: exts,
                //                startup_snapshot: Some(Snapshot::Static(buffer)),
                ..Default::default()
            };

            let mut js_runtime = JsRuntime::new(options);
            {
                let op_state = js_runtime.op_state();
                let mut op_state = op_state.borrow_mut();
                op_state.put(OptAuthedClient(authed_client.clone()));
            }

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

            let expr = replace_with_await_result(expr);

            let r = runtime.block_on(eval(&mut js_runtime, &expr, env, by_id, has_client))?;

            Ok(r) as anyhow::Result<Value>
        }),
    )
    .await
    .map_err(|_| {
        if let Ok(isolate) = receiver.try_recv() {
            isolate.terminate_execution();
        };
        Error::ExecutionErr(format!(
            "The expression of evaluation `{expr2}` took too long to execute (>10000ms)"
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
lazy_static! {
    static ref RE: Regex = Regex::new("(?m)(?P<r>results.([a-z]|[A-Z]|_|[1-9])+)").unwrap();
}

fn replace_with_await_result(expr: String) -> String {
    RE.replace_all(&expr, "(await $r)").to_string()
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

async fn eval(
    context: &mut JsRuntime,
    expr: &str,
    env: Vec<(String, serde_json::Value)>,
    by_id: Option<IdContext>,
    has_client: bool,
) -> anyhow::Result<serde_json::Value> {
    let (api_code, by_id_code) = if has_client {
        let by_id_code = if let Some(by_id) = by_id {
            format!(
                r#"
async function result_by_id(node_id) {{
    let id_map = {{ {} }};
    let id = id_map[node_id];
    if (node_id == "{}") {{
        return previous_result;
    }} else if (id) {{
        if (Array.isArray(id)) {{
            return await Promise.all(id.map(async (id) => await get_result(id)));
        }} else {{
            return await get_result(id);
        }}
    }} else {{
        let flow_job_id = "{}";
        return await Deno.core.opAsync("op_get_id", [flow_job_id, node_id]);
    }}
}}

async function get_result(id) {{
    return await Deno.core.opAsync("op_get_result", [id]);
}}
const results = new Proxy({{}}, {{
    get: function(target, name, receiver) {{
        return result_by_id(name);
    }}
}});

"#,
                by_id
                    .steps_results
                    .into_iter()
                    .map(|(k, v)| {
                        let v_str = match v {
                            JobResult::SingleJob(x) => format!("\"{x}\""),
                            JobResult::ListJob(x) => {
                                format!("[{}]", x.iter().map(|x| format!("\"{x}\"")).join(","))
                            }
                        };
                        format!("\"{k}\": {v_str}")
                    })
                    .join(","),
                by_id.previous_id,
                by_id.flow_job,
            )
        } else {
            String::new()
        };

        let api_code = format!(
            r#"
async function variable(path) {{
    return await Deno.core.opAsync("op_variable", [path]);
}}
async function resource(path) {{
    return await Deno.core.opAsync("op_resource", [path]);
}}
        "#,
        );
        (api_code, by_id_code)
    } else {
        (String::new(), String::new())
    };

    let f = if expr.contains("return ") {
        expr.to_string()
    } else {
        format!("return {expr}")
    };
    let code = format!(
        r#"
{api_code}
{}
{by_id_code}
(async () => {{ 
    {f};
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
    let global = context.execute_script("<anon>", code.into())?;
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
async fn op_variable(
    op_state: Rc<RefCell<OpState>>,
    args: Vec<String>,
) -> Result<String, anyhow::Error> {
    let path = &args[0];
    let client = op_state.borrow().borrow::<OptAuthedClient>().0.clone();
    if let Some(client) = client {
        let result = client
            .get_client()
            .get_variable_value(&client.workspace, path)
            .await?;
        Ok(result.into_inner())
    } else {
        anyhow::bail!("No client found in op state");
    }
}

#[op]
async fn op_get_result(
    op_state: Rc<RefCell<OpState>>,
    args: Vec<String>,
) -> Result<serde_json::Value, anyhow::Error> {
    let id = &args[0];
    let client = op_state.borrow().borrow::<OptAuthedClient>().0.clone();
    if let Some(client) = client {
        let result = client
            .get_client()
            .get_completed_job_result(&client.workspace, &id.parse()?)
            .await?
            .clone();
        Ok(serde_json::json!(result))
    } else {
        anyhow::bail!("No client found in op state");
    }
}

#[op]
async fn op_get_id(
    op_state: Rc<RefCell<OpState>>,
    args: Vec<String>,
) -> Result<Option<serde_json::Value>, anyhow::Error> {
    let flow_job_id = &args[0];
    let node_id = &args[1];

    let client = op_state.borrow().borrow::<OptAuthedClient>().0.clone();
    if let Some(client) = client {
        let result = client
            .get_client()
            .result_by_id(&client.workspace, flow_job_id, node_id)
            .await
            .map_or(None, |e| Some(e.into_inner()));
        Ok(result)
    } else {
        anyhow::bail!("No client found in op state");
    }
}

#[op]
async fn op_resource(
    op_state: Rc<RefCell<OpState>>,
    args: Vec<String>,
) -> Result<serde_json::Value, anyhow::Error> {
    let path = &args[0];

    let client = op_state.borrow().borrow::<OptAuthedClient>().0.clone();
    if let Some(client) = client {
        let result = client
            .get_client()
            .get_resource_value_interpolated(&client.workspace, path)
            .await?;
        Ok(result.into_inner())
    } else {
        anyhow::bail!("No client found in op state");
    }
}

pub fn transpile_ts(expr: String) -> anyhow::Result<String> {
    let parsed = deno_ast::parse_module(ParseParams {
        specifier: "eval.ts".to_string(),
        text_info: SourceTextInfo::from_string(expr),
        capture_tokens: false,
        scope_analysis: false,
        media_type: deno_ast::MediaType::TypeScript,
        maybe_syntax: None,
    })?;
    Ok(parsed.transpile(&Default::default())?.text)
}

static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/FETCH_SNAPSHOT.bin"));

pub struct MainArgs {
    args: Vec<serde_json::Value>,
}

pub struct LogString {
    pub s: String,
}

pub async fn eval_fetch_timeout(
    ts_expr: String,
    js_expr: String,
    args: serde_json::Map<String, Value>,
) -> anyhow::Result<(serde_json::Value, String)> {
    let (sender, mut receiver) = oneshot::channel::<IsolateHandle>();
    let ts_expr2 = ts_expr.clone();
    timeout(
        std::time::Duration::from_millis(100000),
        tokio::task::spawn_blocking(move || {
            let ops = vec![op_get_static_args::decl(), op_log::decl()];
            let ext = Extension::builder("windmill").ops(ops).build();

            let exts: Vec<Extension> = vec![
                deno_webidl::deno_webidl::init_ops(),
                deno_url::deno_url::init_ops(),
                deno_console::deno_console::init_ops(),
                deno_web::deno_web::init_ops::<PermissionsContainer>(
                    BlobStore::default(),
                    None,
                ),
                deno_fetch::deno_fetch::init_ops::<PermissionsContainer>(
                    Default::default(),
                ),
                ext
            ];

            // Use our snapshot to provision our new runtime
            let options = RuntimeOptions {
                is_main: true,
                extensions: exts,
                create_params: Some(deno_core::v8::CreateParams::default().heap_limits(
                    0 as usize,
                    1024*1024*128 as usize,
                )),
                startup_snapshot: Some(Snapshot::Static(RUNTIME_SNAPSHOT)),
                module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
                //                startup_snapshot: Some(Snapshot::Static(buffer)),
                ..Default::default()
            };


            let (memory_limit_tx, mut memory_limit_rx) = mpsc::unbounded_channel::<()>();

            // tracing::info!("starting isolate");
            // let instant = Instant::now();


            let mut js_runtime: JsRuntime = JsRuntime::new(options);
            // tracing::info!("ttc: {:?}", instant.elapsed());

            js_runtime.add_near_heap_limit_callback(move |x,y| {
                tracing::error!("heap limit reached: {x} {y}");

                if memory_limit_tx.send(()).is_err() {
                    tracing::error!("failed to send memory limit reached notification - isolate may already be terminating");
                };
                //to give a bit of time to kill the worker without v8 crashing
                return y*2;
            });

            let parsed_args = windmill_parser_ts::parse_deno_signature(&ts_expr, true)?.args;
            let spread = parsed_args.into_iter().map(|x| args.get(&x.name).map(|x| x.clone()).unwrap_or(serde_json::Value::Null)).collect::<Vec<_>>();

            {
                let op_state = js_runtime.op_state();
                let mut op_state = op_state.borrow_mut();
                op_state.put(PermissionsContainer{});
                op_state.put(HTTP_CLIENT.clone());
                op_state.put(MainArgs { args: spread });
                op_state.put(LogString { s: String::new() });
            }


            sender
                .send(js_runtime.v8_isolate().thread_safe_handle())
                .map_err(|_| Error::ExecutionErr("impossible to send v8 isolate".to_string()))?;

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;


            let future = async { tokio::select! {
                r = eval_fetch(&mut js_runtime, &js_expr) => Ok(r),
                _ = memory_limit_rx.recv() => Err(Error::ExecutionErr("Memory limit reached, killing isolate".to_string()))
            }};
            let r = runtime.block_on(future)?;
            // tracing::info!("total: {:?}", instant.elapsed());

            (r as anyhow::Result<Value>).map(|x| (x, js_runtime.op_state().borrow().borrow::<LogString>().s.clone()))
        }),
    )
    .await
    .map_err(|_| {
        if let Ok(isolate) = receiver.try_recv() {
            isolate.terminate_execution();
        };
        Error::ExecutionErr(format!(
            "The expression of evaluation `{ts_expr2}` took too long to execute (>10000ms)"
        ))
    })??
}

async fn eval_fetch(js_runtime: &mut JsRuntime, expr: &str) -> anyhow::Result<serde_json::Value> {
    let _ = js_runtime
        .load_side_module(
            &deno_core::resolve_url("file:///eval.ts")?,
            Some(expr.to_string().into()),
        )
        .await?;

    let global = js_runtime.execute_script(
        "<anon>",
        r#"
let args = Deno.core.ops.op_get_static_args()
import("file:///eval.ts").then((module) => module.main(...args))
"#
        .to_string()
        .into(),
    )?;
    let global = js_runtime.resolve_value(global).await?;

    let scope = &mut js_runtime.handle_scope();
    let local = v8::Local::new(scope, global);
    // Deserialize a `v8` object into a Rust type using `serde_v8`,
    // in this case deserialize to a JSON `Value`.
    Ok(serde_v8::from_v8::<serde_json::Value>(scope, local)?)
}

#[op]
fn op_get_static_args(op_state: Rc<RefCell<OpState>>) -> Vec<serde_json::Value> {
    return op_state.borrow().borrow::<MainArgs>().args.clone();
}

#[op]
fn op_log(op_state: Rc<RefCell<OpState>>, args: Vec<String>) {
    op_state
        .borrow_mut()
        .borrow_mut::<LogString>()
        .s
        .push_str(args.get(0).unwrap());
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
        let res = eval(&mut runtime, code, env, None, false).await?;
        assert_eq!(res, json!(4));
        Ok(())
    }

    #[tokio::test]
    async fn test_eval_multiline() -> anyhow::Result<()> {
        let env = vec![];
        let code = "let x = 5;
return `my ${x}
multiline template`";

        let mut runtime = JsRuntime::new(RuntimeOptions::default());
        let res = eval(&mut runtime, code, env, None, false).await?;
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

        let res = eval_timeout(code.to_string(), env, None, None).await?;
        assert_eq!(res, json!(2));
        Ok(())
    }

    #[tokio::test]
    async fn test_eval_fetch_timeout() -> anyhow::Result<()> {
        let code = r#"export async function main() { return "" }"#;

        let res =
            eval_fetch_timeout(code.to_string(), code.to_string(), serde_json::Map::new()).await?;
        assert_eq!(res.0, "".to_string());
        Ok(())
    }
}
