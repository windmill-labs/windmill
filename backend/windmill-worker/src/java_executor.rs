use std::{collections::HashMap, process::Stdio};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use tokio::{
    fs::{create_dir_all, metadata, read_to_string, File},
    io::AsyncWriteExt,
    process::Command,
};
use uuid::Uuid;
use windmill_common::{
    ee::{get_license_plan, LicensePlan},
    error::Error,
    jobs::QueuedJob,
    s3_helpers::OBJECT_STORE_CACHE_SETTINGS,
    utils::calculate_hash,
    worker::{save_cache, write_file},
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
    get_common_bun_proc_envs,
    global_cache::{build_tar_and_push, pull_from_tar},
    handle_child, AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, JAVA_CACHE_DIR,
    NSJAIL_PATH, PATH_ENV, PROXY_ENVS,
};

pub const BUN_LOCK_SPLIT: &str = "\n//bun.lock\n";
pub const BUN_LOCKB_SPLIT: &str = "\n//bun.lockb\n";
pub const BUN_LOCK_SPLIT_WINDOWS: &str = "\r\n//bun.lock\r\n";
pub const BUN_LOCKB_SPLIT_WINDOWS: &str = "\r\n//bun.lockb\r\n";

pub const EMPTY_FILE: &str = "<empty>";

// /// Returns (package.json, bun.lock(b), is_empty, is_binary)
// fn split_lockfile(lockfile: &str) -> (&str, Option<&str>, bool, bool) {
//     if let Some(index) = lockfile.find(BUN_LOCK_SPLIT) {
//         // Split using "\n//bun.lock\n"
//         let (before, after_with_sep) = lockfile.split_at(index);
//         let after = &after_with_sep[BUN_LOCK_SPLIT.len()..];
//         (before, Some(after), after == EMPTY_FILE, false)
//     } else if let Some(index) = lockfile.find(BUN_LOCKB_SPLIT) {
//         // Split using "\n//bun.lockb\n"
//         let (before, after_with_sep) = lockfile.split_at(index);
//         let after = &after_with_sep[BUN_LOCKB_SPLIT.len()..];
//         (before, Some(after), after == EMPTY_FILE, true)
//     } else if let Some(index) = lockfile.find(BUN_LOCK_SPLIT_WINDOWS) {
//         // Split using "\r\n//bun.lock\r\n"
//         let (before, after_with_sep) = lockfile.split_at(index);
//         let after = &after_with_sep[BUN_LOCK_SPLIT_WINDOWS.len()..];
//         (before, Some(after), after == EMPTY_FILE, false)
//     } else if let Some(index) = lockfile.find(BUN_LOCKB_SPLIT_WINDOWS) {
//         // Split using "\r\n//bun.lockb\r\n"
//         let (before, after_with_sep) = lockfile.split_at(index);
//         let after = &after_with_sep[BUN_LOCKB_SPLIT_WINDOWS.len()..];
//         (before, Some(after), after == EMPTY_FILE, true)
//     } else {
//         (lockfile, None, false, false)
//     }
// }
// pub async fn gen_bun_lockfile(
//     mem_peak: &mut i32,
//     canceled_by: &mut Option<CanceledBy>,
//     job_id: &Uuid,
//     w_id: &str,
//     db: Option<&sqlx::Pool<sqlx::Postgres>>,
//     token: &str,
//     script_path: &str,
//     job_dir: &str,
//     base_internal_url: &str,
//     worker_name: &str,
//     export_pkg: bool,
//     raw_deps: Option<String>,
//     npm_mode: bool,
//     occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
// ) -> Result<Option<String>> {
//     let common_bun_proc_envs: HashMap<String, String> = get_common_bun_proc_envs(None).await;

//     let mut empty_deps = false;

//     if let Some(raw_deps) = raw_deps {
//         gen_bunfig(job_dir).await?;
//         write_file(job_dir, "package.json", raw_deps.as_str())?;
//     } else {
//         let _ = write_file(
//             &job_dir,
//             "build.js",
//             &format!(
//                 r#"
// {}

