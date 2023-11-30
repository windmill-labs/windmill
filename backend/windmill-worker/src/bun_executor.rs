use std::{collections::HashMap, process::Stdio};

use base64::Engine;
use itertools::Itertools;
use regex::Regex;
use serde_json::value::RawValue;
use uuid::Uuid;
use windmill_queue::CanceledBy;

#[cfg(feature = "enterprise")]
use crate::common::build_envs_map;

use crate::{
    common::{
        create_args_and_out_file, get_reserved_variables, handle_child, read_result, set_logs,
        start_child_process, write_file, write_file_binary,
    },
    AuthedClientBackgroundTask, BUN_CACHE_DIR, BUN_PATH, DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV,
    NSJAIL_PATH, PATH_ENV, TZ_ENV,
};

use tokio::{
    fs::{remove_dir_all, File},
    process::Command,
};

use tokio::io::AsyncReadExt;

#[cfg(feature = "enterprise")]
use tokio::sync::mpsc::Receiver;

#[cfg(feature = "enterprise")]
use windmill_common::variables;

use windmill_common::{
    error::{self, to_anyhow, Result},
    jobs::QueuedJob,
};
use windmill_parser::Typ;

const RELATIVE_BUN_LOADER: &str = include_str!("../loader.bun.ts");

const RELATIVE_BUN_BUILDER: &str = include_str!("../loader_builder.bun.ts");

const NSJAIL_CONFIG_RUN_BUN_CONTENT: &str = include_str!("../nsjail/run.bun.config.proto");

pub const BUN_LOCKB_SPLIT: &str = "\n//bun.lockb\n";
pub const EMPTY_FILE: &str = "<empty>";

lazy_static::lazy_static! {
    pub static ref TRUSTED_DEP: Regex = Regex::new(r"//\s?trustedDependencies:(.*)\n").unwrap();
}

