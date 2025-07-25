use std::{collections::HashMap, fs::read_to_string, process::Stdio, str::FromStr, sync::Arc};

use anyhow::{anyhow, bail};
use async_recursion::async_recursion;
use itertools::Itertools;
use regex::Regex;
use tokio::{
    fs::{self, remove_dir_all, File},
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use uuid::Uuid;
use windmill_common::{
    cache::Storage,
    client::AuthedClient,
    error::{self, Error},
    utils::calculate_hash,
    worker::Connection,
};
use windmill_parser::Arg;
use windmill_parser_ruby::parse_ruby_signature;
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::{
    common::{
        create_args_and_out_file, par_install_language_dependencies, read_result,
        start_child_process, OccupancyMetrics, RequiredDependency,
    },
    handle_child::{self, handle_child},
    DISABLE_NSJAIL, PATH_ENV, PROXY_ENVS, RUBY_CACHE_DIR,
};
lazy_static::lazy_static! {
    static ref RUBY_CONCURRENT_DOWNLOADS: usize = std::env::var("RUBY_CONCURRENT_DOWNLOADS").ok().map(|flag| flag.parse().unwrap_or(20)).unwrap_or(20);
    static ref RUBY_PATH: String = std::env::var("RUBY_PATH").unwrap_or_else(|_| "/usr/bin/ruby".to_string());
    static ref BUNDLE_PATH: String = std::env::var("RUBY_BUNDLE_PATH").unwrap_or_else(|_| "/usr/bin/bundle".to_string());
    static ref GEM_PATH: String = std::env::var("RUBY_GEM_PATH").unwrap_or_else(|_| "/usr/bin/gem".to_string());
}

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

pub async fn handle_ruby_job<'a>(
    mut args: JobHandlerInput<'a>,
) -> Result<Box<sqlx::types::JsonRawValue>, Error> {
    // --- Prepare ---
    {
        prepare(&args).await?;
    }
    // --- Gen Lockfile ---

    let lockfile = resolve(&mut args).await?;

    // --- Install ---
    let rubylib = install(&mut args, lockfile).await?;
    // --- Execute ---
    {
        run(&mut args, rubylib).await?;
    }
    // --- Retrieve results ---
    {
        read_result(&args.job_dir).await
    }
    // Err(anyhow::anyhow!("todo").into())
}
pub async fn prepare<'a>(
    JobHandlerInput {
        job,
        conn,
        job_dir,
        inner_content,
        client,
        //
        ..
    }: &JobHandlerInput<'a>,
) -> Result<(), Error> {
    create_args_and_out_file(&client, job, job_dir, conn).await?;
    File::create(format!("{}/main.rb", job_dir))
        .await?
        .write_all(&wrap(inner_content)?.into_bytes())
        .await?;

    fs::create_dir_all(format!("{RUBY_CACHE_DIR}/gems/windmill-internal/windmill/")).await?;
    File::create(format!("{RUBY_CACHE_DIR}/gems/windmill-internal/windmill/inline.rb"))
            .await?
            .write_all(&wrap(
        r#"    
def source(*a, **rest)
end
def group(*a, **rest)
end
def ruby(*a, **rest)
  warn "Custom Ruby runtime specifications (engine, engine_version, patchlevel) are not supported in this environment. Using system ruby"
end
def gem(name, version = nil, require: nil, **rest)
  case require
  when false
    # Skip requiring entirely
  when nil, true
    Kernel.require(name)
  else # String or other truthy value
    Kernel.require(require.to_s)
  end
end
def gemfile(&block)
  instance_eval(&block) if block_given?
end
def main
end
"#
            )?.into_bytes())
            .await?;
    Ok(())
}
pub async fn resolve<'a>(
    JobHandlerInput {
        mem_peak,
        canceled_by,
        worker_name,
        job,
        conn,
        job_dir,
        inner_content,
        //
        ..
    }: &mut JobHandlerInput<'a>,
) -> Result<String, Error> {
    lazy_static::lazy_static! {
        static ref BUNDLER_RE: Regex = Regex::new(r"^\s*require\s+'bundler/inline'").unwrap();
    }

    if BUNDLER_RE.find(&inner_content).is_some() {
        return Err(anyhow!(
            "Detected `require 'bundler/inline'` - please replace with `require 'windmill/inline'`.
Your Gemfile syntax will continue to work as-is."
        )
        .into());
    }

    let gemfile = windmill_parser_ruby::parse_ruby_requirements(&inner_content)?;

    let mut file = File::create(job_dir.to_owned() + "/Gemfile").await?;
    file.write_all(&gemfile.as_bytes()).await?;

    let req_hash = format!("ruby-{}", calculate_hash(&gemfile));
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
    let lock = {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n--- RESOLVING LOCKFILE ---\n"),
            conn,
        )
        .await;

        let mut cmd = Command::new(if cfg!(windows) {
            "bundle"
        } else {
            BUNDLE_PATH.as_str()
        });
        cmd
            // .env_clear()
            .current_dir(job_dir.to_owned())
            .envs(vec![
                ("PATH".to_owned(), PATH_ENV.clone()),
                // Make sure there is nothing written to actual home
                // This way we keep everything clean, organized and maintable
                ("HOME".to_owned(), RUBY_CACHE_DIR.to_owned()),
            ])
            .envs(PROXY_ENVS.clone());

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
        cmd.args(&["lock", "--retry", "2"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // .args(&get_repos().await)
        // .args(&deps.split("\n").collect_vec())
        // .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            cmd.env("SystemRoot", crate::SYSTEM_ROOT.as_str())
                .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                );
        }
        let child = start_child_process(cmd, "bundle").await?;

        // let mut stdout = String::new();

        handle_child::handle_child(
            &job.id,
            conn,
            mem_peak,
            canceled_by,
            child,
            !*DISABLE_NSJAIL,
            worker_name,
            &job.workspace_id,
            "bundle",
            None,
            false,
            &mut None,
            // Some(&mut stdout),
            None,
        )
        .await?;

        // stdout
        let path_lock = format!("{job_dir}/Gemfile.lock");
        let mut file = File::open(path_lock).await?;
        let mut req_content = "".to_string();
        file.read_to_string(&mut req_content).await?;
        req_content
    };

    if let Some(db) = conn.as_sql() {
        sqlx::query!(
            "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('3 days')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = $2",
            req_hash,
            lock.clone(),
        ).fetch_optional(db).await?;
    }

    append_logs(&job.id, &job.workspace_id, format!("\n{}", &lock), conn).await;
    Ok(lock)
}

