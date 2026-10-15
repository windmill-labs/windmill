//! Out-of-process DuckDB: runs a job's blocks in `windmill_duckdb_runner` instead of the
//! worker's own process, inside nsjail when the worker jails jobs. Off unless
//! `DUCKDB_JAILED_RUNNER` is set; prototype, see the PR description for the measurements.
//!
//! The runner calls the same FFI entry points the worker dlopens, and hands back the exact
//! string they return, so result collection and error decoding stay in `duckdb_executor`.

use std::{
    collections::HashSet,
    process::Stdio,
    sync::{LazyLock, Mutex},
};

use serde::Serialize;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use windmill_common::error::{Error, Result};

use crate::{
    common::{resolve_nsjail_tmp_mount_block, DEV_CONF_NSJAIL},
    is_sandboxing_enabled, DISABLE_NUSER, HOME_ENV, NSJAIL_AVAILABLE, NSJAIL_PATH, PATH_ENV,
};

const NSJAIL_CONFIG_RUN_DUCKDB_CONTENT: &str = include_str!("../nsjail/run.duckdb.config.proto");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Requested {
    Off,
    /// Inside nsjail. Falls back to in-process when the worker does not jail jobs.
    Jailed,
    /// A plain child process with no jail: separates the cost of the process from the cost of
    /// the jail in benchmarks. Never a deployment setting.
    Process,
}

static REQUESTED: LazyLock<Requested> = LazyLock::new(|| {
    match std::env::var("DUCKDB_JAILED_RUNNER")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "true" | "1" | "nsjail" => Requested::Jailed,
        "process" => Requested::Process,
        _ => Requested::Off,
    }
});

/// `DUCKDB_JAILED_RUNNER_FENCE=false` drops the SQL-level fence (`duckdb_isolation_ee`) and
/// `lock_temp_directory` inside the jail, leaving confinement to nsjail alone. Never applies
/// outside nsjail.
static FENCE_IN_JAIL: LazyLock<bool> = LazyLock::new(|| {
    !matches!(
        std::env::var("DUCKDB_JAILED_RUNNER_FENCE")
            .unwrap_or_default()
            .trim(),
        "false" | "0"
    )
});

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RunnerMode {
    Jailed,
    Process,
}

/// Decided once per job, so every connection a job opens runs the same way.
pub(crate) fn runner_mode() -> Option<RunnerMode> {
    match *REQUESTED {
        Requested::Off => None,
        Requested::Process => Some(RunnerMode::Process),
        Requested::Jailed
            if cfg!(unix) && is_sandboxing_enabled() && NSJAIL_AVAILABLE.is_some() =>
        {
            Some(RunnerMode::Jailed)
        }
        Requested::Jailed => {
            static WARNED: std::sync::Once = std::sync::Once::new();
            WARNED.call_once(|| {
                tracing::warn!(
                    "DUCKDB_JAILED_RUNNER is set but this worker does not run jobs under \
                     nsjail; DuckDB jobs keep running in-process. Unshare isolation only \
                     separates PIDs, so it would not confine the engine's file access."
                )
            });
            None
        }
    }
}

/// Whether the SQL-level fence still applies to a job running in `mode`.
pub(crate) fn keeps_fence(mode: Option<RunnerMode>) -> bool {
    mode != Some(RunnerMode::Jailed) || *FENCE_IN_JAIL
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RequestMode {
    Run,
    Prepare,
    Install,
}

#[derive(Serialize, Clone, Copy)]
pub(crate) struct RunnerRequest<'a> {
    pub mode: RequestMode,
    pub blocks: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_args: Option<&'a serde_json::value::RawValue>,
    pub token: &'a str,
    pub base_internal_url: &'a str,
    pub w_id: &'a str,
    pub memory_limit: &'a str,
    pub temp_directory: &'a str,
    pub max_result_size: usize,
    pub collect_last_only: bool,
    pub collect_first_row_only: bool,
}

/// What the in-process FFI call would have returned: the result string (`ERROR <json>` on
/// failure) and the column order.
pub(crate) struct RunnerOutput {
    pub result: String,
    pub column_order: Option<String>,
}

