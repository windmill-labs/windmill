use crate::{common::MaybeLock, get_proxy_envs_for_lang};
use std::{collections::HashMap, fs::DirBuilder, process::Stdio};
use windmill_common::scripts::ScriptLang;

use crate::global_cache::save_cache;
use itertools::Itertools;
use serde_json::value::RawValue;
use tokio::{
    fs::{self, File},
    io::AsyncReadExt,
    process::Command,
};
use uuid::Uuid;
use windmill_common::{
    error::{self, Error},
    utils::calculate_hash,
    worker::{write_file, Connection, GoAnnotations},
};
use windmill_parser_go::{parse_go_imports, REQUIRE_PARSE};
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::{
    common::{
        build_command_with_isolation, capitalize, create_args_and_out_file, get_reserved_variables,
        read_result, resolve_nsjail_timeout, resolve_nsjail_tmp_mount_block, start_child_process,
        OccupancyMetrics, DEV_CONF_NSJAIL,
    },
    handle_child::handle_child,
    is_sandboxing_enabled, read_ee_registry, GoBuildLimits, DISABLE_NUSER, GOPRIVATE, GOPROXY,
    GO_BIN_CACHE_DIR, GO_BUILD_LIMITS, GO_CACHE_DIR, HOME_ENV, NSJAIL_PATH, PATH_ENV, PROXY_ENVS,
    TRACING_PROXY_CA_CERT_PATH, TZ_ENV,
};
use windmill_common::client::AuthedClient;

#[cfg(windows)]
use crate::SYSTEM_ROOT;

#[cfg(windows)]
fn get_windows_tmp_dir() -> String {
    std::env::var("TMP")
        .or_else(|_| std::env::var("TEMP"))
        .unwrap_or_else(|_| {
            let system_drive = std::env::var("SYSTEMDRIVE").unwrap_or_else(|_| "C:".to_string());
            format!("{}\\tmp", system_drive)
        })
}

#[cfg(windows)]
fn get_windows_program_files() -> String {
    std::env::var("ProgramFiles").unwrap_or_else(|_| {
        let system_drive = std::env::var("SYSTEMDRIVE").unwrap_or_else(|_| "C:".to_string());
        format!("{}\\Program Files", system_drive)
    })
}

#[cfg(windows)]
fn windows_gopath() -> String {
    GO_CACHE_DIR.replace('/', "\\")
}

#[cfg(windows)]
fn set_windows_env_vars(cmd: &mut Command) {
    cmd.env("SystemRoot", SYSTEM_ROOT.as_str())
        .env("TMP", get_windows_tmp_dir())
        .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
        .env(
            "APPDATA",
            std::env::var("APPDATA")
                .unwrap_or_else(|_| format!("{}\\AppData\\Roaming", HOME_ENV.as_str())),
        )
        .env("ProgramFiles", get_windows_program_files())
        .env(
            "LOCALAPPDATA",
            std::env::var("LOCALAPPDATA")
                .unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str())),
        );
}

const GO_REQ_SPLITTER: &str = "//go.sum\n";
const NSJAIL_CONFIG_RUN_GO_CONTENT: &str = include_str!("../nsjail/run.go.config.proto");

lazy_static::lazy_static! {
    static ref GO_PATH: String = std::env::var("GO_PATH").unwrap_or_else(|_| "/usr/bin/go".to_string());
}

pub const GO_OBJECT_STORE_PREFIX: &str =
    const_format::concatcp!(crate::global_cache::TARGET, "_gobin/");

/// Worker group env vars forwarded to the Go toolchain (`go build`, `go mod …`).
///
/// Restricted to the GC and scheduler knobs, which bound what a compilation costs
/// without changing what it produces: the built binary is cached under a hash of
/// the source and lockfile alone, so a var that alters codegen (`GOFLAGS`, build
/// tags) would let two worker groups disagree about the contents of one cache
/// entry — including the one shared through the object store.
const GO_TOOLCHAIN_TUNING_ENVS: [&str; 3] = ["GOMEMLIMIT", "GOGC", "GOMAXPROCS"];

