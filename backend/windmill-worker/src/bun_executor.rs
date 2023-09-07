use std::{collections::HashMap, process::Stdio};

#[cfg(feature = "enterprise")]
use std::collections::VecDeque;

#[cfg(feature = "enterprise")]
use anyhow::Context;

use base64::Engine;
use itertools::Itertools;
use uuid::Uuid;

#[cfg(feature = "enterprise")]
use crate::{common::build_envs_map, JobCompleted};

use crate::{
    common::{
        create_args_and_out_file, get_reserved_variables, handle_child, read_result, set_logs,
        write_file, write_file_binary,
    },
    AuthedClientBackgroundTask, BUN_CACHE_DIR, BUN_PATH, DISABLE_NSJAIL, DISABLE_NUSER,
    NPM_CONFIG_REGISTRY, NSJAIL_PATH, PATH_ENV, TZ_ENV,
};

#[cfg(feature = "enterprise")]
use crate::MAX_BUFFERED_DEDICATED_JOBS;

use tokio::{fs::File, process::Command};

#[cfg(feature = "enterprise")]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use tokio::io::AsyncReadExt;

#[cfg(feature = "enterprise")]
use tokio::sync::mpsc::{Receiver, Sender};

#[cfg(feature = "enterprise")]
use windmill_common::variables;

use windmill_common::{
    error::{self, Result},
    jobs::QueuedJob,
};
use windmill_parser::Typ;

const RELATIVE_BUN_LOADER: &str = include_str!("../loader.bun.ts");

const RELATIVE_BUN_BUILDER: &str = include_str!("../loader_builder.bun.ts");

const NSJAIL_CONFIG_RUN_BUN_CONTENT: &str = include_str!("../nsjail/run.bun.config.proto");

pub const BUN_LOCKB_SPLIT: &str = "\n//bun.lockb\n";
pub const EMPTY_FILE: &str = "<empty>";

pub async fn gen_lockfile(
    logs: &mut String,
    job_id: &Uuid,
    w_id: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    token: &str,
    script_path: &str,
    job_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    export_pkg: bool,
) -> Result<Option<String>> {
    let _ = write_file(
        &job_dir,
        "build.ts",
        &format!(
            r#"
{}

{RELATIVE_BUN_BUILDER}
"#,
            RELATIVE_BUN_LOADER
                .replace("W_ID", w_id)
                .replace("BASE_INTERNAL_URL", base_internal_url)
                .replace("TOKEN", token)
                .replace("CURRENT_PATH", script_path)
        ),
    )
    .await?;

    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url);

    let child = Command::new(&*BUN_PATH)
        .current_dir(job_dir)
        .env_clear()
        .envs(common_bun_proc_envs.clone())
        .args(vec!["run", "build.ts"])
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
        w_id,
        "bun build",
        None,
    )
    .await?;

    install_lockfile(
        logs,
        job_id,
        w_id,
        db,
        job_dir,
        worker_name,
        common_bun_proc_envs,
    )
    .await?;

    if export_pkg {
        let mut content = "".to_string();
        {
            let mut file = File::open(format!("{job_dir}/package.json")).await?;
            file.read_to_string(&mut content).await?;
        }
        content.push_str(BUN_LOCKB_SPLIT);
        {
            let file = format!("{job_dir}/bun.lockb");
            if tokio::fs::metadata(&file).await.is_ok() {
                let mut file = File::open(&file).await?;
                let mut buf = vec![];
                file.read_to_end(&mut buf).await?;
                content.push_str(&base64::engine::general_purpose::STANDARD.encode(&buf));
            } else {
                content.push_str(&EMPTY_FILE);
            }
        }
        Ok(Some(content))
    } else {
        Ok(None)
    }
}

