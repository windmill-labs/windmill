use std::{collections::HashMap, process::Stdio};

use itertools::Itertools;
use serde_json::value::RawValue;
use uuid::Uuid;
use windmill_queue::CanceledBy;

use crate::{
    common::{
        create_args_and_out_file, get_reserved_variables, handle_child, parse_npm_config,
        read_result, set_logs, start_child_process, write_file,
    },
    AuthedClientBackgroundTask, DENO_CACHE_DIR, DENO_PATH, DISABLE_NSJAIL, HOME_ENV,
    NPM_CONFIG_REGISTRY, PATH_ENV, TZ_ENV,
};
use tokio::{fs::File, io::AsyncReadExt, process::Command};
use windmill_common::{error::Result, BASE_URL};
use windmill_common::{
    error::{self},
    jobs::QueuedJob,
};
use windmill_parser::Typ;

lazy_static::lazy_static! {

    static ref DENO_FLAGS: Option<Vec<String>> = std::env::var("DENO_FLAGS")
        .ok()
        .map(|x| x.split(' ').map(|x| x.to_string()).collect());

    static ref DENO_EXTRA_IMPORT_MAP: String = std::env::var("DENO_EXTRA_IMPORT_MAP")
        .ok()
        .map(|x| x.split(',').map(|x| {
            let mut splitted = x.split("=");
            let key = splitted.next().unwrap();
            let value = splitted.next().unwrap();
            format!(",\n \"{key}\": \"{value}\"")
        }).join("\n")).unwrap_or_else(|| String::new());

    static ref DENO_AUTH_TOKENS: String = std::env::var("DENO_AUTH_TOKENS")
        .ok()
        .map(|x| format!(";{x}"))
        .unwrap_or_else(|| String::new());

    static ref DENO_CERT: String = std::env::var("DENO_CERT").ok().unwrap_or_else(|| String::new());
    static ref DENO_TLS_CA_STORE: String = std::env::var("DENO_TLS_CA_STORE").ok().unwrap_or_else(|| String::new());

}
async fn get_common_deno_proc_envs(
    token: &str,
    base_internal_url: &str,
) -> HashMap<String, String> {
    let hostname = BASE_URL.read().await.clone();
    let hostname_base = hostname.split("://").last().unwrap_or("localhost");
    let hostname_internal = base_internal_url.split("://").last().unwrap_or("localhost");
    let deno_auth_tokens_base = DENO_AUTH_TOKENS.as_str();
    let deno_auth_tokens =
        format!("{token}@{hostname_base};{token}@{hostname_internal}{deno_auth_tokens_base}",);

    let mut deno_envs: HashMap<String, String> = HashMap::from([
        (String::from("PATH"), PATH_ENV.clone()),
        (String::from("HOME"), HOME_ENV.clone()),
        (String::from("TZ"), TZ_ENV.clone()),
        (String::from("RUST_LOG"), "info".to_string()),
        (String::from("DENO_DIR"), DENO_CACHE_DIR.to_string()),
        (String::from("DENO_AUTH_TOKENS"), deno_auth_tokens),
        (
            String::from("BASE_INTERNAL_URL"),
            base_internal_url.to_string(),
        ),
    ]);

    if let Some(ref s) = NPM_CONFIG_REGISTRY.read().await.clone() {
        let (url, _token_opt) = parse_npm_config(s);
        deno_envs.insert(String::from("NPM_CONFIG_REGISTRY"), url);
    }
    if DENO_CERT.len() > 0 {
        deno_envs.insert(String::from("DENO_CERT"), DENO_CERT.clone());
    }
    if DENO_TLS_CA_STORE.len() > 0 {
        deno_envs.insert(String::from("DENO_TLS_CA_STORE"), DENO_TLS_CA_STORE.clone());
    }
    return deno_envs;
}