/// The two shapes a Go toolchain invocation takes, which spend the budget
/// differently: a build fans out into a driver plus compilers, while the module
/// steps are one process with nothing else running.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GoToolchainStep {
    Build,
    Mod,
}

impl GoToolchainStep {
    fn label(&self) -> &'static str {
        match self {
            GoToolchainStep::Build => "Go compilation",
            GoToolchainStep::Mod => "Go dependency resolution",
        }
    }
}

/// Memory and parallelism settings for the Go toolchain subprocesses, which
/// otherwise inherit nothing (they are spawned with a cleared environment).
///
/// Must be applied before the explicit `.env` calls of each command so the
/// worker's own `PATH`/`GOPATH`/`GOCACHE`/`HOME` win over a worker group setting
/// the same names, as they do on the run step.
fn go_toolchain_envs(step: GoToolchainStep) -> Vec<(String, String)> {
    let worker_config = windmill_common::worker::WORKER_CONFIG.load();
    let envs = merge_go_toolchain_envs(&worker_config.env_vars, *GO_BUILD_LIMITS, step);
    log_go_toolchain_limits(step, &envs);
    envs
}

/// A slow compilation is the symptom of a limit set too low, and nothing else would
/// tell an operator that one is in force or what it resolved to. Reports what the
/// toolchain is actually given, since a worker group can pin values of its own, and
/// once per distinct setting rather than per job, since it can move them at runtime.
fn log_go_toolchain_limits(step: GoToolchainStep, envs: &[(String, String)]) {
    lazy_static::lazy_static! {
        static ref LOGGED: std::sync::Mutex<std::collections::HashSet<String>> =
            Default::default();
    }

    let limits = envs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(" ");
    // Neutral wording: the settings are reported rather than characterized, since
    // a worker group can pin ones that lift the limit (`GOMEMLIMIT=off`) as easily
    // as ones that impose it.
    let line = if limits.is_empty() {
        format!("{} runs with no limits", step.label())
    } else {
        format!("{} runs with {limits}", step.label())
    };

    let Ok(mut logged) = LOGGED.lock() else {
        return;
    };
    if logged.insert(line.clone()) {
        tracing::info!("{line}");
    }
}

fn merge_go_toolchain_envs(
    worker_envs: &HashMap<String, String>,
    derived: Option<GoBuildLimits>,
    step: GoToolchainStep,
) -> Vec<(String, String)> {
    // Values reach the toolchain as the worker group wrote them, the same way they
    // reach the run step: a `GOMEMLIMIT` the Go runtime rejects then fails both
    // steps alike instead of compiling under a rewritten value and dying on the
    // binary it produced. An allowlisted name the worker never set resolves to an
    // empty string, which would shadow the derived pair with nothing.
    let configured = |k: &str| worker_envs.get(k).filter(|v| !v.trim().is_empty());

    let mut envs: Vec<(String, String)> = GO_TOOLCHAIN_TUNING_ENVS
        .iter()
        .filter_map(|k| configured(k).map(|v| (k.to_string(), v.clone())))
        .collect();

    // Half of the derived pair is not a weaker limit but no limit (see
    // `resolve_go_build_limits`), so a worker group that sets either one owns both.
    let pinned_by_worker_group = envs
        .iter()
        .any(|(k, _)| k == "GOMEMLIMIT" || k == "GOMAXPROCS");
    if let (false, Some(limits)) = (pinned_by_worker_group, derived) {
        match step {
            GoToolchainStep::Build => {
                envs.push(("GOMEMLIMIT".to_string(), limits.memlimit.to_string()));
                envs.push(("GOMAXPROCS".to_string(), limits.parallelism.to_string()));
            }
            // Nothing shares the budget with a single process, and its work queue
            // is sized from `GOMAXPROCS` — throttling it would only slow fetches
            // down without bounding anything.
            GoToolchainStep::Mod => {
                envs.push(("GOMEMLIMIT".to_string(), limits.budget.to_string()));
            }
        }
    }
    envs
}

