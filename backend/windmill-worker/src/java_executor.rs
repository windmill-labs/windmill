use std::{collections::HashMap, path::PathBuf, process::Stdio};

use anyhow::{anyhow, bail};
use async_recursion::async_recursion;
use itertools::Itertools;
use serde_json::value::RawValue;
use tokio::{
    fs::{create_dir_all, metadata, remove_dir_all, File},
    io::AsyncWriteExt,
    process::Command,
};
use uuid::Uuid;
use windmill_common::{
    error::{self, Error},
    utils::calculate_hash,
    worker::{copy_dir_recursively, write_file, Connection},
};
use crate::global_cache::save_cache;
use windmill_parser::Arg;
use windmill_parser_java::parse_java_sig_meta;
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::{
    common::{
        build_command_with_isolation, create_args_and_out_file, get_reserved_variables,
        read_result, start_child_process, OccupancyMetrics,
    },
    handle_child, is_sandboxing_enabled, read_ee_registry,
    universal_pkg_installer::{par_install_language_dependencies_all_at_once, RequiredDependency},
    COURSIER_CACHE_DIR, DISABLE_NUSER, JAVA_CACHE_DIR, JAVA_HOME_DIR, JAVA_REPOSITORY_DIR,
    MAVEN_REPOS, NO_DEFAULT_MAVEN, NSJAIL_PATH, PATH_ENV, PROXY_ENVS,
};
use windmill_common::client::AuthedClient;

lazy_static::lazy_static! {
    static ref JAVA_CONCURRENT_DOWNLOADS: usize = std::env::var("JAVA_CONCURRENT_DOWNLOADS").ok().map(|flag| flag.parse().unwrap_or(20)).unwrap_or(20);
    static ref JAVA_PATH: String = std::env::var("JAVA_PATH").unwrap_or_else(|_| "/usr/bin/java".to_string());
    static ref JAVAC_PATH: String = std::env::var("JAVAC_PATH").unwrap_or_else(|_| "/usr/bin/javac".to_string());
    static ref CS_PATH: String = std::env::var("COURSIER_PATH").unwrap_or_else(|_| "/usr/bin/coursier".to_string());
    static ref STOREPASS: String = std::env::var("JAVA_STOREPASS").unwrap_or("123456".into());
    static ref TRUST_STORE_PATH: String = std::env::var("JAVA_TRUST_STORE_PATH").unwrap_or("/usr/local/share/ca-certificates/truststore.jks".into());
}

const NSJAIL_CONFIG_RUN_JAVA_CONTENT: &str = include_str!("../nsjail/run.java.config.proto");

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

pub async fn handle_java_job<'a>(mut args: JobHandlerInput<'a>) -> Result<Box<RawValue>, Error> {
    // --- Prepare ---
    {
        prepare(&mut args).await?;
    }
    // --- Generate Lockfile ---

    let deps = resolve(
        &args.job.id,
        &args.inner_content,
        &args.job_dir,
        &args.conn,
        &args.job.workspace_id,
    )
    .await?;

    // --- Install ---

    let classpath = install(&mut args, deps).await?;

    // --- Build .java files ---
    {
        compile(&mut args, &classpath).await?;
    }
    // --- Run ---
    {
        run(&mut args, &classpath).await?;
    }
    // --- Retrieve results ---
    {
        read_result(&args.job_dir, None).await
    }
}

async fn prepare<'a>(
    JobHandlerInput { job, conn, job_dir, client, inner_content, .. }: &mut JobHandlerInput<'a>,
) -> Result<(), Error> {
    // Create needed files
    {
        create_args_and_out_file(&client, job, job_dir, conn).await?;
        let app_path = format!("{}/src/main/java/net/script/", job_dir);
        create_dir_all(&app_path).await?;
        File::create(format!("{app_path}/App.java"))
            .await?
            .write_all(&wrap(inner_content)?.into_bytes())
            .await?;
        File::create(format!("{app_path}/Main.java"))
            .await?
            .write_all(
                &format!(
                    "package net.script;\n{MINI_CLIENT_IMPORTS}\n{}\n{MINI_CLIENT}",
                    inner_content
                )
                .into_bytes(),
            )
            .await?;
    }
    Ok(())
}

