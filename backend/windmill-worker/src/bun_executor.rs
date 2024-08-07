use std::{collections::HashMap, fs, io, path::Path, process::Stdio, time::Instant};

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
        parse_npm_config, read_file_content, read_result, start_child_process, write_file,
        write_file_binary,
    },
    AuthedClientBackgroundTask, BUNFIG_INSTALL_SCOPES, BUN_BUNDLE_CACHE_DIR, BUN_CACHE_DIR,
    BUN_DEPSTAR_CACHE_DIR, BUN_PATH, DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, NODE_PATH,
    NPM_CONFIG_REGISTRY, NPM_PATH, NSJAIL_PATH, PATH_ENV, TZ_ENV,
};

use tokio::{fs::File, process::Command};

use tokio::io::AsyncReadExt;

#[cfg(feature = "enterprise")]
use tokio::sync::mpsc::Receiver;

#[cfg(feature = "enterprise")]
use windmill_common::variables;

use windmill_common::{
    error::{self, Result},
    get_latest_hash_for_path,
    jobs::QueuedJob,
    scripts::ScriptLang,
    worker::{exists_in_cache, get_annotation, save_cache},
    DB,
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

    let mut empty_deps = false;

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

        let new_package_json = read_file_content(&format!("{job_dir}/package.json")).await?;
        empty_deps = new_package_json
            == r#"{
  "dependencies": {}
}"#;
    }

    if !empty_deps {
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
    } else {
        append_logs(job_id, w_id, "\nempty dependencies, skipping install", db).await;
    }

    if export_pkg {
        let mut content = "".to_string();
        {
            let mut file = File::open(format!("{job_dir}/package.json")).await?;
            file.read_to_string(&mut content).await?;
        }
        if !npm_mode {
            content.push_str(BUN_LOCKB_SPLIT);
            {
                let file = format!("{job_dir}/bun.lockb");
                if !empty_deps && tokio::fs::metadata(&file).await.is_ok() {
                    let mut file = File::open(&file).await?;
                    let mut buf = vec![];
                    file.read_to_end(&mut buf).await?;
                    content.push_str(&base64::engine::general_purpose::STANDARD.encode(&buf));
                } else {
                    content.push_str(&EMPTY_FILE);
                }
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

#[derive(PartialEq)]
enum LoaderMode {
    Node,
    Bun,
    BunBundle,
    NodeBundle,
    BrowserBundle,
}
async fn build_loader(
    job_dir: &str,
    base_internal_url: &str,
    token: &str,
    w_id: &str,
    current_path: &str,
    mode: LoaderMode,
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
    if mode == LoaderMode::Node {
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
    minify: true,
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
    } else if mode == LoaderMode::Bun {
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
    } else if mode == LoaderMode::BunBundle
        || mode == LoaderMode::NodeBundle
        || mode == LoaderMode::BrowserBundle
    {
        write_file(
            &job_dir,
            "node_builder.ts",
            &format!(
                r#"
{}

const bo = await Bun.build({{
    entrypoints: ["{job_dir}/main.ts"],
    outdir: "./",
    target: "{}",
    plugins: [p],
    external: ["electron"],
    minify: {{
        identifiers: false,
        syntax: true,
        whitespace: false
    }},
  }});

if (!bo.success) {{
    bo.logs.forEach((l) => console.log(l));
    process.exit(1);
}}
"#,
                loader,
                if mode == LoaderMode::BunBundle {
                    "bun"
                } else if mode == LoaderMode::NodeBundle {
                    "node"
                } else {
                    "browser"
                }
            ),
        )
        .await?;
    }
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
    fs::rename(
        format!("{job_dir}/wrapper.js"),
        format!("{job_dir}/wrapper.mjs"),
    )
    .map_err(|e| error::Error::InternalErr(format!("Could not move wrapper to mjs: {e:#}")))?;
    Ok(())
}

pub async fn generate_bun_bundle(
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
    Ok(())
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub async fn pull_codebase(w_id: &str, id: &str, job_dir: &str) -> Result<()> {
    use crate::global_cache::extract_tar;

    let path = windmill_common::s3_helpers::bundle(&w_id, &id);
    let bun_cache_path = format!("{}/{}", crate::ROOT_CACHE_NOMOUNT_DIR, path);
    let is_tar = id.ends_with(".tar");

    let dst = format!(
        "{job_dir}/{}",
        if is_tar { "codebase.tar" } else { "main.js" }
    );
    let dirs_splitted = bun_cache_path.split("/").collect_vec();
    tokio::fs::create_dir_all(dirs_splitted[..dirs_splitted.len() - 1].join("/")).await?;
    if tokio::fs::metadata(&bun_cache_path).await.is_ok() {
        tracing::info!("loading {bun_cache_path} from cache");
        if is_tar {
            extract_tar(fs::read(bun_cache_path)?.into(), job_dir).await?;
        } else {
            tokio::fs::symlink(&bun_cache_path, dst).await?;
        }
    } else if let Some(os) = windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS
        .read()
        .await
        .clone()
    {
        let bytes = attempt_fetch_bytes(os, &path).await?;

        tokio::fs::write(&bun_cache_path, &bytes).await?;
        if is_tar {
            extract_tar(bytes, job_dir).await?;
        } else {
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

#[cfg(unix)]
pub fn copy_recursively(
    source: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    skip: Option<&Vec<String>>,
) -> io::Result<()> {
    let mut stack = Vec::new();
    stack.push((
        source.as_ref().to_path_buf(),
        destination.as_ref().to_path_buf(),
        0,
    ));
    while let Some((current_source, current_destination, level)) = stack.pop() {
        for entry in fs::read_dir(&current_source)? {
            let entry = entry?;
            let filetype = entry.file_type()?;
            let destination = current_destination.join(entry.file_name());
            if level == 0 {
                if let Some(skip) = skip {
                    if skip.contains(&entry.file_name().to_string_lossy().to_string()) {
                        continue;
                    }
                }
            }
            let original = entry.path();

            if filetype.is_dir() {
                fs::create_dir_all(&destination)?;
                stack.push((entry.path(), destination, level + 1));
            } else {
                fs::hard_link(&original, &destination)?
            }
        }
    }

    Ok(())
}

pub async fn prebundle_script(
    inner_content: &str,
    lockfile: Option<String>,
    script_path: &str,
    job_id: &Uuid,
    w_id: &str,
    db: &DB,
    job_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    token: &str,
) -> Result<()> {
    let (local_path, remote_path) =
        compute_bundle_local_and_remote_path(inner_content, &lockfile, script_path, db, w_id).await;
    if exists_in_cache(&local_path, &remote_path).await {
        return Ok(());
    }
    let annotation = get_annotation(inner_content);
    if annotation.nobundling {
        return Ok(());
    }
    let origin = format!("{job_dir}/main.js");
    write_file(job_dir, "main.ts", &remove_pinned_imports(inner_content)?).await?;
    build_loader(
        job_dir,
        base_internal_url,
        &token,
        w_id,
        script_path,
        if annotation.nodejs_mode {
            LoaderMode::NodeBundle
        } else if annotation.native_mode {
            LoaderMode::BrowserBundle
        } else {
            LoaderMode::BunBundle
        },
    )
    .await?;

    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url).await;

    generate_bun_bundle(
        job_dir,
        w_id,
        job_id,
        worker_name,
        db,
        None,
        &mut 0,
        &mut None,
        &common_bun_proc_envs,
    )
    .await?;
    save_cache(&local_path, &remote_path, &origin).await?;
    Ok(())
}

pub const BUN_BUNDLE_OBJECT_STORE_PREFIX: &str = "bun_bundle/";

async fn get_script_import_updated_at(db: &DB, w_id: &str, script_path: &str) -> Result<String> {
    let script_hash = get_latest_hash_for_path(&mut db.begin().await?, w_id, script_path).await?;
    let last_updated_at = sqlx::query_scalar!(
        "SELECT created_at FROM script WHERE workspace_id = $1 AND hash = $2",
        w_id,
        script_hash.0 .0
    )
    .fetch_one(db)
    .await?;
    Ok(last_updated_at.to_string())
}

async fn compute_bundle_local_and_remote_path(
    inner_content: &str,
    requirements_o: &Option<String>,
    script_path: &str,
    db: &DB,
    w_id: &str,
) -> (String, String) {
    let mut input_src = format!(
        "{}{}",
        inner_content,
        requirements_o
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or_default()
    );

    let relative_imports = crate::worker_lockfiles::extract_relative_imports(
        &inner_content,
        script_path,
        &Some(ScriptLang::Bun),
    );

    for path in relative_imports.unwrap_or_default() {
        if let Ok(updated_at) = get_script_import_updated_at(db, w_id, &path).await {
            input_src.push_str(&path);
            input_src.push_str(&updated_at.to_string());
        }
    }

    let hash = windmill_common::utils::calculate_hash(&input_src);
    let local_path = format!("{BUN_BUNDLE_CACHE_DIR}/{hash}");
    let remote_path = format!("{BUN_BUNDLE_OBJECT_STORE_PREFIX}{hash}");
    (local_path, remote_path)
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
    let mut annotation = windmill_common::worker::get_annotation(inner_content);

    let (mut has_bundle_cache, cache_logs, local_path, remote_path) =
        if requirements_o.is_some() && !annotation.nobundling && codebase.is_none() {
            let (local_path, remote_path) = compute_bundle_local_and_remote_path(
                inner_content,
                &requirements_o,
                job.script_path(),
                db,
                &job.workspace_id,
            )
            .await;

            let (cache, logs) =
                windmill_common::worker::load_cache(&local_path, &remote_path).await;
            (cache, logs, local_path, remote_path)
        } else {
            (false, "".to_string(), "".to_string(), "".to_string())
        };

    if !codebase.is_some() && !has_bundle_cache {
        let _ = write_file(job_dir, "main.ts", inner_content).await?;
    } else if !annotation.native_mode && codebase.is_none() {
        let _ = write_file(job_dir, "package.json", r#"{ "type": "module" }"#).await?;
    };

    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url).await;

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
    if has_bundle_cache {
        let target = format!("{job_dir}/main.js");
        std::os::unix::fs::symlink(&local_path, &target).map_err(|e| {
            error::Error::ExecutionErr(format!(
                "could not copy cached binary from {local_path} to {job_dir}/main: {e:?}"
            ))
        })?;
    } else if let Some(codebase) = codebase.as_ref() {
        pull_codebase(&job.workspace_id, codebase, job_dir).await?;
    } else if let Some(reqs) = requirements_o.as_ref() {
        let splitted = reqs.split(BUN_LOCKB_SPLIT).collect::<Vec<&str>>();
        if splitted.len() != 2 && !annotation.npm_mode {
            return Err(error::Error::ExecutionErr(
                format!("Invalid requirements, expected to find //bun.lockb split pattern in reqs. Found: |{reqs}|")
            ));
        }

        let _ = write_file(job_dir, "package.json", &splitted[0]).await?;
        let lockb = if annotation.npm_mode { "" } else { splitted[1] };
        if lockb != EMPTY_FILE {
            let mut skip_install = false;
            let mut create_buntar = false;
            let mut buntar_path = "".to_string();

            if !annotation.npm_mode {
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

                let buntar_name =
                    base64::engine::general_purpose::URL_SAFE.encode(sha_path.finalize());
                buntar_path = format!("{BUN_DEPSTAR_CACHE_DIR}/{buntar_name}");

                #[cfg(unix)]
                if tokio::fs::metadata(&buntar_path).await.is_ok() {
                    if let Err(e) = copy_recursively(&buntar_path, job_dir, None) {
                        tracing::error!("Could not extract buntar: {e:#}");
                    } else {
                        gbuntar_name = Some(buntar_name.clone());
                        skip_install = true;
                    }
                } else {
                    create_buntar = true;
                }
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

                #[cfg(unix)]
                if create_buntar {
                    fs::create_dir_all(&buntar_path)?;
                    if let Err(e) = copy_recursively(
                        job_dir,
                        &buntar_path,
                        Some(&vec![
                            "main.ts".to_string(),
                            "package.json".to_string(),
                            "bun.lockb".to_string(),
                            "shared".to_string(),
                            "bunfig.toml".to_string(),
                        ]),
                    ) {
                        fs::remove_dir_all(&buntar_path)?;
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

    let mut init_logs = if annotation.native_mode {
        "\n\n--- NATIVE CODE EXECUTION ---\n".to_string()
    } else if has_bundle_cache {
        if annotation.nodejs_mode {
            "\n\n--- NODE BUNDLE SNAPSHOT EXECUTION ---\n".to_string()
        } else {
            "\n\n--- BUN BUNDLE SNAPSHOT EXECUTION ---\n".to_string()
        }
    } else if codebase.is_some() {
        "\n\n--- NODE CODEBASE SNAPSHOT EXECUTION ---\n".to_string()
    } else if annotation.native_mode {
        "\n\n--- NATIVE CODE EXECUTION ---\n".to_string()
    } else if annotation.nodejs_mode {
        write_file(job_dir, "main.ts", &remove_pinned_imports(inner_content)?).await?;
        "\n\n--- NODE CODE EXECUTION ---\n".to_string()
    } else {
        write_file(job_dir, "main.ts", &remove_pinned_imports(inner_content)?).await?;
        "\n\n--- BUN CODE EXECUTION ---\n".to_string()
    };

    if let Some(gbuntar_name) = gbuntar_name {
        init_logs = format!(
            "\nskipping install, using cached buntar based on lockfile hash: {gbuntar_name}{}",
            init_logs
        );
    }

    if has_bundle_cache {
        init_logs = format!("\n{}{}", cache_logs, init_logs);
    }

    let write_wrapper_f = async {
        if !has_bundle_cache && annotation.native_mode {
            return Ok(()) as error::Result<()>;
        }
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

        let main_import = if codebase.is_some() || has_bundle_cache {
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
    console.error(e);
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
            if !annotation.native_mode {
                create_args_and_out_file(&client, job, job_dir, db).await?;
            }
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

    let build_cache = !has_bundle_cache
        && !annotation.nobundling
        && !codebase.is_some()
        && (requirements_o.is_some() || annotation.native_mode);

    let write_loader_f = async {
        if build_cache {
            build_loader(
                job_dir,
                base_internal_url,
                &client.get_token().await,
                &job.workspace_id,
                &job.script_path(),
                if annotation.nodejs_mode {
                    LoaderMode::NodeBundle
                } else if annotation.native_mode {
                    LoaderMode::BrowserBundle
                } else {
                    LoaderMode::BunBundle
                },
            )
            .await?;

            Ok(())
        } else if !codebase.is_some() && !has_bundle_cache {
            build_loader(
                job_dir,
                base_internal_url,
                &client.get_token().await,
                &job.workspace_id,
                &job.script_path(),
                if annotation.nodejs_mode {
                    LoaderMode::Node
                } else {
                    LoaderMode::Bun
                },
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
    if !codebase.is_some() && !has_bundle_cache {
        if build_cache {
            generate_bun_bundle(
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
            if !local_path.is_empty() {
                match save_cache(&local_path, &remote_path, &format!("{job_dir}/main.js")).await {
                    Err(e) => {
                        let em = format!("could not save {local_path} to bundle cache: {e:?}");
                        tracing::error!(em)
                    }
                    Ok(logs) => {
                        init_logs.push_str(&"\n");
                        init_logs.push_str(&logs);
                        init_logs.push_str(&"\n");
                        tracing::info!("saved bun bundle cache: {logs}")
                    }
                }
            }
            if !annotation.native_mode {
                let ex_wrapper = read_file_content(&format!("{job_dir}/wrapper.mjs")).await?;
                write_file(
                    job_dir,
                    "wrapper.mjs",
                    &ex_wrapper.replace(
                        "import * as Main from \"./main.ts\"",
                        "import * as Main from \"./main.js\"",
                    ),
                )
                .await?;
                write_file(job_dir, "package.json", r#"{ "type": "module" }"#).await?;
            }
            fs::remove_file(format!("{job_dir}/main.ts"))?;
            has_bundle_cache = true;
        } else if annotation.nodejs_mode {
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
    }
    if annotation.native_mode {
        let env_code = format!(
            "const process = {{ env: {{}} }};\nconst BASE_URL = '{base_internal_url}';\nconst BASE_INTERNAL_URL = '{base_internal_url}';\nprocess.env['BASE_URL'] = BASE_URL;process.env['BASE_INTERNAL_URL'] = BASE_INTERNAL_URL;\n{}",
            reserved_variables
                .iter()
                .map(|(k, v)| format!("process.env['{}'] = '{}';\n", k, v))
                .collect::<Vec<String>>()
                .join("\n"));
        let js_code = read_file_content(&format!("{job_dir}/main.js")).await?;
        let started_at = Instant::now();
        let result = crate::js_eval::eval_fetch_timeout(
            env_code,
            inner_content.clone(),
            js_code,
            job.args.as_ref(),
            job.id,
            job.timeout,
            db,
            mem_peak,
            canceled_by,
            worker_name,
            &job.workspace_id,
            false,
        )
        .await?;
        tracing::info!(
            "Executed native code in {}ms",
            started_at.elapsed().as_millis()
        );
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("{}\n{}", init_logs, result.1),
            db,
        )
        .await;
        return Ok(result.0);
    }
    append_logs(&job.id, &job.workspace_id, init_logs, db).await;

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
        } else if codebase.is_some() || has_bundle_cache {
            vec![
                "--config",
                "run.config.proto",
                "--",
                &BUN_PATH,
                "run",
                "--preserve-symlinks",
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
                .args(vec!["--preserve-symlinks", &script_path])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            bun_cmd
        } else {
            let script_path = format!("{job_dir}/wrapper.mjs");

            let mut bun_cmd = Command::new(&*BUN_PATH);
            let args = if codebase.is_some() || has_bundle_cache {
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
        (String::from("FORCE_COLOR"), "1".to_string()),
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
    jobs_rx: Receiver<std::sync::Arc<QueuedJob>>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<()> {
    let mut logs = "".to_string();
    let mut mem_peak: i32 = 0;
    let mut canceled_by: Option<CanceledBy> = None;
    tracing::info!("Starting worker {w_id};{script_path} (codebase: {codebase:?}");
    if !codebase.is_some() {
        let _ = write_file(job_dir, "main.ts", inner_content).await?;
    }

    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(&base_internal_url).await;

    let mut annotation = windmill_common::worker::get_annotation(inner_content);

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

console.log('start'); 

for await (const line of Readline.createInterface({{ input: process.stdin }})) {{
    {print_lines}

    if (line === "end") {{
        process.exit(0);
    }}
    try {{
        let {{ {spread} }} = JSON.parse(line) 
        {dates}
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
            if annotation.nodejs_mode {
                LoaderMode::Node
            } else {
                LoaderMode::Bun
            },
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
