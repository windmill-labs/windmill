use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    process::Stdio,
    sync::Arc
};

use anyhow::anyhow;
use itertools::Itertools;
use regex::Regex;
use serde_json::value::RawValue;
use sqlx::{types::Json, Pool, Postgres};
use tokio::{
    fs::{metadata, DirBuilder, File},
    io::AsyncReadExt,
    process::Command,
    sync::Semaphore,
    task,
};
use uuid::Uuid;
#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
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
    static ref PYTHON_PATH: String =
    std::env::var("PYTHON_PATH").unwrap_or_else(|_| "/usr/local/bin/python3".to_string());

    static ref UV_PATH: String =
    std::env::var("UV_PATH").unwrap_or_else(|_| "/usr/local/bin/uv".to_string());

    static ref PY_CONCURRENT_DOWNLOADS: usize =
    std::env::var("PY_CONCURRENT_DOWNLOADS").ok().map(|flag| flag.parse().unwrap_or(20)).unwrap_or(20);

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


    static ref RELATIVE_IMPORT_REGEX: Regex = Regex::new(r#"(import|from)\s(((u|f)\.)|\.)"#).unwrap();

    static ref EPHEMERAL_TOKEN_CMD: Option<String> = std::env::var("EPHEMERAL_TOKEN_CMD").ok();

}

const NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT: &str = include_str!("../nsjail/download.py.config.proto");
const NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT_FALLBACK: &str =
    include_str!("../nsjail/download.py.pip.config.proto");
const NSJAIL_CONFIG_RUN_PYTHON3_CONTENT: &str = include_str!("../nsjail/run.python3.config.proto");
const RELATIVE_PYTHON_LOADER: &str = include_str!("../loader.py");

#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
use crate::global_cache::{build_tar_and_push, pull_from_tar};

#[cfg(all(feature = "enterprise", feature = "parquet", unix))]
use windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS;

use crate::{
    common::{
        create_args_and_out_file, get_main_override, get_reserved_variables, read_file,
        read_result, start_child_process, OccupancyMetrics,
    },
    handle_child::handle_child,
    AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, LOCK_CACHE_DIR,
    NSJAIL_PATH, PATH_ENV, PIP_CACHE_DIR, PIP_EXTRA_INDEX_URL, PIP_INDEX_URL, PROXY_ENVS,
    PY311_CACHE_DIR, TZ_ENV, UV_CACHE_DIR,
};

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
    // Fallback to pip-compile. Will be removed in future
    mut no_uv: bool,
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
        if let Some(cached) = sqlx::query_scalar!(
            "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
            req_hash
        )
        .fetch_optional(db)
        .await?
        {
            logs.push_str(&format!("\nfound cached resolution: {req_hash}"));
            return Ok(cached);
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
            url.split(",").for_each(|url| {
                args.extend(["--extra-index-url", url]);
                pip_args.push(format!("--extra-index-url {}", url));
            });
            args.push("--no-emit-index-url");
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
            // We dont want UV to manage python installations
            "--python-preference",
            "only-system",
            "--no-python-downloads",
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
            url.split(",").for_each(|url| {
                args.extend(["--extra-index-url", url]);
            });
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
        let child_process = start_child_process(child_cmd, uv_cmd).await?;
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
    let lockfile = req_content
        .lines()
        .filter(|x| !x.trim_start().starts_with('#'))
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join("\n");
    sqlx::query!(
        "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('3 days')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = $2",
        req_hash,
        lockfile
    ).fetch_optional(db).await?;
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
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
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

                if name == "bin" || name.contains("dist-info") {
                    continue;
                }

                if let Some(existing_paths) = lookup_table.get_mut(&name) {
                    tracing::info!(
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
            db,
        )
        .await;
        // Remove PATHs we just moved
        additional_python_paths.retain(|e| !paths_to_remove.contains(e));
        // Instead add shared path
        additional_python_paths.insert(0, format!("{job_dir}/site-packages"));
    }
    Ok(())
}

