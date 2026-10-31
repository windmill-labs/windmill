use serde_json::value::RawValue;
#[cfg(not(windows))]
use std::sync::Once;
use std::{collections::HashMap, process::Stdio};
use uuid::Uuid;
use windmill_parser_rust::parse_rust_deps_into_manifest;

use crate::global_cache::save_cache;
use itertools::Itertools;
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncReadExt,
    process::Command,
};
use windmill_common::{
    error::{self, Error},
    worker::{write_file, Connection},
};
use windmill_queue::MiniPulledJob;
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    common::{
        build_command_with_isolation, check_executor_binary_exists, create_args_and_out_file,
        get_reserved_variables, read_result, resolve_nsjail_timeout,
        resolve_nsjail_tmp_mount_block, script_envars, start_child_process, OccupancyMetrics,
        DEV_CONF_NSJAIL,
    },
    get_proxy_envs_for_lang,
    handle_child::handle_child,
    is_sandboxing_enabled, read_ee_registry_with_workspace_override, CARGO_REGISTRIES,
    DISABLE_NUSER, HOME_ENV, NSJAIL_PATH, PATH_ENV, PROXY_ENVS, RUST_CACHE_DIR,
    TRACING_PROXY_CA_CERT_PATH, TZ_ENV,
};
use windmill_common::client::AuthedClient;
use windmill_common::scripts::ScriptLang;

#[cfg(windows)]
use crate::SYSTEM_ROOT;

const NSJAIL_CONFIG_RUN_RUST_CONTENT: &str = include_str!("../nsjail/run.rust.config.proto");
const NSJAIL_CONFIG_COMPILE_RUST_CONTENT: &str =
    include_str!("../nsjail/download.rust.config.proto");

#[cfg(windows)]
const RUST_BIN_NAME: &str = "main.exe";
#[cfg(not(windows))]
const RUST_BIN_NAME: &str = "main";

fn find_cargo_path() -> String {
    if let Ok(p) = std::env::var("CARGO_PATH") {
        return p;
    }
    let candidates = if cfg!(windows) {
        vec![format!("{}\\bin\\cargo.exe", CARGO_HOME.as_str())]
    } else {
        vec![
            format!("{}/bin/cargo", CARGO_HOME.as_str()),
            "/usr/local/cargo/bin/cargo".to_string(),
            "/usr/bin/cargo".to_string(),
        ]
    };
    for p in &candidates {
        if std::path::Path::new(p).exists() {
            return p.clone();
        }
    }
    candidates.into_iter().next().unwrap()
}

#[cfg(not(windows))]
fn find_preinstalled_dir(env_var: &str, candidates: &[&str]) -> String {
    if let Ok(p) = std::env::var(env_var) {
        return p;
    }
    for c in candidates {
        if std::path::Path::new(c).exists() {
            return c.to_string();
        }
    }
    candidates[0].to_string()
}

lazy_static::lazy_static! {
    static ref CARGO_HOME: String = std::env::var("CARGO_HOME").unwrap_or_else(|_| { CARGO_HOME_DEFAULT.clone() });
    static ref RUSTUP_HOME: String = std::env::var("RUSTUP_HOME").unwrap_or_else(|_| { RUSTUP_HOME_DEFAULT.clone() });
    static ref CARGO_PATH: String = find_cargo_path();
    static ref SWEEP_MAXSIZE: String = std::env::var("CARGO_SWEEP_MAXSIZE").unwrap_or("25GB".to_owned());
    static ref NO_SHARED_BUILD_DIR: bool = std::env::var("RUST_NO_SHARED_BUILD_DIR").ok().map(|flag| flag == "true").unwrap_or(false);
    /// Parent of the worker-owned working directories of host-side cargo invocations, see [`HostCargoCwd`].
    static ref HOST_CARGO_CWD_DIR: String = format!("{}/host_cargo_cwd", *RUST_CACHE_DIR);
}

#[cfg(windows)]
lazy_static::lazy_static! {
    static ref CARGO_HOME_DEFAULT: String = format!("{}\\.cargo", HOME_ENV.as_str());
    static ref RUSTUP_HOME_DEFAULT: String = format!("{}\\.rustup", HOME_ENV.as_str());
}

#[cfg(not(windows))]
lazy_static::lazy_static! {
    static ref CARGO_HOME_DEFAULT: String = format!("{}/.cargo", HOME_ENV.as_str());
    static ref RUSTUP_HOME_DEFAULT: String = format!("{}/.rustup", HOME_ENV.as_str());
}

