#[cfg(feature = "deno_core")]
use std::time::Instant;
use std::{collections::HashMap, fs, process::Stdio};

use base64::Engine;
use itertools::Itertools;

use serde_json::value::RawValue;

use uuid::Uuid;
use windmill_parser_ts::remove_pinned_imports;
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob, PrecomputedAgentInfo};

#[cfg(feature = "enterprise")]
use crate::common::build_envs_map;

use crate::{
    common::{
        create_args_and_out_file, get_reserved_variables, parse_npm_config, read_file,
        read_file_content, read_result, start_child_process, write_file_binary, OccupancyMetrics,
    },
    handle_child::handle_child,
    BUNFIG_INSTALL_SCOPES, BUN_BUNDLE_CACHE_DIR, BUN_CACHE_DIR, BUN_PATH, DISABLE_NSJAIL,
    DISABLE_NUSER, HOME_ENV, NODE_BIN_PATH, NODE_PATH, NPM_CONFIG_REGISTRY, NPM_PATH, NSJAIL_PATH,
    PATH_ENV, PROXY_ENVS, TZ_ENV,
};
use windmill_common::client::AuthedClient;

#[cfg(windows)]
use crate::SYSTEM_ROOT;

use tokio::{fs::File, process::Command};

use tokio::io::AsyncReadExt;

#[cfg(feature = "enterprise")]
use tokio::sync::mpsc::Receiver;

#[cfg(feature = "enterprise")]
use windmill_common::variables;

use windmill_common::{
    error::{self, Result},
    get_latest_hash_for_path,
    scripts::ScriptLang,
    worker::{exists_in_cache, save_cache, to_raw_value, write_file, Connection, DISABLE_BUNDLING},
    DB,
};

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::s3_helpers::attempt_fetch_bytes;

use windmill_parser::Typ;

const RELATIVE_BUN_LOADER: &str = include_str!("../loader.bun.js");

const RELATIVE_BUN_BUILDER: &str = include_str!("../loader_builder.bun.js");

const NSJAIL_CONFIG_RUN_BUN_CONTENT: &str = include_str!("../nsjail/run.bun.config.proto");

pub const BUN_LOCK_SPLIT: &str = "\n//bun.lock\n";
pub const BUN_LOCKB_SPLIT: &str = "\n//bun.lockb\n";
pub const BUN_LOCK_SPLIT_WINDOWS: &str = "\r\n//bun.lock\r\n";
pub const BUN_LOCKB_SPLIT_WINDOWS: &str = "\r\n//bun.lockb\r\n";

pub const EMPTY_FILE: &str = "<empty>";

/// Returns (package.json, bun.lock(b), is_empty, is_binary)
fn split_lockfile(lockfile: &str) -> (&str, Option<&str>, bool, bool) {
    if let Some(index) = lockfile.find(BUN_LOCK_SPLIT) {
        // Split using "\n//bun.lock\n"
        let (before, after_with_sep) = lockfile.split_at(index);
        let after = &after_with_sep[BUN_LOCK_SPLIT.len()..];
        (before, Some(after), after == EMPTY_FILE, false)
    } else if let Some(index) = lockfile.find(BUN_LOCKB_SPLIT) {
        // Split using "\n//bun.lockb\n"
        let (before, after_with_sep) = lockfile.split_at(index);
        let after = &after_with_sep[BUN_LOCKB_SPLIT.len()..];
        (before, Some(after), after == EMPTY_FILE, true)
    } else if let Some(index) = lockfile.find(BUN_LOCK_SPLIT_WINDOWS) {
        // Split using "\r\n//bun.lock\r\n"
        let (before, after_with_sep) = lockfile.split_at(index);
        let after = &after_with_sep[BUN_LOCK_SPLIT_WINDOWS.len()..];
        (before, Some(after), after == EMPTY_FILE, false)
    } else if let Some(index) = lockfile.find(BUN_LOCKB_SPLIT_WINDOWS) {
        // Split using "\r\n//bun.lockb\r\n"
        let (before, after_with_sep) = lockfile.split_at(index);
        let after = &after_with_sep[BUN_LOCKB_SPLIT_WINDOWS.len()..];
        (before, Some(after), after == EMPTY_FILE, true)
    } else {
        (lockfile, None, false, false)
    }
}

