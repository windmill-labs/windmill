use std::{collections::HashMap, process::Stdio};

use itertools::Itertools;
use serde_json::value::RawValue;
use tokio::{fs::File, io::AsyncWriteExt, process::Command};
use windmill_common::{
    error::Error,
    worker::{write_file, Connection},
};
use windmill_parser::Arg;
use windmill_parser_nu::parse_nu_signature;
use windmill_queue::{append_logs, CanceledBy, MiniPulledJob};

use crate::{
    common::{
        build_command_with_isolation, create_args_and_out_file, get_reserved_variables,
        read_result, start_child_process, OccupancyMetrics, DEV_CONF_NSJAIL,
    },
    get_proxy_envs_for_lang, handle_child, is_sandboxing_enabled, DISABLE_NUSER, NSJAIL_PATH, PATH_ENV,
    TRACING_PROXY_CA_CERT_PATH,
};
use windmill_common::client::AuthedClient;
use windmill_common::scripts::ScriptLang;

const NSJAIL_CONFIG_RUN_NU_CONTENT: &str = include_str!("../nsjail/run.nu.config.proto");
lazy_static::lazy_static! {
    static ref NU_PATH: String = std::env::var("NU_PATH").unwrap_or_else(|_| "/usr/bin/nu".to_string());
    // TODO(v1):
    // static ref PLUGIN_USE_RE: Regex = Regex::new(r#"(?:plugin use )(?<plugin>.*)"#).unwrap();
}

// TODO: Can be generalized and used for other handlers
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

pub async fn handle_nu_job<'a>(mut args: JobHandlerInput<'a>) -> Result<Box<RawValue>, Error> {
    // TODO(v1):
    // --- Handle plugins ---
    // let plugins = get_plugins(&mut args).await?;
    // TODO(v1):
    // --- Handle imports ---
    // TODO(v1):
    // --- Handle relative ---
    // --- Wrap and write to fs ---
    {
        create_args_and_out_file(&args.client, args.job, args.job_dir, args.conn).await?;
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
        read_result(&args.job_dir, None).await
    }
}

// async fn get_plugins<'a>(
//     JobHandlerInput {
//         occupancy_metrics,
//         mem_peak,
//         canceled_by,
//         worker_name,
//         job,
//         db,
//         inner_content,
//         ..
//     }: &mut JobHandlerInput<'a>,
// ) -> Result<Vec<&'a str>, Error> {
//     let plugins_dir = concatcp!(NU_CACHE_DIR, "/plugins");
//     let nu_version = from_utf8_mut(
//         Command::new(NU_PATH.as_str())
//             .arg("--version")
//             .output()
//             .await?
//             .stdout
//             .as_mut_slice(),
//     )
//     .map_err(|e| windmill_common::error::Error::ExecutionErr(e.to_string()))?
//     .to_owned();

//     let plugins = parse_plugin_use(inner_content);

//     for plugin in &plugins {
//         let mut run_cmd = Command::new(CARGO_PATH.as_str());
//         // cargo install nu_plugin_query --version (nu --version); plugin add ~/.cargo/bin/nu_plugin_query
//         run_cmd
//             // TODO: make it work with env_clear
//             // .env_clear()
//             .args(&[
//                 "install",
//                 "--root",
//                 plugins_dir,
//                 "--locked",
//                 &format!("nu_plugin_{plugin}"),
//                 "--version",
//                 &nu_version,
//             ])
//             .stdout(Stdio::piped())
//             .stderr(Stdio::piped());

//         #[cfg(windows)]
//         nsjail_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());
//         let child = start_child_process(run_cmd, "cargo", false).await?;
//         // handle_child::handle_child(
//         //     &job.id,
//         //     db,
//         //     mem_peak,
//         //     canceled_by,
//         //     child,
//         //     is_sandboxing_enabled(),
//         //     worker_name,
//         //     &job.workspace_id,
//         //     "cargo",
//         //     job.timeout,
//         //     false,
//         //     &mut Some(occupancy_metrics),
//         // )
//         // .await?;
//     }
//     Ok(plugins)
// }

// fn parse_plugin_use(inner_content: &str) -> Vec<&str> {
//     let mut plugins = vec![];
//     // TODO: Ignore plugins with # in the beginning
//     for cap in PLUGIN_USE_RE.captures_iter(inner_content).into_iter() {
//         if let Some(mat) = cap.name("plugin") {
//             plugins.push(mat.as_str());
//         }
//     }
//     plugins
// }

