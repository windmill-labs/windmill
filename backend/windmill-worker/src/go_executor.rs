use std::process::Stdio;

use itertools::Itertools;
use tokio::{
    fs::{DirBuilder, File},
    io::AsyncReadExt,
    process::Command,
};
use uuid::Uuid;
use windmill_common::{
    error::{self, Error},
    jobs::QueuedJob,
    utils::calculate_hash,
};
use windmill_parser_go::parse_go_imports;

use crate::{
    common::{capitalize, read_result, set_logs},
    create_args_and_out_file, get_reserved_variables, handle_child, write_file,
    AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, GOPRIVATE, GO_CACHE_DIR, HOME_ENV,
    NETRC, NSJAIL_PATH, PATH_ENV,
};

const GO_REQ_SPLITTER: &str = "//go.sum\n";
const NSJAIL_CONFIG_RUN_GO_CONTENT: &str = include_str!("../nsjail/run.go.config.proto");

lazy_static::lazy_static! {
    static ref GO_PATH: String = std::env::var("GO_PATH").unwrap_or_else(|_| "/usr/bin/go".to_string());
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_go_job(
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    inner_content: &str,
    job_dir: &str,
    requirements_o: Option<String>,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
) -> Result<serde_json::Value, Error> {
    //go does not like executing modules at temp root
    let job_dir = &format!("{job_dir}/go");
    let (skip_go_mod, skip_tidy) = if let Some(requirements) = requirements_o {
        gen_go_mod(inner_content, job_dir, &requirements).await?
    } else {
        (false, false)
    };

    logs.push_str("\n\n--- GO DEPENDENCIES SETUP ---\n");
    set_logs(logs, &job.id, db).await;

    install_go_dependencies(
        &job.id,
        inner_content,
        logs,
        job_dir,
        db,
        true,
        skip_go_mod,
        skip_tidy,
        worker_name,
        &job.workspace_id,
    )
    .await?;

    logs.push_str("\n\n--- GO CODE EXECUTION ---\n");
    set_logs(logs, &job.id, db).await;
    let client = &client.get_authed().await;
    create_args_and_out_file(client, job, job_dir).await?;
    {
        let sig = windmill_parser_go::parse_go_sig(&inner_content)?;
        drop(inner_content);

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

        write_file(job_dir, "main.go", WRAPPER_CONTENT).await?;

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
            write_file(&format!("{job_dir}/inner"), "runner.go", &runner_content).await?;
        }
    }
    let mut reserved_variables = get_reserved_variables(job, &client.token, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_GO_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", GO_CACHE_DIR)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )
        .await?;
        let build_go = Command::new(GO_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("GOPATH", GO_CACHE_DIR)
            .env("HOME", HOME_ENV.as_str())
            .args(vec!["build", "main.go"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        handle_child(
            &job.id,
            db,
            logs,
            build_go,
            false,
            worker_name,
            &job.workspace_id,
            "go build",
        )
        .await?;

        Command::new(NSJAIL_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["--config", "run.config.proto", "--", "/tmp/go/main"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        if let Some(ref netrc) = *NETRC {
            write_file(&HOME_ENV, ".netrc", netrc).await?;
        }
        Command::new(GO_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("GOPATH", GO_CACHE_DIR)
            .env("GOPRIVATE", GOPRIVATE.as_ref().unwrap_or(&String::new()))
            .env("HOME", HOME_ENV.as_str())
            .args(vec!["run", "main.go"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    handle_child(
        &job.id,
        db,
        logs,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        &job.workspace_id,
        "go run",
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
        write_file(job_dir, "go.mod", &req).await?;
        write_file(job_dir, "go.sum", &sum).await?;
        Ok((true, true))
    } else {
        write_file(job_dir, "go.mod", &requirements).await?;
        Ok((true, false))
    }
}

pub async fn install_go_dependencies(
    job_id: &Uuid,
    code: &str,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    non_dep_job: bool,
    skip_go_mod: bool,
    has_sum: bool,
    worker_name: &str,
    w_id: &str,
) -> error::Result<String> {
    if !skip_go_mod {
        gen_go_mymod(code, job_dir).await?;
        let child = Command::new("go")
            .current_dir(job_dir)
            .args(vec!["mod", "init", "mymod"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        handle_child(job_id, db, logs, child, false, worker_name, w_id, "go init").await?;
    }

    let mut new_lockfile = false;

    let hash = if !has_sum {
        calculate_hash(parse_go_imports(&code)?.iter().join("\n").as_str())
    } else {
        "".to_string()
    };

    let mut skip_tidy = has_sum;

    if !has_sum {
        if let Some(cached) = sqlx::query_scalar!(
            "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
            hash
        )
        .fetch_optional(db)
        .await?
        {
            logs.push_str(&format!("\nfound cached resolution"));
            gen_go_mod(code, job_dir, &cached).await?;
            skip_tidy = true;
            new_lockfile = false;
        } else {
            new_lockfile = true;
        }
    }

    let mod_command = if skip_tidy { "download" } else { "tidy" };
    let child = Command::new(GO_PATH.as_str())
        .current_dir(job_dir)
        .env("GOPATH", GO_CACHE_DIR)
        .args(vec!["mod", mod_command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    handle_child(
        job_id,
        db,
        logs,
        child,
        false,
        worker_name,
        &w_id,
        &format!("go {mod_command}"),
    )
    .await
    .map_err(|e| Error::ExecutionErr(format!("Lock file generation failed: {e:?}")))?;

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
        sqlx::query!(
            "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('3 days')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = $2",
            hash,
            req_content
        ).fetch_optional(db).await?;

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
        .await
        .expect("could not create go's mymod dir");

    write_file(&mymod_dir, "inner_main.go", &code).await?;

    Ok(())
}