pub async fn gen_bun_lockfile(
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_id: &Uuid,
    w_id: &str,
    db: Option<&Connection>,
    token: &str,
    script_path: &str,
    job_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    export_pkg: bool,
    raw_deps: Option<String>,
    npm_mode: bool,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
) -> Result<Option<String>> {
    let common_bun_proc_envs: HashMap<String, String> = get_common_bun_proc_envs(None).await;

    let mut empty_deps = false;

    if let Some(raw_deps) = raw_deps.as_ref() {
        gen_bunfig(job_dir).await?;
        write_file(job_dir, "package.json", raw_deps.as_str())?;
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
        )?;

        gen_bunfig(job_dir).await?;

        let mut child_cmd = Command::new(&*BUN_PATH);
        child_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(common_bun_proc_envs.clone())
            .args(vec!["run", "build.js"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        child_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

        let mut child_process = start_child_process(child_cmd, &*BUN_PATH).await?;

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
                "bun build",
                None,
                false,
                occupancy_metrics,
                None,
            )
            .await?;
        } else {
            Box::into_pin(child_process.wait()).await?;
        }

        let new_package_json = read_file_content(&format!("{job_dir}/package.json")).await?;
        empty_deps = new_package_json
            == r#"{
  "dependencies": {}
}"#;
    }

    if !empty_deps {
        install_bun_lockfile(
            mem_peak,
            canceled_by,
            job_id,
            w_id,
            db,
            job_dir,
            worker_name,
            common_bun_proc_envs,
            npm_mode,
            occupancy_metrics,
        )
        .await?;
    } else {
        if let Some(db) = db {
            append_logs(job_id, w_id, "\nempty dependencies, skipping install", db).await;
        }
    }

    if export_pkg {
        let mut content;
        {
            let mut file = File::open(format!("{job_dir}/package.json")).await?;
            let mut buf = String::default();
            file.read_to_string(&mut buf).await?;
            if raw_deps.is_some() {
                let mut json_map: HashMap<String, Box<RawValue>> = serde_json::from_str(&buf)?;
                json_map.insert(
                    "generatedFromPackageJson".to_string(),
                    to_raw_value(&"true".to_string()),
                );
                content = serde_json::to_string_pretty(&json_map)?;
            } else {
                content = buf;
            }
        }
        if !npm_mode {
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            content.push_str(BUN_LOCK_SPLIT);

            #[cfg(target_os = "windows")]
            content.push_str(BUN_LOCK_SPLIT_WINDOWS);

            {
                let file = format!("{job_dir}/bun.lock");
                if !empty_deps && tokio::fs::metadata(&file).await.is_ok() {
                    let mut file = File::open(&file).await?;
                    let mut buf = String::default();
                    file.read_to_string(&mut buf).await?;
                    content.push_str(&buf);
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
        let _ = write_file(&job_dir, "bunfig.toml", &bunfig_toml)?;
    }
    Ok(())
}

pub async fn install_bun_lockfile(
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_id: &Uuid,
    w_id: &str,
    db: Option<&Connection>,
    job_dir: &str,
    worker_name: &str,
    common_bun_proc_envs: HashMap<String, String>,
    npm_mode: bool,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
) -> Result<()> {
    let mut child_cmd = Command::new(if npm_mode { &*NPM_PATH } else { &*BUN_PATH });
    child_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .envs(common_bun_proc_envs)
        .args(vec!["install", "--save-text-lockfile"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    child_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

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
            write_file(job_dir, ".npmrc", content)?;
            true
        } else {
            false
        }
    } else {
        false
    };

    if npm_mode {
        if let Some(db) = db {
            append_logs(&job_id.clone(), w_id, npm_logs, db).await;
        }
    }

    let mut child_process = start_child_process(child_cmd, &*BUN_PATH).await?;

    gen_bunfig(job_dir).await?;
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
            "bun install",
            None,
            false,
            occupancy_metrics,
            None,
        )
        .await?;
    } else {
        Box::into_pin(child_process.wait()).await?;
    }

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

try {{
    await Bun.build({{
        entrypoints: ["{job_dir}/wrapper.mjs"],
        outdir: "./",
        target: "node",
        plugins: [p],
        external: fileNames,
        minify: true,
    }});
}} catch(err) {{
    console.log(err);
    console.log("Failed to build node bundle");
    process.exit(1);
}}
"#,
                loader
            ),
        )?;
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
        )?;
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

