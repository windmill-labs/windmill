use std::{collections::HashMap, process::Stdio};

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

use windmill_queue::{
    append_logs, CanceledBy, MiniPulledJob, INIT_SCRIPT_PATH_PREFIX, PERIODIC_SCRIPT_PATH_PREFIX,
};

lazy_static::lazy_static! {
    pub static ref BIN_BASH: String = std::env::var("BASH_PATH").unwrap_or_else(|_| "/bin/bash".to_string());
}
const NSJAIL_CONFIG_RUN_BASH_CONTENT: &str = include_str!("../nsjail/run.bash.config.proto");

#[cfg(feature = "dind")]
use crate::handle_child::run_future_with_polling_update_job_poller;

use crate::{
    common::{
        build_args_map, build_command_with_isolation, get_reserved_variables, read_file,
        read_file_content, start_child_process, OccupancyMetrics, DEV_CONF_NSJAIL,
    },
    get_proxy_envs_for_lang,
    handle_child::handle_child,
    is_sandboxing_enabled, DISABLE_NUSER, HOME_ENV, NSJAIL_AVAILABLE, NSJAIL_PATH, PATH_ENV,
    TRACING_PROXY_CA_CERT_PATH,
};
use windmill_common::client::AuthedClient;
use windmill_common::scripts::ScriptLang;

lazy_static::lazy_static! {

    pub static ref ANSI_ESCAPE_RE: Regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
}

fn raw_to_string(x: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(x) {
        Ok(serde_json::Value::String(x)) => x,
        Ok(x) => serde_json::to_string(&x).unwrap_or_else(|_| String::new()),
        _ => String::new(),
    }
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

    // Check if sandbox annotation is used but nsjail is not available
    if annotation.sandbox && NSJAIL_AVAILABLE.is_none() {
        return Err(Error::ExecutionErr(
            "Script has #sandbox annotation but nsjail is not available on this worker. \
            Please ensure nsjail is installed or remove the #sandbox annotation."
                .to_string(),
        ));
    }

    let mut logs1 = "\n\n--- BASH CODE EXECUTION ---\n".to_string();
    if annotation.docker {
        logs1.push_str("docker mode\n");
    }
    if annotation.sandbox {
        logs1.push_str("sandbox mode (nsjail)\n");
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

    // Check if this is a regular job (not init or periodic script)
    // Init/periodic scripts need full system access without isolation
    let is_regular_job = job
        .runnable_path
        .as_ref()
        .map(|x| {
            !x.starts_with(INIT_SCRIPT_PATH_PREFIX) && !x.starts_with(PERIODIC_SCRIPT_PATH_PREFIX)
        })
        .unwrap_or(true);

    // Use nsjail if globally enabled OR if script has #sandbox annotation
    let nsjail = (is_sandboxing_enabled() || annotation.sandbox) && is_regular_job;
    let child = if nsjail {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_BASH_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{TRACING_PROXY_CA_CERT_PATH}", TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL),
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
            .envs(get_proxy_envs_for_lang(&ScriptLang::Bash).await?)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        let mut cmd_args = vec!["wrapper.sh"];
        cmd_args.extend(&args);
        // Only apply unshare isolation for regular jobs, not init/periodic scripts
        let mut bash_cmd = if is_regular_job {
            build_command_with_isolation(
                BIN_BASH.as_str(),
                &cmd_args.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
            )
        } else {
            let mut cmd = Command::new(BIN_BASH.as_str());
            cmd.args(&cmd_args);
            cmd
        };
        bash_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(get_proxy_envs_for_lang(&ScriptLang::Bash).await?)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(bash_cmd, BIN_BASH.as_str(), false).await?
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
                    biased;
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
    } else {
        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(5), logs).await;
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