// {RELATIVE_BUN_BUILDER}
// "#,
//                 RELATIVE_BUN_LOADER
//                     .replace("W_ID", w_id)
//                     .replace("BASE_INTERNAL_URL", base_internal_url)
//                     .replace("TOKEN", token)
//                     .replace(
//                         "CURRENT_PATH",
//                         &crate::common::use_flow_root_path(script_path)
//                     )
//                     .replace("RAW_GET_ENDPOINT", "raw")
//             ),
//         )?;

//         gen_bunfig(job_dir).await?;

//         let mut child_cmd = Command::new(&*BUN_PATH);
//         child_cmd
//             .current_dir(job_dir)
//             .env_clear()
//             .envs(common_bun_proc_envs.clone())
//             .args(vec!["run", "build.js"])
//             .stdout(Stdio::piped())
//             .stderr(Stdio::piped());

//         #[cfg(windows)]
//         child_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

//         let mut child_process = start_child_process(child_cmd, &*BUN_PATH).await?;

//         if let Some(db) = db {
//             handle_child(
//                 job_id,
//                 db,
//                 mem_peak,
//                 canceled_by,
//                 child_process,
//                 false,
//                 worker_name,
//                 w_id,
//                 "bun build",
//                 None,
//                 false,
//                 occupancy_metrics,
//             )
//             .await?;
//         } else {
//             child_process.wait().await?;
//         }

//         let new_package_json = read_file_content(&format!("{job_dir}/package.json")).await?;
//         empty_deps = new_package_json
//             == r#"{
//   "dependencies": {}
// }"#;
//     }

//     if !empty_deps {
//         install_bun_lockfile(
//             mem_peak,
//             canceled_by,
//             job_id,
//             w_id,
//             db,
//             job_dir,
//             worker_name,
//             common_bun_proc_envs,
//             npm_mode,
//             occupancy_metrics,
//         )
//         .await?;
//     } else {
//         if let Some(db) = db {
//             append_logs(job_id, w_id, "\nempty dependencies, skipping install", db).await;
//         }
//     }

//     if export_pkg {
//         let mut content = "".to_string();
//         {
//             let mut file = File::open(format!("{job_dir}/package.json")).await?;
//             file.read_to_string(&mut content).await?;
//         }
//         if !npm_mode {
//             #[cfg(any(target_os = "linux", target_os = "macos"))]
//             content.push_str(BUN_LOCK_SPLIT);

//             #[cfg(target_os = "windows")]
//             content.push_str(BUN_LOCK_SPLIT_WINDOWS);

//             {
//                 let file = format!("{job_dir}/bun.lock");
//                 if !empty_deps && tokio::fs::metadata(&file).await.is_ok() {
//                     let mut file = File::open(&file).await?;
//                     let mut buf = String::default();
//                     file.read_to_string(&mut buf).await?;
//                     content.push_str(&buf);
//                 } else {
//                     content.push_str(&EMPTY_FILE);
//                 }
//             }
//         }
//         Ok(Some(content))
//     } else {
//         Ok(None)
//     }
// }

// TODO: jared

const NSJAIL_CONFIG_RUN_NU_CONTENT: &str = include_str!("../nsjail/run.nu.config.proto");
const POM_XML_TEMPLATE: &str = r#"
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0
                             http://maven.apache.org/maven-v4_0_0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>net.script</groupId>
    <artifactId>windmill-script</artifactId>
    <packaging>jar</packaging>
    <version>1.1.0</version>
    <name>Windmill Script</name>
    <url>https://repo.maven.apache.org/maven2</url>
    <build>
        <finalName>main</finalName> <!-- Set your desired JAR name here -->
        <plugins>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-jar-plugin</artifactId>
                <version>3.2.0</version> <!-- Use the latest version -->
                <configuration>
                    <archive>
                        <manifest>
                            <addClasspath>true</addClasspath>
                            <mainClass>net.script.App</mainClass> <!-- Replace with your main class -->
                        </manifest>
                    </archive>
                </configuration>
            </plugin>
            <plugin>
              <groupId>org.codehaus.mojo</groupId>
              <artifactId>exec-maven-plugin</artifactId>
              <version>3.2.0</version>
              <configuration>
                  <mainClass>net.script.App</mainClass>
                  <arguments>
                  </arguments>
              </configuration>
            </plugin>
        </plugins>
    </build>
    <dependencies>
      <dependency>
          <groupId>com.fasterxml.jackson.core</groupId>
          <artifactId>jackson-databind</artifactId>
          <version>2.9.8</version>
      </dependency>
      SPREAD_DEPENDENCIES
    </dependencies>
