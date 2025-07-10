use std::{collections::HashMap, fs, process::Stdio};

#[cfg(feature = "dind")]
use bollard::container::{
    KillContainerOptions, RemoveContainerOptions, StatsOptions, StopContainerOptions,
};
#[cfg(feature = "dind")]
use futures::{stream, StreamExt, TryStreamExt};
use regex::Regex;
use serde_json::{json, value::RawValue};
use sqlx::types::Json;
use tokio::process::Command;

#[cfg(feature = "dind")]
use uuid::Uuid;
use windmill_common::{
    error::Error,
    worker::{to_raw_value, write_file, Connection},
};

#[cfg(feature = "dind")]
use windmill_common::error::to_anyhow;

use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

lazy_static::lazy_static! {
    pub static ref BIN_BASH: String = std::env::var("BASH_PATH").unwrap_or_else(|_| "/bin/bash".to_string());
}
const NSJAIL_CONFIG_RUN_BASH_CONTENT: &str = include_str!("../nsjail/run.bash.config.proto");
const NSJAIL_CONFIG_RUN_POWERSHELL_CONTENT: &str =
    include_str!("../nsjail/run.powershell.config.proto");

lazy_static::lazy_static! {
    static ref RE_POWERSHELL_IMPORTS: Regex = Regex::new(r#"^Import-Module\s+(?:-Name\s+)?"?([^-\s"]+)"?"#).unwrap();
}

#[cfg(feature = "dind")]
use crate::handle_child::run_future_with_polling_update_job_poller;

use crate::{
    common::{
        build_args_map, get_reserved_variables, read_file, read_file_content, start_child_process,
        OccupancyMetrics,
    },
    handle_child::handle_child,
    DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, NSJAIL_PATH, PATH_ENV, POWERSHELL_CACHE_DIR,
    POWERSHELL_PATH, PROXY_ENVS, TZ_ENV,
};
use windmill_common::client::AuthedClient;

#[cfg(windows)]
use crate::SYSTEM_ROOT;

lazy_static::lazy_static! {

    pub static ref ANSI_ESCAPE_RE: Regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_bash_job(
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &MiniPulledJob,
    conn: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    content: &str,
    job_dir: &str,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    occupancy_metrics: &mut OccupancyMetrics,
    _killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
) -> Result<Box<RawValue>, Error> {
    let annotation = windmill_common::worker::BashAnnotations::parse(&content);

    let mut logs1 = "\n\n--- BASH CODE EXECUTION ---\n".to_string();
    if annotation.docker {
        logs1.push_str("docker mode\n");
    }
    append_logs(&job.id, &job.workspace_id, logs1, &conn).await;

    write_file(job_dir, "main.sh", &format!("set -e\n{content}"))?;
    let script = format!(
        r#"
set -o pipefail
set -e

# Function to kill child processes
cleanup() {{
    echo "Terminating child processes..."

    # Ignore SIGTERM and SIGINT
    trap '' SIGTERM SIGINT

    rm -f bp 2>/dev/null

    # Kill the process group of the script (negative PID value)
    pkill -P $$ 2>/dev/null || true
    exit
}}


# Trap SIGTERM (or other signals) and call cleanup function
trap cleanup SIGTERM SIGINT

# Create a named pipe
mkfifo bp

# Start background processes
cat bp | tail -1 >> ./result2.out &
tail_pid=$!

# Run main.sh in the same process group
{bash} ./main.sh "$@" 2>&1 | tee bp &
pid=$!

# Wait for main.sh to finish and capture its exit status
wait $pid
exit_status=$?

# Ensure tail has finished before cleanup
wait $tail_pid 2>/dev/null || true

# Clean up the named pipe and background processes
rm -f bp
pkill -P $$ || true

# Exit with the captured status
exit $exit_status
"#,
        bash = BIN_BASH.as_str(),
    );
    write_file(job_dir, "wrapper.sh", &script)?;

    let mut reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let args = build_args_map(job, client, conn).await?.map(Json);
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

    let nsjail = !*DISABLE_NSJAIL
        && job
            .runnable_path
            .as_ref()
            .map(|x| !x.starts_with("init_script_") && !x.starts_with("periodic_init_script_"))
            .unwrap_or(true);
    let child = if nsjail {
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
            .envs(PROXY_ENVS.clone())
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
        conn,
        mem_peak,
        canceled_by,
        child,
        nsjail,
        worker_name,
        &job.workspace_id,
        "bash run",
        job.timeout,
        true,
        &mut Some(occupancy_metrics),
        None,
    )
    .await?;

    #[cfg(feature = "dind")]
    if annotation.docker {
        return handle_docker_job(
            job.id,
            &job.workspace_id,
            conn,
            job.timeout,
            mem_peak,
            canceled_by,
            worker_name,
            occupancy_metrics,
            _killpill_rx,
        )
        .await;
    }

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

#[cfg(feature = "dind")]
async fn rm_container(client: &bollard::Docker, container_id: &str) {
    if let Err(e) = client
        .remove_container(
            container_id,
            Some(RemoveContainerOptions { force: true, ..Default::default() }),
        )
        .await
    {
        tracing::error!("Error removing container: {:?}", e);
    }
}

#[cfg(feature = "dind")]
async fn handle_docker_job(
    job_id: Uuid,
    workspace_id: &str,
    conn: &Connection,
    job_timeout: Option<i32>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
) -> Result<Box<RawValue>, Error> {
    use crate::job_logger::append_logs_with_compaction;

    let client = bollard::Docker::connect_with_unix_defaults().map_err(to_anyhow)?;

    let container_id = job_id.to_string();
    let inspected = client.inspect_container(&container_id, None).await;

    if inspected.is_err() || inspected.unwrap().state.is_none() {
        return Ok(to_raw_value(&format!(
            "Container not found at {job_id}, you must use --name and not --rm it"
        )));
    }

    let wait_f = async {
        let waited = client
            .wait_container::<String>(&container_id, None)
            .try_collect::<Vec<_>>()
            .await;
        match waited {
            Ok(wait) => Ok(wait.first().map(|x| x.status_code)),
            Err(bollard::errors::Error::DockerResponseServerError { status_code, message }) => {
                append_logs(&job_id, &workspace_id, &format!(": {message}"), conn).await;
                Ok(Some(status_code as i64))
            }
            Err(bollard::errors::Error::DockerContainerWaitError { error, code }) => {
                append_logs(&job_id, &workspace_id, &format!("{error}"), conn).await;
                Ok(Some(code as i64))
            }
            Err(e) => {
                tracing::error!("Error waiting for container: {:?}", e);
                Err(Error::ExecutionErr(format!(
                    "Error waiting for container: {:?}",
                    e
                )))
            }
        }
    };

    let ncontainer_id = container_id.to_string();
    let w_id = workspace_id.to_string();
    let j_id = job_id.clone();
    let conn2 = conn.clone();
    let worker_name2 = worker_name.to_string();
    let (tx, mut rx) = tokio::sync::broadcast::channel::<()>(1);
    let workspace_id2 = workspace_id.to_string();
    let mut killpill_rx = killpill_rx.resubscribe();
    let logs = tokio::spawn(async move {
        let client = bollard::Docker::connect_with_unix_defaults().map_err(to_anyhow);
        if let Ok(client) = client {
            let mut log_stream = client.logs(
                &ncontainer_id,
                Some(bollard::container::LogsOptions {
                    follow: true,
                    stdout: true,
                    stderr: true,
                    tail: "all",
                    ..Default::default()
                }),
            );
            append_logs(
                &job_id,
                &workspace_id2,
                "\ndocker logs stream started\n",
                &conn2,
            )
            .await;
            loop {
                tokio::select! {
                    log = log_stream.next() => {
                        match log {
                            Some(Ok(log)) => {
                                match &conn2 {
                                    Connection::Sql(db) => {
                                        append_logs_with_compaction(
                                            &j_id,
                                            &w_id,
                                            &log.to_string(),
                                            &db,
                                            &worker_name2,
                                        )
                                        .await;
                                    }
                                    c @ Connection::Http(_) => {
                                        append_logs(&j_id, &w_id, &log.to_string(), &c).await;
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                tracing::error!("Error getting logs: {:?}", e);
                            }
                            _ => {
                                tracing::info!("End of docker logs stream");
                                return
                            }
                        };
                    },
                    _ = killpill_rx.recv() => {
                        tracing::error!("killing container after receving killpill");
                        if let Err(e) = client
                        .stop_container(&ncontainer_id, Some(StopContainerOptions { t: 3 }))
                        .await
                            {
                                tracing::error!("Error stopping container: {:?}", e);
                            }
                            return
                    },
                    _ = rx.recv() => {
                        return
                    }
                }
            }
        }
    });

    let mem_client = bollard::Docker::connect_with_unix_defaults().map_err(to_anyhow);
    let ncontainer_id = container_id.clone();
    let result = run_future_with_polling_update_job_poller(
        job_id,
        job_timeout,
        conn,
        mem_peak,
        canceled_by,
        wait_f,
        worker_name,
        workspace_id,
        &mut Some(occupancy_metrics),
        Box::pin(match mem_client {
            Ok(client) => client
                .stats(
                    &ncontainer_id,
                    Some(StatsOptions { stream: true, one_shot: false }),
                )
                .map(|x| {
                    x.map(|x| x.memory_stats.usage.map(|m| m / 1024).unwrap_or_default() as i32)
                        .unwrap_or_default()
                })
                .boxed(),
            _ => stream::once(async { 0 }).boxed(),
        }),
    )
    .await;

    if let Err(e) = result {
        if !logs.is_finished() {
            let _ = tx.send(());
            let _ = logs.await;
        }
        if container_is_alive(&client, &container_id).await {
            kill_container(&client, &container_id, "SIGINT").await;
            if container_is_alive(&client, &container_id).await {
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                if let Err(e) = client
                    .stop_container(&container_id, Some(StopContainerOptions { t: 3 }))
                    .await
                {
                    tracing::error!("Error stopping container: {:?}", e);
                }
            }
        }
        rm_container(&client, &container_id).await;

        return Err(e);
    }

    rm_container(&client, &container_id).await;

    let result = result.unwrap();

    if result.is_some_and(|x| x > 0) {
        return Err(Error::ExecutionErr(format!(
            "Docker job completed with unsuccessful exit status: {}",
            result.unwrap()
        )));
    }
    return Ok(to_raw_value(&json!(format!(
        "Docker job completed with success exit status"
    ))));
}

#[cfg(feature = "dind")]
async fn kill_container(client: &bollard::Docker, container_id: &str, signal: &str) {
    if let Err(e) = client
        .kill_container(&container_id, Some(KillContainerOptions { signal }))
        .await
    {
        tracing::error!("Error killing container with signal {signal}: {:?}", e);
    }
}

#[cfg(feature = "dind")]
async fn container_is_alive(client: &bollard::Docker, container_id: &str) -> bool {
    let inspect = client.inspect_container(container_id, None).await;
    if let Ok(inspect) = inspect {
        let r = inspect
            .state
            .map(|x| x.running.unwrap_or_default())
            .unwrap_or_default();
        tracing::error!("Container {container_id} is alive: {r}");
        r
    } else {
        false
    }
}

fn raw_to_string(x: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(x) {
        Ok(serde_json::Value::String(x)) => x,
        Ok(x) => serde_json::to_string(&x).unwrap_or_else(|_| String::new()),
        _ => String::new(),
    }
}

const POWERSHELL_INSTALL_CODE: &str = r#"
$availableModules = Get-Module -ListAvailable
$path = '{path}'

$moduleNames = @({modules})

foreach ($module in $moduleNames) {
    if (-not ($availableModules | Where-Object { $_.Name -eq $module })) {
        Write-Host "Installing module $module..."
        Save-Module -Name $module -Path $path -Force
    } else {
        Write-Host "Module $module already installed"
    }
}
"#;

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_powershell_job(
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &MiniPulledJob,
    db: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    content: &str,
    job_dir: &str,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> Result<Box<RawValue>, Error> {
    let pwsh_args = {
        let args = build_args_map(job, client, &db).await?.map(Json);
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

    let mut modules_to_install: Vec<String> = Vec::new();
    let mut logs1 = String::new();
    for line in content.lines() {
        for cap in RE_POWERSHELL_IMPORTS.captures_iter(line) {
            let module = cap.get(1).unwrap().as_str();
            if !installed_modules.contains(&module.to_lowercase()) {
                modules_to_install.push(module.to_string());
            } else {
                logs1.push_str(&format!("\n{} found in cache", module.to_string()));
            }
        }
    }

    if !logs1.is_empty() {
        append_logs(&job.id, &job.workspace_id, logs1, db).await;
    }

    if !modules_to_install.is_empty() {
        let install_string = POWERSHELL_INSTALL_CODE
            .replace("{path}", POWERSHELL_CACHE_DIR)
            .replace(
                "{modules}",
                &modules_to_install
                    .iter()
                    .map(|x| format!("'{x}'"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        let mut cmd = Command::new(POWERSHELL_PATH.as_str());
        cmd.args(&["-Command", &install_string])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = start_child_process(cmd, POWERSHELL_PATH.as_str()).await?;

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
            None,
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

    let mut reserved_variables =
        get_reserved_variables(job, &client.token, db, parent_runnable_path).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let _ = write_file(job_dir, "result.json", "")?;
    let _ = write_file(job_dir, "result.out", "")?;
    let _ = write_file(job_dir, "result2.out", "")?;

    let nsjail = !*DISABLE_NSJAIL
        && job
            .runnable_path
            .as_ref()
            .map(|x| !x.starts_with("init_script_") && !x.starts_with("periodic_init_script_"))
            .unwrap_or(true);
    let child = if nsjail {
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
        let mut cmd = Command::new(NSJAIL_PATH.as_str());
        cmd.current_dir(job_dir)
            .env_clear()
            .envs(PROXY_ENVS.clone())
            .envs(reserved_variables)
            .env("TZ", TZ_ENV.as_str())
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        start_child_process(cmd, NSJAIL_PATH.as_str()).await?
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
                )
                .env("USERPROFILE", crate::USERPROFILE_ENV.as_str());
        }

        start_child_process(cmd, POWERSHELL_PATH.as_str()).await?
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
        None,
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