async fn install<'a>(
    JobHandlerInput { worker_name, job, conn, job_dir, .. }: &mut JobHandlerInput<'a>,
    lockfile: String,
) -> Result<String, Error> {
    #[derive(Debug, Clone)]
    enum CurrentSource {
        GIT { remote: Option<String>, revision: Option<String> },
        GEM { remote: Option<String> },
        INIT,
    }

    #[derive(Clone, Debug)]
    struct CustomPayload {
        source: CurrentSource,
        version: String,
        pkg: String,
    }

    let mut deps = vec![];
    let mut current_source = CurrentSource::INIT;
    'lock_lines: for (i, line) in lockfile.lines().enumerate() {
        // Trimed line
        let tl = line.trim_start();
        if tl.starts_with("GIT") {
            current_source = CurrentSource::GIT { remote: None, revision: None };
            continue 'lock_lines;
        } else if tl.starts_with("GEM") {
            current_source = CurrentSource::GEM { remote: None };
            continue 'lock_lines;
        }

        const REMOTE: &str = "remote:";
        if line.contains(REMOTE) {
            match &mut current_source {
                CurrentSource::GIT { remote, .. } => remote.replace(tl.replace(REMOTE, "")),
                CurrentSource::GEM { remote } => remote.replace(tl.replace(REMOTE, "")),
                // TODO: Add more descriptive error
                CurrentSource::INIT => return Err(anyhow!("Malformed Gemfile.lock").into()),
            };
            continue 'lock_lines;
        }

        const REVISION: &str = "revision:";
        if line.contains(REVISION) {
            if let CurrentSource::GIT { revision, .. } = &mut current_source {
                revision.replace(tl.replace(REVISION, ""));
            }
            continue 'lock_lines;
        }

        // Check on untrimed line
        if !line.starts_with("    ") || line.starts_with("      ") {
            //               "1234"                      "123456"
            // For example:
            //
            // GIT
            //  remote: https://github.com/rails/rails.git
            //  revision: 8cb4f8ceffe375f64052579cd7a580bfdaf36d61
            //  specs:
            //    actioncable (8.1.0.alpha)
            //      actionpack (= 8.1.0.alpha)
            //      activesupport (= 8.1.0.alpha)
            //      nio4r (~> 2.0)
            //      websocket-driver (>= 0.6.1)
            //      zeitwerk (~> 2.6)
            //    actionmailbox (8.1.0.alpha)
            //      actionpack (= 8.1.0.alpha)
            //      activejob (= 8.1.0.alpha)
            //      activerecord (= 8.1.0.alpha)
            //      activestorage (= 8.1.0.alpha)
            //      activesupport (= 8.1.0.alpha)
            //      mail (>= 2.8.0)
            //    actionmailer (8.1.0.alpha)
            //   ...
            //
            // In this snippet we only want actioncable, actionmailbox and actionmailer
            //
            // If it got the hit than that's not it, just skip
            continue 'lock_lines;
        }

        // TODO: Test for safety
        let buf = tl.replace(['(', ')'], "");
        let &[pkg, version, ..] = buf.split_whitespace().collect_vec().as_slice() else {
            todo!();
        };

        let custom_payload = CustomPayload {
            source: current_source.clone(),
            version: version.into(),
            pkg: pkg.into(),
        };

        // Calculate hash based on source
        // Obfuscation is essential because remote can include credentials
        let mut hash = calculate_hash(&format!("{:?}", custom_payload.clone()));

        // Use 32 characters which is 256 bits
        hash.truncate(32);

        // Final format:
        // 123...zx-activesupport-8.0.2
        // ^^^^^^^^ hash based on source and type (GEM or GIT)
        let handle = format!("{}-{}-{}", hash, pkg, version);
        let path = format!("{RUBY_CACHE_DIR}/gems/{}", &handle);

        deps.push(dbg!(RequiredDependency {
            path,
            s3_handle: Some(handle),
            short_name: Some(tl.to_owned()),
            custom_payload,
        }));
    }

    let rubylib = deps
        .iter()
        .map(|rd| {
            format!(
                "{}/gems/{}-{}/lib",
                rd.path, rd.custom_payload.pkg, rd.custom_payload.version
            )
        })
        .join(":")
        + format!(":{RUBY_CACHE_DIR}/gems/windmill-internal").as_str();
    // if let Some((mut pos, _)) = lockfile.lines().find_position(|x| x.starts_with("GEM")) {
    //     let prefix = lockfile
    //         .lines()
    //         .enumerate()
    //         .skip(pos)
    //         .map_while(|(i, mut x)| {
    //             pos = i;
    //             if x == "  specs:" {
    //                 None
    //             } else {
    //                 if x.len() > 1 && x.ends_with('/') {
    //                     x = &x[0..x.len() - 1];
    //                 }
    //                 Some(
    //                     x.replace(" ", "")
    //                         .replace("https://", "")
    //                         .replace(":", "=")
    //                         .replace("/", "_"),
    //                 )
    //             }
    //         })
    //         .collect_vec()
    //         .join("@");

    //     // dbg!(&prefix);
    //     for x in lockfile.lines().skip(pos + 1).take_while(|x| !x.is_empty()) {
    //         // Gemfile.lock:
    //         //
    //         // GEM
    //         //   specs:
    //         // ____activesupport (8.0.2) - Yes
    //         // 1234__base64              - Ignore
    //         // 123456
    //         //
    //         // Entries we actually want to include do not start with 6 whitespaces
    //         if !x.starts_with(" ".repeat(6).as_str()) {
    //             let mut hash = calculate_hash(&prefix);
    //             hash.truncate(16); // Use 16 characters which is 128 bits

    //             // Final format:
    //             // 123...zx-activesupport-8.0.2
    //             // ^^^^^^^^ hash based on source and type (GEM or GIT)
    //             // Obfuscation is essential because remote can include credentials
    //             let path = format!(
    //                 "{RUBY_CACHE_DIR}/gems/{}-{}",
    //                 hash,
    //                 x.trim_start().replace(" (", "-").replace(')', "").as_str()
    //             );

    //             dbg!(&path);
    //             // let path = format!(
    //             //     "{RUBY_CACHE_DIR}/gems/{}@{}",
    //             //     prefix.clone(),
    //             //     x.trim_start().replace(" (", "=").replace(')', "").as_str()
    //             // );
    //             deps.push(RequiredDependency {
    //                 custom_payload: (path.clone(),),
    //                 path,
    //                 custom_name: None,
    //                 short_name: Some(x.into()),
    //                 // custom_payload: (),
    //             });
    //         }
    //     }
    // }

    let job_dir = job_dir.to_owned();
    // let tmp_install_dir = format!("{RUBY_CACHE_DIR}/tmp-install-{}", Uuid::new_v4());
    // let fetch_dir2 = fetch_dir.clone();
    par_install_language_dependencies(
        deps,
        "ruby",
        "gem",
        false,
        *RUBY_CONCURRENT_DOWNLOADS,
        false,
        crate::common::InstallStrategy::Single(Arc::new(move |dependency| {
            let mut cmd = Command::new(if cfg!(windows) {
                "gem"
            } else {
                GEM_PATH.as_str()
            });
            cmd.env_clear().current_dir(&job_dir).envs(vec![
                ("PATH".to_owned(), PATH_ENV.clone()),
                // Make sure there is nothing written to actual home
                // This way we keep everything clean, organized and maintable
                ("HOME".to_owned(), RUBY_CACHE_DIR.to_owned()),
            ]);
            // .envs(PROXY_ENVS.clone())

            cmd.args(&[
                "install",
                &dependency.custom_payload.pkg,
                "--version",
                &dependency.custom_payload.version,
                "--install-dir",
                &dependency.path,
                // Disable transitive dependencies!
                "--ignore-dependencies",
                "--no-document",
                // Make it output minimal info
                "--quiet",
                "--silent",
                // Do not use default sources
                "--clear-sources",
                // Specify the one from lockfile
                "--source",
                match dependency.custom_payload.source {
                    CurrentSource::GIT { remote, revision } => {
                        return Err(anyhow!("GIT is not supported").into())
                    }
                    CurrentSource::GEM { remote } => remote.unwrap(),
                    CurrentSource::INIT => todo!(),
                }
                .trim(),
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

            Ok(cmd)
        })),
        async move |_| Ok(()),
        &job.id,
        &job.workspace_id,
        worker_name,
        conn,
    )
    .await?;
    Ok(rubylib)
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
    rubylib: String,
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
        format!("\n--- RUBY CODE EXECUTION ---\n"),
        conn,
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
        .env("RUBYLIB", rubylib.as_str())
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
        conn,
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
