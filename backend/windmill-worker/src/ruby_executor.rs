use std::{collections::HashMap, process::Stdio, sync::Arc};

use async_recursion::async_recursion;
use itertools::Itertools;
use tokio::{
    fs::{remove_dir_all, File},
    io::AsyncWriteExt,
    process::Command,
};
use uuid::Uuid;
use windmill_common::{
    cache::Storage,
    error::{self, Error},
    utils::calculate_hash,
};
use windmill_parser::Arg;
use windmill_parser_ruby::parse_ruby_signature;
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::{
    common::{
        create_args_and_out_file, par_install_language_dependencies, read_result,
        start_child_process, OccupancyMetrics, RequiredDependency,
    },
    handle_child, AuthedClient, DISABLE_NSJAIL, PATH_ENV, RUBY_CACHE_DIR,
};
lazy_static::lazy_static! {
    static ref RUBY_CONCURRENT_DOWNLOADS: usize = std::env::var("RUBY_CONCURRENT_DOWNLOADS").ok().map(|flag| flag.parse().unwrap_or(20)).unwrap_or(20);
    static ref RUBY_PATH: String = std::env::var("RUBY_PATH").unwrap_or_else(|_| "/usr/bin/ruby".to_string());
    static ref BUNDLE_PATH: String = std::env::var("RUBY_BUNDLE_PATH").unwrap_or_else(|_| "/usr/bin/bundle".to_string());

    // static ref STOREPASS: String = std::env::var("JAVA_STOREPASS").unwrap_or("123456".into());
    // static ref TRUST_STORE_PATH: String = std::env::var("JAVA_TRUST_STORE_PATH").unwrap_or("/usr/local/share/ca-certificates/truststore.jks".into());
}

#[allow(dead_code)]
pub(crate) struct JobHandlerInput<'a> {
    pub base_internal_url: &'a str,
    pub canceled_by: &'a mut Option<CanceledBy>,
    pub client: &'a AuthedClient,
    pub parent_runnable_path: Option<String>,
    pub db: &'a sqlx::Pool<sqlx::Postgres>,
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

