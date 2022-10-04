/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use itertools::Itertools;
use std::{
    borrow::Borrow,
    collections::HashMap,
    io, panic,
    process::{ExitStatus, Stdio},
    time::Duration,
};
use uuid::Uuid;

use crate::{
    db::DB,
    error::{self, Error},
    jobs::{
        add_completed_job, add_completed_job_error, get_queued_job, postprocess_queued_job, pull,
        JobKind, QueuedJob,
    },
    parser::Typ,
    parser_py,
    scripts::{ScriptHash, ScriptLang},
    users::{create_token_for_owner, get_email_from_username},
    variables,
    worker_flow::{
        handle_flow, update_flow_status_after_job_completion, update_flow_status_in_progress,
    },
};

use serde_json::{json, Map, Value};

use tokio::{
    fs::{DirBuilder, File},
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::watch,
    time::{interval, sleep, Instant, MissedTickBehavior},
};

use futures::{
    future::{self, ready, FutureExt},
    stream::{self, StreamExt},
};

use async_recursion::async_recursion;

const TMP_DIR: &str = "/tmp/windmill";
const PIP_CACHE_DIR: &str = "/tmp/windmill/cache/pip";
const DENO_CACHE_DIR: &str = "/tmp/windmill/cache/deno";
const GO_CACHE_DIR: &str = "/tmp/windmill/cache/go";
const NUM_SECS_ENV_CHECK: u64 = 15;

const INCLUDE_DEPS_PY_SH_CONTENT: &str = include_str!("../../nsjail/download_deps.py.sh");
const NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT: &str =
    include_str!("../../nsjail/download.py.config.proto");
const NSJAIL_CONFIG_RUN_PYTHON3_CONTENT: &str =
    include_str!("../../nsjail/run.python3.config.proto");

const NSJAIL_CONFIG_RUN_GO_CONTENT: &str = include_str!("../../nsjail/run.go.config.proto");

const NSJAIL_CONFIG_RUN_DENO_CONTENT: &str = include_str!("../../nsjail/run.deno.config.proto");
const MAX_LOG_SIZE: u32 = 200000;
const GO_REQ_SPLITTER: &str = "//go.sum";
pub struct Metrics {
    pub jobs_failed: prometheus::IntCounter,
}

#[derive(Clone)]
pub struct WorkerConfig {
    pub base_internal_url: String,
    pub base_url: String,
    pub disable_nuser: bool,
    pub disable_nsjail: bool,
    pub keep_job_dir: bool,
}