pub async fn resolve<'a>(
    job_id: &Uuid,
    code: &str,
    job_dir: &str,
    conn: &Connection,
    w_id: &str,
) -> Result<String, Error> {
    let deps = {
        let find_requirements = code.lines().find_position(|x| {
            x.starts_with("//requirements:") || x.starts_with("// requirements:")
        });

        let specified_deps = if let Some((pos, _)) = find_requirements {
            code.lines()
                .skip(pos + 1)
                .map_while(|x| {
                    if x.starts_with("//") {
                        Some(x.replace("//", "").trim().to_owned())
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
        } else {
            Default::default()
        };

        let mut deps = vec![
            // Default requirements
            "com.fasterxml.jackson.core:jackson-databind:2.9.8".to_owned(),
        ];
        deps.extend(specified_deps);
        deps.join("\n")
    };

    let req_hash = format!("java-{}", calculate_hash(&deps));
    if let Connection::Sql(db) = conn {
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
    let lock = {
        append_logs(
            job_id,
            w_id,
            format!("\n--- RESOLVING LOCKFILE ---\n"),
            &conn,
        )
        .await;

        let mut cmd = Command::new(if cfg!(windows) {
            "java"
        } else {
            JAVA_PATH.as_str()
        });
        cmd.env_clear()
            .current_dir(job_dir.to_owned())
            .env("PATH", PATH_ENV.as_str())
            .env("HOME", JAVA_HOME_DIR)
            .env("COURSIER_CACHE", COURSIER_CACHE_DIR)
            .envs(PROXY_ENVS.clone());

        // Configure proxies
        {
            let jps = parse_proxy()?;
            if let Some(val) = jps.https_host {
                cmd.arg(&format!("-Dhttps.proxyHost={}", val));
            }
            if let Some(val) = jps.https_port {
                cmd.arg(&format!("-Dhttps.proxyPort={}", val));
            }
            if let Some(val) = jps.http_host {
                cmd.arg(&format!("-Dhttp.proxyHost={}", val));
            }
            if let Some(val) = jps.http_port {
                cmd.arg(&format!("-Dhttp.proxyPort={}", val));
            }
            if let Some(val) = jps.no_proxy {
                cmd.arg(&format!("-Dhttp.nonProxyHosts=\"{}\"", val));
            }
        }
        cmd.arg(&format!("-Duser.home={}", JAVA_HOME_DIR));
        if metadata(TRUST_STORE_PATH.clone()).await.is_ok() {
            cmd.args(&[
                &format!("-Djavax.net.ssl.trustStore={}", *TRUST_STORE_PATH),
                &format!("-Djavax.net.ssl.trustStorePassword={}", *STOREPASS),
            ]);
        }
        cmd.args(&[
            "-jar",
            &CS_PATH,
            "resolve",
            &get_no_default(),
            "--parallel",
            &format!("{}", *JAVA_CONCURRENT_DOWNLOADS),
            "--cache",
            COURSIER_CACHE_DIR,
        ])
        .args(&get_repos(job_id, w_id, conn).await)
        .args(&deps.split("\n").collect_vec())
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
        let output = cmd.output().await?;
        // Check if the command was successful
        if output.status.success() {
            String::from_utf8(output.stdout).expect("Failed to convert output to String")
        } else {
            let stderr =
                String::from_utf8(output.stderr).expect("Failed to convert error output to String");
            return Err(error::Error::internal_err(stderr));
        }
    };

    if let Connection::Sql(db) = conn {
        sqlx::query!(
        "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('5 mins')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = EXCLUDED.lockfile",
        req_hash,
            lock.clone(),
        )
        .fetch_optional(db)
        .await?;
    }

    append_logs(job_id, w_id, format!("\n{}", &lock), &conn).await;
    Ok(lock)
}

async fn install<'a>(
    JobHandlerInput { worker_name, job, conn, job_dir, .. }: &mut JobHandlerInput<'a>,
    deps: String,
) -> Result<String, Error> {
    let deps = deps
        .lines()
        .map(|line| {
            let unparsed_dep = line.replace(":jar", "").replace(":lib", "");
            let mut it = unparsed_dep.split(":");

            match (it.next(), it.next(), it.next()) {
                (Some(group_id), Some(artifact_id), Some(version)) => {
                    let path = format!(
                        "{JAVA_REPOSITORY_DIR}/{}/{artifact_id}/{version}",
                        group_id.replace(".", "/")
                    );
                    Ok(RequiredDependency {
                        path,
                        _s3_handle: format!("{group_id}:{artifact_id}:{version}"),
                        display_name: format!("{artifact_id}:{version}"),
                        custom_payload: (),
                    })
                }
                _ => anyhow::bail!("{line} is not parsable"),
            }
        })
        .collect::<anyhow::Result<Vec<RequiredDependency<_>>>>()?;

    let classpath = deps
        .clone()
        .into_iter()
        .map(|RequiredDependency { path, .. }| path + "/*")
        .collect_vec()
        .join(":")
        + ":target";

    #[cfg(windows)]
    let classpath = classpath.replace(":", ";");

    tracing::debug!(
        workspace_id = %job.workspace_id,
        "JAVA classpath: {}", &classpath
    );
    let (repos, no_default, trust_store_metadata) = (
        get_repos(&job.id, &job.workspace_id, conn).await,
        get_no_default(),
        metadata(TRUST_STORE_PATH.clone()).await,
    );
    let job_dir = job_dir.to_owned();
    let fetch_dir = format!("{JAVA_CACHE_DIR}/tmp-fetch-{}", Uuid::new_v4());
    let fetch_dir2 = fetch_dir.clone();
    par_install_language_dependencies_all_at_once(
        deps,
        "java",
        "java",
        true,
        *JAVA_CONCURRENT_DOWNLOADS,
        true,
        move |dependencies| {
            let mut cmd = Command::new(if cfg!(windows) {
                "java"
            } else {
                JAVA_PATH.as_str()
            });
            let artifacts = dependencies
                .into_iter()
                .map(|e| e._s3_handle)
                .collect::<Vec<String>>();
            cmd.env_clear()
                .current_dir(&job_dir)
                .env("PATH", PATH_ENV.as_str())
                .env("HOME", JAVA_HOME_DIR)
                .env("COURSIER_CACHE", COURSIER_CACHE_DIR)
                .envs(PROXY_ENVS.clone());
            // Configure proxies
            {
                let jps = parse_proxy()?;
                if let Some(val) = jps.https_host {
                    cmd.arg(&format!("-Dhttps.proxyHost={}", val));
                }
                if let Some(val) = jps.https_port {
                    cmd.arg(&format!("-Dhttps.proxyPort={}", val));
                }
                if let Some(val) = jps.http_host {
                    cmd.arg(&format!("-Dhttp.proxyHost={}", val));
                }
                if let Some(val) = jps.http_port {
                    cmd.arg(&format!("-Dhttp.proxyPort={}", val));
                }
                if let Some(val) = jps.no_proxy {
                    cmd.arg(&format!("-Dhttp.nonProxyHosts=\"{}\"", val));
                }
            }

            cmd.arg(&format!("-Duser.home={}", JAVA_HOME_DIR));
            if trust_store_metadata.is_ok() {
                cmd.args(&[
                    &format!("-Djavax.net.ssl.trustStore={}", *TRUST_STORE_PATH),
                    &format!("-Djavax.net.ssl.trustStorePassword={}", *STOREPASS),
                ]);
            }
            cmd.args(&[
                "-jar",
                &CS_PATH,
                "fetch",
                &no_default,
                "--quiet",
                "--parallel",
                &format!("{}", *JAVA_CONCURRENT_DOWNLOADS),
                "--cache",
                &fetch_dir,
            ])
            .args(&repos)
            .arg("--intransitive")
            .args(artifacts)
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

            Ok(cmd)
        },
        async move |_| {
            move_to_repository(&fetch_dir2, 0).await?;
            remove_dir_all(&fetch_dir2).await?;
            #[async_recursion]
            async fn move_to_repository(path: &str, depth: u8) -> anyhow::Result<()> {
                if depth == 3 {
                    copy_dir_recursively(
                        &PathBuf::from(path),
                        &PathBuf::from(JAVA_REPOSITORY_DIR),
                    )?;

                    return Ok(());
                }
                let mut entries = tokio::fs::read_dir(path).await?;
                loop {
                    let Some(entry) = entries.next_entry().await? else {
                        break Ok(());
                    };

                    let path = entry
                        .path()
                        .to_str()
                        .ok_or(anyhow!("Internal Error: Cannot convert Path to Str"))?
                        .to_owned();

                    move_to_repository(&path, depth + 1).await?;
                }
            }
            Ok(())
        },
        &job.id,
        &job.workspace_id,
        worker_name,
        is_sandboxing_enabled(),
        conn,
    )
    .await?;
    Ok(classpath)
}

async fn compile<'a>(
    JobHandlerInput {
        occupancy_metrics,
        mem_peak,
        canceled_by,
        worker_name,
        job,
        conn,
        job_dir,
        client,
        envs,
        base_internal_url,
        inner_content,
        requirements_o,
        parent_runnable_path,
        ..
    }: &mut JobHandlerInput<'a>,
    classpath: &'a str,
    // plugins: Vec<&'a str>,
) -> Result<(), Error> {
    fn compute_hash(code: &str, requirements_o: Option<&String>) -> String {
        calculate_hash(&format!(
            "{}{}",
            code,
            requirements_o
                .as_ref()
                .map(|x| x.to_string())
                .unwrap_or_default()
        ))
    }
    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path.clone()).await?;
    let hash = compute_hash(inner_content, *requirements_o);
    let bin_path = format!("{}/{hash}", JAVA_CACHE_DIR);
    let remote_path = format!("java_jar/{hash}");
    let (cache, ..) = crate::global_cache::load_cache(&bin_path, &remote_path, true).await;

    if cache {
        let target = format!("{job_dir}/target");

        #[cfg(unix)]
        let symlink = std::os::unix::fs::symlink(&bin_path, &target);
        #[cfg(windows)]
        let symlink = copy_dir_recursively(&PathBuf::from(&bin_path), &PathBuf::from(&target));

        symlink.map_err(|e| {
            Error::ExecutionErr(format!(
                "could not copy cached binary from {bin_path} to {job_dir}/main: {e:?}"
            ))
        })?;
    } else {
        // let plugin_registry = format!("{job_dir}/plugin-registry");
        let child = {
            append_logs(
                &job.id,
                &job.workspace_id,
                format!("\n--- COMPILING .JAVA FILES\n"),
                &conn,
            )
            .await;

            let mut cmd = Command::new(if cfg!(windows) {
                "javac"
            } else {
                JAVAC_PATH.as_str()
            });
            cmd.env_clear()
                .current_dir(job_dir.to_owned())
                .env("PATH", PATH_ENV.as_str())
                .env("HOME", JAVA_HOME_DIR)
                .env("BASE_INTERNAL_URL", base_internal_url)
                .envs(envs)
                .envs(reserved_variables)
                .envs(PROXY_ENVS.clone())
                .args(&[
                    "-classpath",
                    &classpath,
                    "src/main/java/net/script/Main.java",
                    "src/main/java/net/script/App.java",
                    "-d",
                    "./target",
                ])
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
            start_child_process(cmd, "javac", false).await?
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
            "javac",
            job.timeout,
            false,
            &mut Some(occupancy_metrics),
            None,
            None,
        )
        .await?;

        match save_cache(
            &bin_path,
            &format!("java_jar/{hash}"),
            &format!("{job_dir}/target"),
            true,
        )
        .await
        {
            Err(e) => {
                let em = format!(
                    "could not save {bin_path} to {} to java cache: {e:?}",
                    format!("{job_dir}/main"),
                );
                tracing::error!(em);
            }
            Ok(logs) => {
                tracing::trace!(logs);
            }
        }
    };

    Ok(())
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
    classpath: &'a str,
) -> Result<(), Error> {
    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path.clone()).await?;

    let child = if !cfg!(windows) && is_sandboxing_enabled() {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n--- ISOLATED JAVA CODE EXECUTION ---\n"),
            &conn,
        )
        .await;

        write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_JAVA_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", JAVA_CACHE_DIR)
                .replace("{SHARED_MOUNT}", &shared_mount)
                // .replace("{CACHED_TARGET}", &shared_mount)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string()),
        )?;
        let mut cmd = Command::new(NSJAIL_PATH.as_str());
        cmd.env_clear()
            .current_dir(job_dir)
            .env("PATH", PATH_ENV.as_str())
            .env("HOME", JAVA_HOME_DIR)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(envs)
            .envs(reserved_variables)
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                JAVA_PATH.as_str(),
            ]);
        if metadata(TRUST_STORE_PATH.clone()).await.is_ok() {
            cmd.args(&[
                &format!("-Djavax.net.ssl.trustStore={}", *TRUST_STORE_PATH),
                &format!("-Djavax.net.ssl.trustStorePassword={}", *STOREPASS),
            ]);
        }
        // Configure proxies
        {
            let jps = parse_proxy()?;
            if let Some(val) = jps.https_host {
                cmd.arg(&format!("-Dhttps.proxyHost={}", val));
            }
            if let Some(val) = jps.https_port {
                cmd.arg(&format!("-Dhttps.proxyPort={}", val));
            }
            if let Some(val) = jps.http_host {
                cmd.arg(&format!("-Dhttp.proxyHost={}", val));
            }
            if let Some(val) = jps.http_port {
                cmd.arg(&format!("-Dhttp.proxyPort={}", val));
            }
            if let Some(val) = jps.no_proxy {
                cmd.arg(&format!("-Dhttp.nonProxyHosts=\"{}\"", val));
            }
        }
        cmd.args(vec!["-classpath", &classpath, "net.script.App"]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        start_child_process(cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n--- JAVA CODE EXECUTION ---\n"),
            &conn,
        )
        .await;

        let java_executable = if cfg!(windows) {
            "java"
        } else {
            JAVA_PATH.as_str()
        };

        let mut cmd = build_command_with_isolation(java_executable, &[]);
        cmd.env_clear()
            .current_dir(job_dir.to_owned())
            .env("PATH", PATH_ENV.as_str())
            .env("HOME", JAVA_HOME_DIR)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(envs)
            .envs(reserved_variables);
        if metadata(TRUST_STORE_PATH.clone()).await.is_ok() {
            cmd.args(&[
                &format!("-Djavax.net.ssl.trustStore={}", *TRUST_STORE_PATH),
                &format!("-Djavax.net.ssl.trustStorePassword={}", *STOREPASS),
            ]);
        }
        // Configure proxies
        {
            let jps = parse_proxy()?;
            if let Some(val) = jps.https_host {
                cmd.arg(&format!("-Dhttps.proxyHost={}", val));
            }
            if let Some(val) = jps.https_port {
                cmd.arg(&format!("-Dhttps.proxyPort={}", val));
            }
            if let Some(val) = jps.http_host {
                cmd.arg(&format!("-Dhttp.proxyHost={}", val));
            }
            if let Some(val) = jps.http_port {
                cmd.arg(&format!("-Dhttp.proxyPort={}", val));
            }
            if let Some(val) = jps.no_proxy {
                cmd.arg(&format!("-Dhttp.nonProxyHosts=\"{}\"", val));
            }
        }
        cmd.args(&["-classpath", &classpath, "net.script.App"])
            .stdin(Stdio::null())
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
        start_child_process(cmd, java_executable, false).await?
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
        "java",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;
    Ok(())
}