const RUST_OBJECT_STORE_PREFIX: &str =
    const_format::concatcp!(crate::global_cache::TARGET, "_rustbin/");

#[cfg(not(windows))]
lazy_static::lazy_static! {
    static ref PREINSTALLED_CARGO: String = find_preinstalled_dir(
        "CARGO_PREINSTALL_DIR",
        &["/usr/local/cargo", &format!("{}/.cargo", HOME_ENV.as_str())],
    );
    static ref PREINSTALLED_RUSTUP: String = find_preinstalled_dir(
        "RUSTUP_PREINSTALL_DIR",
        &["/usr/local/rustup", &format!("{}/.rustup", HOME_ENV.as_str())],
    );
}

#[cfg(not(windows))]
static RUST_DIRS_INIT: Once = Once::new();

#[cfg(not(windows))]
fn symlink_preinstalled_entries(preinstalled: &str, target: &str) {
    use std::fs;
    use std::os::unix::fs as unix_fs;
    use std::path::Path;

    if target == preinstalled || !Path::new(preinstalled).exists() {
        return;
    }
    let _ = fs::create_dir_all(target);
    let Ok(entries) = fs::read_dir(preinstalled) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let link_path = Path::new(target).join(&name);
        if !link_path.exists() {
            let _ = unix_fs::symlink(entry.path(), &link_path);
        }
    }
}

#[cfg(not(windows))]
fn symlink_single_entry(preinstalled: &str, target: &str, name: &str) {
    use std::os::unix::fs as unix_fs;
    use std::path::Path;

    let src = Path::new(preinstalled).join(name);
    if !src.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(target);
    let dst = Path::new(target).join(name);
    if !dst.exists() {
        let _ = unix_fs::symlink(&src, &dst);
    }
}

#[cfg(not(windows))]
fn ensure_rust_runtime_dirs() {
    RUST_DIRS_INIT.call_once(|| {
        // Only symlink bin/ from cargo (registry/git must be writable)
        symlink_single_entry(&PREINSTALLED_CARGO, CARGO_HOME.as_str(), "bin");
        // Symlink all entries from rustup (toolchains, settings.toml, etc.)
        symlink_preinstalled_entries(&PREINSTALLED_RUSTUP, RUSTUP_HOME.as_str());
    });
}

#[cfg(windows)]
fn ensure_rust_runtime_dirs() {}

fn gen_cargo_crate(code: &str, job_dir: &str) -> anyhow::Result<()> {
    let manifest = parse_rust_deps_into_manifest(code)?;
    write_file(job_dir, "Cargo.toml", &manifest)?;

    const WRAPPER_CONTENT: &str = r#"
use std::fs::File;
use std::io::{BufReader, Write};
use std::error::Error;
mod inner;

fn main() -> Result<(), Box<dyn Error>> {
    let args_file = File::open("args.json")?;
    let reader = BufReader::new(args_file);
    let args: inner::__WINDMILL_ARGS__ = serde_json::from_reader(reader)?;

    let result = inner::__WINDMILL_RUN__(args)?;

    let mut result_file = File::create("result.json")?;
    result_file.write_all(result.into_bytes().as_ref())?;

    Ok(())
}
"#;
    write_file(job_dir, "main.rs", WRAPPER_CONTENT)?;

    let sig = windmill_parser_rust::parse_rust_signature(code)?;
    let spread = &sig
        .args
        .clone()
        .into_iter()
        .map(|x| format!("_args.{}", &x.name))
        .join(", ");
    let arg_struct_body = &sig
        .args
        .into_iter()
        .map(|x| {
            format!(
                "{}: {},",
                &x.name,
                windmill_parser_rust::otyp_to_string(x.otyp),
            )
        })
        .join("\n");
    let mod_content: String = format!(
        r#"
{code}

#[derive(serde::Deserialize)]
#[allow(non_camel_case_types)]
pub struct __WINDMILL_ARGS__ {{
    {arg_struct_body}
}}


#[allow(non_snake_case)]
pub fn __WINDMILL_RUN__(_args: __WINDMILL_ARGS__) -> Result<String, Box<dyn std::error::Error>> {{
    fn windmill_runner<T, E>(f: impl core::ops::FnOnce() -> core::result::Result<T,E>) -> Result<String, E>
    where
        T: serde::Serialize,
        E: std::fmt::Display,
    {{
        Ok(serde_json::to_string(&f()?).unwrap())
    }}
    Ok(windmill_runner(|| main({spread}))?)
}}
"#
    );

    write_file(job_dir, "inner.rs", &mod_content)?;
    Ok(())
}

