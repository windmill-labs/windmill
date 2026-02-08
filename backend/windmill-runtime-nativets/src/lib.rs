/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Isolated deno_core runtime for NativeTS script execution.
//!
//! This crate encapsulates all deno_core/V8 dependencies for executing
//! TypeScript scripts via the nativets runtime. By isolating this here,
//! deno_core compilation no longer blocks windmill-worker or windmill-api.

use std::{
    borrow::Cow,
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

// Re-export deno_telemetry for use by windmill-worker's otel proxy
pub use deno_telemetry;

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
use tokio::sync::mpsc;
use uuid::Uuid;

use windmill_common::error::Error;
use windmill_common::result_stream::append_result_stream_db;
use windmill_common::worker::{write_file, Connection, TMP_DIR};

// ── Permission container ─────────────────────────────────────────────

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

// ── Types ────────────────────────────────────────────────────────────

struct MainArgs {
    args: Vec<Option<Box<RawValue>>>,
}

struct LogString {
    pub s: mpsc::UnboundedSender<String>,
}

pub struct NativeAnnotation {
    pub useragent: Option<String>,
    pub proxy: Option<(String, Option<(String, String)>)>,
}

/// Serializes V8 isolate creation as defense-in-depth against concurrent
/// creation races on x86_64 Linux. The primary fix is using the unprotected
/// V8 platform (see `setup_deno_runtime`).
static V8_ISOLATE_CREATE_LOCK: Mutex<()> = Mutex::new(());

/// Guard that terminates a running V8 isolate when dropped (e.g. on job cancellation).
struct IsolateDropGuard(Arc<Mutex<Option<IsolateHandle>>>);

impl Drop for IsolateDropGuard {
    fn drop(&mut self) {
        if let Some(handle) = self.0.lock().unwrap().take() {
            handle.terminate_execution();
        }
    }
}

// ── Statics ──────────────────────────────────────────────────────────

static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/FETCH_SNAPSHOT.bin"));

const WINDMILL_CLIENT: &str = include_str!("./windmill-client.js");

const ERROR_DIR: &str = const_format::concatcp!(TMP_DIR, "/native_errors");

lazy_static! {
    static ref RE_PROXY: Regex =
        Regex::new(r"^(https?)://(([^:@\s]+):([^:@\s]+)@)?([^:@\s]+)(:(\d+))?$").unwrap();
}

// ── Public interface ─────────────────────────────────────────────────

/// Set up the deno_core/V8 runtime. Idempotent — safe to call multiple times.
/// Called automatically before JsRuntime creation, but can also be called
/// eagerly at startup for predictable initialization order.
pub fn setup_deno_runtime() -> anyhow::Result<()> {
    use std::sync::Once;
    static INIT: Once = Once::new();

    let mut init_err: Option<String> = None;
    INIT.call_once(|| {
        // deno_fetch requires a TLS provider; install ring as default (idempotent).
        let _ = rustls::crypto::ring::default_provider().install_default();

        let unrecognized_v8_flags = deno_core::v8_set_flags(vec![
            "--stack-size=1024".to_string(),
            "--no-harmony-import-assertions".to_string(),
        ])
        .into_iter()
        .skip(1)
        .collect::<Vec<_>>();

        if !unrecognized_v8_flags.is_empty() {
            init_err = Some(format!("Unrecognized V8 flags: {:?}", unrecognized_v8_flags));
        }

        // Use an unprotected platform that doesn't enforce thread-isolated allocations
        // via Memory Protection Keys (pkeys). The default platform requires all V8-using
        // threads to be descendants of the thread that called v8::Initialize, but tokio's
        // spawn_blocking pool threads don't satisfy this. Without this, V8 crashes with
        // SIGSEGV in WasmCodePointerTable::AllocateUninitializedEntry() on x86_64 Linux.
        // See: https://github.com/denoland/deno_core/issues/952
        let platform = deno_core::v8::new_unprotected_default_platform(0, false).make_shared();
        deno_core::JsRuntime::init_platform(Some(platform), false);
    });

    if let Some(msg) = init_err {
        println!("{msg}");
    }
    Ok(())
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

use windmill_common::utils::unsafe_raw;

async fn append_result_stream(
    conn: &Connection,
    workspace_id: &str,
    job_id: &Uuid,
    nstream: &str,
    offset: i32,
) -> windmill_common::error::Result<()> {
    match conn {
        Connection::Sql(db) => {
            append_result_stream_db(db, workspace_id, job_id, nstream, offset).await?;
        }
        Connection::Http(client) => {
            #[derive(serde::Serialize)]
            struct ResultStreamBody<'a> {
                result_stream: &'a str,
                offset: i32,
            }
            let body = ResultStreamBody { result_stream: nstream, offset };
            if let Err(e) = client
                .post::<_, String>(
                    &format!(
                        "/api/w/{}/agent_workers/push_result_stream/{}",
                        workspace_id, job_id
                    ),
                    None,
                    &body,
                )
                .await
            {
                tracing::error!(%job_id, %e, "error sending result stream for job {job_id}: {e}");
            }
        }
    }
    Ok(())
}

// ── ops ──────────────────────────────────────────────────────────────

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

// ── eval_fetch_timeout ───────────────────────────────────────────────

/// Execute a NativeTS script using deno_core/V8.
///
/// Returns `(result, has_stream)` where `has_stream` indicates if the result
/// came from an async iterable stream.
///
/// `otel_initialized` should be `DENO_OTEL_INITIALIZED.load(SeqCst)`.
///
/// The caller (windmill-worker) is responsible for wrapping this in
/// `run_future_with_polling_update_job_poller` for job cancellation/polling.
#[allow(clippy::too_many_arguments)]
pub async fn eval_fetch_timeout(
    env_code: String,
    ts_expr: String,
    js_expr: String,
    args: Option<&Json<std::collections::HashMap<String, Box<RawValue>>>>,
    script_entrypoint_override: Option<String>,
    job_id: Uuid,
    conn: &Connection,
    w_id: &str,
    load_client: bool,
    otel_initialized: bool,
    stream_notifier_update: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
) -> windmill_common::error::Result<(Box<RawValue>, bool)> {
    let isolate_handle: Arc<Mutex<Option<IsolateHandle>>> = Arc::new(Mutex::new(None));
    let _isolate_guard = IsolateDropGuard(isolate_handle.clone());
    let (append_logs_sender, mut append_logs_receiver) = mpsc::unbounded_channel::<String>();
    let (result_stream_sender, mut result_stream_receiver) = mpsc::unbounded_channel::<String>();

    let conn_ = conn.clone();
    let w_id_ = w_id.to_string();
    tokio::spawn(async move {
        while let Some(log) = append_logs_receiver.recv().await {
            windmill_queue::append_logs(&job_id, &w_id_, log, &conn_).await
        }
    });

    let append_result_stream_fn = append_result_stream;
    let conn_ = conn.clone();
    let w_id_ = w_id.to_string();
    tokio::spawn(async move {
        let mut offset = -1;
        while let Some(stream) = result_stream_receiver.recv().await {
            offset += 1;
            if let Err(e) = append_result_stream_fn(&conn_, &w_id_, &job_id, &stream, offset).await
            {
                tracing::error!("failed to append result stream: {e}");
            }
        }
    });

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
                deno_core::v8::CreateParams::default().heap_limits(0, 1024 * 1024 * 128),
            ),
            startup_snapshot: Some(RUNTIME_SNAPSHOT),
            module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
            extension_transpiler: None,
            ..Default::default()
        };

        let (memory_limit_tx, mut memory_limit_rx) = mpsc::unbounded_channel::<()>();

        // Ensure V8 platform is initialized (idempotent, no-op if already done).
        setup_deno_runtime().expect("V8 platform init failed");

        // Serialize isolate creation as extra safety net against concurrent V8
        // isolate creation races. The main fix is the unprotected platform in
        // setup_deno_runtime(), but this provides defense in depth.
        let mut js_runtime = {
            let _v8_lock = V8_ISOLATE_CREATE_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            JsRuntime::new(options)
        };

        // Bootstrap OpenTelemetry for fetch auto-instrumentation if OTEL was initialized.
        if otel_initialized {
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

        *isolate_handle.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(js_runtime.v8_isolate().thread_safe_handle());

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
                let mut is_stream = false;
                while let Some(log) = log_receiver.recv().await {
                    use windmill_common::result_stream::extract_stream_from_logs;

                    if let Some(stream) = extract_stream_from_logs(&log.trim_end_matches("\n")) {
                        if !is_stream {
                            is_stream = true;
                            if let Some(ref f) = stream_notifier_update {
                                f();
                            }
                        }

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
                r = eval_fetch(&mut js_runtime, &js_expr, Some(env_code), script_entrypoint_override, load_client, &job_id, otel_initialized) => Ok(r),
                _ = memory_limit_rx.recv() => Err(Error::ExecutionErr("Memory limit reached, killing isolate".to_string()))
            };
            *isolate_handle.lock().unwrap_or_else(|e| e.into_inner()) = None;
            drop(js_runtime);
            if let Ok(r) = r {
                match handle.await {
                    Ok(Some(logs)) => {
                        // merge_result_stream: if main result is null but stream exists, use stream
                        match r {
                            Ok(raw) if raw.get() == "null" => Ok((unsafe_raw(logs), true)),
                            Ok(raw) => Ok((raw, true)),
                            Err(e) => Err(e),
                        }
                    }
                    Ok(None) => Ok(r.map(|r| (r, false))?),
                    Err(e) => Err(Error::ExecutionErr(e.to_string())),
                }
            } else {
                r.map(|r| r.map(|r| (r, false)))?
            }
        };
        let r = runtime.block_on(future)?;

        Ok(r) as windmill_common::error::Result<(Box<RawValue>, bool)>
    });

    result_f.await.map_err(windmill_common::error::to_anyhow)?
}

