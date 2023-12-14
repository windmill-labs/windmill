use std::{collections::HashMap, fs, process::Stdio};

use regex::Regex;
use serde_json::{json, value::RawValue};
use sqlx::types::Json;
use tokio::process::Command;
use windmill_common::{error::Error, jobs::QueuedJob, worker::to_raw_value};
use windmill_queue::CanceledBy;

const BIN_BASH: &str = "/bin/bash";
const NSJAIL_CONFIG_RUN_BASH_CONTENT: &str = include_str!("../nsjail/run.bash.config.proto");
const NSJAIL_CONFIG_RUN_POWERSHELL_CONTENT: &str =
    include_str!("../nsjail/run.powershell.config.proto");

lazy_static::lazy_static! {
    static ref RE_POWERSHELL_IMPORTS: Regex = Regex::new(r#"^Import-Module\s+(?:-Name\s+)?"?([^-\s"]+)"?"#).unwrap();
}

use crate::{
    common::{
        build_args_map, get_reserved_variables, handle_child, read_file, read_file_content,
        set_logs, start_child_process, write_file,
    },
    AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, NSJAIL_PATH, PATH_ENV,
    POWERSHELL_CACHE_DIR, POWERSHELL_PATH, TZ_ENV,
};

lazy_static::lazy_static! {

    pub static ref ANSI_ESCAPE_RE: Regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_bash_job(
    logs: &mut String,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    content: &str,
    job_dir: &str,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
) -> Result<Box<RawValue>, Error> {
    logs.push_str("\n\n--- BASH CODE EXECUTION ---\n");
    set_logs(logs, &job.id, db).await;
    write_file(
        job_dir,
        "main.sh",
        &format!("set -e\n{content}\necho \"\"\nsleep 0.02"),
    )
    .await?;
    let token = client.get_token().await;
    let mut reserved_variables = get_reserved_variables(job, &token, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let args = build_args_map(job, client, db).await?.map(Json);
    let job_args = if args.is_some() {
        args.as_ref()
    } else {
        job.args.as_ref()
    };

    let args_owned = windmill_parser_bash::parse_bash_sig(&content)?
        .args
        .iter()
        .map(|arg| {
            job_args
                .and_then(|x| x.get(&arg.name).map(|x| raw_to_string(x.get())))
                .unwrap_or_else(String::new)
        })
        .collect::<Vec<String>>();
    let args = args_owned.iter().map(|s| &s[..]).collect::<Vec<&str>>();
    let _ = write_file(job_dir, "result.json", "").await?;
    let _ = write_file(job_dir, "result.out", "").await?;

    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_BASH_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )
        .await?;
        let mut cmd_args = vec!["--config", "run.config.proto", "--", "/bin/bash", "main.sh"];
        cmd_args.extend(args);
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
    } else {
        let mut cmd_args = vec!["main.sh"];
        cmd_args.extend(&args);
        let mut bash_cmd = Command::new(BIN_BASH);
        bash_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(bash_cmd, BIN_BASH).await?
    };
    handle_child(
        &job.id,
        db,
        logs,
        mem_peak,
        canceled_by,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        &job.workspace_id,
        "bash run",
        job.timeout,
        true,
    )
    .await?;

    let result_json_path = format!("{job_dir}/result.json");
    if let Ok(metadata) = tokio::fs::metadata(&result_json_path).await {
        if metadata.len() > 0 {
            return Ok(read_file(&result_json_path).await?);
        }
    }

    let result_out_path = format!("{job_dir}/result.out");
    if let Ok(metadata) = tokio::fs::metadata(&result_out_path).await {
        if metadata.len() > 0 {
            let result = read_file_content(&result_out_path).await?;
            return Ok(to_raw_value(&json!(result)));
        }
    }

    //for now bash jobs have an empty result object
    let last_line = serde_json::json!(logs
        .lines()
        .last()
        .map(|x| ANSI_ESCAPE_RE.replace_all(x, "").to_string())
        .unwrap_or_else(String::new));
    Ok(to_raw_value(&last_line))
}

fn raw_to_string(x: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(x) {
        Ok(serde_json::Value::String(x)) => x,
        Ok(x) => serde_json::to_string(&x).unwrap_or_else(|_| String::new()),
        _ => String::new(),
    }
}
#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_powershell_job(
    logs: &mut String,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    content: &str,
    job_dir: &str,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
) -> Result<Box<RawValue>, Error> {
    let pwsh_args = {
        let args = build_args_map(job, client, db).await?.map(Json);
        let job_args = if args.is_some() {
            args.as_ref()
        } else {
            job.args.as_ref()
        };

        let args_owned = windmill_parser_bash::parse_powershell_sig(&content)?
            .args
            .iter()
            .map(|arg| {
                (
                    arg.name.clone(),
                    job_args
                        .and_then(|x| x.get(&arg.name).map(|x| raw_to_string(x.get())))
                        .unwrap_or_else(String::new),
                )
            })
            .collect::<Vec<(String, String)>>();
        args_owned
            .iter()
            .map(|(n, v)| vec![format!("--{n}"), format!("{v}")])
            .flatten()
            .collect::<Vec<_>>()
    };

    let installed_modules = fs::read_dir(POWERSHELL_CACHE_DIR)?
        .filter_map(|x| {
            x.ok().map(|x| {
                x.path()
                    .display()
                    .to_string()
                    .split('/')
                    .last()
                    .unwrap_or_default()
                    .to_lowercase()
            })
        })
        .collect::<Vec<String>>();

    let mut install_string: String = String::new();
    for line in content.lines() {
        for cap in RE_POWERSHELL_IMPORTS.captures_iter(line) {
            let module = cap.get(1).unwrap().as_str();
            if !installed_modules.contains(&module.to_lowercase()) {
                logs.push_str(&format!("\n{} not found in cache", module.to_string()));
                // instead of using Install-Module, we use Save-Module so that we can specify the installation path
                install_string.push_str(&format!(
                    "Save-Module -Path {} -Force {};",
                    POWERSHELL_CACHE_DIR, module
                ));
            } else {
                logs.push_str(&format!("\n{} found in cache", module.to_string()));
            }
        }
    }
    set_logs(logs, &job.id, db).await;

    if !install_string.is_empty() {
        logs.push_str("\n\nInstalling modules...");
        set_logs(logs, &job.id, db).await;
        let child = Command::new("pwsh")
            .args(&["-Command", &install_string])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        handle_child(
            &job.id,
            db,
            logs,
            mem_peak,
            canceled_by,
            child,
            false,
            worker_name,
            &job.workspace_id,
            "powershell install",
            job.timeout,
            false,
        )
        .await?;
    }

    logs.push_str("\n\n--- POWERSHELL CODE EXECUTION ---\n");
    set_logs(logs, &job.id, db).await;

    // make sure default (only allhostsallusers) modules are loaded, disable autoload (cache can be large to explore especially on cloud) and add /tmp/windmill/cache to PSModulePath
    let profile = format!(
        "$PSModuleAutoloadingPreference = 'None'
$PSModulePathBackup = $env:PSModulePath
$env:PSModulePath = ($Env:PSModulePath -split ':')[-1]
Get-Module -ListAvailable | Import-Module
$env:PSModulePath = \"{}:$PSModulePathBackup\"",
        POWERSHELL_CACHE_DIR
    );
    // make sure param() is first
    let param_match = windmill_parser_bash::RE_POWERSHELL_PARAM.find(&content);
    let content: String = if let Some(param_match) = param_match {
        let param_match = param_match.as_str();
        format!(
            "{}\n{}\n{}",
            param_match,
            profile,
            content.replace(param_match, "")
        )
    } else {
        format!("{}\n{}", profile, content)
    };

    write_file(job_dir, "main.ps1", content.as_str()).await?;
    let token = client.get_token().await;
    let mut reserved_variables = get_reserved_variables(job, &token, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let _ = write_file(job_dir, "result.json", "").await?;
    let _ = write_file(job_dir, "result.out", "").await?;

    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_POWERSHELL_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{CACHE_DIR}", POWERSHELL_CACHE_DIR),
        )
        .await?;
        let mut cmd_args = vec![
            "--config",
            "run.config.proto",
            "--",
            POWERSHELL_PATH.as_str(),
            "-F",
            "main.ps1",
        ];
        cmd_args.extend(pwsh_args.iter().map(|x| x.as_str()));
        Command::new(NSJAIL_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("TZ", TZ_ENV.as_str())
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        let mut cmd_args = vec!["-F", "main.ps1"];
        cmd_args.extend(pwsh_args.iter().map(|x| x.as_str()));
        Command::new(POWERSHELL_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("TZ", TZ_ENV.as_str())
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    handle_child(
        &job.id,
        db,
        logs,
        mem_peak,
        canceled_by,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        &job.workspace_id,
        "powershell run",
        job.timeout,
        false,
    )
    .await?;

    let last_line = serde_json::json!(logs
        .lines()
        .last()
        .map(|x| ANSI_ESCAPE_RE.replace_all(x, "").to_string())
        .unwrap_or_else(String::new));
    Ok(to_raw_value(&last_line))
}
