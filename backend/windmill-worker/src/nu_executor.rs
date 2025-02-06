use std::{collections::HashMap, path::Path, process::Stdio, str::from_utf8_mut};

use itertools::Itertools;
use regex::Regex;
use serde_json::value::RawValue;
use tokio::{fs::File, io::AsyncWriteExt, process::Command};
use windmill_common::{error::Error, jobs::QueuedJob, worker::write_file};
use windmill_parser_nu::parse_nu_signature;
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    common::{create_args_and_out_file, read_result, start_child_process, OccupancyMetrics},
    handle_child,
    rust_executor::CARGO_PATH,
    AuthedClientBackgroundTask, DISABLE_NSJAIL, DISABLE_NUSER, NSJAIL_PATH, PATH_ENV,
};

const NSJAIL_CONFIG_RUN_NU_CONTENT: &str = include_str!("../nsjail/run.nu.config.proto");
lazy_static::lazy_static! {
    static ref NU_PATH: String = std::env::var("NU_PATH").unwrap_or_else(|_| "~/.cargo/bin/nu".to_string());
    static ref PLUGIN_USE_RE: Regex = Regex::new(r#"(?:plugin use )(?<plugin>.*)"#).unwrap();
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

pub async fn handle_nu_job<'a>(mut args: JobHandlerInput<'a>) -> Result<Box<RawValue>, Error> {
    // --- Handle plugins ---
    {
        get_plugins(&mut args).await?;
    }
    // --- Handle imports ---
    {
        // TODO
    }
    // --- Handle relative ---
    {
        // TODO
    }
    // --- Wrap and write to fs ---
    {
        create_args_and_out_file(&args.client, args.job, args.job_dir, args.db).await?;
        File::create(format!("{}/main.nu", args.job_dir))
            .await?
            .write_all(&wrap(args.inner_content)?.into_bytes())
            .await?;
    }
    // --- Execute ---
    {
        run(&mut args).await?;
    }
    // --- Retrieve results ---
    {
        read_result(&args.job_dir).await
    }
}

async fn get_plugins<'a>(
    JobHandlerInput {
        occupancy_metrics,
        mem_peak,
        canceled_by,
        worker_name,
        job,
        db,
        inner_content,
        ..
    }: &mut JobHandlerInput<'a>,
) -> Result<(), Error> {
    let nu_version = from_utf8_mut(
        Command::new(NU_PATH.as_str())
            .arg("--version")
            .output()
            .await?
            .stdout
            .as_mut_slice(),
    )
    .map_err(|e| windmill_common::error::Error::ExecutionErr(e.to_string()))?
    .to_owned();

    for plugin in parse_plugin_use(inner_content) {
        let mut run_cmd = Command::new(CARGO_PATH.as_str());
        // cargo install nu_plugin_query --version (nu --version); plugin add ~/.cargo/bin/nu_plugin_query
        run_cmd
            // TODO: make it work with env_clear
            // .env_clear()
            .args(&[
                "install",
                &format!("nu_plugin_{plugin}"),
                "--version",
                &nu_version,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        nsjail_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());
        let child = start_child_process(run_cmd, "cargo").await?;
        handle_child::handle_child(
            &job.id,
            db,
            mem_peak,
            canceled_by,
            child,
            !*DISABLE_NSJAIL,
            worker_name,
            &job.workspace_id,
            "cargo",
            job.timeout,
            false,
            &mut Some(occupancy_metrics),
        )
        .await?;
    }
    Ok(())
}

fn parse_plugin_use(inner_content: &str) -> Vec<&str> {
    let mut plugins = vec![];
    // TODO: Ignore plugins with # in the beginning
    for cap in PLUGIN_USE_RE.captures_iter(inner_content).into_iter() {
        if let Some(mat) = cap.name("plugin") {
            plugins.push(mat.as_str());
        }
    }
    plugins
}

fn parse_use() {}
/// Wraps content script
/// that upon execution reads args.json (which are piped and transformed from previous flow step or top level inputs)
/// Also wrapper takes output of program and serializes to result.json (Which windmill will know how to use later)
fn wrap(inner_content: &str) -> Result<String, Error> {
    let sig = parse_nu_signature(inner_content)?;
    let spread = sig
        .args
        .clone()
        .into_iter()
        .map(|x| {
            // Apply additional input transformation
            let transformation = match x.typ {
                // JSON converts X.0 to X and nu can't coerce type automatically
                windmill_parser::Typ::Datetime => "into datetime",
                windmill_parser::Typ::Bytes => "into binary ",
                windmill_parser::Typ::Float => "into float",
                _ => "",
            }
            .to_owned();

            if transformation != "" {
                format!("\n\t\t\t($parsed_args.{}? | {transformation}) ", &x.name)
            } else {
                format!("\n\t\t\t($parsed_args.{}?) ", &x.name)
            }
        })
        .collect_vec()
        .join(" ");
    Ok(
        r#"    

$env.config.table.mode = 'basic'

# TODO:
# def safeguard [ name: string ] {
# 	if ($in == null) {
# 		# let span = (metadata $in).span;
# 		# TODO: Impl more reliable way to find span
#         # let block  = view blocks | find "main" ;
# 		panic $"`($name)` can't be null"
# 		# error make {msg: $"`($name)` can't be null", label: {text: "fish right here", span: {start: $block.start, end: $block.end} } }
# 	}
# 	$in
# }

plugin add ~/.cargo/bin/nu_plugin_polars
plugin add ~/.cargo/bin/nu_plugin_query

def 'main --wrapped' [] {
    let parsed_args = open args.json
    # TRANSFORM
    (main SPREAD
    ) | to json | save -f result.json
}

INNER_CONTENT
            "#
        .replace("INNER_CONTENT", inner_content)
        .replace("SPREAD", &spread), // .replace("TRANSFORM", transform)
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
        ..
    }: &mut JobHandlerInput<'a>,
) -> Result<(), Error> {
    let child = if !*DISABLE_NSJAIL {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n\n--- ISOLATED NU CODE EXECUTION ---\n"),
            db.clone(),
        )
        .await;

        write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_NU_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace(
                    "{NU_PATH}",
                    dbg!(Path::new(NU_PATH.as_str())
                        .parent()
                        .map(|parent| parent.to_str())
                        .flatten()
                        .unwrap_or("~/.cargo/bin")),
                )
                .replace("{SHARED_MOUNT}", &shared_mount)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string()),
        )?;
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .env("PATH", PATH_ENV.as_str())
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                // "/tmp/bin/nu",
                NU_PATH.as_str(),
                "/tmp/main.nu",
                "--wrapped",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        nsjail_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
    } else {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n\n--- NU CODE EXECUTION ---\n"),
            db.clone(),
        )
        .await;

        let mut run_cmd = Command::new(NU_PATH.as_str());
        run_cmd
            .current_dir(job_dir)
            .env_clear()
            .args(&["main.nu", "--wrapped"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        nsjail_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());
        start_child_process(run_cmd, NU_PATH.as_str()).await?
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
    .await
}
#[cfg(test)]
mod test {
    use super::parse_plugin_use;

    #[test]
    fn test_nu_plugin_use() {
        let content = r#"
plugin use foo
plugin use bar
plugin use baz
plugin use meh
        "#;
        assert_eq!(
            vec!["foo", "bar", "baz", "meh"], //
            parse_plugin_use(content)
        );
    }
}
