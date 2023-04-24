// Much of this code is adapted from https://github.com/HurricanKai/deno/blob/f90caa821c5a4acf28f7dec4071994ecf6f26e57/runtime/worker.rs

use std::{
    collections::HashMap,
    path::PathBuf,
    pin::Pin,
    rc::Rc,
    sync::{atomic::AtomicI32, Arc},
    task::{Context, Poll},
};

use deno_core::{
    ascii_str, error::AnyError, v8, CompiledWasmModuleStore, GetErrorClassFn, JsRuntime,
    ModuleCode, ModuleId, ModuleLoader, ModuleSpecifier, RuntimeOptions, SharedArrayBufferStore,
    Snapshot, SourceMapGetter,
};
use deno_runtime::{
    deno_broadcast_channel::{self, InMemoryBroadcastChannel},
    deno_cache::{self, CreateCache, SqliteBackedCache},
    deno_console, deno_crypto, deno_fetch, deno_ffi, deno_fs, deno_http,
    deno_io::{self, Stdio},
    deno_kv::{self, sqlite::SqliteDbHandler},
    deno_napi, deno_net,
    deno_node::{self, RequireNpmResolver},
    deno_tls::{self, rustls::RootCertStore},
    deno_url,
    deno_web::{self, BlobStore},
    deno_webidl, deno_websocket, deno_webstorage, ops,
    permissions::PermissionsContainer,
    worker::FormatJsErrorFn,
    BootstrapOptions,
};
use futures::Future;
use tracing_log::log::debug;

use crate::windmillfs::WindmillFs;

#[derive(Clone, Default)]
pub struct ExitCode(Arc<AtomicI32>);

impl ExitCode {
    pub fn get(&self) -> i32 {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set(&mut self, code: i32) {
        self.0.store(code, std::sync::atomic::Ordering::Relaxed);
    }
}

// Fine-tuned replacement for deno_runtime::worker::MainWorker see https://github.com/HurricanKai/deno/blob/f90caa821c5a4acf28f7dec4071994ecf6f26e57/runtime/worker.rs#L67
pub struct WindmillWorker {
    pub js_runtime: JsRuntime,
    exit_code: ExitCode,
}

pub struct WorkerOptions {
    pub base_dir: PathBuf,

    pub bootstrap: BootstrapOptions,

    /// JsRuntime extensions, not to be confused with ES modules.
    ///
    /// Extensions register "ops" and JavaScript sources provided in `js` or `esm`
    /// configuration. If you are using a snapshot, then extensions shouldn't
    /// provide JavaScript sources that were already snapshotted.
    // pub extensions: Vec<Extension>,

    /// V8 snapshot that should be loaded on startup.
    pub startup_snapshot: Option<Snapshot>,
    pub unsafely_ignore_certificate_errors: Option<Vec<String>>,
    pub root_cert_store: Option<RootCertStore>,
    pub seed: Option<u64>,

    /// Implementation of `ModuleLoader` which will be
    /// called when V8 requests to load ES modules.
    ///
    /// If not provided runtime will error if code being
    /// executed tries to load modules.
    pub module_loader: Rc<dyn ModuleLoader>,
    pub npm_resolver: Option<Rc<dyn RequireNpmResolver>>,
    // Callbacks invoked when creating new instance of WebWorker
    pub create_web_worker_cb: Arc<ops::worker_host::CreateWebWorkerCb>,
    pub web_worker_preload_module_cb: Arc<ops::worker_host::WorkerEventCb>,
    pub web_worker_pre_execute_module_cb: Arc<ops::worker_host::WorkerEventCb>,
    pub format_js_error_fn: Option<Arc<FormatJsErrorFn>>,

    /// Source map reference for errors.
    pub source_map_getter: Option<Box<dyn SourceMapGetter>>,

    /// Allows to map error type to a string "class" used to represent
    /// error in JavaScript.
    pub get_error_class_fn: Option<GetErrorClassFn>,
    pub cache_storage_dir: Option<std::path::PathBuf>,
    pub origin_storage_dir: Option<std::path::PathBuf>,
    pub blob_store: BlobStore,
    pub broadcast_channel: InMemoryBroadcastChannel,

