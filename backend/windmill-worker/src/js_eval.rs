/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(feature = "deno_core")]
use std::{borrow::Cow, cell::RefCell, env, path::PathBuf, rc::Rc};

use std::{collections::HashMap, sync::Arc};

#[cfg(feature = "deno_core")]
use deno_ast::ParseParams;
#[cfg(feature = "deno_core")]
use deno_core::{
    op2, serde_v8, url,
    v8::{self, IsolateHandle},
    Extension, JsRuntime, OpState, PollEventLoopOptions, RuntimeOptions,
};
#[cfg(feature = "deno_core")]
use deno_fetch::FetchPermissions;
#[cfg(feature = "deno_core")]
use deno_net::NetPermissions;

#[cfg(feature = "deno_core")]
use deno_web::{BlobStore, TimersPermission};
#[cfg(feature = "deno_core")]
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::value::RawValue;
use sqlx::types::Json;

#[cfg(feature = "deno_core")]
use tokio::{
    sync::{mpsc, oneshot},
    time::timeout,
};
use uuid::Uuid;

#[cfg(feature = "deno_core")]
use windmill_common::error::Error;
#[cfg(feature = "deno_core")]
use windmill_common::worker::{write_file, TMP_DIR};

use windmill_common::flow_status::JobResult;
use windmill_queue::CanceledBy;

use crate::common::OccupancyMetrics;
use windmill_common::client::AuthedClient;

#[cfg(feature = "deno_core")]
use crate::{common::unsafe_raw, handle_child::run_future_with_polling_update_job_poller};

#[derive(Debug, Clone)]
pub struct IdContext {
    pub flow_job: Uuid,
    #[allow(dead_code)]
    pub steps_results: HashMap<String, JobResult>,
    pub previous_id: String,
}

// #[cfg(feature = "deno_core")]
// pub struct ContainerRootCertStoreProvider {
//     root_cert_store: RootCertStore,
// }

// #[cfg(feature = "deno_core")]
// impl ContainerRootCertStoreProvider {
//     fn new() -> ContainerRootCertStoreProvider {
//         return ContainerRootCertStoreProvider {
//             root_cert_store: deno_tls::create_default_root_cert_store(),
//         };
//     }

//     fn add_certificate(&mut self, cert_path: String) -> io::Result<()> {
//         let cert_file = std::fs::File::open(cert_path)?;
//         let mut reader = BufReader::new(cert_file);
//         let pem_file = rustls_pemfile::certs(&mut reader).collect::<Result<Vec<_>, _>>()?;

//         self.root_cert_store.add_parsable_certificates(pem_file);
//         Ok(())
//     }
// }

// #[cfg(feature = "deno_core")]
// impl deno_tls::RootCertStoreProvider for ContainerRootCertStoreProvider {
//     fn get_or_try_init(&self) -> Result<&RootCertStore, AnyError> {
//         Ok(&self.root_cert_store)
//     }
// }

#[cfg(feature = "deno_core")]
pub struct PermissionsContainer;

#[cfg(feature = "deno_core")]
impl FetchPermissions for PermissionsContainer {
    #[inline(always)]
    fn check_net_url(
        &mut self,
        _url: &deno_core::url::Url,
        _api_name: &str,
    ) -> Result<(), deno_permissions::PermissionCheckError> {
        Ok(())
    }

    #[inline(always)]
    fn check_read<'a>(
        &mut self,
        _resolved: bool,
        p: &'a std::path::Path,
        _api_name: &str,
    ) -> Result<Cow<'a, std::path::Path>, deno_io::fs::FsError> {
        Ok(Cow::Borrowed(p))
    }
}

#[cfg(feature = "deno_core")]
impl TimersPermission for PermissionsContainer {
    #[inline(always)]
    fn allow_hrtime(&mut self) -> bool {
        true
    }
}