/// `go build`'s `-p`, so the parallelism the aggregate assumes is the one it gets.
///
/// `GOMAXPROCS` only supplies the *default* for `-p`, which a `GOFLAGS=-p=…`
/// persisted in the toolchain's own env file outranks — and that file is read,
/// since the command keeps `HOME`. A flag on the command line is applied last.
fn go_build_parallelism_args(envs: &[(String, String)]) -> Vec<String> {
    envs.iter()
        .find(|(k, _)| k == "GOMAXPROCS")
        .and_then(|(_, v)| go_runtime_int32(v))
        .filter(|parallelism| *parallelism > 0)
        .map(|parallelism| vec!["-p".to_string(), parallelism.to_string()])
        .unwrap_or_default()
}

/// `GOMAXPROCS` as the Go runtime reads it: a signed 32-bit decimal, no padding of
/// any kind, and `None` for everything else — which the runtime silently ignores.
///
/// A worker group's value is forwarded verbatim, so it is not necessarily one the
/// `-p` flag would take, and the two parsers disagree in both directions: `-p`
/// infers the base and rejects the `08` the runtime reads as 8, while it accepts
/// the `2147483648` and the `"8 "` the runtime discards — and it would then start
/// that many build workers. Reading the value the runtime's way keeps `-p` in step
/// with it, so a spelling the runtime ignores leaves the flag off and the build
/// keeps the default the runtime itself would have used.
fn go_runtime_int32(v: &str) -> Option<i32> {
    let digits = v.strip_prefix(['+', '-']).unwrap_or(v);
    (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| v.parse::<i32>().ok())
        .flatten()
}

