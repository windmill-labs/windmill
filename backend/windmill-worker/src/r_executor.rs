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
    worker::{write_file, Connection},
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
    is_sandboxing_enabled, DISABLE_NUSER, NSJAIL_PATH, PATH_ENV, PROXY_ENVS, R_CACHE_DIR,
    TRACING_PROXY_CA_CERT_PATH,
};
use windmill_common::scripts::ScriptLang;

lazy_static::lazy_static! {
    static ref RSCRIPT_PATH: String = std::env::var("RSCRIPT_PATH").unwrap_or_else(|_| "/usr/bin/Rscript".to_string());
    static ref R_PROXY_ENVS: Vec<(String, String)> = {
        PROXY_ENVS
            .clone()
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect()
    };
}

const NSJAIL_CONFIG_RUN_R_CONTENT: &str = include_str!("../nsjail/run.r.config.proto");

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
    )
    .await?;
    // --- Install ---
    let lib_path = if !lockfile.is_empty() {
        Some(install(&mut args, &lockfile).await?)
    } else {
        None
    };
    // --- Execute ---
    {
        run(&mut args, lib_path.as_deref()).await?;
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
    if !std::fs::metadata(&wm_lib_path).is_ok() {
        fs::create_dir_all(&wm_lib_path).await?;

        File::create(format!("{}/windmill.r", &wm_lib_path))
            .await?
            .write_all(
                r##"
# Windmill mini client methods for R
# Uses system curl to avoid requiring any extra R packages

.wm_fetch <- function(url) {
    token <- Sys.getenv("WM_TOKEN")
    result <- system2(
        "curl", c("-s", "-f",
            "-H", paste0("Authorization: Bearer ", token),
            url),
        stdout = TRUE, stderr = TRUE
    )
    if (!is.null(attr(result, "status")) && attr(result, "status") != 0) {
        stop(paste("HTTP request failed:", paste(result, collapse = "\n")))
    }
    jsonlite::fromJSON(paste(result, collapse = "\n"))
}

get_variable <- function(path) {
    base_url <- Sys.getenv("BASE_INTERNAL_URL")
    workspace <- Sys.getenv("WM_WORKSPACE")
    url <- paste0(base_url, "/api/w/", workspace, "/variables/get_value/", path)
    .wm_fetch(url)
}

get_resource <- function(path) {
    base_url <- Sys.getenv("BASE_INTERNAL_URL")
    workspace <- Sys.getenv("WM_WORKSPACE")
    url <- paste0(base_url, "/api/w/", workspace, "/resources/get_value_interpolated/", path)
    .wm_fetch(url)
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
) -> Result<String, Error> {
    let packages = parse_r_requirements(inner_content)?;
    if packages.is_empty() {
        return Ok(String::new());
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

    let pkg_vec = packages
        .lines()
        .map(|p| format!("\"{}\"", p))
        .collect_vec()
        .join(", ");

    let resolve_script = format!(
        r#"pkgs <- c({pkg_vec})
con <- file("{job_dir}/r.lock", open = "w")
av <- tryCatch(
    available.packages(repos = "https://cloud.r-project.org"),
    error = function(e) NULL
)
for (pkg in pkgs) {{
    if (!is.null(av) && pkg %in% rownames(av)) {{
        cat(paste0(pkg, "==", av[pkg, "Version"], "\n"))
        writeLines(paste0(pkg, "==", av[pkg, "Version"]), con)
    }} else if (requireNamespace(pkg, quietly = TRUE)) {{
        v <- as.character(packageVersion(pkg))
        cat(paste0(pkg, "==", v, "\n"))
        writeLines(paste0(pkg, "==", v), con)
    }} else {{
        cat(paste0(pkg, "==latest\n"))
        writeLines(paste0(pkg, "==latest"), con)
    }}
}}
close(con)
"#
    );

    let mut file = File::create(format!("{}/resolve.r", job_dir)).await?;
    file.write_all(resolve_script.as_bytes()).await?;

    let rscript_executable = if cfg!(windows) {
        "Rscript.exe"
    } else {
        RSCRIPT_PATH.as_str()
    };

    let mut cmd = Command::new(rscript_executable);
    cmd.current_dir(job_dir)
        .env("PATH", PATH_ENV.as_str())
        .arg("resolve.r")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = start_child_process(cmd, rscript_executable, false).await?;
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

    let lock_path = format!("{}/r.lock", job_dir);
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

async fn install<'a>(args: &mut JobHandlerInput<'a>, lockfile: &str) -> Result<String, Error> {
    let lib_path = format!("{}/r_site_library", *R_CACHE_DIR);
    fs::create_dir_all(&lib_path).await?;

    // Parse lockfile to find packages that need installing
    let mut to_install = vec![];
    for line in lockfile.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, "==").collect();
        if parts.len() != 2 {
            continue;
        }
        let pkg = parts[0];
        let version = parts[1];

        // Check if already installed at the right version
        let pkg_dir = format!("{}/{}", lib_path, pkg);
        let desc_file = format!("{}/DESCRIPTION", pkg_dir);
        if let Ok(desc) = tokio::fs::read_to_string(&desc_file).await {
            if desc.contains(&format!("Version: {}", version)) {
                continue; // Already installed at correct version
            }
        }
        to_install.push((pkg.to_string(), version.to_string()));
    }

    if to_install.is_empty() {
        return Ok(lib_path);
    }

    append_logs(
        &args.job.id,
        &args.job.workspace_id,
        format!(
            "\n--- INSTALLING R PACKAGES ---\n{}\n",
            to_install
                .iter()
                .map(|(p, v)| format!("{} ({})", p, v))
                .join(", ")
        ),
        args.conn,
    )
    .await;

    // Build install script
    let install_calls = to_install
        .iter()
        .map(|(pkg, version)| {
            if version == "latest" {
                format!(
                    r#"install.packages("{pkg}", lib = "{lib_path}", repos = "https://cloud.r-project.org", quiet = TRUE)"#
                )
            } else {
                // Try exact version first via remotes, fall back to latest
                format!(
                    r#"tryCatch({{
    if (requireNamespace("remotes", quietly = TRUE)) {{
        remotes::install_version("{pkg}", version = "{version}", lib = "{lib_path}", repos = "https://cloud.r-project.org", quiet = TRUE)
    }} else {{
        install.packages("{pkg}", lib = "{lib_path}", repos = "https://cloud.r-project.org", quiet = TRUE)
    }}
}}, error = function(e) {{
    install.packages("{pkg}", lib = "{lib_path}", repos = "https://cloud.r-project.org", quiet = TRUE)
}})"#
                )
            }
        })
        .join("\n");

    let install_script = format!(
        r#".libPaths(c("{lib_path}", .libPaths()))
{install_calls}
"#
    );

    let mut file = File::create(format!("{}/install.r", args.job_dir)).await?;
    file.write_all(install_script.as_bytes()).await?;

    let rscript_executable = if cfg!(windows) {
        "Rscript.exe"
    } else {
        RSCRIPT_PATH.as_str()
    };

    let mut cmd = Command::new(rscript_executable);
    cmd.current_dir(args.job_dir)
        .env("PATH", PATH_ENV.as_str())
        .arg("install.r")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = start_child_process(cmd, rscript_executable, false).await?;
    handle_child::handle_child(
        &args.job.id,
        args.conn,
        args.mem_peak,
        args.canceled_by,
        child,
        false,
        args.worker_name,
        &args.job.workspace_id,
        "r install",
        None,
        false,
        &mut None,
        None,
        None,
    )
    .await?;

    Ok(lib_path)
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
) -> Result<(), Error> {
    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path.clone()).await?;

    let child = if !cfg!(windows) && is_sandboxing_enabled() {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n--- ISOLATED R CODE EXECUTION ---\n"),
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
            cmd.env("R_LIBS_USER", lp);
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
            cmd.env("R_LIBS_USER", lp);
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
        is_sandboxing_enabled(),
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
    Ok(r#"INNER_CONTENT

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
    .replace("INNER_CONTENT", inner_content)
    .replace("SPREAD", &spread))
}
