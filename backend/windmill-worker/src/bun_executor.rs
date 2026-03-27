#[cfg(feature = "deno_core")]
use std::time::Instant;
use std::{collections::HashMap, fs, process::Stdio};

use base64::Engine;
use itertools::Itertools;

use serde_json::value::RawValue;

use uuid::Uuid;
use windmill_parser_ts::remove_pinned_imports;

use windmill_queue::{append_logs, CanceledBy, MiniPulledJob, PrecomputedAgentInfo};

use crate::{
    common::{
        build_command_with_isolation, create_args_and_out_file, get_reserved_variables,
        parse_npm_config, read_file, read_file_content, read_result, resolve_nsjail_timeout,
        start_child_process, write_file_binary, MaybeLock, OccupancyMetrics, StreamNotifier,
        DEV_CONF_NSJAIL,
    },
    get_proxy_envs_for_lang,
    handle_child::handle_child,
    is_sandboxing_enabled, read_ee_registry_with_workspace_override, BUNFIG_INSTALL_SCOPES,
    BUN_BUNDLE_CACHE_DIR, BUN_CACHE_DIR, BUN_NO_CACHE, BUN_PATH, DISABLE_NUSER, HOME_ENV,
    NODE_BIN_PATH, NODE_PATH, NPMRC, NPM_CONFIG_REGISTRY, NPM_PATH, NSJAIL_AVAILABLE, NSJAIL_PATH,
    PATH_ENV, PROXY_ENVS, TRACING_PROXY_CA_CERT_PATH, TZ_ENV,
};
use windmill_common::{
    client::AuthedClient,
    scripts::{id_to_codebase_info, CodebaseInfo, ScriptLang},
    utils::WarnAfterExt,
    workspace_dependencies::WorkspaceDependenciesPrefetched,
};
use windmill_types::s3::BundleFormat;

#[cfg(windows)]
use crate::SYSTEM_ROOT;

use tokio::{fs::File, process::Command};

use tokio::io::AsyncReadExt;

use windmill_common::{
    error::{self, Result},
    get_latest_hash_for_path,
    worker::{write_file, Connection, DISABLE_BUNDLING},
    DB,
};

use crate::global_cache::{exists_in_cache, save_cache};
#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_object_store::attempt_fetch_bytes;

use windmill_parser::Typ;

// The Windows loader uses a virtual "windmill-url" namespace instead of writing .url
// files to disk, which avoids Windows path issues. The virtual namespace approach is
// likely better on all fronts but we keep the original .url-file loader on Linux to
// avoid breaking back-compat.
#[cfg(not(windows))]
pub const RELATIVE_BUN_LOADER: &str = include_str!("../loader.bun.js");
#[cfg(windows)]
pub const RELATIVE_BUN_LOADER: &str = include_str!("../loader.bun.windows.js");

pub const RELATIVE_BUN_BUILDER: &str = include_str!("../loader_builder.bun.js");

const NSJAIL_CONFIG_RUN_BUN_CONTENT: &str = include_str!("../nsjail/run.bun.config.proto");

pub const BUN_LOCK_SPLIT: &str = "\n//bun.lock\n";
pub const BUN_LOCKB_SPLIT: &str = "\n//bun.lockb\n";
pub const BUN_LOCK_SPLIT_WINDOWS: &str = "\r\n//bun.lock\r\n";
pub const BUN_LOCKB_SPLIT_WINDOWS: &str = "\r\n//bun.lockb\r\n";

pub const EMPTY_FILE: &str = "<empty>";

/// Bun args for dedicated worker (without the script path)
pub const BUN_DEDICATED_WORKER_ARGS: &[&str] = &["run", "-i", "--prefer-offline"];

/// Pre-computed codegen data for a TypeScript/Bun/Deno script.
/// Computed in Rust from the parsed signature, then baked into the wrapper template.
#[cfg(any(feature = "private", test))]
pub struct TsScriptCodegen {
    pub spread: String,
    pub date_conversions: String,
    pub preprocessor_spread: Option<String>,
    pub preprocessor_date_conversions: Option<String>,
}

/// Parse a TS script and compute the codegen data (arg spread, date conversions, preprocessor).
/// This is the same logic that was used on main in `start_worker`.
#[cfg(any(feature = "private", test))]
pub fn compute_ts_codegen(content: &str) -> TsScriptCodegen {
    let sig =
        windmill_parser_ts::parse_deno_signature(content, true, false, None).unwrap_or_default();
    let arg_names: Vec<&str> = sig.args.iter().map(|a| a.name.as_str()).collect();
    let spread = arg_names.join(", ");

    let dates = sig
        .args
        .iter()
        .filter(|a| matches!(a.typ, Typ::Datetime))
        .map(|a| {
            format!(
                "{name} = {name} ? new Date({name}) : undefined",
                name = a.name
            )
        })
        .join("\n    ");

    let pre_sig = windmill_parser_ts::parse_deno_signature(
        content,
        true,
        false,
        Some("preprocessor".to_string()),
    )
    .ok()
    .filter(|s| !s.args.is_empty());

    let preprocessor_spread = pre_sig
        .as_ref()
        .map(|s| s.args.iter().map(|a| a.name.as_str()).join(", "));
    let preprocessor_date_conversions = pre_sig.as_ref().map(|s| {
        s.args
            .iter()
            .filter(|a| matches!(a.typ, Typ::Datetime))
            .map(|a| {
                format!(
                    "{name} = {name} ? new Date({name}) : undefined",
                    name = a.name
                )
            })
            .join("\n    ")
    });

    TsScriptCodegen {
        spread,
        date_conversions: dates,
        preprocessor_spread,
        preprocessor_date_conversions,
    }
}

/// Script entry for the unified wrapper generator.
/// `import_name`: the file stem used in the import path (e.g., "main" → `./main.ts`, or "f__script" → `./f__script.ts`)
#[cfg(any(feature = "private", test))]
pub struct TsScriptEntry<'a> {
    pub import_name: &'a str,
    pub original_path: &'a str,
    pub codegen: &'a TsScriptCodegen,
}

