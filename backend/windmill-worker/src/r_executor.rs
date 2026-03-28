use std::{collections::HashMap, process::Stdio};

use itertools::Itertools;
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use uuid::Uuid;
use windmill_common::{
    client::AuthedClient,
    error::Error,
    utils::calculate_hash,
    worker::{write_file, Connection, RlangAnnotations},
};
use windmill_parser::Arg;
use windmill_parser_r::{parse_r_requirements, parse_r_signature};
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::{
    common::{
        build_command_with_isolation, create_args_and_out_file, get_reserved_variables,
        read_result, start_child_process, OccupancyMetrics, DEV_CONF_NSJAIL,
    },
    get_proxy_envs_for_lang,
    handle_child::{self},
    is_sandboxing_enabled,
    universal_pkg_installer::{
        par_install_language_dependencies_seq, DependencyGraph, InstallDeps, RequiredDependency,
    },
    DISABLE_NUSER, NSJAIL_AVAILABLE, NSJAIL_PATH, PATH_ENV, PROXY_ENVS, R_CACHE_DIR,
    TRACING_PROXY_CA_CERT_PATH,
};
use windmill_common::scripts::ScriptLang;

lazy_static::lazy_static! {
    static ref RSCRIPT_PATH: String = std::env::var("RSCRIPT_PATH").unwrap_or_else(|_| "/usr/bin/Rscript".to_string());
    static ref R_CONCURRENT_DOWNLOADS: usize = std::env::var("R_CONCURRENT_DOWNLOADS").ok().map(|flag| flag.parse().unwrap_or(20)).unwrap_or(20);
    static ref R_PROXY_ENVS: Vec<(String, String)> = {
        PROXY_ENVS
            .clone()
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect()
    };
}

const NSJAIL_CONFIG_RUN_R_CONTENT: &str = include_str!("../nsjail/run.r.config.proto");
const NSJAIL_CONFIG_INSTALL_R_CONTENT: &str = include_str!("../nsjail/install.r.config.proto");

#[allow(dead_code)]
pub(crate) struct JobHandlerInput<'a> {
    pub base_internal_url: &'a str,
    pub canceled_by: &'a mut Option<CanceledBy>,
    pub client: &'a AuthedClient,
    pub parent_runnable_path: Option<String>,
    pub conn: &'a Connection,
    pub envs: HashMap<String, String>,
    pub inner_content: &'a str,
    pub job: &'a MiniPulledJob,
    pub job_dir: &'a str,
    pub mem_peak: &'a mut i32,
    pub occupancy_metrics: &'a mut OccupancyMetrics,
    pub requirements_o: Option<&'a String>,
    pub shared_mount: &'a str,
    pub worker_name: &'a str,
}

pub async fn handle_r_job<'a>(
    mut args: JobHandlerInput<'a>,
) -> Result<Box<sqlx::types::JsonRawValue>, Error> {
    let annotation = RlangAnnotations::parse(args.inner_content);

    if annotation.sandbox && NSJAIL_AVAILABLE.is_none() {
        return Err(Error::ExecutionErr(
            "Script has #sandbox annotation but nsjail is not available on this worker. \
            Please ensure nsjail is installed or remove the #sandbox annotation."
                .to_string(),
        ));
    }

    // --- Prepare ---
    {
        prepare(&args).await?;
    }
    // --- Resolve lockfile ---
    let lockfile = resolve(
        &args.job.id,
        args.inner_content,
        args.mem_peak,
        args.canceled_by,
        args.job_dir,
        args.conn,
        args.worker_name,
        &args.job.workspace_id,
        annotation.verbose,
    )
    .await?;
    // --- Install ---
    let lib_path = if !lockfile.is_empty() {
        Some(install(&mut args, &lockfile, annotation.verbose).await?)
    } else {
        None
    };
    // --- Execute ---
    {
        run(&mut args, lib_path.as_deref(), annotation.sandbox).await?;
    }
    // --- Retrieve results ---
    {
        read_result(&args.job_dir, None).await
    }
}