#[cfg(feature = "deno_core")]
impl NetPermissions for PermissionsContainer {
    fn check_read<'a>(
        &mut self,
        p: &'a str,
        _api_name: &str,
    ) -> Result<PathBuf, deno_permissions::PermissionCheckError> {
        Ok(PathBuf::from(p))
    }

    fn check_write<'a>(
        &mut self,
        p: &'a str,
        _api_name: &str,
    ) -> Result<PathBuf, deno_permissions::PermissionCheckError> {
        Ok(PathBuf::from(p))
    }

    fn check_net<T: AsRef<str>>(
        &mut self,
        _host: &(T, Option<u16>),
        _api_name: &str,
    ) -> Result<(), deno_permissions::PermissionCheckError> {
        Ok(())
    }

    fn check_write_path<'a>(
        &mut self,
        p: &'a std::path::Path,
        _api_name: &str,
    ) -> Result<std::borrow::Cow<'a, std::path::Path>, deno_permissions::PermissionCheckError> {
        Ok(Cow::Borrowed(p))
    }
}

#[cfg(feature = "deno_core")]
pub struct OptAuthedClient(Option<AuthedClient>);

pub async fn eval_timeout(
    expr: String,
    transform_context: HashMap<String, Arc<Box<RawValue>>>,
    flow_input: Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
    authed_client: Option<&AuthedClient>,
    by_id: Option<&IdContext>,
    #[allow(unused_variables)] ctx: Option<Vec<(String, String)>>,
) -> anyhow::Result<Box<RawValue>> {
    let expr = expr.trim().to_string();

    tracing::debug!(
        "evaluating js eval: {} with context {:?}",
        expr,
        transform_context
    );
    for (k, v) in transform_context.iter() {
        if k == &expr {
            return Ok(v.as_ref().clone());
        }
    }

    if expr.starts_with("flow_input.") || expr.starts_with("flow_input[") {
        if let Some(ref flow_input) = flow_input {
            for (k, v) in flow_input.iter() {
                if &format!("flow_input.{k}") == &expr || &format!("flow_input[\"{k}\"]") == &expr {
                    // tracing::error!("FLOW_INPUT");
                    return Ok(v.clone());
                }
            }
        }
    }

    let p_ids = by_id.map(|x| {
        [
            format!("results.{}", x.previous_id),
            format!("results?.{}", x.previous_id),
            format!("results[\"{}\"]", x.previous_id),
            format!("results?.[\"{}\"]", x.previous_id),
        ]
    });

    if p_ids.is_some()
        && transform_context.contains_key("previous_result")
        && p_ids.as_ref().unwrap().iter().any(|x| x == &expr)
    {
        // tracing::error!("PREVIOUS_RESULT");
        return Ok(transform_context
            .get("previous_result")
            .unwrap()
            .as_ref()
            .clone());
    }

    if let (Some(by_id), Some(authed_client)) = (by_id, authed_client) {
        if let Some((id, idx_o, rest)) = RE_FULL.captures(&expr).map(|x| {
            (
                x.get(1).unwrap().as_str(),
                x.get(2).map(|y| y.as_str()),
                x.get(3).map(|y| y.as_str()),
            )
        }) {
            let query = if let Some(idx) = idx_o {
                match rest {
                    Some(rest) => Some(format!("{}{}", idx, rest)),
                    None => Some(idx.to_string()),
                }
            } else {
                rest.map(|x| x.trim_start_matches('.').to_string())
            };
            return authed_client
                .get_result_by_id(&by_id.flow_job.to_string(), id, query)
                .await;
        }
    }

    #[cfg(not(feature = "deno_core"))]
    {
        #[allow(unreachable_code)]
        return Err(anyhow::anyhow!("Deno core is not enabled".to_string()).into());
    }

    #[cfg(feature = "deno_core")]
    {
        let expr2 = expr.clone();
        let by_id = by_id.cloned();
        let (sender, mut receiver) = oneshot::channel::<IsolateHandle>();
        let has_client = authed_client.is_some();
        let authed_client = authed_client.cloned();
        return timeout(
            std::time::Duration::from_millis(10000),
            tokio::task::spawn_blocking(move || {
                let mut ops = vec![op_get_context()];

                if authed_client.is_some() {
                    ops.extend([
                        // An op for summing an array of numbers
                        // The op-layer automatically deserializes inputs
                        // and serializes the returned Result & value
                        op_variable(),
                        op_resource(),
                    ])
                }

                if by_id.is_some() && authed_client.is_some() {
                    ops.push(op_get_result());
                    ops.push(op_get_id());
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
                    .filter(|x| expr.contains(&x.to_string()))
                    .map(|x| x.clone())
                    .collect_vec();

                if !context_keys.contains(&"previous_result".to_string())
                    && (p_ids.is_some() && p_ids.as_ref().unwrap().iter().any(|x| expr.contains(x)))
                    || expr.contains("error")
                {
                    // tracing::error!("PREVIOUS_RESULT");
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
                        client.force_client = Some(
                            reqwest::ClientBuilder::new()
                                .user_agent("windmill/beta")
                                .danger_accept_invalid_certs(
                                    std::env::var("ACCEPT_INVALID_CERTS").is_ok(),
                                )
                                .build()
                                .unwrap(),
                        );
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
                    .map_err(|_| {
                        Error::ExecutionErr("impossible to send v8 isolate".to_string())
                    })?;

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
                    ctx,
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
        })??;
    }
}

#[cfg(feature = "deno_core")]
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
    static ref RE: Regex =
        Regex::new(r#"(?m)(?P<r>results(?:\?)?(?:(?:\.[a-zA-Z_0-9]+)|(?:\[\".*?\"\])))"#).unwrap();
    static ref RE_FULL: Regex =
        Regex::new(r"(?m)^results(?:\?)?\.([a-zA-Z_0-9]+)(?:\[(\d+)\])?((?:\.[a-zA-Z_0-9]+)+)?$")
            .unwrap();
    static ref RE_PROXY: Regex =
        Regex::new(r"^(https?)://(([^:@\s]+):([^:@\s]+)@)?([^:@\s]+)(:(\d+))?$").unwrap();
}

#[cfg(feature = "deno_core")]
fn replace_with_await_result(expr: String) -> String {
    RE.replace_all(&expr, "(await $r)").to_string()
}

#[cfg(feature = "deno_core")]
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

#[cfg(feature = "deno_core")]
async fn eval(
    context: &mut JsRuntime,
    expr: &str,
    transform_context: Vec<String>,
    by_id: Option<IdContext>,
    has_client: bool,
    ctx: Option<Vec<(String, String)>>,
) -> anyhow::Result<Box<RawValue>> {
    tracing::debug!("evaluating: {} {:#?}", expr, by_id);

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
        return JSON.parse(await Deno.core.ops.op_get_id(flow_job_id, node_id));
    }}
}}

