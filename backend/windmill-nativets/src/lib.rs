/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! This crate isolates deno_core and its heavy transitive dependencies (V8)
//! for NativeTS script execution. Flow expression evaluation uses QuickJS
//! which is in windmill-worker and doesn't require this crate.

use std::{borrow::Cow, cell::RefCell, env, path::PathBuf, rc::Rc, sync::Arc};

use deno_ast::ParseParams;
use deno_core::{
    op2, serde_v8, url,
    v8::{self, IsolateHandle},
    Extension, JsRuntime, OpState, PollEventLoopOptions, RuntimeOptions,
};
use deno_fetch::FetchPermissions;
use deno_net::NetPermissions;
use deno_web::{BlobStore, TimersPermission};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::value::RawValue;
use sqlx::types::Json;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use windmill_common::error::Error;
use windmill_common::worker::{write_file, TMP_DIR};

pub struct PermissionsContainer;

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

impl TimersPermission for PermissionsContainer {
    #[inline(always)]
    fn allow_hrtime(&mut self) -> bool {
        true
    }
}

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

lazy_static! {
    static ref RE_PROXY: Regex =
        Regex::new(r"^(https?)://(([^:@\s]+):([^:@\s]+)@)?([^:@\s]+)(:(\d+))?$").unwrap();
}

pub struct MainArgs {
    args: Vec<Option<Box<RawValue>>>,
}

pub struct LogString {
    pub s: mpsc::UnboundedSender<String>,
}

pub struct NativeAnnotation {
    pub useragent: Option<String>,
    pub proxy: Option<(String, Option<(String, String)>)>,
}

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

static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/FETCH_SNAPSHOT.bin"));

const WINDMILL_CLIENT: &str = include_str!("./windmill-client.js");

const ERROR_DIR: &str = const_format::concatcp!(TMP_DIR, "/native_errors");

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

/// Helper to create an unsafe raw value from a string
/// (The string is assumed to be valid JSON)
pub fn unsafe_raw(json: String) -> Box<RawValue> {
    unsafe { std::mem::transmute::<Box<str>, Box<RawValue>>(json.into_boxed_str()) }
}

/// Result of NativeTS execution, containing the result and whether it produced a stream
pub struct NativeTsResult {
    pub result: Box<RawValue>,
    pub has_stream: bool,
}

/// Channels for receiving logs and result streams from the NativeTS execution
pub struct NativeTsChannels {
    pub logs_receiver: mpsc::UnboundedReceiver<String>,
    pub result_stream_receiver: mpsc::UnboundedReceiver<String>,
}