try {{
    await Bun.build({{
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
}} catch(err) {{
    console.log(err)
    console.log("Failed to build node bundle");
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
        )?;
    }
    Ok(())
}

pub async fn generate_wrapper_mjs(
    job_dir: &str,
    w_id: &str,
    job_id: &Uuid,
    worker_name: &str,
    db: &Connection,
    timeout: Option<i32>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    common_bun_proc_envs: &HashMap<String, String>,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
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

    #[cfg(windows)]
    child.env("SystemRoot", SYSTEM_ROOT.as_str());

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
        occupancy_metrics,
        None,
    )
    .await?;
    fs::rename(
        format!("{job_dir}/wrapper.js"),
        format!("{job_dir}/wrapper.mjs"),
    )
    .map_err(|e| error::Error::internal_err(format!("Could not move wrapper to mjs: {e:#}")))?;
    Ok(())
}

pub async fn generate_bun_bundle(
    job_dir: &str,
    w_id: &str,
    job_id: &Uuid,
    worker_name: &str,
    db: Option<&Connection>,
    timeout: Option<i32>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    common_bun_proc_envs: &HashMap<String, String>,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
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

    #[cfg(windows)]
    child.env("SystemRoot", SYSTEM_ROOT.as_str());

    let mut child_process = start_child_process(child, &*BUN_PATH).await?;
    if let Some(db) = db {
        handle_child(
            job_id,
            &db,
            mem_peak,
            canceled_by,
            child_process,
            false,
            worker_name,
            w_id,
            "bun build",
            timeout,
            false,
            occupancy_metrics,
            None,
        )
        .await?;
    } else {
        Box::into_pin(child_process.wait()).await?;
    }
    Ok(())
}

pub async fn pull_codebase(w_id: &str, id: &str, job_dir: &str) -> Result<()> {
    let path = windmill_common::s3_helpers::bundle(&w_id, &id);
    let bun_cache_path = format!(
        "{}/{}",
        windmill_common::worker::ROOT_CACHE_NOMOUNT_DIR,
        path
    );
    let is_tar = id.ends_with(".tar");

    let dst = format!(
        "{job_dir}/{}",
        if is_tar { "codebase.tar" } else { "main.js" }
    );

    if std::fs::metadata(&bun_cache_path).is_ok() {
        tracing::info!("loading {bun_cache_path} from cache");
        extract_saved_codebase(job_dir, &bun_cache_path, is_tar, &dst, false)?;
    } else {
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        let object_store = windmill_common::s3_helpers::get_object_store().await;

        #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
        let object_store: Option<()> = None;

        if &windmill_common::utils::MODE_AND_ADDONS.mode
            == &windmill_common::utils::Mode::Standalone
            && object_store.is_none()
        {
            let bun_cache_path = format!(
                "{}{}",
                *windmill_common::worker::ROOT_STANDALONE_BUNDLE_DIR,
                id
            );
            if std::fs::metadata(&bun_cache_path).is_ok() {
                tracing::info!("loading {bun_cache_path} from standalone bundle cache");
                extract_saved_codebase(job_dir, &bun_cache_path, is_tar, &dst, true)?;
            } else {
                return Err(error::Error::ExecutionErr(format!(
                    "(standalone bundle test mode) could not find codebase at {bun_cache_path}"
                )));
            }
        } else {
            #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
            return Err(error::Error::ExecutionErr(
                "codebase is an EE feature".to_string(),
            ));

            #[cfg(all(feature = "enterprise", feature = "parquet"))]
            if let Some(os) = object_store {
                let dirs_splitted = bun_cache_path.split("/").collect_vec();
                std::fs::create_dir_all(dirs_splitted[..dirs_splitted.len() - 1].join("/"))?;

                let bytes = attempt_fetch_bytes(os, &path).await?;
                tracing::info!("loading {bun_cache_path} from object store");

                std::fs::write(&bun_cache_path, &bytes)?;
                extract_saved_codebase(job_dir, &bun_cache_path, is_tar, &dst, false)?;
            }
        }
    }

    Ok(())
}

fn extract_saved_codebase(
    job_dir: &str,
    bun_cache_path: &String,
    is_tar: bool,
    dst: &str,
    copy: bool,
) -> Result<()> {
    use crate::global_cache::extract_tar;

    Ok(if is_tar {
        extract_tar(fs::read(bun_cache_path)?.into(), job_dir)?;
    } else {
        if copy {
            std::fs::copy(bun_cache_path, dst)?;
        } else {
            #[cfg(unix)]
            std::os::unix::fs::symlink(bun_cache_path, dst)?;

            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(bun_cache_path, dst)?;
        }
    })
}

pub async fn prebundle_bun_script(
    inner_content: &str,
    lockfile: Option<&String>,
    script_path: &str,
    job_id: &Uuid,
    w_id: &str,
    db: Option<&DB>,
    job_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    token: &str,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
) -> Result<()> {
    let (local_path, remote_path) =
        compute_bundle_local_and_remote_path(inner_content, lockfile, script_path, db, w_id).await;
    if exists_in_cache(&local_path, &remote_path).await {
        return Ok(());
    }
    let annotation = windmill_common::worker::TypeScriptAnnotations::parse(inner_content);
    if annotation.nobundling || *DISABLE_BUNDLING {
        return Ok(());
    }
    let origin = format!("{job_dir}/main.js");

    write_file(job_dir, "main.ts", &remove_pinned_imports(inner_content)?)?;
    build_loader(
        job_dir,
        base_internal_url,
        &token,
        w_id,
        script_path,
        if annotation.nodejs {
            LoaderMode::NodeBundle
        } else if annotation.native {
            LoaderMode::BrowserBundle
        } else {
            LoaderMode::BunBundle
        },
    )
    .await?;

    let common_bun_proc_envs: HashMap<String, String> = get_common_bun_proc_envs(None).await;

    generate_bun_bundle(
        job_dir,
        w_id,
        job_id,
        worker_name,
        db.map(|x| Connection::from(x.clone())).as_ref(),
        None,
        &mut 0,
        &mut None,
        &common_bun_proc_envs,
        occupancy_metrics,
    )
    .await?;

    save_cache(&local_path, &remote_path, &origin, false).await?;

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

pub async fn compute_bundle_local_and_remote_path(
    inner_content: &str,
    requirements_o: Option<&String>,
    script_path: &str,
    db: Option<&DB>,
    w_id: &str,
) -> (String, String) {
    let mut input_src = format!(
        "{}{}",
        inner_content,
        requirements_o.as_ref().map(|x| x.as_str()).unwrap_or("")
    );

    if let Some(db) = db {
        let relative_imports = crate::worker_lockfiles::extract_relative_imports(
            &inner_content,
            script_path,
            &Some(ScriptLang::Bun),
        );
        for path in relative_imports.unwrap_or_default() {
            if let Ok(updated_at) = get_script_import_updated_at(&db, w_id, &path).await {
                input_src.push_str(&path);
                input_src.push_str(&updated_at.to_string());
            }
        }
    };

    let hash = windmill_common::utils::calculate_hash(&input_src);
    let local_path = format!("{BUN_BUNDLE_CACHE_DIR}/{hash}");

    #[cfg(windows)]
    let local_path = local_path.replace("/tmp", r"C:\tmp").replace("/", r"\");

    let remote_path = format!("{BUN_BUNDLE_OBJECT_STORE_PREFIX}{hash}");
    (local_path, remote_path)
}

pub async fn prepare_job_dir(reqs: &str, job_dir: &str) -> Result<()> {
    let (pkg, lock, empty, is_binary) = split_lockfile(reqs);
    let _ = write_file(job_dir, "package.json", pkg)?;

    if !empty {
        if let Some(lock) = lock {
            let _ = write_lock(lock, job_dir, is_binary).await?;
        }
    }

    Ok(())
}
async fn write_lock(splitted_lockb_2: &str, job_dir: &str, is_binary: bool) -> Result<()> {
    if is_binary {
        write_file_binary(
            job_dir,
            "bun.lockb",
            &base64::engine::general_purpose::STANDARD
                .decode(splitted_lockb_2)
                .map_err(|_| error::Error::internal_err(format!("Could not decode bun.lockb")))?,
        )
        .await?;
    } else {
        write_file(job_dir, "bun.lock", splitted_lockb_2)?;
    };
    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_bun_job(
    requirements_o: Option<&String>,
    codebase: Option<&String>,
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
    shared_mount: &str,
    new_args: &mut Option<HashMap<String, Box<RawValue>>>,
    occupancy_metrics: &mut OccupancyMetrics,
    precomputed_agent_info: Option<PrecomputedAgentInfo>,
) -> error::Result<Box<RawValue>> {
    let mut annotation = windmill_common::worker::TypeScriptAnnotations::parse(inner_content);

    let (mut has_bundle_cache, cache_logs, local_path, remote_path) = if requirements_o.is_some()
        && !annotation.nobundling
        && !*DISABLE_BUNDLING
        && codebase.is_none()
    {
        let (local_path, remote_path) = match conn {
            Connection::Sql(db) => {
                compute_bundle_local_and_remote_path(
                    inner_content,
                    requirements_o,
                    job.runnable_path(),
                    Some(db),
                    &job.workspace_id,
                )
                .await
            }
            Connection::Http(_) => {
                let (local_path, remote_path) = match precomputed_agent_info {
                    Some(PrecomputedAgentInfo::Bun { local, remote }) => (local, remote),
                    _ => {
                        return Err(error::Error::ExecutionErr(
                            "bun bundle is missing the precomputed agent info".to_string(),
                        ))
                    }
                };
                (local_path, remote_path)
            }
        };

        let (cache, logs) =
            windmill_common::worker::load_cache(&local_path, &remote_path, false).await;
        (cache, logs, local_path, remote_path)
    } else {
        (false, "".to_string(), "".to_string(), "".to_string())
    };

    if !codebase.is_some() && !has_bundle_cache {
        let _ = write_file(job_dir, "main.ts", inner_content)?;
    } else if !annotation.native && codebase.is_none() {
        let _ = write_file(job_dir, "package.json", r#"{ "type": "module" }"#)?;
    };

    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(Some(&base_internal_url)).await;

    if codebase.is_some() {
        annotation.nodejs = true
    }
    let main_override = job.script_entrypoint_override.as_deref();
    let apply_preprocessor =
        job.flow_step_id.as_deref() != Some("preprocessor") && job.preprocessed == Some(false);

    if has_bundle_cache {
        let target;
        let symlink;

        #[cfg(unix)]
        {
            target = format!("{job_dir}/main.js");
            symlink = std::os::unix::fs::symlink(&local_path, &target);
        }
        #[cfg(windows)]
        {
            target = format!("{job_dir}\\main.js");
            symlink = std::fs::hard_link(&local_path, &target);
        }

        symlink.map_err(|e| {
            error::Error::ExecutionErr(format!(
                "could not copy cached binary from {local_path} to {job_dir}/main: {e:?}"
            ))
        })?;
    } else if let Some(codebase) = codebase.as_ref() {
        pull_codebase(&job.workspace_id, codebase, job_dir).await?;
    } else if let Some(reqs) = requirements_o.as_ref() {
        let (pkg, lock, empty, is_binary) = split_lockfile(reqs);

        if lock.is_none() && !annotation.npm {
            return Err(error::Error::ExecutionErr(
                format!("Invalid requirements, expected to find //bun.lock{} split pattern in reqs. Found: |{reqs}|", if is_binary {"b"} else {""})
            ));
        }

        let _ = write_file(job_dir, "package.json", pkg)?;
        let lock = if annotation.npm { "" } else { lock.unwrap() };
        if !empty {
            if !annotation.npm {
                let _ = write_lock(lock, job_dir, is_binary).await?;
            }

            install_bun_lockfile(
                mem_peak,
                canceled_by,
                &job.id,
                &job.workspace_id,
                Some(conn),
                job_dir,
                worker_name,
                common_bun_proc_envs.clone(),
                annotation.npm,
                &mut Some(occupancy_metrics),
            )
            .await?;
        }
    } else {
        // if !*DISABLE_NSJAIL || !empty_trusted_deps || has_custom_config_registry {
        let logs1 = "\n\n--- BUN INSTALL ---\n".to_string();
        append_logs(&job.id, &job.workspace_id, logs1, conn).await;
        let _ = gen_bun_lockfile(
            mem_peak,
            canceled_by,
            &job.id,
            &job.workspace_id,
            Some(conn),
            &client.token,
            job.runnable_path(),
            job_dir,
            base_internal_url,
            worker_name,
            false,
            None,
            annotation.npm,
            &mut Some(occupancy_metrics),
        )
        .await?;

        // }
    }

    let mut init_logs = if annotation.native {
        "\n\n--- NATIVE CODE EXECUTION ---\n".to_string()
    } else if has_bundle_cache {
        if annotation.nodejs {
            "\n\n--- NODE BUNDLE SNAPSHOT EXECUTION ---\n".to_string()
        } else {
            "\n\n--- BUN BUNDLE SNAPSHOT EXECUTION ---\n".to_string()
        }
    } else if codebase.is_some() {
        "\n\n--- NODE CODEBASE SNAPSHOT EXECUTION ---\n".to_string()
    } else if annotation.native {
        "\n\n--- NATIVE CODE EXECUTION ---\n".to_string()
    } else if annotation.nodejs {
        write_file(job_dir, "main.ts", &remove_pinned_imports(inner_content)?)?;
        "\n\n--- NODE CODE EXECUTION ---\n".to_string()
    } else {
        write_file(job_dir, "main.ts", &remove_pinned_imports(inner_content)?)?;
        "\n\n--- BUN CODE EXECUTION ---\n".to_string()
    };

    if has_bundle_cache {
        init_logs = format!("\n{}{}", cache_logs, init_logs);
    }

    let write_wrapper_f = async {
        if !has_bundle_cache && annotation.native {
            return Ok(()) as error::Result<()>;
        }
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
        // logs.push_str(format!("infer args: {:?}\n", start.elapsed().as_micros()).as_str());
        // we cannot use Bun.read and Bun.write because it results in an EBADF error on cloud
        let main_name = main_override.unwrap_or("main");

        let main_import = if codebase.is_some() || has_bundle_cache {
            "./main.js"
        } else {
            "./main.ts"
        };

        let preprocessor = if let Some(pre_args) = pre_args {
            let pre_spread = pre_args.into_iter().map(|x| x.name).join(",");
            format!(
                r#"if (Main.preprocessor === undefined || typeof Main.preprocessor !== 'function') {{
        throw new Error("preprocessor function is missing");
    }}
    function preArgsObjToArr({{ {pre_spread} }}) {{
        return [ {pre_spread} ];
    }}
    args = await Main.preprocessor(...preArgsObjToArr(args));
    const args_json = JSON.stringify(args ?? null, (key, value) => typeof value === 'undefined' ? null : value);
    await fs.writeFile('args.json', args_json, {{ encoding: 'utf8' }})"#
            )
        } else {
            "".to_string()
        };

        let wrapper_content = format!(
            r#"
import * as Main from "{main_import}";

import * as fs from "fs/promises";

let args = await fs.readFile('args.json', {{ encoding: 'utf8' }}).then(JSON.parse);

function argsObjToArr({{ {spread} }}) {{
    return [ {spread} ];
}}

BigInt.prototype.toJSON = function () {{
    return this.toString();
}};

async function run() {{
    {dates}
    {preprocessor}
    const argsArr = argsObjToArr(args);
    if (Main.{main_name} === undefined || typeof Main.{main_name} !== 'function') {{
        throw new Error("{main_name} function is missing");
    }}
    let res = await Main.{main_name}(...argsArr);
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
    const extra = {{}};
    Object.getOwnPropertyNames(e).forEach((key) => {{
        if (['line', 'name', 'stack', 'column', 'message', 'sourceURL', 'originalLine', 'originalColumn'].includes(key)) {{
            return;
        }}
        extra[key] = e[key];
    }});
    if (Object.keys(extra).length > 0) {{
        err["extra"] = extra;
    }}
    await fs.writeFile("result.json", JSON.stringify(err));
    process.exit(1);
}}
    "#,
        );
        write_file(job_dir, "wrapper.mjs", &wrapper_content)?;
        Ok(()) as error::Result<()>
    };

    let reserved_variables_args_out_f = async {
        let args_and_out_f = async {
            if !annotation.native {
                create_args_and_out_file(&client, job, job_dir, conn).await?;
            }
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

    let build_cache = !has_bundle_cache
        && !annotation.nobundling
        && !*DISABLE_BUNDLING
        && !codebase.is_some()
        && (requirements_o.is_some() || annotation.native);

    let write_loader_f = async {
        if build_cache {
            build_loader(
                job_dir,
                base_internal_url,
                &client.token,
                &job.workspace_id,
                job.runnable_path(),
                if annotation.nodejs {
                    LoaderMode::NodeBundle
                } else if annotation.native {
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
                &client.token,
                &job.workspace_id,
                job.runnable_path(),
                if annotation.nodejs {
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
                Some(conn),
                job.timeout,
                mem_peak,
                canceled_by,
                &common_bun_proc_envs,
                &mut Some(occupancy_metrics),
            )
            .await?;
            if !local_path.is_empty() {
                match save_cache(
                    &local_path,
                    &remote_path,
                    &format!("{job_dir}/main.js"),
                    false,
                )
                .await
                {
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
            if !annotation.native {
                let ex_wrapper = read_file_content(&format!("{job_dir}/wrapper.mjs")).await?;
                write_file(
                    job_dir,
                    "wrapper.mjs",
                    &ex_wrapper.replace(
                        "import * as Main from \"./main.ts\"",
                        "import * as Main from \"./main.js\"",
                    ),
                )?;
                write_file(job_dir, "package.json", r#"{ "type": "module" }"#)?;
            }
            fs::remove_file(format!("{job_dir}/main.ts"))?;
            has_bundle_cache = true;
        } else if annotation.nodejs {
            generate_wrapper_mjs(
                job_dir,
                &job.workspace_id,
                &job.id,
                worker_name,
                conn,
                job.timeout,
                mem_peak,
                canceled_by,
                &common_bun_proc_envs,
                &mut Some(occupancy_metrics),
            )
            .await?;
        }
    }
    if annotation.native {
        #[cfg(not(feature = "deno_core"))]
        {
            tracing::error!(
                r#""deno_core" feature is not activated, but "//native" annotation used. Returning empty value..."#
            );
            return Err(error::Error::internal_err("deno_core feature is not activated, but //native annotation used. Returning empty value...".to_string()));
        }

        #[cfg(feature = "deno_core")]
        {
            let env_code = format!(
            "const process = {{ env: {{}} }};\nconst BASE_URL = '{base_internal_url}';\nconst BASE_INTERNAL_URL = '{base_internal_url}';\nprocess.env['BASE_URL'] = BASE_URL;process.env['BASE_INTERNAL_URL'] = BASE_INTERNAL_URL;\n{}",
            reserved_variables
                .iter()
                .map(|(k, v)| format!("process.env['{}'] = '{}';\n", k, v))
                .collect::<Vec<String>>()
                .join("\n"));
            let js_code = read_file_content(&format!("{job_dir}/main.js")).await?;
            let started_at = Instant::now();
            let args = crate::common::build_args_map(job, client, conn)
                .await?
                .map(sqlx::types::Json);
            let job_args = if args.is_some() {
                args.as_ref()
            } else {
                job.args.as_ref()
            };

            append_logs(&job.id, &job.workspace_id, format!("{init_logs}\n"), conn).await;

            let result = crate::js_eval::eval_fetch_timeout(
                env_code,
                inner_content.clone(),
                js_code,
                job_args,
                job.script_entrypoint_override.clone(),
                job.id,
                job.timeout,
                conn,
                mem_peak,
                canceled_by,
                worker_name,
                &job.workspace_id,
                false,
                occupancy_metrics,
            )
            .await?;
            tracing::info!(
                "Executed native code in {}ms",
                started_at.elapsed().as_millis()
            );
            return Ok(result);
        }
    }
    append_logs(&job.id, &job.workspace_id, init_logs, conn).await;

    //do not cache local dependencies
    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_BUN_CONTENT
                .replace("{LANG}", if annotation.nodejs { "nodejs" } else { "bun" })
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace(
                    "{SHARED_MOUNT}",
                    &shared_mount.replace(
                        "/tmp/shared",
                        if annotation.nodejs {
                            "/tmp/nodejs/shared"
                        } else {
                            "/tmp/bun/shared"
                        },
                    ),
                ),
        )?;

        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        let args = if annotation.nodejs {
            vec![
                "--config",
                "run.config.proto",
                "--",
                &NODE_BIN_PATH,
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
        let cmd = if annotation.nodejs {
            let script_path = format!("{job_dir}/wrapper.mjs");

            let mut bun_cmd = Command::new(&*NODE_BIN_PATH);
            bun_cmd
                .current_dir(job_dir)
                .env_clear()
                .envs(envs)
                .envs(reserved_variables)
                .envs(common_bun_proc_envs)
                .args(vec!["--preserve-symlinks", &script_path])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            #[cfg(windows)]
            bun_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

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

            #[cfg(windows)]
            bun_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

            bun_cmd
        };

        start_child_process(
            cmd,
            if annotation.nodejs {
                &*NODE_BIN_PATH
            } else {
                &*BUN_PATH
            },
        )
        .await?
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
        "bun run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
    )
    .await?;

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
    read_result(job_dir).await
}

pub async fn get_common_bun_proc_envs(base_internal_url: Option<&str>) -> HashMap<String, String> {
    let mut bun_envs: HashMap<String, String> = HashMap::from([
        (String::from("PATH"), PATH_ENV.clone()),
        (String::from("HOME"), HOME_ENV.clone()),
        (String::from("TZ"), TZ_ENV.clone()),
        (String::from("FORCE_COLOR"), "1".to_string()),
        (String::from("DO_NOT_TRACK"), "1".to_string()),
        (
            String::from("BUN_INSTALL_CACHE_DIR"),
            BUN_CACHE_DIR.to_string(),
        ),
        (
            String::from("BUN_RUNTIME_TRANSPILER_CACHE_PATH"),
            "0".to_string(),
        ),
    ]);

    if let Some(base_url) = base_internal_url {
        bun_envs.insert(
            String::from("BASE_URL"),
            base_url.to_string().replace("localhost", "127.0.0.1"),
        );
    }
    if let Some(ref node_path) = NODE_PATH.as_ref() {
        bun_envs.insert(String::from("NODE_PATH"), node_path.to_string());
    }

    #[cfg(windows)]
    {
        bun_envs.insert("SystemRoot".to_string(), crate::SYSTEM_ROOT.to_string());
        bun_envs.insert(
            "USERPROFILE".to_string(),
            crate::USERPROFILE_ENV.to_string(),
        );
    }

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
    jobs_rx: Receiver<std::sync::Arc<MiniPulledJob>>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<()> {
    let mut logs = "".to_string();
    let mut mem_peak: i32 = 0;
    let mut canceled_by: Option<CanceledBy> = None;
    tracing::info!("Starting worker {w_id};{script_path} (codebase: {codebase:?}");
    if !codebase.is_some() {
        let _ = write_file(job_dir, "main.ts", inner_content)?;
    }

    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(Some(&base_internal_url)).await;

    let mut annotation = windmill_common::worker::TypeScriptAnnotations::parse(inner_content);

    //TODO: remove this when bun dedicated workers work without issues
    annotation.nodejs = true;

    let context = variables::get_reserved_variables(
        &Connection::from(db.clone()),
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
        let (pkg, lock, empty, is_binary) = split_lockfile(&reqs);
        if lock.is_none() {
            return Err(error::Error::ExecutionErr(
                format!("Invalid requirements, expected to find //bun.lockb split pattern in reqs. Found: |{reqs}|")
            ));
        }
        let _ = write_file(job_dir, "package.json", pkg)?;
        let lock = lock.unwrap();
        if !empty {
            if is_binary {
                let _ = write_file_binary(
                    job_dir,
                    "bun.lockb",
                    &base64::engine::general_purpose::STANDARD
                        .decode(lock)
                        .map_err(|_| {
                            error::Error::internal_err("Could not decode bun.lockb".to_string())
                        })?,
                )
                .await?;
            } else {
                write_file(job_dir, "bun.lock", lock)?;
            }

            install_bun_lockfile(
                &mut mem_peak,
                &mut canceled_by,
                &Uuid::nil(),
                &w_id,
                Some(&Connection::from(db.clone())),
                job_dir,
                worker_name,
                common_bun_proc_envs.clone(),
                annotation.npm,
                &mut None,
            )
            .await?;
            tracing::info!("dedicated worker requirements installed: {reqs}");
        }
    } else if !*DISABLE_NSJAIL {
        logs.push_str("\n\n--- BUN INSTALL ---\n");
        let _ = gen_bun_lockfile(
            &mut mem_peak,
            &mut canceled_by,
            &Uuid::nil(),
            &w_id,
            Some(&Connection::from(db.clone())),
            token,
            &script_path,
            job_dir,
            base_internal_url,
            worker_name,
            false,
            None,
            annotation.npm,
            &mut None,
        )
        .await?;
    }

    let main_code = remove_pinned_imports(inner_content)?;
    let _ = write_file(job_dir, "main.ts", &main_code)?;

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
        write_file(job_dir, "wrapper.mjs", &wrapper_content)?;
    }

    if !codebase.is_some() {
        build_loader(
            job_dir,
            base_internal_url,
            token,
            w_id,
            script_path,
            if annotation.nodejs {
                LoaderMode::Node
            } else {
                LoaderMode::Bun
            },
        )
        .await?;
    }

    if annotation.nodejs && !codebase.is_some() {
        generate_wrapper_mjs(
            job_dir,
            w_id,
            &Uuid::nil(),
            worker_name,
            &Connection::from(db.clone()),
            None,
            &mut mem_peak,
            &mut canceled_by,
            &common_bun_proc_envs,
            &mut None,
        )
        .await?;
    }

    if annotation.nodejs {
        let script_path = format!("{job_dir}/wrapper.mjs");

        handle_dedicated_process(
            &*NODE_BIN_PATH,
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
