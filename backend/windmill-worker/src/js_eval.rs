/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc, env, io::{BufReader, self}};

use deno_ast::{ParseParams, SourceTextInfo};
use deno_core::{
    op, serde_v8,
    v8::IsolateHandle,
    v8::{self},
    Extension, JsRuntime, Op, OpState, RuntimeOptions, Snapshot, error::AnyError,
};
use deno_fetch::FetchPermissions;
use deno_tls::{rustls::RootCertStore, rustls_pemfile};
use deno_web::{BlobStore, TimersPermission};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::value::RawValue;
use sqlx::types::Json;
use tokio::{
    sync::{mpsc, oneshot},
    time::timeout,
};
use uuid::Uuid;
use windmill_common::{error::Error, flow_status::JobResult};

use crate::{common::unsafe_raw, AuthedClient};

#[derive(Debug, Clone)]
pub struct IdContext {
    pub flow_job: Uuid,
    pub steps_results: HashMap<String, JobResult>,
    pub previous_id: String,
}

pub struct ContainerRootCertStoreProvider {
    root_cert_store: RootCertStore,
}

impl ContainerRootCertStoreProvider {
    fn new() -> ContainerRootCertStoreProvider {
        return ContainerRootCertStoreProvider {
            root_cert_store: deno_tls::create_default_root_cert_store(),
        }
    }

    fn add_certificate(&mut self, cert_path: String) -> io::Result<()> {
        let cert_file = std::fs::File::open(cert_path)?;
        let mut reader = BufReader::new(cert_file);
        let pem_file = rustls_pemfile::certs(&mut reader)?;

        self.root_cert_store.add_parsable_certificates(&pem_file);
        Ok(())
    }
}