#[derive(Default, Debug)]
struct JavaProxySettings {
    http_host: Option<String>,
    http_port: Option<String>,
    https_host: Option<String>,
    https_port: Option<String>,
    no_proxy: Option<String>,
}
fn parse_proxy() -> anyhow::Result<JavaProxySettings> {
    let mut jps = JavaProxySettings::default();
    for (ident, mut val) in PROXY_ENVS.clone() {
        match ident {
            "HTTPS_PROXY" => {
                if !val.contains("://") {
                    val = format!("https://{val}");
                }
                let mut url = url::Url::parse(&val)?;
                let port = url.port();
                {
                    url.set_port(None).unwrap_or_default();
                    let host = url.as_str().replace("https://", "").replace("http://", "");
                    jps.https_host = Some(host);
                    if let Some(port) = port {
                        jps.https_port = Some(format!("{}", port));
                    }
                }
            }
            "HTTP_PROXY" => {
                if val.contains("https://") {
                    bail!("HTTP_PROXY url cannot contain https scheme.");
                }
                if !val.contains("http://") {
                    val = format!("http://{val}");
                }
                let mut url = url::Url::parse(&val)?;
                let port = url.port();
                // Make sure port and schema is not included in final url
                {
                    url.set_port(None).unwrap_or_default();
                    jps.http_host = Some(url.as_str().replace("http://", ""));
                    if let Some(port) = port {
                        jps.https_port = Some(format!("{}", port));
                    }
                }
            }
            // Java uses | instead of ,
            "NO_PROXY" => jps.no_proxy = Some(val.replace(",", "|")),
            _ => {}
        }
    }

    Ok(jps)
}
async fn get_repos(job_id: &Uuid, w_id: &str, conn: &Connection) -> Vec<String> {
    read_ee_registry(
        MAVEN_REPOS.read().await.clone(),
        "maven repos",
        job_id,
        w_id,
        conn,
    )
    .await
    .as_ref()
    .map(|repos| {
        repos
            .trim()
            .split_whitespace()
            .into_iter()
            .map(|el| vec!["--repository".to_owned(), el.to_owned()])
            .collect_vec()
    })
    .unwrap_or_default()
    .concat()
}