async function get_result(id) {{
    return JSON.parse(await Deno.core.ops.op_get_result(id));
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
    return await Deno.core.ops.op_variable(path);
}}
async function resource(path) {{
    return JSON.parse(await Deno.core.ops.op_resource(path));
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

    let ctx_str = ctx
        .map(|x| {
            x.into_iter()
                .map(|(k, v)| format!("let {} = \"{}\";", k, v))
                .join("\n")
        })
        .unwrap_or_default();
    let code = format!(
        r#"
function get_from_env(name) {{
    return JSON.parse(Deno.core.ops.op_get_context(name));
}}
{ctx_str}

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

    let script = context.execute_script("<anon>", code)?;
    let fut = context.resolve(script);
    let global = context
        .with_event_loop_promise(fut, PollEventLoopOptions::default())
        .await?;

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
#[cfg(feature = "deno_core")]
#[op2(async)]
#[string]
async fn op_variable(
    op_state: Rc<RefCell<OpState>>,
    #[string] path: String,
) -> Result<String, deno_error::JsErrorBox> {
    let client = op_state.borrow().borrow::<OptAuthedClient>().0.clone();
    if let Some(client) = client {
        Ok(client
            .get_variable_value(&path)
            .await
            .map_err(|e| deno_error::JsErrorBox::generic(e.to_string()))?)
    } else {
        Err(deno_error::JsErrorBox::generic(
            "No client found in op state",
        ))
    }
}

#[cfg(feature = "deno_core")]
#[op2(async)]
#[string]
async fn op_get_result(
    op_state: Rc<RefCell<OpState>>,
    #[string] id: String,
) -> Result<String, deno_error::JsErrorBox> {
    let client = op_state.borrow().borrow::<OptAuthedClient>().0.clone();
    if let Some(client) = client {
        client
            .get_completed_job_result::<Box<RawValue>>(&id, None)
            .await
            .map_err(|e| deno_error::JsErrorBox::generic(e.to_string()))
            .map(|x| x.get().to_string())
    } else {
        Err(deno_error::JsErrorBox::generic(
            "No client found in op state",
        ))
    }
}

#[cfg(feature = "deno_core")]
#[op2(async)]
#[string]
async fn op_get_id(
    op_state: Rc<RefCell<OpState>>,
    #[string] flow_job_id: String,
    #[string] node_id: String,
) -> Result<Option<String>, deno_error::JsErrorBox> {
    let client = op_state.borrow().borrow::<OptAuthedClient>().0.clone();
    if let Some(client) = client {
        let result = client
            .get_result_by_id::<Option<Box<RawValue>>>(&flow_job_id, &node_id, None)
            .await
            .ok();
        if let Some(result) = result {
            Ok(result.map(|x| x.get().to_string()))
        } else {
            Ok(None)
        }
    } else {
        Err(deno_error::JsErrorBox::generic(
            "No client found in op state",
        ))
    }
}

#[cfg(feature = "deno_core")]
#[op2(async)]
#[string]
async fn op_resource(
    op_state: Rc<RefCell<OpState>>,
    #[string] path: String,
) -> Result<Option<String>, deno_error::JsErrorBox> {
    let client = op_state.borrow().borrow::<OptAuthedClient>().0.clone();
    if let Some(client) = client {
        client
            .get_resource_value_interpolated::<Option<Box<RawValue>>>(&path, None)
            .await
            .map(|x| x.map(|x| x.get().to_string()))
            .map_err(|e| deno_error::JsErrorBox::generic(e.to_string()))
    } else {
        Err(deno_error::JsErrorBox::generic(
            "No client found in op state",
        ))
    }
}

#[cfg(feature = "deno_core")]
pub struct TransformContext {
    pub envs: HashMap<String, Arc<Box<RawValue>>>,
    pub flow_input: Option<mappable_rc::Marc<HashMap<String, Box<RawValue>>>>,
}

#[cfg(feature = "deno_core")]
#[op2]
#[string]
fn op_get_context(op_state: Rc<RefCell<OpState>>, #[string] id: &str) -> String {
    let ops = op_state.borrow();
    let client = ops.borrow::<TransformContext>();
    if id == "flow_input" {
        client
            .flow_input
            .as_ref()
            .and_then(|x| serde_json::to_string(x.as_ref()).ok())
            .unwrap_or_else(|| "null".to_string())
    } else {
        client
            .envs
            .get(id)
            .and_then(|x| serde_json::to_string(x).ok())
            .unwrap_or_else(String::new)
    }
}

#[cfg(feature = "deno_core")]
pub fn transpile_ts(expr: String) -> anyhow::Result<String> {
    let parsed = deno_ast::parse_module(ParseParams {
        specifier: url::Url::parse("file:///eval.ts")?,
        capture_tokens: false,
        scope_analysis: false,
        media_type: deno_ast::MediaType::TypeScript,
        maybe_syntax: None,
        text: deno_core::ModuleCodeString::from(expr).into(),
    })?;
    Ok(parsed
        .transpile(
            &Default::default(),
            &Default::default(),
            &Default::default(),
        )?
        .into_source()
        .text)
}

#[cfg(not(feature = "deno_core"))]
pub fn transpile_ts(_expr: String) -> anyhow::Result<String> {
    Ok("require deno".to_string())
}

#[cfg(feature = "deno_core")]
static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/FETCH_SNAPSHOT.bin"));

#[cfg(feature = "deno_core")]
pub struct MainArgs {
    args: Vec<Option<Box<RawValue>>>,
}

#[cfg(feature = "deno_core")]
pub struct LogString {
    pub s: mpsc::UnboundedSender<String>,
}

#[cfg(feature = "deno_core")]
pub struct NativeAnnotation {
    pub useragent: Option<String>,
    pub proxy: Option<(String, Option<(String, String)>)>,
}
#[cfg(feature = "deno_core")]
pub fn get_annotation(inner_content: &str) -> NativeAnnotation {
    let mut res = NativeAnnotation { useragent: None, proxy: None };

    let anns = inner_content
        .lines()
        .take_while(|x| x.starts_with("//"))
        .map(|x| x.to_string().trim_start_matches("//").trim().to_string())
        .collect_vec();

    for ann in anns.iter() {
        if ann.starts_with("useragent") {
            res.useragent = Some(ann.trim_start_matches("useragent").trim().to_string());
        } else if ann.starts_with("proxy") {
            res.proxy = capture_proxy(ann.trim_start_matches("proxy").trim());
        }
    }
    res
}

#[cfg(feature = "deno_core")]
fn capture_proxy(s: &str) -> Option<(String, Option<(String, String)>)> {
    RE_PROXY.captures(s).map(|x| {
        (
            format!(
                "{}://{}{}",
                x.get(1).map(|x| x.as_str()).unwrap_or_default(),
                x.get(5).map(|x| x.as_str()).unwrap_or_default(),
                x.get(7)
                    .map(|x| format!(":{}", x.as_str()))
                    .unwrap_or_default(),
            ),
            x.get(3).map(|y| {
                (
                    y.as_str().to_string(),
                    x.get(4).map(|x| x.as_str().to_string()).unwrap_or_default(),
                )
            }),
        )
    })
}

use windmill_common::worker::Connection;

#[cfg(not(feature = "deno_core"))]
pub async fn eval_fetch_timeout(
    _env_code: String,
    _ts_expr: String,
    _js_expr: String,
    _args: Option<&Json<HashMap<String, Box<RawValue>>>>,
    _script_entrypoint_override: Option<String>,
    _job_id: Uuid,
    _job_timeout: Option<i32>,
    _conn: &Connection,
    _mem_peak: &mut i32,
    _canceled_by: &mut Option<CanceledBy>,
    _worker_name: &str,
    _w_id: &str,
    _load_client: bool,
    _occupation_metrics: &mut OccupancyMetrics,
) -> anyhow::Result<Box<RawValue>> {
    use serde_json::value::to_raw_value;
    Ok(to_raw_value("require deno_core").unwrap())
}

#[cfg(feature = "deno_core")]
pub async fn eval_fetch_timeout(
    env_code: String,
    ts_expr: String,
    js_expr: String,
    args: Option<&Json<HashMap<String, Box<RawValue>>>>,
    script_entrypoint_override: Option<String>,
    job_id: Uuid,
    job_timeout: Option<i32>,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    w_id: &str,
    load_client: bool,
    occupation_metrics: &mut OccupancyMetrics,
) -> windmill_common::error::Result<Box<RawValue>> {
    use windmill_queue::append_logs;

    let (sender, mut receiver) = oneshot::channel::<IsolateHandle>();

    let parsed_args = windmill_parser_ts::parse_deno_signature(
        &ts_expr,
        true,
        false,
        script_entrypoint_override.clone(),
    )?
    .args;
    let spread = parsed_args
        .into_iter()
        .map(|x| {
            args.as_ref()
                .and_then(|args| args.0.get(&x.name).map(|x| x.clone()))
        })
        .collect::<Vec<_>>();

    let ann = get_annotation(&ts_expr);

    #[cfg(not(feature = "enterprise"))]
    if ann.proxy.is_some() {
        return Err(Error::ExecutionErr("Proxy is an EE feature".to_string()).into());
    }

    let mut extra_logs = String::new();
    if ann.useragent.is_some() {
        extra_logs.push_str(&format!("useragent: {}\n", ann.useragent.as_ref().unwrap()));
    }
    if ann.proxy.is_some() {
        let (proxy, auth) = ann.proxy.as_ref().unwrap();
        extra_logs.push_str(&format!(
            "proxy: {proxy} (basic auth: {})\n",
            auth.is_some()
        ));
    }

    let conn_ = conn.clone();
    let w_id_ = w_id.to_string();
    let result_f = tokio::task::spawn_blocking(move || {
        let ops = vec![op_get_static_args(), op_log()];
        let ext = Extension { name: "windmill", ops: ops.into(), ..Default::default() };

        let fetch_options = deno_fetch::Options {
            root_cert_store_provider: None,
            user_agent: ann.useragent.unwrap_or_else(|| "windmill/beta".to_string()),
            proxy: ann.proxy.map(|x| deno_tls::Proxy {
                url: x.0,
                basic_auth: x
                    .1
                    .map(|(username, password)| deno_tls::BasicAuth { username, password }),
            }),
            ..Default::default()
        };

        let exts: Vec<Extension> = vec![
            deno_telemetry::deno_telemetry::init_ops(),
            deno_webidl::deno_webidl::init_ops(),
            deno_url::deno_url::init_ops(),
            deno_console::deno_console::init_ops(),
            deno_web::deno_web::init_ops::<PermissionsContainer>(
                Arc::new(BlobStore::default()),
                None,
            ),
            deno_fetch::deno_fetch::init_ops::<PermissionsContainer>(fetch_options),
            deno_net::deno_net::init_ops::<PermissionsContainer>(None, None),
            ext,
        ];

        // Use our snapshot to provision our new runtime
        let options = RuntimeOptions {
            is_main: true,
            extensions: exts,
            create_params: Some(
                deno_core::v8::CreateParams::default()
                    .heap_limits(0 as usize, 1024 * 1024 * 128 as usize),
            ),
            // startup_snapshot: None,
            startup_snapshot: Some(RUNTIME_SNAPSHOT),
            module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
            extension_transpiler: None,
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

        let (log_sender, mut log_receiver) = mpsc::unbounded_channel::<String>();

        {
            let op_state = js_runtime.op_state();
            let mut op_state = op_state.borrow_mut();
            op_state.put(PermissionsContainer {});
            //reqwest client seems to not be sharable between runtimes unfortunately
            // op_state.put(HTTP_CLIENT.clone());
            op_state.put(MainArgs { args: spread });
            op_state.put(LogString { s: log_sender });
        }

        sender
            .send(js_runtime.v8_isolate().thread_safe_handle())
            .map_err(|_| Error::ExecutionErr("impossible to send v8 isolate".to_string()))?;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let future = async {
            use crate::common::merge_result_stream;

            if !extra_logs.is_empty() {
                append_logs(&job_id, w_id_.as_str(), format!("{extra_logs}"), &conn_).await;
            }
            let handle = tokio::spawn(async move {
                let mut logs = String::new();
                while let Some(log) = log_receiver.recv().await {
                    use windmill_common::result_stream::extract_stream_from_logs;

                    if let Some(stream) = extract_stream_from_logs(&log.trim_end_matches("\n")) {
                        // tracing::info!("stream: |{}|", stream);
                        logs.push_str(&stream);
                    }
                    append_logs(&job_id, w_id_.as_str(), log, &conn_).await;
                }
                if !logs.is_empty() {
                    Some(logs)
                } else {
                    None
                }
            });

            let r = tokio::select! {
                r = eval_fetch(&mut js_runtime, &js_expr, Some(env_code), script_entrypoint_override, load_client, &job_id) => Ok(r),
                _ = memory_limit_rx.recv() => Err(Error::ExecutionErr("Memory limit reached, killing isolate".to_string()))
            };
            drop(js_runtime);
            if let Ok(r) = r {
                match handle.await {
                    Ok(Some(logs)) => Ok(merge_result_stream(r, Some(logs)).await),
                    Ok(None) => Ok(r),
                    Err(e) => Err(Error::ExecutionErr(e.to_string())),
                }
            } else {
                r
            }
            // r
        };
        let r = runtime.block_on(future)?;
        // tracing::info!("total: {:?}", instant.elapsed());

        r as windmill_common::error::Result<Box<RawValue>>
    });

    let res = run_future_with_polling_update_job_poller(
        job_id,
        job_timeout,
        conn,
        mem_peak,
        canceled_by,
        async { result_f.await.map_err(windmill_common::error::to_anyhow)? },
        worker_name,
        w_id,
        &mut Some(occupation_metrics),
        Box::pin(futures::stream::once(async { 0 })),
    )
    .await
    .map_err(|e| {
        if let Ok(isolate) = receiver.try_recv() {
            isolate.terminate_execution();
        }
        e
    })?;
    *mem_peak = (res.get().len() / 1000) as i32;
    Ok(res)
}

#[cfg(feature = "deno_core")]
const WINDMILL_CLIENT: &str = include_str!("./windmill-client.js");

#[cfg(feature = "deno_core")]
const ERROR_DIR: &str = const_format::concatcp!(TMP_DIR, "/native_errors");

#[cfg(feature = "deno_core")]
fn write_error_expr(expr: &str, uuid: &Uuid) {
    if let Err(e) = std::fs::create_dir_all(ERROR_DIR) {
        tracing::error!("failed to create error dir {ERROR_DIR}: {e}");
        return;
    }
    let dir_entries = match std::fs::read_dir(ERROR_DIR) {
        Ok(entries) => entries.count(),
        Err(_) => {
            tracing::error!("failed to read error dir {ERROR_DIR}");
            return;
        }
    };

    if std::env::var("PRINT_NATIVE_ERRORS").is_ok() {
        tracing::info!("native error for job {uuid}: {expr}");
    }
    if dir_entries >= 100 {
        tracing::info!("Too many error files in {ERROR_DIR}, skipping write");
        return;
    }

    let path = format!("/{uuid}.js");
    tracing::info!(
        "nativets job {uuid} failed, writing error expr to {ERROR_DIR}/{path} for debugging: {path}"
    );
    if let Err(e) = write_file(ERROR_DIR, &path, expr) {
        tracing::error!("failed to write error expr to file {path}: {e}");
    }
}

#[cfg(feature = "deno_core")]
async fn eval_fetch(
    js_runtime: &mut JsRuntime,
    expr: &str,
    env_code: Option<String>,
    script_entrypoint_override: Option<String>,
    load_client: bool,
    job_id: &Uuid,
) -> windmill_common::error::Result<Box<RawValue>> {
    if load_client {
        if let Some(env_code) = env_code.as_ref() {
            let _ = js_runtime
                .load_side_es_module_from_code(
                    &deno_core::resolve_url("file:///windmill.ts").map_err(error::to_anyhow)?,
                    format!("{env_code}\n{}", WINDMILL_CLIENT.to_string()),
                )
                .await
                .map_err(error::to_anyhow)?;
        }
    }
    use anyhow::Context;
    use deno_core::error::CoreError;
    use windmill_common::{error, worker::to_raw_value};
    let source = format!("{}\n{expr}", env_code.unwrap_or_default());
    let _ = js_runtime
        .load_side_es_module_from_code(
            &deno_core::resolve_url("file:///eval.ts").map_err(error::to_anyhow)?,
            source.to_string(),
        )
        .await
        .map_err(|e| {
            write_error_expr(expr, &job_id);
            e
        })
        .context("failed to load module")?;

    let main_override = script_entrypoint_override.unwrap_or("main".to_string());
    let script = js_runtime
        .execute_script(
            "<anon>",
            format!(
                r#"
function isAsyncIterable(obj) {{
    // return true;    // TODO: remove this
    return obj != null && typeof obj[Symbol.asyncIterator] === 'function';
}}

function processStreamIterative(res) {{
    const iterator = res[Symbol.asyncIterator]();
    
    function processLoop() {{
        return new Promise(function(resolve) {{
            function step() {{
                iterator.next().then(function(result) {{
                    if (!result.done) {{
                        const chunk = result.value;
                        console.log("WM_STREAM: " + chunk.replace('\n', '\\n'));
                        // Continue the loop
                        step();
                    }} else {{
                        resolve("null");
                    }}
                }}).catch(function(error) {{
                    resolve("null");
                }});
            }}
            step();
        }});
    }}
    
    return processLoop();
}}

let args = Deno.core.ops.op_get_static_args().map(JSON.parse)
import("file:///eval.ts").then((module) => module.{main_override}(...args))
    .then(res => {{
        if (isAsyncIterable(res)) {{
            return processStreamIterative(res)
        }} else {{
            return JSON.stringify(res ?? null);
        }}
    }})
"#
            ),
        )
        .map_err(|e| {
            write_error_expr(expr, &job_id);
            e
        })
        .context("native script initialization")?;

    let fut = js_runtime.resolve(script);
    let global = js_runtime
        .with_event_loop_promise(fut, PollEventLoopOptions::default())
        .await
        .map_err(|e| {
            write_error_expr(expr, &job_id);
            e
        });

    match global {
        Ok(global) => {
            let scope = &mut js_runtime.handle_scope();
            let local = v8::Local::new(scope, global);
            // Deserialize a `v8` object into a Rust type using `serde_v8`,
            // in this case deserialize to a JSON `Value`.
            let r = serde_v8::from_v8::<Option<String>>(scope, local).map_err(error::to_anyhow)?;
            Ok(unsafe_raw(r.unwrap_or_else(|| "null".to_string())))
        }
        Err(CoreError::Js(e)) => {
            let stack_head = e.frames.first().and_then(|f| {
                if f.file_name.as_ref().is_some_and(|x| x == "file:///eval.ts") {
                    Some(format!(
                        "{}\n",
                        source
                            .lines()
                            .nth((f.line_number.unwrap_or(1)) as usize - 1)
                            .unwrap_or("")
                            .to_string()
                    ))
                } else {
                    None
                }
            });
            let stack_s = format!(
                "{}{}",
                stack_head.unwrap_or("".to_string()),
                e.stack.unwrap_or("".to_string())
            );
            let stack = if stack_s.is_empty() {
                None
            } else {
                Some(stack_s)
            };
            Err(Error::ExecutionRawError(to_raw_value(&serde_json::json!({
                "message": e.message,
                "stack": stack,
                "name": e.name,
            }))))
        }
        Err(e) => Err(Error::ExecutionErr(e.print_with_cause())),
    }
}

#[cfg(feature = "deno_core")]
#[op2]
#[serde]
fn op_get_static_args(op_state: Rc<RefCell<OpState>>) -> Vec<Option<String>> {
    op_state
        .borrow()
        .borrow::<MainArgs>()
        .args
        .iter()
        .map(|x| x.as_ref().map(|y| y.get().to_string()))
        .collect_vec()
}

#[cfg(feature = "deno_core")]
#[op2(fast)]
fn op_log(op_state: Rc<RefCell<OpState>>, #[string] log: &str) {
    // tracing::error!("log: |{}|", log);
    if let Err(e) = op_state
        .borrow_mut()
        .borrow_mut::<LogString>()
        .s
        .send(log.to_string())
    {
        tracing::error!("failed to send log: {e}");
    }
}

#[cfg(feature = "deno_core")]
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

        let ops = vec![op_get_context()];

        let ext = Extension { name: "js_eval", ops: ops.into(), ..Default::default() };
        let exts = vec![ext];

        let options = RuntimeOptions { extensions: exts, ..Default::default() };

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
            None,
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
        let res = eval(&mut runtime, code, env, None, false, None).await?;
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

        let res = eval_timeout(code.to_string(), env, None, None, None, None).await?;
        assert_eq!(res.get(), "2");
        Ok(())
    }

    // #[tokio::test]
    // async fn test_eval_timeout_bug() -> anyhow::Result<()> {
    //     let ops = vec![op_get_static_args(), op_log()];
    //     let ext = Extension { name: "windmill", ops: ops.into(), ..Default::default() };

    //     let deno_fetch_options = if let Some(cert_path) = env::var("DENO_CERT").ok() {
    //         let mut cert_store_provider = ContainerRootCertStoreProvider::new();
    //         cert_store_provider.add_certificate(cert_path)?;

    //         deno_fetch::Options {
    //             root_cert_store_provider: Some(Arc::new(cert_store_provider)),
    //             ..Default::default()
    //         }
    //     } else {
    //         Default::default()
    //     };

    //     let exts: Vec<Extension> = vec![
    //         deno_webidl::deno_webidl::init_ops(),
    //         deno_url::deno_url::init_ops(),
    //         deno_console::deno_console::init_ops(),
    //         deno_web::deno_web::init_ops::<PermissionsContainer>(
    //             Arc::new(BlobStore::default()),
    //             None,
    //         ),
    //         deno_fetch::deno_fetch::init_ops::<PermissionsContainer>(deno_fetch_options),
    //         deno_net::deno_net::init_ops::<PermissionsContainer>(None, None),
    //         ext,
    //     ];

    //     // Use our snapshot to provision our new runtime
    //     let options = RuntimeOptions {
    //         is_main: true,
    //         extensions: exts,
    //         create_params: Some(
    //             deno_core::v8::CreateParams::default()
    //                 .heap_limits(0 as usize, 1024 * 1024 * 128 as usize),
    //         ),
    //         // startup_snapshot: None,
    //         startup_snapshot: Some(RUNTIME_SNAPSHOT),
    //         module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
    //         extension_transpiler: None,
    //         ..Default::default()
    //     };

    //     let mut js_runtime: JsRuntime = JsRuntime::new(options);
    //     Ok(())
    // }

    // #[tokio::test]
    // async fn test_eval_fetch_timeout() -> anyhow::Result<()> {
    //     let code = r#"export async function main() { return "" }"#;

    //     let res = eval_fetch_timeout(code.to_string(), code.to_string(), None, Uuid::new_v4(), None, ).await?;
    //     assert_eq!(res.0.get(), "\"\"");
    //     Ok(())
    // }
}