pub async fn generate_deno_lock(
    job_id: &Uuid,
    code: &str,
    logs: &mut String,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    worker_name: &str,
    base_internal_url: &str,
) -> error::Result<String> {
    let _ = write_file(job_dir, "main.ts", code).await?;

    let import_map_path = format!("{job_dir}/import_map.json");
    let import_map = format!(
        r#"{{
        "imports": {{
            "/": "{base_internal_url}/api/scripts_u/empty_ts/"
        }}
      }}"#,
    );
    write_file(job_dir, "import_map.json", &import_map).await?;
    write_file(job_dir, "empty.ts", "").await?;

    let mut deno_envs = HashMap::new();
    if let Some(ref s) = NPM_CONFIG_REGISTRY.read().await.clone() {
        let (url, _token_opt) = parse_npm_config(s);
        deno_envs.insert(String::from("NPM_CONFIG_REGISTRY"), url);
    }
    let mut child_cmd = Command::new(DENO_PATH.as_str());
    child_cmd
        .current_dir(job_dir)
        .args(vec![
            "cache",
            "--unstable-unsafe-proto",
            "--unstable-bare-node-builtins",
            "--unstable-webgpu",
            "--unstable-ffi",
            "--unstable-fs",
            "--unstable-worker-options",
            "--unstable-http",
            "--lock=lock.json",
            "--lock-write",
            "--import-map",
            &import_map_path,
            "main.ts",
        ])
        .envs(deno_envs)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child_process = start_child_process(child_cmd, DENO_PATH.as_str()).await?;

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
        "deno cache",
        None,
        false,
    )
    .await?;

    let path_lock = format!("{job_dir}/lock.json");
    let mut file = File::open(path_lock).await?;
    let mut req_content = "".to_string();
    file.read_to_string(&mut req_content).await?;
    Ok(req_content)
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_deno_job(
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
) -> error::Result<Box<RawValue>> {
    // let mut start = Instant::now();
    logs.push_str("\n\n--- DENO CODE EXECUTION ---\n");

    let logs_to_set = logs.clone();
    let id = job.id.clone();
    let db2 = db.clone();

    let set_logs_f = async {
        set_logs(&logs_to_set, &id, &db2).await;
        Ok(()) as error::Result<()>
    };

    let write_main_f = write_file(job_dir, "main.ts", inner_content);

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
        let wrapper_content: String = format!(
            r#"
import {{ main }} from "./main.ts";

const args = await Deno.readTextFile("args.json")
    .then(JSON.parse)
    .then(({{ {spread} }}) => [ {spread} ])

BigInt.prototype.toJSON = function () {{
    return this.toString();
}};

{dates}
async function run() {{
    let res: any = await main(...args);
    const res_json = JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value);
    await Deno.writeTextFile("result.json", res_json);
    Deno.exit(0);
}}
try {{
    await run();
}} catch(e) {{
    let err = {{ message: e.message, name: e.name, stack: e.stack }};
    let step_id = Deno.env.get("WM_FLOW_STEP_ID");
    if (step_id) {{
        err["step_id"] = step_id;
    }}
    await Deno.writeTextFile("result.json", JSON.stringify(err));
    Deno.exit(1);
}}
    "#,
        );
        write_file(job_dir, "wrapper.ts", &wrapper_content).await?;
        Ok(()) as error::Result<()>
    };

    let write_import_map_f = build_import_map(
        &job.workspace_id,
        job.script_path(),
        base_internal_url,
        job_dir,
    );

    let reserved_variables_args_out_f = async {
        let args_and_out_f = async {
            create_args_and_out_file(&client, job, job_dir, db).await?;
            Ok(()) as Result<()>
        };
        let reserved_variables_f = async {
            let client = client.get_authed().await;
            let vars = get_reserved_variables(job, &client.token, db).await?;
            Ok((vars, client.token)) as Result<(HashMap<String, String>, String)>
        };
        let (_, reserved_variables) = tokio::try_join!(args_and_out_f, reserved_variables_f)?;
        Ok(reserved_variables) as error::Result<(HashMap<String, String>, String)>
    };

    let (_, (reserved_variables, token), _, _, _) = tokio::try_join!(
        set_logs_f,
        reserved_variables_args_out_f,
        write_main_f,
        write_wrapper_f,
        write_import_map_f
    )?;

    let mut common_deno_proc_envs = get_common_deno_proc_envs(&token, base_internal_url).await;
    if !*DISABLE_NSJAIL {
        common_deno_proc_envs.insert("HOME".to_string(), job_dir.to_string());
    }

    //do not cache local dependencies
    let child = {
        let reload = format!("--reload={base_internal_url}");
        let script_path = format!("{job_dir}/wrapper.ts");
        let import_map_path = format!("{job_dir}/import_map.json");
        let mut args = Vec::with_capacity(12);
        args.push("run");
        args.push("--no-check");
        args.push("--import-map");
        args.push(&import_map_path);
        args.push(&reload);
        args.push("--unstable-unsafe-proto");
        args.push("--unstable-bare-node-builtins");
        args.push("--unstable-webgpu");
        args.push("--unstable-ffi");
        args.push("--unstable-fs");
        args.push("--unstable-worker-options");
        args.push("--unstable-http");
        if let Some(reqs) = requirements_o {
            if !reqs.is_empty() {
                let _ = write_file(job_dir, "lock.json", &reqs).await?;
                args.push("--lock=lock.json");
                args.push("--lock-write");
            }
        }
        let allow_read = format!(
            "--allow-read=./,/tmp/windmill/cache/deno/,{}",
            DENO_PATH.as_str()
        );
        if let Some(deno_flags) = DENO_FLAGS.as_ref() {
            for flag in deno_flags {
                args.push(flag);
            }
        } else if !*DISABLE_NSJAIL {
            args.push("--allow-net");
            args.push("--allow-sys");
            args.push("--allow-hrtime");
            args.push(allow_read.as_str());
            args.push("--allow-write=./");
            args.push("--allow-env");
            args.push("--allow-run=git,/usr/bin/chromium");
        } else {
            args.push("-A");
        }
        args.push(&script_path);
        let mut deno_cmd = Command::new(DENO_PATH.as_str());
        deno_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(common_deno_proc_envs)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(deno_cmd, DENO_PATH.as_str()).await?
    };
    // logs.push_str(format!("prepare: {:?}\n", start.elapsed().as_micros()).as_str());
    // start = Instant::now();
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
        "deno run",
        job.timeout,
        false,
    )
    .await?;
    // logs.push_str(format!("execute: {:?}\n", start.elapsed().as_millis()).as_str());
    if let Err(e) = tokio::fs::remove_dir_all(format!("{DENO_CACHE_DIR}/gen/file/{job_dir}")).await
    {
        tracing::error!("failed to remove deno gen tmp cache dir: {}", e);
    }
    read_result(job_dir).await
}

