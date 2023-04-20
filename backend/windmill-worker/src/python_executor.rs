use std::process::Stdio;

use itertools::Itertools;
use regex::Regex;
use sqlx::{Pool, Postgres};
use tokio::{
    fs::{metadata, DirBuilder, File},
    io::AsyncReadExt,
    process::Command,
};
use uuid::Uuid;
use windmill_common::{
    error::{self, Error},
    jobs::QueuedJob,
    utils::calculate_hash,
};

lazy_static::lazy_static! {
    static ref PYTHON_PATH: String =
    std::env::var("PYTHON_PATH").unwrap_or_else(|_| "/usr/local/bin/python3".to_string());

    static ref PIP_INDEX_URL: Option<String> = std::env::var("PIP_INDEX_URL").ok();
    static ref PIP_EXTRA_INDEX_URL: Option<String> = std::env::var("PIP_EXTRA_INDEX_URL").ok();
    static ref PIP_TRUSTED_HOST: Option<String> = std::env::var("PIP_TRUSTED_HOST").ok();
    static ref PIP_LOCAL_DEPENDENCIES: Option<Vec<String>> = {
        let pip_local_dependencies = std::env::var("PIP_LOCAL_DEPENDENCIES")
            .ok()
            .map(|x| x.split(',').map(|x| x.to_string()).collect());
        if pip_local_dependencies == Some(vec!["".to_string()]) {
            None
        } else {
            pip_local_dependencies
        }
    };

    static ref ADDITIONAL_PYTHON_PATHS: Option<Vec<String>> = std::env::var("ADDITIONAL_PYTHON_PATHS")
        .ok()
        .map(|x| x.split(':').map(|x| x.to_string()).collect());

    static ref RELATIVE_IMPORT_REGEX: Regex = Regex::new(r#"(import|from)\s(((u|f)\.)|\.)"#).unwrap();

}

const NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT: &str = include_str!("../nsjail/download.py.config.proto");
const NSJAIL_CONFIG_RUN_PYTHON3_CONTENT: &str = include_str!("../nsjail/run.python3.config.proto");
const RELATIVE_PYTHON_LOADER: &str = include_str!("../loader.py");

use crate::{
    common::{read_result, set_logs},
    create_args_and_out_file, get_reserved_variables, handle_child, write_file,
    AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, NSJAIL_PATH, PATH_ENV,
    PIP_CACHE_DIR,
};

pub async fn create_dependencies_dir(job_dir: &str) {
    DirBuilder::new()
        .recursive(true)
        .create(&format!("{job_dir}/dependencies"))
        .await
        .expect("could not create dependencies dir");
}

pub async fn pip_compile(
    job_id: &Uuid,
    requirements: &str,
    logs: &mut String,
    job_dir: &str,
    db: &Pool<Postgres>,
    worker_name: &str,
    w_id: &str,
) -> error::Result<String> {
    logs.push_str(&format!("\nresolving dependencies..."));
    set_logs(logs, job_id, db).await;
    logs.push_str(&format!("\ncontent of requirements:\n{}", requirements));
    let req_hash = calculate_hash(&requirements);
    if let Some(cached) = sqlx::query_scalar!(
        "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
        req_hash
    )
    .fetch_optional(db)
    .await?
    {
        logs.push_str(&format!("\nfound cached resolution"));
        return Ok(cached);
    }
    let file = "requirements.in";
    let requirements = if let Some(pip_local_dependencies) = PIP_LOCAL_DEPENDENCIES.as_ref() {
        let deps = pip_local_dependencies.clone();
        requirements
            .lines()
            .filter(|s| !deps.contains(&s.to_string()))
            .join("\n")
    } else {
        requirements.to_string()
    };
    write_file(job_dir, file, &requirements).await?;

    let mut args = vec!["-q", "--no-header", file, "--resolver=backtracking"];
    if let Some(url) = PIP_EXTRA_INDEX_URL.as_ref() {
        args.extend(["--extra-index-url", url]);
    }
    if let Some(url) = PIP_INDEX_URL.as_ref() {
        args.extend(["--index-url", url]);
    }
    if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
        args.extend(["--trusted-host", host]);
    }
    let child = Command::new("pip-compile")
        .current_dir(job_dir)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    handle_child(
        job_id,
        db,
        logs,
        crate::worker::JobChild::ProcessChild(child),
        false,
        worker_name,
        &w_id,
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
    logs: &mut String,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    inner_content: &String,
    shared_mount: &str,
    base_internal_url: &str,
) -> windmill_common::error::Result<serde_json::Value> {
    create_dependencies_dir(job_dir).await;

    let mut additional_python_paths: Vec<String> =
        ADDITIONAL_PYTHON_PATHS.to_owned().unwrap_or_else(|| vec![]);

    let requirements = match requirements_o {
        Some(r) => r,
        None => {
            let requirements = windmill_parser_py::parse_python_imports(&inner_content)?.join("\n");
            if requirements.is_empty() {
                "".to_string()
            } else {
                pip_compile(
                    &job.id,
                    &requirements,
                    logs,
                    job_dir,
                    db,
                    worker_name,
                    &job.workspace_id,
                )
                .await
                .map_err(|e| {
                    Error::ExecutionErr(format!("pip compile failed: {}", e.to_string()))
                })?
            }
        }
    };

    if requirements.len() > 0 {
        if !*DISABLE_NSJAIL {
            let _ = write_file(
                job_dir,
                "download.config.proto",
                &NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT
                    .replace("{WORKER_DIR}", &worker_dir)
                    .replace("{CACHE_DIR}", PIP_CACHE_DIR)
                    .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string()),
            )
            .await?;
        }

        additional_python_paths = handle_python_reqs(
            requirements
                .split("\n")
                .filter(|x| !x.starts_with("--"))
                .collect(),
            &job.id,
            &job.workspace_id,
            logs,
            db,
            worker_name,
            job_dir,
        )
        .await?;
    }
    logs.push_str("\n\n--- PYTHON CODE EXECUTION ---\n");

    set_logs(logs, &job.id, db).await;

    let relative_imports = RELATIVE_IMPORT_REGEX.is_match(&inner_content);

    let script_path_splitted = &job.script_path().split("/");
    let dirs_full = script_path_splitted
        .clone()
        .take(script_path_splitted.clone().count() - 1)
        .join("/")
        .replace("-", "_");
    let dirs = if dirs_full.len() > 0 {
        dirs_full
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
        let _ = write_file(&job_dir, "loader.py", RELATIVE_PYTHON_LOADER).await?;
    }

    let sig = windmill_parser_py::parse_python_signature(inner_content)?;
    let transforms = sig
        .args
        .iter()
        .map(|x| match x.typ {
            windmill_parser::Typ::Bytes => {
                format!(
                    "if \"{}\" in kwargs and kwargs[\"{}\"] is not None:\n    \
                                     kwargs[\"{}\"] = base64.b64decode(kwargs[\"{}\"])\n",
                    x.name, x.name, x.name, x.name
                )
            }
            windmill_parser::Typ::Datetime => {
                format!(
                    "if \"{}\" in kwargs and kwargs[\"{}\"] is not None:\n    \
                                     kwargs[\"{}\"] = datetime.strptime(kwargs[\"{}\"], \
                                     '%Y-%m-%dT%H:%M')\n",
                    x.name, x.name, x.name, x.name
                )
            }
            _ => "".to_string(),
        })
        .collect::<Vec<String>>()
        .join("");
    let client = client.get_authed().await;
    create_args_and_out_file(&client, job, job_dir).await?;

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
            .map(|x| format!("args[\"{}\"] = kwargs.get(\"{}\")", x.name, x.name))
            .join("\n")
    };

    let module_dir_dot = dirs.replace("/", ".").replace("-", "_");
    let wrapper_content: String = format!(
        r#"
import json
{import_loader}
{import_base64}
{import_datetime}
import traceback
import sys
from {module_dir_dot} import {last} as inner_script


with open("args.json") as f:
    kwargs = json.load(f, strict=False)
args = {{}}
{spread}
{transforms}
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
    res_json = json.dumps(res, separators=(',', ':'), default=str).replace('\n', '')
    with open("result.json", 'w') as f:
        f.write(res_json)
except Exception as e:
    exc_type, exc_value, exc_traceback = sys.exc_info()
    tb = traceback.format_tb(exc_traceback)
    with open("result.json", 'w') as f:
        err_json = json.dumps({{ "message": str(e), "name": e.__class__.__name__, "stack": '\n'.join(tb[1:])  }}, separators=(',', ':'), default=str).replace('\n', '')
        f.write(err_json)
        sys.exit(1)
"#,
    );
    write_file(job_dir, "wrapper.py", &wrapper_content).await?;

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
        worker_name = %worker_name,
        job_id = %job.id,
        workspace_id = %job.workspace_id,
        "started python code execution {}",
        job.id
    );
    let child = if !*DISABLE_NSJAIL {
        Command::new(NSJAIL_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            // inject PYTHONPATH here - for some reason I had to do it in nsjail conf
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
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
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new(PYTHON_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["-u", "-m", "wrapper"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };

    handle_child(
        &job.id,
        db,
        logs,
        crate::worker::JobChild::ProcessChild(child),
        !*DISABLE_NSJAIL,
        worker_name,
        &job.workspace_id,
    )
    .await?;
    read_result(job_dir).await
}

pub async fn handle_python_reqs(
    requirements: Vec<&str>,
    job_id: &Uuid,
    w_id: &str,
    logs: &mut String,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    job_dir: &str,
) -> error::Result<Vec<String>> {
    let mut req_paths: Vec<String> = vec![];
    let mut vars = vec![("PATH", PATH_ENV.as_str())];
    if !*DISABLE_NSJAIL {
        if let Some(url) = PIP_EXTRA_INDEX_URL.as_ref() {
            vars.push(("EXTRA_INDEX_URL", url));
        }
        if let Some(url) = PIP_INDEX_URL.as_ref() {
            vars.push(("INDEX_URL", url));
        }
        if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
            vars.push(("TRUSTED_HOST", host));
        }
    };

    for req in requirements {
        // todo: handle many reqs
        let venv_p = format!("{PIP_CACHE_DIR}/{req}");
        if metadata(&venv_p).await.is_ok() {
            req_paths.push(venv_p);
            continue;
        }

        logs.push_str("\n--- PIP INSTALL ---\n");
        logs.push_str(&format!("\n{req} is being installed for the first time.\n It will be cached for all ulterior uses."));

        tracing::info!(
            worker_name = %worker_name,
            job_id = %job_id,
            workspace_id = %w_id,
            "started setup python dependencies"
        );

        let child = if !*DISABLE_NSJAIL {
            tracing::info!(
                worker_name = %worker_name,
                job_id = %job_id,
                workspace_id = %w_id,
                "starting nsjail"
            );
            let mut vars = vars.clone();
            let req = req.to_string();
            vars.push(("REQ", &req));
            vars.push(("TARGET", &venv_p));
            Command::new(NSJAIL_PATH.as_str())
                .current_dir(job_dir)
                .env_clear()
                .envs(vars)
                .args(vec!["--config", "download.config.proto"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        } else {
            let mut args = vec![
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
            if let Some(url) = PIP_EXTRA_INDEX_URL.as_ref() {
                args.extend(["--extra-index-url", url]);
            }
            if let Some(url) = PIP_INDEX_URL.as_ref() {
                args.extend(["--index-url", url]);
            }
            if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
                args.extend(["--trusted-host", &host]);
            }
            Command::new(PYTHON_PATH.as_str())
                .env_clear()
                .env("PATH", PATH_ENV.as_str())
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        };

        let child = handle_child(
            &job_id,
            db,
            logs,
            crate::worker::JobChild::ProcessChild(child),
            false,
            worker_name,
            &w_id,
        )
        .await;
        tracing::info!(
            worker_name = %worker_name,
            job_id = %job_id,
            workspace_id = %w_id,
            is_ok = child.is_ok(),
            "finished setting up python dependencies {}",
            job_id
        );
        child?;

        req_paths.push(venv_p);
    }
    Ok(req_paths)
}