</project>
"#;

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
        .find_position(|x| x.starts_with("//requirements:") || x.starts_with("// requirements:"));
    let deps = if let Some((pos, _)) = find_requirements {
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
        .replace("SPREAD_DEPENDENCIES", &format!("{deps}"));

    let req_hash = format!("java-{}", calculate_hash(&pom));

    File::create(format!("{}/pom.xml", job_dir))
        .await?
        .write_all(&pom.into_bytes())
        .await?;

    // TODO: Use not pip_resolution_cache?
    if let Some(cached) = sqlx::query_scalar!(
        "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
        // Python version is included in hash,
        // hash will be the different for every python version
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
            "nu"
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
            .args(&[
                "dependency:collect",
                "-DoutputFile=dependencies.txt",
                // "-DincludeScope=runtime",
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
lazy_static::lazy_static! {
    static ref JAVA_PATH: String = std::env::var("JAVA_PATH").unwrap_or_else(|_| "/usr/bin/nu".to_string());
    static ref MAVEN_PATH: String = std::env::var("MAVEN_PATH").unwrap_or_else(|_| "/usr/bin/nu".to_string());
}

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

// #[derive(Deserialize, Serialize, Default)]
// #[serde(rename = "dependency")]
// struct POMDependency {
//     #[serde(rename = "groupId")]
//     group_id: String,
//     #[serde(rename = "artifactId")]
//     artifact_id: String,
//     version: String,
//     #[serde(rename = "type")]
//     ty: String,
//     scope: String,
//     optional: bool,
// }

pub async fn handle_java_job<'a>(mut args: JobHandlerInput<'a>) -> Result<Box<RawValue>, Error> {
    let Annotations { jared, no_s3, no_lock_cache } = Annotations::parse(&args.inner_content);
    // --- Handle imports ---
    // --- Wrap and write to fs ---

    // let xml_deps =
    //     quick_xml::se::to_string(&toml::de::from_str::<POMDependency>(&toml_deps).unwrap())
    //         .unwrap_or_default();
    let mut not_installed = vec![];
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
            .write_all(&format!("package net.script;\n {}", args.inner_content).into_bytes())
            .await?;
    }

    let classpath = if !no_s3 {
        let deps = resolve_dependencies(
            &args.job.id,
            &args.inner_content,
            &mut args.mem_peak,
            &mut &mut args.canceled_by,
            &args.job_dir,
            &args.db,
            &args.worker_name,
            &args.job.workspace_id,
            &mut args.occupancy_metrics,
        )
        .await?
        .lines()
        .map(|l| {
            let l = l.replace(":jar", "");
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
            + ":target/main.jar";

        let mut to_install = vec![];
        for (dep, name) in &deps {
            if metadata(dep).await.is_ok() {
                dbg!("There", dep);
                // panic!("");
                // req_paths.push(venv_p);
                // in_cache.push(req.to_string());
            } else {
                dbg!("Not there", dep);
                to_install.push((dep.clone(), name.clone()));
                // There is no valid or no wheel at all. Regardless of if there is content or not, we will overwrite it with --reinstall flag
                // req_with_penv.push((req.to_string(), venv_p));
            }
        }
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        let is_not_pro = !matches!(get_license_plan().await, LicensePlan::Pro);

        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if is_not_pro {
            for (dep, name) in to_install {
                if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
                    tokio::select! {
                        pull = pull_from_tar(os, dep.clone(), "".into(), Some(name.clone())) => {
                            if let Err(e) = pull {
                                not_installed.push((dep, name));
                            } else {
                                append_logs(
                                    &args.job.id,
                                    &args.job.workspace_id,
                                    format!("\n{dep} pulled from s3"),
                                    args.db.clone(),
                                )
                                .await;
                            }
                        }
                    }
                }
            }
        }
        dbg!(classpath)
    } else {
        "".to_owned()
    };
    jar(&mut args).await?;
    run(&mut args, classpath).await?;
    #[cfg(all(feature = "enterprise", feature = "parquet", unix))]
    {
        for (dep, name) in not_installed {
            if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
                tokio::spawn(build_tar_and_push(
                    os,
                    // push.replace("/", "_").clone(),
                    dep.clone(),
                    "".into(),
                    Some(name.clone()),
                    // None,
                ));
            }
        }
    }
    // --- Retrieve results ---
    {
        read_result(&args.job_dir).await
    }
}

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
    classpath: String,
) -> Result<(), Error> {
    let client = &client.get_authed().await;
    let reserved_variables = get_reserved_variables(job, &client.token, db).await?;

    let child = {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n\n--- JAVA CODE EXECUTION ---\n"),
            db.clone(),
        )
        .await;

        let mut cmd = Command::new(if cfg!(windows) {
            "nu"
        } else {
            JAVA_PATH.as_str()
        });
        cmd
            // .env_clear()
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
        start_child_process(cmd, "mvn").await?
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
        "mvn",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
    )
    .await
}