pub async fn prepare<'a>(
    JobHandlerInput { job, conn, job_dir, inner_content, client, .. }: &JobHandlerInput<'a>,
) -> Result<(), Error> {
    create_args_and_out_file(&client, job, job_dir, conn).await?;
    File::create(format!("{}/main.r", job_dir))
        .await?
        .write_all(&wrap(inner_content)?.into_bytes())
        .await?;

    // Create windmill client library for R
    let wm_lib_path = format!("{}/r_libs", *R_CACHE_DIR);
    fs::create_dir_all(&wm_lib_path).await?;
    {
        File::create(format!("{}/windmill.r", &wm_lib_path))
            .await?
            .write_all(
                r##"
# Windmill mini client methods for R
# Uses base R url() + readLines() to avoid requiring any extra R packages

.wm_fetch_raw <- function(url) {
    token <- Sys.getenv("WM_TOKEN")
    con <- url(url, headers = c(Authorization = paste("Bearer", token)))
    on.exit(close(con))
    paste(readLines(con, warn = FALSE), collapse = "\n")
}

get_variable <- function(path) {
    base_url <- Sys.getenv("BASE_INTERNAL_URL")
    workspace <- Sys.getenv("WM_WORKSPACE")
    url <- paste0(base_url, "/api/w/", workspace, "/variables/get_value/", path)
    jsonlite::fromJSON(.wm_fetch_raw(url))
}

get_resource <- function(path) {
    base_url <- Sys.getenv("BASE_INTERNAL_URL")
    workspace <- Sys.getenv("WM_WORKSPACE")
    url <- paste0(base_url, "/api/w/", workspace, "/resources/get_value_interpolated/", path)
    jsonlite::fromJSON(.wm_fetch_raw(url))
}
"##
                .as_bytes(),
            )
            .await?;
    }
    Ok(())
}

pub async fn resolve<'a>(
    job_id: &Uuid,
    inner_content: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    conn: &Connection,
    worker_name: &str,
    w_id: &str,
    verbose: bool,
) -> Result<String, Error> {
    let mut packages = parse_r_requirements(inner_content)?;

    // jsonlite is always needed by the wrapper for JSON arg parsing and result serialization
    let has_jsonlite = packages.lines().any(|l| l.trim() == "jsonlite");
    if !has_jsonlite {
        if packages.is_empty() {
            packages = "jsonlite".to_string();
        } else {
            packages.push_str("\njsonlite");
        }
    }

    // Check cache
    let req_hash = format!("r-{}", calculate_hash(&packages));
    if let Some(db) = conn.as_sql() {
        if let Some(cached) = sqlx::query_scalar!(
            "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
            req_hash
        )
        .fetch_optional(db)
        .await?
        {
            return Ok(cached);
        }
    }

    append_logs(
        job_id,
        w_id,
        format!("\n--- RESOLVING R PACKAGES ---\n"),
        conn,
    )
    .await;

    // main.r is already written by prepare() and contains the library() calls.
    // renv will scan it to detect dependencies.
    // Disable renv's own package cache — Windmill manages its own install cache.
    let resolve_script = format!(
        r#"options(
    repos = c(CRAN = "https://cloud.r-project.org"),
    renv.verbose = {verbose_r},
    renv.config.cache.enabled = FALSE
)
renv::consent(provided = TRUE)
renv::init(bare = TRUE, restart = FALSE)
renv::install(prompt = FALSE)
renv::snapshot(type = "implicit", prompt = FALSE)
"#,
        verbose_r = if verbose { "TRUE" } else { "FALSE" },
    );

    let mut file = File::create(format!("{}/resolve.r", job_dir)).await?;
    file.write_all(resolve_script.as_bytes()).await?;

    let child = {
        let renv_root = format!("{}/renv", *R_CACHE_DIR);
        let rscript_executable = if cfg!(windows) {
            "Rscript.exe"
        } else {
            RSCRIPT_PATH.as_str()
        };
        let mut cmd = Command::new(rscript_executable);
        cmd.current_dir(job_dir)
            .env("PATH", PATH_ENV.as_str())
            .env("RENV_PATHS_ROOT", &renv_root)
            .arg("resolve.r")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(cmd, rscript_executable, false).await?
    };
    handle_child::handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        child,
        false,
        worker_name,
        w_id,
        "r resolve",
        None,
        false,
        &mut None,
        None,
        None,
    )
    .await?;

    let lock_path = format!("{}/renv.lock", job_dir);
    let mut lock_file = File::open(&lock_path).await?;
    let mut lock = String::new();
    lock_file.read_to_string(&mut lock).await?;

    // Cache the lockfile
    if let Some(db) = conn.as_sql() {
        sqlx::query!(
            "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('3 days')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = EXCLUDED.lockfile",
            req_hash,
            lock.clone(),
        ).fetch_optional(db).await?;
    }

    append_logs(job_id, w_id, format!("{}", &lock), conn).await;
    Ok(lock)
}

