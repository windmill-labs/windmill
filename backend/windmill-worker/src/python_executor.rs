use std::{
    collections::HashMap,
    process::Stdio,
    sync::{Arc, RwLock},
};

use itertools::Itertools;
use regex::Regex;
use serde_json::value::RawValue;
use sqlx::{types::Json, Pool, Postgres};
use tokio::{
    fs::{metadata, DirBuilder, File},
    io::AsyncReadExt,
    process::Command,
};
use uuid::Uuid;
#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::ee::{get_license_plan, LicensePlan};
use windmill_common::{
    error::{self, Error},
    jobs::{QueuedJob, PREPROCESSOR_FAKE_ENTRYPOINT},
    utils::calculate_hash,
    worker::{write_file, PythonAnnotations, WORKER_CONFIG},
    DB,
};

#[cfg(feature = "enterprise")]
use windmill_common::variables::get_secret_value_as_admin;

use windmill_queue::{append_logs, CanceledBy};

lazy_static::lazy_static! {


    // Default python
    static ref PYTHON_PATH: String =
    std::env::var("PYTHON_PATH").unwrap_or_else(|_| "/usr/local/bin/python3".to_string());

    static ref UV_PATH: String =
    std::env::var("UV_PATH").unwrap_or_else(|_| "/usr/local/bin/uv".to_string());

    static ref FLOCK_PATH: String =
    std::env::var("FLOCK_PATH").unwrap_or_else(|_| "/usr/bin/flock".to_string());
    static ref NON_ALPHANUM_CHAR: Regex = regex::Regex::new(r"[^0-9A-Za-z=.-]").unwrap();

    static ref PIP_TRUSTED_HOST: Option<String> = std::env::var("PIP_TRUSTED_HOST").ok();
    static ref PIP_INDEX_CERT: Option<String> = std::env::var("PIP_INDEX_CERT").ok();

    static ref USE_PIP_COMPILE: bool = std::env::var("USE_PIP_COMPILE")
        .ok().map(|flag| flag == "true").unwrap_or(false);

    /// Use pip install
    static ref USE_PIP_INSTALL: bool = std::env::var("USE_PIP_INSTALL")
        .ok().map(|flag| flag == "true").unwrap_or(false);

    static ref NO_UV_RUN: bool = std::env::var("NO_UV_RUN")
        .ok().map(|flag| flag == "true").unwrap_or(false);


    static ref RELATIVE_IMPORT_REGEX: Regex = Regex::new(r#"(import|from)\s(((u|f)\.)|\.)"#).unwrap();

    static ref EPHEMERAL_TOKEN_CMD: Option<String> = std::env::var("EPHEMERAL_TOKEN_CMD").ok();
}

const NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT: &str = include_str!("../nsjail/download.py.config.proto");
const NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT_FALLBACK: &str =
    include_str!("../nsjail/download.py.pip.config.proto");
const NSJAIL_CONFIG_RUN_PYTHON3_CONTENT: &str = include_str!("../nsjail/run.python3.config.proto");
const RELATIVE_PYTHON_LOADER: &str = include_str!("../loader.py");

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use crate::global_cache::{build_tar_and_push, pull_from_tar};

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS;

use crate::{
    common::{
        create_args_and_out_file, get_main_override, get_reserved_variables, read_file,
        read_result, start_child_process, OccupancyMetrics,
    },
    handle_child::handle_child,
    AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, HTTPS_PROXY, HTTP_PROXY,
    INSTANCE_PYTHON_VERSION, LOCK_CACHE_DIR, NO_PROXY, NSJAIL_PATH, PATH_ENV, PIP_CACHE_DIR,
    PIP_EXTRA_INDEX_URL, PIP_INDEX_URL, PY311_CACHE_DIR, TZ_ENV, UV_CACHE_DIR,
};

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum PyVersion {
    Py310,
    Py311,
    Py312,
    Py313,
}

impl PyVersion {
    /// e.g.: `/tmp/windmill/cache/python_3xy`
    pub fn to_cache_dir(&self) -> String {
        use windmill_common::worker::ROOT_CACHE_DIR;
        format!("{ROOT_CACHE_DIR}python_{}", &self.to_cache_dir_top_level())
    }
    /// e.g.: `python_3xy`
    pub fn to_cache_dir_top_level(&self) -> String {
        format!("python_{}", self.to_string_no_dots())
    }
    /// e.g.: `(to_cache_dir(), to_cache_dir_top_level())`
    pub fn to_cache_dir_tuple(&self) -> (String, String) {
        use windmill_common::worker::ROOT_CACHE_DIR;
        let top_level = self.to_cache_dir_top_level();
        (format!("{ROOT_CACHE_DIR}python_{}", &top_level), top_level)
    }
    /// e.g.: `3xy`
    pub fn to_string_no_dots(&self) -> String {
        self.to_string_with_dots().replace('.', "")
    }
    /// e.g.: `3.xy`
    pub fn to_string_with_dots(&self) -> &str {
        use PyVersion::*;
        match self {
            Py310 => "3.10",
            Py311 => "3.11",
            Py312 => "3.12",
            Py313 => "3.13",
        }
    }
    pub fn from_string_with_dots(value: &str) -> Option<Self> {
        use PyVersion::*;
        match value {
            "3.10" => Some(Py310),
            "3.11" => Some(Py311),
            "3.12" => Some(Py312),
            "3.13" => Some(Py313),
            _ => None,
        }
    }
    /// e.g.: `# py-3.xy` -> `PyVersion::Py3XY`
    pub fn parse_lockfile(line: &str) -> Option<Self> {
        Self::from_string_with_dots(line.replace("# py-", "").as_str())
    }
    pub fn from_py_annotations(a: PythonAnnotations) -> Option<Self> {
        let PythonAnnotations { py310, py311, py312, py313, .. } = a;
        use PyVersion::*;
        if py313 {
            Some(Py313)
        } else if py312 {
            Some(Py312)
        } else if py311 {
            Some(Py311)
        } else if py310 {
            Some(Py310)
        } else {
            None
        }
    }
    pub async fn install_python(
        job_dir: &str,
        job_id: &Uuid,
        mem_peak: &mut i32,
        // canceled_by: &mut Option<CanceledBy>,
        db: &Pool<Postgres>,
        worker_name: &str,
        w_id: &str,
        occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
        version: &str,
    ) -> error::Result<()> {
        let logs = String::new();
        // let v_with_dot = self.to_string_with_dots();
        let mut child_cmd = Command::new(UV_PATH.as_str());
        child_cmd
            .current_dir(job_dir)
            .args(["python", "install", version])
            // TODO: Do we need these?
            .envs([
                ("UV_PYTHON_INSTALL_DIR", "/tmp/windmill/cache/python"),
                ("UV_PYTHON_PREFERENCE", "only-managed"),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child_process = start_child_process(child_cmd, "uv").await?;

        append_logs(&job_id, &w_id, logs, db).await;
        handle_child(
            job_id,
            db,
            mem_peak,
            &mut None,
            child_process,
            false,
            worker_name,
            &w_id,
            "uv",
            None,
            false,
            occupancy_metrics,
        )
        .await
    }
    async fn get_python_inner(
        job_dir: &str,
        job_id: &Uuid,
        mem_peak: &mut i32,
        // canceled_by: &mut Option<CanceledBy>,
        db: &Pool<Postgres>,
        worker_name: &str,
        w_id: &str,
        occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
        version: &str,
    ) -> error::Result<String> {
        let py_path = Self::find_python(job_dir, version).await;

        // Python is not installed
        if py_path.is_err() {
            // Install it
            if let Err(err) = Self::install_python(
                job_dir,
                job_id,
                mem_peak,
                db,
                worker_name,
                w_id,
                occupancy_metrics,
                version,
            )
            .await
            {
                tracing::error!("Cannot install python: {err}");
                return Err(err);
            } else {
                // Try to find one more time
                let py_path = Self::find_python(job_dir, version).await;

                if let Err(err) = py_path {
                    tracing::error!("Cannot find python version {err}");
                    return Err(err);
                }

                // TODO: Cache the result
                py_path
            }
        } else {
            py_path
        }
    }
    pub async fn get_python(
        &self,
        job_dir: &str,
        job_id: &Uuid,
        mem_peak: &mut i32,
        // canceled_by: &mut Option<CanceledBy>,
        db: &Pool<Postgres>,
        worker_name: &str,
        w_id: &str,
        occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    ) -> error::Result<String> {
        lazy_static::lazy_static! {
            static ref PYTHON_PATHS: Arc<RwLock<HashMap<PyVersion, String>>> = Arc::new(RwLock::new(HashMap::new()));
        }

        Self::get_python_inner(
            job_dir,
            job_id,
            mem_peak,
            db,
            worker_name,
            w_id,
            occupancy_metrics,
            self.to_string_with_dots(),
        )
        .await
    }
    async fn find_python(job_dir: &str, version: &str) -> error::Result<String> {
        // let mut logs = String::new();
        // let v_with_dot = self.to_string_with_dots();
        let mut child_cmd = Command::new(UV_PATH.as_str());
        let output = child_cmd
            .current_dir(job_dir)
            .args(["python", "find", version])
            .envs([
                ("UV_PYTHON_INSTALL_DIR", "/tmp/windmill/cache/python"),
                ("UV_PYTHON_PREFERENCE", "only-managed"),
            ])
            // .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // Check if the command was successful
        if output.status.success() {
            // Convert the output to a String
            let stdout =
                String::from_utf8(output.stdout).expect("Failed to convert output to String");
            return Ok(stdout.replace('\n', ""));
        } else {
            // If the command failed, print the error
            let stderr =
                String::from_utf8(output.stderr).expect("Failed to convert error output to String");
            return Err(error::Error::FindPythonError(stderr));
        }
    }
}

#[cfg(windows)]
use crate::SYSTEM_ROOT;

pub async fn create_dependencies_dir(job_dir: &str) {
    DirBuilder::new()
        .recursive(true)
        .create(&format!("{job_dir}/dependencies"))
        .await
        .expect("could not create dependencies dir");
}

#[inline(always)]
pub fn handle_ephemeral_token(x: String) -> String {
    #[cfg(feature = "enterprise")]
    {
        if let Some(full_cmd) = EPHEMERAL_TOKEN_CMD.as_ref() {
            let mut splitted = full_cmd.split(" ");
            let cmd = splitted.next().unwrap();
            let args = splitted.collect::<Vec<&str>>();
            let output = std::process::Command::new(cmd)
                .args(args)
                .output()
                .map(|x| String::from_utf8(x.stdout).unwrap())
                .unwrap_or_else(|e| panic!("failed to execute  replace_ephemeral command: {}", e));
            let r = x.replace("EPHEMERAL_TOKEN", &output.trim());
            tracing::debug!("replaced ephemeral token: '{}'", r);
            return r;
        }
    }
    x
}

/// Returns lockfile and python version
pub async fn uv_pip_compile(
    job_id: &Uuid,
    requirements: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &Pool<Postgres>,
    worker_name: &str,
    w_id: &str,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    // If not set, will default to INSTANCE_PYTHON_VERSION
    // Will always be Some after execution
    py_version: PyVersion,
    // Fallback to pip-compile. Will be removed in future
    mut no_uv: bool,
    // Debug-only flag
    no_cache: bool,
) -> error::Result<String> {
    let mut logs = String::new();
    logs.push_str(&format!("\nresolving dependencies..."));
    logs.push_str(&format!("\ncontent of requirements:\n{}\n", requirements));

    // New version, the version what we wanna have

    let requirements = if let Some(pip_local_dependencies) =
        WORKER_CONFIG.read().await.pip_local_dependencies.as_ref()
    {
        let deps = pip_local_dependencies.clone();
        let compiled_deps = deps.iter().map(|dep| {
            let compiled_dep = Regex::new(dep);
            match compiled_dep {
                Ok(compiled_dep) => Some(compiled_dep),
                Err(e) => {
                    tracing::warn!("regex compilation failed for Python local dependency: '{}' - it will be ignored", e);
                    return None;
                }
            }
        }).filter(|dep_maybe| dep_maybe.is_some()).map(|dep| dep.unwrap()).collect::<Vec<Regex>>();
        requirements
            .lines()
            .filter(|s| {
                if compiled_deps.iter().any(|dep| dep.is_match(s)) {
                    logs.push_str(&format!("\nignoring local dependency: {}", s));
                    return false;
                } else {
                    return true;
                }
            })
            .join("\n")
    } else {
        requirements.to_string()
    };

    #[cfg(feature = "enterprise")]
    let requirements = replace_pip_secret(db, w_id, &requirements, worker_name, job_id).await?;

    let mut req_hash = format!("py-{}", calculate_hash(&requirements));

    if no_uv || *USE_PIP_COMPILE {
        logs.push_str(&format!("\nFallback to pip-compile (Deprecated!)"));
        // Set no_uv if not setted
        no_uv = true;
        // Make sure that if we put #no_uv (switch to pip-compile) to python code or used `USE_PIP_COMPILE=true` variable.
        // Windmill will recalculate lockfile using pip-compile and dont take potentially broken lockfile (generated by uv) from cache (our db).
        // It will recalculate lockfile even if inputs have not been changed.
        req_hash.push_str("-no_uv");
        // Will be in format:
        //     py-000..000-no_uv
    }
    if !no_cache {
        /*
            There are several scenarious of flow
                1. Cache does not exist for script
                2. Cache exists and in cached lockfile there is no python version
                3. Cache exists and in cached lockfile there is python version
            Flows:
                1: We calculate lockfile and add python version to it
                2: Only possible if script exists since versions of windmill that dont support multipython
                    1. If INSTANCE_PYTHON_VERSION == 3.11 assign this version to lockfile and return
                    3. If INSTANCE_PYTHON_VERSION != 3.11 recalculate lockfile
                3:
                    1. if cached_lockfile != annotated_version recalculate lockfile
                    else: return cache
        */
        if let Some(cached) = sqlx::query_scalar!(
            "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
            // Python version is not included in hash,
            // meaning hash will stay the same independant from python version
            req_hash
        )
        .fetch_optional(db)
        .await?
        {
            if let Some(line) = cached.lines().next() {
                if let Some(cached_version) = PyVersion::parse_lockfile(line) {
                    // We should overwrite any version in lockfile
                    // And only return cache if cached_version == new_version
                    // This will trigger recompilation for all deps,
                    // but it is ok, since we do only on deploy and cached lockfiles for non-deployed scripts
                    // are being cleaned up every 3 days anyway
                    if py_version == cached_version {
                        // All good we found cache
                        logs.push_str(&format!("\nFound cached resolution: {req_hash}"));
                        return Ok(cached);
                    }
                    // Annotated version should be used, thus lockfile regenerated
                } else {
                    tracing::info!(
                        "There is no assigned python version to script in job: {job_id:?}\n"
                    );
                    // We will assign a python version to this script
                }
                // TODO: Small optimisation
                // We end up here only if we try to redeploy script without assigned version
                // So we could check if final_version == 3.11 and if so, assign python version (write to lockfile)
                // and return cache
                // else if new_py_version == PyVersion::Py311 {
                //     tracing::info!(
                //         "There is no assigned python version to script in job: {job_id:?}\n"
                //     );
                //     logs.push_str(&format!("\nFound cached resolution: {req_hash}"));
                //     return Ok(cached);
                //     // We will assign a python version to this script
                // }
            } else {
                tracing::error!("No requirement specified in uv_pip_compile");
            }
        }
    }
    let file = "requirements.in";

    write_file(job_dir, file, &requirements)?;

    // Fallback pip-compile. Will be removed in future
    if no_uv {
        tracing::debug!("Fallback to pip-compile");

        let mut args = vec![
            "-q",
            "--no-header",
            file,
            "--resolver=backtracking",
            "--strip-extras",
        ];
        let mut pip_args = vec![];
        let pip_extra_index_url = PIP_EXTRA_INDEX_URL
            .read()
            .await
            .clone()
            .map(handle_ephemeral_token);
        if let Some(url) = pip_extra_index_url.as_ref() {
            args.extend(["--extra-index-url", url, "--no-emit-index-url"]);
            pip_args.push(format!("--extra-index-url {}", url));
        }
        let pip_index_url = PIP_INDEX_URL
            .read()
            .await
            .clone()
            .map(handle_ephemeral_token);
        if let Some(url) = pip_index_url.as_ref() {
            args.extend(["--index-url", url, "--no-emit-index-url"]);
            pip_args.push(format!("--index-url {}", url));
        }
        if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
            args.extend(["--trusted-host", host]);
        }
        if let Some(cert_path) = PIP_INDEX_CERT.as_ref() {
            args.extend(["--cert", cert_path]);
        }
        let pip_args_str = pip_args.join(" ");
        if pip_args.len() > 0 {
            args.extend(["--pip-args", &pip_args_str]);
        }
        tracing::debug!("pip-compile args: {:?}", args);

        let mut child_cmd = Command::new("pip-compile");
        child_cmd
            .current_dir(job_dir)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child_process = start_child_process(child_cmd, "pip-compile").await?;
        append_logs(&job_id, &w_id, logs, db).await;
        handle_child(
            job_id,
            db,
            mem_peak,
            canceled_by,
            child_process,
            false,
            worker_name,
            &w_id,
            "pip-compile",
            None,
            false,
            occupancy_metrics,
        )
        .await
        .map_err(|e| Error::ExecutionErr(format!("Lock file generation failed: {e:?}")))?;
    } else {
        let mut args = vec![
            "pip",
            "compile",
            "-q",
            "--no-header",
            file,
            "--strip-extras",
            "-o",
            "requirements.txt",
            // Prefer main index over extra
            // https://docs.astral.sh/uv/pip/compatibility/#packages-that-exist-on-multiple-indexes
            // TODO: Use env variable that can be toggled from UI
            "--index-strategy",
            "unsafe-best-match",
            // Target to /tmp/windmill/cache/uv
            "--cache-dir",
            UV_CACHE_DIR,
            "-p",
            py_version.to_string_with_dots(),
        ];
        if no_cache {
            args.extend(["--no-cache"]);
        }
        let pip_extra_index_url = PIP_EXTRA_INDEX_URL
            .read()
            .await
            .clone()
            .map(handle_ephemeral_token);
        if let Some(url) = pip_extra_index_url.as_ref() {
            args.extend(["--extra-index-url", url]);
        }
        let pip_index_url = PIP_INDEX_URL
            .read()
            .await
            .clone()
            .map(handle_ephemeral_token);
        if let Some(url) = pip_index_url.as_ref() {
            args.extend(["--index-url", url]);
        }
        if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
            args.extend(["--trusted-host", host]);
        }
        if let Some(cert_path) = PIP_INDEX_CERT.as_ref() {
            args.extend(["--cert", cert_path]);
        }
        tracing::debug!("uv args: {:?}", args);

        #[cfg(windows)]
        let uv_cmd = "uv";

        #[cfg(unix)]
        let uv_cmd = UV_PATH.as_str();

        let mut child_cmd = Command::new(uv_cmd);
        child_cmd
            .current_dir(job_dir)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child_process = start_child_process(child_cmd, "/usr/local/bin/uv").await?;
        append_logs(&job_id, &w_id, logs, db).await;
        handle_child(
            job_id,
            db,
            mem_peak,
            canceled_by,
            child_process,
            false,
            worker_name,
            &w_id,
            // TODO: Rename to uv-pip-compile?
            "uv",
            None,
            false,
            occupancy_metrics,
        )
        .await
        .map_err(|e| Error::ExecutionErr(format!("Lock file generation failed: {e:?}")))?;
    }

    let path_lock = format!("{job_dir}/requirements.txt");
    let mut file = File::open(path_lock).await?;
    let mut req_content = "".to_string();
    file.read_to_string(&mut req_content).await?;
    let lockfile = format!(
        "# py-{}\n{}",
        py_version.to_string_with_dots(),
        req_content
            .lines()
            .filter(|x| !x.trim_start().starts_with('#'))
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join("\n")
    );
    sqlx::query!(
        "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('3 days')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = $2",
        req_hash,
        // format!("# py-{}\n{lockfile}",py_version.unwrap_or(&*INSTANCE_PYTHON_VERSION))
        // format!("# py-{}\n{lockfile}",py_version)
        lockfile
    ).fetch_optional(db).await?;

    Ok(lockfile)
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_python_job(
    requirements_o: Option<String>,
    job_dir: &str,
    worker_dir: &str,
    worker_name: &str,
    job: &QueuedJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    inner_content: &String,
    shared_mount: &str,
    base_internal_url: &str,
    envs: HashMap<String, String>,
    new_args: &mut Option<HashMap<String, Box<RawValue>>>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> windmill_common::error::Result<Box<RawValue>> {
    let script_path = crate::common::use_flow_root_path(job.script_path());
    let (py_version, additional_python_paths) = handle_python_deps(
        job_dir,
        requirements_o,
        inner_content,
        &job.workspace_id,
        &script_path,
        &job.id,
        db,
        worker_name,
        worker_dir,
        mem_peak,
        canceled_by,
        &mut Some(occupancy_metrics),
    )
    .await?;
    let annotations = windmill_common::worker::PythonAnnotations::parse(inner_content);

    let no_uv = *NO_UV_RUN | annotations.no_uv | annotations.no_uv_run;

    append_logs(
        &job.id,
        &job.workspace_id,
        format!(
            "\n\n--- PYTHON ({}) CODE EXECUTION ---\n",
            py_version.to_string_with_dots()
        ),
        db,
    )
    .await;

    let (
        import_loader,
        import_base64,
        import_datetime,
        module_dir_dot,
        dirs,
        last,
        transforms,
        spread,
        main_name,
        pre_spread,
    ) = prepare_wrapper(
        job_dir,
        inner_content,
        &script_path,
        job.args.as_ref(),
        false,
    )
    .await?;

    let apply_preprocessor = pre_spread.is_some();

    create_args_and_out_file(&client, job, job_dir, db).await?;

    let preprocessor = if let Some(pre_spread) = pre_spread {
        format!(
            r#"if inner_script.preprocessor is None or not callable(inner_script.preprocessor):
        raise ValueError("preprocessor function is missing")
    else:
        pre_args = {{}}
        {pre_spread}
        for k, v in list(pre_args.items()):
            if v == '<function call>':
                del pre_args[k]
        kwargs = inner_script.preprocessor(**pre_args)
        kwrags_json = res_to_json(kwargs)    
        with open("args.json", 'w') as f:
            f.write(kwrags_json)"#
        )
    } else {
        "".to_string()
    };

    let os_main_override = if let Some(main_override) = main_name.as_ref() {
        format!("os.environ[\"MAIN_OVERRIDE\"] = \"{main_override}\"\n")
    } else {
        String::new()
    };
    let main_override = main_name.unwrap_or_else(|| "main".to_string());
    let wrapper_content: String = format!(
        r#"
import os
import json
{import_loader}
{import_base64}
{import_datetime}
import traceback
import sys
{os_main_override}
from {module_dir_dot} import {last} as inner_script
import re

with open("args.json") as f:
    kwargs = json.load(f, strict=False)
args = {{}}
{transforms}

def to_b_64(v: bytes):
    import base64
    b64 = base64.b64encode(v)
    return b64.decode('ascii')

replace_nan = re.compile(r'(?:\bNaN\b|\\*\\u0000)')

result_json = os.path.join(os.path.abspath(os.path.dirname(__file__)), "result.json")

def res_to_json(res):
    typ = type(res)
    if typ.__name__ == 'DataFrame':
        if typ.__module__ == 'pandas.core.frame':
            res = res.values.tolist()
        elif typ.__module__ == 'polars.dataframe.frame':
            res = res.rows()
    elif typ.__name__ == 'bytes':
        res = to_b_64(res)
    elif typ.__name__ == 'dict':
        for k, v in res.items():
            if type(v).__name__ == 'bytes':
                res[k] = to_b_64(v)
    return re.sub(replace_nan, ' null ', json.dumps(res, separators=(',', ':'), default=str).replace('\n', ''))

try:
    {preprocessor}
    {spread}
    for k, v in list(args.items()):
        if v == '<function call>':
            del args[k]
    if inner_script.{main_override} is None or not callable(inner_script.{main_override}):
        raise ValueError("{main_override} function is missing")
    res = inner_script.{main_override}(**args)
    res_json = res_to_json(res)
    with open(result_json, 'w') as f:
        f.write(res_json)
except BaseException as e:
    exc_type, exc_value, exc_traceback = sys.exc_info()
    tb = traceback.format_tb(exc_traceback)
    with open(result_json, 'w') as f:
        err = {{ "message": str(e), "name": e.__class__.__name__, "stack": '\n'.join(tb[1:]) }}
        extra = e.__dict__ 
        if extra and len(extra) > 0:
            err['extra'] = extra
        flow_node_id = os.environ.get('WM_FLOW_STEP_ID')
        if flow_node_id:
            err['step_id'] = flow_node_id
        err_json = json.dumps(err, separators=(',', ':'), default=str).replace('\n', '')
        f.write(err_json)
        sys.exit(1)
"#,
    );
    write_file(job_dir, "wrapper.py", &wrapper_content)?;

    let client = client.get_authed().await;
    let mut reserved_variables = get_reserved_variables(job, &client.token, db).await?;
    let additional_python_paths_folders = additional_python_paths.iter().join(":");

    #[cfg(windows)]
    let additional_python_paths_folders = additional_python_paths_folders.replace(":", ";");

    if !*DISABLE_NSJAIL {
        let shared_deps = additional_python_paths
            .into_iter()
            .map(|pp| {
                format!(
                    r#"
mount {{
    src: "{pp}"
    dst: "{pp}"
    is_bind: true
    rw: false
}}
        "#
                )
            })
            .join("\n");
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_PYTHON3_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{SHARED_DEPENDENCIES}", shared_deps.as_str())
                .replace("{MAIN}", format!("{dirs}/{last}").as_str())
                .replace(
                    "{ADDITIONAL_PYTHON_PATHS}",
                    additional_python_paths_folders.as_str(),
                ),
        )?;
    } else {
        reserved_variables.insert("PYTHONPATH".to_string(), additional_python_paths_folders);
    }

    tracing::info!(
        workspace_id = %job.workspace_id,
        "started python code execution {}",
        job.id
    );

    let python_path = py_version
        .get_python(
            job_dir,
            &job.id,
            mem_peak,
            db,
            worker_name,
            &job.workspace_id,
            &mut Some(occupancy_metrics),
        )
        .await?
        .replace('\n', "");

    let child = if !*DISABLE_NSJAIL {
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            // inject PYTHONPATH here - for some reason I had to do it in nsjail conf
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("BASE_URL", base_internal_url)
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                if no_uv {
                    PYTHON_PATH.as_str()
                } else {
                    &python_path
                },
                "-u",
                "-m",
                "wrapper",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
    } else {
        // let mut python_cmd = Command::new(PYTHON_PATH.as_str());
        // tracing::error!("{}", python_path);
        let mut python_cmd = if no_uv {
            Command::new(PYTHON_PATH.as_str())
        } else {
            Command::new(python_path.as_str())
        };

        let args = vec!["-u", "-m", "wrapper"];
        python_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        python_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

        if no_uv {
            start_child_process(python_cmd, PYTHON_PATH.as_str()).await?
        } else {
            start_child_process(python_cmd, python_path.as_str()).await?
        }
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
        "python run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
    )
    .await?;

    if apply_preprocessor {
        let args = read_file(&format!("{job_dir}/args.json"))
            .await
            .map_err(|e| {
                error::Error::InternalErr(format!(
                    "error while reading args from preprocessing: {e:#}"
                ))
            })?;
        let args: HashMap<String, Box<RawValue>> =
            serde_json::from_str(args.get()).map_err(|e| {
                error::Error::InternalErr(format!(
                    "error while deserializing args from preprocessing: {e:#}"
                ))
            })?;
        *new_args = Some(args.clone());
    }

    read_result(job_dir).await
}

async fn prepare_wrapper(
    job_dir: &str,
    inner_content: &str,
    script_path: &str,
    args: Option<&Json<HashMap<String, Box<RawValue>>>>,
    skip_preprocessor: bool,
) -> error::Result<(
    &'static str,
    &'static str,
    &'static str,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
)> {
    let (main_override, apply_preprocessor) = match get_main_override(args) {
        Some(main_override) => {
            if !skip_preprocessor && main_override == PREPROCESSOR_FAKE_ENTRYPOINT {
                (None, true)
            } else {
                (Some(main_override), false)
            }
        }
        None => (None, false),
    };

    let relative_imports = RELATIVE_IMPORT_REGEX.is_match(&inner_content);

    let script_path_splitted = script_path.split("/").map(|x| {
        if x.starts_with(|x: char| x.is_ascii_digit()) {
            format!("_{}", x)
        } else {
            x.to_string()
        }
    });
    let dirs_full = script_path_splitted
        .clone()
        .take(script_path_splitted.clone().count() - 1)
        .join("/")
        .replace("-", "_")
        .replace("@", ".");
    let dirs = if dirs_full.len() > 0 {
        dirs_full
            .strip_prefix("/")
            .unwrap_or(&dirs_full)
            .to_string()
    } else {
        "tmp".to_string()
    };
    let last = script_path_splitted
        .clone()
        .last()
        .unwrap()
        .replace("-", "_")
        .replace(" ", "_")
        .to_lowercase();
    let module_dir = format!("{}/{}", job_dir, dirs);
    tokio::fs::create_dir_all(format!("{module_dir}/")).await?;

    let _ = write_file(&module_dir, &format!("{last}.py"), inner_content)?;
    if relative_imports {
        let _ = write_file(job_dir, "loader.py", RELATIVE_PYTHON_LOADER)?;
    }

    let sig = windmill_parser_py::parse_python_signature(inner_content, main_override.clone())?;

    let pre_sig = if apply_preprocessor {
        Some(windmill_parser_py::parse_python_signature(
            inner_content,
            Some("preprocessor".to_string()),
        )?)
    } else {
        None
    };

    // transforms should be applied based on the signature of the first function called
    let init_sig = pre_sig.as_ref().unwrap_or(&sig);

    let transforms = init_sig
        .args
        .iter()
        .map(|x| match x.typ {
            windmill_parser::Typ::Bytes => {
                let name = &x.name;
                format!(
                    "if \"{name}\" in kwargs and kwargs[\"{name}\"] is not None:\n    \
                                     kwargs[\"{name}\"] = base64.b64decode(kwargs[\"{name}\"])\n",
                )
            }
            windmill_parser::Typ::Datetime => {
                let name = &x.name;
                format!(
                    "if \"{name}\" in kwargs and kwargs[\"{name}\"] is not None:\n    \
                                     kwargs[\"{name}\"] = datetime.fromisoformat(kwargs[\"{name}\"])\n",
                )
            }
            _ => "".to_string(),
        })
        .collect::<Vec<String>>()
        .join("");

    let import_loader = if relative_imports {
        "import loader"
    } else {
        ""
    };
    let import_base64 = if init_sig
        .args
        .iter()
        .any(|x| x.typ == windmill_parser::Typ::Bytes)
    {
        "import base64"
    } else {
        ""
    };
    let import_datetime = if init_sig
        .args
        .iter()
        .any(|x| x.typ == windmill_parser::Typ::Datetime)
    {
        "from datetime import datetime"
    } else {
        ""
    };
    let spread = if sig.star_kwargs {
        "args = kwargs".to_string()
    } else {
        sig.args
            .into_iter()
            .map(|x| {
                let name = &x.name;
                if x.default.is_none() {
                    format!("args[\"{name}\"] = kwargs.get(\"{name}\")")
                } else {
                    format!(
                        r#"args["{name}"] = kwargs.get("{name}")
    if args["{name}"] is None:
        del args["{name}"]"#
                    )
                }
            })
            .join("\n    ")
    };

    let pre_spread = if let Some(pre_sig) = pre_sig {
        let spread = if pre_sig.star_kwargs {
            "pre_args = kwargs".to_string()
        } else {
            pre_sig
                .args
                .into_iter()
                .map(|x| {
                    let name = &x.name;
                    if x.default.is_none() {
                        format!("pre_args[\"{name}\"] = kwargs.get(\"{name}\")")
                    } else {
                        format!(
                            r#"pre_args["{name}"] = kwargs.get("{name}")
    if pre_args["{name}"] is None:
        del pre_args["{name}"]"#
                        )
                    }
                })
                .join("\n    ")
        };
        Some(spread)
    } else {
        None
    };

    let module_dir_dot = dirs.replace("/", ".").replace("-", "_");

    Ok((
        import_loader,
        import_base64,
        import_datetime,
        module_dir_dot,
        dirs,
        last,
        transforms,
        spread,
        main_override,
        pre_spread,
    ))
}

#[cfg(feature = "enterprise")]
async fn replace_pip_secret(
    db: &DB,
    w_id: &str,
    req: &str,
    worker_name: &str,
    job_id: &Uuid,
) -> error::Result<String> {
    if PIP_SECRET_VARIABLE.is_match(req) {
        let mut joined = "".to_string();
        for req in req.lines() {
            let nreq = if PIP_SECRET_VARIABLE.is_match(req) {
                let capture = PIP_SECRET_VARIABLE.captures(req);
                let variable = capture.unwrap().get(1).unwrap().as_str();
                if !variable.contains("/PIP_SECRET_") {
                    return Err(error::Error::InternalErr(format!(
                        "invalid secret variable in pip requirements, (last part of path ma): {}",
                        req
                    )));
                }
                let secret = get_secret_value_as_admin(db, w_id, variable).await?;
                tracing::info!(
                    worker = %worker_name,
                    job_id = %job_id,
                    workspace_id = %w_id,
                    "found secret variable in pip requirements: {}",
                    req
                );
                PIP_SECRET_VARIABLE
                    .replace(req, secret.as_str())
                    .to_string()
            } else {
                req.to_string()
            };
            joined.push_str(&nreq);
            joined.push_str("\n");
        }

        Ok(joined)
    } else {
        Ok(req.to_string())
    }
}

async fn handle_python_deps(
    job_dir: &str,
    requirements_o: Option<String>,
    inner_content: &str,
    w_id: &str,
    script_path: &str,
    job_id: &Uuid,
    db: &DB,
    worker_name: &str,
    worker_dir: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
) -> error::Result<(PyVersion, Vec<String>)> {
    create_dependencies_dir(job_dir).await;

    let mut additional_python_paths: Vec<String> = WORKER_CONFIG
        .read()
        .await
        .additional_python_paths
        .clone()
        .unwrap_or_else(|| vec![])
        .clone();

    let annotations = windmill_common::worker::PythonAnnotations::parse(inner_content);
    // let mut version = PyVersion::from_py_annotations(annotations)
    //     .or(PyVersion::from_string_with_dots(&*INSTANCE_PYTHON_VERSION));
    // let mut version = PyVersion::from_py_annotations(annotations)
    //     .or(PyVersion::from_string_with_dots(&*INSTANCE_PYTHON_VERSION));
    // Precendence:
    // 1. Annotated version
    // 2. Instance version
    // 3. Hardcoded 3.11

    let instance_version =
        (INSTANCE_PYTHON_VERSION.read().await.clone()).unwrap_or("3.11".to_owned());

    let annotated_or_default_version = PyVersion::from_py_annotations(annotations).unwrap_or(
        PyVersion::from_string_with_dots(&instance_version).unwrap_or_else(|| {
            tracing::error!(
                "Cannot parse INSTANCE_PYTHON_VERSION ({:?}), fallback to 3.11",
                *INSTANCE_PYTHON_VERSION
            );
            PyVersion::Py311
        }),
    );
    let requirements = match requirements_o {
        Some(r) => r,
        None => {
            let mut already_visited = vec![];

            let requirements = windmill_parser_py_imports::parse_python_imports(
                inner_content,
                w_id,
                script_path,
                db,
                &mut already_visited,
            )
            .await?
            .join("\n");
            if requirements.is_empty() {
                // TODO: "# py-3.11".to_string()
                // TODO: Still check lockfile
                "".to_string()
            } else {
                uv_pip_compile(
                    job_id,
                    &requirements,
                    mem_peak,
                    canceled_by,
                    job_dir,
                    db,
                    worker_name,
                    w_id,
                    occupancy_metrics,
                    annotated_or_default_version,
                    annotations.no_uv || annotations.no_uv_compile,
                    annotations.no_cache,
                )
                .await
                .map_err(|e| {
                    Error::ExecutionErr(format!("pip compile failed: {}", e.to_string()))
                })?
            }
        }
    };

    // let final_version = version.unwrap_or_else(|| {
    //     tracing::error!("Version is supposed to be Some");
    //     PyVersion::Py311
    // });

    // Read more in next comment section
    let mut final_version = PyVersion::Py311;

    if requirements.len() > 0 {
        let req: Vec<&str> = requirements
            .split("\n")
            .filter(|x| !x.starts_with("--"))
            .collect();

        // uv_pip_compile stage will be skipped for deployed scripts.
        // Leaving us with 2 scenarious:
        //   1. We have version in lockfile
        //   2. We dont
        //
        // We want to use 3.11 version for scripts without assigned python version
        // Because this means that this script was deployed before multiple python version support was introduced
        // And the default version of python before this point was 3.11
        //
        // But for 1. we just parse line to get version

        // Parse lockfile's line and if there is no version, fallback to annotation_default
        if let Some(v) = PyVersion::parse_lockfile(&req[0]) {
            final_version = v;
        }
        // final_version = PyVersion::parse_lockfile(&req[0]).unwrap_or(final_version);

        let mut venv_path = handle_python_reqs(
            req,
            job_id,
            w_id,
            mem_peak,
            canceled_by,
            db,
            worker_name,
            job_dir,
            worker_dir,
            occupancy_metrics,
            final_version,
            annotations.no_uv || annotations.no_uv_install,
        )
        .await?;
        additional_python_paths.append(&mut venv_path);
    }
    Ok((final_version, additional_python_paths))
}

lazy_static::lazy_static! {
    static ref PIP_SECRET_VARIABLE: Regex = Regex::new(r"\$\{PIP_SECRET:([^\s\}]+)\}").unwrap();
}

/// pip install, include cached or pull from S3
pub async fn handle_python_reqs(
    requirements: Vec<&str>,
    job_id: &Uuid,
    w_id: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    job_dir: &str,
    worker_dir: &str,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    py_version: PyVersion,
    // TODO: Remove (Deprecated)
    mut no_uv_install: bool,
) -> error::Result<Vec<String>> {
    let mut req_paths: Vec<String> = vec![];
    // TODO: Add uv python path and preferences
    let mut vars = vec![("PATH", PATH_ENV.as_str())];
    let pip_extra_index_url;
    let pip_index_url;

    no_uv_install |= *USE_PIP_INSTALL;

    if no_uv_install {
        append_logs(&job_id, w_id, "\nFallback to pip (Deprecated!)\n", db).await;
        tracing::warn!("Fallback to pip");
    }

    let py_path = py_version
        .get_python(
            job_dir,
            job_id,
            mem_peak,
            db,
            worker_name,
            w_id,
            occupancy_metrics,
        )
        .await;
    if let Err(ref err) = py_path {
        tracing::error!("{}", err);
    }

    // TODO: Refactor
    let py_path = py_path?.replace('\n', "");

    if !*DISABLE_NSJAIL {
        append_logs(&job_id, w_id, "\nPrepare NSJAIL\n", db).await;
        pip_extra_index_url = PIP_EXTRA_INDEX_URL
            .read()
            .await
            .clone()
            .map(handle_ephemeral_token);

        if no_uv_install {
            if let Some(url) = pip_extra_index_url.as_ref() {
                vars.push(("EXTRA_INDEX_URL", url));
            }

            pip_index_url = PIP_INDEX_URL
                .read()
                .await
                .clone()
                .map(handle_ephemeral_token);

            if let Some(url) = pip_index_url.as_ref() {
                vars.push(("INDEX_URL", url));
            }
            if let Some(cert_path) = PIP_INDEX_CERT.as_ref() {
                vars.push(("PIP_INDEX_CERT", cert_path));
            }
            if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
                vars.push(("TRUSTED_HOST", host));
            }
            if let Some(http_proxy) = HTTP_PROXY.as_ref() {
                vars.push(("HTTP_PROXY", http_proxy));
            }
            if let Some(https_proxy) = HTTPS_PROXY.as_ref() {
                vars.push(("HTTPS_PROXY", https_proxy));
            }
            if let Some(no_proxy) = NO_PROXY.as_ref() {
                vars.push(("NO_PROXY", no_proxy));
            }
        } else {
            if let Some(url) = pip_extra_index_url.as_ref() {
                vars.push(("UV_EXTRA_INDEX_URL", url));
            }

            pip_index_url = PIP_INDEX_URL
                .read()
                .await
                .clone()
                .map(handle_ephemeral_token);

            if let Some(url) = pip_index_url.as_ref() {
                vars.push(("UV_INDEX_URL", url));
            }
            if let Some(cert_path) = PIP_INDEX_CERT.as_ref() {
                vars.push(("PIP_INDEX_CERT", cert_path));
            }
            if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
                vars.push(("TRUSTED_HOST", host));
            }
            if let Some(http_proxy) = HTTP_PROXY.as_ref() {
                vars.push(("HTTP_PROXY", http_proxy));
            }
            if let Some(https_proxy) = HTTPS_PROXY.as_ref() {
                vars.push(("HTTPS_PROXY", https_proxy));
            }
            if let Some(no_proxy) = NO_PROXY.as_ref() {
                vars.push(("NO_PROXY", no_proxy));
            }
        }

        let _ = write_file(
            job_dir,
            "download.config.proto",
            &(if no_uv_install {
                NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT_FALLBACK
            } else {
                NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT
            })
            .replace("{WORKER_DIR}", &worker_dir)
            .replace(
                "{CACHE_DIR}",
                if no_uv_install {
                    PIP_CACHE_DIR
                } else {
                    PY311_CACHE_DIR
                },
            )
            .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string()),
        )?;
    };

    let mut req_with_penv: Vec<(String, String)> = vec![];

    for req in requirements {
        // Ignore # py-3.xy
        if req.starts_with('#') {
            continue;
        }
        let py_prefix = if no_uv_install {
            PIP_CACHE_DIR
        } else {
            &py_version.to_cache_dir()
        };

        let venv_p = format!(
            "{py_prefix}/{}",
            req.replace(' ', "").replace('/', "").replace(':', "")
        );
        if metadata(&venv_p).await.is_ok() {
            req_paths.push(venv_p);
        } else {
            req_with_penv.push((req.to_string(), venv_p));
        }
    }

    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    enum PullFromTar {
        Pulled(String),
        NotPulled(String, String),
    }

    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if req_with_penv.len() > 0 {
        if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
            let (done_tx, mut done_rx) = tokio::sync::mpsc::channel(1);
            let job_id_2 = job_id.clone();
            let db_2 = db.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                            if let Err(e) = sqlx::query_scalar!("UPDATE queue SET last_ping = now() WHERE id = $1", &job_id_2)
                            .execute(&db_2)
                            .await {
                                tracing::error!("failed to update last_ping: {}", e);
                            }
                        }
                        _ = done_rx.recv() => {
                            break;
                        }
                    }
                }
            });

            let start = std::time::Instant::now();
            let prefix = if no_uv_install {
                "pip".to_owned()
            } else {
                py_version.to_cache_dir_top_level()
            };

            let futures = req_with_penv
                .clone()
                .into_iter()
                .map(|(req, venv_p)| {
                    let os = os.clone();
                    let prefix = prefix.clone();
                    async move {
                        if pull_from_tar(os, venv_p.clone(), prefix).await.is_ok() {
                            PullFromTar::Pulled(venv_p.to_string())
                        } else {
                            PullFromTar::NotPulled(req.to_string(), venv_p.to_string())
                        }
                    }
                })
                .collect::<Vec<_>>();
            let results = futures::future::join_all(futures).await;
            req_with_penv.clear();
            done_tx.send(()).await.expect("failed to send done");
            let mut pulled = vec![];
            for result in results {
                match result {
                    PullFromTar::Pulled(venv_p) => {
                        pulled.push(venv_p.split("/").last().unwrap_or_default().to_string());
                        req_paths.push(venv_p);
                    }
                    PullFromTar::NotPulled(req, venv_p) => {
                        req_with_penv.push((req, venv_p));
                    }
                }
            }
            if pulled.len() > 0 {
                append_logs(
                    &job_id,
                    &w_id,
                    format!(
                        "pulled {} from distributed cache in {}ms",
                        pulled.join(", "),
                        start.elapsed().as_millis()
                    ),
                    db,
                )
                .await;
            }
        }
    }

    for (req, venv_p) in req_with_penv {
        let mut logs1 = String::new();
        logs1.push_str("\n\n--- PIP INSTALL ---\n");
        logs1.push_str(&format!("\n{req} is being installed for the first time.\n It will be cached for all ulterior uses."));
        append_logs(&job_id, w_id, logs1, db).await;

        tracing::info!(
            workspace_id = %w_id,
            "started setup python dependencies"
        );

        let child = if !*DISABLE_NSJAIL {
            tracing::info!(
                workspace_id = %w_id,
                "starting nsjail"
            );
            let mut vars = vars.clone();
            let req = req.to_string();
            vars.push(("REQ", &req));
            vars.push(("TARGET", &venv_p));
            if !no_uv_install {
                vars.push(("PY_PATH", &py_path));
            }
            let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
            nsjail_cmd
                .current_dir(job_dir)
                .env_clear()
                .envs(vars)
                .args(vec!["--config", "download.config.proto"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
        } else {
            let fssafe_req = NON_ALPHANUM_CHAR.replace_all(&req, "_").to_string();
            #[cfg(unix)]
            let req = format!("'{}'", req);

            #[cfg(windows)]
            let req = format!("{}", req);

            let mut command_args = if no_uv_install {
                vec![
                    PYTHON_PATH.as_str(),
                    "-m",
                    "pip",
                    "install",
                    &req,
                    "-I",
                    "--no-deps",
                    "--no-color",
                    "--isolated",
                    "--no-warn-conflicts",
                    "--disable-pip-version-check",
                    "-t",
                    venv_p.as_str(),
                ]
            } else {
                vec![
                    UV_PATH.as_str(),
                    "pip",
                    "install",
                    &req,
                    "--no-deps",
                    "--no-color",
                    "-p",
                    &py_path,
                    // Prevent uv from discovering configuration files.
                    "--no-config",
                    "--link-mode=copy",
                    // TODO: Doublecheck it
                    // "--system",
                    // Prefer main index over extra
                    // https://docs.astral.sh/uv/pip/compatibility/#packages-that-exist-on-multiple-indexes
                    // TODO: Use env variable that can be toggled from UI
                    "--index-strategy",
                    "unsafe-best-match",
                    "--target",
                    venv_p.as_str(),
                    "--no-cache",
                ]
            };
            // panic!("{:?}", command_args);
            let pip_extra_index_url = PIP_EXTRA_INDEX_URL
                .read()
                .await
                .clone()
                .map(handle_ephemeral_token);

            if let Some(url) = pip_extra_index_url.as_ref() {
                command_args.extend(["--extra-index-url", url]);
            }
            let pip_index_url = PIP_INDEX_URL
                .read()
                .await
                .clone()
                .map(handle_ephemeral_token);

            if let Some(url) = pip_index_url.as_ref() {
                command_args.extend(["--index-url", url]);
            }
            if let Some(cert_path) = PIP_INDEX_CERT.as_ref() {
                command_args.extend(["--cert", cert_path]);
            }
            if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
                command_args.extend(["--trusted-host", &host]);
            }

            let mut envs = vec![("PATH", PATH_ENV.as_str())];
            if let Some(http_proxy) = HTTP_PROXY.as_ref() {
                envs.push(("HTTP_PROXY", http_proxy));
            }
            if let Some(https_proxy) = HTTPS_PROXY.as_ref() {
                envs.push(("HTTPS_PROXY", https_proxy));
            }
            if let Some(no_proxy) = NO_PROXY.as_ref() {
                envs.push(("NO_PROXY", no_proxy));
            }

            envs.push(("HOME", HOME_ENV.as_str()));

            tracing::debug!("pip install command: {:?}", command_args);

            // panic!(
            //     "{:?}",
            //     [
            //         "-x",
            //         &format!("{}/pip-{}.lock", LOCK_CACHE_DIR, fssafe_req),
            //         "--command",
            //         &command_args.join(" "),
            //     ]
            //     .join(" ")
            // );
            #[cfg(unix)]
            {
                let mut flock_cmd = Command::new(FLOCK_PATH.as_str());
                flock_cmd
                    .env_clear()
                    .envs(envs)
                    .args([
                        "-x",
                        &format!("{}/pip-{}.lock", LOCK_CACHE_DIR, fssafe_req),
                        "--command",
                        &command_args.join(" "),
                    ])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                start_child_process(flock_cmd, FLOCK_PATH.as_str()).await?
            }

            #[cfg(windows)]
            {
                let mut pip_cmd = Command::new(PYTHON_PATH.as_str());
                pip_cmd
                    .env_clear()
                    .envs(envs)
                    .env("SystemRoot", SYSTEM_ROOT.as_str())
                    .args(&command_args[1..])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                start_child_process(pip_cmd, PYTHON_PATH.as_str()).await?
            }
        };

        let child = handle_child(
            &job_id,
            db,
            mem_peak,
            canceled_by,
            child,
            false,
            worker_name,
            &w_id,
            &format!("pip install {req}"),
            None,
            false,
            occupancy_metrics,
        )
        .await;
        tracing::info!(
            workspace_id = %w_id,
            is_ok = child.is_ok(),
            "finished setting up python dependencies {}",
            job_id
        );
        child?;

        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
            if matches!(get_license_plan().await, LicensePlan::Pro) {
                tracing::warn!("S3 cache not available in the pro plan");
            } else {
                let venv_p = venv_p.clone();

                let (cache_dir, prefix) = if no_uv_install {
                    (PIP_CACHE_DIR.to_owned(), "pip".to_owned())
                } else {
                    py_version.to_cache_dir_tuple()
                };

                tokio::spawn(build_tar_and_push(os, venv_p, cache_dir, prefix));
            }
        }
        req_paths.push(venv_p);
    }
    Ok(req_paths)
}