async fn jar<'a>(
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
    // plugins: Vec<&'a str>,
) -> Result<(), Error> {
    let client = &client.get_authed().await;
    let reserved_variables = get_reserved_variables(job, &client.token, db).await?;
    let hash = compute_hash(inner_content, *requirements_o);
    let bin_path = format!("{}/{hash}", JAVA_CACHE_DIR);
    let remote_path = format!("java_jar/{hash}");
    let (cache, cache_logs) = windmill_common::worker::load_cache(&bin_path, &remote_path).await;

    let cache_logs = if cache {
        let mut target = format!("{job_dir}/target");
        create_dir_all(&target).await?;
        target += "/main.jar";

        #[cfg(unix)]
        let symlink = std::os::unix::fs::symlink(&bin_path, &target);
        #[cfg(windows)]
        let symlink = std::os::windows::fs::symlink_dir(&bin_path, &target);

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
                format!("\n\n--- JARING JAVA ---\n"),
                db.clone(),
            )
            .await;

            // File::create(&plugin_registry).await?;
            //
            let mut cmd = Command::new(if cfg!(windows) {
                "nu"
            } else {
                MAVEN_PATH.as_str()
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
                .args(&["package"])
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
            start_child_process(cmd, "nu").await?
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
            "nu",
            job.timeout,
            false,
            &mut Some(occupancy_metrics),
        )
        .await?;

        //     tokio::fs::copy(
        //     &format!("{job_dir}/target/main.jar"),
        //     format! {"{job_dir}/script.jar"},
        // )
        // .await
        // .map_err(|e| {
        //     Error::ExecutionErr(format!(
        //         "could not copy built binary from [...]/target/script.java to {job_dir}/script.java : {e:?}"
        //     ))
        // })?;

        match save_cache(
            &bin_path,
            &format!("java_jar/{hash}"),
            &format!("{job_dir}/target/main.jar"),
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
// #[cfg(test)]
// mod test {
//     use super::parse_plugin_use;

//     #[test]
//     fn test_nu_plugin_use() {
//         let content = r#"
// plugin use foo
// plugin use bar
// plugin use baz
// plugin use meh
//         "#;
//         assert_eq!(
//             vec!["foo", "bar", "baz", "meh"], //
//             parse_plugin_use(content)
//         );
//     }
// }