impl deno_tls::RootCertStoreProvider for ContainerRootCertStoreProvider {
    fn get_or_try_init(&self) -> Result<&RootCertStore, AnyError> {
        Ok(&self.root_cert_store)
    }
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
    transform_context: HashMap<String, Arc<Box<RawValue>>>,
    flow_input: Option<Arc<HashMap<String, Box<RawValue>>>>,
    authed_client: Option<&AuthedClient>,
    by_id: Option<IdContext>,
) -> anyhow::Result<Box<RawValue>> {

    let expr = expr.trim().to_string();

    for (k,v) in transform_context.iter() {
        if k == &expr {
            return Ok(v.as_ref().clone())
        }
    }

    if expr.starts_with("flow_input.") || expr.starts_with("flow_input[") {
        if let Some(ref flow_input) = flow_input {
            for (k,v) in flow_input.iter() {
                if &format!("flow_input.{k}") == &expr || &format!("flow_input[\"{k}\"]") == &expr  {
                    // tracing::error!("FLOW_INPUT");
                    return Ok(v.clone())
                }
            }
        }
    }

    let p_id = by_id.as_ref().map(|x| format!("results.{}", x.previous_id));
    let p_id2 = by_id.as_ref().map(|x| format!("results[\"{}\"]", x.previous_id));

    if p_id.is_some() && transform_context.contains_key("previous_result") && &expr == p_id.as_ref().unwrap() {
        // tracing::error!("PREVIOUS_RESULT");
        return Ok(transform_context.get("previous_result").unwrap().as_ref().clone())
    }

    if p_id2.is_some() && transform_context.contains_key("previous_result") && &expr == p_id2.as_ref().unwrap() {
        return Ok(transform_context.get("previous_result").unwrap().as_ref().clone())
    }

    if by_id.is_some() && authed_client.is_some() {
        if let Some(x) = RE_FULL.captures(&expr).and_then(|x| x.get(1).map(|y| y.as_str())) {
            // tracing::error!("{:?}", x.split(".").collect::<Vec<_>>());
            let arr = x.split(".").collect::<Vec<_>>();
            let mut iter = arr.iter();
            iter.next();
            if let Some(id) = iter.next() {
                let path = iter.join(".");
                let query = if path.is_empty() { None } else { Some(path) };
                return authed_client.unwrap().get_result_by_id(&by_id.as_ref().unwrap().flow_job.to_string(), id, query).await;
            }
        }
    }


    let expr2 = expr.clone();
    let (sender, mut receiver) = oneshot::channel::<IsolateHandle>();
    let has_client = authed_client.is_some();
    let authed_client = authed_client.cloned();
    timeout(
        std::time::Duration::from_millis(10000),
        tokio::task::spawn_blocking(move || {
            let mut ops = vec![op_get_context::DECL];

            if authed_client.is_some() {
                ops.extend([
                    // An op for summing an array of numbers
                    // The op-layer automatically deserializes inputs
                    // and serializes the returned Result & value
                    op_variable::DECL,
                    op_resource::DECL,
                ])
            }

            if by_id.is_some() && authed_client.is_some() {
                ops.push(op_get_result::DECL);
                ops.push(op_get_id::DECL);
            }

            let ext = Extension { name: "js_eval", ops: ops.into(), ..Default::default() };
            let exts = vec![ext];
            // Use our snapshot to provision our new runtime
            let options = RuntimeOptions {
                extensions: exts,
                //                startup_snapshot: Some(Snapshot::Static(buffer)),
                ..Default::default()
            };


            let mut context_keys = transform_context
                .keys()
                .filter(|x| 
                    expr.contains(&x.to_string())
                )
                .map(|x| x.clone())
                .collect_vec();

            if !context_keys.contains(&"previous_result".to_string()) && (p_id.is_some() && expr.contains(p_id.as_ref().unwrap()))  || expr.contains("error") || (p_id2.is_some() && expr.contains(p_id2.as_ref().unwrap())) {
                context_keys.push("previous_result".to_string());
            }
            let has_flow_input = expr.contains("flow_input");
            if has_flow_input {
                context_keys.push("flow_input".to_string())
            }

            let mut js_runtime = JsRuntime::new(options);
            {
                let op_state = js_runtime.op_state();
                let mut op_state = op_state.borrow_mut();
                let mut client = authed_client.clone();
                if let Some(client) = client.as_mut() {
                    client.force_client = Some(reqwest::ClientBuilder::new()
                    .user_agent("windmill/beta")
                    .danger_accept_invalid_certs(std::env::var("ACCEPT_INVALID_CERTS").is_ok())
                    .build().unwrap());
                }
                op_state.put(OptAuthedClient(client));
                op_state.put(TransformContext {
                    flow_input: if has_flow_input { flow_input } else { None },
                    envs: transform_context
                        .into_iter()
                        .filter(|(a, _)| context_keys.contains(a))
                        .collect(),
                })
            }

            sender
                .send(js_runtime.v8_isolate().thread_safe_handle())
                .map_err(|_| Error::ExecutionErr("impossible to send v8 isolate".to_string()))?;

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;

            // pretty frail but this it to make the expr more user friendly and not require the user to write await
            let expr = ["variable", "resource"]
                .into_iter()
                .fold(expr, replace_with_await);

            let expr = replace_with_await_result(expr);

            let r = runtime.block_on(eval(
                &mut js_runtime,
                &expr,
                context_keys,
                by_id,
                has_client,
            ))?;

            Ok(r) as anyhow::Result<Box<RawValue>>
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
    static ref RE: Regex = Regex::new(r#"(?m)(?P<r>results(?:(?:\.(?:[a-z]|[A-Z]|_|[1-9])+)|(?:\[\".*?\"\])))"#).unwrap();
    static ref RE_FULL: Regex = Regex::new(r"(?m)^results((?:\.(?:(?:[a-z]|[A-Z]|_|[1-9])+))+)$").unwrap();

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
    transform_context: Vec<String>,
    by_id: Option<IdContext>,
    has_client: bool,
) -> anyhow::Result<Box<RawValue>> {
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
        return JSON.parse(await Deno.core.opAsync("op_get_id", [flow_job_id, node_id]));
    }}
}}

async function get_result(id) {{
    return JSON.parse(await Deno.core.opAsync("op_get_result", [id]));
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
function get_from_env(name) {{
    return JSON.parse(Deno.core.ops.op_get_context([name]));
}}
{api_code}
{}
{}
{by_id_code}
((async () => {{ 
    {f};
}})()).then((x) => JSON.stringify(x ?? null))
        "#,
        transform_context
            .iter()
            .map(|a| { format!("let {a} = get_from_env(\"{a}\");\n",) })
            .join(""),
        if expr.contains("error") && transform_context.contains(&"previous_result".to_string()) {
            "let error = previous_result.error"
        } else {
            ""
        },
    );

    
    let global = context.execute_script("<anon>", code.into())?;
    let global = context.resolve_value(global).await?;

    let scope = &mut context.handle_scope();
    let local = v8::Local::new(scope, global);
    // Deserialize a `v8` object into a Rust type using `serde_v8`,
    // in this case deserialize to a JSON `Value`.
    let r = serde_v8::from_v8::<String>(scope, local)?;
    Ok(unsafe_raw(r))
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
        Ok(client.get_variable_value(path).await?)
    } else {
        anyhow::bail!("No client found in op state");
    }
}

