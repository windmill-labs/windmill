use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    process::Stdio,
    str::FromStr,
    sync::Arc,
};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

use anyhow::anyhow;
use itertools::Itertools;
use regex::Regex;
use serde_json::value::RawValue;
use tokio::{
    fs::{metadata, DirBuilder, File},
    io::AsyncReadExt,
    process::Command,
    sync::Semaphore,
    task,
};
use windmill_queue::MiniPulledJob;

use uuid::Uuid;
#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
use windmill_common::ee_oss::{get_license_plan, LicensePlan};
use windmill_common::{
    error::{
        self,
        Error::{self},
    },
    scripts::ScriptLang,
    utils::calculate_hash,
    worker::{
        copy_dir_recursively, pad_string, split_python_requirements, write_file, Connection,
        PyVAlias, PythonAnnotations, WORKER_CONFIG,
    },
};

#[cfg(feature = "enterprise")]
use windmill_common::variables::get_secret_value_as_admin;

use std::env::var;
use windmill_queue::{append_logs, CanceledBy, PrecomputedAgentInfo};

use process_wrap::tokio::TokioChildWrapper;

lazy_static::lazy_static! {
    pub(crate) static ref PYTHON_PATH: Option<String> = var("PYTHON_PATH").ok().map(|v| {
        tracing::warn!("PYTHON_PATH is set to {} and thus python will not be managed by uv and stay static regardless of annotation and instance settings. NOT RECOMMENDED", v);
        v
    });

    pub(crate) static ref UV_PATH: String =
    var("UV_PATH").unwrap_or_else(|_| "/usr/local/bin/uv".to_string());

    static ref PY_CONCURRENT_DOWNLOADS: usize =
    var("PY_CONCURRENT_DOWNLOADS").ok().map(|flag| flag.parse().unwrap_or(20)).unwrap_or(20);


    static ref NON_ALPHANUM_CHAR: Regex = regex::Regex::new(r"[^0-9A-Za-z=.-]").unwrap();

    static ref TRUSTED_HOST: Option<String> = var("PY_TRUSTED_HOST").ok().or(var("PIP_TRUSTED_HOST").ok());
    pub static ref INDEX_CERT: Option<String> = var("PY_INDEX_CERT").ok().or(var("PIP_INDEX_CERT").ok());
    pub static ref NATIVE_CERT: bool = var("PY_NATIVE_CERT").ok().or(var("UV_NATIVE_TLS").ok()).map(|flag| flag == "true").unwrap_or(false);

    static ref RELATIVE_IMPORT_REGEX: Regex = Regex::new(r#"(import|from)\s(((u|f)\.)|\.)"#).unwrap();

    static ref EPHEMERAL_TOKEN_CMD: Option<String> = var("EPHEMERAL_TOKEN_CMD").ok();
}

#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
lazy_static::lazy_static! {
    static ref PIPTAR_UPLOAD_CHANNEL: tokio::sync::mpsc::UnboundedSender<PiptarUploadTask> = {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        // Spawn background task to handle uploads sequentially
        tokio::spawn(handle_piptar_uploads(rx));

        tx
    };
}

#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
#[derive(Debug)]
struct PiptarUploadTask {
    venv_path: String,
    cache_dir: String,
}

#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
async fn handle_piptar_uploads(mut rx: tokio::sync::mpsc::UnboundedReceiver<PiptarUploadTask>) {
    use crate::global_cache::build_tar_and_push;
    use windmill_object_store::get_object_store;

    while let Some(task) = rx.recv().await {
        if let Some(os) = get_object_store().await {
            match build_tar_and_push(os, task.venv_path.clone(), task.cache_dir, None, false).await
            {
                Ok(()) => {
                    tracing::info!("Successfully uploaded piptar for {}", task.venv_path);
                }
                Err(e) => {
                    tracing::error!("Failed to upload piptar for {}: {}", task.venv_path, e);
                }
            }
        } else {
            tracing::warn!(
                "S3 object store not available for piptar upload: {}",
                task.venv_path
            );
        }
    }
}

const NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT: &str = include_str!("../nsjail/download.py.config.proto");
const NSJAIL_CONFIG_RUN_PYTHON3_CONTENT: &str = include_str!("../nsjail/run.python3.config.proto");
const RELATIVE_PYTHON_LOADER: &str = include_str!("../loader.py");

#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
use crate::global_cache::pull_from_tar;

#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
use windmill_object_store::OBJECT_STORE_SETTINGS;

use crate::{
    common::{
        build_command_with_isolation, create_args_and_out_file, get_reserved_variables, read_file,
        read_result, start_child_process, OccupancyMetrics, StreamNotifier, DEV_CONF_NSJAIL,
    },
    get_proxy_envs_for_lang,
    handle_child::handle_child,
    is_sandboxing_enabled, read_ee_registry,
    worker_utils::ping_job_status,
    PyV, DISABLE_NUSER, HOME_ENV, NSJAIL_PATH, PATH_ENV, PIP_EXTRA_INDEX_URL, PIP_INDEX_URL,
    PROXY_ENVS, PY_INSTALL_DIR, TRACING_PROXY_CA_CERT_PATH, TZ_ENV, UV_CACHE_DIR,
    UV_INDEX_STRATEGY,
};
use windmill_common::client::AuthedClient;

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

// This function only invoked during deployment of script or test run.
// And never for already deployed scripts, these have their lockfiles in PostgreSQL
// thus this function call is skipped.
/// Returns lockfile and python version
pub async fn uv_pip_compile(
    job_id: &Uuid,
    requirements: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    conn: &Connection,
    worker_name: &str,
    w_id: &str,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    py_version: PyV,
    // Debug-only flag
    no_cache: bool,
) -> error::Result<String> {
    let mut logs = String::new();
    logs.push_str(&format!("\nresolving dependencies..."));
    logs.push_str(&format!("\ncontent of requirements:\n{}\n", requirements));

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

    let uv_index_strategy = UV_INDEX_STRATEGY.read().await.clone();
    let uv_index_strategy = uv_index_strategy.as_deref().unwrap_or("unsafe-best-match");

    let py_version_str = py_version.clone().to_string();
    // Include python version to requirements.in
    // We need it because same hash based on requirements.in can get calculated even for different python versions
    // To prevent from overwriting same requirements.in but with different python versions, we include version to hash
    let requirements = format!("# py: {}\n{}", py_version.to_string(), requirements);

    #[cfg(feature = "enterprise")]
    let requirements = replace_pip_secret(conn, w_id, &requirements, worker_name, job_id).await?;

    let req_hash = format!("py-{}-{uv_index_strategy}", calculate_hash(&requirements));

    if !no_cache {
        if let Some(db) = conn.as_sql() {
            if let Some(cached) = sqlx::query_scalar!(
                "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
                // Python version is included in hash,
                // hash will be the different for every python version
                req_hash
            )
            .fetch_optional(db)
            .await?
            {
                logs.push_str(&format!(
                    "\nFound cached resolution: {req_hash}, on python version: {}",
                    &py_version_str
                ));
                return Ok(cached);
            }
        }
    }

    let file = "requirements.in";

    write_file(job_dir, file, &requirements)?;

    {
        // Make sure we have python runtime installed
        py_version
            .try_get_python(job_id, mem_peak, conn, worker_name, w_id, occupancy_metrics)
            .await?;

        let mut args = vec![
            "pip",
            "compile",
            "-q",
            "--no-header",
            file,
            "--strip-extras",
            "-o",
            "requirements.txt",
            // Target to /tmp/windmill/cache/uv
            "--cache-dir",
            UV_CACHE_DIR,
        ];

        args.extend(["-p", &py_version_str, "--python-preference", "only-managed"]);

        if no_cache {
            args.extend(["--no-cache"]);
        }
        let pip_extra_index_url = read_ee_registry(
            PIP_EXTRA_INDEX_URL.read().await.clone(),
            "pip extra index url",
            job_id,
            w_id,
            conn,
        )
        .await
        .map(handle_ephemeral_token);
        if let Some(url) = pip_extra_index_url.as_ref() {
            url.split(",").for_each(|url| {
                args.extend(["--extra-index-url", url]);
            });
        }
        let pip_index_url = read_ee_registry(
            PIP_INDEX_URL.read().await.clone(),
            "pip index url",
            job_id,
            w_id,
            conn,
        )
        .await
        .map(handle_ephemeral_token);
        if let Some(url) = pip_index_url.as_ref() {
            args.extend(["--index-url", url]);
        }
        if let Some(host) = TRUSTED_HOST.as_ref() {
            args.extend(["--trusted-host", host]);
        }
        if let Some(cert_path) = INDEX_CERT.as_ref() {
            args.extend(["--cert", cert_path]);
        }
        if *NATIVE_CERT {
            args.extend(["--native-tls"]);
        }
        tracing::debug!("uv args: {:?}", args);

        #[cfg(windows)]
        let uv_cmd = "uv";

        #[cfg(unix)]
        let uv_cmd = UV_PATH.as_str();

        let mut child_cmd = Command::new(uv_cmd);
        child_cmd
            .current_dir(job_dir)
            .env_clear()
            .env("HOME", HOME_ENV.to_string())
            .env("PATH", PATH_ENV.to_string())
            .env("UV_PYTHON_INSTALL_DIR", PY_INSTALL_DIR.to_string())
            .env("UV_INDEX_STRATEGY", uv_index_strategy)
            .envs(PROXY_ENVS.clone())
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            child_cmd
                .env("SystemRoot", SYSTEM_ROOT.as_str())
                .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
                .env("HOME", crate::USERPROFILE_ENV.as_str())
                .env(
                    "LOCALAPPDATA",
                    std::env::var("LOCALAPPDATA")
                        .unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str())),
                )
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                )
                .env(
                    "APPDATA",
                    std::env::var("APPDATA").unwrap_or_else(|_| {
                        format!("{}\\AppData\\Roaming", crate::USERPROFILE_ENV.as_str())
                    }),
                )
                .env(
                    "ComSpec",
                    std::env::var("ComSpec")
                        .unwrap_or_else(|_| String::from("C:\\Windows\\System32\\cmd.exe")),
                )
                .env(
                    "PATHEXT",
                    std::env::var("PATHEXT").unwrap_or_else(|_| {
                        String::from(".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC;.CPL")
                    }),
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
                );
        }

        let child_process = start_child_process(child_cmd, uv_cmd, false).await?;
        append_logs(&job_id, &w_id, logs, conn).await;
        handle_child(
            job_id,
            conn,
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
            None,
            None,
        )
        .await
        .map_err(|e| {
            Error::ExecutionErr(format!(
                "Lock file generation failed.\n\ncommand: {uv_cmd} {}\n\n{e:?}",
                args.join(" ")
            ))
        })?;
    }

    let path_lock = format!("{job_dir}/requirements.txt");
    let mut file = File::open(path_lock).await?;
    let mut req_content = "".to_string();
    file.read_to_string(&mut req_content).await?;
    let lockfile = format!(
        "# py: {}\n{}",
        py_version.to_string(),
        req_content
            .lines()
            .filter(|x| !x.trim_start().starts_with('#'))
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join("\n")
    );
    if let Some(db) = conn.as_sql() {
        sqlx::query!(
        "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('5 mins')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = EXCLUDED.lockfile",
        req_hash,
        lockfile
    ).fetch_optional(db).await?;
    }

    Ok(lockfile)
}