/// The instance/workspace cargo registry configuration, if any is set, as the body of a
/// `.cargo/config.toml`.
async fn cargo_registries_config(job_id: &Uuid, w_id: &str, conn: &Connection) -> Option<String> {
    read_ee_registry_with_workspace_override(
        CARGO_REGISTRIES.read().await.clone(),
        "cargo_registries",
        "cargo registries",
        job_id,
        w_id,
        conn,
    )
    .await
    .filter(|cargo_registries| !cargo_registries.trim().is_empty())
}

/// Write `{dir}/.cargo/config.toml`. cargo only reads it when `dir` is the working
/// directory of the invocation (or an ancestor of it), never because `dir` holds the
/// manifest the invocation was pointed at.
async fn write_cargo_registries_config(dir: &str, registries: &str) -> anyhow::Result<()> {
    let cargo_dir = format!("{dir}/.cargo");
    create_dir_all(&cargo_dir).await?;
    write_file(&cargo_dir, "config.toml", registries)?;
    Ok(())
}

async fn write_cargo_config(
    job_dir: &str,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> anyhow::Result<()> {
    if let Some(cargo_registries) = cargo_registries_config(job_id, w_id, conn).await {
        write_cargo_registries_config(job_dir, &cargo_registries).await?;
    }
    Ok(())
}

/// Stub manifest for a [`HostCargoCwd`]. Nothing is ever built from it; `cargo sweep`
/// only needs *a* manifest in its working directory to ask `cargo metadata` where the
/// target directory is, and `[workspace]` keeps cargo from walking further up.
const HOST_CARGO_CWD_MANIFEST: &str = r#"# Worker-owned stub crate. Host-side cargo invocations run from here so that their
# configuration discovery never reaches a job directory. Nothing is built from it.
[workspace]

[package]
name = "windmill_host_cargo_cwd"
version = "0.0.0"

[lib]
path = "src/lib.rs"
"#;

/// Working directory for cargo invocations that run outside the build jail. cargo reads
/// `.cargo/config.toml` from its working directory and that directory's ancestors (not
/// from the manifest's directory), so this is a worker-owned directory under the rust
/// cache dir, outside any job directory and outside the build dirs.
struct HostCargoCwd {
    path: String,
}

impl HostCargoCwd {
    /// `registries` is the worker's own cargo registry configuration (never the job's), to
    /// be made visible to the invocation as this directory's `.cargo/config.toml`.
    async fn new(kind: &str, registries: Option<&str>) -> anyhow::Result<Self> {
        let path = format!("{}/{kind}-{}", *HOST_CARGO_CWD_DIR, Uuid::new_v4());
        create_dir_all(format!("{path}/src")).await.map_err(|e| {
            anyhow::anyhow!("Could not create host cargo working dir {path}.\ne: {e}")
        })?;
        let cwd = HostCargoCwd { path };
        write_file(&cwd.path, "Cargo.toml", HOST_CARGO_CWD_MANIFEST)?;
        write_file(&format!("{}/src", cwd.path), "lib.rs", "")?;
        if let Some(registries) = registries {
            write_cargo_registries_config(&cwd.path, registries).await?;
        }
        Ok(cwd)
    }

    fn path(&self) -> &str {
        &self.path
    }
}

impl Drop for HostCargoCwd {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// `cargo generate-lockfile` for the crate in `job_dir`, run from `cwd`.
///
/// `cwd` is a [`HostCargoCwd`] and not `job_dir`, so the crate is named with
/// `--manifest-path`; cargo writes `Cargo.lock` next to that manifest, i.e. still at
/// `{job_dir}/Cargo.lock`.
fn build_generate_lockfile_command(cwd: &str, job_dir: &str) -> Command {
    let mut gen_lockfile_cmd = Command::new(CARGO_PATH.as_str());
    gen_lockfile_cmd
        .current_dir(cwd)
        .env_clear()
        .env("PATH", PATH_ENV.as_str())
        .env("HOME", HOME_ENV.as_str())
        .env("CARGO_HOME", CARGO_HOME.as_str())
        .env("RUSTUP_HOME", RUSTUP_HOME.as_str())
        .envs(PROXY_ENVS.clone())
        .args([
            "generate-lockfile",
            "--manifest-path",
            &format!("{job_dir}/Cargo.toml"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        gen_lockfile_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());
        gen_lockfile_cmd.env("USERPROFILE", crate::USERPROFILE_ENV.as_str());
        gen_lockfile_cmd.env(
            "TMP",
            std::env::var("TMP").unwrap_or_else(|_| "C:\\tmp".to_string()),
        );
    }
    gen_lockfile_cmd
}

/// `cargo sweep` over the shared build dir's target dir, run from `cwd`.
///
/// `cwd` is a [`HostCargoCwd`] and not `job_dir`. `cargo sweep` resolves the target
/// directory to prune by running `cargo metadata` for the manifest in its working
/// directory, and `CARGO_TARGET_DIR` overrides what that resolves to — which is how this
/// invocation has always reached `{build_dir}/target`, the stub manifest in `cwd` standing
/// in for the job's crate.
fn build_cargo_sweep_command(cwd: &str, build_dir: &str) -> Command {
    let mut sweep_cmd = Command::new(CARGO_PATH.as_str());
    let sweep_path = format!("{}/bin:{}", CARGO_HOME.as_str(), PATH_ENV.as_str());
    sweep_cmd
        .current_dir(cwd)
        .env_clear()
        .env("PATH", &sweep_path)
        .env("CARGO_HOME", CARGO_HOME.as_str())
        .env("HOME", HOME_ENV.as_str())
        .env("CARGO_TARGET_DIR", &format!("{build_dir}/target"))
        .env("RUSTUP_HOME", RUSTUP_HOME.as_str())
        .args(["sweep", "--maxsize", SWEEP_MAXSIZE.as_str()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        sweep_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());
        sweep_cmd.env(
            "TMP",
            std::env::var("TMP").unwrap_or_else(|_| "C:\\tmp".to_string()),
        );
        sweep_cmd.env("USERPROFILE", crate::USERPROFILE_ENV.as_str());
    }
    sweep_cmd
}

pub async fn generate_cargo_lockfile(
    job_id: &Uuid,
    code: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    conn: &Connection,
    worker_name: &str,
    w_id: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    ensure_rust_runtime_dirs();
    check_executor_binary_exists("cargo", CARGO_PATH.as_str(), "rust")?;

    gen_cargo_crate(code, job_dir)?;
    let registries = cargo_registries_config(job_id, w_id, conn).await;
    if let Some(registries) = registries.as_deref() {
        write_cargo_registries_config(job_dir, registries).await?;
    }

    // Run outside `job_dir`; the registry configuration is carried over explicitly.
    let cwd = HostCargoCwd::new("lockfile", registries.as_deref()).await?;
    let gen_lockfile_cmd = build_generate_lockfile_command(cwd.path(), job_dir);
    let gen_lockfile_process =
        start_child_process(gen_lockfile_cmd, CARGO_PATH.as_str(), false).await?;
    handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        gen_lockfile_process,
        false,
        worker_name,
        w_id,
        "cargo generate-lockfile",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;

    let path_lock = format!("{job_dir}/Cargo.lock");
    let mut file = File::open(path_lock).await?;
    let mut req_content = String::new();
    file.read_to_string(&mut req_content).await?;
    Ok(req_content)
}

async fn get_build_dir(
    job: &MiniPulledJob,
    conn: &Connection,
    worker_name: &str,
    is_preview: bool,
) -> anyhow::Result<String> {
    let (bd, run_sweep) = job
        .runnable_path
        .as_ref()
        .and_then(|p| {
            if !is_preview || *NO_SHARED_BUILD_DIR {
                None
            } else {
                if !is_sandboxing_enabled() {
                    // If nsjail is disabled then entire worker has shared build directory
                    // It drastically improves cache hit-rate.
                    Some((format!("{}/build/{worker_name}", *RUST_CACHE_DIR), true))
                } else {
                    // If nsjail is enabled, having global shared directory is vulnerability and target for an attack
                    // Instead we either:
                    // 1. Create different build directory for workspace script and user. Balanced caching while mainining high degree of security.
                    // 2. If user is not known or something else goes wrong - use random build dir. This is equivalent to no cache at all.
                    Some((
                        format!(
                            "{}/build/{}@{}@{}",
                            *RUST_CACHE_DIR,
                            &job.workspace_id,
                            p.replace('/', "."),
                            &job.created_by
                        ),
                        true,
                    ))
                }
            }
        })
        .unwrap_or((
            format!("{}/build/{}", *RUST_CACHE_DIR, Uuid::new_v4()),
            false,
        ));

    {
        let (t, r, g) = (
            create_dir_all(format!("{}/target", &bd)).await,
            create_dir_all(format!("{}/registry", &bd)).await,
            create_dir_all(format!("{}/git", &bd)).await,
        );

        t.and(r)
            .and(g)
            .map_err(|e| anyhow::anyhow!("Could not create build dir for rust.\ne: {e}"))?;
    }

    if run_sweep {
        // Also run sweep to make sure target isn't using too much disk
        let (job_id, conn, w_id, wk_name, build_dir) = (
            job.id.clone(),
            conn.clone(),
            job.workspace_id.clone(),
            worker_name.to_owned(),
            bd.clone(),
        );

        tokio::spawn(async move {
            // Run from a dedicated directory rather than the job directory. `cwd` is held
            // for the whole sweep; dropping it removes the directory.
            let cwd = match HostCargoCwd::new("sweep", None).await {
                Ok(cwd) => cwd,
                Err(e) => {
                    tracing::warn!(
                        workspace_id = %w_id,
                        job_id = %job_id,
                        "Failed to prepare working directory for `cargo sweep`. Rust cache may grow over time, cargo sweep is meant to clean up unused cache.\ne: {e}\n"
                    );
                    return;
                }
            };
            let sweep_cmd = build_cargo_sweep_command(cwd.path(), &build_dir);
            if let Err(e) = match start_child_process(sweep_cmd, CARGO_PATH.as_str(), false).await {
                Ok(sweep_process) => {
                    handle_child(
                        &job_id,
                        &conn,
                        &mut 0,
                        &mut None,
                        sweep_process,
                        false,
                        &wk_name,
                        &w_id,
                        "cargo sweep",
                        None,
                        false,
                        &mut None,
                        None,
                        None,
                    )
                    .await
                }
                Err(e) => Err(e),
            } {
                tracing::warn!(
                    workspace_id = %w_id,
                    job_id = %job_id,
                    "Failed to run `cargo sweep`. Rust cache may grow over time, cargo sweep is meant to clean up unused cache.\ne: {e}\n"
                );
            }
        });
    }
    Ok(bd)
}

pub async fn build_rust_crate(
    job: &MiniPulledJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    conn: &Connection,
    worker_name: &str,
    base_internal_url: &str,
    hash: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    is_preview: bool,
) -> error::Result<String> {
    ensure_rust_runtime_dirs();
    let bin_path = format!("{}/{hash}", *RUST_CACHE_DIR);

    let build_dir = get_build_dir(job, conn, worker_name, is_preview).await?;

    let child = if is_sandboxing_enabled() {
        let _ = write_file(
            job_dir,
            "download.config.proto",
            &NSJAIL_CONFIG_COMPILE_RUST_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", &*RUST_CACHE_DIR)
                .replace("{CARGO_HOME}", CARGO_HOME.as_str())
                .replace("{RUSTUP_HOME}", RUSTUP_HOME.as_str())
                .replace("{TRACING_PROXY_CA_CERT_PATH}", &*TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL)
                .replace(
                    "{TMP_MOUNT_BLOCK}",
                    &resolve_nsjail_tmp_mount_block(job_dir).await,
                )
                .replace("{BUILD}", &build_dir),
        )?;
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(PROXY_ENVS.clone())
            .env("HOME", HOME_ENV.as_str())
            .env("CARGO_HOME", CARGO_HOME.as_str())
            .env("RUSTUP_HOME", RUSTUP_HOME.as_str())
            .env("CARGO_TARGET_DIR", &(build_dir.clone() + "/target"))
            .args(vec![
                "--config",
                "download.config.proto",
                "--",
                CARGO_PATH.as_ref(),
                "build",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if !is_preview {
            nsjail_cmd.arg("--release");
        }
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        let mut build_rust_cmd = Command::new(CARGO_PATH.as_str());
        build_rust_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(PROXY_ENVS.clone())
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .env("CARGO_HOME", CARGO_HOME.as_str())
            .env("RUSTUP_HOME", RUSTUP_HOME.as_str())
            .env("CARGO_TARGET_DIR", &(build_dir.clone() + "/target"))
            .args(vec!["build"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if !is_preview {
            build_rust_cmd.arg("--release");
        }
        #[cfg(windows)]
        {
            build_rust_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());
            build_rust_cmd.env(
                "TMP",
                std::env::var("TMP").unwrap_or_else(|_| "C:\\tmp".to_string()),
            );
            build_rust_cmd.env("USERPROFILE", crate::USERPROFILE_ENV.as_str());
            // MSVC linker needs LIB and INCLUDE to find kernel32.lib etc.
            if let Ok(lib) = std::env::var("LIB") {
                build_rust_cmd.env("LIB", lib);
            }
            if let Ok(include) = std::env::var("INCLUDE") {
                build_rust_cmd.env("INCLUDE", include);
            }
        }
        start_child_process(build_rust_cmd, CARGO_PATH.as_str(), false).await?
    };
    handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        false,
        worker_name,
        &job.workspace_id,
        "rust build",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;
    append_logs(&job.id, &job.workspace_id, "\n\n", conn).await;

    tokio::fs::copy(
        &format!(
            "{build_dir}/target/{}/{RUST_BIN_NAME}",
            if is_preview { "debug" } else { "release" },
        ),
        format!("{job_dir}/{RUST_BIN_NAME}"),
    )
    .await
    .map_err(|e| {
        Error::ExecutionErr(format!(
            "could not copy built binary from [...]/target/.../{RUST_BIN_NAME} to {job_dir}/{RUST_BIN_NAME}: {e:?}"
        ))
    })?;

    match save_cache(
        &bin_path,
        &format!("{RUST_OBJECT_STORE_PREFIX}{hash}"),
        &format!("{job_dir}/{RUST_BIN_NAME}"),
        false,
    )
    .await
    {
        Err(e) => {
            let em = format!(
                "could not save {bin_path} to {job_dir}/{RUST_BIN_NAME} to rust cache: {e:?}",
            );
            tracing::error!(em);
            Ok(em)
        }
        Ok(logs) => Ok(logs),
    }
}

/// Cache key of a Rust build. The run path and the deploy-time prebuild must derive it
/// the same way or the prebuilt binary is never found and gets rebuilt on first run.
async fn rust_cache_key(
    code: &str,
    requirements_o: Option<&String>,
    w_id: &str,
    modules: Option<&HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> String {
    let mut hash = compute_rust_hash(code, requirements_o, modules);
    hash.push_str(&crate::workspace_registry_cache_suffix(w_id).await);
    hash
}

/// Compile a deployed Rust script ahead of its first run and push the binary to the
/// shared cache.
pub async fn prebuild_rust_binary(
    job: &MiniPulledJob,
    code: &str,
    lock: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    conn: &Connection,
    worker_name: &str,
    base_internal_url: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    modules: Option<&HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> error::Result<Option<String>> {
    ensure_rust_runtime_dirs();
    check_executor_binary_exists("cargo", CARGO_PATH.as_str(), "rust")?;

    let hash = rust_cache_key(code, Some(&lock.to_string()), &job.workspace_id, modules).await;
    let remote_path = format!("{RUST_OBJECT_STORE_PREFIX}{hash}");
    if crate::global_cache::exists_in_object_store(&remote_path).await {
        return Ok(None);
    }

    gen_cargo_crate(code, job_dir)?;
    write_cargo_config(job_dir, &job.id, &job.workspace_id, conn).await?;
    write_file(job_dir, "Cargo.lock", lock)?;

    let logs = build_rust_crate(
        job,
        mem_peak,
        canceled_by,
        job_dir,
        conn,
        worker_name,
        base_internal_url,
        &hash,
        occupancy_metrics,
        false,
    )
    .await?;
    crate::global_cache::ensure_pushed_to_object_store(&remote_path).await?;
    Ok(Some(logs))
}

pub fn compute_rust_hash(
    code: &str,
    requirements_o: Option<&String>,
    // Companion modules are written into the crate dir and compiled into the binary this
    // key names, so leaving them out shares one script's binary with another.
    modules: Option<&HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> String {
    let base = format!(
        "{}{}",
        code,
        requirements_o
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or_default()
    );
    crate::worker::artifact_cache_name(base, modules)
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_rust_job(
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &MiniPulledJob,
    conn: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    inner_content: &str,
    job_dir: &str,
    requirements_o: Option<&String>,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    occupancy_metrics: &mut OccupancyMetrics,
    modules: Option<&HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> Result<Box<RawValue>, Error> {
    ensure_rust_runtime_dirs();
    check_executor_binary_exists("cargo", CARGO_PATH.as_str(), "rust")?;

    let hash = rust_cache_key(inner_content, requirements_o, &job.workspace_id, modules).await;
    let bin_path = format!("{}/{hash}", *RUST_CACHE_DIR);
    let remote_path = format!("{RUST_OBJECT_STORE_PREFIX}{hash}");

    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

    let (cache, cache_logs) = crate::global_cache::load_cache(&bin_path, &remote_path, false).await;

    let cache_logs = if cache {
        let target = format!("{job_dir}/{RUST_BIN_NAME}");

        #[cfg(unix)]
        let symlink = std::os::unix::fs::symlink(&bin_path, &target);
        #[cfg(windows)]
        let symlink = std::os::windows::fs::symlink_file(&bin_path, &target);

        symlink.map_err(|e| {
            Error::ExecutionErr(format!(
                "could not copy cached binary from {bin_path} to {target}: {e:?}"
            ))
        })?;

        create_args_and_out_file(client, job, job_dir, conn).await?;
        cache_logs
    } else {
        let logs1 = format!("{cache_logs}\n\n--- CARGO BUILD ---\n");
        append_logs(&job.id, &job.workspace_id, logs1, conn).await;

        gen_cargo_crate(inner_content, job_dir)?;
        write_cargo_config(job_dir, &job.id, &job.workspace_id, conn).await?;

        if let Some(reqs) = requirements_o {
            if !reqs.is_empty() {
                write_file(job_dir, "Cargo.lock", &reqs)?;
            }
        }

        create_args_and_out_file(client, job, job_dir, conn).await?;

        build_rust_crate(
            &job,
            mem_peak,
            canceled_by,
            job_dir,
            conn,
            worker_name,
            base_internal_url,
            &hash,
            occupancy_metrics,
            requirements_o.is_none(),
        )
        .await?
    };

    let logs2 = format!("{cache_logs}\n\n--- RUST CODE EXECUTION ---\n");
    append_logs(&job.id, &job.workspace_id, logs2, conn).await;

    let child = if is_sandboxing_enabled() {
        let nsjail_timeout =
            resolve_nsjail_timeout(conn, &job.workspace_id, job.id, job.timeout).await;
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .envs(
                get_proxy_envs_for_lang(
                    &ScriptLang::Rust,
                    job.kind,
                    &job.id,
                    &job.workspace_id,
                    conn,
                )
                .await?,
            )
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["--config", "run.config.proto", "--", "/tmp/main"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_RUST_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", &*RUST_CACHE_DIR)
                .replace("{CACHE_HASH}", &hash)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{TRACING_PROXY_CA_CERT_PATH}", &*TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL)
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace(
                    "{TMP_MOUNT_BLOCK}",
                    &resolve_nsjail_tmp_mount_block(job_dir).await,
                )
                .replace("{TIMEOUT}", &nsjail_timeout)
                .replace("{ENVARS}", &script_envars(&envs, &nsjail_cmd, &[])),
        )?;
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        let compiled_executable_name = &format!("{job_dir}/{RUST_BIN_NAME}");
        let mut run_rust = build_command_with_isolation(compiled_executable_name, &[]);
        run_rust
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(
                get_proxy_envs_for_lang(
                    &ScriptLang::Rust,
                    job.kind,
                    &job.id,
                    &job.workspace_id,
                    conn,
                )
                .await?,
            )
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            run_rust.env("SystemRoot", SYSTEM_ROOT.as_str());
            run_rust.env("USERPROFILE", crate::USERPROFILE_ENV.as_str());
        }

        start_child_process(run_rust, compiled_executable_name, false).await?
    };
    handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        is_sandboxing_enabled(),
        worker_name,
        &job.workspace_id,
        "rust run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;
    read_result(job_dir, None).await
}

#[cfg(test)]
mod host_cargo_cwd_tests {
    use super::*;
    use std::ffi::OsStr;
    use std::path::Path;

    /// A plausible job directory: `{TMP}/windmill/{worker}/{job_id}`.
    const JOB_DIR: &str = "/tmp/windmill/wk-default-xyz-AbCdE/01a12c99-6620-74b9-a4ca-189c60de272c";

    fn args(cmd: &Command) -> Vec<String> {
        cmd.as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    fn env(cmd: &Command, key: &str) -> Option<String> {
        cmd.as_std()
            .get_envs()
            .find(|(k, _)| *k == OsStr::new(key))
            .map(|(_, v)| v.unwrap_or(OsStr::new("")).to_string_lossy().into_owned())
    }

    /// The working directory is neither inside the job directory nor an ancestor of it.
    fn assert_cwd_is_isolated_from_job_dir(cmd: &Command, job_dir: &str) {
        let cwd = cmd
            .as_std()
            .get_current_dir()
            .expect("host-side cargo must set an explicit working directory");
        let job_dir = Path::new(job_dir);
        assert!(
            cwd.ancestors().all(|a| a != job_dir),
            "cwd {cwd:?} is inside the job dir {job_dir:?}"
        );
        assert!(
            job_dir.ancestors().all(|a| a != cwd),
            "cwd {cwd:?} is an ancestor of the job dir {job_dir:?}"
        );
        assert!(
            cwd.starts_with(Path::new(HOST_CARGO_CWD_DIR.as_str())),
            "cwd {cwd:?} is not one of the worker-owned dirs under {}",
            *HOST_CARGO_CWD_DIR
        );
    }

    #[test]
    fn generate_lockfile_runs_outside_the_job_dir_and_names_the_manifest() {
        let cwd = format!("{}/lockfile-deadbeef", *HOST_CARGO_CWD_DIR);
        let cmd = build_generate_lockfile_command(&cwd, JOB_DIR);

        assert_eq!(cmd.as_std().get_current_dir(), Some(Path::new(&cwd)));
        assert_cwd_is_isolated_from_job_dir(&cmd, JOB_DIR);
        // Cargo writes Cargo.lock next to the manifest it is given, which is where
        // `generate_cargo_lockfile` reads it back from.
        assert_eq!(
            args(&cmd),
            vec![
                "generate-lockfile".to_string(),
                "--manifest-path".to_string(),
                format!("{JOB_DIR}/Cargo.toml"),
            ]
        );
    }

    #[test]
    fn cargo_sweep_runs_outside_the_job_dir_and_still_targets_the_build_dir() {
        let cwd = format!("{}/sweep-deadbeef", *HOST_CARGO_CWD_DIR);
        let build_dir = format!("{}/build/acme@u.dev.script@dev", *RUST_CACHE_DIR);
        let cmd = build_cargo_sweep_command(&cwd, &build_dir);

        assert_eq!(cmd.as_std().get_current_dir(), Some(Path::new(&cwd)));
        assert_cwd_is_isolated_from_job_dir(&cmd, JOB_DIR);
        assert_eq!(
            args(&cmd),
            vec![
                "sweep".to_string(),
                "--maxsize".to_string(),
                SWEEP_MAXSIZE.to_string(),
            ]
        );
        // `cargo sweep` prunes the target dir that `cargo metadata` reports, and
        // CARGO_TARGET_DIR is what decides that -- not the cwd's manifest.
        assert_eq!(
            env(&cmd, "CARGO_TARGET_DIR").as_deref(),
            Some(format!("{build_dir}/target").as_str())
        );
    }

    #[tokio::test]
    async fn host_cargo_cwd_is_a_worker_owned_stub_crate_removed_on_drop() {
        let registries = "[registries.acme]\nindex = \"sparse+https://acme.test/index/\"\n";
        let path = {
            let cwd = HostCargoCwd::new("unittest", Some(registries))
                .await
                .expect("could not create host cargo cwd");
            let path = cwd.path().to_string();

            assert!(
                path.starts_with(&format!("{}/unittest-", *HOST_CARGO_CWD_DIR)),
                "unexpected host cargo cwd {path}"
            );
            assert!(
                !path.starts_with(&format!("{}/build/", *RUST_CACHE_DIR)),
                "host cargo cwd {path} is under the build dirs"
            );

            // `cargo sweep` needs a manifest in its cwd to ask `cargo metadata` for the
            // target dir; `[workspace]` keeps cargo from walking further up.
            let manifest = std::fs::read_to_string(format!("{path}/Cargo.toml"))
                .expect("stub Cargo.toml missing");
            assert!(
                manifest.contains("[workspace]"),
                "stub manifest is not its own workspace root: {manifest}"
            );
            assert!(
                manifest.contains("path = \"src/lib.rs\""),
                "stub manifest does not point at its stub target: {manifest}"
            );
            assert_eq!(
                std::fs::read_to_string(format!("{path}/src/lib.rs")).expect("stub lib.rs missing"),
                ""
            );
            // The worker's own registry configuration has to follow the invocation out
            // of the job dir, since cargo only reads it from the cwd and CARGO_HOME.
            assert_eq!(
                std::fs::read_to_string(format!("{path}/.cargo/config.toml"))
                    .expect("registry config missing"),
                registries
            );
            path
        };

        assert!(
            !Path::new(&path).exists(),
            "host cargo cwd {path} outlived its guard"
        );
    }

    #[tokio::test]
    async fn host_cargo_cwd_has_no_config_when_no_registries_are_set() {
        let cwd = HostCargoCwd::new("unittest", None)
            .await
            .expect("could not create host cargo cwd");
        assert!(
            !Path::new(&format!("{}/.cargo", cwd.path())).exists(),
            "host cargo cwd has a .cargo dir although no registries are configured"
        );
    }
}