#[op]
async fn op_get_result(
    op_state: Rc<RefCell<OpState>>,
    args: Vec<String>,
) -> Result<String, anyhow::Error> {
    let id = &args[0];
    let client = op_state.borrow().borrow::<OptAuthedClient>().0.clone();
    if let Some(client) = client {
        let result = client
            .get_completed_job_result::<Box<RawValue>>(id, None)
            .await?
            .clone();
        Ok(result.get().to_string())
    } else {
        anyhow::bail!("No client found in op state");
    }
}

#[op]
async fn op_get_id(
    op_state: Rc<RefCell<OpState>>,
    args: Vec<String>,
) -> Result<Option<String>, anyhow::Error> {
    let flow_job_id = &args[0];
    let node_id = &args[1];

    let client = op_state.borrow().borrow::<OptAuthedClient>().0.clone();
    if let Some(client) = client {
        let result = client
            .get_result_by_id::<Option<Box<RawValue>>>(flow_job_id, node_id, None)
            .await.ok();
        if let Some(result) = result {
            Ok(result.map(|x| x.get().to_string()))
        } else {
            Ok(None)
        }
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
        client.get_resource_value_interpolated(path, None).await
    } else {
        anyhow::bail!("No client found in op state");
    }
}

pub struct TransformContext {
    pub envs: HashMap<String, Arc<Box<RawValue>>>,
    pub flow_input: Option<Arc<HashMap<String, Box<RawValue>>>>,
}

#[op]
fn op_get_context(op_state: Rc<RefCell<OpState>>, args: Vec<String>) -> String {
    let id = &args[0];
    let ops = op_state.borrow();
    let client = ops.borrow::<TransformContext>();
    if id == "flow_input" {
        return client
            .flow_input
            .as_ref()
            .and_then(|x| serde_json::to_string(&x).ok())
            .unwrap_or_else(|| "null".to_string());
    }
    return client
        .envs
        .get(id)
        .and_then(|x| serde_json::to_string(x).ok())
        .unwrap_or_else(String::new);
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
    args: Vec<Option<Box<RawValue>>>,
}

pub struct LogString {
    pub s: String,
}