    /// The store to use for transferring SharedArrayBuffers between isolates.
    /// If multiple isolates should have the possibility of sharing
    /// SharedArrayBuffers, they should use the same [SharedArrayBufferStore]. If
    /// no [SharedArrayBufferStore] is specified, SharedArrayBuffer can not be
    /// serialized.
    pub shared_array_buffer_store: Option<SharedArrayBufferStore>,

    /// The store to use for transferring `WebAssembly.Module` objects between
    /// isolates.
    /// If multiple isolates should have the possibility of sharing
    /// `WebAssembly.Module` objects, they should use the same
    /// [CompiledWasmModuleStore]. If no [CompiledWasmModuleStore] is specified,
    /// `WebAssembly.Module` objects cannot be serialized.
    pub compiled_wasm_module_store: Option<CompiledWasmModuleStore>,
    pub stdio: Stdio,
    pub env_vars: HashMap<String, String>,
}

impl WindmillWorker {
    pub fn boostrap_from_options(
        main_module: ModuleSpecifier,
        permissions: PermissionsContainer,
        mut options: WorkerOptions,
    ) -> Self {
        deno_core::extension!(deno_permissions_worker,
          options = {
            permissions: PermissionsContainer,
            unstable: bool,
            enable_testing_features: bool,
          },
          state = |state, options| {
            state.put::<PermissionsContainer>(options.permissions);
            state.put(ops::UnstableChecker { unstable: options.unstable });
            state.put(ops::TestingFeaturesEnabled(options.enable_testing_features));
          },
        );

        // Permissions: many ops depend on this
        let unstable = options.bootstrap.unstable;
        let enable_testing_features = options.bootstrap.enable_testing_features;
        let exit_code = ExitCode(Arc::new(AtomicI32::new(0)));
        let create_cache = options.cache_storage_dir.map(|storage_dir| {
            let create_cache_fn = move || SqliteBackedCache::new(storage_dir.clone());
            CreateCache(Arc::new(create_cache_fn))
        });

        // NOTE(bartlomieju): ordering is important here, keep it in sync with
        // `runtime/build.rs`, `runtime/web_worker.rs` and `cli/build.rs`!
        let mut extensions = vec![
            // Web APIs
            deno_webidl::deno_webidl::init_ops(),
            deno_console::deno_console::init_ops(),
            deno_url::deno_url::init_ops(),
            deno_web::deno_web::init_ops::<PermissionsContainer>(
                options.blob_store.clone(),
                options.bootstrap.location.clone(),
            ),
            deno_fetch::deno_fetch::init_ops::<PermissionsContainer>(deno_fetch::Options {
                user_agent: options.bootstrap.user_agent.clone(),
                root_cert_store: options.root_cert_store.clone(),
                unsafely_ignore_certificate_errors: options
                    .unsafely_ignore_certificate_errors
                    .clone(),
                file_fetch_handler: Rc::new(deno_fetch::FsFetchHandler),
                ..Default::default()
            }),
            deno_cache::deno_cache::init_ops::<SqliteBackedCache>(create_cache),
            deno_websocket::deno_websocket::init_ops::<PermissionsContainer>(
                options.bootstrap.user_agent.clone(),
                options.root_cert_store.clone(),
                options.unsafely_ignore_certificate_errors.clone(),
            ),
            deno_webstorage::deno_webstorage::init_ops(options.origin_storage_dir.clone()),
            deno_crypto::deno_crypto::init_ops(options.seed),
            deno_broadcast_channel::deno_broadcast_channel::init_ops(
                options.broadcast_channel.clone(),
                unstable,
            ),
            deno_ffi::deno_ffi::init_ops::<PermissionsContainer>(unstable),
            deno_net::deno_net::init_ops::<PermissionsContainer>(
                options.root_cert_store.clone(),
                unstable,
                options.unsafely_ignore_certificate_errors.clone(),
            ),
            deno_tls::deno_tls::init_ops(),
            deno_kv::deno_kv::init_ops(
                SqliteDbHandler::<PermissionsContainer>::new(options.origin_storage_dir.clone()),
                unstable,
            ),
            deno_napi::deno_napi::init_ops::<PermissionsContainer>(),
            deno_http::deno_http::init_ops(),
            deno_io::deno_io::init_ops(Some(options.stdio)),
            deno_fs::deno_fs::init_ops::<_, PermissionsContainer>(
                unstable,
                WindmillFs::new(options.base_dir),
            ),
            deno_node::deno_node::init_ops::<deno_runtime::RuntimeNodeEnv>(options.npm_resolver),
            // Ops from this crate
            ops::runtime::deno_runtime::init_ops(main_module.clone()),
            ops::worker_host::deno_worker_host::init_ops(
                options.create_web_worker_cb.clone(),
                options.web_worker_preload_module_cb.clone(),
                options.web_worker_pre_execute_module_cb.clone(),
                options.format_js_error_fn.clone(),
            ),
            ops::fs_events::deno_fs_events::init_ops(),
            crate::ops::os::deno_os::init_ops(exit_code.clone(), options.env_vars),
            ops::permissions::deno_permissions::init_ops(),
            ops::process::deno_process::init_ops(),
            ops::signal::deno_signal::init_ops(),
            ops::tty::deno_tty::init_ops(),
            ops::http::deno_http_runtime::init_ops(),
            deno_permissions_worker::init_ops(permissions, unstable, enable_testing_features),
        ];

        #[cfg(not(feature = "dont_create_runtime_snapshot"))]
        let startup_snapshot = options
            .startup_snapshot
            .unwrap_or_else(deno_cli::js::deno_isolate_init);
        #[cfg(feature = "dont_create_runtime_snapshot")]
        let startup_snapshot = options.startup_snapshot
            .expect("deno_runtime startup snapshot is not available with 'create_runtime_snapshot' Cargo feature.");

        let mut js_runtime = JsRuntime::new(RuntimeOptions {
            module_loader: Some(options.module_loader.clone()),
            startup_snapshot: Some(startup_snapshot),
            source_map_getter: options.source_map_getter,
            get_error_class_fn: options.get_error_class_fn,
            shared_array_buffer_store: options.shared_array_buffer_store.clone(),
            compiled_wasm_module_store: options.compiled_wasm_module_store.clone(),
            extensions,
            inspector: false,
            is_main: true,
            ..Default::default()
        });

        let bootstrap_fn_global = {
            let context = js_runtime.global_context();
            let scope = &mut js_runtime.handle_scope();
            let context_local = v8::Local::new(scope, context);
            let global_obj = context_local.global(scope);
            let bootstrap_str =
                v8::String::new_external_onebyte_static(scope, b"bootstrap").unwrap();
            let bootstrap_ns: v8::Local<v8::Object> = global_obj
                .get(scope, bootstrap_str.into())
                .unwrap()
                .try_into()
                .unwrap();
            let main_runtime_str =
                v8::String::new_external_onebyte_static(scope, b"mainRuntime").unwrap();
            let bootstrap_fn = bootstrap_ns.get(scope, main_runtime_str.into()).unwrap();
            let bootstrap_fn = v8::Local::<v8::Function>::try_from(bootstrap_fn).unwrap();
            v8::Global::new(scope, bootstrap_fn)
        };

        // Bootstrap
        {
            let scope = &mut js_runtime.handle_scope();
            let args = options.bootstrap.as_v8(scope);
            let bootstrap_fn = v8::Local::new(scope, bootstrap_fn_global);
            let undefined = v8::undefined(scope);
            bootstrap_fn
                .call(scope, undefined.into(), &[args.into()])
                .unwrap();
        }

        Self { js_runtime, exit_code }
    }