#[cfg(feature = "enterprise")]
use crate::JobCompletedSender;
#[cfg(feature = "enterprise")]
use crate::{common::build_envs_map, dedicated_worker::handle_dedicated_process};
#[cfg(feature = "enterprise")]
use tokio::sync::mpsc::Receiver;
#[cfg(feature = "enterprise")]
use windmill_common::variables;

#[cfg(feature = "enterprise")]
pub async fn start_worker(
    requirements_o: Option<String>,
    db: &sqlx::Pool<sqlx::Postgres>,
    inner_content: &str,
    base_internal_url: &str,
    job_dir: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    w_id: &str,
    script_path: &str,
    token: &str,
    job_completed_tx: JobCompletedSender,
    jobs_rx: Receiver<std::sync::Arc<QueuedJob>>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> error::Result<()> {
    let mut mem_peak: i32 = 0;
    let mut canceled_by: Option<CanceledBy> = None;
    let context = variables::get_reserved_variables(
        db,
        w_id,
        &token,
        "dedicated_worker@windmill.dev",
        "dedicated_worker",
        "NOT_AVAILABLE",
        "dedicated_worker",
        Some(script_path.to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .to_vec();

    let context_envs = build_envs_map(context).await;
    let (_, additional_python_paths) = handle_python_deps(
        job_dir,
        requirements_o,
        inner_content,
        w_id,
        script_path,
        &Uuid::nil(),
        db,
        worker_name,
        job_dir,
        &mut mem_peak,
        &mut canceled_by,
        &mut None,
    )
    .await?;

    let _args = None;
    let (
        import_loader,
        import_base64,
        import_datetime,
        module_dir_dot,
        _dirs,
        last,
        transforms,
        spread,
        _,
        _,
    ) = prepare_wrapper(job_dir, inner_content, script_path, _args.as_ref(), true).await?;

    {
        let indented_transforms = transforms
            .lines()
            .map(|x| format!("    {}", x))
            .collect::<Vec<String>>()
            .join("\n");

        let wrapper_content: String = format!(
            r#"
import json
{import_loader}
{import_base64}
{import_datetime}
import traceback
import sys
from {module_dir_dot} import {last} as inner_script
import re


def to_b_64(v: bytes):
    import base64
    b64 = base64.b64encode(v)
    return b64.decode('ascii')

replace_nan = re.compile(r'(?:\bNaN\b|\\u0000)')
sys.stdout.write('start\n')

for line in sys.stdin:
    if line == 'end\n':
        break
    kwargs = json.loads(line, strict=False)
    args = {{}}
{indented_transforms}
    {spread}
    for k, v in list(args.items()):
        if v == '<function call>':
            del args[k]

    try:
        res = inner_script.main(**args)
        typ = type(res)
        if typ.__name__ == 'DataFrame':
            if typ.__module__ == 'pandas.core.frame':
                res = res.values.tolist()
            elif typ.__module__ == 'polars.dataframe.frame':
                res = res.rows()
        elif typ.__name__ == 'bytes':
            res = to_b_64(res)
        elif typ.__name__ == 'dict':
            for k, v in res.items():
                if type(v).__name__ == 'bytes':
                    res[k] = to_b_64(v)
        res_json = re.sub(replace_nan, ' null ', json.dumps(res, separators=(',', ':'), default=str).replace('\n', ''))
        sys.stdout.write("wm_res[success]:" + res_json + "\n")
    except BaseException as e:
        exc_type, exc_value, exc_traceback = sys.exc_info()
        tb = traceback.format_tb(exc_traceback)
        err_json = json.dumps({{ "message": str(e), "name": e.__class__.__name__, "stack": '\n'.join(tb[1:])  }}, separators=(',', ':'), default=str).replace('\n', '')
        sys.stdout.write("wm_res[error]:" + err_json + "\n")
    sys.stdout.flush()
"#,
        );
        write_file(job_dir, "wrapper.py", &wrapper_content)?;
    }

    let reserved_variables = windmill_common::variables::get_reserved_variables(
        db,
        w_id,
        token,
        "dedicated_worker",
        "dedicated_worker",
        Uuid::nil().to_string().as_str(),
        "dedicated_worker",
        Some(script_path.to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await;

    let mut proc_envs = HashMap::new();
    let additional_python_paths_folders = additional_python_paths.iter().join(":");
    proc_envs.insert("PYTHONPATH".to_string(), additional_python_paths_folders);
    proc_envs.insert("PATH".to_string(), PATH_ENV.to_string());
    proc_envs.insert("TZ".to_string(), TZ_ENV.to_string());
    proc_envs.insert(
        "BASE_INTERNAL_URL".to_string(),
        base_internal_url.to_string(),
    );
    proc_envs.insert("BASE_URL".to_string(), base_internal_url.to_string());
    handle_dedicated_process(
        &*PYTHON_PATH,
        job_dir,
        context_envs,
        envs,
        reserved_variables,
        proc_envs,
        ["-u", "-m", "wrapper"].to_vec(),
        killpill_rx,
        job_completed_tx,
        token,
        jobs_rx,
        worker_name,
        db,
        script_path,
        "python",
    )
    .await
}
