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
    pub static ref BIN_BASH: String = std::env::var("BASH_PATH").unwrap_or_else(|_| {
        #[cfg(not(windows))]
        { "/bin/bash".to_string() }
        #[cfg(windows)]
        { "bash".to_string() }
    });
}
const NSJAIL_CONFIG_RUN_BASH_CONTENT: &str = include_str!("../nsjail/run.bash.config.proto");

#[cfg(feature = "dind")]
use crate::handle_child::run_future_with_polling_update_job_poller;

use crate::{
    common::{
        build_args_map, build_command_with_isolation, get_reserved_variables, read_file,
        read_file_content, resolve_nsjail_timeout, resolve_nsjail_tmp_mount_block,
        start_child_process, OccupancyMetrics, DEV_CONF_NSJAIL,
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

    // Docker runtime selection: if a Docker daemon is already provided — DOCKER_HOST
    // set, or the host socket mounted at /var/run/docker.sock — use it (legacy,
    // backwards compatible). DOCKER_HOST counts in all modes (a tcp:// dind stays
    // reachable through the jail since the sandbox doesn't isolate the network); a
    // mounted /var/run/docker.sock only counts when NOT under nsjail (the bash nsjail
    // proto never bind-mounts it into the jail, so it's unreachable there).
    //
    // Otherwise, for a `# docker` job, start a per-job rootless podman — BUT NOT under
    // nsjail. The per-job daemon runs OUTSIDE the jail and a `# docker` script can
    // bind-mount worker-visible paths through it, bypassing the jail's filesystem
    // isolation. Auto-providing that under nsjail would be a surprising hole: enabling
    // nsjail is an explicit "fully sandbox jobs" signal, so we refuse instead and make
    // the operator opt into docker explicitly (a provided DOCKER_HOST, or a worker
    // group without nsjail). Requires podman in the image (the *-full images).
    #[cfg(feature = "dind")]
    let docker_daemon_provided = std::env::var("DOCKER_HOST").is_ok()
        || (!nsjail && std::path::Path::new("/var/run/docker.sock").exists());
    #[cfg(feature = "dind")]
    let per_job_podman: Option<PerJobPodman> = if annotation.docker && !docker_daemon_provided {
        if nsjail {
            return Err(Error::ExecutionErr(
                "`# docker` jobs are not given an automatic container runtime under nsjail \
                 sandboxing: the per-job podman would run outside the sandbox and a docker \
                 script could bind-mount worker-visible paths through it, weakening the \
                 isolation you enabled nsjail for. Run docker jobs on a worker group without \
                 nsjail, or provide a Docker daemon explicitly via DOCKER_HOST (a \
                 network-reachable daemon — a mounted unix socket is not reachable inside the \
                 jail)."
                    .to_string(),
            ));
        }
        Some(start_per_job_podman(job_dir).await?)
    } else {
        None
    };

    // DOCKER_HOST handed to the script's docker CLI when using per-job podman: under
    // nsjail the per-job socket is bind-mounted into the jail at /tmp/podman.sock
    // (via {DOCKER_SOCK_MOUNT}); otherwise the script reaches the host socket directly.
    let docker_host_for_script: Option<String> = {
        #[cfg(feature = "dind")]
        {
            per_job_podman.as_ref().map(|p| {
                if nsjail {
                    "unix:///tmp/podman.sock".to_string()
                } else {
                    p.docker_host.clone()
                }
            })
        }
        #[cfg(not(feature = "dind"))]
        {
            None
        }
    };

    // Forward DOCKER_HOST to the bash script in docker mode: the per-job podman
    // socket when one was started, else the provided DOCKER_HOST / docker socket.
    let docker_envs: Vec<(&str, String)> = if annotation.docker {
        if let Some(dh) = &docker_host_for_script {
            vec![("DOCKER_HOST", dh.clone())]
        } else {
            ["DOCKER_HOST", "DOCKER_TLS_VERIFY", "DOCKER_CERT_PATH"]
                .iter()
                .filter_map(|k| std::env::var(k).ok().map(|v| (*k, v)))
                .collect()
        }
    } else {
        vec![]
    };

    // nsjail mount block that exposes the per-job podman socket inside the jail.
    let docker_sock_mount: String = {
        #[cfg(feature = "dind")]
        {
            per_job_podman
                .as_ref()
                .map(|p| {
                    format!(
                        "mount {{\n    src: \"{}\"\n    dst: \"/tmp/podman.sock\"\n    is_bind: true\n    rw: true\n}}\n",
                        p.host_sock
                    )
                })
                .unwrap_or_default()
        }
        #[cfg(not(feature = "dind"))]
        {
            String::new()
        }
    };

    let child = if nsjail {
        let nsjail_timeout =
            resolve_nsjail_timeout(conn, &job.workspace_id, job.id, job.timeout).await;
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_BASH_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{TRACING_PROXY_CA_CERT_PATH}", &*TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL)
                .replace(
                    "{TMP_MOUNT_BLOCK}",
                    &resolve_nsjail_tmp_mount_block(job_dir).await,
                )
                .replace("{DOCKER_SOCK_MOUNT}", &docker_sock_mount)
                .replace("{TIMEOUT}", &nsjail_timeout),
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
            .envs(
                get_proxy_envs_for_lang(
                    &ScriptLang::Bash,
                    job.kind,
                    &job.id,
                    &job.workspace_id,
                    conn,
                )
                .await?,
            )
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(docker_envs.iter().cloned())
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
            .envs(
                get_proxy_envs_for_lang(
                    &ScriptLang::Bash,
                    job.kind,
                    &job.id,
                    &job.workspace_id,
                    conn,
                )
                .await?,
            )
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .envs(docker_envs.iter().cloned())
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
            per_job_podman.as_ref().map(|p| p.docker_host.clone()),
        )
        .await;
        // per_job_podman drops here (after handle_docker_job completes), tearing down
        // the per-job podman instance so no container outlives the job.
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
/// Connect to the Docker daemon, respecting DOCKER_HOST if set (e.g. for dind sidecar),
/// otherwise falling back to the default unix socket at /var/run/docker.sock.
fn connect_docker(docker_host: Option<&str>) -> Result<bollard::Docker, bollard::errors::Error> {
    if let Some(dh) = docker_host {
        // Per-job rootless podman: connect to the job's own unix socket.
        let path = dh.strip_prefix("unix://").unwrap_or(dh);
        bollard::Docker::connect_with_unix(path, 120, bollard::API_DEFAULT_VERSION)
    } else if std::env::var("DOCKER_HOST").is_ok() {
        // DOCKER_HOST is set — use it (e.g. tcp://dind:2375 for docker-in-docker)
        bollard::Docker::connect_with_defaults()
    } else {
        // No DOCKER_HOST — use the unix socket (backward compatible default)
        bollard::Docker::connect_with_unix_defaults()
    }
}

// A per-job rootless podman instance used for docker jobs (any sandbox mode —
// nsjail, unshare, or none) when no Docker daemon is otherwise provided.
// The job's docker CLI and handle_docker_job (in the worker) both talk to this
// one instance's socket; because every container the job can create is registered
// in this instance's isolated storage, tearing it down (`podman system reset`,
// which also removes images and the subuid-owned overlay layers — see Drop)
// guarantees no container outlives the job — even detached or extra ones.
#[cfg(feature = "dind")]
struct PerJobPodman {
    dir: String,
    host_sock: String,
    docker_host: String,
    service: Option<std::process::Child>,
}

// Blocking teardown of a per-job podman instance: `system reset` removes ALL
// containers AND images in the per-job store and, crucially, deletes the
// subuid-owned overlay layers a plain `rm -rf` (or `rm -af`, which only touches
// containers) cannot — preventing a storage leak. Wrapped in `timeout` so a wedged
// runtime/storage can't hang the caller indefinitely.
#[cfg(feature = "dind")]
fn teardown_per_job_podman(dir: String, service: Option<std::process::Child>) {
    let storage = format!("{dir}/storage");
    let runroot = format!("{dir}/runroot");
    let _ = std::process::Command::new("timeout")
        .args([
            "30",
            "podman",
            "--root",
            &storage,
            "--runroot",
            &runroot,
            "system",
            "reset",
            "--force",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if let Some(mut child) = service {
        // Kill the whole process group: when the worker is root the daemon runs as a
        // `runuser` grandchild, so killing only the direct child would leak it.
        unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
        let _ = child.kill();
        let _ = child.wait();
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(feature = "dind")]
impl Drop for PerJobPodman {
    fn drop(&mut self) {
        let dir = std::mem::take(&mut self.dir);
        let service = self.service.take();
        // Teardown shells out (`podman system reset`) and can do non-trivial I/O, so
        // offload it to the blocking pool rather than stalling the Tokio worker thread
        // this Drop runs on. If we're not on a runtime (e.g. tests), run inline.
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.spawn_blocking(move || teardown_per_job_podman(dir, service));
            }
            Err(_) => teardown_per_job_podman(dir, service),
        }
    }
}

// Start a per-job rootless podman service in <job_dir>/podman with isolated
// storage, returning the host socket path. Inherits the system container config
// (/etc/containers) but overrides storage and the network backend via a per-job
// user config so both are job-scoped (and rootful podman elsewhere is untouched).
//
// SECURITY: this is only ever started in NON-nsjail modes (unshare/none) — caller
// refuses to auto-provide it under nsjail, since the daemon runs outside any sandbox.
// A `# docker` script controls this daemon over the Docker API and can bind-mount any
// WORLD-READABLE host path (e.g. `docker run -v /tmp/windmill/...` for other job dirs /
// caches, or `-v /proc` for process cmdlines/args) — so it is NOT a filesystem sandbox,
// and docker-capable workers stay a TRUSTED-TENANT capability (prefer dedicated/per-
// workspace docker workers on shared fleets). What IS protected, when the worker runs
// as root (the default) and the daemon is dropped to a non-root uid here: `0400` files
// like the worker's `/proc/<worker>/environ` (DATABASE_URL etc.) stay root-owned and
// unreadable by the uid-1000 container even via `-v /proc` or `--pid=host` (so don't
// pass secrets as command-line args), and a container escape is unprivileged.
#[cfg(feature = "dind")]
async fn start_per_job_podman(job_dir: &str) -> Result<PerJobPodman, Error> {
    use std::os::unix::process::CommandExt;
    let dir = format!("{job_dir}/podman");
    let xdg = format!("{dir}/xdg");
    tokio::fs::create_dir_all(&xdg).await.map_err(to_anyhow)?;
    // Default this rootless podman's container networking to slirp4netns: the
    // netavark bridge default fails rootless on hosts without the required
    // nftables setup. Scope it to THIS instance via a user (`$HOME`) config so we
    // don't touch the global `/etc/containers` config (which would also change
    // rootful podman behavior for anyone else in the image). `$HOME` is the job
    // dir below for both the root-drop and non-root launch paths.
    let containers_conf_dir = format!("{dir}/.config/containers");
    tokio::fs::create_dir_all(&containers_conf_dir)
        .await
        .map_err(to_anyhow)?;
    tokio::fs::write(
        format!("{containers_conf_dir}/containers.conf"),
        "[containers]\nnetns = \"slirp4netns\"\n",
    )
    .await
    .map_err(to_anyhow)?;
    let host_sock = format!("{dir}/podman.sock");
    let _ = tokio::fs::remove_file(&host_sock).await;
    let storage = format!("{dir}/storage");
    let runroot = format!("{dir}/runroot");
    let sock_url = format!("unix://{host_sock}");

    // If the worker is root (the default for docker workers), drop the per-job podman
    // to a non-root uid via `runuser` so it runs ROOTLESS. This keeps the worker's
    // secrets in `/proc/<worker>/environ` (DATABASE_URL etc.) root-owned and therefore
    // unreadable by the uid-1000 container even via `docker run --pid=host`, and makes
    // a container escape land as an unprivileged user. NB: it does NOT confine the
    // daemon's filesystem view — a script can still `docker run -v <worker path>` to
    // reach other job dirs / caches, so docker-capable workers remain trusted-tenant.
    // A non-root worker can't drop further (it runs podman rootless as itself, and a
    // `--pid=host` container shares its uid) — run the docker worker as root for the
    // /proc env protection. `process_group(0)` lets teardown kill the whole tree (with
    // `runuser`, podman is a grandchild, not the direct child).
    let mut cmd = if unsafe { libc::geteuid() } == 0 {
        let launch = format!(
            "set -e\nchown -R 1000:1000 '{dir}'\nexec runuser -u \"$(id -nu 1000)\" -- env \
             HOME='{dir}' XDG_RUNTIME_DIR='{xdg}' podman --root '{storage}' --runroot '{runroot}' \
             system service --time=0 '{sock_url}'\n"
        );
        let mut c = std::process::Command::new("bash");
        c.args(["-c", &launch]);
        c
    } else {
        let mut c = std::process::Command::new("podman");
        // HOME=<job dir> so podman picks up the per-job containers.conf written
        // above (netns override) and keeps its config/cache job-scoped.
        c.env("HOME", &dir);
        c.args([
            "--root",
            &storage,
            "--runroot",
            &runroot,
            "system",
            "service",
            "--time=0",
            &sock_url,
        ]);
        c
    };
    let service = cmd
        .process_group(0)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| {
            Error::ExecutionErr(format!(
                "no Docker daemon provided (DOCKER_HOST/socket) and failed to start the per-job \
                 podman runtime: {e}. Use a windmill *-full image (ships podman), or provide a \
                 Docker daemon via DOCKER_HOST or a mounted /var/run/docker.sock."
            ))
        })?;
    // Wait (up to ~10s) for the rootless podman service socket to appear.
    for _ in 0..50 {
        if tokio::fs::metadata(&host_sock).await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    Ok(PerJobPodman {
        dir,
        docker_host: format!("unix://{host_sock}"),
        host_sock,
        service: Some(service),
    })
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
    // Some(socket) when a per-job rootless podman runs the container;
    // None to use the provided DOCKER_HOST / default socket (legacy).
    docker_host: Option<String>,
) -> Result<Box<RawValue>, Error> {
    use crate::job_logger::append_logs_with_compaction;

    let client = connect_docker(docker_host.as_deref()).map_err(to_anyhow)?;

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
    let docker_host_logs = docker_host.clone();
    let logs = tokio::spawn(async move {
        let client = connect_docker(docker_host_logs.as_deref()).map_err(to_anyhow);
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

    let mem_client = connect_docker(docker_host.as_deref()).map_err(to_anyhow);
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