    /// See [JsRuntime::execute_script](deno_core::JsRuntime::execute_script)
    pub fn execute_script(
        &mut self,
        script_name: &'static str,
        source_code: ModuleCode,
    ) -> Result<v8::Global<v8::Value>, AnyError> {
        self.js_runtime.execute_script(script_name, source_code)
    }

    /// Loads and instantiates specified JavaScript module as "main" module.
    pub async fn preload_main_module(
        &mut self,
        module_specifier: &ModuleSpecifier,
    ) -> Result<ModuleId, AnyError> {
        self.js_runtime
            .load_main_module(module_specifier, None)
            .await
    }

    /// Loads and instantiates specified JavaScript module as "side" module.
    pub async fn preload_side_module(
        &mut self,
        module_specifier: &ModuleSpecifier,
    ) -> Result<ModuleId, AnyError> {
        self.js_runtime
            .load_side_module(module_specifier, None)
            .await
    }

    /// Executes specified JavaScript module.
    pub async fn evaluate_module(&mut self, id: ModuleId) -> Result<(), AnyError> {
        let mut receiver = self.js_runtime.mod_evaluate(id);
        tokio::select! {
          // Not using biased mode leads to non-determinism for relatively simple
          // programs.
          biased;

          maybe_result = &mut receiver => {
            debug!("received module evaluate {:#?}", maybe_result);
            maybe_result.expect("Module evaluation result not provided.")
          }

          event_loop_result = self.run_event_loop() => {
            event_loop_result?;
            let maybe_result = receiver.await;
            maybe_result.expect("Module evaluation result not provided.")
          }
        }
    }