/// Wraps content script
/// that upon execution reads args.json (which are piped and transformed from previous flow step or top level inputs)
/// Also wrapper takes output of program and serializes to result.json (Which windmill will know how to use later)
fn wrap(inner_content: &str) -> Result<String, Error> {
    let sig = parse_nu_signature(inner_content)?;
    let spread = sig
        .args
        .clone()
        .into_iter()
        .map(|Arg { name, typ, has_default, .. }| {
            // Apply additional input transformation
            let transformation = format!(
                "| if $in != null {{ {} }} else {{ $in }}",
                match typ {
                    // JSON converts X.0 to X and nu can't coerce type automatically
                    windmill_parser::Typ::Datetime => "into datetime",
                    windmill_parser::Typ::Bytes => "into binary",
                    windmill_parser::Typ::Float => "into float",
                    // Ident
                    _ => "$in",
                }
            );
            let nullguard = if has_default || matches!(typ, windmill_parser::Typ::Unknown) {
                "".to_owned()
            } else {
                format!("| nullguard {name}")
            };
            format!("\n\t\t\t($parsed_args.{name}? {nullguard} {transformation}) ",)
        })
        .collect_vec()
        .join(" ");
    Ok(
        r#"
$env.config.table.mode = 'basic'

def nullguard [ name: string ] {
	if ($in == null) {
		panic $"argument `($name)` of main function can't be null"
	}
	$in
}

# TODO: Probably needs rework in order for LSP to work
def get_variable [ pat ] {
    let addr = $"($env.BASE_INTERNAL_URL)/api/w/($env.WM_WORKSPACE)/variables/get_value/($pat)" ;
    http get -H ["Authorization", $"Bearer ($env.WM_TOKEN)"] $addr | return $in
}
def get_resource [ pat ] {
    let addr = $"($env.BASE_INTERNAL_URL)/api/w/($env.WM_WORKSPACE)/resources/get_value_interpolated/($pat)" ;
    http get -H ["Authorization", $"Bearer ($env.WM_TOKEN)"] $addr | return $in
}

def 'main --wrapped' [] {
    let parsed_args = open args.json
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
        conn,
        job_dir,
        shared_mount,
        client,
        parent_runnable_path,
        envs,
        base_internal_url,
        ..
    }: &mut JobHandlerInput<'a>,
    // plugins: Vec<&'a str>,
) -> Result<(), Error> {
    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path.clone()).await?;
    let child = if !cfg!(windows) && is_sandboxing_enabled() {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n\n--- ISOLATED NU CODE EXECUTION ---\n"),
            conn,
        )
        .await;

        write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_NU_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{NU_PATH}", &NU_PATH)
                .replace("{SHARED_MOUNT}", &shared_mount)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{TRACING_PROXY_CA_CERT_PATH}", TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL),
        )?;
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .env_clear()
            .current_dir(job_dir)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(envs)
            .envs(reserved_variables)
            .envs(get_proxy_envs_for_lang(&ScriptLang::Nu).await?)
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                NU_PATH.as_str(),
                "/tmp/main.nu",
                "--wrapped",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("\n\n--- NU CODE EXECUTION ---\n"),
            &conn,
        )
        .await;

        // let plugin_registry = format!("{job_dir}/plugin-registry");
        // File::create(&plugin_registry).await?;
        //
        let nu_executable = if cfg!(windows) {
            "nu"
        } else {
            NU_PATH.as_str()
        };

        let args = vec!["main.nu", "--wrapped"];
        let mut cmd = build_command_with_isolation(nu_executable, &args);
        cmd.env_clear()
            .current_dir(job_dir.to_owned())
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .envs(envs)
            .envs(reserved_variables)
            .envs(get_proxy_envs_for_lang(&ScriptLang::Nu).await?)
            // TODO(v1):
            // "--plugins",
            // &format!(
            //     "[{}]",
            //     plugins
            //         .into_iter()
            //         .map(|pl| format!("{NU_CACHE_DIR}/plugins/bin/nu_plugin_{pl}"))
            //         .collect_vec()
            //         .join(",")
            // ),
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
        start_child_process(cmd, nu_executable, false).await?
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
        "nu",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;
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