pub async fn handle_ruby_job<'a>(
    mut args: JobHandlerInput<'a>,
) -> Result<Box<sqlx::types::JsonRawValue>, Error> {
    {
        create_args_and_out_file(&args.client, args.job, args.job_dir, args.db).await?;
        File::create(format!("{}/main.rb", args.job_dir))
            .await?
            .write_all(&wrap(args.inner_content)?.into_bytes())
            .await?;
    }
    let lockfile = resolve(
        &args.job.id,
        &args.inner_content,
        &args.job_dir,
        &args.db,
        &args.job.workspace_id,
    )
    .await?;

    install(&mut args, lockfile).await?;
    run(&mut args).await?;
    // --- Prepare ---
    // todo!()
    // --- Retrieve results ---
    {
        read_result(&args.job_dir).await
    }
    // Err(anyhow::anyhow!("todo").into())
}
pub async fn resolve<'a>(
    job_id: &Uuid,
    code: &str,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
) -> Result<String, Error> {
    let deps = "# frozen_string_literal: true
source \"https://rubygems.org\"\n"
        .to_owned()
        + windmill_parser_ruby::parse_ruby_requirements(code)?
            .into_iter()
            .map(|im| format!("gem {im}"))
            .join("\n")
            .as_str();

    let mut file = File::create(job_dir.to_owned() + "/Gemfile").await?;
    file.write_all(&deps.as_bytes()).await?;

    let req_hash = format!("ruby-{}", calculate_hash(&deps));
    if let Some(cached) = sqlx::query_scalar!(
        "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
        req_hash
    )
    .fetch_optional(db)
    .await?
    {
        return Ok(cached);
    }
    let lock = {
        append_logs(
            job_id,
            w_id,
            format!("\n--- RESOLVING LOCKFILE ---\n"),
            db.clone(),
        )
        .await;

        let mut cmd = Command::new(if cfg!(windows) {
            "bundle"
        } else {
            BUNDLE_PATH.as_str()
        });
        cmd.env_clear()
            .current_dir(job_dir.to_owned())
            .env("PATH", PATH_ENV.as_str())
            // .envs(PROXY_ENVS.clone())
        ;

        // // Configure proxies
        // {
        //     let jps = parse_proxy()?;
        //     if let Some(val) = jps.https_host {
        //         cmd.arg(&format!("-Dhttps.proxyHost={}", val));
        //     }
        //     if let Some(val) = jps.https_port {
        //         cmd.arg(&format!("-Dhttps.proxyPort={}", val));
        //     }
        //     if let Some(val) = jps.http_host {
        //         cmd.arg(&format!("-Dhttp.proxyHost={}", val));
        //     }
        //     if let Some(val) = jps.http_port {
        //         cmd.arg(&format!("-Dhttp.proxyPort={}", val));
        //     }
        //     if let Some(val) = jps.no_proxy {
        //         cmd.arg(&format!("-Dhttp.nonProxyHosts=\"{}\"", val));
        //     }
        // }
        // if metadata(TRUST_STORE_PATH.clone()).await.is_ok() {
        //     cmd.args(&[
        //         &format!("-Djavax.net.ssl.trustStore={}", *TRUST_STORE_PATH),
        //         &format!("-Djavax.net.ssl.trustStorePassword={}", *STOREPASS),
        //     ]);
        // }
        cmd.args(&["lock", "--print"])
            // .args(&get_repos().await)
            // .args(&deps.split("\n").collect_vec())
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

    sqlx::query!(
        "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('3 days')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = $2",
        req_hash,
        lock.clone(),
    ).fetch_optional(db).await?;

    append_logs(job_id, w_id, format!("\n{}", &lock), db.clone()).await;
    Ok(lock)
}

async fn install<'a>(
    JobHandlerInput { worker_name, job, db, job_dir, .. }: &mut JobHandlerInput<'a>,
    lockfile: String,
) -> Result<String, Error> {
    // lazy_static::lazy_static! {
    //     static ref LOCKED_REQ: regex::Regex = regex::Regex::new(r"\b(\w*)\b\s\((.*)\)").unwrap();
    // }
    //
    let mut file = File::create(job_dir.to_owned() + "/Gemfile.lock").await?;
    file.write_all(&lockfile.as_bytes()).await?;

    let mut deps = vec![];
    if let Some((mut pos, _)) = lockfile.lines().find_position(|x| x.starts_with("GEM")) {
        let prefix = lockfile
            .lines()
            .enumerate()
            .skip(pos)
            .map_while(|(i, mut x)| {
                pos = i;
                if x == "  specs:" {
                    None
                } else {
                    if x.len() > 1 && x.ends_with('/') {
                        x = &x[0..x.len() - 1];
                    }
                    Some(
                        x.replace(" ", "")
                            .replace("https://", "")
                            .replace(":", "=")
                            .replace("/", "_"),
                    )
                }
            })
            .collect_vec()
            .join("@");

        for x in lockfile.lines().skip(pos + 1).take_while(|x| !x.is_empty()) {
            // Gemfile.lock:
            //
            // GEM
            //   specs:
            // ____activesupport (8.0.2) - Yes
            // 1234__base64              - Ignore
            // 123456
            //
            // Entries we actually want to include do not start with 6 whitespaces
            if !x.starts_with(" ".repeat(6).as_str()) {
                // Final format:
                // GIT@remote=github.com-fazibear-colorize@revision=4498bb697dbfab7265b09d121c2ef5fc8d9c4c45@tag=v1.0.0@colorize=1.0.0
                // GEM@remote=rubygems.org@activesupport=8.0.2
                let path = format!(
                    "{RUBY_CACHE_DIR}/gems/{}@{}",
                    prefix.clone(),
                    x.trim_start().replace(" (", "=").replace(')', "").as_str()
                );
                deps.push(RequiredDependency {
                    path,
                    custom_name: None,
                    short_name: Some(x.into()),
                });
            }
        }
    }

    let job_dir = job_dir.to_owned();
    let tmp_install_dir = format!("{RUBY_CACHE_DIR}/tmp-install-{}", Uuid::new_v4());
    // let fetch_dir2 = fetch_dir.clone();
    par_install_language_dependencies(
        deps,
        "bundle (ruby)",
        "bundle (ruby)",
        false,
        *RUBY_CONCURRENT_DOWNLOADS,
        false,
        crate::common::InstallStrategy::AllAtOnce(Arc::new(move |dependencies| {
            let mut cmd = Command::new(if cfg!(windows) {
                "bundle"
            } else {
                BUNDLE_PATH.as_str()
            });
            // let artifacts = dependencies
            //     .into_iter()
            //     .map(|e| {
            //         e.custom_name.ok_or(anyhow::anyhow!(
            //             "Internal Error: Artifact name should be Some!"
            //         ))
            //     })
            //     .collect::<anyhow::Result<Vec<String>>>()?;
            cmd.env_clear()
                .current_dir(&job_dir)
                .env("PATH", PATH_ENV.as_str())
                // .envs(PROXY_ENVS.clone())
            ;

            cmd.args(&["install", "--path", tmp_install_dir.as_str(), "--no-cache"])
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
        })),
        async move |_| {
            move_to_repository(&fetch_dir2, 0).await?;
            // remove_dir_all(&fetch_dir2).await?;
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
        db,
    )
    .await?;
    // todo!()
    Ok("".into())
    // Ok(classpath)
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
        parent_runnable_path,
        ..
    }: &mut JobHandlerInput<'a>,
) -> Result<(), Error> {
    // let reserved_variables =
    //     get_reserved_variables(job, &client.token, db, parent_runnable_path.clone()).await?;

    // let child = if !cfg!(windows) && !*DISABLE_NSJAIL {
    //     // append_logs(
    //     //     &job.id,
    //     //     &job.workspace_id,
    //     //     format!("\n--- ISOLATED JAVA CODE EXECUTION ---\n"),
    //     //     db.clone(),
    //     // )
    //     // .await;

    //     write_file(
    //         job_dir,
    //         "run.config.proto",
    //         &NSJAIL_CONFIG_RUN_JAVA_CONTENT
    //             .replace("{JOB_DIR}", job_dir)
    //             .replace("{CACHE_DIR}", JAVA_CACHE_DIR)
    //             .replace("{SHARED_MOUNT}", &shared_mount)
    //             // .replace("{CACHED_TARGET}", &shared_mount)
    //             .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string()),
    //     )?;
    //     let mut cmd = Command::new(NSJAIL_PATH.as_str());
    //     cmd.env_clear()
    //         .current_dir(job_dir)
    //         .env("PATH", PATH_ENV.as_str())
    //         .env("BASE_INTERNAL_URL", base_internal_url)
    //         .envs(envs)
    //         .envs(reserved_variables)
    //         .envs(PROXY_ENVS.clone())
    //         .args(vec![
    //             "--config",
    //             "run.config.proto",
    //             "--",
    //             JAVA_PATH.as_str(),
    //         ]);
    //     if metadata(TRUST_STORE_PATH.clone()).await.is_ok() {
    //         cmd.args(&[
    //             &format!("-Djavax.net.ssl.trustStore={}", *TRUST_STORE_PATH),
    //             &format!("-Djavax.net.ssl.trustStorePassword={}", *STOREPASS),
    //         ]);
    //     }
    //     // Configure proxies
    //     {
    //         let jps = parse_proxy()?;
    //         if let Some(val) = jps.https_host {
    //             cmd.arg(&format!("-Dhttps.proxyHost={}", val));
    //         }
    //         if let Some(val) = jps.https_port {
    //             cmd.arg(&format!("-Dhttps.proxyPort={}", val));
    //         }
    //         if let Some(val) = jps.http_host {
    //             cmd.arg(&format!("-Dhttp.proxyHost={}", val));
    //         }
    //         if let Some(val) = jps.http_port {
    //             cmd.arg(&format!("-Dhttp.proxyPort={}", val));
    //         }
    //         if let Some(val) = jps.no_proxy {
    //             cmd.arg(&format!("-Dhttp.nonProxyHosts=\"{}\"", val));
    //         }
    //     }
    //     cmd.args(vec!["-classpath", &classpath, "net.script.App"]);
    //     cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    //     start_child_process(cmd, NSJAIL_PATH.as_str()).await?
    // } else {
    append_logs(
        &job.id,
        &job.workspace_id,
        format!("\n--- JAVA CODE EXECUTION ---\n"),
        db.clone(),
    )
    .await;

    let mut cmd = Command::new(if cfg!(windows) {
        "ruby"
    } else {
        RUBY_PATH.as_str()
    });
    cmd.env_clear()
        .current_dir(job_dir.to_owned())
        .env("PATH", PATH_ENV.as_str())
        .env("BASE_INTERNAL_URL", base_internal_url)
        .envs(envs);
    // .envs(reserved_variables);
    // if metadata(TRUST_STORE_PATH.clone()).await.is_ok() {
    //     cmd.args(&[
    //         &format!("-Djavax.net.ssl.trustStore={}", *TRUST_STORE_PATH),
    //         &format!("-Djavax.net.ssl.trustStorePassword={}", *STOREPASS),
    //     ]);
    // }
    // Configure proxies
    // {
    //     let jps = parse_proxy()?;
    //     if let Some(val) = jps.https_host {
    //         cmd.arg(&format!("-Dhttps.proxyHost={}", val));
    //     }
    //     if let Some(val) = jps.https_port {
    //         cmd.arg(&format!("-Dhttps.proxyPort={}", val));
    //     }
    //     if let Some(val) = jps.http_host {
    //         cmd.arg(&format!("-Dhttp.proxyHost={}", val));
    //     }
    //     if let Some(val) = jps.http_port {
    //         cmd.arg(&format!("-Dhttp.proxyPort={}", val));
    //     }
    //     if let Some(val) = jps.no_proxy {
    //         cmd.arg(&format!("-Dhttp.nonProxyHosts=\"{}\"", val));
    //     }
    // }
    cmd.args(&["main.rb"])
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
    let child = start_child_process(cmd, "java").await?;
    // };
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
        None,
    )
    .await
}
fn wrap(inner_content: &str) -> Result<String, Error> {
    let sig = parse_ruby_signature(inner_content)?;
    let spread = sig
        .args
        .clone()
        .into_iter()
        .map(|Arg { name, .. }| format!("a[\"{}\"]", name))
        .collect_vec()
        .join(",");
    Ok(
        r#"    
INNER_CONTENT

require 'json'
a = JSON.parse(File.read("args.json"))

res = main(SPREAD)

File.open("result.json", "w") do |file|
  file.write(JSON.generate(res))
end
            "#
        .replace("INNER_CONTENT", inner_content)
        .replace("SPREAD", &spread), // .replace("TRANSFORM", transform)
    )
}
