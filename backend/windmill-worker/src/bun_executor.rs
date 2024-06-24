use std::{collections::HashMap, process::Stdio};

use base64::Engine;
use itertools::Itertools;
use serde_json::value::RawValue;
use sha2::Digest;
use uuid::Uuid;
use windmill_parser_ts::remove_pinned_imports;
use windmill_queue::{append_logs, CanceledBy};

#[cfg(feature = "enterprise")]
use crate::common::build_envs_map;

use crate::{
    common::{
        create_args_and_out_file, get_main_override, get_reserved_variables, handle_child,
        parse_npm_config, read_result, start_child_process, write_file, write_file_binary,
    },
    AuthedClientBackgroundTask, BUNFIG_INSTALL_SCOPES, BUN_CACHE_DIR, BUN_PATH, BUN_TAR_CACHE_DIR,
    DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, NODE_PATH, NPM_CONFIG_REGISTRY, NPM_PATH, NSJAIL_PATH,
    PATH_ENV, TZ_ENV,
};

use tokio::{fs::File, process::Command};

use tokio::io::AsyncReadExt;

#[cfg(feature = "enterprise")]
use tokio::sync::mpsc::Receiver;

#[cfg(feature = "enterprise")]
use windmill_common::variables;

use windmill_common::{
    error::{self, Result},
    jobs::QueuedJob,
};

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::s3_helpers::attempt_fetch_bytes;

use windmill_parser::Typ;

const RELATIVE_BUN_LOADER: &str = include_str!("../loader.bun.js");

const RELATIVE_BUN_BUILDER: &str = include_str!("../loader_builder.bun.js");

const NSJAIL_CONFIG_RUN_BUN_CONTENT: &str = include_str!("../nsjail/run.bun.config.proto");

pub const BUN_LOCKB_SPLIT: &str = "\n//bun.lockb\n";
pub const EMPTY_FILE: &str = "<empty>";