async fn eval_fetch(
    js_runtime: &mut JsRuntime,
    expr: &str,
    env_code: Option<String>,
    script_entrypoint_override: Option<String>,
    load_client: bool,
    job_id: &Uuid,
    _otel_initialized: bool,
) -> windmill_common::error::Result<Box<RawValue>> {
    if load_client {
        if let Some(env_code) = env_code.as_ref() {
            let _ = js_runtime
                .load_side_es_module_from_code(
                    &deno_core::resolve_url("file:///windmill.ts")
                        .map_err(windmill_common::error::to_anyhow)?,
                    format!("{env_code}\n{}", WINDMILL_CLIENT.to_string()),
                )
                .await
                .map_err(windmill_common::error::to_anyhow)?;
        }
    }
    use anyhow::Context;
    use deno_core::error::CoreError;
    use windmill_common::worker::to_raw_value;
    let source = format!("{}\n{expr}", env_code.unwrap_or_default());
    let _ = js_runtime
        .load_side_es_module_from_code(
            &deno_core::resolve_url("file:///eval.ts")
                .map_err(windmill_common::error::to_anyhow)?,
            source.to_string(),
        )
        .await
        .map_err(|e| {
            write_error_expr(expr, &job_id);
            e
        })
        .context("failed to load module")?;

    let main_override = script_entrypoint_override.unwrap_or("main".to_string());

    #[cfg(all(feature = "private", feature = "enterprise"))]
    let otel_context_inject = if _otel_initialized {
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
            let r = serde_v8::from_v8::<Option<String>>(scope, local)
                .map_err(windmill_common::error::to_anyhow)?;
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
