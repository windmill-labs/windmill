use std::{collections::HashMap, process::Stdio};

use anyhow::anyhow;
use serde_json::value::RawValue;
use tokio::process::Command;
use uuid::Uuid;
use windmill_common::{
    error,
    jobs::QueuedJob,
    worker::{write_file, WORKER_CONFIG},
};
use windmill_parser_yaml::AnsibleRequirements;
use windmill_queue::{append_logs, CanceledBy};
use yaml_rust::{Yaml, YamlEmitter, YamlLoader};

use crate::{
    common::{create_args_and_out_file, get_reserved_variables, handle_child, read_and_check_result, start_child_process},
    python_executor::{create_dependencies_dir, handle_python_reqs, pip_compile},
    AuthedClientBackgroundTask, DISABLE_NSJAIL, HOME_ENV, PATH_ENV, TZ_ENV,
};

lazy_static::lazy_static! {
    static ref ANSIBLE_PLAYBOOK_PATH: String =
    std::env::var("ANSIBLE_PLAYBOOK_PATH").unwrap_or("/bin/ansible-playbook".to_string());

    static ref ANSIBLE_GALAXY_PATH: String =
    std::env::var("ANSIBLE_GALAXY_PATH").unwrap_or("/bin/ansible-galaxy".to_string());
}

async fn handle_ansible_python_deps(
    job_dir: &str,
    requirements_o: Option<String>,
    ansible_reqs: Option<&AnsibleRequirements>,
    inner_content: &str,
    w_id: &str,
    script_path: &str,
    job_id: &Uuid,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
) -> error::Result<Vec<String>> {
    create_dependencies_dir(job_dir).await;

    let mut additional_python_paths: Vec<String> = WORKER_CONFIG
        .read()
        .await
        .additional_python_paths
        .clone()
        .unwrap_or_else(|| vec![])
        .clone();

    let requirements = match requirements_o {
        Some(r) => r,
        None => {
            let requirements = ansible_reqs
                .map(|x| x.python_reqs.join("\n"))
                .unwrap_or("".to_string());
            if requirements.is_empty() {
                "".to_string()
            } else {
                pip_compile(
                    job_id,
                    &requirements,
                    mem_peak,
                    canceled_by,
                    job_dir,
                    db,
                    worker_name,
                    w_id,
                )
                .await
                .map_err(|e| {
                    error::Error::ExecutionErr(format!("pip compile failed: {}", e.to_string()))
                })?
            }
        }
    };

    if requirements.len() > 0 {
        let mut venv_path = handle_python_reqs(
            requirements
                .split("\n")
                .filter(|x| !x.starts_with("--"))
                .collect(),
            job_id,
            w_id,
            mem_peak,
            canceled_by,
            db,
            worker_name,
            job_dir,
            worker_dir,
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
    db: &sqlx::Pool<sqlx::Postgres>,
) -> anyhow::Result<()> {
    write_file(job_dir, "requirements.yml", collections_yml)?;

    append_logs(
        job_id,
        w_id,
        "\n\n--- ANSIBLE GALAXY INSTALL ---\n".to_string(),
        db,
    )
    .await;
    let mut galaxy_command = Command::new(ANSIBLE_GALAXY_PATH.as_str());
    galaxy_command
        .current_dir(job_dir)
        .env_clear()
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
        db,
        mem_peak,
        canceled_by,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        w_id,
        "ansible galaxy install",
        None,
        false,
    )
    .await?;

    Ok(())
}


pub async fn handle_ansible_job(
    requirements_o: Option<String>,
    job_dir: &str,
    worker_dir: &str,
    worker_name: &str,
    job: &QueuedJob,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    inner_content: &String,
    base_internal_url: &str,
    envs: HashMap<String, String>,
) -> windmill_common::error::Result<Box<RawValue>> {
    let (logs, reqs, playbook) =
        windmill_parser_yaml::parse_ansible_reqs(inner_content)?;
    append_logs(
        &job.id,
        &job.workspace_id,
        logs,
        db,
    )
    .await;
    write_file(job_dir, "main.yml", &playbook)?;

    let ansible_cfg_content = r#"
[defaults]
collections_path = ./
roles_path = ./roles
"#;
    write_file(job_dir, "ansible.cfg", &ansible_cfg_content)?;

    let script_path = crate::common::use_flow_root_path(job.script_path());
    let additional_python_paths = handle_ansible_python_deps(
        job_dir,
        requirements_o,
        reqs.as_ref(),
        inner_content,
        &job.workspace_id,
        &script_path,
        &job.id,
        db,
        worker_name,
        worker_dir,
        mem_peak,
        canceled_by,
    )
    .await?;

    if let Some(r) = reqs {
        if let Some(collections) = r.collections {
            install_galaxy_collections(
                collections.as_str(),
                job_dir,
                &job.id,
                worker_name,
                &job.workspace_id,
                mem_peak,
                canceled_by,
                db,
            )
            .await?;
        }
    }
    append_logs(
        &job.id,
        &job.workspace_id,
        "\n\n--- ANSIBLE PLAYBOOK EXECUTION ---\n".to_string(),
        db,
    )
    .await;

    if !*DISABLE_NSJAIL {
        return Err(anyhow!("Ansible is not supported with nsjail, disable nsjail on your worker to run ansible playbooks").into());
    }

    create_args_and_out_file(client, job, job_dir, db).await?;

    let client = client.get_authed().await;
    let mut reserved_variables = get_reserved_variables(job, &client.token, db).await?;
    let additional_python_paths_folders = additional_python_paths.join(":");
    reserved_variables.insert("PYTHONPATH".to_string(), additional_python_paths_folders);

    let child = {
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
            .args(vec!["main.yml", "--extra-vars", "@args.json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(ansible_cmd, ANSIBLE_PLAYBOOK_PATH.as_str()).await?
    };

    handle_child(
        &job.id,
        db,
        mem_peak,
        canceled_by,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        &job.workspace_id,
        "python run",
        job.timeout,
        false,
    )
    .await?;
    read_and_check_result(job_dir).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_ansible_file() {
        let example_file = r#"
---
- dependencies:
    galaxy:
      collections:
        - name: community.windows
        - name: ansible.utils
    python:
      - jmespath
      - pandas>=2.0.0
"#;

        println!("asdasdasdasdasd");
        let ret = parse_ansible_reqs(example_file).await.unwrap();
    }
}