pub async fn install_lockfile(
    logs: &mut String,
    job_id: &Uuid,
    w_id: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    job_dir: &str,
    worker_name: &str,
    common_bun_proc_envs: HashMap<String, String>,
) -> Result<()> {
    let child = Command::new(&*BUN_PATH)
        .current_dir(job_dir)
        .env_clear()
        .envs(common_bun_proc_envs)
        .args(vec!["install"])
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
        w_id,
        "bun install",
        None,
    )
    .await?;
    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_bun_job(
    requirements_o: Option<String>,
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    job_dir: &str,
    inner_content: &String,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    shared_mount: &str,
) -> error::Result<serde_json::Value> {
    let _ = write_file(job_dir, "main.ts", inner_content).await?;
    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url);

    if let Some(reqs) = requirements_o {
        let splitted = reqs.split(BUN_LOCKB_SPLIT).collect::<Vec<&str>>();
        if splitted.len() != 2 {
            return Err(error::Error::ExecutionErr(
                format!("Invalid requirements, expectd to find //bun.lockb split pattern in reqs. Found: |{reqs}|")
            ));
        }
        let _ = write_file(job_dir, "package.json", &splitted[0]).await?;
        let lockb = splitted[1];
        if lockb != EMPTY_FILE {
            let _ = write_file_binary(
                job_dir,
                "bun.lockb",
                &base64::engine::general_purpose::STANDARD
                    .decode(&splitted[1])
                    .map_err(|_| {
                        error::Error::InternalErr("Could not decode bun.lockb".to_string())
                    })?,
            )
            .await?;
        }
        install_lockfile(
            logs,
            &job.id,
            &job.workspace_id,
            db,
            job_dir,
            worker_name,
            common_bun_proc_envs.clone(),
        )
        .await?;
    } else if !*DISABLE_NSJAIL {
        logs.push_str("\n\n--- BUN INSTALL ---\n");
        set_logs(&logs, &job.id, &db).await;
        let _ = gen_lockfile(
            logs,
            &job.id,
            &job.workspace_id,
            db,
            &client.get_token().await,
            &job.script_path(),
            job_dir,
            base_internal_url,
            worker_name,
            false,
        )
        .await?;
    }

    logs.push_str("\n\n--- BUN CODE EXECUTION ---\n");

    let logs_f = async {
        set_logs(&logs, &job.id, &db).await;
        Ok(()) as error::Result<()>
    };

    let write_wrapper_f = async {
        // let mut start = Instant::now();
        let args = windmill_parser_ts::parse_deno_signature(inner_content, true)?.args;
        let dates = args
            .iter()
            .enumerate()
            .filter_map(|(i, x)| {
                if matches!(x.typ, Typ::Datetime) {
                    Some(i)
                } else {
                    None
                }
            })
            .map(|x| return format!("args[{x}] = args[{x}] ? new Date(args[{x}]) : undefined"))
            .join("\n");

        let spread = args.into_iter().map(|x| x.name).join(",");
        // logs.push_str(format!("infer args: {:?}\n", start.elapsed().as_micros()).as_str());
        // we cannot use Bun.read and Bun.write because it results in an EBADF error on cloud
        let wrapper_content: String = format!(
            r#"
import {{ main }} from "./main.ts";

const fs = require('fs/promises');

const args = await fs.readFile('args.json', {{ encoding: 'utf8' }}).then(JSON.parse)
    .then(({{ {spread} }}) => [ {spread} ])

BigInt.prototype.toJSON = function () {{
    return this.toString();
}};

{dates}
async function run() {{
    let res: any = await main(...args);
    const res_json = JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value);
    await fs.writeFile("result.json", res_json);
    process.exit(0);
}}
run().catch(async (e) => {{
    await fs.writeFile("result.json", JSON.stringify({{ message: e.message, name: e.name, stack: e.stack }}));
    process.exit(1);
}});
    "#,
        );
        write_file(job_dir, "wrapper.ts", &wrapper_content).await?;
        Ok(()) as error::Result<()>
    };

    let reserved_variables_args_out_f = async {
        let client = client.get_authed().await;
        let args_and_out_f = async {
            create_args_and_out_file(&client, job, job_dir).await?;
            Ok(()) as Result<()>
        };
        let reserved_variables_f = async {
            let vars = get_reserved_variables(job, &client.token, db).await?;
            Ok(vars) as Result<HashMap<String, String>>
        };
        let (_, reserved_variables) = tokio::try_join!(args_and_out_f, reserved_variables_f)?;
        Ok(reserved_variables) as error::Result<HashMap<String, String>>
    };

    let write_loader_f = async {
        write_file(
            &job_dir,
            "loader.bun.ts",
            &format!(
                r#"
import {{ plugin }} from "bun";

{}

plugin(p)
"#,
                RELATIVE_BUN_LOADER
                    .replace("W_ID", &job.workspace_id)
                    .replace("BASE_INTERNAL_URL", base_internal_url)
                    .replace("TOKEN", &client.get_token().await)
                    .replace("CURRENT_PATH", job.script_path())
            ),
        )
        .await?;
        Ok(()) as error::Result<()>
    };

    let (reserved_variables, _, _, _) = tokio::try_join!(
        reserved_variables_args_out_f,
        write_wrapper_f,
        logs_f,
        write_loader_f
    )?;

    //do not cache local dependencies
    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_BUN_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", BUN_CACHE_DIR)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )
        .await?;

        Command::new(NSJAIL_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(common_bun_proc_envs)
            .env("PATH", PATH_ENV.as_str())
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                &BUN_PATH,
                "run",
                "-i",
                "--prefer-offline",
                "-r",
                "/tmp/bun/loader.bun.ts",
                "/tmp/bun/wrapper.ts",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        let script_path = format!("{job_dir}/wrapper.ts");
        let args = vec![
            "run",
            "-i",
            "--prefer-offline",
            "-r",
            "./loader.bun.ts",
            &script_path,
        ];
        Command::new(&*BUN_PATH)
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(common_bun_proc_envs)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };

    handle_child(
        &job.id,
        db,
        logs,
        child,
        false,
        worker_name,
        &job.workspace_id,
        "bun run",
        job.timeout,
    )
    .await?;
    read_result(job_dir).await
}

