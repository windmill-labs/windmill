use std::{collections::HashMap, process::Stdio};

use itertools::Itertools;
use serde_json::value::RawValue;
use uuid::Uuid;
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::{
    common::{
        build_command_with_isolation, create_args_and_out_file, get_reserved_variables,
        parse_npm_config, read_file, read_result, start_child_process, OccupancyMetrics,
        StreamNotifier,
    },
    get_proxy_envs_for_lang,
    handle_child::handle_child,
    is_sandboxing_enabled, read_ee_registry, DENO_CACHE_DIR, DENO_PATH, HOME_ENV, NPMRC,
    NPM_CONFIG_REGISTRY, PATH_ENV, TZ_ENV,
};
use windmill_common::client::AuthedClient;

use tokio::{fs::File, io::AsyncReadExt, process::Command};
use windmill_common::{error::Result, scripts::ScriptLang, worker::write_file, BASE_URL};
use windmill_common::{
    error::{self},
    worker::Connection,
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
    static ref DISABLE_DENO_LOCK: bool = std::env::var("DISABLE_DENO_LOCK").is_ok();

}

async fn get_common_deno_proc_envs(
    token: &str,
    base_internal_url: &str,
    job_id: &Uuid,
    w_id: &str,
    conn: Option<&Connection>,
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

    let npmrc = if let Some(conn) = conn {
        read_ee_registry(NPMRC.read().await.clone(), "npmrc", job_id, w_id, conn).await
    } else {
        NPMRC.read().await.clone()
    };

    if npmrc.as_ref().map_or(true, |s| s.trim().is_empty()) {
        let registry = if let Some(conn) = conn {
            read_ee_registry(
                NPM_CONFIG_REGISTRY.read().await.clone(),
                "npm registry",
                job_id,
                w_id,
                conn,
            )
            .await
        } else {
            NPM_CONFIG_REGISTRY.read().await.clone()
        };
        if let Some(ref s) = registry {
            let (url, _token_opt) = parse_npm_config(s);
            deno_envs.insert(String::from("NPM_CONFIG_REGISTRY"), url);
        }
    }
    if DENO_CERT.len() > 0 {
        deno_envs.insert(String::from("DENO_CERT"), DENO_CERT.clone());
    }
    if DENO_TLS_CA_STORE.len() > 0 {
        deno_envs.insert(String::from("DENO_TLS_CA_STORE"), DENO_TLS_CA_STORE.clone());
    }

    #[cfg(windows)]
    {
        deno_envs.insert("SystemRoot".to_string(), crate::SYSTEM_ROOT.to_string());
        deno_envs.insert(
            "USERPROFILE".to_string(),
            crate::USERPROFILE_ENV.to_string(),
        );
    }

    // Add proxy envs (including OTEL tracing proxy if enabled for deno)
    for (k, v) in get_proxy_envs_for_lang(&ScriptLang::Deno)
        .await
        .unwrap_or_default()
    {
        deno_envs.insert(k.to_string(), v);
    }

    return deno_envs;
}

