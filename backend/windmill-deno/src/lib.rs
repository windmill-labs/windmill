use anyhow::Result;
use std::{path::PathBuf, sync::Arc};
use windmill_worker::WindmillWorker;

mod module_loader;
mod ops;
pub mod runner;
mod windmill_worker;
mod windmillfs;

fn create_web_worker_callback(
    _ps: deno_cli::proc_state::ProcState,
    _stdio: deno_runtime::deno_io::Stdio,
) -> Arc<deno_runtime::ops::worker_host::CreateWebWorkerCb> {
    // TODO: Implement this based on https://github.com/denoland/deno/blob/d07aa4a0723b04583b7cb1e09152457d866d13d3/cli/worker.rs#L643
    Arc::new(move |_args| todo!("Web worker support"))
}

fn create_web_worker_preload_module_callback() -> Arc<deno_runtime::ops::worker_host::WorkerEventCb>
{
    Arc::new(move |worker| {
        let fut = async move { Ok(worker) };
        futures::task::LocalFutureObj::new(Box::new(fut))
    })
}

fn create_web_worker_pre_execute_module_callback(
    ps: deno_cli::proc_state::ProcState,
) -> Arc<deno_runtime::ops::worker_host::WorkerEventCb> {
    Arc::new(move |mut worker| {
        let ps = ps.clone();
        let fut = async move {
            // this will be up to date after pre-load
            if ps.npm_resolver.has_packages() {
                deno_runtime::deno_node::initialize_runtime(
                    &mut worker.js_runtime,
                    ps.options.has_node_modules_dir(),
                    None,
                )?;
            }

            Ok(worker)
        };
        futures::task::LocalFutureObj::new(Box::new(fut))
    })
}

// Adapted from https://github.com/denoland/deno/blob/d07aa4a0723b04583b7cb1e09152457d866d13d3/cli/worker.rs#L437 with modifications (primarily removing non-deno entrypoint)
async fn create_main_worker(
    ps: &deno_cli::proc_state::ProcState,
    main_module: deno_core::url::Url,
    permissions: deno_runtime::permissions::PermissionsContainer,
    stdio: deno_runtime::deno_io::Stdio,
) -> Result<(deno_core::url::Url, WindmillWorker)> {
    let module_loader = module_loader::WindmillModuleLoader::new(
        ps.clone(),
        deno_runtime::permissions::PermissionsContainer::allow_all(),
        permissions.clone(),
    );

    let create_web_worker_cb = create_web_worker_callback(ps.clone(), stdio.clone());
    let web_worker_preload_module_cb = create_web_worker_preload_module_callback();
    let web_worker_pre_execute_module_cb =
        create_web_worker_pre_execute_module_callback(ps.clone());

    let maybe_storage_key = ps.options.resolve_storage_key(&main_module);
    let origin_storage_dir = maybe_storage_key.as_ref().map(|key| {
        ps.dir
            .origin_data_folder_path()
            .join(deno_cli::util::checksum::gen(&[key.as_bytes()]))
    });
    let cache_storage_dir = maybe_storage_key.map(|key| {
        // DENO_TODO(@satyarohith): storage quota management
        // Note: we currently use temp_dir() to avoid managing storage size.
        std::env::temp_dir()
            .join("deno_cache")
            .join(deno_cli::util::checksum::gen(&[key.as_bytes()]))
    });

    let options = windmill_worker::WorkerOptions {
        bootstrap: deno_runtime::BootstrapOptions {
            args: ps.options.argv().clone(),
            cpu_count: 1,
            debug_flag: false,
            enable_testing_features: ps.options.enable_testing_features(),
            locale: "en".to_owned(),
            location: None,
            no_color: true,
            is_tty: false,
            runtime_version: deno_cli::version::deno().to_string(),
            ts_version: deno_cli::version::TYPESCRIPT.to_string(),
            unstable: true,
            user_agent: format!(
                "Windmill/{}; {}",
                env!("CARGO_PKG_VERSION"),
                deno_cli::version::get_user_agent()
            ),
            inspect: false,
        },
        startup_snapshot: Some(deno_cli::js::deno_isolate_init()),
        unsafely_ignore_certificate_errors: None,
        root_cert_store: Some(ps.root_cert_store.clone()),
        seed: None,
        source_map_getter: Some(Box::new(module_loader.clone())),
        format_js_error_fn: Some(Arc::new(deno_runtime::fmt_errors::format_js_error)),
        create_web_worker_cb,
        web_worker_preload_module_cb,
        web_worker_pre_execute_module_cb,
        module_loader,
        npm_resolver: Some(std::rc::Rc::new(ps.npm_resolver.clone())),
        get_error_class_fn: Some(&deno_cli::errors::get_error_class_name),
        cache_storage_dir,
        origin_storage_dir,
        blob_store: ps.blob_store.clone(),
        broadcast_channel: ps.broadcast_channel.clone(),
        shared_array_buffer_store: Some(ps.shared_array_buffer_store.clone()),
        compiled_wasm_module_store: Some(ps.compiled_wasm_module_store.clone()),
        stdio,
        base_dir: ps.options.initial_cwd().into(),
        env_vars: std::collections::HashMap::new(),
    };

    let worker = WindmillWorker::boostrap_from_options(main_module.clone(), permissions, options);

    Ok((main_module, worker))
}

