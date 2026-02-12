use convert_case::{Case, Casing};
use itertools::Itertools;
use regex::Regex;
use serde_json::value::RawValue;
use std::{collections::HashMap, process::Stdio};
use tokio::{fs::File, io::AsyncReadExt, process::Command};
use uuid::Uuid;
use windmill_common::{
    error::{self, to_anyhow, Result},
    scripts::ScriptLang,
    worker::{write_file, Connection},
    workspace_dependencies::clean_lock_from_annotations,
};
use windmill_queue::MiniPulledJob;

use windmill_parser::Typ;
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    common::{
        build_command_with_isolation, check_executor_binary_exists, create_args_and_out_file,
        get_reserved_variables, read_result, start_child_process, MaybeLock, OccupancyMetrics,
    },
    handle_child::handle_child,
    COMPOSER_CACHE_DIR, COMPOSER_PATH, is_sandboxing_enabled, DISABLE_NUSER, NSJAIL_PATH, PHP_PATH,
};
use windmill_common::client::AuthedClient;

const NSJAIL_CONFIG_RUN_PHP_CONTENT: &str = include_str!("../nsjail/run.php.config.proto");

lazy_static::lazy_static! {
    static ref RE: Regex = Regex::new(r"^//\s?(\S+)\s*$").unwrap();
}

const COMPOSER_LOCK_SPLIT: &str = "\nLOCK\n";

pub fn parse_php_imports(code: &str) -> anyhow::Result<Option<String>> {
    let find_requirements = code
        .lines()
        .find_position(|x| x.starts_with("//require:") || x.starts_with("// require:"));

    if let Some((pos, _)) = find_requirements {
        let requirements = code
            .lines()
            .skip(pos + 1)
            .map_while(|x| {
                RE.captures(x).map(|x| {
                    match x.get(1).unwrap().as_str().split("@").collect_vec()[..] {
                        [path, version] => (path.to_string(), version.to_string()),
                        [path] | [path, ..] => (path.to_string(), "*".to_string()),
                        [] => unreachable!(),
                    }
                })
            })
            .collect::<HashMap<_, _>>();

        let composer_json =
            serde_json::to_string_pretty(&HashMap::from([("require", requirements)]))
                .map_err(to_anyhow)?;

        Ok(Some(composer_json))
    } else {
        Ok(None)
    }
}

