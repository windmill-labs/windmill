use std::{collections::HashMap, process::Stdio, sync::Arc, time::Instant};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{Pool, Postgres};
use tokio::{
    fs::{create_dir_all, metadata, read_to_string, File},
    io::AsyncWriteExt,
    process::Command,
    sync::{RwLock, Semaphore},
};
use uuid::Uuid;
use windmill_common::{
    ee::{get_license_plan, LicensePlan},
    error::Error,
    jobs::QueuedJob,
    s3_helpers::OBJECT_STORE_CACHE_SETTINGS,
    utils::calculate_hash,
    worker::{pad_string, save_cache, write_file},
};
use windmill_macros::annotations;
use windmill_parser::Arg;
use windmill_parser_java::parse_java_signature;
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    common::{
        create_args_and_out_file, get_reserved_variables, read_result, start_child_process,
        OccupancyMetrics,
    },
    global_cache::{build_tar_and_push, pull_from_tar},
    handle_child, AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, JAVA_CACHE_DIR,
    MAVEN_CONFIG, NSJAIL_PATH, PATH_ENV, PROXY_ENVS,
};
lazy_static::lazy_static! {
    static ref JAVA_CONCURRENT_DOWNLOADS: usize = std::env::var("JAVA_CONCURRENT_DOWNLOADS").ok().map(|flag| flag.parse().unwrap_or(20)).unwrap_or(20);
    static ref JAVA_PATH: String = std::env::var("JAVA_PATH").unwrap_or_else(|_| "/usr/bin/java".to_string());
    static ref JAVAC_PATH: String = std::env::var("JAVAC_PATH").unwrap_or_else(|_| "/usr/bin/javac".to_string());
    static ref MAVEN_PATH: String = std::env::var("MAVEN_PATH").unwrap_or_else(|_| "/usr/bin/mvn".to_string());
    static ref FIRST_RUN: RwLock<bool> = RwLock::new(true);
}
const NSJAIL_CONFIG_RUN_JAVA_CONTENT: &str = include_str!("../nsjail/run.java.config.proto");
const POM_XML_TEMPLATE: &str = include_str!("../init-pom.xml");

#[derive(Copy, Clone)]
#[annotations("//")]
pub struct Annotations {
    pub jared: bool,
    /// Do not (use) cache on S3
    pub no_s3: bool,
    /// Do not use cached lock from db
    pub no_lock_cache: bool,
}

// TODO: Can be generalized and used for other handlers
#[allow(dead_code)]
pub(crate) struct JobHandlerInput<'a> {
    pub base_internal_url: &'a str,
    pub canceled_by: &'a mut Option<CanceledBy>,
    pub client: &'a AuthedClientBackgroundTask,
    pub db: &'a sqlx::Pool<sqlx::Postgres>,
    pub envs: HashMap<String, String>,
    pub inner_content: &'a str,
    pub job: &'a QueuedJob,
    pub job_dir: &'a str,
    pub mem_peak: &'a mut i32,
    pub occupancy_metrics: &'a mut OccupancyMetrics,
    pub requirements_o: Option<&'a String>,
    pub shared_mount: &'a str,
    pub worker_name: &'a str,
}

pub fn compute_hash(code: &str, requirements_o: Option<&String>) -> String {
    calculate_hash(&format!(
        "{}{}",
        code,
        requirements_o
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or_default()
    ))
}

pub async fn handle_java_job<'a>(mut args: JobHandlerInput<'a>) -> Result<Box<RawValue>, Error> {
    let Annotations { jared, no_s3, no_lock_cache } = Annotations::parse(&args.inner_content);
    // --- Prepare ---
    {
        create_args_and_out_file(&args.client, args.job, args.job_dir, args.db).await?;
        let app_path = format!("{}/src/main/java/net/script/", args.job_dir);
        create_dir_all(&app_path).await?;
        File::create(format!("{app_path}/App.java"))
            .await?
            .write_all(&wrap(args.inner_content)?.into_bytes())
            .await?;
        File::create(format!("{app_path}/Main.java"))
            .await?
            // .write_all(&format!("{}", args.inner_content).into_bytes())
            .write_all(
                &format!(
                    "package net.script;\n{MINI_CLIENT_IMPORTS}\n{}\n{MINI_CLIENT}",
                    args.inner_content
                )
                .into_bytes(),
            )
            .await?;
    }
    // --- Generate Lockfile ---
    let deps = resolve_dependencies(
        &args.job.id,
        &args.inner_content,
        args.mem_peak,
        args.canceled_by,
        &args.job_dir,
        &args.db,
        &args.worker_name,
        &args.job.workspace_id,
        args.occupancy_metrics,
    )
    .await?;

    // --- Install ---

    let classpath = install(&mut args, deps, no_s3).await?;

    // --- Build to JAR ---
    {
        compile(&mut args, &classpath).await?;
    }

    // --- Run ---
    {
        run(&mut args, &classpath).await?;
    }
    // --- Retrieve results ---
    {
        read_result(&args.job_dir).await
    }
}