/**
    Iterate over all python paths and if same folder has same name multiple times,
    then merge the content and put to <job_dir>/site-packages

    Solves problem with imports for some dependencies.

    Default layout (/windmill/cache/):

    dep==x.y.z
    └── X
        └── A
    dep-ext==x.y.z
    └── X
        └── B

    In this case python would be confused with finding B module.

    This function will convert it to (/<job_dir>):

    site-packages
    └── X
        ├── A
        └── B

    This way python has no problems with finding correct module
*/
#[tracing::instrument(level = "trace", skip_all)]
async fn postinstall(
    additional_python_paths: &mut Vec<String>,
    job_dir: &str,
    job: &MiniPulledJob,
    conn: &Connection,
) -> windmill_common::error::Result<()> {
    // It is guranteed that additional_python_paths only contains paths within windmill/cache/
    // All other paths you would usually expect in PYTHONPATH are NOT included. These are added in downstream
    //
    //                      <PackageName, Vec<GlobalPath>>
    let mut lookup_table: HashMap<String, Vec<String>> = HashMap::new();
    // e.g.: <"requests", ["/tmp/windmill/cache/python_311/requests==1.0.0"]>
    for path in additional_python_paths.iter() {
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            // Ignore all files, we only need directories.
            // We cannot merge files.
            if entry.file_type()?.is_dir() {
                // Short name, e.g.: requests
                let name = entry
                    .file_name()
                    .to_str()
                    .ok_or(anyhow::anyhow!("Cannot convert OsString to String"))?
                    .to_owned();

                if name == "bin" || name == "__pycache__" || name.contains("dist-info") {
                    continue;
                }

                if let Some(existing_paths) = lookup_table.get_mut(&name) {
                    tracing::debug!(
                        "Found existing package name: {:?} in {}",
                        entry.file_name(),
                        path
                    );
                    existing_paths.push(path.to_owned())
                } else {
                    lookup_table.insert(name, vec![path.to_owned()]);
                }
            }
        }
    }
    let mut paths_to_remove: HashSet<String> = HashSet::new();
    // Copy to shared dir
    for existing_paths in lookup_table.values() {
        if existing_paths.len() == 1 {
            // There is only single path for given name
            // So we skip it
            continue;
        }

        for path in existing_paths {
            copy_dir_recursively(
                Path::new(path),
                &std::path::PathBuf::from(job_dir).join("site-packages"),
            )?;
            paths_to_remove.insert(path.to_owned());
        }
    }

    if !paths_to_remove.is_empty() {
        append_logs(
            &job.id,
            &job.workspace_id,
            "\n\nCopying some packages from cache to job_dir...\n".to_string(),
            conn,
        )
        .await;
        // Remove PATHs we just moved
        additional_python_paths.retain(|e| !paths_to_remove.contains(e));
        // Instead add shared path
        additional_python_paths.insert(0, format!("{job_dir}/site-packages"));
    }
    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_python_job(
    requirements_o: Option<&String>,
    job_dir: &str,
    worker_dir: &str,
    worker_name: &str,
    job: &MiniPulledJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    conn: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    inner_content: &String,
    shared_mount: &str,
    base_internal_url: &str,
    envs: HashMap<String, String>,
    new_args: &mut Option<HashMap<String, Box<RawValue>>>,
    occupancy_metrics: &mut OccupancyMetrics,
    precomputed_agent_info: Option<PrecomputedAgentInfo>,
    has_stream: &mut bool,
) -> windmill_common::error::Result<Box<RawValue>> {
    let script_path = crate::common::use_flow_root_path(job.runnable_path());

    let annotations = PythonAnnotations::parse(inner_content);

    let (py_version, mut additional_python_paths) = handle_python_deps(
        job_dir,
        requirements_o,
        inner_content,
        &job.workspace_id,
        &script_path,
        &job.id,
        conn,
        worker_name,
        worker_dir,
        mem_peak,
        canceled_by,
        &mut Some(occupancy_metrics),
        precomputed_agent_info,
        annotations.clone(),
    )
    .await?;

    tracing::debug!("Finished handling python dependencies");
    let python_path = py_version
        .get_python(
            worker_name,
            &job.id,
            &job.workspace_id,
            mem_peak,
            conn,
            &mut Some(occupancy_metrics),
        )
        .await?;

    if !annotations.no_postinstall {
        if let Err(e) = postinstall(&mut additional_python_paths, job_dir, job, conn).await {
            tracing::error!("Postinstall stage has failed. Reason: {e}");
        }
        tracing::debug!("Finished deps postinstall stage");
    }

    {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!(
                "\n\n--- PYTHON ({}) CODE EXECUTION ---\n",
                py_version.clone().to_string()
            ),
            conn,
        )
        .await;
    }
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
        job.flow_step_id.as_deref(),
        job.preprocessed,
        job.script_entrypoint_override.as_deref(),
        inner_content,
        &script_path,
    )
    .await?;

    tracing::debug!("Finished preparing wrapper");

    let apply_preprocessor = pre_spread.is_some();

    create_args_and_out_file(&client, job, job_dir, conn).await?;
    tracing::debug!("Finished preparing wrapper");

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
        kwrags_json = res_to_json(kwargs, type(kwargs))
        with open("args.json", 'w') as f:
            f.write(kwrags_json)"#
        )
    } else {
        "".to_string()
    };

    let postprocessor = get_result_postprocessor(annotations.skip_result_postprocessing);

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

