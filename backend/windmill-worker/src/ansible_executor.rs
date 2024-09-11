use std::{collections::HashMap, process::Stdio};

use anyhow::anyhow;
use serde_json::value::RawValue;
use tokio::process::Command;
use uuid::Uuid;
use windmill_common::{jobs::QueuedJob, worker::write_file};
use windmill_queue::{append_logs, CanceledBy};
use yaml_rust::{Yaml, YamlEmitter, YamlLoader};

use crate::{
    common::{create_args_and_out_file, handle_child, read_and_check_result, start_child_process},
    AuthedClientBackgroundTask, DISABLE_NSJAIL, HOME_ENV, PATH_ENV, TZ_ENV,
};

lazy_static::lazy_static! {
    static ref ANSIBLE_PLAYBOOK_PATH: String =
    std::env::var("ANSIBLE_PLAYBOOK_PATH").unwrap_or("/bin/ansible-playbook".to_string());

    static ref ANSIBLE_GALAXY_PATH: String =
    std::env::var("ANSIBLE_GALAXY_PATH").unwrap_or("/bin/ansible-galaxy".to_string());
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
        format!("Entered to install galaxy collections"),
        db).await;
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
            "collections",
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

#[derive(Debug)]
struct FileResource {
    windmill_path: String,
    local_path: Option<String>,
}

#[derive(Debug)]
struct AnsibleRequirements {
    python_reqs: Vec<String>,
    collections: Option<String>,
    file_resources: Vec<FileResource>,
}

async fn parse_ansible_yaml(
    job_id: &Uuid,
    w_id: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    inner_content: &str,
) -> anyhow::Result<(Option<AnsibleRequirements>, String)> {
    let docs = YamlLoader::load_from_str(inner_content)
        .map_err(|e| anyhow!("Failed to parse yaml: {}", e))?;

    if docs.len() < 2 {
        return Ok((None, inner_content.to_string()));
    }

    let mut ret =
        AnsibleRequirements { python_reqs: vec![], collections: None, file_resources: vec![] };

    if let Yaml::Hash(doc) = &docs[0] {
        for (key, value) in doc {
            match key {
                Yaml::String(key) if key == "dependencies" => {
                    if let Yaml::Hash(deps) = value {
                        if let Some(galaxy_requirements) =
                            deps.get(&Yaml::String("galaxy".to_string()))
                        {
                            let mut out_str = String::new();
                            let mut emitter = YamlEmitter::new(&mut out_str);
                            emitter.dump(galaxy_requirements)?;
                            ret.collections = Some(out_str);
                        }
                        if let Some(Yaml::Array(py_reqs)) =
                            deps.get(&Yaml::String("python".to_string()))
                        {
                            ret.python_reqs = py_reqs
                                .iter()
                                .map(|d| d.as_str().map(|s| s.to_string()))
                                .filter_map(|x| x)
                                .collect();
                        }
                    }
                }
                Yaml::String(key) if key == "file_resources" => {
                    if let Yaml::Array(file_resources) = value {
                        let resources: anyhow::Result<Vec<FileResource>> =
                            file_resources.iter().map(parse_file_resource).collect();
                        ret.file_resources = resources?;
                    }
                }
                Yaml::String(key) if key == "extra_vars" => {}
                Yaml::String(key) if key == "inventory" => {}
                Yaml::String(key) => {
                    append_logs(
                        job_id,
                        w_id,
                        format!("\nUnknown field `{}`. Ignoring", key),
                        db,
                    )
                    .await;
                }
                _ => (),
            }
        }
    }
    let mut out_str = String::new();
    let mut emitter = YamlEmitter::new(&mut out_str);

    for i in 1..docs.len() {
        emitter.dump(&docs[i])?;
    }
    Ok((Some(ret), out_str))
}

fn parse_file_resource(yaml: &Yaml) -> anyhow::Result<FileResource> {
    if let Yaml::Hash(f) = yaml {
        if let Some(Yaml::String(windmill_path)) = f.get(&Yaml::String("windmill_path".to_string()))
        {
            let local_path = f
                .get(&Yaml::String("windmill_path".to_string()))
                .and_then(|x| x.as_str())
                .map(|x| x.to_string());
            return Ok(FileResource { windmill_path: windmill_path.clone(), local_path });
        }
    }
    return Err(anyhow!("Invalid file resource {:?}", yaml));
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
    shared_mount: &str,
    base_internal_url: &str,
    envs: HashMap<String, String>,
) -> windmill_common::error::Result<Box<RawValue>> {
    let (reqs, playbook) =
        parse_ansible_yaml(&job.id, &job.workspace_id, db, inner_content).await?;
    tracing::error!("{:?}", reqs);
    write_file(job_dir, "main.yml", &playbook)?;

    let ansible_cfg_content = r#"
[defaults]
collections_paths = ./collections"#;
    write_file(job_dir, "ansible.cfg", &ansible_cfg_content)?;
    if let Some(r) = reqs {
        if let Some(collections) = r.collections {
            tracing::error!(collections);
            install_galaxy_collections(collections.as_str(), job_dir, &job.id, worker_name, &job.workspace_id, mem_peak, canceled_by, db).await?;
        }
    }
    create_args_and_out_file(client, job, job_dir, db).await?;
    // let reserved_variables = todo!();
    let child = if !*DISABLE_NSJAIL {
        return Err(anyhow!("Ansible is not supported in nsjail, disable nsjail on your worker to run ansible playbooks").into());
        // let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str()); nsjail_cmd
        //     .current_dir(job_dir)
        //     .env_clear()
        //     // inject PYTHONPATH here - for some reason I had to do it in nsjail conf
        //     .envs(reserved_variables)
        //     .env("PATH", PATH_ENV.as_str())
        //     .env("TZ", TZ_ENV.as_str())
        //     .env("BASE_INTERNAL_URL", base_internal_url)
        //     .env("BASE_URL", base_internal_url)
        //     .args(vec![
        //         "--config",
        //         "run.config.proto",
        //         "--",
        //         ANSIBLE_PLAYBOOK_PATH.as_str(),
        //         "-u",
        //         "-m",
        //         "wrapper",
        //     ])
        //     .stdout(Stdio::piped())
        //     .stderr(Stdio::piped());
        // start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
    } else {
        let mut ansible_cmd = Command::new(ANSIBLE_PLAYBOOK_PATH.as_str());
        ansible_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            // .envs(reserved_variables)
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
        let ret = parse_ansible_yaml(example_file).await.unwrap();
    }
}