pub async fn composer_install(
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_id: &Uuid,
    w_id: &str,
    conn: &Connection,
    job_dir: &str,
    worker_name: &str,
    requirements: String,
    lock: Option<String>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> Result<String> {
    check_executor_binary_exists("php", PHP_PATH.as_str(), "php")?;

    write_file(job_dir, "composer.json", &requirements)?;

    if let Some(lock) = lock.as_ref() {
        write_file(job_dir, "composer.lock", lock)?;
    }

    let mut child_cmd = Command::new(&*COMPOSER_PATH);
    let args = vec!["install", "--no-dev", "--no-progress"];
    child_cmd
        .current_dir(job_dir)
        .env("COMPOSER_HOME", &*COMPOSER_CACHE_DIR)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child_process = start_child_process(child_cmd, &*COMPOSER_PATH, false).await?;

    handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        child_process,
        false,
        worker_name,
        w_id,
        "composer install",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;

    match lock {
        Some(lock) => Ok(format!("{requirements}{COMPOSER_LOCK_SPLIT}{lock}")),
        None => {
            let mut lock_content = "".to_string();
            let mut lock_file = File::open(format!("{job_dir}/composer.lock")).await?;
            lock_file.read_to_string(&mut lock_content).await?;
            Ok(format!("{requirements}{COMPOSER_LOCK_SPLIT}{lock_content}"))
        }
    }
}

fn generate_resource_class(rt_name: &str, arg_name: &str) -> String {
    let rt_name = rt_name.to_case(Case::Pascal);
    format!(
        "#[AllowDynamicProperties]
class {rt_name} {{
  public function __construct($data) {{
    foreach ($data AS $key => $value) $this->{{$key}} = $value;
  }}
}}
$args->{arg_name} = new {rt_name}($args->{arg_name});"
    )
}

fn split_reqs_and_lock(content: &String) -> error::Result<(Option<String>, Option<String>)> {
    let splitted = content.split(COMPOSER_LOCK_SPLIT).collect_vec();
    if splitted.len() != 2 {
        return Err(error::Error::ExecutionErr(format!(
            "Invalid requirements, expected to find LOCK split pattern in reqs. Found: |{content}|"
        )));
    }

    Ok((
        Some(clean_lock_from_annotations(splitted[0], ScriptLang::Php)),
        Some(splitted[1].to_string()),
    ))
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_php_job(
    maybe_lock: MaybeLock,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &MiniPulledJob,
    conn: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    job_dir: &str,
    inner_content: &String,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    shared_mount: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<Box<RawValue>> {
    check_executor_binary_exists("php", PHP_PATH.as_str(), "php")?;

    let (composer_json, composer_lock) = match &maybe_lock {
        MaybeLock::Resolved { lock } if !lock.is_empty() => split_reqs_and_lock(lock)?,
        MaybeLock::Unresolved { workspace_dependencies } => (
            workspace_dependencies
                .get_php()?
                .or(parse_php_imports(inner_content)?),
            None,
        ),
        _ => (parse_php_imports(inner_content)?, None),
    };

    let autoload_line = if let Some(composer_json) = composer_json {
        let logs1 = "\n\n--- COMPOSER INSTALL ---\n".to_string();
        append_logs(&job.id, &job.workspace_id, logs1, conn).await;

        composer_install(
            mem_peak,
            canceled_by,
            &job.id,
            &job.workspace_id,
            conn,
            job_dir,
            worker_name,
            composer_json,
            composer_lock,
            occupancy_metrics,
        )
        .await?;
        "require './vendor/autoload.php';"
    } else {
        ""
    };

    let init_logs = "\n\n--- PHP CODE EXECUTION ---\n".to_string();

    append_logs(&job.id, job.workspace_id.to_string(), init_logs, conn).await;

    let _ = write_file(job_dir, "main.php", inner_content)?;

    let main_override = job.script_entrypoint_override.as_deref();

    let write_wrapper_f = async {
        let args = windmill_parser_php::parse_php_signature(
            inner_content,
            main_override.map(ToString::to_string),
        )?
        .args;

        let args_to_include = args
            .iter()
            .filter(|x| {
                !x.has_default || job.args.as_ref().is_some_and(|a| a.contains_key(&x.name))
            })
            .collect::<Vec<_>>();

        let func_args_str = args_to_include
            .iter()
            .map(|x| format!("$args->{}", x.name))
            .collect::<Vec<String>>()
            .join(",");

        let resource_classes = args_to_include
            .iter()
            .filter_map(|x| match &x.typ {
                Typ::Resource(name) => Some((name, &x.name)),
                _ => None,
            })
            .unique()
            .map(|(rt_name, arg_name)| generate_resource_class(rt_name, arg_name))
            .collect::<Vec<_>>()
            .join("\n\n");

        let main_name = main_override.unwrap_or("main");

        let wrapper_content: String = format!(
            r#"
<?php
{autoload_line}
require './main.php';

$args_str = file_get_contents('./args.json');
$args = json_decode($args_str);

{resource_classes}

try {{
  $res = {main_name}({func_args_str});
  file_put_contents("result.json", json_encode($res));
}} catch (Exception $e) {{
    $err = [
        "message" => $e->getMessage(),
        "name" => get_class($e),
        "stack" => $e->getTraceAsString()
    ];
    $step_id = getenv('WM_FLOW_STEP_ID');
    if ($step_id) {{
        $err["step_id"] = $step_id;
    }}
    file_put_contents("result.json", json_encode($err));
    exit(1);
}}
    "#,
        );
        write_file(job_dir, "wrapper.php", &wrapper_content)?;
        Ok(()) as error::Result<()>
    };

    let reserved_variables_args_out_f = async {
        let args_and_out_f = async {
            create_args_and_out_file(&client, job, job_dir, conn).await?;
            Ok(()) as Result<()>
        };
        let reserved_variables_f = async {
            let vars =
                get_reserved_variables(job, &client.token, conn, parent_runnable_path.clone())
                    .await?;
            Ok(vars) as Result<HashMap<String, String>>
        };
        let (_, reserved_variables) = tokio::try_join!(args_and_out_f, reserved_variables_f)?;
        Ok(reserved_variables) as error::Result<HashMap<String, String>>
    };

    let (reserved_variables, _) = tokio::try_join!(reserved_variables_args_out_f, write_wrapper_f)?;

    let child = if is_sandboxing_enabled() {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_PHP_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )?;

        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        let args = vec![
            "--config",
            "run.config.proto",
            "--",
            &PHP_PATH,
            "/tmp/wrapper.php",
        ];
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("COMPOSER_HOME", &*COMPOSER_CACHE_DIR)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        let script_path = format!("{job_dir}/wrapper.php");
        let args = vec![script_path.as_str()];

        let mut php_cmd = build_command_with_isolation(&*PHP_PATH, &args);
        php_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("COMPOSER_HOME", &*COMPOSER_CACHE_DIR)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        start_child_process(php_cmd, &*PHP_PATH, false).await?
    };

    handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        is_sandboxing_enabled(),
        worker_name,
        &job.workspace_id,
        "php run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;
    read_result(job_dir, None).await
}