fn get_no_default() -> String {
    if NO_DEFAULT_MAVEN.load(std::sync::atomic::Ordering::Relaxed) {
        "--no-default"
    } else {
        // Command does not take empty arguments
        "-q"
    }
    .into()
}

/// Wraps content script
/// that upon execution reads args.json (which are piped and transformed from previous flow step or top level inputs)
/// Also wrapper takes output of program and serializes to result.json (Which windmill will know how to use later)
fn wrap(inner_content: &str) -> Result<String, Error> {
    let sig = parse_java_sig_meta(inner_content)?;
    let ret_void = sig.returns_void;
    let spread = sig
        .main_sig
        .args
        .clone()
        .into_iter()
        .map(|Arg { name, .. }| {
            // Apply additional input transformation
            format!(" parsedArgs.{name}")
        })
        .collect_vec()
        .join(",");
    let args = sig
        .main_sig
        .args
        .clone()
        .into_iter()
        .map(|Arg { name, otyp, .. }| {
            // Apply additional input transformation
            format!("public {} {name};\n", otyp.unwrap())
        })
        .collect_vec()
        .join(" ");
    Ok(r#"
package net.script;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.FileInputStream;
import java.io.InputStream;
import java.io.FileOutputStream;
import net.script.Main;

public class App{

  public static class Args {ARGS}

  public static void main(String[] args) {
    try {
      InputStream fileInputStream = new FileInputStream("args.json");
      ObjectMapper mapper = new ObjectMapper();
      Args parsedArgs = mapper.readValue(fileInputStream, Args.class);
      fileInputStream.close();
      {MAIN_HANDLER}
      FileOutputStream fileOutputStream = new FileOutputStream("result.json");
      mapper.writeValue(fileOutputStream, res);
      fileOutputStream.close();

    } catch (Exception e) { // Catching general Exception
        e.printStackTrace(); // Handle the exception
    }
  }
}
            "#
    .replace(
        "{MAIN_HANDLER}",
        if ret_void {
            "
            Main.main(SPREAD);
            Object res = null;
            "
        } else {
            "
            Object res = Main.main(SPREAD);
            "
        },
    )
    .replace("SPREAD", &spread)
    .replace("ARGS", &args))
}
const MINI_CLIENT_IMPORTS: &str = r#"
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
"#;
const MINI_CLIENT: &str = r#"
class Wmill {
    public static String getVariable(String path) {
        var baseUrl = System.getenv("BASE_INTERNAL_URL");
        var workspace = System.getenv("WM_WORKSPACE");
        var uri = java.text.MessageFormat.format("{0}/api/w/{1}/variables/get_value/{2}", baseUrl, workspace, path);

        // Create an HttpRequest
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(uri))
                .header("Authorization", "Bearer " + System.getenv("WM_TOKEN")) // Add the Authorization header
                .GET() // Set the request method to GET
                .build();

            // Send the request and get the response
        return HttpClient.newHttpClient().sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(HttpResponse::body)
                .join(); // Wait for the completion
    }
    public static String getResource(String path) {
        var baseUrl = System.getenv("BASE_INTERNAL_URL");
        var workspace = System.getenv("WM_WORKSPACE");
        var uri = java.text.MessageFormat.format("{0}/api/w/{1}/resources/get_value_interpolated/{2}", baseUrl, workspace, path);

        // Create an HttpRequest
        HttpRequest request = HttpRequest.newBuilder()
                .uri(URI.create(uri))
                .header("Authorization", "Bearer " + System.getenv("WM_TOKEN")) // Add the Authorization header
                .GET() // Set the request method to GET
                .build();

            // Send the request and get the response
        return HttpClient.newHttpClient().sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(HttpResponse::body)
                .join(); // Wait for the completion
    }
}
"#;
