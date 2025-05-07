#[cfg(unix)]
use std::{collections::HashMap, os::unix::fs::PermissionsExt, path::PathBuf, process::Stdio};

#[cfg(windows)]
use std::{collections::HashMap, path::PathBuf, process::Stdio};

use anyhow::anyhow;
use itertools::Itertools;
use serde_json::value::RawValue;
use tokio::process::Command;
use uuid::Uuid;
use windmill_common::{
    error,
    worker::{
        to_raw_value, write_file, write_file_at_user_defined_location, Connection, WORKER_CONFIG,
    },
};
use windmill_queue::MiniPulledJob;

use windmill_parser_yaml::{AnsibleRequirements, ResourceOrVariablePath};
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    bash_executor::BIN_BASH,
    common::{
        check_executor_binary_exists, get_reserved_variables, read_and_check_result,
        start_child_process, transform_json, OccupancyMetrics,
    },
    handle_child::handle_child,
    python_executor::{
        create_dependencies_dir, handle_python_reqs, split_requirements, uv_pip_compile, PyVersion,
    },
    AuthedClient, DISABLE_NSJAIL, DISABLE_NUSER, HOME_ENV, NSJAIL_PATH, PATH_ENV, PROXY_ENVS,
    PY_INSTALL_DIR, TZ_ENV,
};

lazy_static::lazy_static! {
    static ref ANSIBLE_PLAYBOOK_PATH: String =
    std::env::var("ANSIBLE_PLAYBOOK_PATH").unwrap_or("/usr/local/bin/ansible-playbook".to_string());

    static ref ANSIBLE_GALAXY_PATH: String =
    std::env::var("ANSIBLE_GALAXY_PATH").unwrap_or("/usr/local/bin/ansible-galaxy".to_string());
}

const NSJAIL_CONFIG_RUN_ANSIBLE_CONTENT: &str = include_str!("../nsjail/run.ansible.config.proto");