/// Configuration for OpenTelemetry (enterprise feature)
#[cfg(all(feature = "private", feature = "enterprise"))]
pub static DENO_OTEL_INITIALIZED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Execute a NativeTS script.
///
/// This is a lower-level function that returns a JoinHandle for the blocking task.
/// The caller is responsible for wrapping this with job polling and cancellation logic.
///
/// Returns:
/// - A oneshot receiver for the isolate handle (to terminate execution if needed)
/// - Channels for receiving logs and result streams
/// - A JoinHandle for the blocking execution task
pub async fn eval_fetch_timeout(
    env_code: String,
    ts_expr: String,
    js_expr: String,
    args: Option<&Json<HashMap<String, Box<RawValue>>>>,
    script_entrypoint_override: Option<String>,
    job_id: Uuid,
) -> windmill_common::error::Result<(
    oneshot::Receiver<IsolateHandle>,
    NativeTsChannels,
    tokio::task::JoinHandle<windmill_common::error::Result<(Box<RawValue>, bool)>>,
    String, // extra_logs
)> {
    let (sender, receiver) = oneshot::channel::<IsolateHandle>();
    let (append_logs_sender, append_logs_receiver) = mpsc::unbounded_channel::<String>();
    let (result_stream_sender, result_stream_receiver) = mpsc::unbounded_channel::<String>();

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

    // Clone extra_logs since it's used both in the closure and returned
    let extra_logs_return = extra_logs.clone();

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

        let options = RuntimeOptions {
            is_main: true,
            extensions: exts,
            create_params: Some(
                deno_core::v8::CreateParams::default()
                    .heap_limits(0 as usize, 1024 * 1024 * 128 as usize),
            ),
            startup_snapshot: Some(RUNTIME_SNAPSHOT),
            module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
            extension_transpiler: None,
            ..Default::default()
        };

        let (memory_limit_tx, mut memory_limit_rx) = mpsc::unbounded_channel::<()>();

        let mut js_runtime: JsRuntime = JsRuntime::new(options);

        // Bootstrap OpenTelemetry for fetch auto-instrumentation if OTEL was initialized.
        #[cfg(all(feature = "private", feature = "enterprise"))]
        if DENO_OTEL_INITIALIZED.load(std::sync::atomic::Ordering::SeqCst) {
            if let Err(e) =
                js_runtime.execute_script("<otel_bootstrap>", "globalThis.__bootstrapOtel()")
            {
                tracing::warn!("Failed to bootstrap OTEL telemetry: {}", e);
            }
        }

        js_runtime.add_near_heap_limit_callback(move |x, y| {
            tracing::error!("heap limit reached: {x} {y}");

            if memory_limit_tx.send(()).is_err() {
                tracing::error!("failed to send memory limit reached notification - isolate may already be terminating");
            };
            y * 2
        });

        let (log_sender, mut log_receiver) = mpsc::unbounded_channel::<String>();

        {
            let op_state = js_runtime.op_state();
            let mut op_state = op_state.borrow_mut();
            op_state.put(PermissionsContainer {});
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
            if !extra_logs.is_empty() {
                if let Err(e) = append_logs_sender.send(extra_logs) {
                    tracing::error!("failed to send extra logs: {e}");
                }
            }
            let handle = tokio::spawn(async move {
                let mut result_stream = String::new();
                while let Some(log) = log_receiver.recv().await {
                    use windmill_common::result_stream::extract_stream_from_logs;

                    if let Some(stream) = extract_stream_from_logs(&log.trim_end_matches("\n")) {
                        result_stream.push_str(&stream);
                        if let Err(e) = result_stream_sender.send(stream) {
                            tracing::error!("failed to send result stream: {e}");
                        }
                    } else {
                        if let Err(e) = append_logs_sender.send(log) {
                            tracing::error!("failed to send log: {e}");
                        }
                    }
                }
                if !result_stream.is_empty() {
                    Some(result_stream)
                } else {
                    None
                }
            });

            let r = tokio::select! {
                r = eval_fetch(&mut js_runtime, &js_expr, Some(env_code), script_entrypoint_override, true, &job_id) => Ok(r),
                _ = memory_limit_rx.recv() => Err(Error::ExecutionErr("Memory limit reached, killing isolate".to_string()))
            };
            drop(js_runtime);
            if let Ok(r) = r {
                match handle.await {
                    Ok(Some(logs)) => {
                        Ok(merge_result_stream(r, Some(logs)).await.map(|r| (r, true)))
                    }
                    Ok(None) => Ok(r.map(|r| (r, false))),
                    Err(e) => Err(Error::ExecutionErr(e.to_string())),
                }
            } else {
                r.map(|r| r.map(|r| (r, false)))
            }
        };
        let r = runtime.block_on(future)?;

        r as windmill_common::error::Result<(Box<RawValue>, bool)>
    });

    Ok((
        receiver,
        NativeTsChannels {
            logs_receiver: append_logs_receiver,
            result_stream_receiver,
        },
        result_f,
        extra_logs_return,
    ))
}

async fn merge_result_stream(
    r: windmill_common::error::Result<Box<RawValue>>,
    stream: Option<String>,
) -> windmill_common::error::Result<Box<RawValue>> {
    if let Some(stream) = stream {
        if let Ok(ref result) = r {
            if result.get() == "null" {
                return Ok(unsafe_raw(format!("\"{}\"", stream)));
            }
        }
    }
    r
}

async fn eval_fetch(
    js_runtime: &mut JsRuntime,
    expr: &str,
    env_code: Option<String>,
    script_entrypoint_override: Option<String>,
    load_client: bool,
    job_id: &Uuid,
) -> windmill_common::error::Result<Box<RawValue>> {
    use anyhow::Context;
    use deno_core::error::CoreError;
    use windmill_common::error;
    use windmill_common::worker::to_raw_value;

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

    // Inject parent trace context using enterSpan with a duck-typed span object.
    #[cfg(all(feature = "private", feature = "enterprise"))]
    let otel_context_inject = if DENO_OTEL_INITIALIZED.load(std::sync::atomic::Ordering::SeqCst) {
        let trace_id = job_id.as_simple().to_string();
        format!(
r#"globalThis.__enterSpan?.({{
    isRecording: () => true,
    spanContext: () => ({{ traceId: "{trace_id}", spanId: "ffffffffffffffff", traceFlags: 1 }})
}});"#
        )
    } else {
        String::new()
    };

    #[cfg(not(all(feature = "private", feature = "enterprise")))]
    let otel_context_inject = "";

    let script = js_runtime
        .execute_script(
            "<anon>",
            format!(
                r#"
function isAsyncIterable(obj) {{
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
                        console.log("WM_STREAM: " + chunk.replace(/\n/g, '\\n'));
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

{otel_context_inject}

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

#[op2(fast)]
fn op_log(op_state: Rc<RefCell<OpState>>, #[string] log: &str) {
    if let Err(e) = op_state
        .borrow_mut()
        .borrow_mut::<LogString>()
        .s
        .send(log.to_string())
    {
        tracing::error!("failed to send log: {e}");
    }
}
