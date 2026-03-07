use std::{collections::HashMap, process::Stdio};

use itertools::Itertools;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    process::Command,
};
use windmill_common::{
    client::AuthedClient,
    error::Error,
    worker::{write_file, Connection},
};
use windmill_parser::Arg;
use windmill_parser_r::parse_r_signature;
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::{
    common::{
        build_command_with_isolation, create_args_and_out_file, get_reserved_variables,
        read_result, start_child_process, OccupancyMetrics, DEV_CONF_NSJAIL,
    },
    get_proxy_envs_for_lang,
    handle_child::{self},
    is_sandboxing_enabled, DISABLE_NUSER, NSJAIL_PATH, PATH_ENV, PROXY_ENVS, R_CACHE_DIR,
    TRACING_PROXY_CA_CERT_PATH,
};
use windmill_common::scripts::ScriptLang;

lazy_static::lazy_static! {
    static ref RSCRIPT_PATH: String = std::env::var("RSCRIPT_PATH").unwrap_or_else(|_| "/usr/bin/Rscript".to_string());
    static ref R_PROXY_ENVS: Vec<(String, String)> = {
        PROXY_ENVS
            .clone()
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect()
    };
}

const NSJAIL_CONFIG_RUN_R_CONTENT: &str = include_str!("../nsjail/run.r.config.proto");

#[allow(dead_code)]
pub(crate) struct JobHandlerInput<'a> {
    pub base_internal_url: &'a str,
    pub canceled_by: &'a mut Option<CanceledBy>,
    pub client: &'a AuthedClient,
    pub parent_runnable_path: Option<String>,
    pub conn: &'a Connection,
    pub envs: HashMap<String, String>,
    pub inner_content: &'a str,
    pub job: &'a MiniPulledJob,
    pub job_dir: &'a str,
    pub mem_peak: &'a mut i32,
    pub occupancy_metrics: &'a mut OccupancyMetrics,
    pub requirements_o: Option<&'a String>,
    pub shared_mount: &'a str,
    pub worker_name: &'a str,
}

pub async fn handle_r_job<'a>(
    mut args: JobHandlerInput<'a>,
) -> Result<Box<sqlx::types::JsonRawValue>, Error> {
    // --- Prepare ---
    {
        prepare(&args).await?;
    }
    // --- Execute ---
    {
        run(&mut args).await?;
    }
    // --- Retrieve results ---
    {
        read_result(&args.job_dir, None).await
    }
}

pub async fn prepare<'a>(
    JobHandlerInput { job, conn, job_dir, inner_content, client, .. }: &JobHandlerInput<'a>,
) -> Result<(), Error> {
    create_args_and_out_file(&client, job, job_dir, conn).await?;
    File::create(format!("{}/main.r", job_dir))
        .await?
        .write_all(&wrap(inner_content)?.into_bytes())
        .await?;

    // Create windmill client library for R
    let wm_lib_path = format!("{}/r_libs", *R_CACHE_DIR);
    if !std::fs::metadata(&wm_lib_path).is_ok() {
        fs::create_dir_all(&wm_lib_path).await?;

        File::create(format!("{}/windmill.r", &wm_lib_path))
            .await?
            .write_all(
                r##"
# Windmill mini client methods for R
# Uses system curl to avoid requiring any extra R packages

.wm_fetch <- function(url) {
    token <- Sys.getenv("WM_TOKEN")
    result <- system2(
        "curl", c("-s", "-f",
            "-H", paste0("Authorization: Bearer ", token),
            url),
        stdout = TRUE, stderr = TRUE
    )
    if (!is.null(attr(result, "status")) && attr(result, "status") != 0) {
        stop(paste("HTTP request failed:", paste(result, collapse = "\n")))
    }
    jsonlite::fromJSON(paste(result, collapse = "\n"))
}

get_variable <- function(path) {
    base_url <- Sys.getenv("BASE_INTERNAL_URL")
    workspace <- Sys.getenv("WM_WORKSPACE")
    url <- paste0(base_url, "/api/w/", workspace, "/variables/get_value/", path)
    .wm_fetch(url)
}

get_resource <- function(path) {
    base_url <- Sys.getenv("BASE_INTERNAL_URL")
    workspace <- Sys.getenv("WM_WORKSPACE")
    url <- paste0(base_url, "/api/w/", workspace, "/resources/get_value_interpolated/", path)
    .wm_fetch(url)
}
"##
                .as_bytes(),
            )
            .await?;
    }
    Ok(())
}

async fn run<'a>(
    JobHandlerInput {
        occupancy_metrics,
        mem_peak,
        canceled_by,
        worker_name,
        job,
        conn,
        job_dir,
        shared_mount,
        client,
        envs,
        base_internal_url,
        parent_runnable_path,
        ..
    }: &mut JobHandlerInput<'a>,
) -> Result<(), Error> {
    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path.clone()).await?;

    let child = if !cfg!(windows) && is_sandboxing_enabled() {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n--- ISOLATED R CODE EXECUTION ---\n"),
            conn,
        )
        .await;

        write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_R_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{SHARED_MOUNT}", &shared_mount)
                .replace("{R_CACHE_DIR}", &*R_CACHE_DIR)
                .replace("{TRACING_PROXY_CA_CERT_PATH}", &*TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string()),
        )?;
        let mut cmd = Command::new(NSJAIL_PATH.as_str());
        cmd.env_clear()
            .current_dir(job_dir)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(envs)
            .envs(reserved_variables)
            .envs(R_PROXY_ENVS.clone())
            .envs(get_proxy_envs_for_lang(&ScriptLang::Rlang).await?)
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                RSCRIPT_PATH.as_str(),
                "main.r",
            ]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        start_child_process(cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n--- R CODE EXECUTION ---\n"),
            conn,
        )
        .await;

        let rscript_executable = if cfg!(windows) {
            "Rscript.exe"
        } else {
            RSCRIPT_PATH.as_str()
        };

        let args = vec!["main.r"];
        let mut cmd = build_command_with_isolation(rscript_executable, &args);

        cmd.env_clear()
            .current_dir(job_dir.to_owned())
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(reserved_variables)
            .envs(R_PROXY_ENVS.clone())
            .envs(get_proxy_envs_for_lang(&ScriptLang::Rlang).await?)
            .envs(envs);

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            cmd.env("SystemRoot", crate::SYSTEM_ROOT.as_str())
                .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                );
        }
        start_child_process(cmd, rscript_executable, false).await?
    };
    handle_child::handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        is_sandboxing_enabled(),
        worker_name,
        &job.workspace_id,
        "r",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;
    Ok(())
}

fn wrap(inner_content: &str) -> Result<String, Error> {
    let sig = parse_r_signature(inner_content)?;
    let spread = sig
        .args
        .clone()
        .into_iter()
        .map(|Arg { name, .. }| format!("{name} = args${name}", name = name))
        .collect_vec()
        .join(", ");
    Ok(r#"INNER_CONTENT

library(jsonlite)
args <- fromJSON("args.json")

tryCatch({
    res <- main(SPREAD)
    write(toJSON(res, auto_unbox = TRUE, null = "null"), "result.json")
}, error = function(e) {
    error_obj <- list(
        name = class(e)[1],
        message = conditionMessage(e),
        stack = paste(capture.output(traceback()), collapse = "\n")
    )
    write(toJSON(error_obj, auto_unbox = TRUE), "result.json")
    stop(e)
})
"#
    .replace("INNER_CONTENT", inner_content)
    .replace("SPREAD", &spread))
}