/// Cache key of a Go build. The run path and the deploy-time prebuild must derive it the
/// same way or the prebuilt binary is never found and gets rebuilt on first run.
fn go_cache_key(
    code: &str,
    maybe_lock: &MaybeLock,
    modules: Option<&HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> String {
    // A module whose path starts with `go/` lands inside the module dir this builds, so
    // its content ends up in the binary the key names.
    let base = format!("{}{:?}v2", code, maybe_lock);
    crate::worker::artifact_cache_name(base, modules)
}

/// Install the deps, generate the entrypoint wrapper, `go build`, and push the binary to
/// the shared cache. `job_dir` must already be the `go` subdirectory the module lives in.
async fn build_go_binary(
    job: &MiniPulledJob,
    inner_content: &str,
    maybe_lock: MaybeLock,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    conn: &Connection,
    worker_name: &str,
    base_internal_url: &str,
    occupation_metrics: &mut OccupancyMetrics,
    hash: &str,
    skip_go_mod: bool,
    skip_tidy: bool,
) -> Result<String, Error> {
    let bin_path = format!("{}/{hash}", *GO_BIN_CACHE_DIR);
    install_go_dependencies(
        &job.id,
        inner_content,
        maybe_lock,
        mem_peak,
        canceled_by,
        job_dir,
        conn,
        true,
        skip_go_mod,
        skip_tidy,
        worker_name,
        &job.workspace_id,
        occupation_metrics,
    )
    .await?;

    {
        let sig = windmill_parser_go::parse_go_sig(&inner_content)?;

        const WRAPPER_CONTENT: &str = r#"package main

import (
"encoding/json"
"os"
"fmt"
"mymod/inner"
)

func main() {{

dat, err := os.ReadFile("args.json")
if err != nil {{
    fmt.Println(err)
    os.Exit(1)
}}

var req inner.Req

if err := json.Unmarshal(dat, &req); err != nil {{
    fmt.Println(err)
    os.Exit(1)
}}

res, err := inner.Run(req)
if err != nil {{
    fmt.Println(err)
    os.Exit(1)
}}
res_json, err := json.Marshal(res)
if err != nil {{
    fmt.Println(err)
    os.Exit(1)
}}
f, err := os.OpenFile("result.json", os.O_APPEND|os.O_WRONLY, os.ModeAppend)
if err != nil {{
    fmt.Println(err)
    os.Exit(1)
}}
_, err = f.WriteString(string(res_json))
if err != nil {{
    fmt.Println(err)
    os.Exit(1)
}}
}}"#;

        write_file(job_dir, "main.go", WRAPPER_CONTENT)?;

        {
            let spread = &sig
                .args
                .clone()
                .into_iter()
                .map(|x| format!("req.{}", capitalize(&x.name)))
                .join(", ");
            let req_body = &sig
                .args
                .into_iter()
                .map(|x| {
                    format!(
                        "{} {} `json:\"{}\"`",
                        capitalize(&x.name),
                        windmill_parser_go::otyp_to_string(x.otyp),
                        x.name
                    )
                })
                .join("\n");
            let runner_content: String = format!(
                r#"package inner
type Req struct {{
{req_body}
}}

func Run(req Req) (interface{{}}, error){{
return main({spread})
}}

"#,
            );
            write_file(&format!("{job_dir}/inner"), "runner.go", &runner_content)?;
        }
    }

    let toolchain_envs = go_toolchain_envs(GoToolchainStep::Build);
    let build_args = [
        vec!["build".to_string()],
        go_build_parallelism_args(&toolchain_envs),
        vec!["main.go".to_string()],
    ]
    .concat();

    let mut build_go_cmd = Command::new(GO_PATH.as_str());
    build_go_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(toolchain_envs)
        .env("PATH", PATH_ENV.as_str())
        .env("BASE_INTERNAL_URL", base_internal_url)
        .env("GOPATH", {
            #[cfg(unix)]
            {
                GO_CACHE_DIR.as_str()
            }
            #[cfg(windows)]
            {
                &windows_gopath()
            }
        })
        .env("HOME", HOME_ENV.as_str())
        .env("GOCACHE", GO_CACHE_DIR.as_str())
        .envs(PROXY_ENVS.clone())
        .args(build_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    set_windows_env_vars(&mut build_go_cmd);

    let build_go_process = start_child_process(build_go_cmd, GO_PATH.as_str(), false).await?;
    handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        build_go_process,
        false,
        worker_name,
        &job.workspace_id,
        "go build",
        None,
        false,
        &mut Some(occupation_metrics),
        None,
        None,
    )
    .await?;

    #[cfg(unix)]
    let executable_path = format!("{job_dir}/main");
    #[cfg(windows)]
    let executable_path = format!("{job_dir}/main.exe");

    // Set executable permissions on Windows
    #[cfg(windows)]
    {
        use std::fs;
        if let Ok(metadata) = fs::metadata(&executable_path) {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(false);
            // On Windows, we need to ensure the file is not marked as read-only
            // and has appropriate permissions for execution
            let _ = fs::set_permissions(&executable_path, permissions);
        }
    }

    Ok(
        match save_cache(
            &bin_path,
            &format!("{GO_OBJECT_STORE_PREFIX}{hash}"),
            &executable_path,
            false,
        )
        .await
        {
            Err(e) => {
                let em = format!("could not save {bin_path} to go cache: {e:?}");
                tracing::error!(em);
                em
            }
            Ok(logs) => logs,
        },
    )
}

/// Build a deployed Go script ahead of its first run and push the binary to the shared
/// cache.
pub async fn prebuild_go_binary(
    job: &MiniPulledJob,
    code: &str,
    lock: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    conn: &Connection,
    worker_name: &str,
    base_internal_url: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    modules: Option<&HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> Result<Option<String>, Error> {
    let maybe_lock = MaybeLock::Resolved { lock: lock.to_string() };
    let hash = go_cache_key(code, &maybe_lock, modules);
    let remote_path = format!("{GO_OBJECT_STORE_PREFIX}{hash}");
    if crate::global_cache::exists_in_object_store(&remote_path).await {
        return Ok(None);
    }

    // go refuses to build a module at the job dir root, same as `handle_go_job`.
    let job_dir = &format!("{job_dir}/go");
    DirBuilder::new()
        .recursive(true)
        .create(&job_dir)
        .map_err(|e| Error::internal_err(format!("could not create go job dir: {e:?}")))?;

    let (skip_go_mod, skip_tidy) = gen_go_mod(code, job_dir, lock).await?;

    let logs = build_go_binary(
        job,
        code,
        maybe_lock,
        mem_peak,
        canceled_by,
        job_dir,
        conn,
        worker_name,
        base_internal_url,
        occupancy_metrics,
        &hash,
        skip_go_mod,
        skip_tidy,
    )
    .await?;
    crate::global_cache::ensure_pushed_to_object_store(&remote_path).await?;
    Ok(Some(logs))
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_go_job(
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &MiniPulledJob,
    conn: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    inner_content: &str,
    job_dir: &str,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    occupation_metrics: &mut OccupancyMetrics,
    maybe_lock: MaybeLock,
    modules: Option<&HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> Result<Box<RawValue>, Error> {
    //go does not like executing modules at temp root
    let job_dir = &format!("{job_dir}/go");
    DirBuilder::new()
        .recursive(true)
        .create(&job_dir)
        .expect("could not create go job dir");

    let hash = go_cache_key(inner_content, &maybe_lock, modules);
    let bin_path = format!("{}/{hash}", *GO_BIN_CACHE_DIR);
    let remote_path = format!("{GO_OBJECT_STORE_PREFIX}{hash}");
    let (cache, cache_logs) = crate::global_cache::load_cache(&bin_path, &remote_path, false).await;

    let (skip_go_mod, skip_tidy) = if cache {
        (true, true)
    } else if let Some(lock) = maybe_lock.get_lock() {
        gen_go_mod(inner_content, job_dir, &lock).await?
    } else {
        (false, false)
    };

    let cache_logs = if !cache {
        let logs1 = format!("{cache_logs}\n\n--- GO DEPENDENCIES SETUP ---\n");
        append_logs(&job.id, &job.workspace_id, logs1, conn).await;

        let build_logs = build_go_binary(
            job,
            inner_content,
            maybe_lock,
            mem_peak,
            canceled_by,
            job_dir,
            conn,
            worker_name,
            base_internal_url,
            occupation_metrics,
            &hash,
            skip_go_mod,
            skip_tidy,
        )
        .await?;

        create_args_and_out_file(client, job, job_dir, conn).await?;
        build_logs
    } else {
        #[cfg(unix)]
        let target = format!("{job_dir}/main");
        #[cfg(windows)]
        let target = format!("{job_dir}/main.exe");

        #[cfg(unix)]
        let symlink = std::os::unix::fs::symlink(&bin_path, &target);
        #[cfg(windows)]
        let symlink = {
            // On Windows, copy the file instead of creating a symlink
            // because symlinks might not work correctly for executables
            use std::fs;
            fs::copy(&bin_path, &target).map(|_| ())
        };

        symlink.map_err(|e| {
            Error::ExecutionErr(format!(
                "could not copy cached binary from {bin_path} to {job_dir}/main: {e:?}"
            ))
        })?;

        create_args_and_out_file(client, job, job_dir, conn).await?;
        cache_logs
    };

    let logs2 = format!("{cache_logs}\n\n--- GO CODE EXECUTION ---\n");
    append_logs(&job.id, &job.workspace_id, logs2, conn).await;

    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

    let child = if is_sandboxing_enabled() {
        let nsjail_timeout =
            resolve_nsjail_timeout(conn, &job.workspace_id, job.id, job.timeout).await;
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_GO_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{TRACING_PROXY_CA_CERT_PATH}", &*TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL)
                .replace(
                    "{TMP_MOUNT_BLOCK}",
                    &resolve_nsjail_tmp_mount_block(job_dir).await,
                )
                .replace("{TIMEOUT}", &nsjail_timeout),
        )?;
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(
                get_proxy_envs_for_lang(
                    &ScriptLang::Go,
                    job.kind,
                    &job.id,
                    &job.workspace_id,
                    conn,
                )
                .await?,
            )
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["--config", "run.config.proto", "--", "/tmp/go/main"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        #[cfg(unix)]
        let compiled_executable_name = "./main";
        #[cfg(windows)]
        let compiled_executable_name = format!("{}/main.exe", job_dir);

        let mut run_go = build_command_with_isolation(&compiled_executable_name, &[]);

        run_go
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(
                get_proxy_envs_for_lang(
                    &ScriptLang::Go,
                    job.kind,
                    &job.id,
                    &job.workspace_id,
                    conn,
                )
                .await?,
            )
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("GOPATH", {
                #[cfg(unix)]
                {
                    GO_CACHE_DIR.as_str()
                }
                #[cfg(windows)]
                {
                    &windows_gopath()
                }
            })
            .env("HOME", HOME_ENV.as_str());

        if let Some(ref goprivate) = read_ee_registry(
            GOPRIVATE.clone(),
            "go private",
            &job.id,
            &job.workspace_id,
            conn,
        )
        .await
        {
            run_go.env("GOPRIVATE", goprivate);
        }
        if let Some(ref goproxy) = read_ee_registry(
            GOPROXY.clone(),
            "go proxy",
            &job.id,
            &job.workspace_id,
            conn,
        )
        .await
        {
            run_go.env("GOPROXY", goproxy);
        }

        #[cfg(windows)]
        set_windows_env_vars(&mut run_go);

        run_go
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(run_go, &compiled_executable_name, false).await?
    };
    let handle_result = handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        is_sandboxing_enabled(),
        worker_name,
        &job.workspace_id,
        "go run",
        job.timeout,
        false,
        &mut Some(occupation_metrics),
        None,
        None,
    )
    .await?;

    read_result(job_dir, handle_result.result_stream).await
}

async fn gen_go_mod(inner_content: &str, job_dir: &str, lock: &str) -> error::Result<(bool, bool)> {
    gen_go_mymod(inner_content, job_dir).await?;

    let md = lock.split_once(GO_REQ_SPLITTER);
    if let Some((req, sum)) = md {
        write_file(job_dir, "go.mod", &req)?;
        write_file(job_dir, "go.sum", &sum)?;
        Ok((true, true))
    } else {
        write_file(job_dir, "go.mod", &lock)?;
        Ok((true, false))
    }
}

use std::fs::OpenOptions;
use std::io::prelude::*;

pub async fn install_go_dependencies(
    job_id: &Uuid,
    code: &str,
    maybe_lock: MaybeLock,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    conn: &Connection,
    non_dep_job: bool,
    // NOTE: this is impossible for skip_go_mod be `false` and maybe_lock be `Resolved`.
    // TODO: make it comptime gurantee
    skip_go_mod: bool,
    has_sum: bool,
    worker_name: &str,
    w_id: &str,
    occupation_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    let anns = GoAnnotations::parse(code);

    let hash_input = match maybe_lock {
        MaybeLock::Resolved { ref lock } => lock.clone(),
        MaybeLock::Unresolved { ref workspace_dependencies } => {
            // NOTE: This will always be none, go workspace dependencies are disabled for now.
            // read more on discord (internal):
            // https://discord.com/channels/930051556043276338/1031563866641018910/1443541229349634189
            if let Some(go_mod) = workspace_dependencies.get_go()? {
                if !skip_go_mod {
                    gen_go_mymod(code, job_dir).await?;
                    fs::write(format!("{job_dir}/go.mod"), &go_mod).await?;
                }
                go_mod
            } else {
                if !skip_go_mod {
                    gen_go_mymod(code, job_dir).await?;
                    let mut child_cmd = Command::new(GO_PATH.as_str());
                    child_cmd
                        .current_dir(job_dir)
                        .env_clear()
                        .envs(go_toolchain_envs(GoToolchainStep::Mod))
                        .args(vec!["mod", "init", "mymod"])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped());

                    #[cfg(windows)]
                    child_cmd.env("GOPATH", windows_gopath());
                    #[cfg(unix)]
                    child_cmd.env("GOPATH", GO_CACHE_DIR.as_str());

                    #[cfg(windows)]
                    set_windows_env_vars(&mut child_cmd);
                    let child_process =
                        start_child_process(child_cmd, GO_PATH.as_str(), false).await?;

                    handle_child(
                        job_id,
                        conn,
                        mem_peak,
                        canceled_by,
                        child_process,
                        false,
                        worker_name,
                        w_id,
                        "go init",
                        None,
                        false,
                        &mut Some(occupation_metrics),
                        None,
                        None,
                    )
                    .await?;

                    for x in REQUIRE_PARSE.captures_iter(code) {
                        let mut file = OpenOptions::new()
                            .write(true)
                            .append(true)
                            .open(format!("{job_dir}/go.mod"))
                            .unwrap();

                        writeln!(file, "require {}\n", &x[1])?;
                    }
                }
                if !has_sum {
                    calculate_hash(parse_go_imports(&code)?.iter().join("\n").as_str())
                } else {
                    "".to_owned()
                }
            }
        }
    };

    let hash = format!(
        "go{}-{}",
        if anns.go1_22_compat { "1.22" } else { "" },
        calculate_hash(&hash_input)
    );

    let (mut new_lockfile, mut skip_tidy) = (false, has_sum);

    if !has_sum {
        if let Some(db) = conn.as_sql() {
            if let Some(cached) = sqlx::query_scalar!(
                "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
                hash
            )
            .fetch_optional(db)
            .await?
            {
                let logs1 = format!("\nfound cached resolution: {}", hash);
                append_logs(&job_id, w_id, logs1, conn).await;
                gen_go_mod(code, job_dir, &cached).await?;
                skip_tidy = true;
                new_lockfile = false;
            } else {
                new_lockfile = true;
            }
        }
    }

    let mod_command = if skip_tidy { "download" } else { "tidy" };
    let mut child_cmd = Command::new(GO_PATH.as_str());
    child_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(go_toolchain_envs(GoToolchainStep::Mod))
        .env("HOME", HOME_ENV.as_str())
        .env("PATH", PATH_ENV.as_str())
        .envs(PROXY_ENVS.clone())
        .env("GOPATH", {
            #[cfg(unix)]
            {
                GO_CACHE_DIR.as_str()
            }
            #[cfg(windows)]
            {
                &windows_gopath()
            }
        })
        .args(vec!["mod", mod_command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(ref goprivate) =
        read_ee_registry(GOPRIVATE.clone(), "go private", job_id, w_id, conn).await
    {
        child_cmd.env("GOPRIVATE", goprivate);
    }

    // TODO: Remove if no incidents reported
    if !std::env::var("WMDEBUG_NO_GOPROXY_ON_TIDY").ok().is_some() {
        if let Some(ref goproxy) =
            read_ee_registry(GOPROXY.clone(), "go proxy", job_id, w_id, conn).await
        {
            child_cmd.env("GOPROXY", goproxy);
        }
    }

    // If annotation used we want to call tidy with special flag to pin go to 1.22
    // The reason for this that at some point we had to jump from go 1.22 to 1.25 and this addds backward compatibility.
    if anns.go1_22_compat && mod_command == "tidy" {
        child_cmd.args(vec!["-go", "1.22"]);
    }

    #[cfg(windows)]
    set_windows_env_vars(&mut child_cmd);
    let child_process = start_child_process(child_cmd, GO_PATH.as_str(), false).await?;

    handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        child_process,
        false,
        worker_name,
        &w_id,
        &format!("go {mod_command}"),
        None,
        false,
        &mut Some(occupation_metrics),
        None,
        None,
    )
    .await?;

    if (!new_lockfile || has_sum) && non_dep_job {
        return Ok("".to_string());
    }

    let mut req_content = "".to_string();

    let mut file = File::open(format!("{job_dir}/go.mod")).await?;
    file.read_to_string(&mut req_content).await?;
    req_content.push_str(GO_REQ_SPLITTER);
    let sum_path = format!("{job_dir}/go.sum");
    if tokio::fs::metadata(&sum_path).await.is_ok() {
        let mut file = File::open(sum_path).await?;
        file.read_to_string(&mut req_content).await?;
    }

    if non_dep_job {
        if let Some(db) = conn.as_sql() {
            sqlx::query!(
                "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('5 mins')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = EXCLUDED.lockfile",
                hash,
                req_content
            )
            .fetch_optional(db)
            .await?;
        }

        return Ok(String::new());
    } else {
        Ok(req_content)
    }
}

async fn gen_go_mymod(code: &str, job_dir: &str) -> error::Result<()> {
    let code = if code.trim_start().starts_with("package") {
        code.to_string()
    } else {
        format!("package inner; {code}")
    };

    let mymod_dir = format!("{job_dir}/inner");
    DirBuilder::new()
        .recursive(true)
        .create(&mymod_dir)
        .expect("could not create go's mymod dir");

    write_file(&mymod_dir, "inner_main.go", &code)?;

    Ok(())
}

#[cfg(test)]
mod go_toolchain_envs_tests {
    use super::{
        go_build_parallelism_args, merge_go_toolchain_envs, GoBuildLimits, GoToolchainStep, HashMap,
    };

    const DERIVED: Option<GoBuildLimits> =
        Some(GoBuildLimits { budget: 2048, memlimit: 512, parallelism: 3 });

    fn worker_envs(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn merged(pairs: &[(&str, &str)], derived: Option<GoBuildLimits>) -> Vec<(String, String)> {
        let mut envs =
            merge_go_toolchain_envs(&worker_envs(pairs), derived, GoToolchainStep::Build);
        envs.sort();
        envs
    }

    fn expect(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        let mut envs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        envs.sort();
        envs
    }

    #[test]
    fn worker_group_settings_replace_the_derived_pair_whole() {
        assert_eq!(
            merged(&[], DERIVED),
            expect(&[("GOMEMLIMIT", "512"), ("GOMAXPROCS", "3")])
        );
        // Either half configured hands the whole policy over, since a cap without a
        // process count (or the reverse) bounds nothing.
        assert_eq!(
            merged(&[("GOMEMLIMIT", "2GiB")], DERIVED),
            expect(&[("GOMEMLIMIT", "2GiB")])
        );
        assert_eq!(
            merged(&[("GOMAXPROCS", "32")], DERIVED),
            expect(&[("GOMAXPROCS", "32")])
        );
        // GOGC is orthogonal to the pair, so it rides along with it.
        assert_eq!(
            merged(&[("GOGC", "50")], DERIVED),
            expect(&[("GOGC", "50"), ("GOMEMLIMIT", "512"), ("GOMAXPROCS", "3")])
        );
        // Forwarded untouched: a trimmed value would compile under a spelling the
        // run step then rejects.
        assert_eq!(
            merged(&[("GOMEMLIMIT", " 2GiB ")], DERIVED),
            expect(&[("GOMEMLIMIT", " 2GiB ")])
        );
        // An allowlisted name the worker never set must not shadow the pair.
        assert_eq!(
            merged(&[("GOMEMLIMIT", "  ")], DERIVED),
            expect(&[("GOMEMLIMIT", "512"), ("GOMAXPROCS", "3")])
        );
        assert_eq!(merged(&[], None), expect(&[]));
    }

    #[test]
    fn the_module_steps_get_the_whole_budget() {
        // One process, nothing sharing with it, and no compilers to hold back.
        assert_eq!(
            merge_go_toolchain_envs(&worker_envs(&[]), DERIVED, GoToolchainStep::Mod),
            expect(&[("GOMEMLIMIT", "2048")])
        );
    }

    #[test]
    fn parallelism_is_passed_on_the_command_line_when_it_is_a_number() {
        assert_eq!(
            go_build_parallelism_args(&merged(&[], DERIVED)),
            vec!["-p".to_string(), "3".to_string()]
        );
        // A worker group value is forwarded verbatim, so `-p` is only asserted over
        // `GOFLAGS` when it is one the toolchain would accept.
        assert!(go_build_parallelism_args(&merged(&[("GOMAXPROCS", "many")], DERIVED)).is_empty());
        // `-p` infers the base and rejects `08`, which the runtime reads as 8, so
        // the number is emitted rather than the spelling it arrived in.
        assert_eq!(
            go_build_parallelism_args(&merged(&[("GOMAXPROCS", "08")], DERIVED)),
            vec!["-p".to_string(), "8".to_string()]
        );
        // Past the signed 32-bit range, and padded with whitespace, the runtime
        // ignores the value, so `-p` has to as well: the flag would take either and
        // start that many workers.
        assert!(
            go_build_parallelism_args(&merged(&[("GOMAXPROCS", "2147483648")], DERIVED)).is_empty()
        );
        assert!(
            go_build_parallelism_args(&merged(&[("GOMAXPROCS", "2147483647 ")], DERIVED))
                .is_empty()
        );
        // A leading `+` is one the runtime does take, so the flag keeps it too.
        assert_eq!(
            go_build_parallelism_args(&merged(&[("GOMAXPROCS", "+8")], DERIVED)),
            vec!["-p".to_string(), "8".to_string()]
        );
        assert!(go_build_parallelism_args(&merged(&[], None)).is_empty());
    }
}