pub async fn generate_deno_lock(
    job_id: &Uuid,
    code: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: Option<&Connection>,
    w_id: &str,
    worker_name: &str,
    base_internal_url: &str,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
) -> error::Result<String> {
    let _ = write_file(job_dir, "main.ts", code)?;

    let import_map_path = format!("{job_dir}/import_map.json");
    let import_map = format!(
        r#"{{
        "imports": {{
            "/": "{base_internal_url}/api/scripts_u/empty_ts/"
        }}
      }}"#,
    );
    write_file(job_dir, "import_map.json", &import_map)?;
    write_file(job_dir, "empty.ts", "")?;

    let deno_envs = get_common_deno_proc_envs("", base_internal_url, job_id, w_id, db).await;

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
            "--frozen=false",
            "--allow-import",
            "--import-map",
            &import_map_path,
            "main.ts",
        ])
        .envs(deno_envs)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child_process = start_child_process(child_cmd, DENO_PATH.as_str(), false).await?;

    if let Some(db) = db {
        handle_child(
            job_id,
            db,
            mem_peak,
            canceled_by,
            child_process,
            false,
            worker_name,
            w_id,
            "deno cache",
            None,
            false,
            occupancy_metrics,
            None,
            None,
        )
        .await?;
    } else {
        Box::into_pin(child_process.wait()).await?;
    }

    let path_lock = format!("{job_dir}/lock.json");
    if let Ok(mut file) = File::open(path_lock).await {
        let mut req_content = "".to_string();
        file.read_to_string(&mut req_content).await?;
        Ok(req_content)
    } else {
        Ok("".to_string())
    }
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_deno_job(
    requirements_o: Option<&String>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &MiniPulledJob,
    conn: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    job_dir: &str,
    inner_content: &String,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    new_args: &mut Option<HashMap<String, Box<RawValue>>>,
    occupancy_metrics: &mut OccupancyMetrics,
    has_stream: &mut bool,
) -> error::Result<Box<RawValue>> {
    // let mut start = Instant::now();
    let logs1 = "\n\n--- DENO CODE EXECUTION ---\n".to_string();
    append_logs(&job.id, &job.workspace_id, logs1, conn).await;

    let main_override = job.script_entrypoint_override.as_deref();
    let apply_preprocessor =
        job.flow_step_id.as_deref() != Some("preprocessor") && job.preprocessed == Some(false);

    write_file(job_dir, "main.ts", inner_content)?;

    let write_wrapper_f = async {
        // let mut start = Instant::now();
        let args = windmill_parser_ts::parse_deno_signature(
            inner_content,
            true,
            false,
            main_override.map(ToString::to_string),
        )?
        .args;

        let pre_args = if apply_preprocessor {
            Some(
                windmill_parser_ts::parse_deno_signature(
                    inner_content,
                    true,
                    false,
                    Some("preprocessor".to_string()),
                )?
                .args,
            )
        } else {
            None
        };

        let dates = pre_args
            .as_ref()
            .unwrap_or(&args)
            .iter()
            .filter_map(|x| {
                if matches!(x.typ, Typ::Datetime) {
                    Some(x.name.as_str())
                } else {
                    None
                }
            })
            .map(|x| {
                return format!(r#"args["{x}"] = args["{x}"] ? new Date(args["{x}"]) : undefined"#);
            })
            .join("\n    ");

        let spread = args.into_iter().map(|x| x.name).join(",");
        let main_name = main_override.unwrap_or("main");
        // logs.push_str(format!("infer args: {:?}\n", start.elapsed().as_micros()).as_str());
        let (preprocessor_import, preprocessor) = if let Some(pre_args) = pre_args {
            let pre_spread = pre_args.into_iter().map(|x| x.name).join(",");
            (
                r#"import { preprocessor } from "./main.ts";"#.to_string(),
                format!(
                    r#"if (preprocessor === undefined || typeof preprocessor !== 'function') {{
        throw new Error("preprocessor function is missing");
    }}
    function preArgsObjToArr({{ {pre_spread} }}) {{
        return [ {pre_spread} ];
    }}
    args = await preprocessor(...preArgsObjToArr(args));
    const args_json = JSON.stringify(args ?? null, (key, value) => typeof value === 'undefined' ? null : value);
    await Deno.writeTextFile("args.json", args_json);"#
                ),
            )
        } else {
            ("".to_string(), "".to_string())
        };

        let wrapper_content: String = format!(
            r#"
import {{ {main_name} }} from "./main.ts";
{preprocessor_import}

let args = await Deno.readTextFile("args.json")
    .then(JSON.parse);

function argsObjToArr({{ {spread} }}) {{
    return [ {spread} ];
}}

BigInt.prototype.toJSON = function () {{
    return this.toString();
}};

function isAsyncIterable(obj) {{
    return obj != null && typeof obj[Symbol.asyncIterator] === 'function';
}}

async function run() {{
    {dates}
    {preprocessor}
    const argsArr = argsObjToArr(args);
    if ({main_name} === undefined || typeof {main_name} !== 'function') {{
        throw new Error("{main_name} function is missing");
    }}
    let res: any = await {main_name}(...argsArr);
    if (isAsyncIterable(res)) {{
        for await (const chunk of res) {{
            console.log("WM_STREAM: " + chunk.replace(/\n/g, '\\n'));
        }}
        res = null;
    }}
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
        write_file(job_dir, "wrapper.ts", &wrapper_content)?;
        Ok(()) as error::Result<()>
    };

    let write_import_map_f = build_import_map(
        &job.workspace_id,
        job.runnable_path(),
        base_internal_url,
        job_dir,
    );

    let reserved_variables_args_out_f = async {
        let args_and_out_f = async {
            create_args_and_out_file(&client, job, job_dir, conn).await?;
            Ok(()) as Result<()>
        };
        let reserved_variables_f = async {
            let vars =
                get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;
            Ok(vars) as Result<HashMap<String, String>>
        };
        let (_, reserved_variables) = tokio::try_join!(args_and_out_f, reserved_variables_f)?;
        Ok(reserved_variables) as error::Result<HashMap<String, String>>
    };

    let (reserved_variables, _, _) = tokio::try_join!(
        reserved_variables_args_out_f,
        write_wrapper_f,
        write_import_map_f
    )?;

    let mut common_deno_proc_envs = get_common_deno_proc_envs(
        &client.token,
        base_internal_url,
        &job.id,
        &job.workspace_id,
        Some(conn),
    )
    .await;
    if is_sandboxing_enabled() {
        common_deno_proc_envs.insert("HOME".to_string(), job_dir.to_string());
    }

    let npmrc = read_ee_registry(
        NPMRC.read().await.clone(),
        "npmrc",
        &job.id,
        &job.workspace_id,
        conn,
    )
    .await;
    if let Some(ref npmrc_content) = npmrc {
        if !npmrc_content.trim().is_empty() {
            write_file(job_dir, ".npmrc", npmrc_content)?;
            write_file(job_dir, "deno.json", "{}")?;
        }
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

        if !*DISABLE_DENO_LOCK {
            if let Some(reqs) = requirements_o {
                if !reqs.is_empty() {
                    let _ = write_file(job_dir, "lock.json", &reqs)?;
                    args.push("--lock=lock.json");
                    args.push("--frozen=false");
                }
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
        } else if is_sandboxing_enabled() {
            args.push("--allow-net");
            args.push("--allow-sys");
            args.push(allow_read.as_str());
            args.push("--allow-write=./");
            args.push("--allow-env");
            args.push("--allow-import");
            args.push("--allow-run=git,/usr/bin/chromium");
        } else {
            args.push("-A");
        }
        args.push(&script_path);

        let mut deno_cmd = build_command_with_isolation(
            DENO_PATH.as_str(),
            &args.iter().map(|s| s.as_ref()).collect::<Vec<&str>>(),
        );
        deno_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(common_deno_proc_envs)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(deno_cmd, DENO_PATH.as_str(), false).await?
    };

    let stream_notifier = StreamNotifier::new(conn, job);

    // logs.push_str(format!("prepare: {:?}\n", start.elapsed().as_micros()).as_str());
    // start = Instant::now();
    let handle_result = handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        false,
        worker_name,
        &job.workspace_id,
        "deno run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        stream_notifier,
    )
    .await?;

    *has_stream = handle_result.result_stream.is_some();

    // logs.push_str(format!("execute: {:?}\n", start.elapsed().as_millis()).as_str());
    if let Err(e) = tokio::fs::remove_dir_all(format!("{DENO_CACHE_DIR}/gen/file/{job_dir}")).await
    {
        tracing::error!("failed to remove deno gen tmp cache dir: {}", e);
    }
    if apply_preprocessor {
        let args = read_file(&format!("{job_dir}/args.json"))
            .await
            .map_err(|e| {
                error::Error::internal_err(format!(
                    "error while reading args from preprocessing: {e:#}"
                ))
            })?;
        let args: HashMap<String, Box<RawValue>> =
            serde_json::from_str(args.get()).map_err(|e| {
                error::Error::internal_err(format!(
                    "error while deserializing args from preprocessing: {e:#}"
                ))
            })?;
        *new_args = Some(args.clone());
    }
    read_result(job_dir, handle_result.result_stream).await
}

async fn build_import_map(
    w_id: &str,
    script_path: &str,
    base_internal_url: &str,
    job_dir: &str,
) -> error::Result<()> {
    let rooted_path = crate::common::use_flow_root_path(script_path);
    let script_path_split = rooted_path.split("/");
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
    write_file(job_dir, "import_map.json", &import_map)?;
    Ok(()) as error::Result<()>
}

#[cfg(feature = "private")]
use crate::{dedicated_worker_oss::handle_dedicated_process, JobCompletedSender};
#[cfg(feature = "private")]
use tokio::sync::mpsc::Receiver;
#[cfg(feature = "private")]
use windmill_queue::DedicatedWorkerJob;

#[cfg(feature = "private")]
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
    jobs_rx: Receiver<DedicatedWorkerJob>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: windmill_common::client::AuthedClient,
) -> Result<()> {
    use windmill_common::variables;

    use crate::common::build_envs_map;

    let _ = write_file(job_dir, "main.ts", inner_content)?;
    let common_deno_proc_envs = get_common_deno_proc_envs(
        &token,
        base_internal_url,
        &Uuid::nil(),
        w_id,
        Some(&db.into()),
    )
    .await;

    let context = variables::get_reserved_variables(
        &db.into(),
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
        None,
        None,
    )
    .await;
    let context_envs = build_envs_map(context.to_vec()).await;

    {
        // let mut start = Instant::now();
        let args = windmill_parser_ts::parse_deno_signature(inner_content, true, false, None)?.args;
        let dates = args
            .iter()
            .filter_map(|x| {
                if matches!(x.typ, Typ::Datetime) {
                    Some(x.name.clone())
                } else {
                    None
                }
            })
            .map(|x| return format!("{x} = {x} ? new Date({x}) : undefined"))
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
            {dates}
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
        write_file(job_dir, "wrapper.ts", &wrapper_content)?;
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
        db,
        script_path,
        "deno",
        client,
    )
    .await
}