    /// Loads, instantiates and executes specified JavaScript module.
    pub async fn execute_side_module(
        &mut self,
        module_specifier: &ModuleSpecifier,
    ) -> Result<(), AnyError> {
        let id = self.preload_side_module(module_specifier).await?;
        self.evaluate_module(id).await
    }

    /// Loads, instantiates and executes specified JavaScript module.
    ///
    /// This module will have "import.meta.main" equal to true.
    pub async fn execute_main_module(
        &mut self,
        module_specifier: &ModuleSpecifier,
    ) -> Result<(), AnyError> {
        let id = self.preload_main_module(module_specifier).await?;
        self.evaluate_module(id).await
    }

    pub fn poll_event_loop(&mut self, cx: &mut Context) -> Poll<Result<(), AnyError>> {
        self.js_runtime.poll_event_loop(cx, false)
    }

    pub async fn run_event_loop(&mut self) -> Result<(), AnyError> {
        self.js_runtime.run_event_loop(false).await
    }

    /// A utility function that runs provided future concurrently with the event loop.
    ///
    /// Useful when using a local inspector session.
    pub async fn with_event_loop<'a, T>(
        &mut self,
        mut fut: Pin<Box<dyn Future<Output = T> + 'a>>,
    ) -> T {
        loop {
            tokio::select! {
              biased;
              result = &mut fut => {
                return result;
              }
              _ = self.run_event_loop() => {}
            };
        }
    }

    /// Return exit code set by the executed code (either in main worker
    /// or one of child web workers).
    pub fn exit_code(&self) -> i32 {
        self.exit_code.get()
    }

    /// Dispatches "load" event to the JavaScript runtime.
    ///
    /// Does not poll event loop, and thus not await any of the "load" event handlers.
    pub fn dispatch_load_event(&mut self, script_name: &'static str) -> Result<(), AnyError> {
        self.js_runtime.execute_script(
            script_name,
            // NOTE(@bartlomieju): not using `globalThis` here, because user might delete
            // it. Instead we're using global `dispatchEvent` function which will
            // used a saved reference to global scope.
            ascii_str!("dispatchEvent(new Event('load'))"),
        )?;
        Ok(())
    }

    /// Dispatches "unload" event to the JavaScript runtime.
    ///
    /// Does not poll event loop, and thus not await any of the "unload" event handlers.
    pub fn dispatch_unload_event(&mut self, script_name: &'static str) -> Result<(), AnyError> {
        self.js_runtime.execute_script(
            script_name,
            // NOTE(@bartlomieju): not using `globalThis` here, because user might delete
            // it. Instead we're using global `dispatchEvent` function which will
            // used a saved reference to global scope.
            ascii_str!("dispatchEvent(new Event('unload'))"),
        )?;
        Ok(())
    }

    /// Dispatches "beforeunload" event to the JavaScript runtime. Returns a boolean
    /// indicating if the event was prevented and thus event loop should continue
    /// running.
    pub fn dispatch_beforeunload_event(
        &mut self,
        script_name: &'static str,
    ) -> Result<bool, AnyError> {
        let value = self.js_runtime.execute_script(
            script_name,
            // NOTE(@bartlomieju): not using `globalThis` here, because user might delete
            // it. Instead we're using global `dispatchEvent` function which will
            // used a saved reference to global scope.
            ascii_str!("dispatchEvent(new Event('beforeunload', { cancelable: true }));"),
        )?;
        let local_value = value.open(&mut self.js_runtime.handle_scope());
        Ok(local_value.is_false())
    }
}