fn runner_path() -> Result<String> {
    if let Ok(p) = std::env::var("DUCKDB_RUNNER_PATH") {
        return Ok(p);
    }
    let exe = std::env::current_exe()
        .map_err(|e| Error::InternalErr(format!("Could not locate the worker binary: {e}")))?;
    let path = exe.with_file_name("windmill_duckdb_runner");
    if !path.exists() {
        return Err(Error::InternalErr(format!(
            "DUCKDB_JAILED_RUNNER is set but {} does not exist. Build it with \
             windmill-duckdb-ffi-internal/build_dev.sh or set DUCKDB_RUNNER_PATH.",
            path.display()
        )));
    }
    Ok(path.to_string_lossy().into_owned())
}

fn extension_dir() -> String {
    format!("{}/.duckdb/extensions", HOME_ENV.as_str())
}

/// Extensions a job may load inside the jail, where the cache is read-only: the two
/// `setup_duckdb_connection` installs on every connection, the fence's preloads, and every
/// extension a block installs by name. A `FORCE INSTALL` stays forced: it is the one
/// statement here that must run every time.
fn needed_extensions(blocks: &[String]) -> Vec<String> {
    let mut out: Vec<String> = ["httpfs", "azure", "parquet", "json"]
        .iter()
        .map(|s| format!("INSTALL {s};"))
        .collect();
    for block in blocks {
        let stmt = block.trim().trim_end_matches(';').trim();
        let mut toks = stmt.split_whitespace().peekable();
        let force = toks.peek().is_some_and(|t| t.eq_ignore_ascii_case("FORCE"));
        if force {
            toks.next();
        }
        if !toks
            .next()
            .is_some_and(|t| t.eq_ignore_ascii_case("INSTALL"))
        {
            continue;
        }
        let Some(name) = toks.next() else { continue };
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let name = name.to_ascii_lowercase();
        let force = if force { "FORCE " } else { "" };
        let stmt = match (toks.next(), toks.next(), toks.next()) {
            (None, _, _) => format!("{force}INSTALL {name};"),
            (Some(f), Some(r), None)
                if f.eq_ignore_ascii_case("FROM") && r.eq_ignore_ascii_case("community") =>
            {
                format!("{force}INSTALL {name} FROM community;")
            }
            _ => continue,
        };
        if !out.contains(&stmt) {
            out.push(stmt);
        }
    }
    out
}

static INSTALLED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

/// Installs, outside the jail, what the job will load inside it. The runner only runs
/// `INSTALL` here, and only of statements built above, so no script SQL and no extension
/// code runs unjailed. Once per extension per worker process.
async fn install_missing_extensions(blocks: &[String], job_dir: &str) -> Result<()> {
    let missing: Vec<String> = {
        let installed = INSTALLED.lock().unwrap();
        needed_extensions(blocks)
            .into_iter()
            .filter(|s| s.starts_with("FORCE ") || !installed.contains(s))
            .collect()
    };
    if missing.is_empty() {
        return Ok(());
    }
    let out = spawn_runner(
        RunnerMode::Process,
        &RunnerRequest {
            mode: RequestMode::Install,
            blocks: &missing,
            job_args: None,
            token: "",
            base_internal_url: "",
            w_id: "",
            memory_limit: "",
            temp_directory: "",
            max_result_size: 0,
            collect_last_only: false,
            collect_first_row_only: false,
        },
        job_dir,
        true,
    )
    .await?;
    if let Some(raw) = out.result.strip_prefix("ERROR ") {
        let msg = serde_json::from_str::<String>(raw).unwrap_or_else(|_| raw.to_string());
        return Err(Error::ExecutionErr(format!(
            "Installing DuckDB extensions for the jailed runner failed: {msg}"
        )));
    }
    INSTALLED.lock().unwrap().extend(missing.into_iter().filter(|s| !s.starts_with("FORCE ")));
    Ok(())
}