pub async fn gen_lockfile(
    logs: &mut String,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_id: &Uuid,
    w_id: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    token: &str,
    script_path: &str,
    job_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    export_pkg: bool,
    trusted_deps: Vec<String>,
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
        get_common_bun_proc_envs(&base_internal_url).await;

    let mut child_cmd = Command::new(&*BUN_PATH);
    child_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(common_bun_proc_envs.clone())
        .args(vec!["run", "build.ts"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child_process = start_child_process(child_cmd, &*BUN_PATH).await?;

    handle_child(
        job_id,
        db,
        logs,
        mem_peak,
        canceled_by,
        child_process,
        false,
        worker_name,
        w_id,
        "bun build",
        None,
        false,
    )
    .await?;

    if trusted_deps.len() > 0 {
        logs.push_str(&format!(
            "\ndetected trustedDependencies: {}\n",
            trusted_deps.join(", ")
        ));
        let mut content = "".to_string();
        {
            let mut file = File::open(format!("{job_dir}/package.json")).await?;
            file.read_to_string(&mut content).await?;
        }
        let mut value =
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&content)
                .map_err(to_anyhow)?;
        value.insert(
            "trustedDependencies".to_string(),
            serde_json::json!(trusted_deps),
        );
        write_file(
            job_dir,
            "package.json",
            &serde_json::to_string(&value).map_err(to_anyhow)?,
        )
        .await?;

        let mut content = "".to_string();
        {
            let mut file = File::open(format!("{job_dir}/package.json")).await?;
            file.read_to_string(&mut content).await?;
        }
    }

    install_lockfile(
        logs,
        mem_peak,
        canceled_by,
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
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_id: &Uuid,
    w_id: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    job_dir: &str,
    worker_name: &str,
    common_bun_proc_envs: HashMap<String, String>,
) -> Result<()> {
    let mut child_cmd = Command::new(&*BUN_PATH);
    child_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(common_bun_proc_envs)
        .args(vec!["install"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child_process = start_child_process(child_cmd, &*BUN_PATH).await?;

    handle_child(
        job_id,
        db,
        logs,
        mem_peak,
        canceled_by,
        child_process,
        false,
        worker_name,
        w_id,
        "bun install",
        None,
        false,
    )
    .await?;
    Ok(())
}

pub fn get_trusted_deps(code: &str) -> Vec<String> {
    // postinstall not allowed with nsjail
    if !*DISABLE_NSJAIL {
        return vec![];
    }
    TRUSTED_DEP
        .captures(code)
        .map(|x| {
            x[1].to_string()
                .trim()
                .split(',')
                .map(|y| y.trim().to_string())
                .collect_vec()
        })
        .unwrap_or_default()
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_bun_job(
    requirements_o: Option<String>,
    logs: &mut String,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    job_dir: &str,
    inner_content: &String,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    shared_mount: &str,
) -> error::Result<Box<RawValue>> {
    let _ = write_file(job_dir, "main.ts", inner_content).await?;

    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url).await;

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
            let has_trusted_deps = &splitted[0].contains("trustedDependencies");

            if !has_trusted_deps {
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
                mem_peak,
                canceled_by,
                &job.id,
                &job.workspace_id,
                db,
                job_dir,
                worker_name,
                common_bun_proc_envs.clone(),
            )
            .await?;
            if !has_trusted_deps {
                remove_dir_all(format!("{}/node_modules", job_dir)).await?;
            }
        }
    } else {
        // TODO: remove once bun implement a reasonable set of trusted deps
        let trusted_deps = get_trusted_deps(inner_content);
        let empty_trusted_deps = trusted_deps.len() == 0;
        let has_custom_config_registry = common_bun_proc_envs.contains_key("NPM_CONFIG_REGISTRY");
        if !*DISABLE_NSJAIL || !empty_trusted_deps || has_custom_config_registry {
            logs.push_str("\n\n--- BUN INSTALL ---\n");
            set_logs(&logs, &job.id, &db).await;
            let _ = gen_lockfile(
                logs,
                mem_peak,
                canceled_by,
                &job.id,
                &job.workspace_id,
                db,
                &client.get_token().await,
                &job.script_path(),
                job_dir,
                base_internal_url,
                worker_name,
                false,
                trusted_deps,
            )
            .await?;
        }

        if empty_trusted_deps && !has_custom_config_registry {
            let node_modules_path = format!("{}/node_modules", job_dir);
            let node_modules_exists = tokio::fs::metadata(&node_modules_path).await.is_ok();
            if node_modules_exists {
                remove_dir_all(&node_modules_path).await?;
            }
        }
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
        let args_and_out_f = async {
            create_args_and_out_file(&client, job, job_dir, db).await?;
            Ok(()) as Result<()>
        };
        let reserved_variables_f = async {
            let client = client.get_authed().await;
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

        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
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
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
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
        let mut bun_cmd = Command::new(&*BUN_PATH);
        bun_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(common_bun_proc_envs)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(bun_cmd, &*BUN_PATH).await?
    };

    handle_child(
        &job.id,
        db,
        logs,
        mem_peak,
        canceled_by,
        child,
        false,
        worker_name,
        &job.workspace_id,
        "bun run",
        job.timeout,
        false,
    )
    .await?;
    read_result(job_dir).await
}

pub async fn get_common_bun_proc_envs(base_internal_url: &str) -> HashMap<String, String> {
    let bun_envs: HashMap<String, String> = HashMap::from([
        (String::from("PATH"), PATH_ENV.clone()),
        (String::from("HOME"), HOME_ENV.clone()),
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
    return bun_envs;
}

#[cfg(feature = "enterprise")]
use crate::{dedicated_worker::handle_dedicated_process, JobCompletedSender};
#[cfg(feature = "enterprise")]
use std::sync::Arc;

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
    jobs_rx: Receiver<Arc<QueuedJob>>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<()> {
    let mut logs = "".to_string();
    let mut mem_peak: i32 = 0;
    let mut canceled_by: Option<CanceledBy> = None;
    let _ = write_file(job_dir, "main.ts", inner_content).await?;
    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url).await;
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
    .await;
    let context_envs = build_envs_map(context.to_vec());
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
            let has_trusted_deps = &splitted[0].contains("trustedDependencies");

            if !has_trusted_deps {
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
                &mut mem_peak,
                &mut canceled_by,
                &Uuid::nil(),
                &w_id,
                db,
                job_dir,
                worker_name,
                common_bun_proc_envs.clone(),
            )
            .await?;
            if !has_trusted_deps {
                remove_dir_all(format!("{}/node_modules", job_dir)).await?;
            }
            tracing::info!("dedicated worker requirements installed: {reqs}");
        }
    } else if !*DISABLE_NSJAIL {
        let trusted_deps = get_trusted_deps(inner_content);
        logs.push_str("\n\n--- BUN INSTALL ---\n");
        let _ = gen_lockfile(
            &mut logs,
            &mut mem_peak,
            &mut canceled_by,
            &Uuid::nil(),
            &w_id,
            db,
            token,
            &script_path,
            job_dir,
            base_internal_url,
            worker_name,
            false,
            trusted_deps,
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
stdout.write('start\n'); 

for await (const chunk of Bun.stdin.stream()) {{
    const lines = Buffer.from(chunk).toString();
    let exit = false;
    for (const line of lines.trim().split("\n")) {{
        if (line === "end") {{
            exit = true;
            break;
        }}
        try {{
            let {{ {spread} }} = JSON.parse(line) 
            let res: any = await main(...[ {spread} ]);
            stdout.write("wm_res:" + JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value) + '\n');
        }} catch (e) {{
            stdout.write("wm_res:" + JSON.stringify({{ error: {{ message: e.message, name: e.name, stack: e.stack, line: line }}}}) + '\n');
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

    handle_dedicated_process(
        &*BUN_PATH,
        job_dir,
        context_envs,
        envs,
        context,
        common_bun_proc_envs,
        vec![
            "run",
            "-i",
            "--prefer-offline",
            "-r",
            "./loader.bun.ts",
            &format!("{job_dir}/wrapper.ts"),
        ],
        killpill_rx,
        job_completed_tx,
        token,
        jobs_rx,
        worker_name,
    )
    .await
}