async fn build_import_map(
    w_id: &str,
    script_path: &str,
    base_internal_url: &str,
    job_dir: &str,
) -> error::Result<()> {
    let script_path_split = script_path.split("/");
    let script_path_parts_len = script_path_split.clone().count();
    let mut relative_mounts = "".to_string();
    for c in 0..script_path_parts_len {
        relative_mounts += ",\n          ";
        relative_mounts += &format!(
            "\"./{}\": \"{base_internal_url}/api/w/{w_id}/scripts/raw/p/{}{}\"",
            (0..c).map(|_| "../").join(""),
            &script_path_split
                .clone()
                .take(script_path_parts_len - c - 1)
                .join("/"),
            if c == script_path_parts_len - 1 {
                ""
            } else {
                "/"
            },
        );
    }
    let extra_import_map = DENO_EXTRA_IMPORT_MAP.as_str();
    let import_map = format!(
        r#"{{
            "imports": {{
              "{base_internal_url}/api/w/{w_id}/scripts/raw/p/": "{base_internal_url}/api/w/{w_id}/scripts/raw/p/",
              "{base_internal_url}": "{base_internal_url}/api/w/{w_id}/scripts/raw/p/",
              "/": "{base_internal_url}/api/w/{w_id}/scripts/raw/p/",
              "./wrapper.ts": "./wrapper.ts",
              "./main.ts": "./main.ts"{relative_mounts}
              {extra_import_map}
            }}
          }}"#,
    );
    write_file(job_dir, "import_map.json", &import_map).await?;
    Ok(()) as error::Result<()>
}

#[cfg(feature = "enterprise")]
use crate::{dedicated_worker::handle_dedicated_process, JobCompletedSender};
#[cfg(feature = "enterprise")]
use std::sync::Arc;
#[cfg(feature = "enterprise")]
use tokio::sync::mpsc::Receiver;

#[cfg(feature = "enterprise")]
pub async fn start_worker(
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
    use windmill_common::variables;

    use crate::common::build_envs_map;

    let _ = write_file(job_dir, "main.ts", inner_content).await?;
    let common_deno_proc_envs = get_common_deno_proc_envs(&token, base_internal_url).await;

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
        None,
        None,
    )
    .await;
    let context_envs = build_envs_map(context.to_vec()).await;

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

console.log('start\n'); 

const decoder = new TextDecoder();
for await (const chunk of Deno.stdin.readable) {{
    const lines = decoder.decode(chunk);
    let exit = false;
    for (const line of lines.trim().split("\n")) {{
        if (line === "end") {{
            exit = true;
            break;
        }}
        try {{
            let {{ {spread} }} = JSON.parse(line) 
            let res: any = await main(...[ {spread} ]);
            console.log("wm_res[success]:" + JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value) + '\n');
        }} catch (e) {{
            console.log("wm_res[error]:" + JSON.stringify({{ message: e.message, name: e.name, stack: e.stack, line: line }}) + '\n');
        }}
    }}
    if (exit) {{
        break;
    }}
}}
"#,
        );
        write_file(job_dir, "wrapper.ts", &wrapper_content).await?;
    }

    build_import_map(w_id, script_path, base_internal_url, job_dir).await?;

    handle_dedicated_process(
        &*DENO_PATH,
        job_dir,
        context_envs,
        envs,
        context,
        common_deno_proc_envs,
        vec![
            "run",
            "--no-check",
            "--import-map",
            &format!("{job_dir}/import_map.json"),
            &format!("--reload={base_internal_url}"),
            "--unstable-unsafe-proto",
            "--unstable-bare-node-builtins",
            "--unstable-webgpu",
            "--unstable-ffi",
            "--unstable-fs",
            "--unstable-worker-options",
            "--unstable-http",
            "-A",
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