replace_invalid_fields = re.compile(r'(?:\bNaN\b|\\*\\u0000|Infinity|\-Infinity)')

result_json = os.path.join(os.path.abspath(os.path.dirname(__file__)), "result.json")

def res_to_json(res, typ):
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
    unprocessed = json.dumps(res, separators=(',', ':'), default=str).replace('\n', '')
    return {postprocessor}

try:
    {preprocessor}
    {spread}
    for k, v in list(args.items()):
        if v == '<function call>':
            del args[k]
    if inner_script.{main_override} is None or not callable(inner_script.{main_override}):
        raise ValueError("{main_override} function is missing")
    res = inner_script.{main_override}(**args)
    typ = type(res)
    if hasattr(res, '__iter__') and not isinstance(res, (str, dict, list, bytes, tuple, set, frozenset, range, memoryview, bytearray)) and typ.__name__ != 'DataFrame':
        for chunk in res:
            print("WM_STREAM: " + chunk.replace('\n', '\\n'))
        res = None
    res_json = res_to_json(res, typ)
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

    tracing::debug!("Finished writing wrapper");

    let mut reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

    // Add /tmp/windmill/cache/python_x_y_z/global-site-packages to PYTHONPATH.
    // Usefull if certain wheels needs to be preinstalled before execution.
    let global_site_packages_path = py_version.to_cache_dir(true) + "/global-site-packages";
    let additional_python_paths_folders = {
        let mut paths = additional_python_paths.clone();
        if std::fs::metadata(&global_site_packages_path).is_ok() {
            // We want global_site_packages_path to be included in additonal_python_paths_folders, but
            // we don't want it to be included in global_site_packages_path.
            // The reason for this is that additional_python_paths_folders is used to fill PYTHONPATH env variable for jailed script
            // When global_site_packages_path used to place mount point of wheels to the jail config.
            // Since we handle mount of global_site_packages on our own, we don't want it to be mounted automatically.
            // We do this because existence of every wheel in cache is mandatory and if it is not there and nsjail expects it, it is a bug.
            // On the other side global_site_packages is purely optional.
            // NOTE: This behaviour can be changed in future, so verification of wheels can be delegated from nsjail to windmill
            paths.insert(0, global_site_packages_path.clone());
            //    ^^^^^^ ^
            // We also want this be priorotized, that's why we insert it to the beginning
        }
        #[cfg(windows)]
        {
            paths.iter().join(";")
        }
        #[cfg(not(windows))]
        {
            paths.iter().join(":")
        }
    };

    #[cfg(windows)]
    let additional_python_paths_folders = additional_python_paths_folders.replace(":", ";");

    if is_sandboxing_enabled() {
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
                .replace("{PY_INSTALL_DIR}", PY_INSTALL_DIR)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{SHARED_DEPENDENCIES}", shared_deps.as_str())
                .replace("{GLOBAL_SITE_PACKAGES}", &global_site_packages_path)
                .replace("{MAIN}", format!("{dirs}/{last}").as_str())
                .replace(
                    "{ADDITIONAL_PYTHON_PATHS}",
                    additional_python_paths_folders.as_str(),
                )
                .replace("{TRACING_PROXY_CA_CERT_PATH}", TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL),
        )?;
    } else {
        reserved_variables.insert("PYTHONPATH".to_string(), additional_python_paths_folders);
    }

    tracing::info!(
        workspace_id = %job.workspace_id,
        "started python code execution {}",
        job.id
    );

    let child = if is_sandboxing_enabled() {
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            // inject PYTHONPATH here - for some reason I had to do it in nsjail conf
            .envs(reserved_variables)
            .envs(get_proxy_envs_for_lang(&ScriptLang::Python3).await?)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("BASE_URL", base_internal_url)
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                &python_path,
                "-u",
                "-m",
                "wrapper",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        let args = vec!["-u", "-m", "wrapper"];

        let mut python_cmd = build_command_with_isolation(&python_path, &args);
        python_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(get_proxy_envs_for_lang(&ScriptLang::Python3).await?)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            python_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());
            python_cmd.env("USERPROFILE", crate::USERPROFILE_ENV.as_str());
            python_cmd.env("windir", SYSTEM_ROOT.as_str());
            python_cmd.env(
                "LOCALAPPDATA",
                std::env::var("LOCALAPPDATA")
                    .unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str())),
            );
        }

        start_child_process(python_cmd, &python_path, false).await?
    };

    let stream_notifier = StreamNotifier::new(conn, job);

    let handle_result = handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        is_sandboxing_enabled(),
        worker_name,
        &job.workspace_id,
        "python run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        stream_notifier,
    )
    .await?;

    *has_stream = handle_result.result_stream.is_some();

    if apply_preprocessor {
        let args = read_file(&format!("{job_dir}/args.json"))
            .await
            .map_err(|e| {
                error::Error::internal_err(format!(
                    "error while reading args from preprocessing: {e:#}"
                ))
            })?;
        let args: HashMap<String, Box<RawValue>> =
            serde_json::from_str(args.get()).map_err(|e| {
                error::Error::internal_err(format!(
                    "error while deserializing args from preprocessing: {e:#}"
                ))
            })?;
        *new_args = Some(args.clone());
    }

    read_result(job_dir, handle_result.result_stream).await
}