/// Returns whether to continue
async fn run_once(worker: &mut WindmillWorker) -> anyhow::Result<bool> {
    worker.run_event_loop().await?;

    Ok(worker.dispatch_beforeunload_event(deno_core::located_script_name!())?)
}

async fn pre_run(
    main_module: &deno_core::url::Url,
    worker: &mut WindmillWorker,
    ps: &deno_cli::proc_state::ProcState,
) -> anyhow::Result<()> {
    let id = worker.preload_main_module(&main_module).await?;
    if ps.npm_resolver.has_packages() || ps.graph().has_node_specifier {
        deno_runtime::deno_node::initialize_runtime(&mut worker.js_runtime, false, None)?;
    }
    worker.evaluate_module(id).await?;

    worker.dispatch_load_event(deno_core::located_script_name!())?;

    Ok(())
}

async fn post_run(worker: &mut WindmillWorker) -> anyhow::Result<i32> {
    worker.dispatch_unload_event(deno_core::located_script_name!())?;

    Ok(worker.exit_code())
}

fn make_cli_options(
    flags: deno_cli::args::Flags,
    job_dir: &str,
) -> Result<deno_cli::args::CliOptions> {
    deno_cli::args::CliOptions::new(flags, job_dir.into(), None, None, None)
}

pub struct WatcherPipes {
    #[cfg(unix)]
    // NOTE: stdout & stderr should only be read from!!
    pub stdout: tokio::net::unix::pipe::Receiver,
    #[cfg(unix)]
    pub stderr: tokio::net::unix::pipe::Receiver,
}

async fn prepare_run(
    args: Vec<String>,
    job_dir: &str,
    cache_dir: &str,
    stdio: deno_runtime::deno_io::Stdio,
) -> std::result::Result<
    (
        deno_core::url::Url,
        WindmillWorker,
        deno_cli::proc_state::ProcState,
    ),
    anyhow::Error,
> {
    let mut flags = deno_cli::args::flags_from_vec(args)
        .expect("Args are built by the app and should always be valid");

    flags.cache_path = Some(cache_dir.into());

    deno_cli::util::v8::init_v8_flags(&flags.v8_flags, deno_cli::util::v8::get_v8_flags_from_env());

    let _ = tracing_log::LogTracer::init(); // TODO: I don't think this works. Not really what we want anyways
                                            // deno_cli::util::logger::init(flags.log_level);

    let deno_cli::args::flags::DenoSubcommand::Run(run_flags) = flags.subcommand.clone() else {
        unreachable!("Flags should always be set to run");
    };

    let ps =
        deno_cli::proc_state::ProcState::from_options(Arc::new(make_cli_options(flags, job_dir)?))
            .await?;

    let main_module = deno_core::resolve_url_or_path(&run_flags.script, ps.options.initial_cwd())
        .map_err(deno_core::error::AnyError::from)?;

    let permissions = deno_runtime::permissions::PermissionsContainer::new(
        deno_runtime::permissions::Permissions::from_options(&ps.options.permissions_options())?,
    );

    let (main_module, worker) = create_main_worker(&ps, main_module, permissions, stdio).await?;

    Ok((main_module, worker, ps))
}

pub fn make_stdio(job_dir: &str) -> anyhow::Result<(deno_runtime::deno_io::Stdio, WatcherPipes)> {
    if cfg!(unix) {
        let path: PathBuf = job_dir.into();
        let mut stdout = path.clone();
        stdout.push("stdout.pipe");
        let mut stderr = path;
        stderr.push("stderr.pipe");

        unix_named_pipe::create(stdout.as_path(), None)?;
        unix_named_pipe::create(stderr.as_path(), None)?;

        println!("opening read stdio {stdout:?} {stderr:?}");

        let stderr_reader =
            tokio::net::unix::pipe::OpenOptions::new().open_receiver(stderr.as_path())?;
        let stdout_reader =
            tokio::net::unix::pipe::OpenOptions::new().open_receiver(stdout.as_path())?;

        let stdout_writer = unix_named_pipe::open_write(stdout.as_path())?;
        let stderr_writer = unix_named_pipe::open_write(stderr.as_path())?;

        Ok((
            deno_runtime::deno_io::Stdio {
                stdin: deno_runtime::deno_io::StdioPipe::File(std::fs::File::open("/dev/zero")?),
                stdout: deno_runtime::deno_io::StdioPipe::File(stdout_writer),
                stderr: deno_runtime::deno_io::StdioPipe::File(stderr_writer),
            },
            WatcherPipes { stdout: stdout_reader, stderr: stderr_reader },
        ))
    } else {
        todo!("Cannot create pipes for this target")
    }
}