pub async fn eval_fetch_timeout(
    ts_expr: String,
    js_expr: String,
    args: Option<&Json<HashMap<String, Box<RawValue>>>>,
) -> anyhow::Result<(Box<RawValue>, String)> {
    let (sender, mut receiver) = oneshot::channel::<IsolateHandle>();
    let ts_expr2 = ts_expr.clone();

    let parsed_args = windmill_parser_ts::parse_deno_signature(&ts_expr, true)?.args;
    let spread = parsed_args.into_iter().map(|x| args.as_ref().and_then(|args| args.0.get(&x.name).map(|x| x.clone()))).collect::<Vec<_>>();

    timeout(
        std::time::Duration::from_secs(100),
        tokio::task::spawn_blocking(move || {
            let ops = vec![op_get_static_args::DECL, op_log::DECL];
            let ext = Extension {
                  name: "windmill",
                  ops: ops.into(),
                  ..Default::default()
                };

            let deno_fetch_options = if let Some(cert_path) = env::var("DENO_CERT").ok() {
                let mut cert_store_provider = ContainerRootCertStoreProvider::new();
                cert_store_provider.add_certificate(cert_path)?;

                deno_fetch::Options {
                    root_cert_store_provider: Some(Arc::new(cert_store_provider)),
                    ..Default::default()
                }
            } else {
                Default::default()
            };

            let exts: Vec<Extension> = vec![
                deno_webidl::deno_webidl::init_ops(),
                deno_url::deno_url::init_ops(),
                deno_console::deno_console::init_ops(),
                deno_web::deno_web::init_ops::<PermissionsContainer>(
                    Arc::new(BlobStore::default()),
                    None,
                ),
                deno_fetch::deno_fetch::init_ops::<PermissionsContainer>(deno_fetch_options),
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

  
            {
                let op_state = js_runtime.op_state();
                let mut op_state = op_state.borrow_mut();
                op_state.put(PermissionsContainer{});
                //reqwest client seems to not be sharable between runtimes unfortunately
                // op_state.put(HTTP_CLIENT.clone());
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

            (r as anyhow::Result<Box<RawValue>>).map(|x| (x, js_runtime.op_state().borrow().borrow::<LogString>().s.clone()))
        }),
    )
    .await
    .map_err(|_| {
        if let Ok(isolate) = receiver.try_recv() {
            isolate.terminate_execution();
        };
        Error::ExecutionErr(format!(
            "The expression of evaluation `{ts_expr2}` took too long to execute (>100s)"
        ))
    })??
}

async fn eval_fetch(js_runtime: &mut JsRuntime, expr: &str) -> anyhow::Result<Box<RawValue>> {
    let _ = js_runtime
        .load_side_module(
            &deno_core::resolve_url("file:///eval.ts")?,
            Some(expr.to_string().into()),
        )
        .await?;

    let global = js_runtime.execute_script(
        "<anon>",
        r#"
let args = Deno.core.ops.op_get_static_args().map(JSON.parse)
import("file:///eval.ts").then((module) => module.main(...args)).then(JSON.stringify)
"#
        .to_string()
        .into(),
    )?;
    let global = js_runtime.resolve_value(global).await?;

    let scope = &mut js_runtime.handle_scope();
    let local = v8::Local::new(scope, global);
    // Deserialize a `v8` object into a Rust type using `serde_v8`,
    // in this case deserialize to a JSON `Value`.
    let r = serde_v8::from_v8::<Option<String>>(scope, local)?;
    Ok(unsafe_raw(r.unwrap_or_else(|| "null".to_string())))
}

#[op]
fn op_get_static_args(op_state: Rc<RefCell<OpState>>) -> Vec<Option<String>> {
    return op_state.borrow().borrow::<MainArgs>().args.iter().map(|x| x.as_ref().map(|y| y.get().to_string())).collect_vec();
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
    use windmill_common::worker::to_raw_value;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[tokio::test]
    async fn test_eval() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "params".to_string(),
            Arc::new(to_raw_value(&json!({"test": 2}))),
        );
        env.insert(
            "value".to_string(),
            Arc::new(to_raw_value(&json!({"test": 2}))),
        );

        let code = "value.test + params.test";

        let ops = vec![op_get_context::DECL];

        let ext = Extension { name: "js_eval", ops: ops.into(), ..Default::default() };
        let exts = vec![ext];

        let options = RuntimeOptions {
            extensions: exts,
            ..Default::default()
        };


        let mut runtime = JsRuntime::new(options);
        {
            let op_state = runtime.op_state();
            let mut op_state = op_state.borrow_mut();
            op_state.put(TransformContext { flow_input: None, envs: env.clone() })
        }

        let res = eval(
            &mut runtime,
            code,
            vec!["params".to_string(), "value".to_string()],
            None,
            false,
        )
        .await?;
        assert_eq!(res.get(), "4");
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
        assert_eq!(res.get(), "\"my 5\\nmultiline template\"");
        Ok(())
    }

    #[tokio::test]
    async fn test_eval_timeout() -> anyhow::Result<()> {
        let mut env = HashMap::new();
        env.insert(
            "params".to_string(),
            Arc::new(to_raw_value(&json!({"test": 2}))),
        );
        env.insert(
            "value".to_string(),
            Arc::new(to_raw_value(&json!({"test": 2}))),
        );

        let code = r#"params.test"#;

        let mut js_runtime = JsRuntime::new(RuntimeOptions::default());
        {
            let op_state = js_runtime.op_state();
            let mut op_state = op_state.borrow_mut();
            op_state.put(TransformContext { flow_input: None, envs: env.clone() })
        }

        let res = eval_timeout(code.to_string(), env, None, None, None).await?;
        assert_eq!(res.get(), "2");
        Ok(())
    }

    #[tokio::test]
    async fn test_eval_fetch_timeout() -> anyhow::Result<()> {
        let code = r#"export async function main() { return "" }"#;

        let res = eval_fetch_timeout(code.to_string(), code.to_string(), None).await?;
        assert_eq!(res.0.get(), "\"\"");
        Ok(())
    }
}