fn copy_dir_recursively(src: &Path, dst: &Path) -> windmill_common::error::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursively(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
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
    let mut additional_python_paths = handle_python_deps(
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

    if !PythonAnnotations::parse(inner_content).no_postinstall {
        if let Err(e) = postinstall(&mut additional_python_paths, job_dir, job, db).await {
            tracing::error!("Postinstall stage has failed. Reason: {e}");
        }
    }

    append_logs(
        &job.id,
        &job.workspace_id,
        "\n\n--- PYTHON CODE EXECUTION ---\n".to_string(),
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
    let child = if !*DISABLE_NSJAIL {
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            // inject PYTHONPATH here - for some reason I had to do it in nsjail conf
            .envs(reserved_variables)
            .envs(PROXY_ENVS.clone())
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("BASE_URL", base_internal_url)
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                PYTHON_PATH.as_str(),
                "-u",
                "-m",
                "wrapper",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
    } else {
        let mut python_cmd = Command::new(PYTHON_PATH.as_str());
        python_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .args(vec!["-u", "-m", "wrapper"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        python_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

        start_child_process(python_cmd, PYTHON_PATH.as_str()).await?
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
) -> error::Result<Vec<String>> {
    create_dependencies_dir(job_dir).await;

    let mut additional_python_paths: Vec<String> = WORKER_CONFIG
        .read()
        .await
        .additional_python_paths
        .clone()
        .unwrap_or_else(|| vec![])
        .clone();

    let annotations = windmill_common::worker::PythonAnnotations::parse(inner_content);
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

    if requirements.len() > 0 {
        let mut venv_path = handle_python_reqs(
            requirements
                .split("\n")
                .filter(|x| !x.starts_with("--"))
                .collect(),
            job_id,
            w_id,
            mem_peak,
            canceled_by,
            db,
            worker_name,
            job_dir,
            worker_dir,
            occupancy_metrics,
            annotations.no_uv || annotations.no_uv_install,
            false,
        )
        .await?;
        additional_python_paths.append(&mut venv_path);
    }
    Ok(additional_python_paths)
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
    no_uv_install: bool,
) -> Result<tokio::process::Child, Error> {
    if !*DISABLE_NSJAIL {
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
        if let Some(cert_path) = PIP_INDEX_CERT.as_ref() {
            vars.push(("PIP_INDEX_CERT", cert_path));
        }
        if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
            vars.push(("TRUSTED_HOST", host));
        }

        vars.push(("REQ", &req));
        vars.push(("TARGET", venv_p));

        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(vars)
            .envs(PROXY_ENVS.clone())
            .args(vec!["--config", "download.config.proto"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await
    } else {
        let fssafe_req = NON_ALPHANUM_CHAR.replace_all(&req, "_").to_string();
        #[cfg(unix)]
        let req = if no_uv_install {
            format!("'{}'", req)
        } else {
            req.to_owned()
        };

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
                venv_p,
            ]
        } else {
            vec![
                UV_PATH.as_str(),
                "pip",
                "install",
                &req,
                "--no-deps",
                "--no-color",
                // "-p",
                // "3.11",
                // Prevent uv from discovering configuration files.
                "--no-config",
                "--link-mode=copy",
                "--system",
                // Prefer main index over extra
                // https://docs.astral.sh/uv/pip/compatibility/#packages-that-exist-on-multiple-indexes
                // TODO: Use env variable that can be toggled from UI
                "--index-strategy",
                "unsafe-best-match",
                "--target",
                venv_p,
                "--no-cache",
                "-q",
            ]
        };

        if let Some(url) = pip_extra_index_url.as_ref() {
            url.split(",").for_each(|url| {
                command_args.extend(["--extra-index-url", url]);
            });
        }

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
        envs.push(("HOME", HOME_ENV.as_str()));

        tracing::debug!("uv pip install command: {:?}", command_args);

        #[cfg(unix)]
        {
            if no_uv_install {
              let mut flock_cmd = Command::new(FLOCK_PATH.as_str());
              flock_cmd
                  .env_clear()
                  .envs(PROXY_ENVS.clone())
                  .envs(envs)
                  .args([
                      "-x",
                      &format!(
                          "{}/{}-{}.lock",
                          LOCK_CACHE_DIR,
                          if no_uv_install { "pip" } else { "py311" },
                          fssafe_req
                      ),
                      "--command",
                      &command_args.join(" "),
                  ])
                  .stdout(Stdio::piped())
                  .stderr(Stdio::piped());
              start_child_process(flock_cmd, FLOCK_PATH.as_str()).await
            } else {
                let mut cmd = Command::new(command_args[0]);
                cmd
                    .env_clear()
                    .envs(PROXY_ENVS.clone())
                    .envs(envs)
                    .args(&command_args[1..])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                start_child_process(cmd, UV_PATH.as_str()).await
            }
        }

        #[cfg(windows)]
        {
            let installer_path = if no_uv_install { command_args[0] } else { "uv" };
            let mut cmd: Command = Command::new(&installer_path);
            cmd.env_clear()
                .envs(envs)
                .envs(PROXY_ENVS.clone())
                .env("SystemRoot", SYSTEM_ROOT.as_str())
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                )
                .args(&command_args[1..])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            start_child_process(cmd, installer_path).await
        }
    }
}

/// length = 5
/// value  = "foo"
/// output = "foo  "
///           12345
fn pad_string(value: &str, total_length: usize) -> String {
    if value.len() >= total_length {
        value.to_string() // Return the original string if it's already long enough
    } else {
        let padding_needed = total_length - value.len();
        format!("{value}{}", " ".repeat(padding_needed)) // Pad with spaces
    }
}

/// pip install, include cached or pull from S3
pub async fn handle_python_reqs(
    requirements: Vec<&str>,
    job_id: &Uuid,
    w_id: &str,
    _mem_peak: &mut i32,
    _canceled_by: &mut Option<CanceledBy>,
    db: &sqlx::Pool<sqlx::Postgres>,
    _worker_name: &str,
    job_dir: &str,
    worker_dir: &str,
    _occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    // TODO: Remove (Deprecated)
    mut no_uv_install: bool,
    is_ansible: bool,
) -> error::Result<Vec<String>> {

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
        db: Pool<Postgres>,
    ) {
        #[cfg(not(all(feature = "enterprise", feature = "parquet", unix)))]
        {
            (s3_pull, s3_push) = (false, false);
        }

        #[cfg(all(feature = "enterprise", feature = "parquet", unix))]
        if OBJECT_STORE_CACHE_SETTINGS.read().await.is_none() {
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
            db,
        )
        .await;
        // Drop lock, so next print success can fire
    }
    no_uv_install |= *USE_PIP_INSTALL;

    if no_uv_install && !is_ansible {
        append_logs(&job_id, w_id, "\nFallback to pip (Deprecated!)\n", db).await;
        tracing::warn!("Fallback to pip");
    }
    // Parallelism level (N)
    let parallel_limit = if no_uv_install {
        1
    } else {
        // Semaphore will panic if value less then 1
        PY_CONCURRENT_DOWNLOADS.clamp(1, 30)
    };

    tracing::info!(
        workspace_id = %w_id,
        // is_ok = out,
        "Parallel limit: {}, job: {}",
        parallel_limit,
        job_id
    );

    let pip_indexes = (
        PIP_EXTRA_INDEX_URL
            .read()
            .await
            .clone()
            .map(handle_ephemeral_token),
        PIP_INDEX_URL
            .read()
            .await
            .clone()
            .map(handle_ephemeral_token),
    );


    // Prepare NSJAIL
    if !*DISABLE_NSJAIL {
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

    // Cached paths
    let mut req_with_penv: Vec<(String, String)> = vec![];
    // Requirements to pull (not cached)
    let mut req_paths: Vec<String> = vec![];
    // Find out if there is already cached dependencies
    // If so, skip them
    for req in requirements {
        // Ignore python version annotation backed into lockfile
        if req.starts_with('#') {
            continue;
        }
        // TODO: Remove
        let py_prefix = if no_uv_install {
            PIP_CACHE_DIR
        } else {
            PY311_CACHE_DIR
        };

        let venv_p = format!(
            "{py_prefix}/{}",
            req.replace(' ', "").replace('/', "").replace(':', "")
        );
        if metadata(&venv_p).await.is_ok() {
            // If dir exists skip installation and push path to output
            req_paths.push(venv_p);
        } else {
            req_with_penv.push((req.to_string(), venv_p));
        }
    }

    let (kill_tx, ..) = tokio::sync::broadcast::channel::<()>(1);
    let kill_rxs: Vec<tokio::sync::broadcast::Receiver<()>> = 
        (0..req_with_penv.len()).map(|_| kill_tx.subscribe()).collect();

    //   ________ Read comments at the end of the function to get more context
    let (_done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

    let job_id_2 = job_id.clone();
    let db_2 = db.clone();
    let w_id_2 = w_id.to_string();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                    // Notify server that we are still alive
                    // Detect if job has been canceled
                    let canceled =
                        sqlx::query_scalar::<_, bool>
                        (r#"

                               UPDATE queue 
                                  SET last_ping = now() 
                                WHERE id = $1 
                            RETURNING canceled

                            "#)
                        .bind(job_id_2)
                        .fetch_optional(&db_2)
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!(%e, "error updating job {job_id_2}: {e:#}");
                            Some(false)
                        })
                        .unwrap_or_else(|| {
                            // if the job is not in queue, it can only be in the completed_job so it is already complete
                            false
                        });

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

    // tl = total_length
    // "small".len == 5
    // "middle".len == 6
    // "largest".len == 7
    //  ==> req_tl = 7
    let mut req_tl = 0;
    // Wheels to install
    let total_to_install = req_with_penv.len();
    if total_to_install > 0 {
        let mut logs = String::new();
        // Do we use UV?
        if no_uv_install {
            logs.push_str("\n\n--- PIP INSTALL ---\n");
        } else {
            logs.push_str("\n\n--- UV PIP INSTALL ---\n");
        }

        logs.push_str("\nTo be installed: \n\n");
        for (req, _) in &req_with_penv {
            if req.len() > req_tl {
                req_tl = req.len();
            }
            logs.push_str(&format!("{} \n", &req));
        }

        // Do we use Nsjail?
        if !*DISABLE_NSJAIL {
            logs.push_str(&format!("\nStarting isolated installation... ({} tasks in parallel ) \n", parallel_limit));
        } else {
            logs.push_str(&format!("\nStarting installation... ({} tasks in parallel) \n", parallel_limit));
        }
        append_logs(&job_id, w_id, logs, db).await;
    }

    let semaphore = Arc::new(Semaphore::new(parallel_limit));
    let mut handles = Vec::with_capacity(total_to_install);

    #[cfg(all(feature = "enterprise", feature = "parquet", unix))]
    let is_not_pro =  matches!(get_license_plan().await, LicensePlan::Pro);

    for ((req, venv_p), mut kill_rx) in req_with_penv.iter().zip(kill_rxs.into_iter()) {
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

        let db = db.clone();
        let job_id = job_id.clone();
        let job_dir = job_dir.to_owned();
        let w_id = w_id.to_owned();
        let req = req.clone();
        let venv_p = venv_p.clone();
        let counter_arc = counter_arc.clone();
        let pip_indexes = pip_indexes.clone();

        handles.push(task::spawn(async move {
            // permit will be dropped anyway if this thread exits at any point
            // so we dont have to drop it manually
            // but we need to move permit into scope to take ownership
            let _permit = permit;

            tracing::info!(
                workspace_id = %w_id,
                // is_ok = out,
                "started thread to install wheel {}",
                job_id
            );

            let start = std::time::Instant::now();
            #[cfg(all(feature = "enterprise", feature = "parquet", unix))]
            if is_not_pro {
                if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
                    tokio::select! {
                        // Cancel was called on the job
                        _ = kill_rx.recv() => return Err(anyhow::anyhow!("S3 pull was canceled")),
                        
                        pull = pull_from_tar(os, venv_p.clone(), no_uv_install) => {
                            if let Err(e) = pull {
                                tracing::info!(
                                    workspace_id = %w_id,
                                    "No tarball was found on S3 or different problem occured {job_id}:\n{e}",
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
                                    db
                                ).await;
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
                no_uv_install,
            ).await {
                Ok(r) => r,
                Err(e) => { 
                    append_logs(
                        &job_id,
                        w_id,
                        format!(
                            "\nError while spawning proccess:\n{e}", 
                        ),
                        db,
                    )
                    .await;
                    return Err(e.into());
                }
            };

            let mut stderr = uv_install_proccess
                .stderr
                .take()
                .ok_or(anyhow!("Cannot take stderr from uv_install_proccess"))?;

            tokio::select! {
                // Canceled
                _ = kill_rx.recv() => {
                    uv_install_proccess.kill().await?;
                    return Err(anyhow::anyhow!("uv pip install was canceled"));
                }
                // Finished
                exitstatus = uv_install_proccess.wait() => match exitstatus {
                    Ok(status) => if !status.success() {
                        tracing::warn!(
                            workspace_id = %w_id,
                            "uv install {} did not succeed, exit status: {:?}",
                            &req,
                            status.code()
                        );

                        let mut buf = String::new();
                        stderr.read_to_string(&mut buf).await.unwrap_or_else(|_|{
                            buf = "Cannot read stderr to string".to_owned();
                            0
                        });

                        append_logs(
                            &job_id,
                            w_id,
                            format!(
                                "\nError while installing {}:\n{buf}", 
                                &req
                            ),
                            db,
                        )
                        .await;
                        return Err(anyhow!(buf));
                    },
                    Err(e) => {
                        tracing::error!(
                            workspace_id = %w_id,
                            "Cannot wait for uv_install_proccess, ExitStatus is Err: {e:?}",
                        );
                        return Err(e.into());
                    }
                }
            };

            print_success(
                false,
                true,
                &job_id,
                &w_id,
                &req,
                req_tl,
                counter_arc,
                total_to_install,
                start,
                db, //
            )
            .await;

            #[cfg(all(feature = "enterprise", feature = "parquet", unix))]
            if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
                if !is_not_pro {
                    tracing::warn!("S3 cache not available in the pro plan");
                } else {
                    tokio::spawn(build_tar_and_push(os, venv_p.clone(), no_uv_install));
                }
            }

            tracing::info!(
                workspace_id = %w_id,
                // is_ok = out,
                "finished setting up python dependency {}",
                job_id
            );

            Ok(())
        }));
    }

    let mut failed = false;
    for (handle, (_, venv_p)) in handles.into_iter().zip(req_with_penv.into_iter()) {
        if let Err(e) = handle.await.unwrap_or(Err(anyhow!("Problem by joining handle"))) {
            failed = true;
            tracing::warn!(
                workspace_id = %w_id,
                "Installation failed: {:?}",
                e
            );
            if let Err(e) = fs::remove_dir_all(&venv_p) {
                tracing::warn!(
                    workspace_id = %w_id,
                    "Failed to remove cache dir: {:?}",
                    e
                );
            }
        } else {
            req_paths.push(venv_p);
        }
    }

    // Usually done_tx will drop after this return
    // If there is listener on other side, 
    // it will be triggered
    // If there is no listener, it will be dropped safely
    return if failed {
        Err(anyhow!("Installation did not succeed, check logs").into())
    } else {
        Ok(req_paths)
    };
}

#[cfg(feature = "enterprise")]
use crate::JobCompletedSender;
#[cfg(feature = "enterprise")]
use crate::{common::build_envs_map, dedicated_worker::handle_dedicated_process};
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
    jobs_rx: tokio::sync::mpsc::Receiver<std::sync::Arc<QueuedJob>>,
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
    let additional_python_paths = handle_python_deps(
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