/// `FORCE INSTALL` rewrites the cache, which is read-only inside the jail. The worker has
/// already honored it outside, so inside it becomes a plain `INSTALL`, which finds the
/// extension present and does nothing. Same block count, so result collection is unchanged.
fn unforce_installs(blocks: &[String]) -> Vec<String> {
    blocks
        .iter()
        .map(|b| {
            let t = b.trim_start();
            match t.split_once(char::is_whitespace) {
                Some((first, rest))
                    if first.eq_ignore_ascii_case("FORCE")
                        && rest
                            .trim_start()
                            .get(..7)
                            .is_some_and(|k| k.eq_ignore_ascii_case("INSTALL")) =>
                {
                    rest.trim_start().to_string()
                }
                _ => b.clone(),
            }
        })
        .collect()
}

pub(crate) async fn run(
    mode: RunnerMode,
    req: &RunnerRequest<'_>,
    job_dir: &str,
) -> Result<RunnerOutput> {
    if mode != RunnerMode::Jailed {
        return spawn_runner(mode, req, job_dir, keeps_fence(Some(mode))).await;
    }
    install_missing_extensions(req.blocks, job_dir).await?;
    let blocks = unforce_installs(req.blocks);
    spawn_runner(mode, &RunnerRequest { blocks: &blocks, ..*req }, job_dir, keeps_fence(Some(mode))).await
}

async fn spawn_runner(
    mode: RunnerMode,
    req: &RunnerRequest<'_>,
    job_dir: &str,
    lock_temp_directory: bool,
) -> Result<RunnerOutput> {
    let runner = runner_path()?;
    let call_id = uuid::Uuid::new_v4();
    let result_path = format!("{job_dir}/duckdb_runner_{call_id}.out");

    let mut cmd = match mode {
        RunnerMode::Jailed => {
            let ext_dir = extension_dir();
            tokio::fs::create_dir_all(&ext_dir).await?;
            let bigquery_mount = match std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
                Ok(p) if !p.is_empty() => format!(
                    "mount {{\n    src: \"{p}\"\n    dst: \"{p}\"\n    is_bind: true\n    mandatory: false\n}}"
                ),
                _ => String::new(),
            };
            let config_name = format!("run.duckdb.{call_id}.config.proto");
            tokio::fs::write(
                format!("{job_dir}/{config_name}"),
                NSJAIL_CONFIG_RUN_DUCKDB_CONTENT
                    .replace("{JOB_DIR}", job_dir)
                    .replace("{RUNNER_PATH}", &runner)
                    .replace("{EXTENSION_DIR}", &ext_dir)
                    .replace("{BIGQUERY_CREDENTIALS_MOUNT}", &bigquery_mount)
                    .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                    .replace("#{DEV}", DEV_CONF_NSJAIL)
                    .replace(
                        "{TMP_MOUNT_BLOCK}",
                        &resolve_nsjail_tmp_mount_block(job_dir).await,
                    ),
            )
            .await?;
            let mut cmd = Command::new(NSJAIL_PATH.as_str());
            cmd.args([
                "--config",
                &config_name,
                "--",
                "/tmp/windmill_duckdb_runner",
            ]);
            cmd
        }
        RunnerMode::Process => Command::new(&runner),
    };

    cmd.current_dir(job_dir)
        .env_clear()
        .env("PATH", PATH_ENV.as_str())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Cancellation and timeouts drop the job future. nsjail's child dies with nsjail, so
        // killing the process we spawned is enough in both modes.
        .kill_on_drop(true);
    // The jail sets its own HOME, over the read-only extension cache.
    if mode == RunnerMode::Process {
        cmd.env("HOME", HOME_ENV.as_str());
    }
    for var in [
        "DUCKDB_MAX_TEMP_DIRECTORY_SIZE",
        "GOOGLE_APPLICATION_CREDENTIALS",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "no_proxy",
        "all_proxy",
    ] {
        if let Ok(v) = std::env::var(var) {
            cmd.env(var, v);
        }
    }

    let mut payload = serde_json::to_value(req).map_err(|e| Error::InternalErr(e.to_string()))?;
    payload["result_path"] = serde_json::Value::String(result_path.clone());
    payload["lock_temp_directory"] = serde_json::Value::Bool(lock_temp_directory);
    let payload = serde_json::to_vec(&payload).map_err(|e| Error::InternalErr(e.to_string()))?;

    // The engine's memory lives outside the worker, so a native OOM can take the job alone, as
    // long as the kernel picks the runner first. Set before exec so nsjail, and the runner it
    // clones, inherit it: written after spawn it races the clone, and the jail's /proc is
    // read-only. Only raw syscalls here, between fork and exec.
    #[cfg(target_os = "linux")]
    {
        let adj = windmill_common::worker::JOB_OOM_SCORE_ADJ.to_string();
        unsafe {
            cmd.pre_exec(move || {
                let fd = nix::libc::open(c"/proc/self/oom_score_adj".as_ptr(), nix::libc::O_WRONLY);
                if fd >= 0 {
                    nix::libc::write(fd, adj.as_ptr().cast(), adj.len());
                    nix::libc::close(fd);
                }
                Ok(())
            });
        }
    }
    let mut child = cmd.spawn().map_err(|e| {
        Error::InternalErr(format!("Could not start the DuckDB runner ({mode:?}): {e}"))
    })?;
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let write = async move {
        stdin.write_all(&payload).await?;
        stdin.shutdown().await
    };
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let (write, status, stdout, stderr) =
        tokio::join!(write, child.wait(), output_tail(stdout), output_tail(stderr));
    let status = status?;
    let read = tokio::fs::read_to_string(&result_path).await;
    let _ = tokio::fs::remove_file(&result_path).await;

    if !status.success() || read.is_err() {
        #[cfg(unix)]
        let signal = std::os::unix::process::ExitStatusExt::signal(&status);
        #[cfg(not(unix))]
        let signal: Option<i32> = None;
        let how = match signal {
            Some(sig) => format!("was killed by signal {sig}"),
            None => format!("exited with {status}"),
        };
        return Err(Error::ExecutionErr(format!(
            "The DuckDB engine process {how}. The worker is unaffected.{}{}{}",
            write
                .err()
                .map(|e| format!("\nrequest: {e}"))
                .unwrap_or_default(),
            if stderr.trim().is_empty() {
                String::new()
            } else {
                format!("\nstderr: {}", stderr.trim())
            },
            if stdout.trim().is_empty() {
                String::new()
            } else {
                format!("\nstdout: {}", stdout.trim())
            },
        )));
    }
    let mut column_order = read?;
    let header_end = column_order.find('\n').unwrap_or(column_order.len());
    let result = column_order.split_off((header_end + 1).min(column_order.len()));
    column_order.truncate(header_end);
    Ok(RunnerOutput { column_order: (!column_order.is_empty()).then_some(column_order), result })
}