pub async fn run_worker(
    db: &DB,
    timeout: i32,
    worker_instance: &str,
    worker_name: String,
    i_worker: u64,
    num_workers: u64,
    ip: &str,
    sleep_queue: u64,
    worker_config: WorkerConfig,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) {
    let worker_dir = format!("{TMP_DIR}/{worker_name}");
    tracing::debug!(worker_dir = %worker_dir, worker_name = %worker_name, "Creating worker dir");

    for x in [&worker_dir, PIP_CACHE_DIR, DENO_CACHE_DIR, GO_CACHE_DIR] {
        DirBuilder::new()
            .recursive(true)
            .create(x)
            .await
            .expect("could not create initial worker dir");
    }

    let _ = write_file(
        &worker_dir,
        "download_deps.py.sh",
        INCLUDE_DEPS_PY_SH_CONTENT,
    )
    .await;

    let mut last_ping = Instant::now() - Duration::from_secs(NUM_SECS_ENV_CHECK + 1);

    insert_initial_ping(worker_instance, &worker_name, ip, db).await;

    prometheus::register_int_gauge!(prometheus::Opts::new(
        "start_time_seconds",
        "Start time of worker as seconds since unix epoch",
    )
    .const_label("name", &worker_name))
    .expect("register prometheus metric")
    .set(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0),
    );

    let job_duration_seconds = prometheus::register_histogram_vec!(
        prometheus::HistogramOpts::new(
            "job_duration_seconds",
            "Duration between receiving a job and completing it",
        )
        .const_label("name", &worker_name),
        &["workspace_id", "language"],
    )
    .expect("register prometheus metric");

    let jobs_failed = prometheus::register_int_counter_vec!(
        prometheus::Opts::new("jobs_failed", "Number of failed jobs",)
            .const_label("name", &worker_name),
        &["workspace_id", "language"],
    )
    .expect("register prometheus metric");

    let mut jobs_executed = 0;

    let deno_path = std::env::var("DENO_PATH").unwrap_or_else(|_| "/usr/bin/deno".to_string());
    let go_path = std::env::var("GO_PATH").unwrap_or_else(|_| "/usr/bin/go".to_string());
    let python_path =
        std::env::var("PYTHON_PATH").unwrap_or_else(|_| "/usr/local/bin/python3".to_string());
    let nsjail_path = std::env::var("NSJAIL_PATH").unwrap_or_else(|_| "nsjail".to_string());
    let path_env = std::env::var("PATH").unwrap_or_else(|_| String::new());
    let gopath_env = std::env::var("GOPATH").unwrap_or_else(|_| String::new());
    let home_env = std::env::var("HOME").unwrap_or_else(|_| String::new());
    let pip_index_url = std::env::var("PIP_INDEX_URL").ok();
    let pip_extra_index_url = std::env::var("PIP_EXTRA_INDEX_URL").ok();
    let pip_trusted_host = std::env::var("PIP_TRUSTED_HOST").ok();
    let envs = Envs {
        deno_path,
        go_path,
        python_path,
        nsjail_path,
        path_env,
        gopath_env,
        home_env,
        pip_index_url,
        pip_extra_index_url,
        pip_trusted_host,
    };

    loop {
        if last_ping.elapsed().as_secs() > NUM_SECS_ENV_CHECK {
            sqlx::query!(
                "UPDATE worker_ping SET ping_at = now(), jobs_executed = $1 WHERE worker = $2",
                jobs_executed,
                &worker_name
            )
            .execute(db)
            .await
            .expect("update worker ping");

            last_ping = Instant::now();
        }

        match pull(db).await {
            Ok(Some(job)) => {
                let label_values = [
                    &job.workspace_id,
                    job.language.as_ref().map(|l| l.as_str()).unwrap_or(""),
                ];

                let _timer = job_duration_seconds
                    .with_label_values(label_values.as_slice())
                    .start_timer();

                jobs_executed += 1;

                let metrics =
                    Metrics { jobs_failed: jobs_failed.with_label_values(label_values.as_slice()) };

                tracing::info!(worker = %worker_name, id = %job.id, "fetched job {}", job.id);

                if let Some(err) = handle_queued_job(
                    job.clone(),
                    db,
                    timeout,
                    &worker_name,
                    &worker_dir,
                    &worker_config,
                    &metrics,
                    &envs,
                )
                .await
                .err()
                {
                    let m = add_completed_job_error(
                        db,
                        &job,
                        "Unexpected error during job execution:\n".to_string(),
                        &err,
                        &metrics,
                    )
                    .await
                    .map(|(_, m)| m)
                    .unwrap_or_else(|_| Map::new());

                    let _ = postprocess_queued_job(
                        job.is_flow_step,
                        job.schedule_path.clone(),
                        job.script_path.clone(),
                        &job.workspace_id,
                        job.id,
                        db,
                    )
                    .await;

                    if let Some(parent_job_id) = job.parent_job {
                        let updated_flow = update_flow_status_after_job_completion(
                            db,
                            &job,
                            false,
                            serde_json::Value::Object(m),
                            &metrics,
                        )
                        .await;
                        if let Err(err) = updated_flow {
                            if let Ok(mut tx) = db.begin().await {
                                if let Ok(Some(parent_job)) =
                                    get_queued_job(parent_job_id, &job.workspace_id, &mut tx).await
                                {
                                    let _ = add_completed_job_error(
                                        db,
                                        &parent_job,
                                        format!("Unexpected error during flow job error handling:\n{err}")
                                            ,
                                        err,
                                        &metrics,
                                    )
                                    .await;

                                    let _ = postprocess_queued_job(
                                        parent_job.is_flow_step,
                                        parent_job.schedule_path.clone(),
                                        parent_job.script_path.clone(),
                                        &job.workspace_id,
                                        parent_job.id,
                                        db,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    tracing::error!(job_id = %job.id, err = err.alt(), "error handling job: {} {} {}", job.id, job.workspace_id, job.created_by);
                };
            }
            Ok(None) => (),
            Err(err) => {
                tracing::error!(worker = %worker_name, "run_worker: pulling jobs: {}", err);
            }
        };

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(sleep_queue * num_workers))    => (),
            _ = rx.recv() => {
                 println!("received killpill for worker {}", i_worker);
                 break;
            }
        }
    }
}

async fn insert_initial_ping(worker_instance: &str, worker_name: &str, ip: &str, db: &DB) {
    sqlx::query!(
        "INSERT INTO worker_ping (worker_instance, worker, ip) VALUES ($1, $2, $3)",
        worker_instance,
        worker_name,
        ip
    )
    .execute(db)
    .await
    .expect("insert worker_ping initial value");
}

struct Envs {
    deno_path: String,
    go_path: String,
    python_path: String,
    nsjail_path: String,
    path_env: String,
    gopath_env: String,
    home_env: String,
    pip_index_url: Option<String>,
    pip_extra_index_url: Option<String>,
    pip_trusted_host: Option<String>,
}
async fn handle_queued_job(
    job: QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    timeout: i32,
    worker_name: &str,
    worker_dir: &str,
    worker_config: &WorkerConfig,
    metrics: &Metrics,
    envs: &Envs,
) -> crate::error::Result<()> {
    let job_id = job.id;
    let w_id = &job.workspace_id.clone();

    match job.job_kind {
        JobKind::FlowPreview | JobKind::Flow => {
            let args = job.args.clone().unwrap_or(Value::Null);
            handle_flow(&job, db, args).await?;
        }
        _ => {
            let mut logs = "".to_string();
            let mut last_line = "{}".to_string();

            if job.is_flow_step {
                update_flow_status_in_progress(
                    db,
                    &job.workspace_id,
                    job.parent_job
                        .ok_or_else(|| Error::InternalErr(format!("expected parent job")))?,
                    job.id,
                )
                .await?;
            }

            let execution = handle_job(
                &job,
                db,
                timeout,
                worker_name,
                worker_dir,
                &mut logs,
                &mut last_line,
                worker_config,
                envs,
            )
            .await;
            println!("{logs:#?} {execution:?}");

            match execution {
                Ok(r) => {
                    add_completed_job(db, &job, true, false, r.clone(), logs).await?;
                    if job.is_flow_step {
                        update_flow_status_after_job_completion(db, &job, true, r, metrics).await?;
                    }
                }
                Err(e) => {
                    let (_, output_map) =
                        add_completed_job_error(db, &job, logs, e, &metrics).await?;
                    if job.is_flow_step {
                        update_flow_status_after_job_completion(
                            db,
                            &job,
                            false,
                            serde_json::Value::Object(output_map),
                            metrics,
                        )
                        .await?;
                    }
                }
            };

            let _ = postprocess_queued_job(
                job.is_flow_step,
                job.schedule_path,
                job.script_path,
                &w_id,
                job_id,
                db,
            )
            .await;
        }
    }
    Ok(())
}

async fn write_file(dir: &str, path: &str, content: &str) -> Result<File, Error> {
    let path = format!("{}/{}", dir, path);
    let mut file = File::create(&path).await?;
    file.write_all(content.as_bytes()).await?;
    Ok(file)
}

#[async_recursion]
async fn transform_json_value(
    token: &str,
    workspace: &str,
    base_url: &str,
    v: Value,
) -> error::Result<Value> {
    match v {
        Value::String(y) if y.starts_with("$var:") => {
            let path = y.strip_prefix("$var:").unwrap();
            let v = crate::client::get_variable(workspace, path, token, base_url).await?;
            Ok(Value::String(v))
        }
        Value::String(y) if y.starts_with("$res:") => {
            let path = y.strip_prefix("$res:").unwrap();
            if path.split("/").count() < 2 {
                return Err(Error::InternalErr(
                    format!("invalid resource path: {path}",),
                ));
            }
            let v = crate::client::get_resource(workspace, path, token, base_url)
                .await?
                .ok_or_else(|| {
                    error::Error::InternalErr(format!("resource path: {path} not found",))
                })?;
            transform_json_value(token, workspace, base_url, v).await
        }
        Value::Object(mut m) => {
            for (a, b) in m.clone().into_iter() {
                m.insert(
                    a,
                    transform_json_value(token, workspace, base_url, b).await?,
                );
            }
            Ok(Value::Object(m))
        }
        a @ _ => Ok(a),
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_job(
    job: &QueuedJob,
    db: &DB,
    timeout: i32,
    worker_name: &str,
    worker_dir: &str,
    logs: &mut String,
    last_line: &mut String,
    worker_config: &WorkerConfig,
    envs: &Envs,
) -> Result<serde_json::Value, Error> {
    tracing::info!(
        worker = %worker_name,
        job_id = %job.id,
        workspace_id = %job.workspace_id,
        "handling job {}",
        job.id
    );

    logs.push_str(&format!("job {} on worker {}\n", &job.id, &worker_name));
    let job_dir = format!("{worker_dir}/{}", job.id);

    DirBuilder::new()
        .create(&job_dir)
        .await
        .expect("could not create job dir");

    let mut status: Result<ExitStatus, Error> =
        Err(Error::InternalErr("job not started".to_string()));

    if matches!(job.job_kind, JobKind::Dependencies) {
        handle_dependency_job(
            job,
            logs,
            &job_dir,
            &mut status,
            db,
            last_line,
            timeout,
            &envs,
        )
        .await?;
    } else {
        handle_code_execution_job(
            job,
            db,
            &job_dir,
            worker_dir,
            logs,
            &mut status,
            last_line,
            timeout,
            worker_config,
            envs,
        )
        .await?;
    }

    if !worker_config.keep_job_dir {
        tokio::fs::remove_dir_all(job_dir).await?;
    }

    if status.is_ok() && status.as_ref().unwrap().success() {
        let result = serde_json::from_str::<serde_json::Value>(last_line).map_err(|e| {
            Error::ExecutionErr(format!(
                "result {} is not parsable.\n err: {}",
                last_line,
                e.to_string()
            ))
        })?;
        Ok(result)
    } else {
        let err = match status {
            Ok(_) => {
                let s = format!(
                    "Error during execution of the script\nlast 5 logs lines:\n{}",
                    logs.lines()
                        .skip(logs.lines().count().max(5) - 5)
                        .join("\n")
                );
                logs.push_str("\n\n--- ERROR ---\n");
                s
            }
            Err(err) => format!("error before termination: {err}"),
        };
        Err(Error::ExecutionErr(err))
    }
}

async fn handle_code_execution_job(
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    job_dir: &String,
    worker_dir: &str,
    logs: &mut String,
    status: &mut Result<ExitStatus, Error>,
    last_line: &mut String,
    timeout: i32,
    worker_config: &WorkerConfig,
    envs: &Envs,
) -> Result<(), Error> {
    let (inner_content, requirements_o, language) = if matches!(job.job_kind, JobKind::Preview)
        || matches!(job.job_kind, JobKind::Script_Hub)
    {
        let code = (job.raw_code.as_ref().unwrap_or(&"no raw code".to_owned())).to_owned();
        let reqs = match job.language {
            Some(ScriptLang::Python3) => Some(parser_py::parse_python_imports(&code)?.join("\n")),
            _ => None,
        };
        (code, reqs, job.language.to_owned())
    } else {
        sqlx::query_as::<_, (String, Option<String>, Option<ScriptLang>)>(
            "SELECT content, lock, language FROM script WHERE hash = $1 AND (workspace_id = $2 OR \
             workspace_id = 'starter')",
        )
        .bind(&job.script_hash.unwrap_or(ScriptHash(0)).0)
        .bind(&job.workspace_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| Error::InternalErr(format!("expected content and lock")))?
    };
    let worker_name = worker_dir.split("/").last().unwrap_or("unknown");
    let lang_str = job
        .language
        .as_ref()
        .map(|x| format!("{x:?}"))
        .unwrap_or_else(|| "NO_LANG".to_string());

    tracing::info!(
        worker_name = %worker_name,
        job_id = %job.id,
        workspace_id = %job.workspace_id,
        is_ok = status.is_ok(),
        "started {} job {}",
        &lang_str,
        job.id
    );
    match language {
        None => {
            return Err(Error::ExecutionErr(
                "Require language to be not null".to_string(),
            ))?;
        }
        Some(ScriptLang::Python3) => {
            handle_python_job(
                worker_config,
                envs,
                requirements_o,
                job_dir,
                worker_dir,
                worker_name,
                job,
                logs,
                status,
                db,
                last_line,
                timeout,
                &inner_content,
            )
            .await?
        }
        Some(ScriptLang::Deno) => {
            handle_deno_job(
                worker_config,
                envs,
                logs,
                job,
                db,
                job_dir,
                &inner_content,
                timeout,
                status,
                last_line,
            )
            .await?;
        }
        Some(ScriptLang::Go) => {
            handle_go_job(
                worker_config,
                envs,
                logs,
                job,
                db,
                &inner_content,
                timeout,
                job_dir,
                requirements_o,
                status,
                last_line,
            )
            .await?
        }
    }
    tracing::info!(
        worker_name = %worker_name,
        job_id = %job.id,
        workspace_id = %job.workspace_id,
        is_ok = status.is_ok(),
        "finished {} job {}",
        &lang_str,
        job.id
    );
    Ok(())
}

async fn handle_go_job(
    WorkerConfig { base_internal_url, disable_nuser, disable_nsjail, base_url, .. }: &WorkerConfig,
    Envs { nsjail_path, go_path, path_env, gopath_env, home_env, .. }: &Envs,
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    inner_content: &str,
    timeout: i32,
    job_dir: &String,
    requirements_o: Option<String>,
    status: &mut Result<ExitStatus, Error>,
    last_line: &mut String,
) -> Result<(), Error> {
    //go does not like executing modules at temp root
    let job_dir = &format!("{job_dir}/go");
    if let Some(requirements) = requirements_o {
        gen_go_mymod(inner_content, job_dir).await?;
        let (md, sum) = requirements
            .split_once(GO_REQ_SPLITTER)
            .ok_or(Error::ExecutionErr(
                "Invalid requirement file, missing splitter".to_string(),
            ))?;
        write_file(job_dir, "go.mod", md).await?;
        write_file(job_dir, "go.sum", sum).await?;
    } else {
        logs.push_str("\n\n--- GO DEPENDENCIES SETUP---\n");
        set_logs(logs, job.id, db).await;

        install_go_dependencies(
            &job.id,
            inner_content,
            logs,
            job_dir,
            status,
            db,
            last_line,
            timeout,
            go_path,
            true,
        )
        .await?;
    }

    logs.push_str("\n\n--- GO CODE EXECUTION ---\n");
    set_logs(logs, job.id, db).await;

    let sig = crate::parser_go::parse_go_sig(&inner_content)?;
    let token = create_token_for_owner(
        &db,
        &job.workspace_id,
        &job.permissioned_as,
        "ephemeral-script",
        timeout * 2,
        &job.created_by,
    )
    .await?;
    let args = if let Some(args) = &job.args {
        Some(
            transform_json_value(&token, &job.workspace_id, &base_internal_url, args.clone())
                .await?,
        )
    } else {
        None
    };
    let ser_args = serde_json::to_string(&args).map_err(|e| Error::ExecutionErr(e.to_string()))?;
    write_file(job_dir, "args.json", &ser_args).await?;
    let spread = sig
        .args
        .into_iter()
        .map(|x| {
            format!(
                "json_arg[\"{}\"].({})",
                x.name,
                x.otyp.unwrap_or_else(|| "interface{}".to_string())
            )
        })
        .join(", ");

    let wrapper_content: String = format!(
        r#"
package main

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

    var json_arg map[string]interface{{}}

    if err := json.Unmarshal(dat, &json_arg); err != nil {{
        fmt.Println(err)
        os.Exit(1)
    }}

    res, err := inner.Inner_main({spread})
    if err != nil {{
        fmt.Println(err)
        os.Exit(1)
    }}
    res_json, err := json.Marshal(res)
    if err != nil {{
        fmt.Println(err)
        os.Exit(1)
    }}
    fmt.Println()
    fmt.Println("result:")
    fmt.Println(string(res_json))
}}

"#,
    );
    write_file(job_dir, "main.go", &wrapper_content).await?;
    let mut reserved_variables = get_reserved_variables(job, &token, &base_url, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let child = if !disable_nsjail {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_GO_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", GO_CACHE_DIR)
                .replace("{CLONE_NEWUSER}", &(!disable_nuser).to_string()),
        )
        .await?;

        Command::new(nsjail_path)
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", path_env)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                go_path,
                "run",
                "main.go",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new(go_path)
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", path_env)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("GOPATH", gopath_env)
            .env("HOME", home_env)
            .args(vec!["run", "main.go"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    *status = handle_child(&job.id, db, logs, last_line, timeout, child).await;
    Ok(())
}

async fn handle_deno_job(
    WorkerConfig { base_internal_url, base_url, disable_nuser, disable_nsjail, .. }: &WorkerConfig,
    Envs { nsjail_path, deno_path, path_env, .. }: &Envs,
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    job_dir: &String,
    inner_content: &String,
    timeout: i32,
    status: &mut Result<ExitStatus, Error>,
    last_line: &mut String,
) -> Result<(), Error> {
    logs.push_str("\n\n--- DENO CODE EXECUTION ---\n");
    set_logs(logs, job.id, db).await;
    let _ = write_file(job_dir, "inner.ts", inner_content).await?;
    let sig = crate::parser_ts::parse_deno_signature(inner_content)?;
    let token = create_token_for_owner(
        &db,
        &job.workspace_id,
        &job.permissioned_as,
        "ephemeral-script",
        timeout * 2,
        &job.created_by,
    )
    .await?;
    let args = if let Some(args) = &job.args {
        Some(
            transform_json_value(&token, &job.workspace_id, &base_internal_url, args.clone())
                .await?,
        )
    } else {
        None
    };
    let ser_args = serde_json::to_string(&args).map_err(|e| Error::ExecutionErr(e.to_string()))?;
    write_file(job_dir, "args.json", &ser_args).await?;
    let spread = sig.args.into_iter().map(|x| x.name).join(",");
    let wrapper_content: String = format!(
        r#"
import {{ main }} from "./inner.ts";

const args = await Deno.readTextFile("args.json")
    .then(JSON.parse)
    .then(({{ {spread} }}) => [ {spread} ])

async function run() {{
    let res: any = await main(...args);
    const res_json = JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value);
    console.log();
    console.log("result:");
    console.log(res_json);
    Deno.exit(0);
}}
run();
"#,
    );
    write_file(job_dir, "main.ts", &wrapper_content).await?;
    let mut reserved_variables = get_reserved_variables(job, &token, &base_url, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let hostname_base = base_url.split("://").last().unwrap_or("localhost");
    let hostname_internal = base_internal_url.split("://").last().unwrap_or("localhost");
    let deno_auth_tokens = format!("{token}@{hostname_base};{token}@{hostname_internal}");
    let child = if !disable_nsjail {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_DENO_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", DENO_CACHE_DIR)
                .replace("{CLONE_NEWUSER}", &(!disable_nuser).to_string()),
        )
        .await?;
        Command::new(nsjail_path)
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", path_env)
            .env("DENO_AUTH_TOKENS", deno_auth_tokens)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                deno_path,
                "run",
                "--unstable",
                "--v8-flags=--max-heap-size=2048",
                "-A",
                "/tmp/main.ts",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new(deno_path)
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", path_env)
            .env("DENO_AUTH_TOKENS", deno_auth_tokens)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec![
                "run",
                "--unstable",
                "--v8-flags=--max-heap-size=2048",
                "-A",
                "main.ts",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    *status = handle_child(&job.id, db, logs, last_line, timeout, child).await;

    Ok(())
}

async fn handle_python_job(
    WorkerConfig { base_internal_url, base_url, disable_nuser, disable_nsjail, .. }: &WorkerConfig,
    Envs {
        nsjail_path,
        python_path,
        path_env,
        pip_extra_index_url,
        pip_index_url,
        pip_trusted_host,
        ..
    }: &Envs,
    requirements_o: Option<String>,
    job_dir: &String,
    worker_dir: &str,
    worker_name: &str,
    job: &QueuedJob,
    logs: &mut String,
    status: &mut Result<ExitStatus, Error>,
    db: &sqlx::Pool<sqlx::Postgres>,
    last_line: &mut String,
    timeout: i32,
    inner_content: &String,
) -> Result<(), Error> {
    let requirements =
        requirements_o.ok_or_else(|| Error::InternalErr(format!("lockfile missing")))?;

    create_dependencies_dir(job_dir).await;

    if requirements.len() > 0 {
        if !disable_nsjail {
            let _ = write_file(
                job_dir,
                "download.config.proto",
                &NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT
                    .replace("{JOB_DIR}", job_dir)
                    .replace("{WORKER_DIR}", &worker_dir)
                    .replace("{CACHE_DIR}", PIP_CACHE_DIR)
                    .replace("{CLONE_NEWUSER}", &(!disable_nuser).to_string()),
            )
            .await?;
        }
        let _ = write_file(job_dir, "requirements.txt", &requirements).await?;

        tracing::info!(
            worker_name = %worker_name,
            job_id = %job.id,
            workspace_id = %job.workspace_id,
            "started setup python dependencies"
        );

        let mut vars = vec![];
        if let Some(url) = pip_extra_index_url {
            vars.push(("EXTRA_INDEX_URL", url));
        }
        if let Some(url) = pip_index_url {
            vars.push(("INDEX_URL", url));
        }
        if let Some(host) = pip_trusted_host {
            vars.push(("TRUSTED_HOST", host));
        }
        let child = if !disable_nsjail {
            Command::new(nsjail_path)
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
                "--no-color",
                "--isolated",
                "--no-warn-conflicts",
                "--disable-pip-version-check",
                "-t",
                "./dependencies",
                "-r",
                "./requirements.txt",
            ];
            if let Some(url) = pip_extra_index_url {
                args.extend(["--extra-index-url", url]);
            }
            if let Some(url) = pip_index_url {
                args.extend(["--index-url", url]);
            }
            if let Some(host) = pip_trusted_host {
                args.extend(["--trusted-host", host]);
            }
            Command::new(python_path)
                .current_dir(job_dir)
                .env_clear()
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        };

        logs.push_str("\n--- PIP DEPENDENCIES INSTALL ---\n");
        *status = handle_child(&job.id, db, logs, last_line, timeout, child).await;
        tracing::info!(
            worker_name = %worker_name,
            job_id = %job.id,
            workspace_id = %job.workspace_id,
            is_ok = status.is_ok(),
            "finished setting up python dependencies {}",
            job.id
        );
    }
    if requirements.len() == 0 || status.is_ok() {
        logs.push_str("\n\n--- PYTHON CODE EXECUTION ---\n");

        set_logs(logs, job.id, db).await;

        let _ = write_file(job_dir, "inner.py", inner_content).await?;

        let sig = crate::parser_py::parse_python_signature(inner_content)?;
        let transforms = sig
            .args
            .into_iter()
            .map(|x| match x.typ {
                Typ::Bytes => {
                    format!(
                        "if \"{}\" in kwargs and kwargs[\"{}\"] is not None:\n    \
                                     kwargs[\"{}\"] = base64.b64decode(kwargs[\"{}\"])\n",
                        x.name, x.name, x.name, x.name
                    )
                }
                Typ::Datetime => {
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

        let token = create_token_for_owner(
            &db,
            &job.workspace_id,
            &job.permissioned_as,
            "ephemeral-script",
            timeout * 2,
            &job.created_by,
        )
        .await?;

        let args = if let Some(args) = &job.args {
            Some(
                transform_json_value(&token, &job.workspace_id, &base_internal_url, args.clone())
                    .await?,
            )
        } else {
            None
        };
        let ser_args =
            serde_json::to_string(&args).map_err(|e| Error::ExecutionErr(e.to_string()))?;
        write_file(job_dir, "args.json", &ser_args).await?;

        let wrapper_content: String = format!(
            r#"
import json
import base64
from datetime import datetime

inner_script = __import__("inner")

with open("args.json") as f:
    kwargs = json.load(f, strict=False)
for k, v in list(kwargs.items()):
    if v == '<function call>':
        del kwargs[k]
{transforms}
res = inner_script.main(**kwargs)
res_json = json.dumps(res, separators=(',', ':'), default=str).replace('\n', '')
print()
print("result:")
print(res_json)
"#,
        );
        write_file(job_dir, "main.py", &wrapper_content).await?;

        let mut reserved_variables = get_reserved_variables(job, &token, &base_url, db).await?;
        if !disable_nsjail {
            let _ = write_file(
                job_dir,
                "run.config.proto",
                &NSJAIL_CONFIG_RUN_PYTHON3_CONTENT
                    .replace("{JOB_DIR}", job_dir)
                    .replace("{CLONE_NEWUSER}", &(!disable_nuser).to_string()),
            )
            .await?;
        } else {
            reserved_variables.insert("PYTHONPATH".to_string(), format!("{job_dir}/dependencies"));
        }

        tracing::info!(
            worker_name = %worker_name,
            job_id = %job.id,
            workspace_id = %job.workspace_id,
            "started python code execution {}",
            job.id
        );
        let child = if !disable_nsjail {
            Command::new(nsjail_path)
                .current_dir(job_dir)
                .env_clear()
                .envs(reserved_variables)
                .env("PATH", path_env)
                .env("BASE_INTERNAL_URL", base_internal_url)
                .args(vec![
                    "--config",
                    "run.config.proto",
                    "--",
                    python_path,
                    "-u",
                    "/tmp/main.py",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        } else {
            Command::new(python_path)
                .current_dir(job_dir)
                .env_clear()
                .envs(reserved_variables)
                .env("PATH", path_env)
                .env("BASE_INTERNAL_URL", base_internal_url)
                .args(vec!["-u", "main.py"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        };

        *status = handle_child(&job.id, db, logs, last_line, timeout, child).await;
    }
    Ok(())
}

async fn create_dependencies_dir(job_dir: &str) {
    DirBuilder::new()
        .recursive(true)
        .create(&format!("{job_dir}/dependencies"))
        .await
        .expect("could not create dependencies dir");
}

async fn handle_dependency_job(
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &String,
    status: &mut error::Result<ExitStatus>,
    db: &sqlx::Pool<sqlx::Postgres>,
    last_line: &mut String,
    timeout: i32,
    Envs { go_path, pip_extra_index_url, pip_index_url, pip_trusted_host, .. }: &Envs,
) -> error::Result<()> {
    let content = match job.language {
        Some(ScriptLang::Python3) => {
            create_dependencies_dir(job_dir).await;

            let requirements = job
                .raw_code
                .as_ref()
                .ok_or_else(|| Error::ExecutionErr("missing requirements".to_string()))?;
            logs.push_str(&format!("content of requirements:\n{}\n", &requirements));
            let file = "requirements.in";
            write_file(job_dir, file, &requirements).await?;
            let mut args = vec!["-q", "--no-header", file];
            if let Some(url) = pip_extra_index_url {
                args.extend(["--extra-index-url", url]);
            }
            if let Some(url) = pip_index_url {
                args.extend(["--index-url", url]);
            }
            if let Some(host) = pip_trusted_host {
                args.extend(["--trusted-host", host]);
            }
            let child = Command::new("pip-compile")
                .current_dir(job_dir)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            *status = handle_child(&job.id, db, logs, last_line, timeout, child).await;
            if status.is_ok() && status.as_ref().unwrap().success() {
                let path_lock = format!("{job_dir}/requirements.txt");
                let mut file = File::open(path_lock).await?;

                let mut req_content = "".to_string();
                file.read_to_string(&mut req_content).await?;
                Ok(req_content
                    .lines()
                    .filter(|x| !x.trim_start().starts_with('#'))
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join("\n"))
            } else {
                Err(format!("Lock file generation failed: {status:?}"))
            }
        }
        Some(ScriptLang::Go) => {
            let requirements = job
                .raw_code
                .as_ref()
                .ok_or_else(|| Error::ExecutionErr("missing requirements".to_string()))?;
            install_go_dependencies(
                &job.id,
                &requirements,
                logs,
                job_dir,
                status,
                db,
                last_line,
                timeout,
                go_path,
                false,
            )
            .await
            .map_err(|e| e.to_string())
        }
        _ => Err("Language incompatible with dep job".to_string()),
    };

    match content {
        Ok(content) => {
            let as_json = json!(content);

            *last_line =
                format!(r#"{{ "success": "Successful lock file generation", "lock": {as_json} }}"#);

            sqlx::query!(
                "UPDATE script SET lock = $1 WHERE hash = $2 AND workspace_id = $3",
                &content,
                &job.script_hash.unwrap_or(ScriptHash(0)).0,
                &job.workspace_id
            )
            .execute(db)
            .await?;
        }
        Err(error) => {
            sqlx::query!(
                "UPDATE script SET lock_error_logs = $1 WHERE hash = $2 AND workspace_id = $3",
                &format!("{logs}\n{error}"),
                &job.script_hash.unwrap_or(ScriptHash(0)).0,
                &job.workspace_id
            )
            .execute(db)
            .await?;
        }
    }
    Ok(())
}

async fn install_go_dependencies(
    job_id: &Uuid,
    code: &str,
    logs: &mut String,
    job_dir: &String,
    status: &mut Result<ExitStatus, Error>,
    db: &sqlx::Pool<sqlx::Postgres>,
    last_line: &mut String,
    timeout: i32,
    go_path: &str,
    preview: bool,
) -> error::Result<String> {
    gen_go_mymod(code, job_dir).await?;
    let child = Command::new("go")
        .current_dir(job_dir)
        .args(vec!["mod", "init", "mymod"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    *status = handle_child(job_id, db, logs, last_line, timeout, child).await;
    if status.is_ok() {
        let child = Command::new(go_path)
            .current_dir(job_dir)
            .args(vec!["mod", "tidy"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        *status = handle_child(job_id, db, logs, last_line, timeout, child).await;
    }
    if status.is_ok() && status.as_ref().unwrap().success() {
        if preview {
            Ok(String::new())
        } else {
            let mut req_content = "".to_string();

            let mut file = File::open(format!("{job_dir}/go.mod")).await?;
            file.read_to_string(&mut req_content).await?;

            req_content.push_str(&format!("\n{GO_REQ_SPLITTER}\n"));

            let mut file = File::open(format!("{job_dir}/go.sum")).await?;
            file.read_to_string(&mut req_content).await?;

            Ok(req_content)
        }
    } else {
        tracing::info!("go mod error");

        Err(error::Error::ExecutionErr(format!(
            "Lock file generation failed. Status: {status:?}",
        )))
    }
}

async fn gen_go_mymod(code: &str, job_dir: &String) -> error::Result<()> {
    let code = &format!("package inner; {code}").replace("func main(", "func Inner_main(");

    let mymod_dir = format!("{job_dir}/inner");
    DirBuilder::new()
        .recursive(true)
        .create(&mymod_dir)
        .await
        .expect("could not create go's mymod dir");

    write_file(&mymod_dir, "inner_main.go", &code).await?;

    Ok(())
}

async fn get_reserved_variables(
    job: &QueuedJob,
    token: &str,
    base_url: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> Result<HashMap<String, String>, Error> {
    let flow_path = if let Some(uuid) = job.parent_job {
        sqlx::query_scalar!("SELECT script_path FROM queue WHERE id = $1", uuid)
            .fetch_optional(db)
            .await?
            .flatten()
    } else {
        None
    };
    let variables = variables::get_reserved_variables(
        &job.workspace_id,
        token,
        &get_email_from_username(&job.created_by, db)
            .await?
            .unwrap_or_else(|| "nosuitable@email.xyz".to_string()),
        &job.created_by,
        &job.id.to_string(),
        &job.permissioned_as,
        base_url,
        job.script_path.clone(),
        job.parent_job.map(|x| x.to_string()),
        flow_path,
        job.schedule_path.clone(),
    );
    Ok(variables
        .into_iter()
        .map(|rv| (rv.name, rv.value))
        .collect())
}

/// - wait until child exits and return with exit status
/// - read lines from stdout and stderr and append them to the "queue"."logs"
///   quitting early if output exceedes MAX_LOG_SIZE characters (not bytes)
/// - update the `last_line` and `logs` strings with the program output
/// - update "queue"."last_ping" every five seconds
/// - kill process if we exceed timeout or "queue"."canceled" is set
async fn handle_child(
    job_id: &Uuid,
    db: &DB,
    logs: &mut String,
    last_line: &mut String,
    timeout: i32,
    mut child: Child,
) -> crate::error::Result<ExitStatus> {
    let timeout = Duration::from_secs(u64::try_from(timeout).expect("invalid timeout"));
    let ping_interval = Duration::from_secs(5);
    let cancel_check_interval = Duration::from_millis(500);
    let write_logs_delay = Duration::from_millis(500);

    let (set_too_many_logs, mut too_many_logs) = watch::channel::<bool>(false);

    let output = child_joined_output_stream(&mut child);
    let job_id = job_id.clone();

    /* the cancellation future is polled on by `wait_on_child` while
     * waiting for the child to exit normally */
    let cancel_check = async {
        let db = db.clone();

        let mut interval = interval_skipping_missed(cancel_check_interval).boxed();
        while let Some(_) = interval.next().await {
            if sqlx::query_scalar!("SELECT canceled FROM queue WHERE id = $1", job_id)
                .fetch_optional(&db)
                .await
                .map(|v| Some(true) == v)
                .unwrap_or_else(|err| {
                    tracing::error!(%job_id, %err, "error checking cancelation for job {job_id}: {err}");
                    false
                })
            {
                break;
            }
        }
    };

    /* a future that completes when the child process exits */
    let wait_on_child = async {
        let db = db.clone();

        let timed_out = tokio::select! {
            biased;
            result = child.wait() => return result.map(Some),
            _ = too_many_logs.changed() => false,
            _ = cancel_check => false,
            _ = sleep(timeout) => true,
        };

        let set_reason = async {
            if timed_out {
                if let Err(err) = sqlx::query(
                    r#"
                       UPDATE queue
                          SET canceled = true
                            , canceled_by = 'timeout',
                            , canceled_reason = $1
                        WHERE id = $2
                    r"#,
                )
                .bind(format!("duration > {}", timeout.as_secs()))
                .bind(job_id)
                .execute(&db)
                .await
                {
                    tracing::error!(%job_id, %err, "error setting cancelation reason for job {job_id}: {err}");
                }
            }
        };

        /* send SIGKILL and reap child process */
        let (_, kill) = future::join(set_reason, child.kill()).await;
        kill.map(|()| None)
    };

    /* a future that reads output from the child and appends to the database */
    let lines = async move {
        /* log_remaining is zero when output limit was reached */
        let mut log_remaining = (MAX_LOG_SIZE as usize).saturating_sub(logs.chars().count());
        let mut result = io::Result::Ok(());
        let mut output = output;
        /* `do_write` resolves the task, but does not contain the Result.
         * It's useful to know if the task completed. */
        let (mut do_write, mut write_result) = tokio::spawn(ready(())).remote_handle();

        while let Some(line) = output.by_ref().next().await {
            let do_write_ = do_write.shared();

            let mut read_lines = stream::once(async { line })
                .chain(output.by_ref())
                /* after receiving a line, continue until some delay has passed
                 * _and_ the previous database write is complete */
                .take_until(future::join(sleep(write_logs_delay), do_write_.clone()))
                .boxed();

            /* Read up until an error is encountered,
             * handle log lines first and then the error... */
            let mut joined = String::new();

            while let Some(line) = read_lines.next().await {
                match line {
                    Ok(_) if log_remaining == 0 => (),
                    Ok(line) => {
                        append_with_limit(&mut joined, &line, &mut log_remaining);

                        *last_line = line;

                        if log_remaining == 0 {
                            tracing::info!(%job_id, "Too many logs lines for job {job_id}");
                            let _ = set_too_many_logs.send(true);
                            joined.push_str(&format!(
                                "Job logs or result reached character limit of {MAX_LOG_SIZE}; killing job."
                            ));
                            /* stop reading and drop our streams fairly quickly */
                            break;
                        }
                    }
                    Err(err) => {
                        result = Err(err);
                        break;
                    }
                }
            }

            logs.push_str(&joined);

            /* Ensure the last flush completed before starting a new one.
             *
             * This shouldn't pause since `take_until()` reads lines until `do_write`
             * resolves. We only stop reading lines before `take_until()` resolves if we reach
             * EOF or a read error.  In those cases, waiting on a database query to complete is
             * fine because we're done. */

            if let Some(Ok(p)) = do_write_
                .then(|()| write_result)
                .await
                .err()
                .map(|err| err.try_into_panic())
            {
                panic::resume_unwind(p);
            }

            (do_write, write_result) =
                tokio::spawn(append_logs(job_id, joined, db.clone())).remote_handle();

            if let Err(err) = result {
                tracing::error!(%job_id, %err, "error reading output for job {job_id}: {err}");
                break;
            }

            if *set_too_many_logs.borrow() {
                break;
            }
        }

        /* drop our end of the pipe */
        drop(output);

        if let Some(Ok(p)) = do_write
            .then(|()| write_result)
            .await
            .err()
            .map(|err| err.try_into_panic())
        {
            panic::resume_unwind(p);
        }
    };

    /* a stream updating "queue"."last_ping" at an interval */

    let ping = interval_skipping_missed(ping_interval)
        .map(|_| db.clone())
        .then(move |db| async move {
            if let Err(err) =
                sqlx::query!("UPDATE queue SET last_ping = now() WHERE id = $1", job_id)
                    .execute(&db)
                    .await
            {
                tracing::error!(%job_id, %err, "error setting last ping for job {job_id}: {err}");
            }
        });

    let wait_result = tokio::select! {
        (w, _) = future::join(wait_on_child, lines) => w,
        /* ping should repeat forever without stopping */
        _ = ping.collect::<()>() => unreachable!("job ping stopped"),
    };

    match wait_result {
        _ if *too_many_logs.borrow() => Err(Error::ExecutionErr(
            "logs or result reached limit".to_string(),
        )),
        Ok(Some(status)) => Ok(status),
        Ok(None) => Err(Error::ExecutionErr("job process killed".to_string())),
        Err(err) => Err(Error::ExecutionErr(format!("job process io error: {err}"))),
    }
}

fn interval_skipping_missed(period: Duration) -> impl futures::Stream<Item = Instant> {
    let mut interval = interval(period);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    stream::poll_fn(move |cx| interval.poll_tick(cx).map(Some))
}

/// takes stdout and stderr from Child, panics if either are not present
///
/// builds a stream joining both stdout and stderr each read line by line
fn child_joined_output_stream(
    child: &mut Child,
) -> impl stream::FusedStream<Item = io::Result<String>> {
    let stderr = child
        .stderr
        .take()
        .expect("child did not have a handle to stdout");

    let stdout = child
        .stdout
        .take()
        .expect("child did not have a handle to stdout");

    let stdout = BufReader::new(stdout).lines();
    let stderr = BufReader::new(stderr).lines();
    stream::select(lines_to_stream(stderr), lines_to_stream(stdout))
}

fn lines_to_stream<R: tokio::io::AsyncBufRead + Unpin>(
    mut lines: tokio::io::Lines<R>,
) -> impl futures::Stream<Item = io::Result<String>> {
    stream::poll_fn(move |cx| {
        std::pin::Pin::new(&mut lines)
            .poll_next_line(cx)
            .map(|result| result.transpose())
    })
}

// as a detail, `BufReader::lines()` removes \n and \r\n from the strings it yields,
// so this pushes \n to thd destination string in each call
fn append_with_limit(dst: &mut String, src: &str, limit: &mut usize) {
    if *limit > 0 {
        dst.push('\n');
    }
    *limit -= 1;

    let src_len = src.chars().count();
    if src_len <= *limit {
        dst.push_str(&src);
        *limit -= src_len;
    } else {
        let byte_pos = src
            .char_indices()
            .skip(*limit)
            .next()
            .map(|(byte_pos, _)| byte_pos)
            .unwrap_or(0);
        dst.push_str(&src[0..byte_pos]);
        *limit = 0;
    }
}

async fn set_logs(logs: &str, id: uuid::Uuid, db: &DB) {
    if sqlx::query!(
        "UPDATE queue SET logs = $1 WHERE id = $2",
        logs.to_owned(),
        id
    )
    .execute(db)
    .await
    .is_err()
    {
        tracing::error!(%id, "error updating logs for id {id}")
    };
}

/* TODO retry this? */
async fn append_logs(job_id: uuid::Uuid, logs: impl AsRef<str>, db: impl Borrow<DB>) {
    if logs.as_ref().is_empty() {
        return;
    }

    if let Err(err) = sqlx::query!(
        "UPDATE queue SET logs = concat(logs, $1::text) WHERE id = $2",
        logs.as_ref(),
        job_id,
    )
    .execute(db.borrow())
    .await
    {
        tracing::error!(%job_id, %err, "error updating logs for job {job_id}: {err}");
    }
}

pub async fn restart_zombie_jobs_periodically(
    db: &DB,
    timeout: i32,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) {
    loop {
        let restarted = sqlx::query!(
            "UPDATE queue SET running = false WHERE last_ping < now() - ($1 || ' seconds')::interval AND running = true AND job_kind != $2 RETURNING id, workspace_id, last_ping",
            (timeout * 5).to_string(),
            JobKind::Flow: JobKind,
        )
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

        for r in restarted {
            tracing::info!(
                "restarted zombie job {} {} {}",
                r.id,
                r.workspace_id,
                r.last_ping
            );
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(60))    => (),
            _ = rx.recv() => {
                    println!("received killpill for monitor job");
                    break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::Stream;
    use serde_json::json;
    use sqlx::{postgres::PgListener, query_scalar};
    use uuid::Uuid;

    use crate::{
        db::DB,
        flows::{FlowModule, FlowModuleValue, FlowValue, InputTransform},
        jobs::{push, JobPayload, RawCode},
        scripts::ScriptLang,
        DEFAULT_SLEEP_QUEUE,
    };

    use super::*;

    async fn initialize_tracing() {
        use std::sync::Once;

        static ONCE: Once = Once::new();
        ONCE.call_once(crate::tracing_init::initialize_tracing);
    }

    /// it's important this is unique between tests as there is one prometheus registry and
    /// run_worker shouldn't register the same metric with the same worker name more than once.
    ///
    /// this must fit in varchar(50)
    fn next_worker_name() -> String {
        use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

        static ID: AtomicUsize = AtomicUsize::new(0);

        // n.b.: when tests are run with RUST_TEST_THREADS or --test-threads set to 1, the name
        // will be "main"... The id provides uniqueness & thread_name gives context.
        let id = ID.fetch_add(1, SeqCst);
        let thread = std::thread::current();
        let thread_name = thread
            .name()
            .map(|s| {
                s.len()
                    .checked_sub(39)
                    .and_then(|start| s.get(start..))
                    .unwrap_or(s)
            })
            .unwrap_or("no thread name");
        format!("{id}/{thread_name}")
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_deno_flow(db: DB) {
        initialize_tracing().await;

        let numbers = "export function main() { return [1, 2, 3]; }";
        let doubles = "export function main(n) { return n * 2; }";

        let flow = FlowValue {
            modules: vec![
                FlowModule {
                    value: FlowModuleValue::RawScript(RawCode {
                        language: ScriptLang::Deno,
                        content: numbers.to_string(),
                        path: None,
                    }),
                    input_transforms: Default::default(),
                    stop_after_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                },
                FlowModule {
                    value: FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Javascript { expr: "result".to_string() },
                        skip_failures: false,
                        modules: vec![FlowModule {
                            value: FlowModuleValue::RawScript(RawCode {
                                language: ScriptLang::Deno,
                                content: doubles.to_string(),
                                path: None,
                            }),
                            input_transforms: [(
                                "n".to_string(),
                                InputTransform::Javascript {
                                    expr: "previous_result.iter.value".to_string(),
                                },
                            )]
                            .into(),
                            stop_after_if: Default::default(),
                            summary: Default::default(),
                            suspend: Default::default(),
                            retry: None,
                        }],
                    },
                    input_transforms: Default::default(),
                    stop_after_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                },
            ],
            ..Default::default()
        };

        let job = JobPayload::RawFlow { value: flow, path: None };

        for i in 0..50 {
            println!("deno flow iteration: {}", i);
            let result = run_job_in_new_worker_until_complete(&db, job.clone()).await;
            assert_eq!(result, serde_json::json!([2, 4, 6]), "iteration: {}", i);
        }
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_stop_after_if(db: DB) {
        initialize_tracing().await;

        let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [
                {
                    "input_transforms": { "n": { "type": "javascript", "expr": "flow_input.n" } },
                    "value": {
                        "type": "rawscript",
                        "language": "python3",
                        "content": "def main(n): return n",
                    },
                    "stop_after_if": {
                        "expr": "result < 0",
                        "skip_if_stopped": false,
                    },
                },
                {
                    "input_transforms": { "n": { "type": "javascript", "expr": "previous_result" } },
                    "value": {
                        "type": "rawscript",
                        "language": "python3",
                        "content": "def main(n): return f'last step saw {n}'",
                    },
                },
            ],
        }))
        .unwrap();
        let job = JobPayload::RawFlow { value: flow, path: None };

        let result = RunJob::from(job.clone())
            .arg("n", json!(123))
            .run_until_complete(&db)
            .await;
        assert_eq!(json!("last step saw 123"), result);

        let result = RunJob::from(job.clone())
            .arg("n", json!(-123))
            .run_until_complete(&db)
            .await;
        assert_eq!(json!(-123), result);
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_python_flow(db: DB) {
        initialize_tracing().await;

        let numbers = "def main(): return [1, 2, 3]";
        let doubles = "def main(n): return n * 2";

        let flow: FlowValue = serde_json::from_value(serde_json::json!( {
            "input_transform": {},
            "modules": [
                {
                    "value": {
                        "type": "rawscript",
                        "language": "python3",
                        "content": numbers,
                    },
                },
                {
                    "value": {
                        "type": "forloopflow",
                        "iterator": { "type": "javascript", "expr": "result" },
                        "skip_failures": false,
                        "modules": [{
                            "value": {
                                "type": "rawscript",
                                "language": "python3",
                                "content": doubles,
                            },
                            "input_transform": {
                                "n": {
                                    "type": "javascript",
                                    "expr": "previous_result.iter.value",
                                },
                            },
                        }],
                    },
                },
            ],
        }))
        .unwrap();

        for i in 0..50 {
            println!("python flow iteration: {}", i);
            let result = run_job_in_new_worker_until_complete(
                &db,
                JobPayload::RawFlow { value: flow.clone(), path: None },
            )
            .await;

            assert_eq!(result, serde_json::json!([2, 4, 6]), "iteration: {i}");
        }
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_python_flow_2(db: DB) {
        initialize_tracing().await;

        let flow: FlowValue = serde_json::from_value(serde_json::json!({
                "modules": [
                    {
                        "value": {
                            "type": "rawscript",
                            "content": "import wmill\ndef main():  return \"Hello\"",
                            "language": "python3"
                        },
                        "input_transform": {}
                    }
                ]
        }))
        .unwrap();

        for i in 0..10 {
            println!("python flow iteration: {}", i);
            let result = run_job_in_new_worker_until_complete(
                &db,
                JobPayload::RawFlow { value: flow.clone(), path: None },
            )
            .await;

            assert_eq!(result, serde_json::json!("Hello"), "iteration: {i}");
        }
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_go_job(db: DB) {
        initialize_tracing().await;

        let content = r#"
import "fmt"

func main(derp string) (string, error) {
	fmt.Println("Hello, 世界")
	return fmt.Sprintf("hello %s", derp), nil
}
        "#
        .to_owned();

        let result = RunJob::from(JobPayload::Code(RawCode {
            content,
            path: None,
            language: ScriptLang::Go,
        }))
        .arg("derp", json!("world"))
        .run_until_complete(&db)
        .await;

        assert_eq!(result, serde_json::json!("hello world"));
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_python_job(db: DB) {
        initialize_tracing().await;

        let content = r#"
def main():
    return "hello world"
        "#
        .to_owned();

        let job = JobPayload::Code(RawCode { content, path: None, language: ScriptLang::Python3 });

        let result = run_job_in_new_worker_until_complete(&db, job).await;

        assert_eq!(result, serde_json::json!("hello world"));
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_python_job_with_imports(db: DB) {
        initialize_tracing().await;

        let content = r#"
import wmill

def main():
    return wmill.get_workspace()
        "#
        .to_owned();

        let job = JobPayload::Code(RawCode { content, path: None, language: ScriptLang::Python3 });

        let result = run_job_in_new_worker_until_complete(&db, job).await;

        assert_eq!(result, serde_json::json!("test-workspace"));
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_empty_loop(db: DB) {
        initialize_tracing().await;

        let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [
                {
                    "value": {
                        "type": "forloopflow",
                        "iterator": { "type": "static", "value": [] },
                        "modules": [
                            {
                                "input_transform": {
                                    "n": {
                                        "type": "javascript",
                                        "expr": "previous_result.iter.value",
                                    },
                                },
                                "value": {
                                    "type": "rawscript",
                                    "language": "python3",
                                    "content": "def main(n): return n",
                                },
                            }
                        ],
                    },
                },
                {
                    "input_transform": {
                        "items": {
                            "type": "javascript",
                            "expr": "previous_result",
                        },
                    },
                    "value": {
                        "type": "rawscript",
                        "language": "python3",
                        "content": "def main(items): return sum(items)",
                    },
                },
            ],
        }))
        .unwrap();

        let flow = JobPayload::RawFlow { value: flow, path: None };
        let result = run_job_in_new_worker_until_complete(&db, flow).await;

        assert_eq!(result, serde_json::json!(0));
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_empty_loop_2(db: DB) {
        initialize_tracing().await;

        let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [
                {
                    "value": {
                        "type": "forloopflow",
                        "iterator": { "type": "static", "value": [] },
                        "modules": [
                            {
                                "input_transform": {
                                    "n": {
                                        "type": "javascript",
                                        "expr": "previous_result.iter.value",
                                    },
                                },
                                "value": {
                                    "type": "rawscript",
                                    "language": "python3",
                                    "content": "def main(n): return n",
                                },
                            }
                        ],
                    },
                },
            ],
        }))
        .unwrap();

        let flow = JobPayload::RawFlow { value: flow, path: None };
        let result = run_job_in_new_worker_until_complete(&db, flow).await;

        assert_eq!(result, serde_json::json!([]));
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_step_after_loop(db: DB) {
        initialize_tracing().await;

        let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [
                {
                    "value": {
                        "type": "forloopflow",
                        "iterator": { "type": "static", "value": [2,3,4] },
                        "modules": [
                            {
                                "input_transform": {
                                    "n": {
                                        "type": "javascript",
                                        "expr": "previous_result.iter.value",
                                    },
                                },
                                "value": {
                                    "type": "rawscript",
                                    "language": "python3",
                                    "content": "def main(n): return n",
                                } ,
                            }
                        ],
                    },
                },
                {
                    "input_transform": {
                        "items": {
                            "type": "javascript",
                            "expr": "previous_result",
                        },
                    },
                    "value": {
                        "type": "rawscript",
                        "language": "python3",
                        "content": "def main(items): return sum(items)",
                    },
                },
            ],
        }))
        .unwrap();

        let flow = JobPayload::RawFlow { value: flow, path: None };
        let result = run_job_in_new_worker_until_complete(&db, flow).await;

        assert_eq!(result, serde_json::json!(9));
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_failure_module(db: DB) {
        initialize_tracing().await;

        let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [{
                "input_transform": {
                    "l": { "type": "javascript", "expr": "[]", },
                    "n": { "type": "javascript", "expr": "flow_input.n", },
                },
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 0) throw l; return { l: [...l, 0] } }",
                },
            }, {
                "input_transform": {
                    "l": { "type": "javascript", "expr": "previous_result.l", },
                    "n": { "type": "javascript", "expr": "flow_input.n", },
                },
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 1) throw l; return { l: [...l, 1] } }",
                },
            }, {
                "input_transform": {
                    "l": { "type": "javascript", "expr": "previous_result.l", },
                    "n": { "type": "javascript", "expr": "flow_input.n", },
                },
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 2) throw l; return { l: [...l, 2] } }",
                },
            }],
            "failure_module": {
                "input_transform": { "error": { "type": "javascript", "expr": "previous_result", } },
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(error) { return { 'from failure module': error } }",
                }
            },
        }))
        .unwrap();

        let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
            .arg("n", json!(0))
            .run_until_complete(&db)
            .await;
        assert!(result["from failure module"]["error"]
            .as_str()
            .unwrap()
            .contains("Uncaught (in promise) []"));

        let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
            .arg("n", json!(1))
            .run_until_complete(&db)
            .await;
        assert!(result["from failure module"]["error"]
            .as_str()
            .unwrap()
            .contains("Uncaught (in promise) [ 0 ]"));

        let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
            .arg("n", json!(2))
            .run_until_complete(&db)
            .await;
        assert!(result["from failure module"]["error"]
            .as_str()
            .unwrap()
            .contains("Uncaught (in promise) [ 0, 1 ]"));

        let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
            .arg("n", json!(3))
            .run_until_complete(&db)
            .await;
        assert_eq!(json!({ "l": [0, 1, 2] }), result);
    }

    mod suspend_resume {
        use super::*;

        use futures::{Stream, StreamExt};

        struct ApiServer {
            addr: std::net::SocketAddr,
            tx: tokio::sync::oneshot::Sender<()>,
            task: tokio::task::JoinHandle<hyper::Result<()>>,
        }

        impl ApiServer {
            async fn start(db: DB) -> Self {
                let (tx, rx) = tokio::sync::oneshot::channel::<()>();

                let sock = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

                let addr = sock.local_addr().unwrap();

                use crate::jobs::{cancel_suspended_job, resume_suspended_job};
                use axum::routing::{get, post};

                let router = axum::Router::new()
                    .route("/w/:workspace/jobs/resume/:id", get(resume_suspended_job))
                    .route("/w/:workspace/jobs/resume/:id", post(resume_suspended_job))
                    .route("/w/:workspace/jobs/cancel/:id", get(cancel_suspended_job))
                    .route("/w/:workspace/jobs/cancel/:id", post(cancel_suspended_job))
                    .layer(axum::extract::Extension(db));

                let serve = axum::Server::from_tcp(sock.into_std().unwrap())
                    .unwrap()
                    .serve(router.into_make_service())
                    .with_graceful_shutdown(async { drop(rx.await) });

                let task = tokio::task::spawn(serve);

                return Self { addr, tx, task };
            }

            async fn close(self) -> hyper::Result<()> {
                let Self { task, tx, .. } = self;
                drop(tx);
                task.await.unwrap()
            }
        }

        async fn wait_until_flow_suspends(
            flow: Uuid,
            mut queue: impl Stream<Item = Uuid> + Unpin,
            db: &DB,
        ) {
            loop {
                queue.by_ref().find(&flow).await.unwrap();
                if query_scalar("SELECT suspend > 0 FROM queue WHERE id = $1")
                    .bind(flow)
                    .fetch_one(db)
                    .await
                    .unwrap()
                {
                    break;
                }
            }
        }

        fn flow() -> FlowValue {
            serde_json::from_value(serde_json::json!({
                "modules": [{
                    "input_transform": {
                        "n": { "type": "javascript", "expr": "flow_input.n", },
                        "port": { "type": "javascript", "expr": "flow_input.port", },
                        "op": { "type": "javascript", "expr": "flow_input.op ?? 'resume'", },
                    },
                    "value": {
                        "type": "rawscript",
                        "language": "deno",
                        "content": "\
                            export async function main(n, port, op) {\
                                const job = Deno.env.get('WM_JOB_ID');
                                const r = await fetch(
                                    `http://localhost:${port}/w/test-workspace/jobs/${op}/${job}`,\
                                    {\
                                        method: 'POST',\
                                        body: JSON.stringify('from job'),\
                                        headers: { 'content-type': 'application/json' }\
                                    }\
                                );\
                                console.log(r);
                                return n + 1;\
                            }",
                    },
                    "suspend": 1,
                }, {
                    "input_transform": {
                        "n": { "type": "javascript", "expr": "previous_result", },
                        "resume": { "type": "javascript", "expr": "resume", },
                        "resumes": { "type": "javascript", "expr": "resumes", },
                    },
                    "value": {
                        "type": "rawscript",
                        "language": "deno",
                        "content": "export function main(n, resume, resumes) { return { n: n + 1, resume, resumes } }"
                    },
                    "suspend": 1,
                }, {
                    "input_transform": {
                        "last": { "type": "javascript", "expr": "previous_result", },
                        "resume": { "type": "javascript", "expr": "resume", },
                        "resumes": { "type": "javascript", "expr": "resumes", },
                    },
                    "value": {
                        "type": "rawscript",
                        "language": "deno",
                        "content": "export function main(last, resume, resumes) { return { last, resume, resumes } }"
                    },
                }],
            }))
            .unwrap()
        }

        #[sqlx::test(fixtures("base"))]
        async fn test(db: DB) {
            initialize_tracing().await;

            let server = ApiServer::start(db.clone()).await;
            let port = server.addr.port();

            let flow = RunJob::from(JobPayload::RawFlow { value: flow(), path: None })
                .arg("n", json!(1))
                .arg("port", json!(port))
                .push(&db)
                .await;

            let mut completed = listen_for_completed_jobs(&db).await;
            let queue = listen_for_queue(&db).await;
            let db_ = db.clone();

            in_test_worker(&db, async move {
                let db = db_;

                wait_until_flow_suspends(flow, queue, &db).await;
                /* The first job resumes itself. */
                let _first = completed.next().await.unwrap();
                /* ... and send a request resume it. */
                let second = completed.next().await.unwrap();

                /* ImZyb20gdGVzdCIK = base64 "from test" */
                reqwest::get(format!(
                        "http://localhost:{port}/w/test-workspace/jobs/resume/{second}?payload=ImZyb20gdGVzdCIK"
                    ))
                    .await
                    .unwrap()
                    .error_for_status()
                    .unwrap();

                completed.find(&flow).await.unwrap();
            })
            .await;

            server.close().await.unwrap();

            let result = completed_job_result(flow, &db).await;

            assert_eq!(
                json!({
                    "last": {
                        "resume": "from job",
                        "resumes": ["from job"],
                        "n": 3,
                    },
                    "resume": "from test",
                    "resumes": ["from test"],
                }),
                result
            );

            // ensure resumes are cleaned up through CASCADE when the flow is finished
            assert_eq!(
                0,
                query_scalar::<_, i64>("SELECT count(*) FROM resume_job")
                    .fetch_one(&db)
                    .await
                    .unwrap()
            );
        }

        #[sqlx::test(fixtures("base"))]
        async fn cancel_from_job(db: DB) {
            initialize_tracing().await;

            let server = ApiServer::start(db.clone()).await;
            let port = server.addr.port();

            let result = RunJob::from(JobPayload::RawFlow { value: flow(), path: None })
                .arg("n", json!(1))
                .arg("op", json!("cancel"))
                .arg("port", json!(port))
                .run_until_complete(&db)
                .await;

            server.close().await.unwrap();

            assert_eq!(json!("from job"), result);
        }

        #[sqlx::test(fixtures("base"))]
        async fn cancel_after_suspend(db: DB) {
            initialize_tracing().await;

            let server = ApiServer::start(db.clone()).await;
            let port = server.addr.port();

            let flow = RunJob::from(JobPayload::RawFlow { value: flow(), path: None })
                .arg("n", json!(1))
                .arg("port", json!(port))
                .push(&db)
                .await;

            let mut completed = listen_for_completed_jobs(&db).await;
            let queue = listen_for_queue(&db).await;
            let db_ = db.clone();

            in_test_worker(&db, async move {
                let db = db_;

                wait_until_flow_suspends(flow, queue, &db).await;
                /* The first job resumes itself. */
                let _first = completed.next().await.unwrap();
                /* ... and send a request resume it. */
                let second = completed.next().await.unwrap();

                reqwest::get(format!(
                        "http://localhost:{port}/w/test-workspace/jobs/cancel/{second}?payload=ImZyb20gdGVzdCIK"
                    ))
                    .await
                    .unwrap()
                    .error_for_status()
                    .unwrap();

                completed.find(&flow).await.unwrap();
            })
            .await;

            server.close().await.unwrap();

            let result = completed_job_result(flow, &db).await;

            assert_eq!(json!("from test"), result);
        }
    }

    mod retry {
        use super::*;

        /// test helper provides some external state to help steps fail at specific points
        struct Server {
            addr: std::net::SocketAddr,
            tx: tokio::sync::oneshot::Sender<()>,
            task: tokio::task::JoinHandle<Vec<u8>>,
        }

        impl Server {
            async fn start(responses: Vec<Option<u8>>) -> Self {
                use tokio::net::TcpListener;

                let (tx, rx) = tokio::sync::oneshot::channel::<()>();
                let sock = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = sock.local_addr().unwrap();

                let task = tokio::task::spawn(async move {
                    tokio::pin!(rx);
                    let mut results = vec![];

                    for next in responses {
                        let (mut peer, _) = tokio::select! {
                            _ = &mut rx => break,
                            r = sock.accept() => r,
                        }
                        .unwrap();

                        let n = peer.read_u8().await.unwrap();
                        results.push(n);

                        if let Some(next) = next {
                            peer.write_u8(next).await.unwrap();
                        }
                    }

                    results
                });

                return Self { addr, tx, task };
            }

            async fn close(self) -> Vec<u8> {
                let Self { task, tx, .. } = self;
                drop(tx);
                task.await.unwrap()
            }
        }

        fn inner_step() -> &'static str {
            r#"
export async function main(index, port) {
    const buf = new Uint8Array([0]);
    const sock = await Deno.connect({ port });
    await sock.write(new Uint8Array([index]));
    if (await sock.read(buf) != 1) throw "read";
    return buf[0];
}
            "#
        }

        fn last_step() -> &'static str {
            r#"
def main(last, port):
    with __import__("socket").create_connection((None, port)) as sock:
        sock.send(b'\xff')
        return last + [sock.recv(1)[0]]
"#
        }

        fn flow_forloop_retry() -> FlowValue {
            serde_json::from_value(serde_json::json!({
                "modules": [{
                    "value": {
                        "type": "forloopflow",
                        "iterator": { "type": "javascript", "expr": "result.items" },
                        "skip_failures": false,
                        "modules": [{
                            "input_transform": {
                                "index": { "type": "javascript", "expr": "previous_result.iter.index" },
                                "port": { "type": "javascript", "expr": "flow_input.port" },
                            },
                            "value": {
                                "type": "rawscript",
                                "language": "deno",
                                "content": inner_step(),
                            },
                        }],
                    },
                    "retry": { "constant": { "attempts": 2, "seconds": 0 } },
                }, {
                    "input_transform": {
                        "last": { "type": "javascript", "expr": "previous_result" },
                        "port": { "type": "javascript", "expr": "flow_input.port" },
                    },
                    "value": {
                        "type": "rawscript",
                        "language": "python3",
                        "content": last_step(),
                    },
                    "retry": { "constant": { "attempts": 2, "seconds": 0 } },
                }],
            })).unwrap()
        }

        #[sqlx::test(fixtures("base"))]
        async fn test_pass(db: DB) {
            initialize_tracing().await;

            /* fails twice in the loop, then once on the last step
             * retry attempts is measured per-step, so it _retries_ at most two times on each step,
             * which means it may run the step three times in total */

            let (attempts, responses) = [
                /* pass fail */
                (0, Some(99)),
                (1, None),
                /* pass pass fail */
                (0, Some(99)),
                (1, Some(99)),
                (2, None),
                /* pass pass pass */
                (0, Some(3)),
                (1, Some(5)),
                (2, Some(7)),
                /* fail the last step once */
                (0xff, None),
                (0xff, Some(9)),
            ]
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();
            let server = Server::start(responses).await;
            let result =
                RunJob::from(JobPayload::RawFlow { value: flow_forloop_retry(), path: None })
                    .arg("items", json!(["unused", "unused", "unused"]))
                    .arg("port", json!(server.addr.port()))
                    .run_until_complete(&db)
                    .await;

            assert_eq!(server.close().await, attempts);
            assert_eq!(json!([3, 5, 7, 9]), result);
        }

        #[sqlx::test(fixtures("base"))]
        async fn test_fail_step_zero(db: DB) {
            initialize_tracing().await;

            /* attempt and fail the first step three times and stop */
            let (attempts, responses) = [
                /* pass fail x3 */
                (0, Some(99)),
                (1, None),
                (0, Some(99)),
                (1, None),
                (0, Some(99)),
                (1, None),
            ]
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();
            let server = Server::start(responses).await;
            let result =
                RunJob::from(JobPayload::RawFlow { value: flow_forloop_retry(), path: None })
                    .arg("items", json!(["unused", "unused", "unused"]))
                    .arg("port", json!(server.addr.port()))
                    .run_until_complete(&db)
                    .await;

            assert_eq!(server.close().await, attempts);
            assert!(result["error"]
                .as_str()
                .unwrap()
                .contains(r#"Uncaught (in promise) "read""#));
        }

        #[sqlx::test(fixtures("base"))]
        async fn test_fail_step_one(db: DB) {
            initialize_tracing().await;

            /* attempt and fail the first step three times and stop */
            let (attempts, responses) = [
                /* fail once, then pass */
                (0, None),
                (0, Some(1)),
                (1, Some(2)),
                (2, Some(3)),
                /* fail three times */
                (0xff, None),
                (0xff, None),
                (0xff, None),
            ]
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();
            let server = Server::start(responses).await;
            let result =
                RunJob::from(JobPayload::RawFlow { value: flow_forloop_retry(), path: None })
                    .arg("items", json!(["unused", "unused", "unused"]))
                    .arg("port", json!(server.addr.port()))
                    .run_until_complete(&db)
                    .await;

            assert_eq!(server.close().await, attempts);
            assert!(result["error"]
                .as_str()
                .unwrap()
                .contains("index out of range"));
        }

        #[sqlx::test(fixtures("base"))]
        async fn test_with_failure_module(db: DB) {
            let value = serde_json::from_value(json!({
                "modules": [{
                    "input_transform": { "port": { "type": "javascript", "expr": "flow_input.port" } },
                    "value": {
                        "type": "rawscript",
                        "language": "python3",
                        "content": r#"
def main(port):
    with __import__("socket").create_connection((None, port)) as sock:
        sock.send(b'\x00')
        return sock.recv(1)[0]"#,
                    },
                    "retry": { "constant": { "attempts": 1, "seconds": 0 } },
                }],
                "failure_module": {
                    "input_transform": { "error": { "type": "javascript", "expr": "previous_result", },
                                         "port": { "type": "javascript", "expr": "flow_input.port" } },
                    "value": {
                        "type": "rawscript",
                        "language": "python3",
                        "content": r#"
def main(error, port):
    with __import__("socket").create_connection((None, port)) as sock:
        sock.send(b'\xff')
        return { "recv": sock.recv(1)[0], "from failure module": error }"#,
                    },
                    "retry": { "constant": { "attempts": 1, "seconds": 0 } },
                },
            }))
            .unwrap();
            let (attempts, responses) = [
                /* fail the first step twice */
                (0x00, None),
                (0x00, None),
                /* and the failure module once */
                (0xff, None),
                (0xff, Some(42)),
            ]
            .into_iter()
            .unzip::<_, _, Vec<_>, Vec<_>>();
            let server = Server::start(responses).await;
            let result = RunJob::from(JobPayload::RawFlow { value, path: None })
                .arg("port", json!(server.addr.port()))
                .run_until_complete(&db)
                .await;

            assert_eq!(server.close().await, attempts);
            assert_eq!(
                result,
                json!({
                    "recv": 42,
                    "from failure module": {
                        "error": "\
                            Error during execution of the script\nlast 5 logs lines:\n  \
                            File \"/tmp/main.py\", line 14, in <module>\n    \
                            res = inner_script.main(**kwargs)\n  \
                            File \"/tmp/inner.py\", line 5, in main\n    \
                            return sock.recv(1)[0]\nIndexError: index out of range"
                    }
                })
            );
        }

        #[sqlx::test(fixtures("base"))]
        async fn bad_values_max(db: DB) {
            let value = serde_json::from_value(json!({
                "modules": [{
                    "value": { "type": "rawscript", "language": "python3", "content": "asdf" },
                    "retry": { "exponential": { "attempts": 50, "seconds": 60 } },
                }],
            }))
            .unwrap();

            let result = RunJob::from(JobPayload::RawFlow { value, path: None })
                .run_until_complete(&db)
                .await;
            assert_eq!(
                result,
                json!({"error": "Bad request: retry interval exceeds the maximum of 21600 seconds"})
            )
        }
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_iteration(db: DB) {
        initialize_tracing().await;

        let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [{
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "result.items" },
                    "skip_failures": false,
                    "modules": [{
                        "input_transform": {
                            "n": {
                                "type": "javascript",
                                "expr": "previous_result.iter.value",
                            },
                        },
                        "value": {
                            "type": "rawscript",
                            "language": "python3",
                            "content": "def main(n):\n    if 1 < n:\n        raise StopIteration(n)",
                        },
                    }],
                },
            }],
        }))
        .unwrap();

        let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
            .arg("items", json!([]))
            .run_until_complete(&db)
            .await;
        assert_eq!(result, serde_json::json!([]));

        /* Don't actually test that this does 257 jobs or that will take forever. */
        let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
            .arg("items", json!((0..257).collect::<Vec<_>>()))
            .run_until_complete(&db)
            .await;
        assert!(matches!(result, Value::Object(_)));
        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("StopIteration: 2"));
    }

    struct RunJob {
        payload: JobPayload,
        args: Map<String, serde_json::Value>,
    }

    impl From<JobPayload> for RunJob {
        fn from(payload: JobPayload) -> Self {
            Self { payload, args: Default::default() }
        }
    }

    impl RunJob {
        fn arg<S: Into<String>>(mut self, k: S, v: serde_json::Value) -> Self {
            self.args.insert(k.into(), v);
            self
        }

        async fn push(self, db: &DB) -> Uuid {
            let RunJob { payload, args } = self;

            let (uuid, tx) = push(
                db.begin().await.unwrap(),
                "test-workspace",
                payload,
                Some(args),
                /* user */ "test-user",
                /* permissioned_as */ Default::default(),
                /* scheduled_for_o */ None,
                /* schedule_path */ None,
                /* parent_job */ None,
                /* is_flow_step */ false,
            )
            .await
            .unwrap();

            tx.commit().await.unwrap();

            uuid
        }

        /// push the job, spawn a worker, wait until the job is in completed_job
        async fn run_until_complete(self, db: &DB) -> serde_json::Value {
            let uuid = self.push(db).await;
            let listener = listen_for_completed_jobs(db).await;
            in_test_worker(db, listener.find(&uuid)).await;
            completed_job_result(uuid, db).await
        }
    }

    async fn run_job_in_new_worker_until_complete(db: &DB, job: JobPayload) -> serde_json::Value {
        RunJob::from(job).run_until_complete(db).await
    }

    /// Start a worker with a timeout and run a future, until the worker quits or we time out.
    ///
    /// Cleans up the worker before resolving.
    async fn in_test_worker<Fut: std::future::Future>(
        db: &DB,
        inner: Fut,
    ) -> <Fut as std::future::Future>::Output {
        let (quit, worker) = spawn_test_worker(db);
        let worker = tokio::time::timeout(std::time::Duration::from_secs(19), worker);
        tokio::pin!(worker);

        let res = tokio::select! {
            biased;
            res = inner => res,
            res = &mut worker => match
                res.expect("worker timed out")
                   .expect("worker panicked") {
                _ => panic!("worker quit early"),
            },
        };

        /* ensure the worker quits before we return */
        drop(quit);

        let _: () = worker
            .await
            .expect("worker timed out")
            .expect("worker panicked");

        res
    }

    fn spawn_test_worker(
        db: &DB,
    ) -> (
        tokio::sync::broadcast::Sender<()>,
        tokio::task::JoinHandle<()>,
    ) {
        let (tx, rx) = tokio::sync::broadcast::channel(1);
        let db = db.to_owned();
        let timeout = 4_000;
        let worker_instance: &str = "test worker instance";
        let worker_name: String = next_worker_name();
        let i_worker: u64 = Default::default();
        let num_workers: u64 = 2;
        let ip: &str = Default::default();
        let sleep_queue: u64 = DEFAULT_SLEEP_QUEUE / num_workers;
        let worker_config = WorkerConfig {
            base_internal_url: String::new(),
            base_url: String::new(),
            disable_nuser: false,
            disable_nsjail: false,
            keep_job_dir: false,
        };
        let future = async move {
            run_worker(
                &db,
                timeout,
                worker_instance,
                worker_name,
                i_worker,
                num_workers,
                ip,
                sleep_queue,
                worker_config,
                rx,
            )
            .await
        };

        (tx, tokio::task::spawn(future))
    }

    async fn listen_for_completed_jobs(db: &DB) -> impl Stream<Item = Uuid> + Unpin {
        listen_for_uuid_on(db, "insert on completed_job").await
    }

    async fn listen_for_queue(db: &DB) -> impl Stream<Item = Uuid> + Unpin {
        listen_for_uuid_on(db, "queue").await
    }

    async fn listen_for_uuid_on(
        db: &DB,
        channel: &'static str,
    ) -> impl Stream<Item = Uuid> + Unpin {
        let mut listener = PgListener::connect_with(db).await.unwrap();
        listener.listen(channel).await.unwrap();

        Box::pin(stream::unfold(listener, |mut listener| async move {
            let uuid = listener
                .try_recv()
                .await
                .unwrap()
                .expect("lost database connection")
                .payload()
                .parse::<Uuid>()
                .expect("invalid uuid");
            Some((uuid, listener))
        }))
    }

    async fn completed_job_result(uuid: Uuid, db: &DB) -> Value {
        query_scalar("SELECT result FROM completed_job WHERE id = $1")
            .bind(uuid)
            .fetch_one(db)
            .await
            .unwrap()
    }

    #[axum::async_trait(?Send)]
    trait StreamFind: futures::Stream + Unpin + Sized {
        async fn find(self, item: &Self::Item) -> Option<Self::Item>
        where
            for<'l> &'l Self::Item: std::cmp::PartialEq,
        {
            use futures::{future::ready, StreamExt};

            self.filter(|i| ready(i == item)).next().await
        }
    }

    impl<T: futures::Stream + Unpin + Sized> StreamFind for T {}
}
