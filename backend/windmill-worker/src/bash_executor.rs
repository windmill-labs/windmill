use std::{collections::HashMap, process::Stdio};

use itertools::Itertools;
use regex::Regex;
use serde_json::{json, Value};
use tokio::process::Command;
use windmill_common::{error::Error, jobs::QueuedJob};

const NSJAIL_CONFIG_RUN_BASH_CONTENT: &str = include_str!("../nsjail/run.bash.config.proto");

use crate::{
    common::{
        get_reserved_variables, handle_child, read_file, read_file_content, set_logs,
        transform_json_value, write_file,
    },
    AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, NSJAIL_PATH, PATH_ENV,
    TZ_ENV,
};

lazy_static::lazy_static! {

    pub static ref ANSI_ESCAPE_RE: Regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_bash_job(
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    content: &str,
    job_dir: &str,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
) -> Result<serde_json::Value, Error> {
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

    let client = client.get_authed().await;
    let hm = match transform_json_value(
        "args",
        &client,
        &job.workspace_id,
        job.args.clone().unwrap_or_else(|| json!({})),
    )
    .await?
    {
        Value::Object(ref hm) => hm.clone(),
        _ => serde_json::Map::new(),
    };
    let args_owned = windmill_parser_bash::parse_bash_sig(&content)?
        .args
        .iter()
        .map(|arg| {
            hm.get(&arg.name)
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => serde_json::to_string(v).ok(),
                })
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
        Command::new(NSJAIL_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        let mut cmd_args = vec!["main.sh"];
        cmd_args.extend(&args);
        Command::new("/bin/bash")
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
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
            return Ok(json!(result));
        }
    }

    //for now bash jobs have an empty result object
    let last_line = serde_json::json!(logs
        .lines()
        .last()
        .map(|x| ANSI_ESCAPE_RE.replace_all(x, "").to_string())
        .unwrap_or_else(String::new));
    Ok(last_line)
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_powershell_job(
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    content: &str,
    job_dir: &str,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
) -> Result<serde_json::Value, Error> {
    logs.push_str("\n\n--- POWERSHELL CODE EXECUTION ---\n");
    set_logs(logs, &job.id, db).await;
    let pwsh_args = {
        let client = client.get_authed().await;
        let hm = match transform_json_value(
            "args",
            &client,
            &job.workspace_id,
            job.args.clone().unwrap_or_else(|| json!({})),
        )
        .await?
        {
            Value::Object(ref hm) => hm.clone(),
            _ => serde_json::Map::new(),
        };

        let args_owned = windmill_parser_bash::parse_powershell_sig(&content)?
            .args
            .iter()
            .map(|arg| {
                (
                    arg.name.clone(),
                    hm.get(&arg.name)
                        .and_then(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            _ => serde_json::to_string(v).ok(),
                        })
                        .unwrap_or_else(String::new),
                )
            })
            .collect::<Vec<(String, String)>>();
        args_owned
            .iter()
            .map(|(n, v)| format!("--{n} {v}"))
            .join(" ")
    };

    let content = content
        .replace('$', r"\$") // escape powershell variables
        .replace("`", r"\`"); // escape powershell backticks

    write_file(job_dir, "main.sh", &format!("set -e\ncat > script.ps1 << EOF\n{content}\nEOF\npwsh -File script.ps1 {pwsh_args}\necho \"\"\nsleep 0.02")).await?;
    let token = client.get_token().await;
    let mut reserved_variables = get_reserved_variables(job, &token, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

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
        let cmd_args = vec!["--config", "run.config.proto", "--", "/bin/bash", "main.sh"];
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
        let cmd_args = vec!["main.sh"];
        Command::new("/bin/bash")
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
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        &job.workspace_id,
        "bash/powershell run",
        job.timeout,
        false,
    )
    .await?;

    let last_line = serde_json::json!(logs
        .lines()
        .last()
        .map(|x| ANSI_ESCAPE_RE.replace_all(x, "").to_string())
        .unwrap_or_else(String::new));
    Ok(last_line)
}