async fn prepare_wrapper(
    job_dir: &str,
    job_flow_step_id: Option<&str>,
    job_preprocessed: Option<bool>,
    job_script_entrypoint_override: Option<&str>,
    inner_content: &str,
    script_path: &str,
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
    let main_override = job_script_entrypoint_override.as_deref();
    let apply_preprocessor =
        job_flow_step_id != Some("preprocessor") && job_preprocessed == Some(false);

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

    let sig = windmill_parser_py::parse_python_signature(
        inner_content,
        main_override.map(ToString::to_string),
        false,
    )?;

    let pre_sig = if apply_preprocessor {
        Some(windmill_parser_py::parse_python_signature(
            inner_content,
            Some("preprocessor".to_string()),
            false,
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
            windmill_parser::Typ::Date => {
                let name = &x.name;
                format!(
                    "if \"{name}\" in kwargs and kwargs[\"{name}\"] is not None:\n    \
                                     try:\n        \
                                         kwargs[\"{name}\"] = date.fromisoformat(kwargs[\"{name}\"])\n    \
                                     except ValueError:\n        \
                                         for _fmt in (\"%d-%m-%Y\", \"%m/%d/%Y\", \"%d/%m/%Y\", \"%Y/%m/%d\"):\n            \
                                             try:\n                \
                                                 kwargs[\"{name}\"] = datetime.strptime(kwargs[\"{name}\"], _fmt).date()\n                \
                                                 break\n            \
                                             except ValueError:\n                \
                                                 continue\n",
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
    let has_datetime = init_sig
        .args
        .iter()
        .any(|x| x.typ == windmill_parser::Typ::Datetime);
    let has_date = init_sig
        .args
        .iter()
        .any(|x| x.typ == windmill_parser::Typ::Date);
    let import_datetime = match (has_datetime, has_date) {
        (true, true) => "from datetime import datetime, date",
        (true, false) => "from datetime import datetime",
        (false, true) => "from datetime import datetime, date",
        (false, false) => "",
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
                .join("\n        ")
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
        main_override.map(ToString::to_string),
        pre_spread,
    ))
}

#[cfg(feature = "enterprise")]
async fn replace_pip_secret(
    conn: &Connection,
    w_id: &str,
    req: &str,
    worker_name: &str,
    job_id: &Uuid,
) -> error::Result<String> {
    if let Some(db) = conn.as_sql() {
        if PIP_SECRET_VARIABLE.is_match(req) {
            let mut joined = "".to_string();
            for req in req.lines() {
                let nreq = if PIP_SECRET_VARIABLE.is_match(req) {
                    let capture = PIP_SECRET_VARIABLE.captures(req);
                    let variable = capture.unwrap().get(1).unwrap().as_str();
                    if !variable.contains("/PIP_SECRET_") {
                        return Err(error::Error::internal_err(format!(
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
    } else {
        Ok(req.to_string())
    }
}

async fn handle_python_deps(
    job_dir: &str,
    requirements_o: Option<&String>,
    inner_content: &str,
    w_id: &str,
    script_path: &str,
    job_id: &Uuid,
    conn: &Connection,
    worker_name: &str,
    worker_dir: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    precomputed_agent_info: Option<PrecomputedAgentInfo>,
    annotations: PythonAnnotations,
) -> error::Result<(PyV, Vec<String>)> {
    create_dependencies_dir(job_dir).await;

    let mut additional_python_paths: Vec<String> = WORKER_CONFIG
        .read()
        .await
        .additional_python_paths
        .clone()
        .unwrap_or_else(|| vec![])
        .clone();

    let (pyv, resolved_lines) = match requirements_o {
        // Deployed
        Some(r) => {
            let rl = split_python_requirements(r);
            (PyV::parse_from_requirements(&rl), rl)
        }
        // Preview
        None => {
            let (v, requirements_lines, error_hint) = match conn {
                Connection::Sql(db) => {
                    let (mut version_specifiers, mut locked_v) = (vec![], None);
                    let (r, h) = Box::pin(windmill_parser_py_imports::parse_python_imports(
                        inner_content,
                        w_id,
                        script_path,
                        db,
                        &mut version_specifiers,
                        &mut locked_v,
                        &None,
                    ))
                    .await?;

                    let v = if let Some(v) = locked_v {
                        v.into()
                    } else {
                        PyV::resolve(
                            version_specifiers,
                            job_id,
                            w_id,
                            annotations.py_select_latest,
                            Some(conn.clone()),
                            None,
                            None,
                        )
                        .await?
                    };

                    (v, r, h)
                }

                Connection::Http(_) => match precomputed_agent_info {
                    Some(PrecomputedAgentInfo::Python {
                        requirements,
                        py_version,
                        py_version_v2,
                    }) => {
                        let v = {
                            let v_v2 = py_version_v2
                                .clone()
                                .and_then(|s| pep440_rs::Version::from_str(&s).ok().map(PyV::from));
                            let v_v1 = py_version.and_then(PyVAlias::try_from_v1).map(PyV::from);

                            match v_v2.or(v_v1) {
                                Some(v) => v,
                                None => {
                                    tracing::warn!(
                                        workspace_id = %w_id,
                                        "
Failed to get precomputed python version from server. Fallback to Default ({})
Returned from server: py_version - {:?}, py_version_v2 - {:?}
                                        ",
                                        *PyV::default(),
                                        py_version,
                                        py_version_v2
                                    );
                                    Default::default()
                                }
                            }
                        };

                        let r = split_python_requirements(requirements.unwrap_or_default());
                        let h = None;

                        (v, r, h)
                    }
                    _ => Default::default(),
                },
            };

            (
                v.clone(),
                if !requirements_lines.is_empty() {
                    uv_pip_compile(
                        job_id,
                        &requirements_lines.join("\n"),
                        mem_peak,
                        canceled_by,
                        job_dir,
                        conn,
                        worker_name,
                        w_id,
                        occupancy_metrics,
                        // annotated_pyv.unwrap_or(instance_pyv),
                        v,
                        annotations.no_cache,
                    )
                    .await
                    .map_err(|e| {
                        Error::ExecutionErr(format!(
                            "pip compile failed: {}{}",
                            e.to_string(),
                            error_hint.unwrap_or_default()
                        ))
                    })?
                    .lines()
                    .map(|s| s.to_owned())
                    .collect_vec()
                } else {
                    vec![]
                },
            )
        }
    };

    if !resolved_lines.is_empty() {
        let mut venv_path = handle_python_reqs(
            resolved_lines,
            job_id,
            w_id,
            mem_peak,
            canceled_by,
            conn,
            worker_name,
            job_dir,
            worker_dir,
            occupancy_metrics,
            pyv.clone(),
            None,
        )
        .await?;
        additional_python_paths.append(&mut venv_path);
    }

    Ok((pyv, additional_python_paths))
}

lazy_static::lazy_static! {
    static ref PIP_SECRET_VARIABLE: Regex = Regex::new(r"\$\{PIP_SECRET:([^\s\}]+)\}").unwrap();
}

/// Spawn process of uv install
/// Can be wrapped by nsjail depending on configuration
#[inline]
async fn spawn_uv_install(
    w_id: &str,
    req: &str,
    venv_p: &str,
    job_dir: &str,
    (pip_extra_index_url, pip_index_url): (Option<String>, Option<String>),
    // If none, it is system python
    py_path: Option<String>,
    worker_dir: &str,
) -> Result<Box<dyn TokioChildWrapper>, Error> {
    let uv_index_strategy_guard = UV_INDEX_STRATEGY.read().await.clone();
    let uv_index_strategy = uv_index_strategy_guard
        .as_deref()
        .unwrap_or("unsafe-best-match");

    if is_sandboxing_enabled() {
        tracing::info!(
            workspace_id = %w_id,
            "starting nsjail"
        );

        let mut vars = vec![("PATH", PATH_ENV.as_str())];
        if let Some(url) = pip_extra_index_url.as_ref() {
            vars.push(("EXTRA_INDEX_URL", url));
        }
        if let Some(url) = pip_index_url.as_ref() {
            vars.push(("INDEX_URL", url));
        }
        if let Some(cert_path) = INDEX_CERT.as_ref() {
            vars.push(("SSL_CERT_FILE", cert_path));
        }
        if let Some(host) = TRUSTED_HOST.as_ref() {
            vars.push(("TRUSTED_HOST", host));
        }
        if *NATIVE_CERT {
            vars.push(("UV_NATIVE_TLS", "true"));
        }

        let _owner;
        if let Some(py_path) = py_path.as_ref() {
            _owner = format!(
                "-p {} --python-preference only-managed",
                py_path.as_str() //
            );
            vars.push(("PY_PATH", &_owner));
        }
        vars.push(("REQ", &req));
        vars.push(("TARGET", venv_p));
        vars.push(("UV_INDEX_STRATEGY", uv_index_strategy));

        std::fs::create_dir_all(venv_p)?;
        let nsjail_proto = format!("{req}.config.proto");
        // Prepare NSJAIL
        let _ = write_file(
            job_dir,
            &nsjail_proto,
            NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT
                .replace("{WORKER_DIR}", worker_dir)
                .replace("{PY_INSTALL_DIR}", &PY_INSTALL_DIR)
                .replace("{TARGET_DIR}", &venv_p)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{TRACING_PROXY_CA_CERT_PATH}", TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL)
                .as_str(),
        )?;

        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(vars)
            .envs(PROXY_ENVS.clone())
            .args(vec!["--config", &nsjail_proto])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await
    } else {
        #[cfg(unix)]
        let req = req.to_owned();

        #[cfg(windows)]
        let req = format!("{}", req);

        let mut command_args = vec![
            UV_PATH.as_str(),
            "pip",
            "install",
            &req,
            "--no-deps",
            "--no-color",
            // Prevent uv from discovering configuration files.
            "--no-config",
            "--link-mode=copy",
            "--system",
            "--target",
            venv_p,
            "--no-cache",
            // If we invoke uv pip install, then we want to overwrite existing data
            "--reinstall",
        ];

        if let Some(py_path) = py_path.as_ref() {
            command_args.extend([
                "-p",
                py_path.as_str(),
                "--python-preference",
                "only-managed", //
            ]);
        } else {
            command_args.extend([
                "--python-preference",
                "only-system", //
            ]);
        }

        if let Some(url) = pip_extra_index_url.as_ref() {
            url.split(",").for_each(|url| {
                command_args.extend(["--extra-index-url", url]);
            });
        }

        let mut envs = vec![("PATH", PATH_ENV.as_str())];
        envs.push(("HOME", HOME_ENV.as_str()));
        envs.push(("UV_INDEX_STRATEGY", uv_index_strategy));

        if let Some(url) = pip_index_url.as_ref() {
            command_args.extend(["--index-url", url]);
        }
        if let Some(host) = TRUSTED_HOST.as_ref() {
            command_args.extend(["--trusted-host", &host]);
        }
        if *NATIVE_CERT {
            command_args.extend(["--native-tls"]);
        }
        // TODO:
        // Track https://github.com/astral-sh/uv/issues/6715
        if let Some(cert_path) = INDEX_CERT.as_ref() {
            // Once merged --cert can be used instead
            //
            // command_args.extend(["--cert", cert_path]);
            envs.push(("SSL_CERT_FILE", cert_path));
        }

        tracing::debug!("uv pip install command: {:?}", command_args);

        #[cfg(unix)]
        {
            let mut cmd = Command::new(command_args[0]);
            cmd.env_clear()
                .envs(PROXY_ENVS.clone())
                .envs(envs)
                .args(&command_args[1..])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            start_child_process(cmd, UV_PATH.as_str(), false).await
        }

        #[cfg(windows)]
        {
            let mut cmd: Command = Command::new("uv");
            cmd.env_clear()
                .envs(envs)
                .envs(PROXY_ENVS.clone())
                .env("SystemRoot", SYSTEM_ROOT.as_str())
                .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
                .env("HOME", HOME_ENV.as_str())
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                )
                .env(
                    "LOCALAPPDATA",
                    std::env::var("LOCALAPPDATA")
                        .unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str())),
                )
                .env(
                    "APPDATA",
                    std::env::var("APPDATA").unwrap_or_else(|_| {
                        format!("{}\\AppData\\Roaming", crate::USERPROFILE_ENV.as_str())
                    }),
                )
                .env(
                    "ComSpec",
                    std::env::var("ComSpec")
                        .unwrap_or_else(|_| String::from("C:\\Windows\\System32\\cmd.exe")),
                )
                .env(
                    "PATHEXT",
                    std::env::var("PATHEXT").unwrap_or_else(|_| {
                        String::from(".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC;.CPL")
                    }),
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
                .args(&command_args[1..])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            start_child_process(cmd, "uv", false).await
        }
    }
}

/// uv pip install, include cached or pull from S3
pub async fn handle_python_reqs(
    requirements: Vec<String>,
    job_id: &Uuid,
    w_id: &str,
    mem_peak: &mut i32,
    _canceled_by: &mut Option<CanceledBy>,
    conn: &Connection,
    _worker_name: &str,
    job_dir: &str,
    worker_dir: &str,
    _occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    py_version: PyV,
    reduced_concurrent_downloads: Option<usize>,
) -> error::Result<Vec<String>> {
    let worker_dir = worker_dir.to_string();

    let counter_arc = Arc::new(tokio::sync::Mutex::new(0));
    // Append logs with line like this:
    // [9/21]   +  requests==2.32.3            << (S3) |  in 57ms
    #[allow(unused_assignments)]
    async fn print_success(
        mut s3_pull: bool,
        mut s3_push: bool,
        job_id: &Uuid,
        w_id: &str,
        req: &str,
        req_tl: usize,
        counter_arc: Arc<tokio::sync::Mutex<usize>>,
        total_to_install: usize,
        instant: std::time::Instant,
        conn: &Connection,
    ) {
        #[cfg(not(all(feature = "enterprise", feature = "parquet", unix)))]
        {
            (s3_pull, s3_push) = (false, false);
        }

        #[cfg(all(feature = "enterprise", feature = "parquet", unix))]
        if OBJECT_STORE_SETTINGS.read().await.is_none() {
            (s3_pull, s3_push) = (false, false);
        }

        let mut counter = counter_arc.lock().await;
        *counter += 1;

        append_logs(
            job_id,
            w_id,
            format!(
                "\n{}+  {}{}{}|  in {}ms",
                pad_string(&format!("[{}/{total_to_install}]", counter), 9),
                // Because we want to align to max len [999/999] we take ^
                //                                     123456789
                pad_string(&req, req_tl + 1),
                // Margin to the right    ^
                if s3_pull { "<< (S3) " } else { "" },
                if s3_push { " > (S3) " } else { "" },
                instant.elapsed().as_millis(),
            ),
            conn,
        )
        .await;
        // Drop lock, so next print success can fire
    }

    // Parallelism level (N)
    let parallel_limit = reduced_concurrent_downloads
        .unwrap_or(*PY_CONCURRENT_DOWNLOADS)
        .clamp(1, 30);
    // Semaphore will panic if value less then 1

    tracing::info!(
        workspace_id = %w_id,
        // is_ok = out,
        "Parallel limit: {}, job: {}",
        parallel_limit,
        job_id
    );

    let pip_indexes = (
        read_ee_registry(
            PIP_EXTRA_INDEX_URL.read().await.clone(),
            "pip extra index url",
            job_id,
            w_id,
            conn,
        )
        .await
        .map(handle_ephemeral_token),
        read_ee_registry(
            PIP_INDEX_URL.read().await.clone(),
            "pip index url",
            job_id,
            w_id,
            conn,
        )
        .await
        .map(handle_ephemeral_token),
    );

    // Cached paths
    let mut req_with_penv: Vec<(String, String)> = vec![];
    // Requirements to pull (not cached)
    let mut req_paths: Vec<String> = vec![];
    // Find out if there is already cached dependencies
    // If so, skip them
    let mut in_cache = vec![];
    for req in &requirements {
        // Ignore python version annotation backed into lockfile
        if req.starts_with('#') || req.starts_with('-') || req.trim().is_empty() {
            continue;
        }
        let py_prefix = &py_version.to_cache_dir(false);

        let venv_p = format!(
            "{py_prefix}/{}",
            req.replace(' ', "").replace('/', "").replace(':', "")
        );
        if metadata(venv_p.clone() + "/.valid.windmill").await.is_ok() {
            req_paths.push(venv_p);
            in_cache.push(req.to_string());
        } else {
            // There is no valid or no wheel at all. Regardless of if there is content or not, we will overwrite it with --reinstall flag
            req_with_penv.push((req.to_string(), venv_p));
        }
    }
    if in_cache.len() > 0 {
        append_logs(
            &job_id,
            w_id,
            format!("\nenv deps from local cache: {}\n", in_cache.join(", ")),
            conn,
        )
        .await;
    }

    let (kill_tx, ..) = tokio::sync::broadcast::channel::<()>(1);
    let kill_rxs: Vec<tokio::sync::broadcast::Receiver<()>> = (0..req_with_penv.len())
        .map(|_| kill_tx.subscribe())
        .collect();

    //   _______ Read comments at the end of the function to get more context
    let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

    let job_id_2 = job_id.clone();
    let conn_2 = conn.clone();
    let w_id_2 = w_id.to_string();

    // Wheels to install
    let total_to_install = req_with_penv.len();
    let pids = Arc::new(tokio::sync::Mutex::new(vec![None; total_to_install]));
    let mem_peak_thread_safe = Arc::new(tokio::sync::Mutex::new(0));
    {
        let pids = pids.clone();
        let mem_peak_thread_safe = mem_peak_thread_safe.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                        let mut local_mem_peak = 0;
                        for pid_o in pids.lock().await.iter() {
                            if pid_o.is_some(){
                                let mem = crate::handle_child::get_mem_peak(*pid_o, is_sandboxing_enabled()).await;
                                if mem < 0 {
                                    tracing::warn!(
                                        workspace_id = %w_id_2,
                                        "Cannot get memory peak for pid: {:?}, job_id: {:?}, exit code: {mem}",
                                        pid_o,
                                        job_id_2
                                    );
                                } else {
                                    local_mem_peak += mem;
                                }
                            }
                        }

                        let mem_peak_actual = {
                            let mut mem_peak_lock = mem_peak_thread_safe.lock().await;

                            if local_mem_peak > *mem_peak_lock{
                                *mem_peak_lock = local_mem_peak;
                            } else {
                                tracing::debug!(
                                    workspace_id = %w_id_2,
                                    "Local mem_peak {:?}mb is smaller then global one {:?}mb, ignoring. job_id: {:?}",
                                    local_mem_peak / 1000,
                                    *mem_peak_lock / 1000,
                                    job_id_2
                                );

                            }
                            // Get the copy of value and drop lock itself, to release it as fast as possible
                            *mem_peak_lock
                        };


                        // Notify server that we are still alive
                        // Detect if job has been canceled
                        let canceled = match conn_2 {
                            Connection::Sql(ref db) => {
                            sqlx::query_scalar!(
                            "UPDATE v2_job_runtime r SET
                                memory_peak = $1,
                                ping = now()
                            FROM v2_job_queue q
                            WHERE r.id = $2 AND q.id = r.id
                            RETURNING canceled_by IS NOT NULL AS \"canceled!\"",
                            mem_peak_actual,
                            job_id_2
                            )
                            .fetch_optional(db)
                            .await
                            .unwrap_or_else(|e| {
                                tracing::error!(%e, "error updating job {job_id_2}: {e:#}");
                                Some(false)
                            })
                            .unwrap_or_else(|| {
                                // if the job is not in queue, it can only be in the completed_job so it is already complete
                                false
                                })
                            }
                            Connection::Http(_) => {
                                if let Err(e) = ping_job_status(&conn_2, &job_id_2, Some(mem_peak_actual), None).await {
                                    tracing::error!(%e, "error pinging job {job_id_2}: {e:#}");
                                }
                                false
                            }
                        };

                        if canceled {
                            tracing::info!(
                                // If there is listener on other side,
                                workspace_id = %w_id_2,
                                "cancelling installations",
                            );

                            if let Err(ref e) = kill_tx.send(()){
                                tracing::error!(
                                    // If there is listener on other side,
                                    workspace_id = %w_id_2,
                                    "failed to send done: Probably receiving end closed too early or have not opened yet\n{}",
                                    // If there is no listener, it will be dropped safely
                                    e
                                );
                            }
                        }
                    }
                    // Once done_tx is dropped, this will be fired
                    _ = done_rx.recv() => break
                }
            }
        });
    }

    // tl = total_length
    // "small".len == 5
    // "middle".len == 6
    // "largest".len == 7
    //  ==> req_tl = 7
    let mut req_tl = 0;
    if total_to_install > 0 {
        let mut logs = String::new();
        logs.push_str("\n\n--- UV PIP INSTALL ---\n");
        logs.push_str("\nTo be installed: \n\n");
        for (req, _) in &req_with_penv {
            if req.len() > req_tl {
                req_tl = req.len();
            }
            logs.push_str(&format!("{} \n", &req));
        }

        // Do we use Nsjail?
        if is_sandboxing_enabled() {
            logs.push_str(&format!(
                "\nStarting isolated installation... ({} tasks in parallel) \n",
                parallel_limit
            ));
        } else {
            logs.push_str(&format!(
                "\nStarting installation... ({} tasks in parallel) \n",
                parallel_limit
            ));
        }
        append_logs(&job_id, w_id, logs, conn).await;
    }

    let semaphore = Arc::new(Semaphore::new(parallel_limit));
    let mut handles = Vec::with_capacity(total_to_install);
    // let mem_peak_thread_safe = Arc::new(tokio::sync::Mutex::new(0));

    #[cfg(all(feature = "enterprise", feature = "parquet", unix))]
    let is_not_pro = !matches!(get_license_plan().await, LicensePlan::Pro);

    let total_time = std::time::Instant::now();
    let py_path = py_version
        .try_get_python(
            job_id,
            mem_peak,
            conn,
            _worker_name,
            w_id,
            _occupancy_metrics,
        )
        .await?;

    let has_work = req_with_penv.len() > 0;
    for ((i, (req, venv_p)), mut kill_rx) in
        req_with_penv.iter().enumerate().zip(kill_rxs.into_iter())
    {
        let permit = semaphore.clone().acquire_owned().await; // Acquire a permit

        if let Err(_) = permit {
            tracing::error!(
                workspace_id = %w_id,
                "Cannot acquire permit on semaphore, that can only mean that semaphore has been closed."
            );
            break;
        }

        let permit = permit.unwrap();

        tracing::info!(
            workspace_id = %w_id,
            "started setup python dependencies"
        );

        let conn = conn.clone();
        let job_id = job_id.clone();
        let job_dir = job_dir.to_owned();
        let w_id = w_id.to_owned();
        let req = req.clone();
        let venv_p = venv_p.clone();
        let counter_arc = counter_arc.clone();
        let pip_indexes = pip_indexes.clone();
        let py_path = py_path.clone();
        let pids = pids.clone();
        let worker_dir = worker_dir.clone();

        #[cfg(all(feature = "enterprise", feature = "parquet", unix))]
        let py_version = py_version.clone();

        handles.push(task::spawn(async move {
            // permit will be dropped anyway if this thread exits at any point
            // so we dont have to drop it manually
            // but we need to move permit into scope to take ownership
            let _permit = permit;

            tracing::info!(
                workspace_id = %w_id,
                job_id = %job_id,
                // is_ok = out,
                "started thread to install wheel {}",
                venv_p
            );

            let start = std::time::Instant::now();
            #[cfg(all(feature = "enterprise", feature = "parquet", unix))]
            if is_not_pro {
                if let Some(os) = windmill_object_store::get_object_store().await {
                    tokio::select! {
                        // Cancel was called on the job
                        _ = kill_rx.recv() => return Err(Error::from(anyhow::anyhow!("S3 pull was canceled"))),
                        pull = pull_from_tar(os, venv_p.clone(), py_version.to_cache_dir_top_level(false), None, false) => {
                            if let Err(e) = pull {
                                tracing::info!(
                                    workspace_id = %w_id,
                                    "No tarball was found for {venv_p} on S3 or different problem occurred {job_id}:\n{e}",
                                );
                            } else {
                                print_success(
                                    true,
                                    false,
                                    &job_id,
                                    &w_id,
                                    &req,
                                    req_tl,
                                    counter_arc,
                                    total_to_install,
                                    start,
                                    &conn
                                ).await;
                                pids.lock().await.get_mut(i).and_then(|e| e.take());

                                // Create a file to indicate that installation was successfull
                                let valid_path = venv_p.clone() + "/.valid.windmill";
                                // This is atomic operation, meaning, that it either completes and wheel is valid,
                                // or it does not and wheel is invalid and will be reinstalled next run
                                if let Err(e) = File::create(&valid_path).await{
                                    tracing::error!(
                                    workspace_id = %w_id,
                                    job_id = %job_id,
                                        "Failed to create {}!\n{e}\n
                                        This file needed for python jobs to function", valid_path)
                                };
                                return Ok(());
                            }
                        }
                    }
                }
            }

            let mut uv_install_proccess = match spawn_uv_install(
                &w_id,
                &req,
                &venv_p,
                &job_dir,
                pip_indexes,
                py_path,
                &worker_dir
            ).await {
                Ok(r) => r,
                Err(e) => {
                    append_logs(
                        &job_id,
                        w_id,
                        format!(
                            "\nError while spawning proccess:\n{e}",
                        ),
                        &conn,
                    )
                    .await;
                    pids.lock().await.get_mut(i).and_then(|e| e.take());
                    return Err(Error::from(e));
                }
            };

            let (mut stderr_buf, mut stdout_buf) = Default::default();
            let (mut stderr_pipe, mut stdout_pipe) = (
                uv_install_proccess
                    .stderr()
                    .take()
                    .ok_or(anyhow!("Cannot take stderr from uv_install_proccess"))?,
                uv_install_proccess
                    .stdout()
                    .take()
                    .ok_or(anyhow!("Cannot take stdout from uv_install_proccess"))?
            );
            let (stderr_future, stdout_future) = (
                stderr_pipe.read_to_string(&mut stderr_buf),
                stdout_pipe.read_to_string(&mut stdout_buf)
            );

            if let Some(pid) = pids.lock().await.get_mut(i) {
                *pid = uv_install_proccess.id();
                #[cfg(unix)]
                if let Err(e) = uv_install_proccess
                  .id()
                  .ok_or(Error::InternalErr(format!(
                    "failed to get PID for python installation process: {}",
                    &req
                  )))
                  .and_then(|pid| write_file(&format!("/proc/{pid}"), "oom_score_adj", "1000"))
                {
                  tracing::error!(
                      req = %req,
                      "Failed to create oom_score_adj for python dependency installation process: {e}"
                  );
                }
            } else {
                tracing::error!(
                    workspace_id = %w_id,
                    "Index out of range for uv pids",
                );
            }

            tokio::select! {
                // Canceled
                _ = kill_rx.recv() => {
                    Box::into_pin(uv_install_proccess.kill()).await?;
                    pids.lock().await.get_mut(i).and_then(|e| e.take());
                    return Err(Error::from(anyhow::anyhow!("uv pip install was canceled")));
                },
                (_, _, exitstatus) = async {
                    // See tokio::process::Child::wait_with_output() for more context
                    // Sometimes uv_install_proccess.wait() is not exiting if stderr is not awaited first
                    (stderr_future.await, stdout_future.await, Box::into_pin(uv_install_proccess.wait()).await)
                } => match exitstatus {
                    Ok(status) => if !status.success() {
                        #[cfg(unix)]
                        let code = status.signal();
                        #[cfg(not(unix))]
                        let code = status.code();

                        tracing::warn!(
                            workspace_id = %w_id,
                            "uv install {} did not succeed, exit status: {:?}",
                            &req,
                            code
                        );
                        append_logs(
                            &job_id,
                            w_id,
                            format!(
                                "\nError while installing {}: \nStderr:\n{stderr_buf}\nStdout:\n{stdout_buf}\nExit status: {:?}",
                                &req,
                                code
                            ),
                            &conn,
                        )
                        .await;
                        pids.lock().await.get_mut(i).and_then(|e| e.take());
                        return Err(Error::ExitStatus(stderr_buf, code.unwrap_or(1)));
                    },
                    Err(e) => {
                        tracing::error!(
                            workspace_id = %w_id,
                            "Cannot wait for uv_install_proccess, ExitStatus is Err: {e:?}",
                        );
                        pids.lock().await.get_mut(i).and_then(|e| e.take());
                        return Err(Error::from(e));
                    }
                }
            };

            #[cfg(all(feature = "enterprise", feature = "parquet", unix))]
            let s3_push = is_not_pro;

            #[cfg(not(all(feature = "enterprise", feature = "parquet", unix)))]
            let s3_push = false;

            if is_sandboxing_enabled() {
                let _ = std::fs::remove_file(format!("{job_dir}/{req}.config.proto"));
            }

            print_success(
                false,
                s3_push,
                &job_id,
                &w_id,
                &req,
                req_tl,
                counter_arc,
                total_to_install,
                start,
                &conn, //
            )
            .await;

            #[cfg(all(feature = "enterprise", feature = "parquet", unix))]
            if s3_push {
                // Send to upload channel for sequential processing
                let upload_task = PiptarUploadTask {
                    venv_path: venv_p.clone(),
                    cache_dir: py_version.to_cache_dir_top_level(false),
                };

                if let Err(e) = PIPTAR_UPLOAD_CHANNEL.send(upload_task) {
                    tracing::warn!("Failed to queue piptar upload for {venv_p}: {e}");
                } else {
                    tracing::info!("Queued piptar upload for {venv_p}");
                }
            }

            tracing::info!(
                workspace_id = %w_id,
                job_id = %job_id,
                // is_ok = out,
                "finished setting up python dependency {}",
                venv_p
            );

            pids.lock().await.get_mut(i).and_then(|e| e.take());
            // Create a file to indicate that installation was successfull
            let valid_path = venv_p.clone() + "/.valid.windmill";
            // This is atomic operation, meaning, that it either completes and wheel is valid,
            // or it does not and wheel is invalid and will be reinstalled next run
            if let Err(e) = File::create(&valid_path).await{
                tracing::error!(
                workspace_id = %w_id,
                job_id = %job_id,
                    "Failed to create {}!\n{e}\n
                    This file needed for python jobs to function", valid_path)
            };
            Ok(())
        }));
    }

    let (mut failed, mut oom_killed) = (false, false);
    for (handle, (_, venv_p)) in handles.into_iter().zip(req_with_penv.into_iter()) {
        if let Err(e) = handle
            .await
            .unwrap_or(Err(Error::from(anyhow!("Problem by joining handle"))))
        {
            failed = true;

            // OOM code is 9 or 137
            if matches!(e, Error::ExitStatus(_, 9 | 137)) {
                oom_killed = true;
            }

            append_logs(
                &job_id,
                w_id,
                format!("\nEnv installation failed: {:?}", e),
                conn,
            )
            .await;
            tracing::warn!(
                workspace_id = %w_id,
                "Env installation failed: {:?}",
                e
            );
        } else {
            req_paths.push(venv_p);
        }
    }

    if has_work {
        let total_time = total_time.elapsed().as_millis();
        append_logs(
            &job_id,
            w_id,
            format!("\nenv set in {}ms", total_time),
            conn,
        )
        .await;
    }

    *mem_peak = *mem_peak_thread_safe.lock().await;

    // Usually done_tx will drop after this return
    // If there is listener on other side,
    // it will be triggered
    // If there is no listener, it will be dropped safely
    return if failed {
        if cfg!(unix) && oom_killed && parallel_limit > 1 {
            // We want to drop it and stop monitor
            // new invocation will create another one
            drop(done_tx);

            let reduced_limit = parallel_limit / 2;

            append_logs(
                &job_id,
                w_id,
                format!(
                    "\n
    ======================
    ===== IMPORTANT! =====
    ======================

Some of installations have been killed by OOM,
restarting with reduced concurrency: {parallel_limit} -> {reduced_limit}

This is not normal behavior, please make sure all workers have enough memory.\n
"
                ),
                conn,
            )
            .await;

            // restart with half of concurrency
            Box::pin(handle_python_reqs(
                requirements,
                job_id,
                w_id,
                mem_peak,
                _canceled_by,
                conn,
                _worker_name,
                job_dir,
                &worker_dir,
                _occupancy_metrics,
                py_version,
                Some(reduced_limit),
            ))
            .await
        } else {
            Err(anyhow!("Env installation did not succeed, check logs").into())
        }
    } else {
        Ok(req_paths)
    };
}

// Returns code snippet that needs to be injected into wrapper to post-process results or leave unprocessed
fn get_result_postprocessor<'a>(skip: bool) -> &'a str {
    if skip {
        "unprocessed"
    } else {
        "re.sub(replace_invalid_fields, ' null ', unprocessed)"
    }
}

#[cfg(feature = "private")]
use crate::JobCompletedSender;
#[cfg(feature = "private")]
use crate::{common::build_envs_map, dedicated_worker_oss::handle_dedicated_process};
#[cfg(feature = "private")]
use windmill_common::variables;
#[cfg(feature = "private")]
use windmill_queue::DedicatedWorkerJob;

#[cfg(feature = "private")]
pub async fn start_worker(
    requirements_o: Option<&String>,
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
    jobs_rx: tokio::sync::mpsc::Receiver<DedicatedWorkerJob>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
    client: windmill_common::client::AuthedClient,
) -> error::Result<()> {
    use crate::PyV;
    tracing::info!("script path: {}", script_path);

    let mut mem_peak: i32 = 0;
    let mut canceled_by: Option<CanceledBy> = None;
    let context = variables::get_reserved_variables(
        &Connection::Sql(db.clone()),
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
        None,
        None,
    )
    .await
    .to_vec();

    let annotations = PythonAnnotations::parse(inner_content);
    let context_envs = build_envs_map(context).await;
    let (_, additional_python_paths) = handle_python_deps(
        job_dir,
        requirements_o,
        inner_content,
        w_id,
        script_path,
        &Uuid::nil(),
        &Connection::Sql(db.clone()),
        worker_name,
        job_dir,
        &mut mem_peak,
        &mut canceled_by,
        &mut None,
        None,
        annotations,
    )
    .await?;

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
    ) = prepare_wrapper(job_dir, None, None, None, inner_content, script_path).await?;

    {
        let postprocessor = get_result_postprocessor(annotations.skip_result_postprocessing);
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

replace_invalid_fields = re.compile(r'(?:\bNaN\b|\\u0000|Infinity|\-Infinity)')
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
        unprocessed = json.dumps(res, separators=(',', ':'), default=str).replace('\n', '')
        res_json = {postprocessor}
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
        &Connection::Sql(db.clone()),
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

    let py_version = if let Some(requirements) = requirements_o {
        PyV::parse_from_requirements(&split_python_requirements(requirements.as_str()))
    } else {
        tracing::warn!(workspace_id = %w_id, "lockfile is empty for dedicated worker, thus python version cannot be inferred. Fallback to 3.11");
        PyVAlias::default().into()
    };

    let python_path = py_version
        .get_python(
            worker_name,
            &Uuid::nil(),
            w_id,
            &mut mem_peak,
            &Connection::Sql(db.clone()),
            &mut None,
        )
        .await?;
    handle_dedicated_process(
        &python_path,
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
        client,
    )
    .await
}