pub async fn resolve_dependencies<'a>(
    job_id: &Uuid,
    code: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    w_id: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> Result<String, Error> {
    let find_requirements = code
        .lines()
        .find_position(|x| x.starts_with("//<dependency>") || x.starts_with("// <dependency>"));
    let deps = if let Some((pos, _)) = find_requirements {
        code.lines()
            .skip(pos)
            .map_while(|x| {
                if x.starts_with("//") {
                    Some(x.to_owned())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
            .replace("//", "")
    } else {
        "".to_owned()
    };
    let find_repos = code
        .lines()
        .find_position(|x| x.starts_with("//<repository>") || x.starts_with("// <repository>"));
    let repos = if let Some((pos, _)) = find_repos {
        code.lines()
            .skip(pos + 1)
            .map_while(|x| {
                if x.starts_with("//") {
                    Some(x.to_owned())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
            .replace("//", "")
    } else {
        "".to_owned()
    };
    let pom = POM_XML_TEMPLATE
        .to_owned()
        .replace("<!-- SPREAD_DEPENDENCIES -->", &format!("{deps}"))
        .replace("<!-- SPREAD_REPOS -->", &format!("{repos}"));

    let req_hash = format!("java-{}", calculate_hash(&pom));

    File::create(format!("{}/pom.xml", job_dir))
        .await?
        .write_all(&pom.into_bytes())
        .await?;

    // TODO: Use not pip_resolution_cache?
    if let Some(cached) = sqlx::query_scalar!(
        "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
        req_hash
    )
    .fetch_optional(db)
    .await?
    {
        append_logs(
            job_id,
            w_id,
            &format!("\nFound cached resolution: {req_hash}",),
            db.clone(),
        )
        .await;
        return Ok(cached);
    }
    let child = {
        append_logs(
            job_id,
            w_id,
            format!("\n\n--- RESOLVING LOCKFILE ---\n"),
            db.clone(),
        )
        .await;

        let mut cmd = Command::new(if cfg!(windows) {
            "mvn"
        } else {
            MAVEN_PATH.as_str()
        });
        cmd.env_clear()
            .current_dir(job_dir.to_owned())
            .env(
                "MAVEN_OPTS",
                "-Dmaven.repo.local=/tmp/windmill/cache/java/maven-meta",
            )
            .env("PATH", PATH_ENV.as_str())
            // .envs(envs)
            .envs(PROXY_ENVS.clone())
            .args(&["dependency:collect", "-DoutputFile=dependencies.txt", "-q"])
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
        start_child_process(cmd, "mvn").await?
    };

    handle_child::handle_child(
        job_id,
        db,
        mem_peak,
        canceled_by,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        w_id,
        "mvn",
        None,
        false,
        &mut Some(occupancy_metrics),
    )
    .await?;

    let lock = read_to_string(format!("{job_dir}/dependencies.txt"))
        .await?
        .lines()
        .skip(2)
        .map(|dep| dep.trim())
        .collect_vec()
        .join("\n");

    sqlx::query!(
        "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('3 days')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = $2",
        req_hash,
        lock.clone(),
    ).fetch_optional(db).await?;

    Ok(dbg!(lock))
    // .map_err(|e| Error::IoErr { error: e, location: "".into() }))
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

/// Wraps content script
/// that upon execution reads args.json (which are piped and transformed from previous flow step or top level inputs)
/// Also wrapper takes output of program and serializes to result.json (Which windmill will know how to use later)
fn wrap(inner_content: &str) -> Result<String, Error> {
    let sig = parse_java_signature(inner_content)?;
    let spread = sig
        .args
        .clone()
        .into_iter()
        .map(|Arg { name, typ, has_default, .. }| {
            // Apply additional input transformation
            format!(" parsedArgs.{name}")
        })
        .collect_vec()
        .join(",");
    let args = sig
        .args
        .clone()
        .into_iter()
        .map(|Arg { name, typ, has_default, otyp, .. }| {
            // Apply additional input transformation
            format!("public {} {name};\n", otyp.unwrap())
        })
        .collect_vec()
        .join(" ");
    Ok(
        r#"    
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
      Object res = Main.main(SPREAD);
      FileOutputStream fileOutputStream = new FileOutputStream("result.json");
      mapper.writeValue(fileOutputStream, res);
      fileOutputStream.close();

    } catch (Exception e) { // Catching general Exception
        e.printStackTrace(); // Handle the exception
    }
  }
}
            "#
        .replace("SPREAD", &spread) // .replace("TRANSFORM", transform)
        .replace("ARGS", &args), // .replace("TRANSFORM", transform)
                                 // .replace("INNER_CONTENT", inner_content)
    )
}

async fn install<'a>(
    JobHandlerInput {
        occupancy_metrics,
        mem_peak,
        canceled_by,
        worker_name,
        job,
        db,
        job_dir,
        shared_mount,
        client,
        envs,
        base_internal_url,
        inner_content,
        ..
    }: &mut JobHandlerInput<'a>,
    deps: String,
    no_s3: bool,
) -> Result<String, Error> {
    append_logs(
        &job.id,
        &job.workspace_id,
        format!("\n\n--- INSTALLATION ---\n"),
        db.clone(),
    )
    .await;

    // Total to install
    let mut to_install = vec![];
    // Only install by maven
    let mut mvn_install = vec![];
    let total_to_install;
    // let mut not_installed = vec![];
    let counter_arc = Arc::new(tokio::sync::Mutex::new(0));
    // Append logs with line like this:
    // [9/21]   +  requests==2.32.3            << (S3) |  in 57ms
    #[allow(unused_assignments)]
    async fn print_success(
        mut s3_pull: bool,
        mut s3_push: bool,
        job_id: &Uuid,
        w_id: &str,
        req: &str,
        req_tl: usize,
        counter_arc: Arc<tokio::sync::Mutex<usize>>,
        total_to_install: usize,
        instant: std::time::Instant,
        db: Pool<Postgres>,
    ) {
        #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
        {
            (s3_pull, s3_push) = (false, false);
        }

        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if OBJECT_STORE_CACHE_SETTINGS.read().await.is_none() {
            (s3_pull, s3_push) = (false, false);
        }

        let mut counter = counter_arc.lock().await;
        *counter += 1;

        append_logs(
            job_id,
            w_id,
            format!(
                "\n{}+  {}{}{}|  in {}ms",
                pad_string(&format!("[{}/{total_to_install}]", counter), 9),
                // Because we want to align to max len [999/999] we take ^
                //                                     123456789
                pad_string(&req, req_tl + 1),
                // Margin to the right    ^
                if s3_pull { "<< (S3) " } else { "" },
                if s3_push { " > (S3) " } else { "" },
                instant.elapsed().as_millis(),
            ),
            db,
        )
        .await;
        // Drop lock, so next print success can fire
    }
    let deps = deps
        .lines()
        .map(|l| {
            let l = l.replace(":jar", "").replace(":lib", "");
            let mut iterator = l.split(":");

            let group_id = iterator.next().unwrap();
            let artifact_id = iterator.next().unwrap();
            let version = iterator.next().unwrap();

            let path = format!(
                "/tmp/windmill/cache/java/maven-jars/{}/{artifact_id}/{version}",
                group_id.replace(".", "/")
            );

            (path, format!("{group_id}:{artifact_id}:{version}"))
        })
        .collect_vec();

    let classpath = deps
        .clone()
        .into_iter()
        .map(|(path, ..)| path + "/*")
        .collect_vec()
        .join(":")
        + ":target";

    // TODO: Change to debug
    tracing::info!(
        workspace_id = %job.workspace_id,
        "JAVA classpath: {}", &classpath
    );

    for (dep, name) in &deps {
        if metadata(dep).await.is_ok() {
            append_logs(
                &job.id,
                &job.workspace_id,
                format!("\n{name} already in cache"),
                db.clone(),
            )
            .await;
        } else {
            to_install.push((dep.clone(), name.clone()));
        }
    }
    total_to_install = to_install.len();
    if total_to_install == 0 {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n\nINFO: All dependencies are present, skipping install\n"),
            db.clone(),
        )
        .await;

        return Ok(classpath);
    }
    let mut name_tl = 0;
    for (.., name) in &to_install {
        if name.len() > name_tl {
            name_tl = name.len();
        }
    }
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    let is_not_pro = !matches!(get_license_plan().await, LicensePlan::Pro);

    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if is_not_pro {
        for (path, name) in to_install {
            if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
                let (path_2, name_2, job_id_2, w_id_2, db_2, counter_arc) = (
                    path.clone(),
                    name.clone(),
                    job.id.clone(),
                    job.workspace_id.clone(),
                    db.clone(),
                    counter_arc.clone(),
                );
                let start = std::time::Instant::now();
                tokio::select! {
                    pull = pull_from_tar(os, path.clone(), "".into(), Some(name.clone())) => {
                        if let Err(e) = pull {
                            tracing::info!(
                                workspace_id = %w_id_2,
                                "No tarball was found for {} on S3 or different problem occured {job_id_2}:\n{e}",
                                &name_2
                            );
                            mvn_install.push((path, name));
                        } else {
                            print_success(
                                true,
                                false,
                                &job_id_2,
                                &w_id_2,
                                &name,
                                name_tl,
                                counter_arc,
                                total_to_install,
                                start,
                                db_2,
                            )
                            .await;
                        }
                    }
                }
            }
        }
    }

    // Parallelism level (N)
    let parallel_limit = // Semaphore will panic if value less then 1
        JAVA_CONCURRENT_DOWNLOADS.clamp(1, 30);

    // tracing::info!(
    //     workspace_id = %job.w_id,
    //     // is_ok = out,
    //     "Java install parallel limit: {}, job: {}",
    //     parallel_limit,
    //     job_id
    // );
    let client = &client.get_authed().await;
    let reserved_variables = get_reserved_variables(job, &client.token, db).await?;

    let mut handles = vec![];
    let semaphore = Arc::new(Semaphore::new(parallel_limit));
    // let mut handles = Vec::with_capacity(total_to_install);
    for (path, name) in mvn_install {
        let permit = semaphore.clone().acquire_owned().await; // Acquire a permit

        if let Err(_) = permit {
            // tracing::error!(
            //     workspace_id = %w_id,
            //     "Cannot acquire permit on semaphore, that can only mean that semaphore has been closed."
            // );
            break;
        }

        let permit = permit.unwrap();

        let child = {
            let mut cmd = Command::new(if cfg!(windows) {
                "mvn"
            } else {
                MAVEN_PATH.as_str()
            });
            cmd.env_clear()
                .current_dir(job_dir.to_owned())
                .env(
                    "MAVEN_OPTS",
                    "-Dmaven.repo.local=/tmp/windmill/cache/java/maven-jars",
                )
                .env("PATH", PATH_ENV.as_str())
                .envs(PROXY_ENVS.clone())
                .args(&[
                    "dependency:get",
                    &format!("-Dartifact={name}"),
                    "-Dtransitive=false",
                    "-q",
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
            start_child_process(cmd, "mvn").await?
        };

        let (worker_name_2, path_2, name_2, job_id_2, w_id_2, db_2, counter_arc) = (
            worker_name.to_owned(),
            path.clone(),
            name.clone(),
            job.id.clone(),
            job.workspace_id.clone(),
            db.clone(),
            counter_arc.clone(),
        );
        let start = std::time::Instant::now();
        let handle = tokio::spawn(async move {
            let _permit = permit;
            // let job_id = job_id_2.clone();

            if let Err(e) = handle_child::handle_child(
                &job_id_2,
                &db_2,
                &mut 0,
                &mut None,
                child,
                !*DISABLE_NSJAIL,
                &worker_name_2,
                &w_id_2,
                "mvn",
                None,
                false,
                &mut None,
            )
            .await
            {
                append_logs(
                    &job_id_2,
                    &w_id_2,
                    format!("error while installing {}: {e:?}", &name_2),
                    db_2.clone(),
                )
                .await;
            } else {
                print_success(
                    false,
                    true,
                    &job_id_2,
                    &w_id_2,
                    &name,
                    name_tl,
                    counter_arc,
                    total_to_install,
                    start,
                    db_2,
                )
                .await;
                #[cfg(all(feature = "enterprise", feature = "parquet"))]
                {
                    if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
                        tokio::spawn(build_tar_and_push(
                            os,
                            // push.replace("/", "_").clone(),
                            path_2,
                            "".into(),
                            Some(name_2),
                            // None,
                        ));
                    }
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    Ok(classpath)
}

async fn compile<'a>(
    JobHandlerInput {
        occupancy_metrics,
        mem_peak,
        canceled_by,
        worker_name,
        job,
        db,
        job_dir,
        shared_mount,
        client,
        envs,
        base_internal_url,
        inner_content,
        requirements_o,
        ..
    }: &mut JobHandlerInput<'a>,
    classpath: &'a str,
    // plugins: Vec<&'a str>,
) -> Result<(), Error> {
    let client = &client.get_authed().await;
    let reserved_variables = get_reserved_variables(job, &client.token, db).await?;
    let hash = compute_hash(inner_content, *requirements_o);
    let bin_path = format!("{}/{hash}", JAVA_CACHE_DIR);
    let remote_path = format!("java_jar/{hash}");
    let (cache, cache_logs) =
        windmill_common::worker::load_cache(&bin_path, &remote_path, true).await;

    let cache_logs = if cache {
        let target = format!("{job_dir}/target");
        // create_dir_all(&target).await?;
        // target += "/main.jar";

        #[cfg(unix)]
        let symlink = std::os::unix::fs::symlink(&bin_path, &target);
        #[cfg(windows)]
        let symlink = std::os::windows::fs::symlink_dir(&bin_path, &target);
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n\nPulled existing jar\n"),
            db.clone(),
        )
        .await;

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
                format!("\n\n--- COMPILING .CLASS FILES ---\n"),
                db.clone(),
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
            start_child_process(cmd, "javac").await?
        };
        handle_child::handle_child(
            &job.id,
            db,
            mem_peak,
            canceled_by,
            child,
            !*DISABLE_NSJAIL,
            worker_name,
            &job.workspace_id,
            "javac",
            job.timeout,
            false,
            &mut Some(occupancy_metrics),
        )
        .await?;

        match save_cache(
            &bin_path,
            &format!("java_jar/{hash}"),
            &format!("{job_dir}/target"),
            Some(hash),
        )
        .await
        {
            Err(e) => {
                let em = format!(
                    "could not save {bin_path} to {} to java cache: {e:?}",
                    format!("{job_dir}/main"),
                );
                tracing::error!(em);
                // Ok(em)
            }
            Ok(logs) => {}
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
        db,
        job_dir,
        shared_mount,
        client,
        envs,
        base_internal_url,
        ..
    }: &mut JobHandlerInput<'a>,
    classpath: &'a str,
) -> Result<(), Error> {
    let client = &client.get_authed().await;
    let reserved_variables = get_reserved_variables(job, &client.token, db).await?;

    let child = if !cfg!(windows) && !*DISABLE_NSJAIL {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n\n--- ISOLATED JAVA CODE EXECUTION ---\n"),
            db.clone(),
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
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .env_clear()
            .current_dir(job_dir)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(envs)
            .envs(reserved_variables)
            .envs(PROXY_ENVS.clone())
            // .env(
            //     "MAVEN_OPTS",
            //     "-Dmaven.repo.local=/tmp/windmill/cache/java/maven-jars",
            // )
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                JAVA_PATH.as_str(),
                "-classpath",
                &classpath,
                "net.script.App",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
    } else {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n\n--- JAVA CODE EXECUTION ---\n"),
            db.clone(),
        )
        .await;

        let mut cmd = Command::new(if cfg!(windows) {
            "java"
        } else {
            JAVA_PATH.as_str()
        });
        cmd.env_clear()
            .current_dir(job_dir.to_owned())
            // .env(
            //     "MAVEN_OPTS",
            //     "-Dmaven.repo.local=/tmp/windmill/cache/java/maven-jars",
            // )
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(envs)
            .envs(reserved_variables)
            .envs(PROXY_ENVS.clone())
            // .args(&[
            //     "-Dorg.slf4j.simpleLogger.defaultLogLevel=WARN",
            //     "compile",
            //     "exec:java",
            // ])
            .args(&["-classpath", &classpath, "net.script.App"])
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
        start_child_process(cmd, "java").await?
    };
    handle_child::handle_child(
        &job.id,
        db,
        mem_peak,
        canceled_by,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        &job.workspace_id,
        "java",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
    )
    .await
}
