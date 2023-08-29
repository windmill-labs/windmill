use std::{collections::HashMap, process::Stdio};

use itertools::Itertools;
use uuid::Uuid;

use crate::{
    common::{
        create_args_and_out_file, get_reserved_variables, handle_child, read_result, set_logs,
        write_file,
    },
    AuthedClientBackgroundTask, DENO_CACHE_DIR, DENO_PATH, DISABLE_NSJAIL, NPM_CONFIG_REGISTRY,
    PATH_ENV,
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


}
fn get_common_deno_proc_envs(token: &str, base_internal_url: &str) -> HashMap<String, String> {
    let hostname_base = BASE_URL.split("://").last().unwrap_or("localhost");
    let hostname_internal = base_internal_url.split("://").last().unwrap_or("localhost");
    let deno_auth_tokens_base = DENO_AUTH_TOKENS.as_str();
    let deno_auth_tokens =
        format!("{token}@{hostname_base};{token}@{hostname_internal}{deno_auth_tokens_base}",);

    let mut deno_envs: HashMap<String, String> = HashMap::from([
        (String::from("PATH"), PATH_ENV.clone()),
        (String::from("DENO_AUTH_TOKENS"), deno_auth_tokens),
        (
            String::from("BASE_INTERNAL_URL"),
            base_internal_url.to_string(),
        ),
    ]);

    if let Some(ref s) = *NPM_CONFIG_REGISTRY {
        deno_envs.insert(String::from("NPM_CONFIG_REGISTRY"), s.clone());
    }
    return deno_envs;
}

pub async fn generate_deno_lock(
    job_id: &Uuid,
    code: &str,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    worker_name: &str,
) -> error::Result<String> {
    let _ = write_file(job_dir, "main.ts", code).await?;

    let child = Command::new(DENO_PATH.as_str())
        .current_dir(job_dir)
        .args(vec![
            "cache",
            "--unstable",
            "--lock=lock.json",
            "--lock-write",
            "main.ts",
        ])
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
        "deno cache",
        None,
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
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    job_dir: &str,
    inner_content: &String,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
) -> error::Result<serde_json::Value> {
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
run().catch(async (e) => {{
    await Deno.writeTextFile("result.json", JSON.stringify({{ message: e.message, name: e.name, stack: e.stack }}));
    Deno.exit(1);
}});
    "#,
        );
        write_file(job_dir, "wrapper.ts", &wrapper_content).await?;
        Ok(()) as error::Result<()>
    };

    let write_import_map_f = async {
        let w_id = job.workspace_id.clone();
        let script_path_split = job.script_path().split("/");
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
    };

    let reserved_variables_args_out_f = async {
        let client = client.get_authed().await;
        let args_and_out_f = async {
            create_args_and_out_file(&client, job, job_dir).await?;
            Ok(()) as Result<()>
        };
        let reserved_variables_f = async {
            let mut vars = get_reserved_variables(job, &client.token, db).await?;
            vars.insert("RUST_LOG".to_string(), "info".to_string());
            Ok(vars) as Result<HashMap<String, String>>
        };
        let (_, reserved_variables) = tokio::try_join!(args_and_out_f, reserved_variables_f)?;
        Ok((reserved_variables, client.token)) as error::Result<(HashMap<String, String>, String)>
    };

    let (_, (reserved_variables, token), _, _, _) = tokio::try_join!(
        set_logs_f,
        reserved_variables_args_out_f,
        write_main_f,
        write_wrapper_f,
        write_import_map_f
    )?;

    let common_deno_proc_envs = get_common_deno_proc_envs(&token, base_internal_url);

    //do not cache local dependencies
    let reload = format!("--reload={base_internal_url}");
    let child = {
        let script_path = format!("{job_dir}/wrapper.ts");
        let import_map_path = format!("{job_dir}/import_map.json");
        let mut args = Vec::with_capacity(12);
        args.push("run");
        args.push("--no-check");
        args.push("--import-map");
        args.push(&import_map_path);
        args.push(&reload);
        args.push("--unstable");
        if let Some(reqs) = requirements_o {
            if !reqs.is_empty() {
                let _ = write_file(job_dir, "lock.json", &reqs).await?;
                args.push("--lock=lock.json");
                args.push("--lock-write");
            }
        }
        if let Some(deno_flags) = DENO_FLAGS.as_ref() {
            for flag in deno_flags {
                args.push(flag);
            }
        } else if !*DISABLE_NSJAIL {
            args.push("--allow-net");
            args.push("--allow-read=./,/tmp/windmill/cache/deno/");
            args.push("--allow-write=./");
            args.push("--allow-env");
        } else {
            args.push("-A");
        }
        args.push(&script_path);
        Command::new(DENO_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(common_deno_proc_envs)
            .env("DENO_DIR", DENO_CACHE_DIR)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    // logs.push_str(format!("prepare: {:?}\n", start.elapsed().as_micros()).as_str());
    // start = Instant::now();
    handle_child(
        &job.id,
        db,
        logs,
        child,
        false,
        worker_name,
        &job.workspace_id,
        "deno run",
        job.timeout,
    )
    .await?;
    // logs.push_str(format!("execute: {:?}\n", start.elapsed().as_millis()).as_str());
    if let Err(e) = tokio::fs::remove_dir_all(format!("{DENO_CACHE_DIR}/gen/file/{job_dir}")).await
    {
        tracing::error!("failed to remove deno gen tmp cache dir: {}", e);
    }
    read_result(job_dir).await
}