pub fn get_common_bun_proc_envs(base_internal_url: &str) -> HashMap<String, String> {
    let mut deno_envs: HashMap<String, String> = HashMap::from([
        (String::from("PATH"), PATH_ENV.clone()),
        (String::from("TZ"), TZ_ENV.clone()),
        (String::from("DISABLE_COLORS"), "0".to_string()),
        (String::from("DO_NOT_TRACK"), "1".to_string()),
        (
            String::from("BASE_URL"),
            base_internal_url
                .to_string()
                .replace("localhost", "127.0.0.1"),
        ),
        (
            String::from("BUN_INSTALL_CACHE_DIR"),
            BUN_CACHE_DIR.to_string(),
        ),
    ]);

    if let Some(ref s) = *NPM_CONFIG_REGISTRY {
        deno_envs.insert(String::from("NPM_CONFIG_REGISTRY"), s.clone());
    }
    return deno_envs;
}

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
    job_completed_tx: Sender<JobCompleted>,
    mut jobs_rx: Receiver<QueuedJob>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<()> {
    let mut logs = "".to_string();
    let _ = write_file(job_dir, "main.ts", inner_content).await?;
    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url);
    let context = variables::get_reserved_variables(
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
    )
    .to_vec();
    let context_envs = build_envs_map(context);
    if let Some(reqs) = requirements_o {
        let splitted = reqs.split(BUN_LOCKB_SPLIT).collect::<Vec<&str>>();
        if splitted.len() != 2 {
            return Err(error::Error::ExecutionErr(
                format!("Invalid requirements, expectd to find //bun.lockb split pattern in reqs. Found: |{reqs}|")
            ));
        }
        let _ = write_file(job_dir, "package.json", &splitted[0]).await?;
        let lockb = splitted[1];
        if lockb != EMPTY_FILE {
            let _ = write_file_binary(
                job_dir,
                "bun.lockb",
                &base64::engine::general_purpose::STANDARD
                    .decode(&splitted[1])
                    .map_err(|_| {
                        error::Error::InternalErr("Could not decode bun.lockb".to_string())
                    })?,
            )
            .await?;
        }
        install_lockfile(
            &mut logs,
            &Uuid::nil(),
            &w_id,
            db,
            job_dir,
            worker_name,
            common_bun_proc_envs.clone(),
        )
        .await?;
    } else if !*DISABLE_NSJAIL {
        logs.push_str("\n\n--- BUN INSTALL ---\n");
        let _ = gen_lockfile(
            &mut logs,
            &Uuid::nil(),
            &w_id,
            db,
            token,
            &script_path,
            job_dir,
            base_internal_url,
            worker_name,
            false,
        )
        .await?;
    }

    {
        // let mut start = Instant::now();
        let args = windmill_parser_ts::parse_deno_signature(inner_content, true)?.args;
        let dates = args
            .iter()
            .enumerate()
            .filter_map(|(i, x)| {
                if matches!(x.typ, Typ::Datetime) {
                    Some(i)
                } else {
                    None
                }
            })
            .map(|x| return format!("args[{x}] = args[{x}] ? new Date(args[{x}]) : undefined"))
            .join("\n");

        let spread = args.into_iter().map(|x| x.name).join(",");
        // logs.push_str(format!("infer args: {:?}\n", start.elapsed().as_micros()).as_str());
        // we cannot use Bun.read and Bun.write because it results in an EBADF error on cloud
        let wrapper_content: String = format!(
            r#"
import {{ main }} from "./main.ts";

BigInt.prototype.toJSON = function () {{
    return this.toString();
}};

{dates}

let stdout = Bun.stdout.writer();
// let stdout = Bun.file("output.txt").writer();
stdout.write('start\n'); 

for await (const chunk of Bun.stdin.stream()) {{
    const lines = Buffer.from(chunk).toString();
    let exit = false;
    for (const line of lines.trim().split("\n")) {{
        // stdout.write('s: ' + line + 'EE\n'); 
        if (line === "end") {{
            exit = true;
            break;
        }}
        try {{
            let {{ {spread} }} = JSON.parse(line) 
            let res: any = await main(...[ {spread} ]);
            stdout.write(JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value) + '\n');
        }} catch (e) {{
            stdout.write(JSON.stringify({{ error: {{ message: e.message, name: e.name, stack: e.stack, line: line }}}}) + '\n');
        }}
        stdout.flush();
    }}
    if (exit) {{
        break;
    }}
}}
"#,
        );
        write_file(job_dir, "wrapper.ts", &wrapper_content).await?;
    }

    let reserved_variables = windmill_common::variables::get_reserved_variables(
        w_id,
        token,
        "dedicated_worker",
        "dedicated_worker",
        Uuid::nil().to_string().as_str(),
        "dedicted_worker",
        Some(script_path.to_string()),
        None,
        None,
        None,
        None,
    );

    let _ = write_file(
        &job_dir,
        "loader.bun.ts",
        &format!(
            r#"
import {{ plugin }} from "bun";

{}

plugin(p)
"#,
            RELATIVE_BUN_LOADER
                .replace("W_ID", &w_id)
                .replace("BASE_INTERNAL_URL", base_internal_url)
                .replace("TOKEN", token)
                .replace("CURRENT_PATH", script_path)
        ),
    )
    .await?;

    //do not cache local dependencies
    let mut child = {
        let script_path = format!("{job_dir}/wrapper.ts");
        let args = vec![
            "run",
            "-i",
            "--prefer-offline",
            "-r",
            "./loader.bun.ts",
            &script_path,
        ];
        Command::new(&*BUN_PATH)
            .current_dir(job_dir)
            .env_clear()
            .envs(context_envs)
            .envs(envs)
            .envs(
                reserved_variables
                    .iter()
                    .map(|x| (x.name.clone(), x.value.clone()))
                    .collect::<Vec<_>>(),
            )
            .envs(common_bun_proc_envs)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };

    let stdout = child
        .stdout
        .take()
        .expect("child did not have a handle to stdout");

    let mut reader = BufReader::new(stdout).lines();

    let mut stdin = child
        .stdin
        .take()
        .expect("child did not have a handle to stdin");

    // Ensure the child process is spawned in the runtime so it can
    // make progress on its own while we await for any output.
    let child = tokio::spawn(async move {
        let status = child
            .wait()
            .await
            .expect("child process encountered an error");

        println!("child status was: {}", status);
    });

    let mut jobs = VecDeque::with_capacity(MAX_BUFFERED_DEDICATED_JOBS);
    // let mut i = 0;
    // let mut j = 0;
    let mut alive = true;
    loop {
        tokio::select! {
            biased;
            _ = killpill_rx.recv(), if alive => {
                println!("received killpill for dedicated worker");
                alive = false;
                if let Err(e) = write_stdin(&mut stdin, "end").await {
                    tracing::info!("Could not write end message to stdin: {e:?}")
                }
            },
            line = reader.next_line() => {
                // j += 1;

                if let Some(line) = line.expect("line is ok") {
                    if line == "start" {
                        tracing::info!("dedicated worker process started");
                        continue;
                    }
                    tracing::debug!("processed job");

                    let result = serde_json::from_str(&line).expect("json is ok");
                    let job: QueuedJob = jobs.pop_front().expect("pop");
                    job_completed_tx.send(JobCompleted { job , result, logs: "".to_string(), success: true, cached_res_path: None, token: token.to_string() }).await.unwrap();
                } else {
                    tracing::info!("dedicated worker process exited");
                    break;
                }
            }
            job = jobs_rx.recv(), if alive && jobs.len() < MAX_BUFFERED_DEDICATED_JOBS => {
                // i += 1;
                if let Some(job) = job {
                    tracing::debug!("received job");
                    jobs.push_back(job.clone());
                    // write_stdin(&mut stdin, &serde_json::to_string(&job.args.unwrap_or_else(|| serde_json::json!({"x": job.id}))).expect("serialize")).await?;
                    write_stdin(&mut stdin, &serde_json::to_string(&job.args.unwrap_or_else(|| serde_json::json!({}))).expect("serialize")).await?;
                    stdin.flush().await.context("stdin flush")?;
                } else {
                    tracing::debug!("job channel closed");
                    alive = false;
                    if let Err(e) = write_stdin(&mut stdin, "end").await {
                        tracing::error!("Could not write end message to stdin: {e:?}")
                    }
                }
            }
        }
    }

    child
        .await
        .map_err(|e| anyhow::anyhow!("child process encountered an error: {e}"))?;
    tracing::info!("dedicated worker child process exited successfully");
    Ok(())
}

#[cfg(feature = "enterprise")]
async fn write_stdin(stdin: &mut tokio::process::ChildStdin, s: &str) -> error::Result<()> {
    let _ = &stdin.write_all(format!("{s}\n").as_bytes()).await?;
    stdin.flush().await.context("stdin flush")?;
    Ok(())
}
