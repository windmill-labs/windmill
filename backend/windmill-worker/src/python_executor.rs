use std::{collections::HashMap, process::Stdio};

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
    jobs::QueuedJob,
    utils::calculate_hash,
    worker::WORKER_CONFIG,
    DB,
};

#[cfg(feature = "enterprise")]
use windmill_common::variables::get_secret_value_as_admin;

use windmill_queue::{append_logs, CanceledBy};

lazy_static::lazy_static! {
    static ref PYTHON_PATH: String =
    std::env::var("PYTHON_PATH").unwrap_or_else(|_| "/usr/local/bin/python3".to_string());

    static ref FLOCK_PATH: String =
    std::env::var("FLOCK_PATH").unwrap_or_else(|_| "/usr/bin/flock".to_string());
    static ref NON_ALPHANUM_CHAR: Regex = regex::Regex::new(r"[^0-9A-Za-z=.-]").unwrap();

    static ref PIP_TRUSTED_HOST: Option<String> = std::env::var("PIP_TRUSTED_HOST").ok();
    static ref PIP_INDEX_CERT: Option<String> = std::env::var("PIP_INDEX_CERT").ok();


    static ref RELATIVE_IMPORT_REGEX: Regex = Regex::new(r#"(import|from)\s(((u|f)\.)|\.)"#).unwrap();

    static ref EPHEMERAL_TOKEN_CMD: Option<String> = std::env::var("EPHEMERAL_TOKEN_CMD").ok();

}

const NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT: &str = include_str!("../nsjail/download.py.config.proto");
const NSJAIL_CONFIG_RUN_PYTHON3_CONTENT: &str = include_str!("../nsjail/run.python3.config.proto");
const RELATIVE_PYTHON_LOADER: &str = include_str!("../loader.py");

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use crate::global_cache::{build_tar_and_push, pull_from_tar};

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS;

use crate::{
    common::{
        create_args_and_out_file, get_main_override, get_reserved_variables, handle_child,
        read_result, start_child_process, write_file,
    },
    AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, HTTPS_PROXY, HTTP_PROXY,
    LOCK_CACHE_DIR, NO_PROXY, NSJAIL_PATH, PATH_ENV, PIP_CACHE_DIR, PIP_EXTRA_INDEX_URL,
    PIP_INDEX_URL, TZ_ENV,
};

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

pub async fn pip_compile(
    job_id: &Uuid,
    requirements: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &Pool<Postgres>,
    worker_name: &str,
    w_id: &str,
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

    let req_hash = format!("py-{}", calculate_hash(&requirements));
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
    let file = "requirements.in";

    write_file(job_dir, file, &requirements).await?;

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
    )
    .await
    .map_err(|e| Error::ExecutionErr(format!("Lock file generation failed: {e:?}")))?;
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
) -> windmill_common::error::Result<Box<RawValue>> {
    let script_path = crate::common::use_flow_root_path(job.script_path());
    let additional_python_paths = handle_python_deps(
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
    )
    .await?;

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
    ) = prepare_wrapper(job_dir, inner_content, &script_path, job.args.as_ref()).await?;

    create_args_and_out_file(&client, job, job_dir, db).await?;

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
{spread}
for k, v in list(args.items()):
    if v == '<function call>':
        del args[k]

def to_b_64(v: bytes):
    import base64
    b64 = base64.b64encode(v)
    return b64.decode('ascii')

replace_nan = re.compile(r'(?:\bNaN\b|\\u0000)')

result_json = os.path.join(os.path.abspath(os.path.dirname(__file__)), "result.json")

try:
    res = inner_script.{main_override}(**args)
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
    with open(result_json, 'w') as f:
        f.write(res_json)