struct RenvPackage {
    name: String,
    version: String,
    repo_url: String,
    /// Package names from Imports + Depends fields
    dependencies: Vec<String>,
}

/// Parse renv.lock JSON and extract package info including dependency edges.
fn parse_renv_lock(lockfile: &str) -> Result<Vec<RenvPackage>, Error> {
    let lock: serde_json::Value = serde_json::from_str(lockfile)
        .map_err(|e| Error::ExecutionErr(format!("Failed to parse renv.lock: {}", e)))?;

    // Build repo name -> URL map from R.Repositories
    let mut repo_urls: HashMap<String, String> = HashMap::new();
    if let Some(repos) = lock.get("R").and_then(|r| r.get("Repositories")).and_then(|r| r.as_array())
    {
        for repo in repos {
            if let (Some(name), Some(url)) = (
                repo.get("Name").and_then(|v| v.as_str()),
                repo.get("URL").and_then(|v| v.as_str()),
            ) {
                repo_urls.insert(name.to_string(), url.to_string());
            }
        }
    }

    let packages = lock
        .get("Packages")
        .and_then(|p| p.as_object())
        .ok_or_else(|| Error::ExecutionErr("renv.lock missing Packages field".to_string()))?;

    let mut result = vec![];
    for (_name, pkg) in packages {
        let pkg_name = pkg
            .get("Package")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let version = pkg
            .get("Version")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let repo_name = pkg
            .get("Repository")
            .and_then(|v| v.as_str())
            .unwrap_or("CRAN");
        let repo_url = repo_urls
            .get(repo_name)
            .cloned()
            .unwrap_or_else(|| "https://cloud.r-project.org".to_string());

        let mut dependencies = vec![];
        if let Some(imports) = pkg.get("Imports").and_then(|v| v.as_array()) {
            for entry in imports {
                if let Some(s) = entry.as_str() {
                    // Entries look like "cli (>= 3.6.2)" — take just the name
                    let name = s.split_whitespace().next().unwrap_or("");
                    if !name.is_empty() && name != "R" {
                        dependencies.push(name.to_string());
                    }
                }
            }
        }

        if !pkg_name.is_empty() && !version.is_empty() {
            result.push(RenvPackage { name: pkg_name, version, repo_url, dependencies });
        }
    }
    Ok(result)
}

