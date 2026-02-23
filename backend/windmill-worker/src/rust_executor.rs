use serde_json::value::RawValue;
#[cfg(not(windows))]
use std::sync::Once;
use std::{collections::HashMap, process::Stdio};
use uuid::Uuid;
use windmill_parser_rust::parse_rust_deps_into_manifest;

use itertools::Itertools;
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncReadExt,
    process::Command,
};
use windmill_common::{
    error::{self, Error},
    utils::calculate_hash,
    worker::{write_file, Connection},
};
use crate::global_cache::save_cache;
use windmill_queue::MiniPulledJob;
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    common::{
        build_command_with_isolation, check_executor_binary_exists, create_args_and_out_file,
        get_reserved_variables, read_result, start_child_process, OccupancyMetrics,
        DEV_CONF_NSJAIL,
    },
    get_proxy_envs_for_lang,
    handle_child::handle_child,
    is_sandboxing_enabled, read_ee_registry, CARGO_REGISTRIES, DISABLE_NUSER, HOME_ENV,
    NSJAIL_PATH, PATH_ENV, PROXY_ENVS, RUST_CACHE_DIR, TRACING_PROXY_CA_CERT_PATH, TZ_ENV,
};
use windmill_common::client::AuthedClient;
use windmill_common::scripts::ScriptLang;

#[cfg(windows)]
use crate::SYSTEM_ROOT;

const NSJAIL_CONFIG_RUN_RUST_CONTENT: &str = include_str!("../nsjail/run.rust.config.proto");
const NSJAIL_CONFIG_COMPILE_RUST_CONTENT: &str =
    include_str!("../nsjail/download.rust.config.proto");

fn find_cargo_path() -> String {
    if let Ok(p) = std::env::var("CARGO_PATH") {
        return p;
    }
    let from_home = format!("{}/bin/cargo", CARGO_HOME.as_str());
    if std::path::Path::new(&from_home).exists() {
        return from_home;
    }
    for p in ["/usr/local/cargo/bin/cargo", "/usr/bin/cargo"] {
        if std::path::Path::new(p).exists() {
            return p.to_string();
        }
    }
    from_home
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
    static ref HOME_DIR: String = std::env::var("HOME").expect("Could not find the HOME environment variable");
    static ref CARGO_HOME: String = std::env::var("CARGO_HOME").unwrap_or_else(|_| { CARGO_HOME_DEFAULT.clone() });
    static ref RUSTUP_HOME: String = std::env::var("RUSTUP_HOME").unwrap_or_else(|_| { RUSTUP_HOME_DEFAULT.clone() });
    static ref CARGO_PATH: String = find_cargo_path();
    static ref SWEEP_MAXSIZE: String = std::env::var("CARGO_SWEEP_MAXSIZE").unwrap_or("25GB".to_owned());
    static ref NO_SHARED_BUILD_DIR: bool = std::env::var("RUST_NO_SHARED_BUILD_DIR").ok().map(|flag| flag == "true").unwrap_or(false);
}

#[cfg(windows)]
lazy_static::lazy_static! {
    static ref CARGO_HOME_DEFAULT: String = format!("{}\\.cargo", *HOME_DIR);
    static ref RUSTUP_HOME_DEFAULT: String = format!("{}\\.rustup", *HOME_DIR);
}

#[cfg(not(windows))]
lazy_static::lazy_static! {
    static ref CARGO_HOME_DEFAULT: String = format!("{}/.cargo", *HOME_DIR);
    static ref RUSTUP_HOME_DEFAULT: String = format!("{}/.rustup", *HOME_DIR);
}

const RUST_OBJECT_STORE_PREFIX: &str = "rustbin/";

