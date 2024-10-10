use std::{collections::HashMap, fs, process::Stdio};

use regex::Regex;
use serde_json::{json, value::RawValue};
use sqlx::types::Json;
use tokio::process::Command;
use windmill_common::{
    error::Error,
    jobs::QueuedJob,
    worker::{to_raw_value, write_file},
};
use windmill_queue::{append_logs, CanceledBy};

lazy_static::lazy_static! {
    pub static ref BIN_BASH: String = std::env::var("BASH_PATH").unwrap_or_else(|_| "/bin/bash".to_string());
}
const NSJAIL_CONFIG_RUN_BASH_CONTENT: &str = include_str!("../nsjail/run.bash.config.proto");
const NSJAIL_CONFIG_RUN_POWERSHELL_CONTENT: &str =
    include_str!("../nsjail/run.powershell.config.proto");

lazy_static::lazy_static! {
    static ref RE_POWERSHELL_IMPORTS: Regex = Regex::new(r#"^Import-Module\s+(?:-Name\s+)?"?([^-\s"]+)"?"#).unwrap();
}

use crate::{
    common::{
        build_args_map, get_reserved_variables, read_file, read_file_content, start_child_process,
        OccupancyMetrics,
    },
    handle_child::handle_child,
    AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, NSJAIL_PATH, PATH_ENV,
    POWERSHELL_CACHE_DIR, POWERSHELL_PATH, TZ_ENV,
};

#[cfg(windows)]
use crate::SYSTEM_ROOT;

lazy_static::lazy_static! {

    pub static ref ANSI_ESCAPE_RE: Regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_bash_job(
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
    occupancy_metrics: &mut OccupancyMetrics,
) -> Result<Box<RawValue>, Error> {
    let logs1 = "\n\n--- BASH CODE EXECUTION ---\n".to_string();
    append_logs(&job.id, &job.workspace_id, logs1, db).await;

    write_file(job_dir, "main.sh", &format!("set -e\n{content}"))?;
    write_file(
        job_dir,
        "wrapper.sh",
        &format!("set -o pipefail\nset -e\nmkfifo bp\ncat bp | tail -1 > ./result2.out &\n {bash} ./main.sh \"$@\" 2>&1 | tee bp\nwait $!", bash = BIN_BASH.as_str()),
    )?;

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
    let _ = write_file(job_dir, "result.json", "")?;
    let _ = write_file(job_dir, "result.out", "")?;
    let _ = write_file(job_dir, "result2.out", "")?;

    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_BASH_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )?;
        let mut cmd_args = vec![
            "--config",
            "run.config.proto",
            "--",
            BIN_BASH.as_str(),
            "wrapper.sh",
        ];
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
        let mut cmd_args = vec!["wrapper.sh"];
        cmd_args.extend(&args);
        let mut bash_cmd = Command::new(BIN_BASH.as_str());
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
        start_child_process(bash_cmd, BIN_BASH.as_str()).await?
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
        "bash run",
        job.timeout,
        true,
        &mut Some(occupancy_metrics),
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

    let result_out_path2 = format!("{job_dir}/result2.out");
    if tokio::fs::metadata(&result_out_path2).await.is_ok() {
        let result = read_file_content(&result_out_path2)
            .await?
            .trim()
            .to_string();
        return Ok(to_raw_value(&json!(result)));
    }

    Ok(to_raw_value(&json!(
        "No result.out, result2.out or result.json found"
    )))
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
    occupancy_metrics: &mut OccupancyMetrics,
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

    #[cfg(windows)]
    let split_char = '\\';

    #[cfg(unix)]
    let split_char = '/';

    let installed_modules = fs::read_dir(POWERSHELL_CACHE_DIR)?
        .filter_map(|x| {
            x.ok().map(|x| {
                x.path()
                    .display()
                    .to_string()
                    .split(split_char)
                    .last()
                    .unwrap_or_default()
                    .to_lowercase()
            })
        })
        .collect::<Vec<String>>();

    let mut install_string: String = String::new();
    let mut logs1 = String::new();
    for line in content.lines() {
        for cap in RE_POWERSHELL_IMPORTS.captures_iter(line) {
            let module = cap.get(1).unwrap().as_str();
            if !installed_modules.contains(&module.to_lowercase()) {
                logs1.push_str(&format!("\n{} not found in cache", module.to_string()));
                // instead of using Install-Module, we use Save-Module so that we can specify the installation path
                install_string.push_str(&format!(
                    "Save-Module -Path {} -Force {};",
                    POWERSHELL_CACHE_DIR, module
                ));
            } else {
                logs1.push_str(&format!("\n{} found in cache", module.to_string()));
            }
        }
    }

    if !install_string.is_empty() {
        logs1.push_str("\n\nInstalling modules...");
        append_logs(&job.id, &job.workspace_id, logs1, db).await;
        let child = Command::new(POWERSHELL_PATH.as_str())
            .args(&["-Command", &install_string])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        handle_child(
            &job.id,
            db,
            mem_peak,
            canceled_by,
            child,
            false,
            worker_name,
            &job.workspace_id,
            "powershell install",
            job.timeout,
            false,
            &mut Some(occupancy_metrics),
        )
        .await?;
    }

    let mut logs2 = "".to_string();
    logs2.push_str("\n\n--- POWERSHELL CODE EXECUTION ---\n");
    append_logs(&job.id, &job.workspace_id, logs2, db).await;

    // make sure default (only allhostsallusers) modules are loaded, disable autoload (cache can be large to explore especially on cloud) and add /tmp/windmill/cache to PSModulePath
    #[cfg(unix)]
    let profile = format!(
        "$PSModuleAutoloadingPreference = 'None'
$PSModulePathBackup = $env:PSModulePath
$env:PSModulePath = \"$PSHome/Modules\"
Get-Module -ListAvailable | Import-Module
$env:PSModulePath = \"{}:$PSModulePathBackup\"",
        POWERSHELL_CACHE_DIR
    );

    #[cfg(windows)]
    let profile = format!(
        "$PSModuleAutoloadingPreference = 'None'
$PSModulePathBackup = $env:PSModulePath
$env:PSModulePath = \"C:\\Program Files\\PowerShell\\7\\Modules\"
Get-Module -ListAvailable | Import-Module
$env:PSModulePath = \"{};$PSModulePathBackup\"",
        POWERSHELL_CACHE_DIR
    );

    // NOTE: powershell error handling / termination is quite tricky compared to bash
    // here we're trying to catch terminating errors and propagate the exit code
    // to the caller such that the job will be marked as failed. It's up to the user
    // to catch specific errors in their script not caught by the below as there is no
    // generic set -eu as in bash
    let strict_termination_start = "$ErrorActionPreference = 'Stop'\n\
        Set-StrictMode -Version Latest\n\
        try {\n";

    let strict_termination_end = "\n\
        } catch {\n\
            Write-Output \"An error occurred:\n\"\
            Write-Output $_
            exit 1\n\
        }\n";

    // make sure param() is first
    let param_match = windmill_parser_bash::RE_POWERSHELL_PARAM.find(&content);
    let content: String = if let Some(param_match) = param_match {
        let param_match = param_match.as_str();
        format!(
            "{}\n{}\n{}\n{}\n{}",
            param_match,
            profile,
            strict_termination_start,
            content.replace(param_match, ""),
            strict_termination_end
        )
    } else {
        format!("{}\n{}", profile, content)
    };

    write_file(job_dir, "main.ps1", content.as_str())?;

    #[cfg(unix)]
    write_file(
        job_dir,
        "wrapper.sh",
        &format!("set -o pipefail\nset -e\nmkfifo bp\ncat bp | tail -1 > ./result2.out &\n{} -F ./main.ps1 \"$@\" 2>&1 | tee bp\nwait $!", POWERSHELL_PATH.as_str()),
    )?;

    #[cfg(windows)]
    write_file(
        job_dir,
        "wrapper.ps1",
        &format!(
            "param([string[]]$args)\n\
    $ErrorActionPreference = 'Stop'\n\
    $pipe = New-TemporaryFile\n\
    & \"{}\" -File ./main.ps1 @args 2>&1 | Tee-Object -FilePath $pipe\n\
    Get-Content -Path $pipe | Select-Object -Last 1 | Set-Content -Path './result2.out'\n\
    Remove-Item $pipe\n\
    exit $LASTEXITCODE\n",
            POWERSHELL_PATH.as_str()
        ),
    )?;

    let token = client.get_token().await;
    let mut reserved_variables = get_reserved_variables(job, &token, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let _ = write_file(job_dir, "result.json", "")?;
    let _ = write_file(job_dir, "result.out", "")?;
    let _ = write_file(job_dir, "result2.out", "")?;

    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_POWERSHELL_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{CACHE_DIR}", POWERSHELL_CACHE_DIR),
        )?;
        let mut cmd_args = vec![
            "--config",
            "run.config.proto",
            "--",
            BIN_BASH.as_str(),
            "wrapper.sh",
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
        let mut cmd;
        let mut cmd_args;

        #[cfg(unix)]
        {
            cmd_args = vec!["wrapper.sh"];
            cmd_args.extend(pwsh_args.iter().map(|x| x.as_str()));
            cmd = Command::new(BIN_BASH.as_str());
        }

        #[cfg(windows)]
        {
            cmd_args = vec![r".\wrapper.ps1".to_string()];
            cmd_args.extend(pwsh_args.iter().map(|x| x.replace("--", "-")));
            cmd = Command::new(POWERSHELL_PATH.as_str());
        }

        cmd.current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("TZ", TZ_ENV.as_str())
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .args(&cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            cmd.env("SystemRoot", SYSTEM_ROOT.as_str())
                .env(
                    "LOCALAPPDATA",
                    std::env::var("LOCALAPPDATA")
                        .unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str())),
                )
                .env(
                    "ProgramData",
                    std::env::var("ProgramData")
                        .unwrap_or_else(|_| String::from("C:\\ProgramData")),
                )
                .env(
                    "ProgramFiles",
                    std::env::var("ProgramFiles")
                        .unwrap_or_else(|_| String::from("C:\\Program Files")),
                )
                .env(
                    "ProgramFiles(x86)",
                    std::env::var("ProgramFiles(x86)")
                        .unwrap_or_else(|_| String::from("C:\\Program Files (x86)")),
                )
                .env(
                    "ProgramW6432",
                    std::env::var("ProgramW6432")
                        .unwrap_or_else(|_| String::from("C:\\Program Files")),
                )
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                )
                .env(
                    "PATHEXT",
                    std::env::var("PATHEXT").unwrap_or_else(|_| {
                        String::from(".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC;.CPL")
                    }),
                );
        }

        cmd.spawn()?
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
        "powershell run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
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

    let result_out_path2 = format!("{job_dir}/result2.out");
    if tokio::fs::metadata(&result_out_path2).await.is_ok() {
        let result = read_file_content(&result_out_path2)
            .await?
            .trim()
            .to_string();
        return Ok(to_raw_value(&json!(result)));
    }

    Ok(to_raw_value(&json!(
        "No result.out, result2.out or result.json found"
    )))
}
