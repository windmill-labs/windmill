//! DuckDB jobs run in `windmill_duckdb_runner`, a process of their own, under the same job
//! isolation as every other language: inside nsjail when the worker jails jobs, under
//! `unshare` when it isolates that way, and as a plain child process otherwise. An engine
//! crash or a cancellation then ends the runner, never the worker.
//!
//! The runner hands back the string the engine call produced, so result collection and error
//! decoding stay in `duckdb_executor`.

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
    common::{build_command_with_isolation, resolve_nsjail_tmp_mount_block, DEV_CONF_NSJAIL},
    is_sandboxing_enabled, is_unshare_enabled, DISABLE_NUSER, HOME_ENV, NSJAIL_AVAILABLE,
    NSJAIL_PATH, PATH_ENV,
};

const NSJAIL_CONFIG_RUN_DUCKDB_CONTENT: &str = include_str!("../nsjail/run.duckdb.config.proto");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RunnerMode {
    /// Inside nsjail: the jail confines the file system.
    Jailed,
    /// Under `unshare`, as other languages are on such a worker: its own PID namespace and a
    /// cleared environment, with the worker user's view of the file system.
    Unshare,
    /// A plain child process, on a worker that does not isolate jobs.
    Plain,
}

/// Decided once per job, so every connection a job opens runs the same way.
pub(crate) fn runner_mode() -> RunnerMode {
    if cfg!(unix) && is_sandboxing_enabled() && NSJAIL_AVAILABLE.is_some() {
        RunnerMode::Jailed
    } else if is_unshare_enabled() {
        // `build_command_with_isolation` logs and runs unisolated when `unshare` is missing,
        // as it does for every other language.
        RunnerMode::Unshare
    } else {
        RunnerMode::Plain
    }
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

/// The runner's answer: the result string (`ERROR <json>` on failure) and the column order.
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
    let name = format!("windmill_duckdb_runner{}", std::env::consts::EXE_SUFFIX);
    let path = exe.with_file_name(&name);
    // Cargo runs test binaries from `target/<profile>/deps`, one level below where the
    // worker binary and `build_dev.sh`'s runner sit.
    let in_target_dir = exe
        .parent()
        .filter(|d| d.ends_with("deps"))
        .and_then(|d| d.parent())
        .map(|d| d.join(&name));
    let path = match in_target_dir {
        Some(p) if !path.exists() && p.exists() => p,
        _ => path,
    };
    if !path.exists() {
        return Err(Error::InternalErr(format!(
            "DuckDB jobs need {}, alongside the worker binary. Build it with \
             windmill-duckdb-ffi-internal/build_dev.sh, or point DUCKDB_RUNNER_PATH at it.",
            path.display()
        )));
    }
    Ok(path.to_string_lossy().into_owned())
}

/// The jail's own extension cache, separate from the worker user's `~/.duckdb` and mounted
/// read-only. Only [`install_missing_extensions`] writes to it, so a job can neither add to
/// nor replace what the next job loads from it.
fn extension_dir() -> String {
    format!("{}/.duckdb/windmill_jail_extensions", HOME_ENV.as_str())
}

/// Extensions a job may load inside the jail, where the cache is read-only: the two
/// `setup_duckdb_connection` installs on every connection, the two the platform reaches for
/// without naming them (Parquet data files, the JSON type), and every extension a block
/// installs by name. A `FORCE INSTALL` stays forced: it is the one
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
    let ext_dir = extension_dir();
    tokio::fs::create_dir_all(&ext_dir).await?;
    let blocks: Vec<String> = std::iter::once(format!(
        "SET extension_directory='{}';",
        ext_dir.replace('\'', "''")
    ))
    .chain(missing.iter().cloned())
    .collect();
    let out = spawn_runner(
        RunnerMode::Plain,
        &RunnerRequest {
            mode: RequestMode::Install,
            blocks: &blocks,
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
        return spawn_runner(mode, req, job_dir).await;
    }
    install_missing_extensions(req.blocks, job_dir).await?;
    let blocks = unforce_installs(req.blocks);
    spawn_runner(mode, &RunnerRequest { blocks: &blocks, ..*req }, job_dir).await
}

/// Kills the runner's process group when the job future is dropped (cancellation, timeout)
/// before the runner exits. Killing the spawned process alone is not enough: `unshare --fork`
/// leaves its child running.
struct KillGroupOnDrop(Option<i32>);

impl Drop for KillGroupOnDrop {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(pgid) = self.0 {
            let _ = nix::sys::signal::killpg(
                nix::unistd::Pid::from_raw(pgid),
                nix::sys::signal::Signal::SIGKILL,
            );
        }
    }
}

async fn spawn_runner(
    mode: RunnerMode,
    req: &RunnerRequest<'_>,
    job_dir: &str,
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
        RunnerMode::Unshare => build_command_with_isolation(&runner, &[]),
        RunnerMode::Plain => Command::new(&runner),
    };

    cmd.current_dir(job_dir)
        .env_clear()
        .env("PATH", PATH_ENV.as_str())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    cmd.process_group(0);
    // The jail sets its own HOME, over the read-only extension cache.
    if mode != RunnerMode::Jailed {
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
    let payload = serde_json::to_vec(&payload).map_err(|e| Error::InternalErr(e.to_string()))?;

    // The engine's memory lives outside the worker, so a native OOM can take the job alone, as
    // long as the kernel picks the runner first. Set before exec so nsjail or unshare, and the
    // runner they start, inherit it: written after spawn it races that, and the jail's /proc is
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
    let mut guard = KillGroupOnDrop(child.id().map(|p| p as i32));
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
    guard.0 = None;
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