#[cfg(not(windows))]
lazy_static::lazy_static! {
    static ref PREINSTALLED_CARGO: String = find_preinstalled_dir(
        "CARGO_PREINSTALL_DIR",
        &["/usr/local/cargo", &format!("{}/.cargo", *HOME_DIR)],
    );
    static ref PREINSTALLED_RUSTUP: String = find_preinstalled_dir(
        "RUSTUP_PREINSTALL_DIR",
        &["/usr/local/rustup", &format!("{}/.rustup", *HOME_DIR)],
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

async fn write_cargo_config(
    job_dir: &str,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
) -> anyhow::Result<()> {
    if let Some(cargo_registries) = read_ee_registry(
        CARGO_REGISTRIES.read().await.clone(),
        "cargo registries",
        job_id,
        w_id,
        conn,
    )
    .await
    {
        if !cargo_registries.trim().is_empty() {
            let cargo_dir = format!("{job_dir}/.cargo");
            create_dir_all(&cargo_dir).await?;
            write_file(&cargo_dir, "config.toml", &cargo_registries)?;
        }
    }
    Ok(())
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
    write_cargo_config(job_dir, job_id, w_id, conn).await?;

    let mut gen_lockfile_cmd = Command::new(CARGO_PATH.as_str());
    gen_lockfile_cmd
        .current_dir(job_dir)
        .env_clear()
        .env("PATH", PATH_ENV.as_str())
        .env("HOME", HOME_ENV.as_str())
        .env("CARGO_HOME", CARGO_HOME.as_str())
        .env("RUSTUP_HOME", RUSTUP_HOME.as_str())
        .envs(PROXY_ENVS.clone())
        .args(vec!["generate-lockfile"])
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
    job_dir: &str,
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
                    Some((format!("{RUST_CACHE_DIR}/build/{worker_name}"), true))
                } else {
                    // If nsjail is enabled, having global shared directory is vulnerability and target for an attack
                    // Instead we either:
                    // 1. Create different build directory for workspace script and user. Balanced caching while mainining high degree of security.
                    // 2. If user is not known or something else goes wrong - use random build dir. This is equivalent to no cache at all.
                    Some((
                        format!(
                            "{RUST_CACHE_DIR}/build/{}@{}@{}",
                            &job.workspace_id,
                            p.replace('/', "."),
                            &job.created_by
                        ),
                        true,
                    ))
                }
            }
        })
        .unwrap_or((format!("{RUST_CACHE_DIR}/build/{}", Uuid::new_v4()), false));

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
        let mut sweep_cmd = Command::new(CARGO_PATH.as_str());
        let sweep_path = format!("{}/bin:{}", CARGO_HOME.as_str(), PATH_ENV.as_str());
        sweep_cmd
            .current_dir(job_dir)
            .env_clear()
            .env("PATH", &sweep_path)
            .env("CARGO_HOME", CARGO_HOME.as_str())
            .env("HOME", HOME_ENV.as_str())
            .env("CARGO_TARGET_DIR", &(bd.clone() + "/target"))
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

        let (job_id, conn, w_id, wk_name) = (
            job.id.clone(),
            conn.clone(),
            job.workspace_id.clone(),
            worker_name.to_owned(),
        );

        tokio::spawn(async move {
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
    let bin_path = format!("{}/{hash}", RUST_CACHE_DIR);

    let build_dir = get_build_dir(job, job_dir, conn, worker_name, is_preview).await?;

    let child = if is_sandboxing_enabled() {
        let _ = write_file(
            job_dir,
            "download.config.proto",
            &NSJAIL_CONFIG_COMPILE_RUST_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", RUST_CACHE_DIR)
                .replace("{CARGO_HOME}", CARGO_HOME.as_str())
                .replace("{TRACING_PROXY_CA_CERT_PATH}", TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL)
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
            "{build_dir}/target/{}/main",
            if is_preview { "debug" } else { "release" },
        ),
        format! {"{job_dir}/main"},
    )
    .await
    .map_err(|e| {
        Error::ExecutionErr(format!(
            "could not copy built binary from [...]/target/.../main to {job_dir}/main: {e:?}"
        ))
    })?;

    match save_cache(
        &bin_path,
        &format!("{RUST_OBJECT_STORE_PREFIX}{hash}"),
        &format!("{job_dir}/main"),
        false,
    )
    .await
    {
        Err(e) => {
            let em = format!(
                "could not save {bin_path} to {} to rust cache: {e:?}",
                format!("{job_dir}/main"),
            );
            tracing::error!(em);
            Ok(em)
        }
        Ok(logs) => Ok(logs),
    }
}

pub fn compute_rust_hash(code: &str, requirements_o: Option<&String>) -> String {
    calculate_hash(&format!(
        "{}{}",
        code,
        requirements_o
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or_default()
    ))
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
) -> Result<Box<RawValue>, Error> {
    ensure_rust_runtime_dirs();
    check_executor_binary_exists("cargo", CARGO_PATH.as_str(), "rust")?;

    let hash = compute_rust_hash(inner_content, requirements_o);
    let bin_path = format!("{}/{hash}", RUST_CACHE_DIR);
    let remote_path = format!("{RUST_OBJECT_STORE_PREFIX}{hash}");

    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

    let (cache, cache_logs) =
        crate::global_cache::load_cache(&bin_path, &remote_path, false).await;

    let cache_logs = if cache {
        let target = format!("{job_dir}/main");

        #[cfg(unix)]
        let symlink = std::os::unix::fs::symlink(&bin_path, &target);
        #[cfg(windows)]
        let symlink = std::os::windows::fs::symlink_dir(&bin_path, &target);

        symlink.map_err(|e| {
            Error::ExecutionErr(format!(
                "could not copy cached binary from {bin_path} to {job_dir}/main: {e:?}"
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
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_RUST_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", RUST_CACHE_DIR)
                .replace("{CACHE_HASH}", &hash)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{TRACING_PROXY_CA_CERT_PATH}", TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL)
                .replace("{SHARED_MOUNT}", shared_mount),
        )?;
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(get_proxy_envs_for_lang(&ScriptLang::Rust).await?)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["--config", "run.config.proto", "--", "/tmp/main"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        let compiled_executable_name = "./main";
        let mut run_rust = build_command_with_isolation(compiled_executable_name, &[]);
        run_rust
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(get_proxy_envs_for_lang(&ScriptLang::Rust).await?)
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