except BaseException as e:
    exc_type, exc_value, exc_traceback = sys.exc_info()
    tb = traceback.format_tb(exc_traceback)
    with open(result_json, 'w') as f:
        err = {{ "message": str(e), "name": e.__class__.__name__, "stack": '\n'.join(tb[1:])  }}
        flow_node_id = os.environ.get('WM_FLOW_STEP_ID')
        if flow_node_id:
            err['step_id'] = flow_node_id
        err_json = json.dumps(err, separators=(',', ':'), default=str).replace('\n', '')
        f.write(err_json)
        sys.exit(1)
"#,
    );
    write_file(job_dir, "wrapper.py", &wrapper_content).await?;

    let client = client.get_authed().await;
    let mut reserved_variables = get_reserved_variables(job, &client.token, db).await?;
    let additional_python_paths_folders = additional_python_paths.iter().join(":");

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
        )
        .await?;
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
    )
    .await?;
    read_result(job_dir).await
}

async fn prepare_wrapper(
    job_dir: &str,
    inner_content: &str,
    script_path: &str,
    args: Option<&Json<HashMap<String, Box<RawValue>>>>,
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
)> {
    let main_override = get_main_override(args);

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

    let _ = write_file(&module_dir, &format!("{last}.py"), inner_content).await?;
    if relative_imports {
        let _ = write_file(job_dir, "loader.py", RELATIVE_PYTHON_LOADER).await?;
    }

    let sig = windmill_parser_py::parse_python_signature(inner_content, main_override.clone())?;
    let transforms = sig
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
    let import_base64 = if sig
        .args
        .iter()
        .any(|x| x.typ == windmill_parser::Typ::Bytes)
    {
        "import base64"
    } else {
        ""
    };
    let import_datetime = if sig
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
            .join("\n")
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
) -> error::Result<Vec<String>> {
    create_dependencies_dir(job_dir).await;

    let mut additional_python_paths: Vec<String> = WORKER_CONFIG
        .read()
        .await
        .additional_python_paths
        .clone()
        .unwrap_or_else(|| vec![])
        .clone();

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
                pip_compile(
                    job_id,
                    &requirements,
                    mem_peak,
                    canceled_by,
                    job_dir,
                    db,
                    worker_name,
                    w_id,
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
        )
        .await?;
        additional_python_paths.append(&mut venv_path);
    }
    Ok(additional_python_paths)
}

lazy_static::lazy_static! {
    static ref PIP_SECRET_VARIABLE: Regex = Regex::new(r"\$\{PIP_SECRET:([^\s\}]+)\}").unwrap();
}

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
) -> error::Result<Vec<String>> {
    let mut req_paths: Vec<String> = vec![];
    let mut vars = vec![("PATH", PATH_ENV.as_str())];
    let pip_extra_index_url;
    let pip_index_url;

    if !*DISABLE_NSJAIL {
        pip_extra_index_url = PIP_EXTRA_INDEX_URL
            .read()
            .await
            .clone()
            .map(handle_ephemeral_token);

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

        let _ = write_file(
            job_dir,
            "download.config.proto",
            &NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT
                .replace("{WORKER_DIR}", &worker_dir)
                .replace("{CACHE_DIR}", PIP_CACHE_DIR)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string()),
        )
        .await?;
    };

    let mut req_with_penv: Vec<(String, String)> = vec![];

    for req in requirements {
        let venv_p = format!(
            "{PIP_CACHE_DIR}/{}",
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
            let futures = req_with_penv
                .clone()
                .into_iter()
                .map(|(req, venv_p)| {
                    let os = os.clone();
                    async move {
                        if pull_from_tar(os, venv_p.clone()).await.is_ok() {
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
            let req = format!("'{}'", req);
            let mut command_args = vec![
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
            ];
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
                tokio::spawn(build_tar_and_push(os, venv_p));
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
    ) = prepare_wrapper(job_dir, inner_content, script_path, _args.as_ref()).await?;

    {
        let indented_transforms = transforms
            .lines()
            .map(|x| format!("    {}", x))
            .collect::<Vec<String>>()
            .join("\n");
        let indented_spread = spread
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
{indented_spread}
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
        write_file(job_dir, "wrapper.py", &wrapper_content).await?;
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