/// Generate a wrapper for dedicated workers and runner groups.
/// All scripts are baked in at codegen time with static imports and inline arg handling.
/// Protocol:
///   exec:<path>:<json_args>         -> wm_res[success]:<result> | wm_res[error]:<err>
///   exec_preprocess:<path>:<json>   -> wm_res[preprocessed_args]:<result> then wm_res[success]:<result> | wm_res[error]:<err>
///   end                             -> exit
#[cfg(any(feature = "private", test))]
pub fn generate_multi_script_wrapper(scripts: &[TsScriptEntry<'_>], ext: &str) -> String {
    let is_debug = std::env::var("RUST_LOG").is_ok_and(|x| x == "windmill=debug");
    let print_lines = if is_debug {
        r#"console.log("[debug] " + line);"#
    } else {
        ""
    };

    let imports: String = scripts
        .iter()
        .enumerate()
        .map(|(i, e)| {
            format!(
                "import * as _s{i} from \"./{import_name}.{ext}\";",
                import_name = e.import_name
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Generate per-script getArgs / getPreArgs functions
    let mut functions = String::new();
    let mut registrations = String::new();

    for (i, entry) in scripts.iter().enumerate() {
        let cg = entry.codegen;
        let spread = &cg.spread;
        let dates = &cg.date_conversions;

        functions.push_str(&format!(
            r#"
function getArgs_{i}(line) {{
    let {{ {spread} }} = JSON.parse(line);
    {dates}
    return [ {spread} ];
}}
"#
        ));

        let pre_fn = if let Some(ref pre_spread) = cg.preprocessor_spread {
            let pre_dates = cg.preprocessor_date_conversions.as_deref().unwrap_or("");
            functions.push_str(&format!(
                r#"
function getPreArgs_{i}(line) {{
    let {{ {pre_spread} }} = JSON.parse(line);
    {pre_dates}
    return [ {pre_spread} ];
}}
"#
            ));
            format!("getPreArgs_{i}")
        } else {
            "null".to_string()
        };

        registrations.push_str(&format!(
            "scripts.set(\"{path}\", {{ module: _s{i}, getArgs: getArgs_{i}, getPreArgs: {pre_fn} }});\n",
            path = entry.original_path,
        ));
    }

    format!(
        r#"
{imports}
import * as Readline from "node:readline"

BigInt.prototype.toJSON = function () {{
    return this.toString();
}};

const scripts = new Map();
{functions}
{registrations}

console.log('start');

for await (const line of Readline.createInterface({{ input: process.stdin }})) {{
    {print_lines}

    if (line === "end") {{
        process.exit(0);
    }}

    if (line.startsWith("exec_preprocess:")) {{
        const rest = line.slice("exec_preprocess:".length);
        const colonIdx = rest.indexOf(":");
        if (colonIdx === -1) {{
            console.log("wm_res[error]:" + JSON.stringify({{ message: "Malformed exec_preprocess command: missing colon separator", name: "Error" }}));
            continue;
        }}
        const scriptPath = rest.slice(0, colonIdx);
        const argsJson = rest.slice(colonIdx + 1);

        const entry = scripts.get(scriptPath);
        if (!entry) {{
            console.log("wm_res[error]:" + JSON.stringify({{ message: "Script not found: " + scriptPath, name: "Error" }}));
            continue;
        }}

        try {{
            if (!entry.getPreArgs) {{
                console.log("wm_res[error]:" + JSON.stringify({{ message: "preprocessor function is missing", name: "Error" }}));
                continue;
            }}
            const preArgs = entry.getPreArgs(argsJson);
            const preprocessedArgs = await entry.module.preprocessor(...preArgs);
            console.log("wm_res[preprocessed_args]:" + JSON.stringify(preprocessedArgs ?? {{}}, (key, value) => typeof value === 'undefined' ? null : value));
            const mainArgs = entry.getArgs(JSON.stringify(preprocessedArgs ?? {{}}));
            const res = await entry.module.main(...mainArgs);
            console.log("wm_res[success]:" + JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value));
        }} catch (e) {{
            console.log("wm_res[error]:" + JSON.stringify({{ message: e.message, name: e.name, stack: e.stack, line: argsJson }}));
        }}
        continue;
    }}

    if (line.startsWith("exec:")) {{
        const rest = line.slice("exec:".length);
        const colonIdx = rest.indexOf(":");
        if (colonIdx === -1) {{
            console.log("wm_res[error]:" + JSON.stringify({{ message: "Malformed exec command: missing colon separator", name: "Error" }}));
            continue;
        }}
        const scriptPath = rest.slice(0, colonIdx);
        const argsJson = rest.slice(colonIdx + 1);

        const entry = scripts.get(scriptPath);
        if (!entry) {{
            console.log("wm_res[error]:" + JSON.stringify({{ message: "Script not found: " + scriptPath, name: "Error" }}));
            continue;
        }}

        try {{
            const args = entry.getArgs(argsJson);
            const res = await entry.module.main(...args);
            console.log("wm_res[success]:" + JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value));
        }} catch (e) {{
            console.log("wm_res[error]:" + JSON.stringify({{ message: e.message, name: e.name, stack: e.stack, line: argsJson }}));
        }}
        continue;
    }}

    console.error("Unknown command:", line);
}}
"#
    )
}

/// Returns (package.json, bun.lock(b), is_empty, is_binary)
pub(crate) fn split_lockfile(lockfile: &str) -> (&str, Option<&str>, bool, bool) {
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
    workspace_dependencies: &WorkspaceDependenciesPrefetched,
    npm_mode: bool,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    temp_script_refs: &Option<HashMap<String, String>>,
    quiet: bool,
) -> Result<Option<String>> {
    let common_bun_proc_envs: HashMap<String, String> = get_common_bun_proc_envs(None).await;

    let mut empty_deps = false;

    if let Some(package_json_content) = workspace_dependencies.get_bun()? {
        gen_bunfig(job_dir, job_id, w_id, db).await?;
        write_file(job_dir, "package.json", package_json_content.as_str())?;
    } else {
        let temp_refs_json = temp_script_refs
            .as_ref()
            .and_then(|m| serde_json::to_string(m).ok())
            .unwrap_or_else(|| "null".to_string());

        let loader = RELATIVE_BUN_LOADER
            .replace("W_ID", w_id)
            .replace("BASE_INTERNAL_URL", base_internal_url)
            .replace("TOKEN", token)
            .replace(
                "CURRENT_PATH",
                &crate::common::use_flow_root_path(script_path),
            )
            .replace("RAW_GET_ENDPOINT", "raw")
            .replace("TEMP_SCRIPT_REFS_PLACEHOLDER", &temp_refs_json);

        write_file(
            &job_dir,
            "build.js",
            &format!(
                r#"
{loader}

{RELATIVE_BUN_BUILDER}
"#
            ),
        )?;

        gen_bunfig(job_dir, job_id, w_id, db).await?;

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

        let mut child_process = start_child_process(child_cmd, &*BUN_PATH, false).await?;

        if let Some(db) = db {
            let mut quiet_buf = String::new();
            let result = handle_child(
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
                if quiet { Some(&mut quiet_buf) } else { None },
                None,
            )
            .await;
            if quiet && result.is_err() {
                append_logs(
                    job_id,
                    w_id,
                    format!("\n--- BUN BUILD (failed) ---\n{quiet_buf}"),
                    db,
                )
                .await;
            }
            result?;
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
            quiet,
        )
        .await?;
    } else {
        if !quiet {
            if let Some(db) = db {
                append_logs(job_id, w_id, "\nempty dependencies, skipping install", db).await;
            }
        }
    }

    if export_pkg {
        let mut content;
        {
            let mut file = File::open(format!("{job_dir}/package.json")).await?;
            let mut buf = String::default();
            file.read_to_string(&mut buf).await?;
            content = buf;
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

async fn gen_bunfig(
    job_dir: &str,
    job_id: &Uuid,
    w_id: &str,
    db: Option<&Connection>,
) -> Result<()> {
    let npmrc = if let Some(conn) = db {
        read_ee_registry_with_workspace_override(
            NPMRC.read().await.clone(),
            "npmrc",
            "npmrc",
            job_id,
            w_id,
            conn,
        )
        .await
    } else {
        NPMRC.read().await.clone()
    };

    if let Some(ref npmrc_content) = npmrc {
        if !npmrc_content.trim().is_empty() {
            tracing::debug!("Writing .npmrc for bun from npmrc setting");
            write_file(job_dir, ".npmrc", npmrc_content)?;
            return Ok(());
        }
    }

    let (registry, bunfig_install_scopes) = if let Some(conn) = db {
        (
            read_ee_registry_with_workspace_override(
                NPM_CONFIG_REGISTRY.read().await.clone(),
                "npm_config_registry",
                "npm registry",
                job_id,
                w_id,
                conn,
            )
            .await,
            read_ee_registry_with_workspace_override(
                BUNFIG_INSTALL_SCOPES.read().await.clone(),
                "bunfig_install_scopes",
                "bunfig install scopes",
                job_id,
                w_id,
                conn,
            )
            .await,
        )
    } else {
        (
            NPM_CONFIG_REGISTRY.read().await.clone(),
            BUNFIG_INSTALL_SCOPES.read().await.clone(),
        )
    };
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
    quiet: bool,
) -> Result<()> {
    let mut child_cmd = Command::new(if npm_mode { &*NPM_PATH } else { &*BUN_PATH });

    let mut args = vec!["install", "--save-text-lockfile"];

    let no_cache = !npm_mode && *BUN_NO_CACHE;

    if no_cache {
        args.push("--no-cache");
    }

    child_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .envs(common_bun_proc_envs)
        .envs(&*crate::worker::WHITELIST_ENVS)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    child_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

    let mut npm_logs = if npm_mode {
        "NPM mode\n".to_string()
    } else if no_cache {
        "Bun install with --no-cache flag (BUN_NO_CACHE=true)\n".to_string()
    } else {
        "".to_string()
    };

    let has_file = if npm_mode {
        let npmrc = if let Some(conn) = db {
            read_ee_registry_with_workspace_override(
                NPMRC.read().await.clone(),
                "npmrc",
                "npmrc",
                job_id,
                w_id,
                conn,
            )
            .await
        } else {
            NPMRC.read().await.clone()
        };

        if let Some(ref npmrc_content) = npmrc {
            if !npmrc_content.trim().is_empty() {
                npm_logs.push_str("Using .npmrc from instance settings\n");
                write_file(job_dir, ".npmrc", npmrc_content)?;
                true
            } else {
                false
            }
        } else {
            let registry = if let Some(conn) = db {
                read_ee_registry_with_workspace_override(
                    NPM_CONFIG_REGISTRY.read().await.clone(),
                    "npm_config_registry",
                    "npm registry",
                    job_id,
                    w_id,
                    conn,
                )
                .await
            } else {
                NPM_CONFIG_REGISTRY.read().await.clone()
            };
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
        }
    } else {
        false
    };

    if !quiet && (npm_mode || no_cache) {
        if let Some(db) = db {
            append_logs(&job_id.clone(), w_id, npm_logs, db).await;
        }
    }

    if !has_file {
        gen_bunfig(job_dir, job_id, w_id, db).await?;
    }

    let mut child_process = start_child_process(child_cmd, &*BUN_PATH, false).await?;
    if let Some(db) = db {
        let mut quiet_buf = String::new();
        let result = handle_child(
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
            if quiet { Some(&mut quiet_buf) } else { None },
            None,
        )
        .warn_after_seconds(10)
        .await;
        if quiet && result.is_err() {
            // On failure, flush suppressed install output so the user can diagnose
            append_logs(
                job_id,
                w_id,
                format!("\n--- BUN INSTALL (failed) ---\n{quiet_buf}"),
                db,
            )
            .await;
        }
        result?;
    } else {
        Box::into_pin(child_process.wait()).await?;
    }

    if has_file {
        tokio::fs::remove_file(format!("{job_dir}/.npmrc")).await?;
    }

    Ok(())
}

#[derive(PartialEq)]
pub enum LoaderMode {
    Node,
    Bun,
    BunBundle,
    NodeBundle,
    BrowserBundle,
}
pub async fn build_loader(
    job_dir: &str,
    base_internal_url: &str,
    token: &str,
    w_id: &str,
    current_path: &str,
    mode: LoaderMode,
    temp_script_refs: &Option<HashMap<String, String>>,
) -> Result<()> {
    // Use forward slashes in JS strings to avoid backslash escape issues on Windows
    let job_dir_js = job_dir.replace('\\', "/");
    let temp_refs_json = temp_script_refs
        .as_ref()
        .and_then(|m| serde_json::to_string(m).ok())
        .unwrap_or_else(|| "null".to_string());

    let loader = RELATIVE_BUN_LOADER
        .replace("W_ID", w_id)
        .replace("BASE_INTERNAL_URL", base_internal_url)
        .replace("TOKEN", token)
        .replace(
            "CURRENT_PATH",
            &crate::common::use_flow_root_path(current_path),
        )
        .replace("RAW_GET_ENDPOINT", "raw_unpinned")
        .replace("TEMP_SCRIPT_REFS_PLACEHOLDER", &temp_refs_json);

    if mode == LoaderMode::Node {
        write_file(
            &job_dir,
            "node_builder.ts",
            &format!(
                r#"
{loader}

import {{ readdir }} from "node:fs/promises";

let fileNames = []
try {{
    fileNames = await readdir("{job_dir_js}/node_modules")
}} catch (e) {{
}}

try {{
    await Bun.build({{
        entrypoints: ["{job_dir_js}/wrapper.mjs"],
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
"#
            ),
        )?;
    } else if mode == LoaderMode::Bun {
        write_file(
            &job_dir,
            "loader.bun.js",
            &format!(
                r#"
import {{ plugin }} from "bun";

{loader}

plugin(p)
"#
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
{loader}

try {{
    await Bun.build({{
        entrypoints: ["{job_dir_js}/main.ts"],
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

    let child_process = start_child_process(child, &*BUN_PATH, false).await?;
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

    let mut child_process = start_child_process(child, &*BUN_PATH, false).await?;
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
            None,
        )
        .await?;
    } else {
        Box::into_pin(child_process.wait()).await?;
    }
    Ok(())
}

struct PulledCodebase {
    is_esm: bool,
}
async fn pull_codebase(w_id: &str, id: &str, job_dir: &str) -> Result<PulledCodebase> {
    let path = windmill_object_store::bundle(&w_id, &id);
    let CodebaseInfo { is_tar, is_esm } = id_to_codebase_info(id);

    let bun_cache_path = format!(
        "{}/{}.{}",
        *windmill_common::worker::ROOT_CACHE_NOMOUNT_DIR,
        path,
        if is_tar { "tar" } else { "js" }
    );

    let dst = format!(
        "{job_dir}/{}",
        if is_tar { "codebase.tar" } else { "main.js" }
    );

    if std::fs::metadata(&bun_cache_path).is_ok() {
        tracing::info!("loading {bun_cache_path} from cache");
        extract_saved_codebase(job_dir, &bun_cache_path, is_tar, &dst, false)?;
    } else {
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        let object_store = windmill_object_store::get_object_store().await;

        #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
        let object_store: Option<()> = None;

        if &windmill_common::utils::MODE_AND_ADDONS.mode
            == &windmill_common::utils::Mode::Standalone
            && object_store.is_none()
        {
            let bun_cache_path = format!(
                "{}/{}",
                *windmill_common::worker::ROOT_STANDALONE_BUNDLE_DIR,
                path
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

    Ok(PulledCodebase { is_esm })
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
    lock: &str,
    script_path: &str,
    job_id: &Uuid,
    w_id: &str,
    db: Option<&DB>,
    job_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    token: &str,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    temp_script_refs: &Option<HashMap<String, String>>,
) -> Result<()> {
    let (local_path, remote_path) =
        compute_bundle_local_and_remote_path(inner_content, lock, script_path, db, w_id).await;
    if exists_in_cache(&local_path, &remote_path).await {
        return Ok(());
    }
    let annotation = windmill_common::worker::TypeScriptAnnotations::parse(inner_content);
    if annotation.nobundling || *DISABLE_BUNDLING {
        return Ok(());
    }
    let origin = format!("{job_dir}/main.js");

    let mut content = remove_pinned_imports(inner_content)?;
    if crate::wac_executor::is_wac_v2_ts(inner_content) {
        content = crate::wac_executor::inject_wac_task_names(&content);
        content = format!("export {{ WorkflowCtx, StepSuspend, setWorkflowCtx }} from \"windmill-client\";\n{content}");
    }
    write_file(job_dir, "main.ts", &content)?;
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
        temp_script_refs,
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
    let script_hash = get_latest_hash_for_path(db, w_id, script_path, false).await?;
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
    lock: &str,
    script_path: &str,
    db: Option<&DB>,
    w_id: &str,
) -> (String, String) {
    let mut input_src = format!("{inner_content}{lock}",);

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

    let ws_suffix = crate::workspace_registry_cache_suffix(w_id).await;
    input_src.push_str(&ws_suffix);
    let hash = windmill_common::utils::calculate_hash(&input_src);
    let local_path = format!("{}/{hash}", *BUN_BUNDLE_CACHE_DIR);

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
    maybe_lock: MaybeLock,
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
    has_stream: &mut bool,
    modules: &Option<std::collections::HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> error::Result<Box<RawValue>> {
    let mut annotation = windmill_common::worker::TypeScriptAnnotations::parse(inner_content);

    if annotation.sandbox && NSJAIL_AVAILABLE.is_none() {
        return Err(error::Error::ExecutionErr(
            "Script has //sandbox annotation but nsjail is not available on this worker. \
            Please ensure nsjail is installed or remove the //sandbox annotation."
                .to_string(),
        ));
    }

    let (mut has_bundle_cache, cache_logs, local_path, remote_path) = if let (Some(lock), true) = (
        maybe_lock.get_lock(),
        !annotation.nobundling && !*DISABLE_BUNDLING && codebase.is_none(),
    ) {
        let (local_path, remote_path) = match conn {
            Connection::Sql(db) => {
                compute_bundle_local_and_remote_path(
                    inner_content,
                    lock,
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

        let (cache, logs) = crate::global_cache::load_cache(&local_path, &remote_path, false).await;
        (cache, logs, local_path, remote_path)
    } else {
        (false, "".to_string(), "".to_string(), "".to_string())
    };

    if !codebase.is_some() && !has_bundle_cache {
        let _ = write_file(job_dir, "main.ts", inner_content)?;
    } else if !annotation.native && codebase.is_none() {
        let _ = write_file(job_dir, "package.json", r#"{ "type": "module" }"#)?;
    } else if codebase.is_some() {
        // Write a valid fallback package.json for codebase mode. Without this,
        // nsjail creates an empty 0-byte file (from the mandatory: false mount)
        // which Node.js fails to parse as JSON (ERR_INVALID_PACKAGE_CONFIG).
        // If the codebase TAR includes a package.json, it will overwrite this.
        let _ = write_file(job_dir, "package.json", "{}")?;
    };

    let common_bun_proc_envs: HashMap<String, String> =
        get_common_bun_proc_envs(Some(&base_internal_url)).await;

    let main_override = job.script_entrypoint_override.as_deref();
    let apply_preprocessor =
        job.flow_step_id.as_deref() != Some("preprocessor") && job.preprocessed == Some(false);

    let is_wac_v2 = main_override.is_none() && crate::wac_executor::is_wac_v2_ts(inner_content);

    // Detect WAC v2 replay (resumed from suspend) to suppress verbose logs.
    // The actual step name is logged later by handle_wac_v2_output.
    let wac_replay_info: Option<String> = if is_wac_v2 {
        if let Connection::Sql(db) = conn {
            let checkpoint = crate::wac_executor::load_checkpoint(db, &job.id).await?;
            if !checkpoint.completed_steps.is_empty() {
                Some(String::new())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // For WAC v2, inject variable names into unnamed task() calls so the
    // runtime can use them for step naming (timeline, graph).
    // `const double = task(async ...` → `const double = task("double", async ...`
    // Also handles: export const, let, var, and optional generic type parameters.
    // Skips calls that already have a string argument: `task("path", async ...`
    let inner_content = if is_wac_v2 {
        crate::wac_executor::inject_wac_task_names(inner_content)
    } else {
        inner_content.to_string()
    };
    let inner_content = inner_content.as_str();

    let mut format = BundleFormat::Cjs;
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
        let pulled_codebase = pull_codebase(&job.workspace_id, codebase, job_dir).await?;
        if pulled_codebase.is_esm {
            format = BundleFormat::Esm;
        }
    } else {
        match &maybe_lock {
            MaybeLock::Resolved { lock } => {
                let (package_json, bun_lock, empty, is_binary) = split_lockfile(lock);

                if bun_lock.is_none() && !annotation.npm {
                    return Err(error::Error::ExecutionErr(
                        format!("Invalid requirements, expected to find //bun.lock{} split pattern in reqs. Found: |{lock}|", if is_binary {"b"} else {""})
                    ));
                }

                write_file(job_dir, "package.json", package_json)?;

                let bun_lock = if annotation.npm {
                    ""
                } else {
                    bun_lock.unwrap()
                };

                if !empty {
                    if !annotation.npm {
                        write_lock(bun_lock, job_dir, is_binary).await?;
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
                        wac_replay_info.is_some(),
                    )
                    .await?;
                }
            }
            MaybeLock::Unresolved { ref workspace_dependencies } => {
                // if is_sandboxing_enabled() || !empty_trusted_deps || has_custom_config_registry {
                if wac_replay_info.is_none() {
                    let logs1 = "\n\n--- BUN INSTALL ---\n".to_string();
                    append_logs(&job.id, &job.workspace_id, logs1, conn).await;
                }
                gen_bun_lockfile(
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
                    workspace_dependencies,
                    annotation.npm,
                    &mut Some(occupancy_metrics),
                    &None,
                    wac_replay_info.is_some(),
                )
                .await?;

                // }
            }
        }
    }

    if codebase.is_some() && format == BundleFormat::Cjs {
        annotation.nodejs = true
    }

    let mut init_logs = if let Some(ref replay_header) = wac_replay_info {
        // WAC v2 replay: use concise header, but still write main.ts if needed
        if !annotation.native && !has_bundle_cache && codebase.is_none() {
            write_file(job_dir, "main.ts", &remove_pinned_imports(inner_content)?)?;
        }
        replay_header.clone()
    } else if annotation.native {
        "\n\n--- NATIVE CODE EXECUTION ---\n".to_string()
    } else if has_bundle_cache {
        if annotation.nodejs {
            "\n\n--- NODE BUNDLE SNAPSHOT EXECUTION ---\n".to_string()
        } else {
            "\n\n--- BUN BUNDLE SNAPSHOT EXECUTION ---\n".to_string()
        }
    } else if codebase.is_some() {
        if format == BundleFormat::Esm {
            "\n\n--- ESM CODEBASE SNAPSHOT EXECUTION ---\n".to_string()
        } else {
            "\n\n--- CJS CODEBASE SNAPSHOT EXECUTION ---\n".to_string()
        }
    } else if annotation.native {
        "\n\n--- NATIVE CODE EXECUTION ---\n".to_string()
    } else if annotation.nodejs {
        write_file(job_dir, "main.ts", &remove_pinned_imports(inner_content)?)?;
        "\n\n--- NODE CODE EXECUTION ---\n".to_string()
    } else {
        write_file(job_dir, "main.ts", &remove_pinned_imports(inner_content)?)?;
        // Module inlining has two phases:
        // 1. BUILD phase: loader.bun.js checks for local module files on disk (written by
        //    write_module_files) and resolves them directly, so they get inlined into the bundle.
        // 2. RUN phase: overwrite main.ts with the bundled output below. The runtime wrapper
        //    imports main.ts, which now contains the inlined modules from the build step.
        if modules.as_ref().is_some_and(|m| !m.is_empty()) {
            let bundle_path = std::path::Path::new(job_dir).join("out").join("main.js");
            if bundle_path.exists() {
                let bundled = std::fs::read_to_string(&bundle_path)?;
                write_file(job_dir, "main.ts", &bundled)?;
            }
        }
        "\n\n--- BUN CODE EXECUTION ---\n".to_string()
    };

    if has_bundle_cache {
        init_logs = format!("\n{}{}", cache_logs, init_logs);
    }

    if annotation.sandbox {
        init_logs.push_str("sandbox mode (nsjail)\n");
    }

    let write_wrapper_f = async {
        if !has_bundle_cache && annotation.native {
            return Ok(()) as error::Result<()>;
        }
        // let mut start = Instant::now();
        let args = if is_wac_v2 {
            // For WAC v2, try to parse "main" args; if that fails, try the default export
            windmill_parser_ts::parse_deno_signature(inner_content, true, false, None)
                .unwrap_or_default()
                .args
        } else {
            windmill_parser_ts::parse_deno_signature(
                inner_content,
                true,
                false,
                main_override.map(ToString::to_string),
            )?
            .args
        };

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

        // For WAC child jobs where the parser can't find params (task-wrapped consts),
        // fall back to passing arg values directly (filtering out internal fields)
        let child_spread = if spread.is_empty() && main_override.is_some() {
            "Object.values(Object.fromEntries(Object.entries(args).filter(([k]) => !k.startsWith('_'))))".to_string()
        } else {
            "argsObjToArr(args)".to_string()
        };

        let main_import = if codebase.is_some() || has_bundle_cache {
            "./main.js"
        } else {
            "./main.ts"
        };

        let wac_client_import = if has_bundle_cache {
            "./main.js"
        } else {
            "windmill-client"
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

        let wac_spread = if spread.is_empty() {
            "Object.values(args)".to_string()
        } else {
            format!("argsObjToArr(args)")
        };

        let wrapper_content = if is_wac_v2 {
            format!(
                r#"
import * as Main from "{main_import}";
import {{ WorkflowCtx, StepSuspend, setWorkflowCtx }} from "{wac_client_import}";

import * as fs from "fs/promises";

let args = await fs.readFile('args.json', {{ encoding: 'utf8' }}).then(JSON.parse);
const checkpoint = JSON.parse(await fs.readFile('checkpoint.json', {{ encoding: 'utf8' }}));

function argsObjToArr({{ {spread} }}) {{
    return [ {spread} ];
}}

BigInt.prototype.toJSON = function () {{
    return this.toString();
}};

// Find the workflow entrypoint (export default)
let workflowFn = Main.default;
if (!workflowFn || !workflowFn._is_workflow) {{
    for (const key of Object.keys(Main)) {{
        if (Main[key]?._is_workflow) {{
            workflowFn = Main[key];
            break;
        }}
    }}
}}
if (!workflowFn) {{
    throw new Error("No workflow() entrypoint found. Wrap your main function with workflow().");
}}

async function run() {{
    {dates}
    {preprocessor}
    const argsArr = {wac_spread};

    const ctx = new WorkflowCtx(checkpoint);
    setWorkflowCtx(ctx);

    try {{
        const result = await workflowFn(...argsArr);
        setWorkflowCtx(null);
        // Flush any unawaited tasks (e.g. forgotten await on last statement)
        const trailing = ctx._flushPending();
        if (trailing.length > 0) {{
            return {{ type: "dispatch", mode: trailing.length > 1 ? "parallel" : "sequential", steps: trailing }};
        }}
        return {{ type: "complete", result: result ?? null }};
    }} catch (e) {{
        setWorkflowCtx(null);
        if (e?.name === "StepSuspend" || e instanceof StepSuspend) {{
            const dispatch = e.dispatchInfo ?? e.dispatch_info ?? {{}};
            if (dispatch.mode === "step_complete") {{
                return {{ type: "complete", result: dispatch.result ?? null }};
            }}
            if (dispatch.mode === "inline_checkpoint") {{
                return {{ type: "inline_checkpoint", key: dispatch.key, result: dispatch.result ?? null, started_at: dispatch.started_at, duration_ms: dispatch.duration_ms }};
            }}
            if (dispatch.mode === "approval") {{
                return {{ type: "approval", key: dispatch.key, timeout: dispatch.timeout, form: dispatch.form, self_approval_disabled: dispatch.self_approval_disabled }};
            }}
            if (dispatch.mode === "sleep") {{
                return {{ type: "sleep", key: dispatch.key, seconds: dispatch.seconds }};
            }}
            return {{ type: "dispatch", mode: dispatch.mode ?? "sequential", steps: dispatch.steps ?? [] }};
        }}
        throw e;
    }}
}}

try {{
    const output = await run();
    if (output.type === "complete") {{
        console.log(`\n--- WAC: complete ---`);
    }}
    const output_json = JSON.stringify(output, (key, value) =>
        typeof value === 'undefined' ? null : value
    );
    await fs.writeFile("result.json", output_json);
    process.exit(0);
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
            )
        } else {
            format!(
                r#"
import * as Main from "{main_import}";

import * as fs from "fs/promises";

let args = await fs.readFile('args.json', {{ encoding: 'utf8' }}).then(JSON.parse);

function argsObjToArr({{ {spread} }}) {{
    return [ {spread} ];
}}

function isAsyncIterable(obj) {{
    return obj != null && typeof obj[Symbol.asyncIterator] === 'function';
}}

BigInt.prototype.toJSON = function () {{
    return this.toString();
}};

async function run() {{
    {dates}
    {preprocessor}
    // If the entrypoint has no parsed params (spread is empty), pass values directly
    // This handles WAC child jobs where tasks are const-wrapped functions
    const argsArr = {child_spread};
    if (Main.{main_name} === undefined || typeof Main.{main_name} !== 'function') {{
        throw new Error("{main_name} function is missing");
    }}
    let entrypoint = Main.{main_name};
    let res = await entrypoint(...argsArr);
    if (isAsyncIterable(res)) {{
        for await (const chunk of res) {{
            console.log("WM_STREAM: " + chunk.replace(/\n/g, '\\n'));
        }}
        res = null;
    }}
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
            )
        };
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
        && (maybe_lock.get_lock().is_some() || annotation.native);

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
                &None,
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
                &None,
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

    // For WAC v2, write checkpoint.json before bun runs
    if is_wac_v2 {
        if let Connection::Sql(db) = conn {
            let checkpoint = crate::wac_executor::load_checkpoint(db, &job.id).await?;
            let checkpoint =
                crate::wac_executor::prepare_checkpoint_for_resume(db, &job.id, checkpoint).await?;

            let checkpoint_json = serde_json::to_string(&checkpoint).map_err(|e| {
                error::Error::internal_err(format!("Failed to serialize checkpoint: {e}"))
            })?;
            write_file(job_dir, "checkpoint.json", &checkpoint_json)?;
        } else {
            write_file(job_dir, "checkpoint.json", r#"{"completed_steps":{}}"#)?;
        }
    }

    // Prepend WAC re-exports to main.ts so the bundle includes WorkflowCtx etc.
    if build_cache && is_wac_v2 {
        let main_path = format!("{job_dir}/main.ts");
        let current = read_file_content(&main_path).await?;
        write_file(
            job_dir,
            "main.ts",
            &format!("export {{ WorkflowCtx, StepSuspend, setWorkflowCtx }} from \"windmill-client\";\n{current}"),
        )?;
    }

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
                let mut rewritten = ex_wrapper.replace(
                    "import * as Main from \"./main.ts\"",
                    "import * as Main from \"./main.js\"",
                );
                if is_wac_v2 {
                    rewritten = rewritten.replace("from \"windmill-client\"", "from \"./main.js\"");
                }
                write_file(job_dir, "wrapper.mjs", &rewritten)?;
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
            let env_code = build_nativets_env_code(base_internal_url, &reserved_variables);
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

            if apply_preprocessor {
                // First pass: run preprocessor function
                let pre_result = crate::js_eval::eval_fetch_timeout(
                    env_code.clone(),
                    inner_content.to_string(),
                    js_code.clone(),
                    job_args,
                    Some("preprocessor".to_string()),
                    job.id,
                    job.timeout,
                    conn,
                    mem_peak,
                    canceled_by,
                    worker_name,
                    &job.workspace_id,
                    false,
                    occupancy_metrics,
                    None,
                    has_stream,
                )
                .await?;

                let preprocessed: HashMap<String, Box<RawValue>> =
                    serde_json::from_str(pre_result.get()).map_err(|e| {
                        error::Error::internal_err(format!(
                            "error deserializing preprocessed args: {e:#}"
                        ))
                    })?;
                *new_args = Some(preprocessed.clone());

                // Second pass: run main with preprocessed args
                let preprocessed_json = sqlx::types::Json(preprocessed);
                let stream_notifier = StreamNotifier::new(conn, job);

                let result = crate::js_eval::eval_fetch_timeout(
                    env_code,
                    inner_content.to_string(),
                    js_code,
                    Some(&preprocessed_json),
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
                    stream_notifier,
                    has_stream,
                )
                .await?;
                tracing::info!(
                    "Executed native code (with preprocessor) in {}ms",
                    started_at.elapsed().as_millis()
                );
                return Ok(result);
            }

            let stream_notifier = StreamNotifier::new(conn, job);

            let result = crate::js_eval::eval_fetch_timeout(
                env_code,
                inner_content.to_string(),
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
                stream_notifier,
                has_stream,
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
    let child = if is_sandboxing_enabled() || annotation.sandbox {
        let nsjail_timeout =
            resolve_nsjail_timeout(conn, &job.workspace_id, job.id, job.timeout).await;
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
                )
                .replace("{TRACING_PROXY_CA_CERT_PATH}", &*TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL)
                .replace("{TIMEOUT}", &nsjail_timeout),
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
            .envs(
                get_proxy_envs_for_lang(&ScriptLang::Bun, &job.id, &job.workspace_id, conn).await?,
            )
            .envs(common_bun_proc_envs)
            .env("PATH", PATH_ENV.as_str())
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        let cmd = if annotation.nodejs {
            let script_path = format!("{job_dir}/wrapper.mjs");
            let args = vec!["--preserve-symlinks", script_path.as_str()];

            let mut bun_cmd = build_command_with_isolation(&*NODE_BIN_PATH, &args);
            bun_cmd
                .current_dir(job_dir)
                .env_clear()
                .envs(envs)
                .envs(reserved_variables)
                .envs(
                    get_proxy_envs_for_lang(&ScriptLang::Bun, &job.id, &job.workspace_id, conn)
                        .await?,
                )
                .envs(common_bun_proc_envs)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            #[cfg(windows)]
            bun_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

            bun_cmd
        } else {
            let script_path = format!("{job_dir}/wrapper.mjs");

            let args: Vec<&str> = if codebase.is_some() || has_bundle_cache {
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
            let mut bun_cmd = build_command_with_isolation(&*BUN_PATH, &args);
            bun_cmd
                .current_dir(job_dir)
                .env_clear()
                .envs(envs)
                .envs(reserved_variables)
                .envs(
                    get_proxy_envs_for_lang(&ScriptLang::Bun, &job.id, &job.workspace_id, conn)
                        .await?,
                )
                .envs(common_bun_proc_envs)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            #[cfg(windows)]
            bun_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

            bun_cmd
        };

        let executable = if annotation.nodejs {
            &*NODE_BIN_PATH
        } else {
            &*BUN_PATH
        };
        start_child_process(cmd, executable, false).await?
    };

    let stream_notifier = StreamNotifier::new(conn, job);

    let handle_result = handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        is_sandboxing_enabled(),
        worker_name,
        &job.workspace_id,
        "bun run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        stream_notifier,
    )
    .await?;

    *has_stream = handle_result.result_stream.is_some();

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

    let result = read_result(job_dir, handle_result.result_stream).await?;

    // WAC v2 post-execution: parse output and handle dispatch/suspend
    if is_wac_v2 {
        return handle_wac_v2_output(result, job, conn, modules).await;
    }

    Ok(result)
}

/// Resolve a module file from the parent script's modules map.
/// For Script jobs, fetches from the `script` table by hash.
/// For Preview jobs, fetches from `v2_job.raw_code` (modules stored inline).
fn resolve_parent_module(
    modules: &Option<std::collections::HashMap<String, windmill_common::scripts::ScriptModule>>,
    module_key: &str,
) -> error::Result<windmill_common::scripts::ScriptModule> {
    if let Some(modules) = modules {
        if let Some(module) = modules.get(module_key) {
            return Ok(module.clone());
        }
    }
    Err(error::Error::ExecutionErr(format!(
        "Module '{}' not found in script modules",
        module_key
    )))
}

/// Handle WAC v2 output after bun/python exits. Parse result as WacOutput,
/// dispatch child jobs on suspend, or return the final result.
pub async fn handle_wac_v2_output(
    result: Box<RawValue>,
    job: &MiniPulledJob,
    conn: &Connection,
    modules: &Option<std::collections::HashMap<String, windmill_common::scripts::ScriptModule>>,
) -> error::Result<Box<RawValue>> {
    use crate::wac_executor::{
        add_completed_step, load_checkpoint, parse_wac_output, update_checkpoint_for_dispatch,
        WacOutput,
    };
    use serde_json::Value;
    use windmill_common::get_latest_flow_version_info_for_path;
    use windmill_common::jobs::{script_path_to_payload, JobKind, JobPayload, RawCode};
    use windmill_common::runnable_settings::{
        ConcurrencySettings, ConcurrencySettingsWithCustom, DebouncingSettings,
    };
    use windmill_queue::{push, PushArgs, PushIsolationLevel};

    let output = parse_wac_output(&result)?;

    match output {
        WacOutput::Complete { result: value } => {
            // Workflow completed — return the inner result value
            let raw = serde_json::value::to_raw_value(&value).map_err(|e| {
                error::Error::internal_err(format!("Failed to serialize WAC result: {e}"))
            })?;
            Ok(raw)
        }
        WacOutput::Dispatch { mode, steps } => {
            if steps.is_empty() {
                return Err(error::Error::internal_err(
                    "WAC v2 dispatch with no steps — this is a bug in the workflow SDK".to_string(),
                ));
            }
            let db = match conn {
                Connection::Sql(db) => db,
                _ => {
                    return Err(error::Error::internal_err(
                        "WAC v2 dispatch requires SQL connection".to_string(),
                    ))
                }
            };

            let mut checkpoint = load_checkpoint(db, &job.id).await?;

            // Source hash validation: detect if code changed between replays
            let current_hash = job.runnable_id.map(|h| h.0.to_string()).unwrap_or_default();
            if !current_hash.is_empty() {
                if checkpoint.source_hash.is_empty() {
                    checkpoint.source_hash = current_hash.clone();
                } else if checkpoint.source_hash != current_hash {
                    return Err(error::Error::ExecutionErr(
                        "Workflow source code changed between replays. \
                         Cannot safely resume from checkpoint — step keys may have shifted. \
                         Please restart this workflow."
                            .to_string(),
                    ));
                }
            }
            let num_steps = steps.len();

            tracing::info!(
                job_id = %job.id,
                mode = %mode,
                num_steps = num_steps,
                steps = ?steps.iter().map(|s| &s.name).collect::<Vec<_>>(),
                "WAC v2 dispatching child jobs"
            );

            // Create child jobs for each step.
            // Each child re-runs the full workflow with a checkpoint containing
            // _executing_key = step_key, so only that step runs its inner function.
            //
            // IMPORTANT: To prevent a race condition where a fast child completes
            // before the parent is suspended, we:
            //   1. Pre-generate child UUIDs
            //   2. Save checkpoint + suspend parent + seed child checkpoints
            //   3. THEN push the child jobs (making them visible to workers)

            // Read the parent's original args for the child jobs
            let parent_args: HashMap<String, Box<RawValue>> = {
                let stored: serde_json::Map<String, Value> = checkpoint.input_args.clone();
                if stored.is_empty() {
                    // First dispatch — read from the parent job's args
                    let row: Option<Value> = sqlx::query_scalar(
                        "SELECT args FROM v2_job WHERE id = $1 AND workspace_id = $2",
                    )
                    .bind(&job.id)
                    .bind(&job.workspace_id)
                    .fetch_optional(db)
                    .await?;
                    let args_val = row.unwrap_or(Value::Object(Default::default()));
                    if let Value::Object(map) = args_val {
                        // Store for future re-runs
                        checkpoint.input_args = map.clone();
                        map.into_iter()
                            .map(|(k, v)| {
                                let raw = serde_json::value::to_raw_value(&v).map_err(|e| {
                                    error::Error::internal_err(format!(
                                        "Failed to serialize arg '{k}': {e}"
                                    ))
                                })?;
                                Ok((k, raw))
                            })
                            .collect::<error::Result<HashMap<_, _>>>()?
                    } else {
                        HashMap::new()
                    }
                } else {
                    stored
                        .into_iter()
                        .map(|(k, v)| {
                            let raw = serde_json::value::to_raw_value(&v).map_err(|e| {
                                error::Error::internal_err(format!(
                                    "Failed to serialize arg '{k}': {e}"
                                ))
                            })?;
                            Ok((k, raw))
                        })
                        .collect::<error::Result<HashMap<_, _>>>()?
                }
            };

            // Pre-generate child UUIDs so we can save them in the checkpoint
            // before the children become visible to workers.
            // Validate key uniqueness — duplicate keys would cause one child's
            // UUID to be overwritten in the job_ids map, making it unmappable
            // on completion (the parent would hang).
            {
                let mut seen_keys = std::collections::HashSet::new();
                for s in &steps {
                    if !seen_keys.insert(&s.key) {
                        return Err(error::Error::internal_err(format!(
                            "WAC v2 duplicate step key '{}' — each task call must produce a unique key",
                            s.key
                        )));
                    }
                }
            }
            let job_ids: Vec<(String, Uuid)> = steps
                .iter()
                .map(|s| (s.key.clone(), ulid::Ulid::new().into()))
                .collect();

            // Resolve job_payload once (same for all children since they re-run
            // the parent script)
            let job_payload_template = match job.kind {
                JobKind::Script => {
                    if let Some(hash) = job.runnable_id {
                        Ok(JobPayload::ScriptHash {
                            hash,
                            path: job.runnable_path.clone().unwrap_or_default(),
                            cache_ttl: job.cache_ttl,
                            cache_ignore_s3_path: job.cache_ignore_s3_path,
                            dedicated_worker: None,
                            language: job.script_lang.unwrap_or(ScriptLang::Bun),
                            priority: job.priority,
                            apply_preprocessor: false,
                            concurrency_settings: ConcurrencySettings::default(),
                            debouncing_settings: DebouncingSettings::default(),
                        })
                    } else {
                        Err(error::Error::internal_err(
                            "WAC v2 Script job missing runnable_id".to_string(),
                        ))
                    }
                }
                JobKind::Preview => {
                    let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
                        "SELECT raw_code, raw_lock FROM v2_job WHERE id = $1 AND workspace_id = $2",
                    )
                    .bind(&job.id)
                    .bind(&job.workspace_id)
                    .fetch_optional(db)
                    .await?;
                    let (code, lock) = row.unwrap_or_default();
                    Ok(JobPayload::Code(RawCode {
                        content: code.unwrap_or_default(),
                        path: job.runnable_path.clone(),
                        hash: None,
                        language: job.script_lang.unwrap_or(ScriptLang::Bun),
                        lock: lock,
                        cache_ttl: job.cache_ttl,
                        cache_ignore_s3_path: job.cache_ignore_s3_path,
                        dedicated_worker: None,
                        concurrency_settings: ConcurrencySettingsWithCustom::default(),
                        debouncing_settings: DebouncingSettings::default(),
                        modules: None,
                    }))
                }
                _ => Err(error::Error::internal_err(format!(
                    "WAC v2 unsupported job kind: {:?}",
                    job.kind
                ))),
            }?;

            // Step 1: Save checkpoint, suspend parent, and seed child checkpoints
            // in a single transaction — all BEFORE children become visible.
            {
                let mut tx = db.begin().await?;

                // Update checkpoint with pending steps
                update_checkpoint_for_dispatch(&mut checkpoint, &steps, &mode, &job_ids);
                let status_json = serde_json::to_value(&checkpoint).map_err(|e| {
                    error::Error::internal_err(format!("Failed to serialize checkpoint: {e}"))
                })?;
                sqlx::query(
                    "INSERT INTO v2_job_status (id, workflow_as_code_status)
                     VALUES ($1, jsonb_build_object('_checkpoint', $2::jsonb))
                     ON CONFLICT (id) DO UPDATE SET
                        workflow_as_code_status = jsonb_set(
                            COALESCE(v2_job_status.workflow_as_code_status, '{}'::jsonb),
                            '{_checkpoint}',
                            $2::jsonb
                        )",
                )
                .bind(&job.id)
                .bind(&status_json)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error::Error::internal_err(format!("Failed to save WAC checkpoint: {e}"))
                })?;

                // Store per-child-job info for the WorkflowTimeline UI
                for (step, (_, child_id)) in steps.iter().zip(job_ids.iter()) {
                    let child_id_str = child_id.to_string();
                    let timeline_val = serde_json::json!({
                        "scheduled_for": chrono::Utc::now().to_rfc3339(),
                        "name": step.key,
                    });
                    sqlx::query(
                        "UPDATE v2_job_status SET workflow_as_code_status = jsonb_set(
                            COALESCE(workflow_as_code_status, '{}'::jsonb),
                            ARRAY[$2],
                            $3
                        ) WHERE id = $1",
                    )
                    .bind(&job.id)
                    .bind(&child_id_str)
                    .bind(&timeline_val)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        error::Error::internal_err(format!(
                            "Failed to update WAC timeline status: {e}"
                        ))
                    })?;
                }

                // Suspend parent before children become visible.
                // Keep running = true so the normal pull query ignores it.
                // The suspended pull query picks it up when suspend reaches 0
                // (it checks: suspend_until IS NOT NULL AND suspend <= 0).
                let suspend_count = num_steps as i32;
                sqlx::query!(
                    "UPDATE v2_job_queue SET suspend = $2, suspend_until = now() + interval '14 day' WHERE id = $1",
                    job.id,
                    suspend_count,
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error::Error::internal_err(format!(
                        "Failed to suspend WAC parent job {}: {e}",
                        job.id
                    ))
                })?;

                tx.commit().await?;
            }

            // Step 2: Push child jobs (now visible to workers).
            // Parent is already suspended, so child completions are safe.
            // Track successfully pushed children so we can cancel them on
            // partial failure (e.g. pushing child 3 of 5 fails).
            let mut pushed_ids: Vec<Uuid> = Vec::with_capacity(num_steps);
            let push_result: error::Result<()> = async {
                for (step, (_, child_uuid)) in steps.iter().zip(job_ids.iter()) {
                    // Resolve job payload based on dispatch_type
                    let (job_payload, child_args, is_external) = match step.dispatch_type.as_str() {
                        "script" if step.script.starts_with("./") => {
                            // Module-relative path: resolve from parent script's modules
                            let module_key = step.script.strip_prefix("./").unwrap();
                            let module = resolve_parent_module(modules, module_key)?;
                            let payload = JobPayload::Code(RawCode {
                                content: module.content,
                                path: job.runnable_path.clone(),
                                hash: None,
                                language: module.language,
                                lock: module.lock,
                                cache_ttl: job.cache_ttl,
                                cache_ignore_s3_path: job.cache_ignore_s3_path,
                                dedicated_worker: None,
                                concurrency_settings: ConcurrencySettingsWithCustom::default(),
                                debouncing_settings: DebouncingSettings::default(),
                                modules: None,
                            });
                            let step_args: HashMap<String, Box<RawValue>> = step
                                .args
                                .iter()
                                .map(|(k, v)| {
                                    let raw = serde_json::value::to_raw_value(v).unwrap();
                                    (k.clone(), raw)
                                })
                                .collect();
                            (payload, step_args, true)
                        }
                        "script" => {
                            // Resolve script path to job payload (handles hash, lang, etc.)
                            let (payload, _, _, _, _) = script_path_to_payload(
                                &step.script,
                                None, // no authed db for background workers
                                db.clone(),
                                &job.workspace_id,
                                Some(true), // skip preprocessor
                            )
                            .await?;
                            let step_args: HashMap<String, Box<RawValue>> = step
                                .args
                                .iter()
                                .map(|(k, v)| {
                                    let raw = serde_json::value::to_raw_value(v).unwrap();
                                    (k.clone(), raw)
                                })
                                .collect();
                            (payload, step_args, true)
                        }
                        "flow" => {
                            let flow_info = get_latest_flow_version_info_for_path(
                                None,
                                db,
                                &job.workspace_id,
                                &step.script,
                                true,
                            )
                            .await?;
                            let payload = JobPayload::Flow {
                                path: step.script.clone(),
                                dedicated_worker: flow_info.dedicated_worker,
                                apply_preprocessor: false,
                                version: flow_info.version,
                            };
                            let step_args: HashMap<String, Box<RawValue>> = step
                                .args
                                .iter()
                                .map(|(k, v)| {
                                    let raw = serde_json::value::to_raw_value(v).unwrap();
                                    (k.clone(), raw)
                                })
                                .collect();
                            (payload, step_args, true)
                        }
                        _ => {
                            // "inline" — re-run parent with _executing_key
                            (job_payload_template.clone(), parent_args.clone(), false)
                        }
                    };

                    let push_args = PushArgs { args: &child_args, extra: None };

                    // Apply step-level overrides to payload (cache, concurrency)
                    let mut job_payload = job_payload;
                    if let Some(cache_ttl) = step.cache_ttl {
                        match &mut job_payload {
                            JobPayload::ScriptHash { cache_ttl: ref mut ct, .. } => {
                                *ct = Some(cache_ttl)
                            }
                            JobPayload::Code(ref mut code) => code.cache_ttl = Some(cache_ttl),
                            _ => {}
                        }
                    }
                    if step.concurrent_limit.is_some()
                        || step.concurrency_key.is_some()
                        || step.concurrency_time_window_s.is_some()
                    {
                        match &mut job_payload {
                            JobPayload::ScriptHash { concurrency_settings: ref mut cs, .. } => {
                                if let Some(limit) = step.concurrent_limit {
                                    cs.concurrent_limit = Some(limit);
                                }
                                if let Some(ref key) = step.concurrency_key {
                                    cs.concurrency_key = Some(key.clone());
                                }
                                if let Some(window) = step.concurrency_time_window_s {
                                    cs.concurrency_time_window_s = Some(window);
                                }
                            }
                            JobPayload::Code(ref mut code) => {
                                if let Some(limit) = step.concurrent_limit {
                                    code.concurrency_settings.concurrent_limit = Some(limit);
                                }
                                if let Some(ref key) = step.concurrency_key {
                                    code.concurrency_settings.custom_concurrency_key =
                                        Some(key.clone());
                                }
                                if let Some(window) = step.concurrency_time_window_s {
                                    code.concurrency_settings.concurrency_time_window_s =
                                        Some(window);
                                }
                            }
                            _ => {}
                        }
                    }

                    let (_, mut tx) = push(
                        db,
                        PushIsolationLevel::IsolatedRoot(db.clone()),
                        &job.workspace_id,
                        job_payload,
                        push_args,
                        &job.created_by,
                        &job.permissioned_as_email,
                        job.permissioned_as.clone(),
                        None,
                        None,
                        None,
                        Some(job.id),                  // parent_job
                        job.root_job.or(Some(job.id)), // root_job
                        job.flow_innermost_root_job,
                        Some(*child_uuid), // pre-generated job_id
                        false,             // is_flow_step
                        false,             // same_worker
                        None,              // pre_run_error
                        job.visible_to_owner,
                        step.tag.clone().or_else(|| Some(job.tag.clone())),
                        step.timeout.or(job.timeout),
                        None,          // flow_step_id
                        step.priority, // priority_override
                        None,          // authed
                        false,         // running
                        None,          // end_user_email
                        None,          // trigger
                        None,          // suspended_mode
                    )
                    .await?;

                    // Seed child checkpoint only for inline tasks (they need
                    // _executing_key to know which step to run). External
                    // scripts/flows don't need a WAC checkpoint.
                    if !is_external {
                        let child_checkpoint_json = serde_json::json!({
                            "completed_steps": &checkpoint.completed_steps,
                            "_executing_key": &step.key,
                        });
                        sqlx::query(
                            "INSERT INTO v2_job_status (id, workflow_as_code_status)
                             VALUES ($1, jsonb_build_object('_checkpoint', $2::jsonb))
                             ON CONFLICT (id) DO UPDATE SET
                                workflow_as_code_status = jsonb_set(
                                    COALESCE(v2_job_status.workflow_as_code_status, '{}'::jsonb),
                                    '{_checkpoint}',
                                    $2::jsonb
                                )",
                        )
                        .bind(child_uuid)
                        .bind(&child_checkpoint_json)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| {
                            error::Error::internal_err(format!(
                                "Failed to seed child checkpoint: {e}"
                            ))
                        })?;
                    }

                    tx.commit().await.map_err(|e| {
                        error::Error::internal_err(format!("Failed to commit child push: {e}"))
                    })?;

                    pushed_ids.push(*child_uuid);

                    tracing::info!(
                        parent_job = %job.id,
                        child_job = %child_uuid,
                        step_name = %step.name,
                        step_key = %step.key,
                        "WAC v2 dispatched child job"
                    );
                }
                Ok(())
            }
            .await;

            if let Err(e) = push_result {
                tracing::error!(
                    job_id = %job.id,
                    error = %e,
                    pushed_count = pushed_ids.len(),
                    total_count = num_steps,
                    "WAC v2 failed to push child jobs, cleaning up"
                );

                // Cancel already-pushed children so they don't complete and
                // corrupt the checkpoint (they'd decrement suspend on a parent
                // that's about to be unsuspended and re-run).
                for child_id in &pushed_ids {
                    let _ = sqlx::query!(
                        "UPDATE v2_job_queue SET canceled_by = $2, canceled_reason = $3 WHERE id = $1",
                        child_id,
                        "system",
                        "WAC dispatch failed: not all children could be pushed",
                    )
                    .execute(db)
                    .await;
                }

                // Clear pending_steps from checkpoint so the parent doesn't
                // think children are outstanding when it re-runs.
                let _ = sqlx::query(
                    "UPDATE v2_job_status SET workflow_as_code_status = \
                     workflow_as_code_status #- '{_checkpoint,pending_steps}' \
                     WHERE id = $1",
                )
                .bind(&job.id)
                .execute(db)
                .await;

                // Unsuspend parent so the error propagates instead of a 14-day hang
                let _ = sqlx::query!(
                    "UPDATE v2_job_queue SET suspend = 0, suspend_until = NULL WHERE id = $1",
                    job.id,
                )
                .execute(db)
                .await;
                return Err(e);
            }

            tracing::info!(
                job_id = %job.id,
                num_steps = num_steps,
                "WAC v2 parent job suspended"
            );

            Err(error::Error::WacSuspended(format!(
                "WAC v2 job {} suspended waiting for {} child job(s)",
                job.id, num_steps
            )))
        }
        WacOutput::Approval { key, timeout, form, self_approval_disabled } => {
            let db = match conn {
                Connection::Sql(db) => db,
                _ => {
                    return Err(error::Error::internal_err(
                        "WAC v2 approval requires SQL connection".to_string(),
                    ))
                }
            };

            let mut checkpoint = load_checkpoint(db, &job.id).await?;
            let timeout_secs = timeout.unwrap_or(1800) as f64;

            // Mark this step as pending approval
            checkpoint.pending_steps = Some(crate::wac_executor::WacPendingSteps {
                mode: "approval".to_string(),
                keys: vec![key.clone()],
                job_ids: serde_json::Map::new(),
            });

            let mut tx = db.begin().await?;

            // Save checkpoint
            let status_json = serde_json::to_value(&checkpoint).map_err(|e| {
                error::Error::internal_err(format!("Failed to serialize checkpoint: {e}"))
            })?;
            sqlx::query(
                "INSERT INTO v2_job_status (id, workflow_as_code_status)
                 VALUES ($1, jsonb_build_object('_checkpoint', $2::jsonb))
                 ON CONFLICT (id) DO UPDATE SET
                    workflow_as_code_status = jsonb_set(
                        COALESCE(v2_job_status.workflow_as_code_status, '{}'::jsonb),
                        '{_checkpoint}',
                        $2::jsonb
                    )",
            )
            .bind(&job.id)
            .bind(&status_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| error::Error::internal_err(format!("Failed to save checkpoint: {e}")))?;

            // Store approval_conditions in flow_status for resume endpoint auth checks
            let sad = self_approval_disabled.unwrap_or(false);
            if sad {
                #[cfg(not(feature = "enterprise"))]
                return Err(error::Error::ExecutionErr(
                    "Disabling self-approval is an enterprise only feature".to_string(),
                ));

                #[cfg(feature = "enterprise")]
                {
                    use windmill_common::flow_status::ApprovalConditions;
                    let approval_conditions = ApprovalConditions {
                        user_auth_required: true,
                        user_groups_required: vec![],
                        self_approval_disabled: true,
                    };
                    sqlx::query(
                        "UPDATE v2_job_status SET flow_status = JSONB_SET(
                        COALESCE(flow_status, '{}'::jsonb),
                        '{approval_conditions}',
                        $2::jsonb
                    ) WHERE id = $1",
                    )
                    .bind(&job.id)
                    .bind(&serde_json::json!(approval_conditions))
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        error::Error::internal_err(format!(
                            "Failed to save approval conditions: {e}"
                        ))
                    })?;
                }
            }

            // Generate resume URLs for the inline approval buttons.
            // Use a hash of the step key as resume_id so each waitForApproval()
            // in the same workflow gets a unique resume_job record.
            let resume_id: u32 = {
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                key.hash(&mut hasher);
                (hasher.finish() & 0xFFFF_FFFF) as u32
            };
            // Generate stateless approval token using shared utility
            let approval_token =
                windmill_common::variables::generate_approval_token(&job.workspace_id, job.id, db)
                    .await?;

            let (resume_url, cancel_url, approval_page_url) = {
                use hmac::{Hmac, Mac};
                use sha2::Sha256;
                use windmill_common::variables::get_workspace_key;

                let wkey = get_workspace_key(&job.workspace_id, db).await?;
                let mut mac = Hmac::<Sha256>::new_from_slice(wkey.as_bytes())
                    .map_err(|e| error::Error::internal_err(format!("HMAC key error: {e}")))?;
                mac.update(job.id.as_bytes());
                mac.update(resume_id.to_be_bytes().as_ref());
                let signature = hex::encode(mac.finalize().into_bytes());

                let base_url = windmill_common::BASE_URL.read().await.clone();
                let w_id = &job.workspace_id;
                let job_id = &job.id;

                let resume = format!(
                    "{base_url}/api/w/{w_id}/jobs_u/resume/{job_id}/{resume_id}/{signature}"
                );
                let cancel = format!(
                    "{base_url}/api/w/{w_id}/jobs_u/cancel/{job_id}/{resume_id}/{signature}"
                );
                let approval_page =
                    format!("{base_url}/approve/{w_id}/{job_id}?token={approval_token}");
                (resume, cancel, approval_page)
            };

            // Store approval form metadata for the approval page endpoint
            let approval_meta = serde_json::json!({
                "key": key,
                "form": form,
                "timeout": timeout_secs as u32,
                "self_approval_disabled": sad,
                "resume": resume_url,
                "cancel": cancel_url,
                "approvalPage": approval_page_url,
            });
            sqlx::query(
                "UPDATE v2_job_status SET workflow_as_code_status = jsonb_set(
                    COALESCE(workflow_as_code_status, '{}'::jsonb),
                    '{_approval}',
                    $2::jsonb
                ) WHERE id = $1",
            )
            .bind(&job.id)
            .bind(&approval_meta)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error::Error::internal_err(format!("Failed to save approval meta: {e}"))
            })?;

            // Write timeline entry for the approval step
            {
                let now_str = chrono::Utc::now().to_rfc3339();
                let timeline_val = serde_json::json!({
                    "scheduled_for": &now_str,
                    "started_at": &now_str,
                    "name": key,
                    "approval": true,
                    "self_approval_disabled": sad,
                    "form": form,
                    "resume": &resume_url,
                    "cancel": &cancel_url,
                    "approvalPage": &approval_page_url,
                });
                let step_timeline_key = format!("_step/{}", key);
                sqlx::query(
                    "UPDATE v2_job_status SET workflow_as_code_status = jsonb_set(
                        COALESCE(workflow_as_code_status, '{}'::jsonb),
                        ARRAY[$2],
                        $3
                    ) WHERE id = $1",
                )
                .bind(&job.id)
                .bind(&step_timeline_key)
                .bind(&timeline_val)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error::Error::internal_err(format!("Failed to write approval timeline: {e}"))
                })?;
            }

            // Suspend parent with suspend=1 (waiting for 1 approval event)
            sqlx::query!(
                "UPDATE v2_job_queue SET suspend = 1, suspend_until = now() + make_interval(secs => $2) WHERE id = $1",
                job.id,
                timeout_secs,
            )
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;

            tracing::info!(
                job_id = %job.id,
                approval_key = %key,
                timeout_secs = timeout_secs,
                "WAC v2 parent job suspended waiting for approval"
            );

            Err(error::Error::WacSuspended(format!(
                "WAC v2 job {} suspended waiting for approval (key: {})",
                job.id, key
            )))
        }
        WacOutput::Sleep { key, seconds } => {
            let db = match conn {
                Connection::Sql(db) => db,
                _ => {
                    return Err(error::Error::internal_err(
                        "WAC v2 sleep requires SQL connection".to_string(),
                    ))
                }
            };

            let mut checkpoint = load_checkpoint(db, &job.id).await?;
            let sleep_secs = seconds.max(1) as f64;

            // Mark this step as pending sleep
            checkpoint.pending_steps = Some(crate::wac_executor::WacPendingSteps {
                mode: "sleep".to_string(),
                keys: vec![key.clone()],
                job_ids: serde_json::Map::new(),
            });

            let mut tx = db.begin().await?;

            // Save checkpoint
            let status_json = serde_json::to_value(&checkpoint).map_err(|e| {
                error::Error::internal_err(format!("Failed to serialize checkpoint: {e}"))
            })?;
            sqlx::query(
                "INSERT INTO v2_job_status (id, workflow_as_code_status)
                 VALUES ($1, jsonb_build_object('_checkpoint', $2::jsonb))
                 ON CONFLICT (id) DO UPDATE SET
                    workflow_as_code_status = jsonb_set(
                        COALESCE(v2_job_status.workflow_as_code_status, '{}'::jsonb),
                        '{_checkpoint}',
                        $2::jsonb
                    )",
            )
            .bind(&job.id)
            .bind(&status_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| error::Error::internal_err(format!("Failed to save checkpoint: {e}")))?;

            // Write a "sleep" marker in the timeline.  Unlike real steps it
            // carries no execution bar — the frontend renders it as a
            // minimal label row (e.g. "sleep (2s)").
            {
                let now_str = chrono::Utc::now().to_rfc3339();
                let timeline_val = serde_json::json!({
                    "scheduled_for": &now_str,
                    "name": key,
                    "sleep_duration_s": seconds,
                });
                let step_timeline_key = format!("_step/{}", key);
                sqlx::query(
                    "UPDATE v2_job_status SET workflow_as_code_status = jsonb_set(
                        COALESCE(workflow_as_code_status, '{}'::jsonb),
                        ARRAY[$2],
                        $3
                    ) WHERE id = $1",
                )
                .bind(&job.id)
                .bind(&step_timeline_key)
                .bind(&timeline_val)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error::Error::internal_err(format!("Failed to write sleep timeline: {e}"))
                })?;
            }

            // Suspend parent — it will auto-resume when suspend_until passes.
            // Use suspend=1 (not 0) so the suspended pull query only picks it up
            // when `suspend_until <= now()`, not via `suspend <= 0`.
            sqlx::query!(
                "UPDATE v2_job_queue SET suspend = 1, suspend_until = now() + make_interval(secs => $2) WHERE id = $1",
                job.id,
                sleep_secs,
            )
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;

            tracing::info!(
                job_id = %job.id,
                sleep_key = %key,
                sleep_secs = sleep_secs,
                "WAC v2 parent job sleeping for {}s",
                sleep_secs
            );

            Err(error::Error::WacSuspended(format!(
                "WAC v2 job {} sleeping for {}s (key: {})",
                job.id, seconds, key
            )))
        }
        WacOutput::InlineCheckpoint { key, result: value, started_at, duration_ms } => {
            let db = match conn {
                Connection::Sql(db) => db,
                _ => {
                    return Err(error::Error::internal_err(
                        "WAC v2 inline checkpoint requires SQL connection".to_string(),
                    ))
                }
            };

            let mut checkpoint = load_checkpoint(db, &job.id).await?;

            // Source hash validation (same as Dispatch path)
            let current_hash = job.runnable_id.map(|h| h.0.to_string()).unwrap_or_default();
            if !current_hash.is_empty() {
                if checkpoint.source_hash.is_empty() {
                    checkpoint.source_hash = current_hash.clone();
                } else if checkpoint.source_hash != current_hash {
                    return Err(error::Error::ExecutionErr(
                        "Workflow source code changed between replays. \
                         Cannot safely resume from checkpoint — step keys may have shifted. \
                         Please restart this workflow."
                            .to_string(),
                    ));
                }
            }

            tracing::info!(
                job_id = %job.id,
                step_key = %key,
                "WAC v2 inline checkpoint — persisting step result"
            );

            add_completed_step(&mut checkpoint, &key, value);

            // Save checkpoint + write step timeline entry + reset running in a single transaction
            {
                let mut tx = db.begin().await?;
                let status_json = serde_json::to_value(&checkpoint).map_err(|e| {
                    error::Error::internal_err(format!("Failed to serialize checkpoint: {e}"))
                })?;
                sqlx::query(
                    "INSERT INTO v2_job_status (id, workflow_as_code_status)
                     VALUES ($1, jsonb_build_object('_checkpoint', $2::jsonb))
                     ON CONFLICT (id) DO UPDATE SET
                        workflow_as_code_status = jsonb_set(
                            COALESCE(v2_job_status.workflow_as_code_status, '{}'::jsonb),
                            '{_checkpoint}',
                            $2::jsonb
                        )",
                )
                .bind(&job.id)
                .bind(&status_json)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error::Error::internal_err(format!("Failed to save WAC checkpoint: {e}"))
                })?;

                // Write timeline entry for the inline step (keyed as _step/<key>).
                // Fall back to now() when the client doesn't provide started_at
                // (older windmill-client versions omit it).
                {
                    let now_str = chrono::Utc::now().to_rfc3339();
                    let sa = started_at.as_deref().unwrap_or(&now_str);
                    let mut timeline_val = serde_json::json!({
                        "scheduled_for": sa,
                        "started_at": sa,
                        "name": key,
                    });
                    if let Some(dur) = duration_ms {
                        timeline_val["duration_ms"] = serde_json::json!(dur);
                    }
                    let step_timeline_key = format!("_step/{}", key);
                    sqlx::query(
                        "UPDATE v2_job_status SET workflow_as_code_status = jsonb_set(
                            COALESCE(workflow_as_code_status, '{}'::jsonb),
                            ARRAY[$2],
                            $3
                        ) WHERE id = $1",
                    )
                    .bind(&job.id)
                    .bind(&step_timeline_key)
                    .bind(&timeline_val)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        error::Error::internal_err(format!("Failed to write step timeline: {e}"))
                    })?;
                }

                // Reset running=false so the job is immediately eligible for pickup.
                // Unlike dispatch (which sets suspend>0), inline checkpoints don't suspend —
                // the job should be re-run right away to continue past the cached step.
                sqlx::query!(
                    "UPDATE v2_job_queue SET running = false, started_at = null WHERE id = $1",
                    job.id,
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error::Error::internal_err(format!(
                        "Failed to reset running state for inline checkpoint: {e}"
                    ))
                })?;

                tx.commit().await?;
            }

            Err(error::Error::WacSuspended(format!(
                "WAC v2 job {} inline checkpoint for step {}",
                job.id, key
            )))
        }
    }
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

#[cfg(any(feature = "deno_core", feature = "private"))]
pub fn build_nativets_env_code(
    base_internal_url: &str,
    reserved_variables: &HashMap<String, String>,
) -> String {
    format!(
        "const process = {{ env: {{}} }};\nconst BASE_URL = '{base_internal_url}';\nconst BASE_INTERNAL_URL = '{base_internal_url}';\nprocess.env['BASE_URL'] = BASE_URL;process.env['BASE_INTERNAL_URL'] = BASE_INTERNAL_URL;\n{}",
        reserved_variables
            .iter()
            .map(|(k, v)| {
                let escaped = v.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n").replace('\r', "\\r");
                format!("process.env['{}'] = '{}';", k, escaped)
            })
            .collect::<Vec<String>>()
            .join("\n")
    )
}

#[cfg(feature = "private")]
use crate::{
    common::build_envs_map, dedicated_worker_oss::handle_dedicated_process, JobCompletedSender,
};
#[cfg(feature = "private")]
use tokio::sync::mpsc::Receiver;
#[cfg(feature = "private")]
use windmill_common::variables;
#[cfg(feature = "private")]
use windmill_queue::DedicatedWorkerJob;

#[cfg(feature = "private")]
async fn handle_dedicated_bunnative(
    inner_content: &str,
    js_code: &str,
    env_code: &str,
    token: &str,
    worker_name: &str,
    _w_id: &str,
    script_path: &str,
    db: &DB,
    jobs_rx: Receiver<DedicatedWorkerJob>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
    job_completed_tx: JobCompletedSender,
    client: &windmill_common::client::AuthedClient,
) -> Result<()> {
    #[cfg(not(feature = "deno_core"))]
    {
        let _ = (
            inner_content,
            js_code,
            env_code,
            token,
            worker_name,
            script_path,
            db,
            jobs_rx,
            killpill_rx,
            job_completed_tx,
            client,
        );
        return Err(error::Error::internal_err(
            "deno_core feature is not activated but native dedicated worker was started"
                .to_string(),
        ));
    }

    #[cfg(feature = "deno_core")]
    {
        use std::sync::Arc;

        use crate::common::transform_json;
        use windmill_common::worker::to_raw_value;
        use windmill_queue::{append_logs, JobCompleted, MiniCompletedJob};
        use windmill_runtime_nativets::PrewarmedIsolate;

        let ann = windmill_runtime_nativets::get_annotation(inner_content);
        let parsed_args =
            windmill_parser_ts::parse_deno_signature(inner_content, true, false, None)?.args;
        let arg_names: Vec<String> = parsed_args.into_iter().map(|x| x.name).collect();

        let env_code = env_code.to_string();
        let js_code = js_code.to_string();

        let pre_arg_names: Option<Vec<String>> = windmill_parser_ts::parse_deno_signature(
            inner_content,
            true,
            false,
            Some("preprocessor".to_string()),
        )
        .ok()
        .filter(|sig| !sig.args.is_empty())
        .map(|sig| sig.args.into_iter().map(|x| x.name).collect());

        let mut warm = PrewarmedIsolate::spawn(
            env_code.clone(),
            js_code.clone(),
            ann.clone(),
            arg_names.clone(),
            None,
        );

        // Pre-warm preprocessor isolate if the script has a preprocessor
        let mut pre_warm = pre_arg_names.as_ref().map(|pre_names| {
            PrewarmedIsolate::spawn(
                env_code.clone(),
                js_code.clone(),
                ann.clone(),
                pre_names.clone(),
                Some("preprocessor".to_string()),
            )
        });

        let init_log = format!("dedicated worker nativets: {worker_name}\n\n");
        let alive = true;
        let mut killpill_rx = killpill_rx;
        let mut jobs_rx = jobs_rx;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv(), if alive => {
                    tracing::info!("received killpill for nativets dedicated worker");
                    break;
                },
                job = jobs_rx.recv(), if alive => {
                    if let Some(DedicatedWorkerJob { job, flow_runners, done_tx }) = job {
                        let id = job.id;
                        tracing::info!(
                            "received job on nativets dedicated worker for {script_path}: {id}"
                        );

                        let args = if let Some(args) = job.args.as_ref() {
                            if let Some(x) = transform_json(
                                client, &job.workspace_id, &args.0, &job, &db.into(),
                            ).await? {
                                serde_json::to_string(&x)
                                    .unwrap_or_else(|_| "{}".to_string())
                            } else {
                                serde_json::to_string(&args)
                                    .unwrap_or_else(|_| "{}".to_string())
                            }
                        } else {
                            "{}".to_string()
                        };

                        // Run the job: preprocess if needed, then execute main.
                        // Uses a labeled block to unify error handling with a single JobCompleted send.
                        let (result, success, preprocessed_args, logs) = 'job: {
                            if let Err(e) = warm.wait_ready().await {
                                break 'job (
                                    Arc::new(to_raw_value(&serde_json::json!({"message": format!("isolate init failed: {e}"), "name": "Error"}))),
                                    false, None, init_log.clone(),
                                );
                            }

                            let needs_preprocessing = job.preprocessed == Some(false);

                            let (main_args, preprocessed) = if needs_preprocessing {
                                let Some(ref pre_names) = pre_arg_names else {
                                    break 'job (
                                        Arc::new(to_raw_value(&serde_json::json!({"message": "preprocessor function is missing", "name": "Error"}))),
                                        false, None, init_log.clone(),
                                    );
                                };

                                let mut pre_isolate = pre_warm.take().unwrap_or_else(|| {
                                    PrewarmedIsolate::spawn(
                                        env_code.clone(),
                                        js_code.clone(),
                                        ann.clone(),
                                        pre_names.clone(),
                                        Some("preprocessor".to_string()),
                                    )
                                });
                                if let Err(e) = pre_isolate.wait_ready().await {
                                    break 'job (
                                        Arc::new(to_raw_value(&serde_json::json!({"message": format!("preprocessor isolate init failed: {e}"), "name": "Error"}))),
                                        false, None, init_log.clone(),
                                    );
                                }

                                let pre_executing = pre_isolate.start_execution(args.clone());
                                // Pipeline: start pre-warming the next preprocessor isolate
                                pre_warm = Some(PrewarmedIsolate::spawn(
                                    env_code.clone(),
                                    js_code.clone(),
                                    ann.clone(),
                                    pre_names.clone(),
                                    Some("preprocessor".to_string()),
                                ));

                                let pre_result = match pre_executing.wait().await {
                                    Ok(r) => r,
                                    Err(e) => break 'job (
                                        Arc::new(to_raw_value(&serde_json::json!({"message": format!("preprocessor failed: {e}"), "name": "Error"}))),
                                        false, None, init_log.clone(),
                                    ),
                                };
                                if !pre_result.logs.is_empty() {
                                    append_logs(&id, &job.workspace_id, pre_result.logs, &db.into()).await;
                                }
                                let raw = match pre_result.result {
                                    Ok(r) => r,
                                    Err(e) => break 'job (
                                        Arc::new(to_raw_value(&serde_json::json!({"message": format!("preprocessor failed: {e}"), "name": "Error"}))),
                                        false, None, init_log.clone(),
                                    ),
                                };

                                let preprocessed: HashMap<String, Box<RawValue>> = match serde_json::from_str(raw.get()) {
                                    Ok(v) => v,
                                    Err(e) => break 'job (
                                        Arc::new(to_raw_value(&serde_json::json!({"message": format!("error deserializing preprocessed args: {e:#}"), "name": "Error"}))),
                                        false, None, init_log.clone(),
                                    ),
                                };
                                let main_args = serde_json::to_string(&preprocessed)
                                    .unwrap_or_else(|_| "{}".to_string());
                                (main_args, Some(preprocessed))
                            } else {
                                (args, None)
                            };

                            let executing = warm.start_execution(main_args);
                            // Pipeline: start pre-warming the next main isolate
                            warm = PrewarmedIsolate::spawn(
                                env_code.clone(),
                                js_code.clone(),
                                ann.clone(),
                                arg_names.clone(),
                                None,
                            );

                            let main_result = match executing.wait().await {
                                Ok(r) => r,
                                Err(e) => break 'job (
                                    Arc::new(to_raw_value(&serde_json::json!({"message": format!("{e}"), "name": "Error"}))),
                                    false, preprocessed, init_log.clone(),
                                ),
                            };

                            let mut logs = init_log.clone();
                            if !main_result.logs.is_empty() {
                                logs.push_str(&main_result.logs);
                            }

                            let (result, success) = match main_result.result {
                                Ok(raw) => (Arc::new(raw), true),
                                Err(e) => (
                                    Arc::new(to_raw_value(&serde_json::json!({
                                        "message": e,
                                        "name": "Error",
                                    }))),
                                    false,
                                ),
                            };

                            (result, success, preprocessed, logs)
                        };

                        append_logs(&id, &job.workspace_id, logs, &db.into()).await;
                        job_completed_tx.send_job(JobCompleted {
                            job: MiniCompletedJob::from(job),
                            result,
                            result_columns: None,
                            mem_peak: 0,
                            canceled_by: None,
                            success,
                            cached_res_path: None,
                            token: token.to_string(),
                            duration: None,
                            preprocessed_args,
                            has_stream: Some(false),
                            from_cache: None,
                            flow_runners,
                            done_tx,
                        }, true).await?;
                    } else {
                        tracing::debug!("job channel closed for nativets dedicated worker");
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "private")]
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
    jobs_rx: Receiver<DedicatedWorkerJob>,
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
    client: windmill_common::client::AuthedClient,
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
    if !annotation.native {
        annotation.nodejs = true;
    }

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
        None,
        None,
        None,
    )
    .await;
    let context_envs = build_envs_map(context.to_vec()).await;

    if annotation.native {
        // Native (V8) dedicated worker: bundle the code and dispatch to V8 instead of a subprocess.
        let main_code = remove_pinned_imports(inner_content)?;
        write_file(job_dir, "main.ts", &main_code)?;

        if let Some(reqs) = requirements_o.as_ref() {
            let (pkg, lock, empty, is_binary) = split_lockfile(reqs);
            write_file(job_dir, "package.json", pkg)?;
            if let Some(lock) = lock {
                if !empty {
                    write_lock(lock, job_dir, is_binary).await?;
                    install_bun_lockfile(
                        &mut mem_peak,
                        &mut canceled_by,
                        &Uuid::nil(),
                        w_id,
                        Some(&Connection::from(db.clone())),
                        job_dir,
                        worker_name,
                        common_bun_proc_envs.clone(),
                        annotation.npm,
                        &mut None,
                        false,
                    )
                    .await?;
                }
            }
        }

        build_loader(
            job_dir,
            base_internal_url,
            token,
            w_id,
            script_path,
            LoaderMode::BrowserBundle,
            &None,
        )
        .await?;
        generate_bun_bundle(
            job_dir,
            w_id,
            &Uuid::nil(),
            worker_name,
            Some(&Connection::from(db.clone())),
            None,
            &mut mem_peak,
            &mut canceled_by,
            &common_bun_proc_envs,
            &mut None,
        )
        .await?;
        let js_code = read_file_content(&format!("{job_dir}/main.js")).await?;

        let reserved_variables: HashMap<String, String> = context
            .iter()
            .map(|x| (x.name.clone(), x.value.clone()))
            .collect();
        let env_code = build_nativets_env_code(base_internal_url, &reserved_variables);

        return handle_dedicated_bunnative(
            inner_content,
            &js_code,
            &env_code,
            token,
            worker_name,
            w_id,
            script_path,
            db,
            jobs_rx,
            killpill_rx,
            job_completed_tx,
            &client,
        )
        .await;
    }

    let mut format = BundleFormat::Cjs;
    if let Some(codebase) = codebase.as_ref() {
        let pulled_codebase = pull_codebase(w_id, codebase, job_dir).await?;
        if pulled_codebase.is_esm {
            format = BundleFormat::Esm;
        }
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
                false,
            )
            .await?;
            tracing::info!("dedicated worker requirements installed: {reqs}");
        }
    } else if is_sandboxing_enabled() {
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
            &WorkspaceDependenciesPrefetched::extract(
                inner_content,
                ScriptLang::Bun,
                w_id,
                &None,
                &script_path,
                db.into(),
            )
            .await?,
            annotation.npm,
            &mut None,
            &None,
            false,
        )
        .await?;
    }

    let main_code = remove_pinned_imports(inner_content)?;
    let _ = write_file(job_dir, "main.ts", &main_code)?;

    let codegen = compute_ts_codegen(inner_content);
    let wrapper_ext = if codebase.is_some() { "js" } else { "ts" };
    {
        let scripts =
            [
                TsScriptEntry {
                    import_name: "main",
                    original_path: script_path,
                    codegen: &codegen,
                },
            ];
        let wrapper_content = generate_multi_script_wrapper(&scripts, wrapper_ext);
        write_file(job_dir, "wrapper.mjs", &wrapper_content)?;
    }

    if format == BundleFormat::Esm {
        annotation.nodejs = false;
    }

    if !codebase.is_some() || format == BundleFormat::Esm {
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
            &None,
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
            client,
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
            client,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_lockfile_text_unix() {
        let lockfile = r#"{"dependencies":{"lodash":"^4.17.21"}}
//bun.lock
lockfile-content-here"#;

        let (pkg, lock, is_empty, is_binary) = split_lockfile(lockfile);

        assert_eq!(pkg, r#"{"dependencies":{"lodash":"^4.17.21"}}"#);
        assert_eq!(lock, Some("lockfile-content-here"));
        assert!(!is_empty);
        assert!(!is_binary);
    }

    #[test]
    fn test_split_lockfile_text_windows() {
        let lockfile = "{\"dependencies\":{}}\r\n//bun.lock\r\nlockfile-content";

        let (pkg, lock, is_empty, is_binary) = split_lockfile(lockfile);

        assert_eq!(pkg, "{\"dependencies\":{}}");
        assert_eq!(lock, Some("lockfile-content"));
        assert!(!is_empty);
        assert!(!is_binary);
    }

    #[test]
    fn test_split_lockfile_binary_unix() {
        let lockfile = r#"{"dependencies":{}}
//bun.lockb
YmluYXJ5LWNvbnRlbnQ="#; // base64 encoded "binary-content"

        let (pkg, lock, is_empty, is_binary) = split_lockfile(lockfile);

        assert_eq!(pkg, r#"{"dependencies":{}}"#);
        assert_eq!(lock, Some("YmluYXJ5LWNvbnRlbnQ="));
        assert!(!is_empty);
        assert!(is_binary);
    }

    #[test]
    fn test_split_lockfile_binary_windows() {
        let lockfile = "{\"dependencies\":{}}\r\n//bun.lockb\r\nYmluYXJ5LWNvbnRlbnQ=";

        let (pkg, lock, is_empty, is_binary) = split_lockfile(lockfile);

        assert_eq!(pkg, "{\"dependencies\":{}}");
        assert_eq!(lock, Some("YmluYXJ5LWNvbnRlbnQ="));
        assert!(!is_empty);
        assert!(is_binary);
    }

    #[test]
    fn test_split_lockfile_empty() {
        let lockfile = r#"{"dependencies":{}}
//bun.lock
<empty>"#;

        let (pkg, lock, is_empty, is_binary) = split_lockfile(lockfile);

        assert_eq!(pkg, r#"{"dependencies":{}}"#);
        assert_eq!(lock, Some(EMPTY_FILE));
        assert!(is_empty);
        assert!(!is_binary);
    }

    #[test]
    fn test_split_lockfile_no_lock() {
        let lockfile = r#"{"dependencies":{"lodash":"^4.17.21"}}"#;

        let (pkg, lock, is_empty, is_binary) = split_lockfile(lockfile);

        assert_eq!(pkg, r#"{"dependencies":{"lodash":"^4.17.21"}}"#);
        assert!(lock.is_none());
        assert!(!is_empty);
        assert!(!is_binary);
    }

    #[test]
    fn test_split_lockfile_multiline_package_json() {
        let lockfile = r#"{
  "dependencies": {
    "lodash": "^4.17.21"
  }
}
//bun.lock
lockfile-content"#;

        let (pkg, lock, is_empty, is_binary) = split_lockfile(lockfile);

        assert_eq!(
            pkg,
            r#"{
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#
        );
        assert_eq!(lock, Some("lockfile-content"));
        assert!(!is_empty);
        assert!(!is_binary);
    }

    #[test]
    fn test_compute_ts_codegen_basic_args() {
        let code = r#"export function main(x: string, y: number) { return x; }"#;
        let cg = compute_ts_codegen(code);
        assert_eq!(cg.spread, "x, y");
        assert!(cg.date_conversions.is_empty());
        assert!(cg.preprocessor_spread.is_none());
    }

    #[test]
    fn test_compute_ts_codegen_with_datetime() {
        let code = r#"export function main(name: string, created_at: Date, count: number) { return name; }"#;
        let cg = compute_ts_codegen(code);
        assert_eq!(cg.spread, "name, created_at, count");
        assert!(cg.date_conversions.contains("created_at"));
        assert!(cg.date_conversions.contains("new Date"));
    }

    #[test]
    fn test_compute_ts_codegen_with_preprocessor() {
        let code = r#"
export function main(x: string, ts: Date) { return x; }
export function preprocessor(input: string, when: Date) { return { x: input, ts: when }; }
"#;
        let cg = compute_ts_codegen(code);
        assert_eq!(cg.spread, "x, ts");
        assert!(cg.date_conversions.contains("ts"));
        assert_eq!(cg.preprocessor_spread.as_deref(), Some("input, when"));
        assert!(cg
            .preprocessor_date_conversions
            .as_ref()
            .unwrap()
            .contains("when"));
    }

    #[test]
    fn test_compute_ts_codegen_no_args() {
        let code = r#"export function main() { return 42; }"#;
        let cg = compute_ts_codegen(code);
        assert!(cg.spread.is_empty());
        assert!(cg.date_conversions.is_empty());
        assert!(cg.preprocessor_spread.is_none());
    }
}
