use serde_json::value::RawValue;
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
    worker::{save_cache, write_file},
};
use windmill_queue::MiniPulledJob;
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    common::{
        check_executor_binary_exists, create_args_and_out_file, get_reserved_variables,
        read_result, start_child_process, OccupancyMetrics,
    },
    handle_child::handle_child,
    AuthedClient, DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, NSJAIL_PATH, PATH_ENV, PROXY_ENVS,
    RUST_CACHE_DIR, TZ_ENV,
};

#[cfg(windows)]
use crate::SYSTEM_ROOT;

const NSJAIL_CONFIG_RUN_RUST_CONTENT: &str = include_str!("../nsjail/run.rust.config.proto");

lazy_static::lazy_static! {
    static ref HOME_DIR: String = std::env::var("HOME").expect("Could not find the HOME environment variable");
    static ref CARGO_HOME: String = std::env::var("CARGO_HOME").unwrap_or_else(|_| { CARGO_HOME_DEFAULT.clone() });
    static ref RUSTUP_HOME: String = std::env::var("RUSTUP_HOME").unwrap_or_else(|_| { RUSTUP_HOME_DEFAULT.clone() });
    static ref CARGO_PATH: String = std::env::var("CARGO_PATH").unwrap_or_else(|_| format!("{}/bin/cargo", CARGO_HOME.as_str()));
    static ref CARGO_SWEEP_PATH: String = std::env::var("CARGO_SWEEP_PATH").unwrap_or_else(|_| format!("{}/bin/cargo-sweep", CARGO_HOME.as_str()));
    static ref SWEEP_MAXSIZE: String = std::env::var("CARGO_SWEEP_MAXSIZE").unwrap_or("5GB".to_owned());
    static ref NO_SHARED_BUILD_DIR: bool = var("RUST_NO_SHARED_BUILD_DIR").ok().map(|flag| flag == "true").unwrap_or(false);

}

#[cfg(windows)]
lazy_static::lazy_static! {
    static ref CARGO_HOME_DEFAULT: String = format!("{}\\.cargo", *HOME_DIR);
    static ref RUSTUP_HOME_DEFAULT: String = format!("{}\\.rustup", *HOME_DIR);
}

#[cfg(unix)]
lazy_static::lazy_static! {
    static ref CARGO_HOME_DEFAULT: String = format!("{}/.cargo", *HOME_DIR);
    static ref RUSTUP_HOME_DEFAULT: String = format!("{}/.rustup", *HOME_DIR);
}

const RUST_OBJECT_STORE_PREFIX: &str = "rustbin/";

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

pub async fn generate_cargo_lockfile(
    job_id: &Uuid,
    code: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    w_id: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    check_executor_binary_exists("cargo", CARGO_PATH.as_str(), "rust")?;

    gen_cargo_crate(code, job_dir)?;

    let mut gen_lockfile_cmd = Command::new(CARGO_PATH.as_str());
    gen_lockfile_cmd
        .current_dir(job_dir)
        .args(vec!["generate-lockfile"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        gen_lockfile_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());
        gen_lockfile_cmd.env(
            "TMP",
            std::env::var("TMP").unwrap_or_else(|_| "C:\\tmp".to_string()),
        );
    }
    let gen_lockfile_process = start_child_process(gen_lockfile_cmd, CARGO_PATH.as_str()).await?;
    handle_child(
        job_id,
        db,
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
    )
    .await?;

    let path_lock = format!("{job_dir}/Cargo.lock");
    let mut file = File::open(path_lock).await?;
    let mut req_content = String::new();
    file.read_to_string(&mut req_content).await?;
    Ok(req_content)
}

