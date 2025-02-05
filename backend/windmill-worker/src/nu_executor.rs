use std::{collections::HashMap, process::Stdio};

use itertools::Itertools;
use serde_json::value::RawValue;
use sqlx::error::ErrorKind;
use tokio::{fs::File, io::AsyncWriteExt, process::Command};
use windmill_common::{error::Error, jobs::QueuedJob};
use windmill_parser_nu::parse_nu_signature;
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    common::{create_args_and_out_file, read_result, start_child_process, OccupancyMetrics},
    handle_child, AuthedClientBackgroundTask, DISABLE_NSJAIL,
};

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
        job_dir, //
        ..
    }: &mut JobHandlerInput<'a>,
) -> Result<(), Error> {
    {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n\n--- NU CODE EXECUTION ---\n"),
            db.clone(),
        )
        .await;
    }
    {
        let child = {
            let mut run_cmd = Command::new("nu");
            run_cmd
                .current_dir(job_dir)
                .args(&["main.nu", "--wrapped"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            start_child_process(run_cmd, "nu main.nu --wrapped").await?
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
}
