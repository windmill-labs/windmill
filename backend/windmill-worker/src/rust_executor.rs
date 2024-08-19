use serde_json::value::RawValue;
use std::{collections::HashMap, process::Stdio};

use itertools::Itertools;
use tokio::process::Command;
use windmill_common::{error::Error, jobs::QueuedJob, utils::calculate_hash, worker::save_cache};
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    common::{
        create_args_and_out_file, get_reserved_variables, handle_child, read_result,
        start_child_process, write_file,
    },
    AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, NSJAIL_PATH, PATH_ENV,
    RUST_CACHE_DIR, TZ_ENV,
};

const NSJAIL_CONFIG_RUN_RUST_CONTENT: &str = include_str!("../nsjail/run.rust.config.proto");

lazy_static::lazy_static! {
    static ref CARGO_PATH: String = std::env::var("RUST_PATH").unwrap_or_else(|_| "/home/wendrul/.cargo/bin/cargo".to_string());
}

const RUST_OBJECT_STORE_PREFIX: &str = "rustbin/";

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_rust_job(
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    inner_content: &str,
    job_dir: &str,
    requirements_o: Option<String>,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
) -> Result<Box<RawValue>, Error> {
    tracing::error!(
        ?mem_peak,
        ?canceled_by,
        ?job,
        ?db,
        ?inner_content,
        ?job_dir,
        ?requirements_o,
        ?shared_mount,
        ?base_internal_url,
        ?worker_name,
        ?envs,
        "this is all folks",
    );

    let hash = calculate_hash(&format!(
        "{}{}",
        inner_content,
        requirements_o
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or_default()
    ));
    let bin_path = format!("{}/{hash}", RUST_CACHE_DIR);
    let remote_path = format!("{RUST_OBJECT_STORE_PREFIX}{hash}");

    let (cache, cache_logs) = windmill_common::worker::load_cache(&bin_path, &remote_path).await;


    let cache_logs = if cache {
        let target = format!("{job_dir}/main");
        std::os::unix::fs::symlink(&bin_path, &target).map_err(|e| {
            Error::ExecutionErr(format!(
                "could not copy cached binary from {bin_path} to {job_dir}/main: {e:?}"
            ))
        })?;

        create_args_and_out_file(client, job, job_dir, db).await?;
        cache_logs
    } else {
        let logs1 = format!("{cache_logs}\n\n--- CARGO BUILD ---\n");
        append_logs(&job.id, &job.workspace_id, logs1, db).await;

        // install_rust_dependencies(
        //     &job.id,
        //     inner_content,
        //     mem_peak,
        //     canceled_by,
        //     job_dir,
        //     db,
        //     true,
        //     worker_name,
        //     &job.workspace_id,
        // )
        // .await?;

        create_args_and_out_file(client, job, job_dir, db).await?;

        let sig = windmill_parser_rust::parse_rust_signature(&inner_content)?;
        let manifest = windmill_parser_rust::parse_rust_deps_into_manifest(&inner_content)?;
        write_file(job_dir, "Cargo.toml", &manifest).await?;

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
    result_file.write_all(serde_json::to_string(&result)?.into_bytes().as_ref())?;

    Ok(())
}
"#;
        write_file(job_dir, "main.rs", WRAPPER_CONTENT).await?;

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
{inner_content}

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

        write_file(job_dir, "inner.rs", &mod_content).await?;

        let mut build_rust_cmd = Command::new(CARGO_PATH.as_str());
        build_rust_cmd
            .current_dir(job_dir)
            .env_clear()
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            // .env("RUSTC_PATH", RUST_CACHE_DIR)
            .env("HOME", HOME_ENV.as_str())
            .args(vec!["build", "--release"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
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
        )
        .await?;
        append_logs(&job.id, &job.workspace_id, "\n\n", db).await;

        tokio::fs::copy(
            &format!("{job_dir}/target/release/main"),
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
        )
        .await
        {
            Err(e) => {
                let em = format!(
                    "could not save {bin_path} to {} to rust cache: {e:?}",
                    format!("{job_dir}/main"),
                );
                tracing::error!(em);
                em
            }
            Ok(logs) => logs,
        }
    };

    let logs2 = format!("{cache_logs}\n\n--- RUST CODE EXECUTION ---\n");
    append_logs(&job.id, &job.workspace_id, logs2, db).await;

    let client = &client.get_authed().await;
    let reserved_variables = get_reserved_variables(job, &client.token, db).await?;

    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_RUST_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", RUST_CACHE_DIR)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )
        .await?;
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
            // .env("GOPATH", RUST_CACHE_DIR)
            .env("HOME", HOME_ENV.as_str())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

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
    )
    .await?;
    read_result(job_dir).await
}

// async fn install_rust_dependencies(
//     id: _,
//     inner_content: _,
//     mem_peak: _,
//     canceled_by: _,
//     job_dir: _,
//     db: _,
//     arg: _,
//     worker_name: _,
//     workspace_id: _,
// ) -> _ {
//     todo!()
// }
