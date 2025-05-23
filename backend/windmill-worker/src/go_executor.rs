use crate::PROXY_ENVS;
use std::{collections::HashMap, fs::DirBuilder, process::Stdio};

use itertools::Itertools;
use serde_json::value::RawValue;
use tokio::{fs::File, io::AsyncReadExt, process::Command};
use uuid::Uuid;
use windmill_common::{
    error::{self, Error},
    utils::calculate_hash,
    worker::{save_cache, write_file, Connection},
};
use windmill_parser_go::{parse_go_imports, REQUIRE_PARSE};
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::{
    common::{
        capitalize, create_args_and_out_file, get_reserved_variables, read_result,
        start_child_process, OccupancyMetrics,
    },
    handle_child::handle_child,
    DISABLE_NSJAIL, DISABLE_NUSER, GOPRIVATE, GOPROXY, GO_BIN_CACHE_DIR,
    GO_CACHE_DIR, HOME_ENV, NSJAIL_PATH, PATH_ENV, TZ_ENV,
};
use windmill_common::client::AuthedClient;


const GO_REQ_SPLITTER: &str = "//go.sum\n";
const NSJAIL_CONFIG_RUN_GO_CONTENT: &str = include_str!("../nsjail/run.go.config.proto");

lazy_static::lazy_static! {
    static ref GO_PATH: String = std::env::var("GO_PATH").unwrap_or_else(|_| "/usr/bin/go".to_string());
}

pub const GO_OBJECT_STORE_PREFIX: &str = "gobin/";
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
    requirements_o: Option<&String>,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    occupation_metrics: &mut OccupancyMetrics,
) -> Result<Box<RawValue>, Error> {
    //go does not like executing modules at temp root
    let job_dir = &format!("{job_dir}/go");
    DirBuilder::new()
        .recursive(true)
        .create(&job_dir)
        .expect("could not create go job dir");

    let hash = calculate_hash(&format!(
        "{}{}v2",
        inner_content,
        requirements_o
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or_default()
    ));
    let bin_path = format!("{}/{hash}", GO_BIN_CACHE_DIR);
    let remote_path = format!("{GO_OBJECT_STORE_PREFIX}{hash}");
    let (cache, cache_logs) =
        windmill_common::worker::load_cache(&bin_path, &remote_path, false).await;

    let (skip_go_mod, skip_tidy) = if cache {
        (true, true)
    } else if let Some(requirements) = requirements_o {
        gen_go_mod(inner_content, job_dir, &requirements).await?
    } else {
        (false, false)
    };

    let cache_logs = if !cache {
        let logs1 = format!("{cache_logs}\n\n--- GO DEPENDENCIES SETUP ---\n");
        append_logs(&job.id, &job.workspace_id, logs1, conn).await;

        install_go_dependencies(
            &job.id,
            inner_content,
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

        create_args_and_out_file(client, job, job_dir, conn).await?;
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

        let mut build_go_cmd = Command::new(GO_PATH.as_str());
        build_go_cmd
            .current_dir(job_dir)
            .env_clear()
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("GOPATH", GO_CACHE_DIR)
            .env("HOME", HOME_ENV.as_str())
            .envs(PROXY_ENVS.clone())
            .args(vec!["build", "main.go"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        build_go_cmd.env("USERPROFILE", crate::USERPROFILE_ENV.as_str());

        let build_go_process = start_child_process(build_go_cmd, GO_PATH.as_str()).await?;
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
        )
        .await?;

        match save_cache(
            &bin_path,
            &format!("{GO_OBJECT_STORE_PREFIX}{hash}"),
            &format!("{job_dir}/main"),
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
        }
    } else {
        let target = format!("{job_dir}/main");
        #[cfg(unix)]
        let symlink = std::os::unix::fs::symlink(&bin_path, &target);
        #[cfg(windows)]
        let symlink = std::os::windows::fs::symlink_dir(&bin_path, &target);

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

    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_GO_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )?;
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["--config", "run.config.proto", "--", "/tmp/go/main"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
    } else {
        let compiled_executable_name = "./main";
        let mut run_go = Command::new(compiled_executable_name);
        run_go
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("GOPATH", GO_CACHE_DIR)
            .env("HOME", HOME_ENV.as_str());

        if let Some(ref goprivate) = *GOPRIVATE {
            run_go.env("GOPRIVATE", goprivate);
        }
        if let Some(ref goproxy) = *GOPROXY {
            run_go.env("GOPROXY", goproxy);
        }

        #[cfg(windows)]
        run_go.env("USERPROFILE", crate::USERPROFILE_ENV.as_str());

        run_go.stdout(Stdio::piped()).stderr(Stdio::piped());
        start_child_process(run_go, compiled_executable_name).await?
    };
    handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        &job.workspace_id,
        "go run",
        job.timeout,
        false,
        &mut Some(occupation_metrics),
        None,
    )
    .await?;

    read_result(job_dir).await
}

async fn gen_go_mod(
    inner_content: &str,
    job_dir: &str,
    requirements: &str,
) -> error::Result<(bool, bool)> {
    gen_go_mymod(inner_content, job_dir).await?;

    let md = requirements.split_once(GO_REQ_SPLITTER);
    if let Some((req, sum)) = md {
        write_file(job_dir, "go.mod", &req)?;
        write_file(job_dir, "go.sum", &sum)?;
        Ok((true, true))
    } else {
        write_file(job_dir, "go.mod", &requirements)?;
        Ok((true, false))
    }
}

use std::fs::OpenOptions;
use std::io::prelude::*;

pub async fn install_go_dependencies(
    job_id: &Uuid,
    code: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    conn: &Connection,
    non_dep_job: bool,
    skip_go_mod: bool,
    has_sum: bool,
    worker_name: &str,
    w_id: &str,
    occupation_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    if !skip_go_mod {
        gen_go_mymod(code, job_dir).await?;
        let mut child_cmd = Command::new(GO_PATH.as_str());
        child_cmd
            .current_dir(job_dir)
            .args(vec!["mod", "init", "mymod"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child_process = start_child_process(child_cmd, GO_PATH.as_str()).await?;

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

    let mut new_lockfile = false;

    let hash = if !has_sum {
        calculate_hash(parse_go_imports(&code)?.iter().join("\n").as_str())
    } else {
        "".to_string()
    };
    let hash = format!("go-{}", hash);

    let mut skip_tidy = has_sum;

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
        .env("GOPATH", GO_CACHE_DIR)
        .args(vec!["mod", mod_command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child_process = start_child_process(child_cmd, GO_PATH.as_str()).await?;

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
                "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('5 mins')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = $2",
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