/// Drains a runner pipe keeping only its last few KB. A script controls how much the engine
/// prints (`CALL enable_logging(storage='stdout')`), so buffering it whole would let a jailed
/// job grow the worker's memory without bound.
async fn output_tail(mut r: impl AsyncRead + Unpin) -> String {
    const KEEP: usize = 4096;
    let mut buf = vec![0u8; 16 * 1024];
    let mut tail: Vec<u8> = Vec::with_capacity(2 * KEEP);
    loop {
        match r.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                tail.extend_from_slice(&buf[..n]);
                if tail.len() > KEEP {
                    tail.drain(..tail.len() - KEEP);
                }
            }
        }
    }
    String::from_utf8_lossy(&tail).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn force_installs_run_outside_and_become_plain_inside() {
        let blocks = strs(&[
            "force  install httpfs;",
            "INSTALL bigquery FROM community;",
            "INSTALL x FROM 'http://evil';",
            "INSTALL '/tmp/x.so';",
            "SELECT 'FORCE INSTALL';",
        ]);
        assert_eq!(
            needed_extensions(&blocks)[4..],
            strs(&["FORCE INSTALL httpfs;", "INSTALL bigquery FROM community;"])
        );
        assert_eq!(
            unforce_installs(&blocks),
            strs(&[
                "install httpfs;",
                "INSTALL bigquery FROM community;",
                "INSTALL x FROM 'http://evil';",
                "INSTALL '/tmp/x.so';",
                "SELECT 'FORCE INSTALL';",
            ])
        );
    }
}