async fn handle_ansible_python_deps(
    job_dir: &str,
    requirements_o: Option<&String>,
    ansible_reqs: Option<&AnsibleRequirements>,
    w_id: &str,
    job_id: &Uuid,
    conn: &Connection,
    worker_name: &str,
    worker_dir: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<Vec<String>> {
    create_dependencies_dir(job_dir).await;

    let mut additional_python_paths: Vec<String> = WORKER_CONFIG
        .read()
        .await
        .additional_python_paths
        .clone()
        .unwrap_or_else(|| vec![])
        .clone();

    let mut requirements;
    let requirements = match requirements_o {
        Some(r) => r,
        None => {
            requirements = ansible_reqs
                .map(|x| x.python_reqs.join("\n"))
                .unwrap_or("".to_string());
            if !requirements.is_empty() {
                requirements = uv_pip_compile(
                    job_id,
                    &requirements,
                    mem_peak,
                    canceled_by,
                    job_dir,
                    conn,
                    worker_name,
                    w_id,
                    &mut Some(occupancy_metrics),
                    // PyVersion::Py311,
                    todo!(),
                    false,
                )
                .await
                .map_err(|e| {
                    error::Error::ExecutionErr(format!("pip compile failed: {}", e.to_string()))
                })?;
            }
            &requirements
        }
    };

    if requirements.len() > 0 {
        let mut venv_path = handle_python_reqs(
            split_requirements(requirements),
            job_id,
            w_id,
            mem_peak,
            canceled_by,
            conn,
            worker_name,
            job_dir,
            worker_dir,
            &mut Some(occupancy_metrics),
            crate::python_executor::PyVAlias::default().into(),
        )
        .await?;
        additional_python_paths.append(&mut venv_path);
    }
    Ok(additional_python_paths)
}

async fn install_galaxy_collections(
    collections_yml: &str,
    job_dir: &str,
    job_id: &Uuid,
    worker_name: &str,
    w_id: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    conn: &Connection,
    occupancy_metrics: &mut OccupancyMetrics,
) -> anyhow::Result<()> {
    write_file(job_dir, "requirements.yml", collections_yml)?;

    append_logs(
        job_id,
        w_id,
        "\n\n--- ANSIBLE GALAXY INSTALL ---\n".to_string(),
        conn,
    )
    .await;
    let mut galaxy_command = Command::new(ANSIBLE_GALAXY_PATH.as_str());
    galaxy_command
        .current_dir(job_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .env("PATH", PATH_ENV.as_str())
        .env("TZ", TZ_ENV.as_str())
        // .env("BASE_INTERNAL_URL", base_internal_url)
        // .env("HOME", HOME_ENV.as_str())
        .args(vec![
            "collection",
            "install",
            "-r",
            "requirements.yml",
            "-p",
            "./",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = start_child_process(galaxy_command, ANSIBLE_GALAXY_PATH.as_str()).await?;
    handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        w_id,
        "ansible galaxy install",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
    )
    .await?;

    Ok(())
}

pub async fn handle_ansible_job(
    requirements_o: Option<&String>,
    job_dir: &str,
    worker_dir: &str,
    worker_name: &str,
    job: &MiniPulledJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    conn: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    inner_content: &String,
    shared_mount: &str,
    base_internal_url: &str,
    envs: HashMap<String, String>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> windmill_common::error::Result<Box<RawValue>> {
    check_executor_binary_exists(
        "ansible-playbook",
        ANSIBLE_PLAYBOOK_PATH.as_str(),
        "ansible",
    )?;

    let (logs, reqs, playbook) = windmill_parser_yaml::parse_ansible_reqs(inner_content)?;
    append_logs(&job.id, &job.workspace_id, logs, conn).await;
    write_file(job_dir, "main.yml", &playbook)?;

    let additional_python_paths = handle_ansible_python_deps(
        job_dir,
        requirements_o,
        reqs.as_ref(),
        &job.workspace_id,
        &job.id,
        conn,
        worker_name,
        worker_dir,
        mem_peak,
        canceled_by,
        occupancy_metrics,
    )
    .await?;

    let interpolated_args;
    if let Some(args) = &job.args {
        let mut args = args.0.clone();
        if let Some(reqs) = reqs.clone() {
            for (name, path) in reqs.resources.iter().chain(reqs.vars.iter()) {
                args.insert(name.clone(), to_raw_value(path));
            }
        }
        if let Some(x) = transform_json(client, &job.workspace_id, &args, job, conn).await? {
            write_file(
                job_dir,
                "args.json",
                &serde_json::to_string(&x).unwrap_or_else(|_| "{}".to_string()),
            )?;
            interpolated_args = Some(x);
        } else {
            write_file(
                job_dir,
                "args.json",
                &serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string()),
            )?;
            interpolated_args = Some(args);
        }
    } else {
        interpolated_args = None;
        write_file(job_dir, "args.json", "{}")?;
    };
    write_file(job_dir, "result.json", "")?;

    let cmd_options: Vec<String> = reqs
        .as_ref()
        .map(|r| r.options.clone())
        .map(|r| get_cmd_options(r))
        .unwrap_or_default();

    let inventories: Vec<String> = reqs
        .as_ref()
        .map(|x| {
            x.inventories
                .clone()
                .iter()
                .flat_map(|i| vec!["-i".to_string(), i.name.clone()].into_iter())
                .collect()
        })
        .unwrap_or_else(|| vec![]);

    let mut nsjail_extra_mounts = vec![];
    if let Some(r) = reqs {
        if let Some(db) = conn.as_sql() {
            nsjail_extra_mounts = create_file_resources(
                &job.id,
                &job.workspace_id,
                job_dir,
                interpolated_args.as_ref(),
                &r,
                &client,
                db,
            )
            .await?;
        }

        if let Some(collections) = r.collections {
            install_galaxy_collections(
                collections.as_str(),
                job_dir,
                &job.id,
                worker_name,
                &job.workspace_id,
                mem_peak,
                canceled_by,
                conn,
                occupancy_metrics,
            )
            .await?;
        }
    }
    append_logs(
        &job.id,
        &job.workspace_id,
        "\n\n--- ANSIBLE PLAYBOOK EXECUTION ---\n".to_string(),
        conn,
    )
    .await;
    let ansible_cfg_content = format!(
        r#"
[defaults]
collections_path = ./
roles_path = ./roles
home={job_dir}/.ansible
local_tmp={job_dir}/.ansible/tmp
remote_tmp={job_dir}/.ansible/tmp
"#
    );
    write_file(job_dir, "ansible.cfg", &ansible_cfg_content)?;

    let mut reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;
    let additional_python_paths_folders = additional_python_paths.join(":");

    if !*DISABLE_NSJAIL {
        let shared_deps = additional_python_paths
            .into_iter()
            .map(|pp| {
                format!(
                    r#"
mount {{
    src: "{pp}"
    dst: "{pp}"
    is_bind: true
    rw: false
}}
        "#
                )
            })
            .join("\n");
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_ANSIBLE_CONTENT
                .replace("{PY_INSTALL_DIR}", PY_INSTALL_DIR)
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{SHARED_DEPENDENCIES}", shared_deps.as_str())
                .replace("{FILE_RESOURCES}", nsjail_extra_mounts.join("\n").as_str())
                .replace(
                    "{ADDITIONAL_PYTHON_PATHS}",
                    additional_python_paths_folders.as_str(),
                ),
        )?;
    } else {
        reserved_variables.insert("PYTHONPATH".to_string(), additional_python_paths_folders);
    }

    let mut cmd_args = vec!["main.yml", "--extra-vars", "@args.json"];
    cmd_args.extend(inventories.iter().map(|s| s.as_str()));
    cmd_args.extend(cmd_options.iter().map(|s| s.as_str()));

    let child = if !*DISABLE_NSJAIL {
        let wrapper = format!(
            r#"set -eou pipefail
{0} "$@"
if [ -f "result" ]; then
    cat result > result_nsjail_mount.json
fi
if [ -f "result.json" ]; then
    cat result.json > result_nsjail_mount.json
fi
"#,
            ANSIBLE_PLAYBOOK_PATH.as_str()
        );

        let file = write_file(job_dir, "wrapper.sh", &wrapper)?;

        #[cfg(unix)]
        file.metadata()?.permissions().set_mode(0o777);
        // let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(PROXY_ENVS.clone())
            // inject PYTHONPATH here - for some reason I had to do it in nsjail conf
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("BASE_URL", base_internal_url)
            .args(
                vec![
                    "--config",
                    "run.config.proto",
                    "--",
                    BIN_BASH.as_str(),
                    "/tmp/wrapper.sh",
                ]
                .into_iter()
                .chain(cmd_args),
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
    } else {
        let mut ansible_cmd = Command::new(ANSIBLE_PLAYBOOK_PATH.as_str());
        ansible_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        ansible_cmd.env("USERPROFILE", crate::USERPROFILE_ENV.as_str());

        start_child_process(ansible_cmd, ANSIBLE_PLAYBOOK_PATH.as_str()).await?
    };

    handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        &job.workspace_id,
        "python run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
    )
    .await?;
    read_and_check_result(job_dir).await
}

fn get_cmd_options(r: windmill_parser_yaml::AnsiblePlaybookOptions) -> Vec<String> {
    let mut ret = vec![];

    if let Some(v) = r.verbosity {
        if v.chars().all(|c| c == 'v') && v.len() <= 6 {
            ret.push(format!("-{}", v));
        }
    }

    if let Some(o) = r.timeout {
        ret.push("--timeout".to_string());
        ret.push(o.to_string());
    }

    if let Some(o) = r.forks {
        ret.push("--forks".to_string());
        ret.push(o.to_string());
    }

    if r.flush_cache.is_some() {
        ret.push("--flush-cache".to_string());
    }

    if r.force_handlers.is_some() {
        ret.push("--force-handlers".to_string());
    }

    ret
}

fn define_nsjail_mount(job_dir: &str, path: &PathBuf) -> anyhow::Result<String> {
    Ok(format!(
        r#"
mount {{
    src: "{0}/{1}"
    dst: "/tmp/{1}"
    is_bind: true
    rw: false
    mandatory: false
}}
        "#,
        job_dir,
        path.strip_prefix(job_dir)?
            .to_str()
            .ok_or(anyhow!("Invalid path."))?
    ))
}

async fn create_file_resources(
    job_id: &Uuid,
    w_id: &str,
    job_dir: &str,
    args: Option<&HashMap<String, Box<RawValue>>>,
    r: &AnsibleRequirements,
    client: &crate::AuthedClient,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> error::Result<Vec<String>> {
    let mut logs = String::new();
    let mut nsjail_mounts: Vec<String> = vec![];

    for inventory in &r.inventories {
        let content;
        if let Some(resource_path) = &inventory.pinned_resource {
            content = client
                .get_resource_value_interpolated::<serde_json::Value>(
                    resource_path,
                    Some(job_id.to_string()),
                )
                .await?;
        } else {
            let o = args
                .as_ref()
                .and_then(|g| g.get(&inventory.name))
                .ok_or(anyhow!(
                    "Specified inventory was missing in the script arguments"
                ))?;

            content = serde_json::from_str(o.get())
                .map_err(|e| anyhow!("Failed to parse inventory arg: {}", e))?;

            if content == serde_json::value::Value::Null {
                Err(anyhow!("The inventory argument was left empty. If you do not wish to specify an inventory for this script, remove the `inventory:` section from the yaml."))?;
            }
        }

        let validated_path = write_file_at_user_defined_location(
            job_dir,
            &inventory.name,
            content
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or(anyhow!(
                    "Invalid inventory resource, `content` field absent or invalid"
                ))?,
            None,
        )
        .map_err(|e| anyhow!("Couldn't write inventory: {}", e))?;

        nsjail_mounts.push(
            define_nsjail_mount(job_dir, &validated_path)
                .map_err(|e| anyhow!("Inventory path (a.k.a. `name`) is invalid: {}", e))?,
        );

        logs.push_str(&format!("\nCreated inventory `{}`", inventory.name));
    }

    for file_res in &r.file_resources {
        let r =
            get_resource_or_variable_content(client, &file_res.resource_path, job_id.to_string())
                .await?;
        let path = file_res.target_path.clone();
        let validated_path =
            write_file_at_user_defined_location(job_dir, path.as_str(), &r, file_res.mode)
                .map_err(|e| anyhow!("Couldn't write text file at {}: {}", path, e))?;

        nsjail_mounts.push(
            define_nsjail_mount(job_dir, &validated_path)
                .map_err(|e| anyhow!("File resource path is invalid: {}", e))?,
        );

        logs.push_str(&format!(
            "\nCreated {} from {:?}",
            file_res.target_path, file_res.resource_path
        ));
    }
    append_logs(job_id, w_id, logs, &Connection::Sql(db.clone())).await;

    Ok(nsjail_mounts)
}

async fn get_resource_or_variable_content(
    client: &crate::AuthedClient,
    path: &ResourceOrVariablePath,
    job_id: String,
) -> anyhow::Result<String> {
    Ok(match path {
        ResourceOrVariablePath::Resource(p) => {
            let r = client
                .get_resource_value_interpolated::<serde_json::Value>(&p, Some(job_id))
                .await?;

            r.get("content")
                .and_then(|v| v.as_str())
                .ok_or(anyhow!(
                    "Invalid text file resource {}, `content` field absent or invalid",
                    p
                ))?
                .to_string()
        }
        ResourceOrVariablePath::Variable(p) => client.get_variable_value(&p).await?,
    })
}