pub async fn gen_lockfile(
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
    raw_deps: Option<String>,
    npm_mode: bool,
) -> Result<Option<String>> {
    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url).await;

    if let Some(raw_deps) = raw_deps {
        gen_bunfig(job_dir).await?;
        write_file(job_dir, "package.json", raw_deps.as_str()).await?;
    } else {
        let _ = write_file(
            &job_dir,
            "build.js",
            &format!(
                r#"
{}

{RELATIVE_BUN_BUILDER}
"#,
                RELATIVE_BUN_LOADER
                    .replace("W_ID", w_id)
                    .replace("BASE_INTERNAL_URL", base_internal_url)
                    .replace("TOKEN", token)
                    .replace(
                        "CURRENT_PATH",
                        &crate::common::use_flow_root_path(script_path)
                    )
                    .replace("RAW_GET_ENDPOINT", "raw")
            ),
        )
        .await?;

        gen_bunfig(job_dir).await?;

        let mut child_cmd = Command::new(&*BUN_PATH);
        child_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(common_bun_proc_envs.clone())
            .args(vec!["run", "build.js"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child_process = start_child_process(child_cmd, &*BUN_PATH).await?;

        handle_child(
            job_id,
            db,
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
    }

    install_lockfile(
        mem_peak,
        canceled_by,
        job_id,
        w_id,
        db,
        job_dir,
        worker_name,
        common_bun_proc_envs,
        npm_mode,
    )
    .await?;

    if export_pkg && !npm_mode {
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

async fn gen_bunfig(job_dir: &str) -> Result<()> {
    let registry = NPM_CONFIG_REGISTRY.read().await.clone();
    let bunfig_install_scopes = BUNFIG_INSTALL_SCOPES.read().await.clone();
    if registry.is_some() || bunfig_install_scopes.is_some() {
        let (url, token_opt) = if let Some(ref s) = registry {
            let url = s.trim();
            if url.is_empty() {
                ("https://registry.npmjs.org/".to_string(), None)
            } else {
                parse_npm_config(s)
            }
        } else {
            ("https://registry.npmjs.org/".to_string(), None)
        };
        let registry_toml_string = if let Some(token) = token_opt {
            format!("{{ url = \"{url}\", token = \"{token}\" }}")
        } else {
            format!("\"{url}\"")
        };
        let bunfig_toml = format!(
            r#"
[install]
registry = {}

{}
"#,
            registry_toml_string,
            bunfig_install_scopes
                .map(|x| format!("[install.scopes]\n{x}"))
                .unwrap_or("".to_string())
        );
        tracing::debug!("Writing following bunfig.toml: {bunfig_toml}");
        let _ = write_file(&job_dir, "bunfig.toml", &bunfig_toml).await?;
    }
    Ok(())
}

pub async fn install_lockfile(
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_id: &Uuid,
    w_id: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    job_dir: &str,
    worker_name: &str,
    common_bun_proc_envs: HashMap<String, String>,
    npm_mode: bool,
) -> Result<()> {
    let mut child_cmd = Command::new(if npm_mode { &*NPM_PATH } else { &*BUN_PATH });
    child_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(common_bun_proc_envs)
        .args(vec!["install"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut npm_logs = if npm_mode {
        "NPM mode\n".to_string()
    } else {
        "".to_string()
    };

    let has_file = if npm_mode {
        let registry = NPM_CONFIG_REGISTRY.read().await.clone();
        if let Some(registry) = registry {
            let content = registry
                .trim_start_matches("https:")
                .trim_start_matches("http:");

            let mut splitted = registry.split(":_authToken=");
            let custom_registry = splitted.next().unwrap_or_default();
            npm_logs.push_str(&format!(
                "Using custom npm registry: {custom_registry} {}\n",
                if splitted.next().is_some() {
                    "with authToken"
                } else {
                    "without authToken"
                }
            ));

            child_cmd.env("NPM_CONFIG_REGISTRY", custom_registry);
            write_file(job_dir, ".npmrc", content).await?;
            true
        } else {
            false
        }
    } else {
        false
    };

    if npm_mode {
        append_logs(&job_id.clone(), w_id, npm_logs, db).await;
    }

    let child_process = start_child_process(child_cmd, &*BUN_PATH).await?;

    gen_bunfig(job_dir).await?;
    handle_child(
        job_id,
        db,
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

    if has_file {
        tokio::fs::remove_file(format!("{job_dir}/.npmrc")).await?;
    }

    Ok(())
}

struct Annotations {
    npm_mode: bool,
    nodejs_mode: bool,
}

fn get_annotation(inner_content: &str) -> Annotations {
    let annotations = inner_content
        .lines()
        .take_while(|x| x.starts_with("//"))
        .map(|x| x.to_string().replace("//", "").trim().to_string())
        .collect_vec();
    let nodejs_mode: bool = annotations.contains(&"nodejs".to_string());
    let npm_mode: bool = annotations.contains(&"npm".to_string());

    Annotations { npm_mode, nodejs_mode }
}

pub async fn build_loader(
    job_dir: &str,
    base_internal_url: &str,
    token: &str,
    w_id: &str,
    current_path: &str,
    nodejs_mode: bool,
) -> Result<()> {
    let loader = RELATIVE_BUN_LOADER
        .replace("W_ID", w_id)
        .replace("BASE_INTERNAL_URL", base_internal_url)
        .replace("TOKEN", token)
        .replace(
            "CURRENT_PATH",
            &crate::common::use_flow_root_path(current_path),
        )
        .replace("RAW_GET_ENDPOINT", "raw_unpinned");
    if nodejs_mode {
        write_file(
            &job_dir,
            "node_builder.ts",
            &format!(
                r#"
{}

import {{ readdir }} from "node:fs/promises";

let fileNames = []
try {{
    fileNames = await readdir("{job_dir}/node_modules")
}} catch (e) {{

}}

const bo = await Bun.build({{
    entrypoints: ["{job_dir}/wrapper.mjs"],
    outdir: "./",
    target: "node",
    plugins: [p],
    external: fileNames,
  }});

if (!bo.success) {{
    bo.logs.forEach((l) => console.log(l));
    process.exit(1);
}}
"#,
                loader
            ),
        )
        .await?;
    } else {
        write_file(
            &job_dir,
            "loader.bun.js",
            &format!(
                r#"
import {{ plugin }} from "bun";

{}

plugin(p)
"#,
                loader
            ),
        )
        .await?;
    };
    Ok(())
}

pub async fn generate_wrapper_mjs(
    job_dir: &str,
    w_id: &str,
    job_id: &Uuid,
    worker_name: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    timeout: Option<i32>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    common_bun_proc_envs: &HashMap<String, String>,
) -> Result<()> {
    let mut child = Command::new(&*BUN_PATH);
    child
        .current_dir(job_dir)
        .env_clear()
        .envs(common_bun_proc_envs.clone())
        .env("PATH", PATH_ENV.as_str())
        .args(vec!["run", "node_builder.ts"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child_process = start_child_process(child, &*BUN_PATH).await?;
    handle_child(
        job_id,
        db,
        mem_peak,
        canceled_by,
        child_process,
        false,
        worker_name,
        w_id,
        "bun build",
        timeout,
        false,
    )
    .await?;
    tokio::fs::rename(
        format!("{job_dir}/wrapper.js"),
        format!("{job_dir}/wrapper.mjs"),
    )
    .await
    .map_err(|e| error::Error::InternalErr(format!("Could not move wrapper to mjs: {e:#}")))?;
    Ok(())
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub async fn pull_codebase(w_id: &str, id: &str, job_dir: &str) -> Result<()> {
    let path = windmill_common::s3_helpers::bundle(&w_id, &id);
    let bun_cache_path = format!("{}/{}", BUN_CACHE_DIR, path);
    let dst = format!("{job_dir}/main.js");
    let dirs_splitted = bun_cache_path.split("/").collect_vec();
    tokio::fs::create_dir_all(dirs_splitted[..dirs_splitted.len() - 1].join("/")).await?;
    if tokio::fs::metadata(&bun_cache_path).await.is_ok() {
        tracing::info!("loading {bun_cache_path} from cache");
        tokio::fs::symlink(&bun_cache_path, dst).await?;
    } else if let Some(os) = windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS
        .read()
        .await
        .clone()
    {
        let bytes = attempt_fetch_bytes(os, &path).await?;
        if *windmill_common::worker::CLOUD_HOSTED {
            tokio::fs::write(dst, &bytes).await?;
        } else {
            tokio::fs::write(&bun_cache_path, &bytes).await?;
            tokio::fs::symlink(bun_cache_path, dst).await?;
        }

        // extract_tar(bytes, job_dir).await?;
    }

    return Ok(());
}

#[cfg(not(all(feature = "enterprise", feature = "parquet")))]
pub async fn pull_codebase(_w_id: &str, _id: &str, _job_dir: &str) -> Result<()> {
    return Err(error::Error::ExecutionErr(
        "codebase is an EE feature".to_string(),
    ));
}

fn untar_file(file_path: &str, output_dir: &str) -> anyhow::Result<()> {
    // Open the tar file
    let file = std::fs::File::open(file_path)?;
    let file = std::io::BufReader::new(file);

    // For a plain tar file, use it directly
    let mut archive = tar::Archive::new(file);
    archive.unpack(output_dir)?;

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_bun_job(
    requirements_o: Option<String>,
    codebase: Option<String>,
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
    if !codebase.is_some() {
        let _ = write_file(job_dir, "main.ts", inner_content).await?;
    } else {
        let _ = write_file(job_dir, "package.json", r#"{ "type": "module" }"#).await?;
    };

    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url).await;

    let mut annotation = get_annotation(inner_content);

    if codebase.is_some() {
        annotation.nodejs_mode = true
    }
    let main_override = get_main_override(job.args.as_ref());

    #[cfg(not(feature = "enterprise"))]
    if annotation.nodejs_mode || annotation.npm_mode {
        return Err(error::Error::ExecutionErr(
            "Nodejs / npm mode is an EE feature".to_string(),
        ));
    }

    let mut gbuntar_name = None;
    if let Some(codebase) = codebase.as_ref() {
        pull_codebase(&job.workspace_id, codebase, job_dir).await?;
    } else if let Some(reqs) = requirements_o {
        let splitted = reqs.split(BUN_LOCKB_SPLIT).collect::<Vec<&str>>();
        if splitted.len() != 2 {
            return Err(error::Error::ExecutionErr(
                format!("Invalid requirements, expected to find //bun.lockb split pattern in reqs. Found: |{reqs}|")
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

            let mut sha_path = sha2::Sha256::new();
            sha_path.update(lockb.as_bytes());

            let buntar_name = base64::engine::general_purpose::URL_SAFE.encode(sha_path.finalize());
            let buntar_path = format!("{BUN_TAR_CACHE_DIR}/{buntar_name}.tar");

            let mut skip_install = false;
            let mut create_buntar = false;
            if tokio::fs::metadata(&buntar_path).await.is_ok() {
                if let Err(e) = untar_file(&buntar_path, job_dir) {
                    tracing::error!("Could not untar buntar: {e}");
                } else {
                    gbuntar_name = Some(buntar_name.clone());
                    skip_install = true;
                }
            } else {
                create_buntar = true;
            }
            if !skip_install {
                install_lockfile(
                    mem_peak,
                    canceled_by,
                    &job.id,
                    &job.workspace_id,
                    db,
                    job_dir,
                    worker_name,
                    common_bun_proc_envs.clone(),
                    annotation.npm_mode,
                )
                .await?;

                if create_buntar {
                    let f = std::fs::File::create(&buntar_path);
                    if let Err(e) = f {
                        tracing::error!("Could not create buntar file {buntar_path}: {e}");
                    } else if let Err(e) =
                        tar::Builder::new(f.unwrap()).append_dir_all(".", job_dir)
                    {
                        tracing::error!("Could not create buntar: {e}");
                    }
                }
            }
        }
    } else {
        // if !*DISABLE_NSJAIL || !empty_trusted_deps || has_custom_config_registry {
        let logs1 = "\n\n--- BUN INSTALL ---\n".to_string();
        append_logs(&job.id, &job.workspace_id, logs1, db).await;

        let _ = gen_lockfile(
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
            None,
            annotation.npm_mode,
        )
        .await?;

        // }
    }

    let _ = write_file(job_dir, "main.ts", &remove_pinned_imports(inner_content)?).await?;

    let mut init_logs = if codebase.is_some() {
        "\n\n--- NODE SNAPSHOT EXECUTION ---\n".to_string()
    } else if annotation.nodejs_mode {
        "\n\n--- NODE CODE EXECUTION ---\n".to_string()
    } else {
        "\n\n--- BUN CODE EXECUTION ---\n".to_string()
    };

    if let Some(gbuntar_name) = gbuntar_name {
        init_logs = format!(
            "\nskipping install, using cached buntar based on lockfile hash: {gbuntar_name}{}",
            init_logs
        );
    }

    append_logs(&job.id, &job.workspace_id, init_logs, db).await;

    let write_wrapper_f = async {
        // let mut start = Instant::now();
        let args =
            windmill_parser_ts::parse_deno_signature(inner_content, true, main_override.clone())?
                .args;
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
        let main_name = main_override.unwrap_or("main".to_string());

        let main_import = if codebase.is_some() {
            "./main.js"
        } else {
            "./main.ts"
        };

        let wrapper_content: String = format!(
            r#"
import * as Main from "{main_import}";

import * as fs from "fs/promises";

const args = await fs.readFile('args.json', {{ encoding: 'utf8' }}).then(JSON.parse)
    .then(({{ {spread} }}) => [ {spread} ])

BigInt.prototype.toJSON = function () {{
    return this.toString();
}};

{dates}
async function run() {{
    let res = await Main.{main_name}(...args);
    const res_json = JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value);
    await fs.writeFile("result.json", res_json);
    process.exit(0);
}}
try {{
    await run();
}} catch(e) {{
    let err = {{ message: e.message, name: e.name, stack: e.stack }};
    let step_id = process.env.WM_FLOW_STEP_ID;
    if (step_id) {{
        err["step_id"] = step_id;
    }}
    await fs.writeFile("result.json", JSON.stringify(err));
    process.exit(1);
}}
    "#,
        );
        write_file(job_dir, "wrapper.mjs", &wrapper_content).await?;
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
        if !codebase.is_some() {
            build_loader(
                job_dir,
                base_internal_url,
                &client.get_token().await,
                &job.workspace_id,
                &job.script_path(),
                annotation.nodejs_mode,
            )
            .await
        } else {
            Ok(())
        }
    };

    let (reserved_variables, _, _) = tokio::try_join!(
        reserved_variables_args_out_f,
        write_wrapper_f,
        write_loader_f
    )?;

    if annotation.nodejs_mode && !codebase.is_some() {
        generate_wrapper_mjs(
            job_dir,
            &job.workspace_id,
            &job.id,
            worker_name,
            db,
            job.timeout,
            mem_peak,
            canceled_by,
            &common_bun_proc_envs,
        )
        .await?;
    }

    //do not cache local dependencies
    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_BUN_CONTENT
                .replace(
                    "{LANG}",
                    if annotation.nodejs_mode {
                        "nodejs"
                    } else {
                        "bun"
                    },
                )
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", BUN_CACHE_DIR)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace(
                    "{SHARED_MOUNT}",
                    &shared_mount.replace(
                        "/tmp/shared",
                        if annotation.nodejs_mode {
                            "/tmp/nodejs/shared"
                        } else {
                            "/tmp/bun/shared"
                        },
                    ),
                ),
        )
        .await?;

        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        let args = if annotation.nodejs_mode {
            vec![
                "--config",
                "run.config.proto",
                "--",
                &NODE_PATH,
                "/tmp/nodejs/wrapper.mjs",
            ]
        } else if codebase.is_some() {
            vec![
                "--config",
                "run.config.proto",
                "--",
                &BUN_PATH,
                "run",
                "/tmp/bun/wrapper.mjs",
            ]
        } else {
            vec![
                "--config",
                "run.config.proto",
                "--",
                &BUN_PATH,
                "run",
                "-i",
                "--prefer-offline",
                "-r",
                "/tmp/bun/loader.bun.js",
                "/tmp/bun/wrapper.mjs",
            ]
        };
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(common_bun_proc_envs)
            .env("PATH", PATH_ENV.as_str())
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
    } else {
        let cmd = if annotation.nodejs_mode {
            let script_path = format!("{job_dir}/wrapper.mjs");

            let mut bun_cmd = Command::new(&*NODE_PATH);
            bun_cmd
                .current_dir(job_dir)
                .env_clear()
                .envs(envs)
                .envs(reserved_variables)
                .envs(common_bun_proc_envs)
                .args(vec![&script_path])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            bun_cmd
        } else {
            let script_path = format!("{job_dir}/wrapper.mjs");

            let mut bun_cmd = Command::new(&*BUN_PATH);
            let args = if codebase.is_some() {
                vec!["run", &script_path]
            } else {
                vec![
                    "run",
                    "-i",
                    "--prefer-offline",
                    "-r",
                    "./loader.bun.js",
                    &script_path,
                ]
            };
            bun_cmd
                .current_dir(job_dir)
                .env_clear()
                .envs(envs)
                .envs(reserved_variables)
                .envs(common_bun_proc_envs)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            bun_cmd
        };
        start_child_process(
            cmd,
            if annotation.nodejs_mode {
                &*NODE_PATH
            } else {
                &*BUN_PATH
            },
        )
        .await?
    };

    handle_child(
        &job.id,
        db,
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
    codebase: Option<String>,
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
    tracing::info!("Starting worker {w_id};{script_path} (codebase: {codebase:?}");
    if !codebase.is_some() {
        let _ = write_file(job_dir, "main.ts", inner_content).await?;
    } else {
        let _ = write_file(job_dir, "package.json", r#"{ "type": "module" }"#).await?;
    }

    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url).await;

    let mut annotation = get_annotation(inner_content);

    //TODO: remove this when bun dedicated workers work without issues
    annotation.nodejs_mode = true;

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
    )
    .await;
    let context_envs = build_envs_map(context.to_vec()).await;

    if let Some(codebase) = codebase.as_ref() {
        pull_codebase(w_id, codebase, job_dir).await?;
    } else if let Some(reqs) = requirements_o {
        let splitted = reqs.split(BUN_LOCKB_SPLIT).collect::<Vec<&str>>();
        if splitted.len() != 2 {
            return Err(error::Error::ExecutionErr(
                format!("Invalid requirements, expected to find //bun.lockb split pattern in reqs. Found: |{reqs}|")
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

            install_lockfile(
                &mut mem_peak,
                &mut canceled_by,
                &Uuid::nil(),
                &w_id,
                db,
                job_dir,
                worker_name,
                common_bun_proc_envs.clone(),
                annotation.npm_mode,
            )
            .await?;
            tracing::info!("dedicated worker requirements installed: {reqs}");
        }
    } else if !*DISABLE_NSJAIL {
        logs.push_str("\n\n--- BUN INSTALL ---\n");
        let _ = gen_lockfile(
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
            None,
            annotation.npm_mode,
        )
        .await?;
    }

    let main_code = remove_pinned_imports(inner_content)?;
    let _ = write_file(job_dir, "main.ts", &main_code).await?;

    {
        // let mut start = Instant::now();
        let args = windmill_parser_ts::parse_deno_signature(inner_content, true, None)?.args;
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

        let is_debug = std::env::var("RUST_LOG").is_ok_and(|x| x == "windmill=debug");
        let print_lines = if is_debug {
            r#"console.log(line);"#
        } else {
            ""
        };

        let main_import = if codebase.is_some() {
            "./main.js"
        } else {
            "./main.ts"
        };
        let wrapper_content: String = format!(
            r#"
import * as Main from "{main_import}";
import * as Readline from "node:readline"

BigInt.prototype.toJSON = function () {{
    return this.toString();
}};

{dates}

console.log('start'); 

for await (const line of Readline.createInterface({{ input: process.stdin }})) {{
    {print_lines}

    if (line === "end") {{
        process.exit(0);
    }}
    try {{
        let {{ {spread} }} = JSON.parse(line) 
        let res = await Main.main(...[ {spread} ]);
        console.log("wm_res[success]:" + JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value));
    }} catch (e) {{
        console.log("wm_res[error]:" + JSON.stringify({{ message: e.message, name: e.name, stack: e.stack, line: line }}));
    }}
}}
"#,
        );
        write_file(job_dir, "wrapper.mjs", &wrapper_content).await?;
    }

    if !codebase.is_some() {
        build_loader(
            job_dir,
            base_internal_url,
            token,
            w_id,
            script_path,
            annotation.nodejs_mode,
        )
        .await?;
    }

    if annotation.nodejs_mode && !codebase.is_some() {
        generate_wrapper_mjs(
            job_dir,
            w_id,
            &Uuid::nil(),
            worker_name,
            db,
            None,
            &mut mem_peak,
            &mut canceled_by,
            &common_bun_proc_envs,
        )
        .await?;
    }

    if annotation.nodejs_mode {
        let script_path = format!("{job_dir}/wrapper.mjs");

        handle_dedicated_process(
            &*NODE_PATH,
            job_dir,
            context_envs,
            envs,
            context,
            common_bun_proc_envs,
            vec![&script_path],
            killpill_rx,
            job_completed_tx,
            token,
            jobs_rx,
            worker_name,
            db,
            &script_path,
            "nodejs",
        )
        .await
    } else {
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
                "./loader.bun.js",
                &format!("{job_dir}/wrapper.mjs"),
            ],
            killpill_rx,
            job_completed_tx,
            token,
            jobs_rx,
            worker_name,
            db,
            script_path,
            "bun",
        )
        .await
    }
}