pub async fn build_rust_crate(
    job: &MiniPulledJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    base_internal_url: &str,
    hash: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    let bin_path = format!("{}/{hash}", RUST_CACHE_DIR);
    let target = if let Some(path) = &job.runnable_path {
        let t = format!(
            "{RUST_CACHE_DIR}/targets/{}@{}@{}",
            &job.workspace_id,
            path.replace('/', "."),
            &job.created_by
        );
        if *NO_SHARED_BUILD_DIR {
            None
        } else if let Err(e) = create_dir_all(&t).await {
            tracing::warn!("Could not create shared target dir. Incremental compilation is not possible.\ne: {e}");
            None
        } else {
            let mut sweep_cmd = Command::new(CARGO_SWEEP_PATH.as_str());
            sweep_cmd
                .current_dir(job_dir)
                .env_clear()
                .env("PATH", PATH_ENV.as_str())
                .env("CARGO_HOME", CARGO_HOME.as_str())
                .env("HOME", HOME_ENV.as_str())
                .env("CARGO_TARGET_DIR", &(t.clone() + "/target"))
                .env("RUSTUP_HOME", RUSTUP_HOME.as_str())
                .args(["sweep", "--maxsize", SWEEP_MAXSIZE.as_str()])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            if let Err(e) = match start_child_process(sweep_cmd, CARGO_SWEEP_PATH.as_str()).await {
                Ok(sweep_process) => {
                    handle_child(
                        &job.id,
                        db,
                        mem_peak,
                        canceled_by,
                        sweep_process,
                        false,
                        worker_name,
                        &job.workspace_id,
                        "cargo sweep",
                        None,
                        false,
                        &mut Some(occupancy_metrics),
                        None,
                    )
                    .await
                }
                Err(e) => Err(e),
            } {
                tracing::warn!(
                    workspace_id = %job.workspace_id,
                    job_id = %job.id,
                    "Failed to run `cargo sweep`. Rust cache may grow over time, cargo sweep is meant to clean up unused cache.\ne: {e}\n"
                );
            }
            Some(t)
        }
    } else {
        tracing::warn!("Runnable path is None. Incremental compilation is not possible.");
        None
    };

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
        .args(vec!["build", "--release"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(ref t) = target {
        build_rust_cmd.env("CARGO_TARGET_DIR", &(t.clone() + "/target"));
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

    let build_rust_process = start_child_process(build_rust_cmd, CARGO_PATH.as_str()).await?;
    handle_child(
        &job.id,
        db,
        mem_peak,
        canceled_by,
        build_rust_process,
        false,
        worker_name,
        &job.workspace_id,
        "rust build",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
    )
    .await?;
    append_logs(&job.id, &job.workspace_id, "\n\n", db).await;

    tokio::fs::copy(
        &format!(
            "{}/target/release/main",
            target.as_ref().unwrap_or(&(job_dir.to_owned()))
        ),
        format! {"{job_dir}/main"},
    )
    .await
    .map_err(|e| {
        Error::ExecutionErr(format!(
            "could not copy built binary from [...]/target/release/main to {job_dir}/main: {e:?}"
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
    db: &sqlx::Pool<sqlx::Postgres>,
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
    check_executor_binary_exists("cargo", CARGO_PATH.as_str(), "rust")?;

    let hash = compute_rust_hash(inner_content, requirements_o);
    let bin_path = format!("{}/{hash}", RUST_CACHE_DIR);
    let remote_path = format!("{RUST_OBJECT_STORE_PREFIX}{hash}");

    let (cache, cache_logs) =
        windmill_common::worker::load_cache(&bin_path, &remote_path, false).await;

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

        create_args_and_out_file(client, job, job_dir, db).await?;
        cache_logs
    } else {
        let logs1 = format!("{cache_logs}\n\n--- CARGO BUILD ---\n");
        append_logs(&job.id, &job.workspace_id, logs1, db).await;

        gen_cargo_crate(inner_content, job_dir)?;

        if let Some(reqs) = requirements_o {
            if !reqs.is_empty() {
                write_file(job_dir, "Cargo.lock", &reqs)?;
            }
        }

        create_args_and_out_file(client, job, job_dir, db).await?;

        build_rust_crate(
            &job,
            mem_peak,
            canceled_by,
            job_dir,
            db,
            worker_name,
            base_internal_url,
            &hash,
            occupancy_metrics,
        )
        .await?
    };

    let logs2 = format!("{cache_logs}\n\n--- RUST CODE EXECUTION ---\n");
    append_logs(&job.id, &job.workspace_id, logs2, db).await;

    let reserved_variables =
        get_reserved_variables(job, &client.token, db, parent_runnable_path).await?;

    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_RUST_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", RUST_CACHE_DIR)
                .replace("{CACHE_HASH}", &hash)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )?;
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["--config", "run.config.proto", "--", "/tmp/main"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
    } else {
        let compiled_executable_name = "./main";
        let mut run_rust = Command::new(compiled_executable_name);
        run_rust
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            run_rust.env("SystemRoot", SYSTEM_ROOT.as_str());
            run_rust.env("USERPROFILE", crate::USERPROFILE_ENV.as_str());
        }

        start_child_process(run_rust, compiled_executable_name).await?
    };
    handle_child(
        &job.id,
        db,
        mem_peak,
        canceled_by,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        &job.workspace_id,
        "rust run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
    )
    .await?;
    read_result(job_dir).await
}