async fn install<'a>(args: &mut JobHandlerInput<'a>, lockfile: &str, verbose: bool) -> Result<String, Error> {
    let lib_path = format!("{}/r_site_library", *R_CACHE_DIR);
    fs::create_dir_all(&lib_path).await?;

    let packages = parse_renv_lock(lockfile)?;
    if packages.is_empty() {
        return Ok(lib_path);
    }

    #[derive(Clone, Debug)]
    struct RPackagePayload {
        pkg: String,
        version: String,
        #[allow(dead_code)]
        repo_url: String,
    }

    // Build dependency graph for topological layering
    let mut graph = DependencyGraph::new();
    for renv_pkg in &packages {
        let handle = format!("{}-{}", renv_pkg.name, renv_pkg.version);
        // renv uses staged installation: it builds to a temp dir then rename()s onto
        // the target. If the target is a bind mount point, rename fails with
        // "target file already exists". We work around this by mounting the parent
        // (wrapper) dir at /install so renv can freely create /install/{pkg}/ via rename.
        let pkg_outer = format!("{}/{}_outer", lib_path, renv_pkg.name);
        let path = format!("{}/{}", pkg_outer, renv_pkg.name);
        graph.insert(
            renv_pkg.name.clone(),
            RequiredDependency {
                path,
                _s3_handle: handle,
                display_name: format!("{} ({})", renv_pkg.name, renv_pkg.version),
                custom_payload: RPackagePayload {
                    pkg: renv_pkg.name.clone(),
                    version: renv_pkg.version.clone(),
                    repo_url: renv_pkg.repo_url.clone(),
                },
            },
            renv_pkg.dependencies.clone(),
        );
    }

    let jailed = !cfg!(windows) && is_sandboxing_enabled();
    let job_dir = args.job_dir.to_owned();

    par_install_language_dependencies_seq(
        InstallDeps::Layered(graph),
        "r",
        "Rscript",
        false,
        *R_CONCURRENT_DOWNLOADS,
        move |dependency| {
            let lib_path_c = lib_path.clone();
            let job_dir = job_dir.clone();
            let pkg_name = &dependency.custom_payload.pkg;
            // pkg_outer is the wrapper dir mounted rw at /install inside nsjail.
            // renv creates /install/{pkg}/ inside it via staged rename.
            let pkg_outer = format!("{}/{}_outer", lib_path_c, pkg_name);
            std::fs::create_dir_all(&pkg_outer)?;

            let mut cmd = if jailed {
                let nsjail_proto = format!("{}.install.config.proto", Uuid::new_v4());
                let config_content = NSJAIL_CONFIG_INSTALL_R_CONTENT
                    .replace("{JOB_DIR}", &job_dir)
                    .replace("{PKG_DIR}", &pkg_outer)
                    .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                    .replace("{TRACING_PROXY_CA_CERT_PATH}", &*TRACING_PROXY_CA_CERT_PATH)
                    .replace("#{DEV}", DEV_CONF_NSJAIL);
                let _ = write_file(
                    &job_dir,
                    &nsjail_proto,
                    &config_content,
                )?;
                let mut cmd = Command::new(NSJAIL_PATH.as_str());
                cmd.args(vec![
                    "--config",
                    &nsjail_proto,
                    "--",
                    RSCRIPT_PATH.as_str(),
                ]);
                cmd
            } else {
                Command::new(if cfg!(windows) {
                    "Rscript.exe"
                } else {
                    RSCRIPT_PATH.as_str()
                })
            };

            let verbose_r = if verbose { "TRUE" } else { "FALSE" };
            let install_lib = if jailed { "/install".to_string() } else { lib_path_c };
            cmd.env_clear()
                .current_dir(&job_dir)
                .env("PATH", PATH_ENV.as_str())
                .envs(R_PROXY_ENVS.clone());
            cmd
                .args(&[
                    "-e",
                    &format!(
                        r#"options(renv.verbose = {verbose_r}); renv::install("{pkg}@{version}", library = "{lib}", dependencies = FALSE)"#,
                        verbose_r = verbose_r,
                        pkg = dependency.custom_payload.pkg,
                        version = dependency.custom_payload.version,
                        lib = install_lib,
                    ),
                    // install.packages fallback (no version pinning):
                    // &format!(
                    //     r#"install.packages("{pkg}", lib = "{lib}", repos = "{repo}", dependencies = FALSE, quiet = {quiet}, INSTALL_opts = "--no-test-load --no-lock")"#,
                    //     pkg = dependency.custom_payload.pkg,
                    //     lib = install_lib,
                    //     repo = dependency.custom_payload.repo_url,
                    //     quiet = quiet_flag,
                    // ),
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            Ok(cmd)
        },
        None,
        &args.job.id,
        &args.job.workspace_id,
        args.worker_name,
        jailed,
        args.conn,
    )
    .await?;

    Ok(format!("{}/r_site_library", *R_CACHE_DIR))
}

/// Build R_LIBS_USER from lib_path by listing *_outer subdirs.
/// Each package wrapper dir ({pkg}_outer) is added so R finds {pkg}_outer/{pkg}/DESCRIPTION.
fn r_libs_user(lib_path: &str) -> String {
    std::fs::read_dir(lib_path)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && e.file_name().to_string_lossy().ends_with("_outer")
        })
        .map(|e| e.path().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(":")
}

async fn run<'a>(
    JobHandlerInput {
        occupancy_metrics,
        mem_peak,
        canceled_by,
        worker_name,
        job,
        conn,
        job_dir,
        shared_mount,
        client,
        envs,
        base_internal_url,
        parent_runnable_path,
        ..
    }: &mut JobHandlerInput<'a>,
    lib_path: Option<&str>,
    sandbox: bool,
) -> Result<(), Error> {
    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path.clone()).await?;

    let nsjail = !cfg!(windows) && (is_sandboxing_enabled() || sandbox);
    let child = if nsjail {
        append_logs(
            &job.id,
            &job.workspace_id,
            "\n--- R CODE EXECUTION (nsjail) ---\n".to_string(),
            conn,
        )
        .await;

        write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_R_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{SHARED_MOUNT}", &shared_mount)
                .replace("{R_CACHE_DIR}", &*R_CACHE_DIR)
                .replace("{TRACING_PROXY_CA_CERT_PATH}", &*TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string()),
        )?;
        let mut cmd = Command::new(NSJAIL_PATH.as_str());
        cmd.env_clear()
            .current_dir(job_dir)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(envs)
            .envs(reserved_variables)
            .envs(R_PROXY_ENVS.clone())
            .envs(get_proxy_envs_for_lang(&ScriptLang::Rlang).await?);
        if let Some(lp) = lib_path {
            cmd.env("R_LIBS_USER", r_libs_user(lp));
        }
        cmd.args(vec![
            "--config",
            "run.config.proto",
            "--",
            RSCRIPT_PATH.as_str(),
            "main.r",
        ]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        start_child_process(cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n--- R CODE EXECUTION ---\n"),
            conn,
        )
        .await;

        let rscript_executable = if cfg!(windows) {
            "Rscript.exe"
        } else {
            RSCRIPT_PATH.as_str()
        };

        let args = vec!["main.r"];
        let mut cmd = build_command_with_isolation(rscript_executable, &args);

        cmd.env_clear()
            .current_dir(job_dir.to_owned())
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(reserved_variables)
            .envs(R_PROXY_ENVS.clone())
            .envs(get_proxy_envs_for_lang(&ScriptLang::Rlang).await?)
            .envs(envs);
        if let Some(lp) = lib_path {
            cmd.env("R_LIBS_USER", r_libs_user(lp));
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            cmd.env("SystemRoot", crate::SYSTEM_ROOT.as_str())
                .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                );
        }
        start_child_process(cmd, rscript_executable, false).await?
    };
    handle_child::handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        nsjail,
        worker_name,
        &job.workspace_id,
        "r",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;
    Ok(())
}

fn wrap(inner_content: &str) -> Result<String, Error> {
    let sig = parse_r_signature(inner_content)?;
    let spread = sig
        .args
        .clone()
        .into_iter()
        .map(|Arg { name, .. }| format!("{name} = args${name}", name = name))
        .collect_vec()
        .join(", ");
    let wm_lib_path = format!("{}/r_libs/windmill.r", *R_CACHE_DIR);
    Ok(r#"source("WM_LIB_PATH")

suppressPackageStartupMessages({
INNER_CONTENT
})

library(jsonlite)
args <- fromJSON("args.json")

tryCatch({
    res <- main(SPREAD)
    write(toJSON(res, auto_unbox = TRUE, null = "null"), "result.json")
}, error = function(e) {
    error_obj <- list(
        name = class(e)[1],
        message = conditionMessage(e),
        stack = paste(capture.output(traceback()), collapse = "\n")
    )
    write(toJSON(error_obj, auto_unbox = TRUE), "result.json")
    stop(e)
})
"#
    .replace("WM_LIB_PATH", &wm_lib_path)
    .replace("INNER_CONTENT", inner_content)
    .replace("SPREAD", &spread))
}
