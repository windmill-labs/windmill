#[cfg(unix)]
use std::{collections::HashMap, os::unix::fs::PermissionsExt, path::PathBuf, process::Stdio};

#[cfg(windows)]
use std::{collections::HashMap, path::PathBuf, process::Stdio};

use anyhow::anyhow;
use futures::future::try_join_all;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use tokio::process::Command;
use uuid::Uuid;
use windmill_common::{
    error,
    git_sync_oss::prepend_token_to_github_url,
    worker::{
        is_allowed_file_location, split_python_requirements, to_raw_value, write_file,
        write_file_at_user_defined_location, Connection, PyVAlias, WORKER_CONFIG,
    },
};
use windmill_queue::MiniPulledJob;

use windmill_parser_yaml::{
    AnsibleRequirements, GitRepo, PreexistingAnsibleInventory, ResourceOrVariablePath,
};
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    bash_executor::BIN_BASH,
    common::{
        build_command_with_isolation, check_executor_binary_exists, get_reserved_variables,
        read_and_check_result, start_child_process, transform_json, OccupancyMetrics,
    },
    handle_child::handle_child,
    python_executor::{create_dependencies_dir, handle_python_reqs, uv_pip_compile},
    is_sandboxing_enabled, DISABLE_NUSER, GIT_PATH, HOME_ENV, NSJAIL_PATH, PATH_ENV, PROXY_ENVS,
    PY_INSTALL_DIR, TZ_ENV,
};
use windmill_common::client::AuthedClient;

lazy_static::lazy_static! {
    static ref ANSIBLE_PLAYBOOK_PATH: String =
    std::env::var("ANSIBLE_PLAYBOOK_PATH").unwrap_or("/usr/local/bin/ansible-playbook".to_string());

    static ref ANSIBLE_GALAXY_PATH: String =
    std::env::var("ANSIBLE_GALAXY_PATH").unwrap_or("/usr/local/bin/ansible-galaxy".to_string());
}

const NSJAIL_CONFIG_RUN_ANSIBLE_CONTENT: &str = include_str!("../nsjail/run.ansible.config.proto");
const WINDMILL_ANSIBLE_PASSWORD_FILENAME: &str = ".windmill.ansible_vault_password_file";

async fn clone_repo(
    repo: &GitRepo,
    job_dir: &str,
    job_id: &Uuid,
    worker_name: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    w_id: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    git_ssh_cmd: &str,
) -> error::Result<String> {
    let target_path = is_allowed_file_location(job_dir, &repo.target_path)?;

    let mut clone_cmd = Command::new(GIT_PATH.as_str());
    clone_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .env("PATH", PATH_ENV.as_str())
        .env("TZ", TZ_ENV.as_str())
        .env("GIT_SSH_COMMAND", git_ssh_cmd)
        .args(["clone", "--quiet"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(branch) = &repo.branch {
        clone_cmd.args(["--branch", branch]);
    }
    clone_cmd.arg(&repo.url);
    clone_cmd.arg(&target_path);

    let clone_cmd_child = start_child_process(clone_cmd, GIT_PATH.as_str(), false).await?;
    handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        clone_cmd_child,
        false,
        worker_name,
        w_id,
        "git clone",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;

    // Checkout specific commit if provided
    if let Some(commit) = &repo.commit {
        let mut checkout_cmd = Command::new(GIT_PATH.as_str());
        checkout_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(PROXY_ENVS.clone())
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("GIT_SSH_COMMAND", git_ssh_cmd)
            .arg("-C")
            .arg(&target_path)
            .args(["checkout", "--quiet", commit])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let checkout_cmd_child =
            start_child_process(checkout_cmd, GIT_PATH.as_str(), false).await?;
        handle_child(
            job_id,
            conn,
            mem_peak,
            canceled_by,
            checkout_cmd_child,
            false,
            worker_name,
            w_id,
            "git checkout",
            None,
            false,
            &mut Some(occupancy_metrics),
            None,
            None,
        )
        .await?;
    }

    let mut rev_parse_cmd = Command::new(GIT_PATH.as_str());

    let commit_hash_output = rev_parse_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .env("PATH", PATH_ENV.as_str())
        .env("TZ", TZ_ENV.as_str())
        .env("GIT_SSH_COMMAND", git_ssh_cmd)
        .arg("-C")
        .arg(&target_path)
        .args(["rev-parse", "HEAD"])
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !commit_hash_output.status.success() {
        let stderr = String::from_utf8(commit_hash_output.stderr)?;
        return Err(anyhow!("Error getting git repo commit hash: {stderr}").into());
    }

    let commit_hash = String::from_utf8(commit_hash_output.stdout)?
        .trim()
        .to_string();

    Ok(commit_hash)
}

pub fn create_empty_dir(path: &PathBuf) -> std::io::Result<()> {
    if path.exists() {
        if path.is_dir() {
            let mut entries = std::fs::read_dir(&path)?;
            if entries.next().is_some() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!(
                        "Directory '{}' already exists and is not empty",
                        path.display()
                    ),
                ));
            }
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Path '{}' exists and is not a directory", path.display()),
            ))
        }
    } else {
        std::fs::create_dir_all(path)
    }
}

async fn clone_repo_without_history(
    repo: &GitRepo,
    full_commit: &str,
    job_dir: &str,
    job_id: &Uuid,
    worker_name: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    w_id: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    git_ssh_cmd: &str,
) -> error::Result<()> {
    let target_path = is_allowed_file_location(job_dir, &repo.target_path)?;

    create_empty_dir(&target_path)?;

    let mut init_cmd = Command::new(GIT_PATH.as_str());
    init_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .env("PATH", PATH_ENV.as_str())
        .env("TZ", TZ_ENV.as_str())
        .arg("-C")
        .arg(&target_path)
        .args(["init", "--quiet"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(branch) = &repo.branch {
        init_cmd.args(["--initial-branch", branch]);
    }

    let init_cmd_child = start_child_process(init_cmd, GIT_PATH.as_str(), false).await?;
    handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        init_cmd_child,
        false,
        worker_name,
        w_id,
        "git init",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;

    let mut add_remote_cmd = Command::new(GIT_PATH.as_str());
    add_remote_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .env("PATH", PATH_ENV.as_str())
        .env("TZ", TZ_ENV.as_str())
        .env("GIT_SSH_COMMAND", git_ssh_cmd)
        .arg("-C")
        .arg(&target_path)
        .args(vec!["remote", "add", "origin", &repo.url])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let add_remote_cmd_child =
        start_child_process(add_remote_cmd, GIT_PATH.as_str(), false).await?;
    handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        add_remote_cmd_child,
        false,
        worker_name,
        w_id,
        "git add remote",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;

    let mut fetch_cmd = Command::new(GIT_PATH.as_str());
    fetch_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .env("PATH", PATH_ENV.as_str())
        .env("TZ", TZ_ENV.as_str())
        .env("GIT_SSH_COMMAND", git_ssh_cmd)
        .arg("-C")
        .arg(&target_path)
        .args(vec!["fetch", "--depth=1", "--quiet", "origin", full_commit])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let fetch_cmd_child = start_child_process(fetch_cmd, GIT_PATH.as_str(), false).await?;
    handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        fetch_cmd_child,
        false,
        worker_name,
        w_id,
        "git fetch",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;

    let mut checkout_cmd = Command::new(GIT_PATH.as_str());
    checkout_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .env("PATH", PATH_ENV.as_str())
        .env("TZ", TZ_ENV.as_str())
        .env("GIT_SSH_COMMAND", git_ssh_cmd)
        .arg("-C")
        .arg(&target_path)
        .args(["checkout", "--quiet", "FETCH_HEAD"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let checkout_cmd_child = start_child_process(checkout_cmd, GIT_PATH.as_str(), false).await?;
    handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        checkout_cmd_child,
        false,
        worker_name,
        w_id,
        "git checkout",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;

    Ok(())
}
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
                    PyVAlias::Py311.into(),
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
            split_python_requirements(requirements),
            job_id,
            w_id,
            mem_peak,
            canceled_by,
            conn,
            worker_name,
            job_dir,
            worker_dir,
            &mut Some(occupancy_metrics),
            PyVAlias::default().into(),
            None,
        )
        .await?;
        additional_python_paths.append(&mut venv_path);
    }
    Ok(additional_python_paths)
}

pub async fn install_galaxy_collections(
    collections_yml: &str,
    job_dir: &str,
    job_id: &Uuid,
    worker_name: &str,
    w_id: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    conn: &Connection,
    occupancy_metrics: &mut OccupancyMetrics,
    git_ssh_cmd: &str,
) -> anyhow::Result<()> {
    write_file(job_dir, "requirements.yml", collections_yml)?;

    append_logs(
        job_id,
        w_id,
        "\n\n--- ANSIBLE GALAXY INSTALL ---\n".to_string(),
        conn,
    )
    .await;

    let mut galaxy_roles_cmd = Command::new(ANSIBLE_GALAXY_PATH.as_str());
    galaxy_roles_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .env("PATH", PATH_ENV.as_str())
        .env("TZ", TZ_ENV.as_str())
        .env("GIT_SSH_COMMAND", git_ssh_cmd)
        .args(vec![
            "role",
            "install",
            "-r",
            "requirements.yml",
            "-p",
            "./roles",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = start_child_process(galaxy_roles_cmd, ANSIBLE_GALAXY_PATH.as_str(), false).await?;
    handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        child,
        is_sandboxing_enabled(),
        worker_name,
        w_id,
        "ansible-galaxy role install",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;

    let mut galaxy_collections_cmd = Command::new(ANSIBLE_GALAXY_PATH.as_str());
    galaxy_collections_cmd
        .current_dir(job_dir)
        .env_clear()
        .envs(PROXY_ENVS.clone())
        .env("PATH", PATH_ENV.as_str())
        .env("TZ", TZ_ENV.as_str())
        .env("GIT_SSH_COMMAND", git_ssh_cmd)
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

    let child =
        start_child_process(galaxy_collections_cmd, ANSIBLE_GALAXY_PATH.as_str(), false).await?;
    handle_child(
        job_id,
        conn,
        mem_peak,
        canceled_by,
        child,
        is_sandboxing_enabled(),
        worker_name,
        w_id,
        "ansible-galaxy collection install",
        None,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;

    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct AnsibleDependencyLocks {
    pub python_lockfile: String,
    pub git_repos: HashMap<String, String>, // URL to full commit hash
    pub collections_and_roles: String,
    pub collections_and_roles_logs: String,
    // pub collection_versions: HashMap<String, String>, //
    // pub role_versions: HashMap<String, String>,
}

pub async fn get_collection_locks(
    job_dir: &str,
) -> anyhow::Result<(HashMap<String, String>, String)> {
    let mut ansible_cmd = Command::new(ANSIBLE_GALAXY_PATH.as_str());

    ansible_cmd
        .current_dir(job_dir)
        .args(["collection", "list", "--format", "json", "-p", "./"]);

    let output = ansible_cmd.output().await?;

    let mut ret = HashMap::new();
    let mut logs = String::new();

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)?;
        return Err(anyhow!(
            "Error getting ansible collection versions: {stderr}"
        ));
    }

    let stdout = String::from_utf8(output.stdout)?;

    let val: serde_json::Value = serde_json::from_str(&stdout)?;

    let Some(own_collections) = val.get(format!("{}/ansible_collections", job_dir)) else {
        return Ok((ret, logs));
    };

    let collections = own_collections.as_object().ok_or(anyhow!(
        "Expected an object (map) for the `ansible-galaxy collection list` command output and got {}",
        own_collections
    ))?;

    for (c_name, c) in collections.iter() {
        if let Some(v) = c.get("version").and_then(|v| v.as_str()) {
            // TODO: Check if version is not something like `(undefined)`
            ret.insert(c_name.clone(), v.to_string());
        } else {
            logs.push_str(&format!("Failed to get version for collection `{}`. Expected an object with a string in the `version` field but received {}\n", c_name, c));
        }
    }

    Ok((ret, logs))
}

pub async fn get_role_locks(job_dir: &str) -> anyhow::Result<(HashMap<String, String>, String)> {
    let mut ansible_cmd = Command::new(ANSIBLE_GALAXY_PATH.as_str());

    ansible_cmd
        .current_dir(job_dir)
        .args(["role", "list", "-p", "./roles"]);

    let output = ansible_cmd.output().await?;
    let mut ret = HashMap::new();
    let mut logs = String::new();

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)?;
        logs.push_str(&format!("Error getting ansible role versions: {stderr}"));
        return Ok((ret, logs));
    }

    let stdout = String::from_utf8(output.stdout)?;

    let mut lines = stdout.lines();

    while let Some(line) = lines.next() {
        if line == format!("# {}/roles", job_dir) {
            break;
        }
    }

    for line in lines {
        let line = line.strip_prefix("-").unwrap_or(line);
        let mut cols = line.split(",");

        if let Some(name) = cols.next().map(|n| n.trim()) {
            if let Some(version) = cols.next().map(|v| v.trim()) {
                // TODO: Check if version is not something like `(undefined)`
                ret.insert(name.to_string(), version.to_string());
            } else {
                logs.push_str(&format!("Failed to get version for role `{}`.", name));
            }
        }
    }

    Ok((ret, logs))
}

pub async fn get_git_repo_full_head_commit_hash(
    repo: &GitRepo,
    git_ssh_cmd: &str,
) -> anyhow::Result<String> {
    let mut git_cmd = Command::new(GIT_PATH.as_str());

    git_cmd
        .env("GIT_SSH_COMMAND", git_ssh_cmd)
        .args(["ls-remote", &repo.url, "HEAD"]);

    let output = git_cmd.stderr(Stdio::piped()).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)?;
        return Err(anyhow!("Error getting git repo commit hash: {stderr}"));
    }

    let stdout = String::from_utf8(output.stdout)?;

    let lines: Vec<&str> = stdout.lines().collect();

    if lines.len() != 1 {
        return Err(anyhow!("Unexpected output format for git ls-remote",));
    }

    Ok(lines
        .first()
        .ok_or(anyhow!(
            "The HEAD commit hash was not found for repo `{}`",
            &repo.url
        ))?
        .split_whitespace()
        .next()
        .map(|s| s.to_string())
        .ok_or(anyhow!("Unexpected output format for git ls-remote"))?)
}

pub async fn get_git_repos_lock(
    repos: &Vec<GitRepo>,
    job_dir: &str,
    job_id: &Uuid,
    worker_name: &str,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    w_id: &str,
    occupancy_metrics: &mut OccupancyMetrics,
    git_ssh_cmd: &str,
) -> anyhow::Result<HashMap<String, String>> {
    let mut ret = HashMap::new();

    for repo in repos {
        if repo.commit.is_some() {
            ret.insert(
                repo.url.to_string(),
                clone_repo(
                    repo,
                    job_dir,
                    job_id,
                    worker_name,
                    conn,
                    mem_peak,
                    canceled_by,
                    w_id,
                    occupancy_metrics,
                    git_ssh_cmd,
                )
                .await?,
            );
        } else {
            ret.insert(
                repo.url.to_string(),
                get_git_repo_full_head_commit_hash(repo, git_ssh_cmd).await?,
            );
        }
    }

    Ok(ret)
}

pub fn create_ansible_cfg(
    reqs: Option<&AnsibleRequirements>,
    job_dir: &str,
    vault_password_file_exists: bool,
) -> error::Result<()> {
    let mut passwords_cfg = String::new();
    if vault_password_file_exists {
        passwords_cfg.push_str(&format!(
            "vault_password_file = {WINDMILL_ANSIBLE_PASSWORD_FILENAME}\n"
        ));
    }
    if let Some(vault_ids) = reqs.as_ref().map(|r| &r.vault_id) {
        if !vault_ids.is_empty() {
            let password_files = vault_ids.join(",");

            passwords_cfg.push_str(&format!("vault_identity_list = {password_files}\n"));
        }
    }
    let ansible_cfg_content = format!(
        r#"
[defaults]
collections_path = ./
roles_path = ./roles
home={job_dir}/.ansible
local_tmp={job_dir}/.ansible/tmp
remote_tmp={job_dir}/.ansible/tmp
{passwords_cfg}
"#
    );

    write_file(job_dir, "ansible.cfg", &ansible_cfg_content)?;

    Ok(())
}

pub async fn get_git_ssh_cmd(
    reqs: &AnsibleRequirements,
    job_dir: &str,
    client: &AuthedClient,
) -> error::Result<String> {
    let ssh_id_files = try_join_all(reqs.git_ssh_identity.iter().enumerate().map(
        async |(i, var_path)| -> error::Result<String> {
            let id_file_name = format!(".ssh_id_priv_{}", i);
            let loc = is_allowed_file_location(job_dir, &id_file_name)?;

            let mut content = client.get_variable_value(var_path).await.map_err(|e| {
                error::Error::NotFound(format!(
                    "Variable {var_path} not found for git ssh identity: {e:#}"
                ))
            })?;
            content.push_str("\n");

            #[cfg(not(unix))]
            let _ = write_file(job_dir, &id_file_name, &content)?;

            #[cfg(unix)]
            {
                let file = write_file(job_dir, &id_file_name, &content)?;
                let perm = std::os::unix::fs::PermissionsExt::from_mode(0o600);
                file.set_permissions(perm)?;
            }

            Ok(format!(
                " -i '{}'",
                loc.to_string_lossy().replace('\'', r"'\''")
            ))
        },
    ))
    .await?;

    let git_ssh_cmd = format!("ssh -o StrictHostKeyChecking=no{}", ssh_id_files.join(""));
    Ok(git_ssh_cmd)
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

    let req_lockfiles: Option<AnsibleDependencyLocks> = if let Some(s) = requirements_o {
        if let Ok(lockfile) = serde_json::from_str(s) {
            Some(lockfile)
        } else {
            if !s.trim_start().starts_with('{') {
                append_logs(
                    &job.id,
                    &job.workspace_id,
                    format!("WARN: lockfile seems to be in an older version, roles and collections are therefore using the latest version and not the one locked at deployment. Redeploy the script to correct this"),
                    conn,
                )
                .await;
                Some(AnsibleDependencyLocks {
                    python_lockfile: s.to_string(),
                    git_repos: HashMap::new(),
                    collections_and_roles: String::new(),
                    collections_and_roles_logs: String::new(),
                })
            } else {
                append_logs(
                    &job.id,
                    &job.workspace_id,
                    format!("WARN: lockfile could not be parsed: {s}"),
                    conn,
                )
                .await;
                None
            }
        }
    } else {
        None
    };

    let (logs, reqs, playbook) = windmill_parser_yaml::parse_ansible_reqs(inner_content)?;
    append_logs(&job.id, &job.workspace_id, logs, conn).await;
    write_file(job_dir, "main.yml", &playbook)?;

    let additional_python_paths = handle_ansible_python_deps(
        job_dir,
        req_lockfiles.as_ref().map(|r| &r.python_lockfile),
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

    let git_ssh_cmd = &match &reqs {
        Some(r) => get_git_ssh_cmd(r, job_dir, client).await?,
        None => "ssh".to_string(),
    };

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
        .map(|x| -> Result<Vec<String>, _> {
            let mut ret: Vec<String> = x
                .inventories
                .clone()
                .iter()
                .flat_map(|i| vec!["-i".to_string(), i.name.clone()].into_iter())
                .collect();

            let additional: Vec<String> = x
                .additional_inventories
                .iter()
                .map(|i| match i {
                    PreexistingAnsibleInventory::Static(name) => Ok(Some(vec![name.clone()])),
                    PreexistingAnsibleInventory::PassedInArgs(inv_def) => interpolated_args
                        .as_ref()
                        .and_then(|args| args.get(&inv_def.name))
                        .and_then(|v| serde_json::from_str(v.get()).transpose())
                        .transpose(),
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .flatten()
                .flat_map(|name| vec!["-i".to_string(), name])
                .collect();

            ret.extend(additional);
            Ok::<_, windmill_common::error::Error>(ret)
        })
        .transpose()?
        .unwrap_or_else(|| vec![]);

    let mut nsjail_extra_mounts = vec![];
    let mut playbook_override = None;

    if let Some(r) = reqs.as_ref() {
        nsjail_extra_mounts = create_file_resources(
            &job.id,
            &job.workspace_id,
            job_dir,
            interpolated_args.as_ref(),
            &r,
            &client,
            conn,
        )
        .await?;

        if let Some(delegated_git_repo) = r.delegate_to_git_repo.as_ref() {
            let serde_json::Value::Object(git_repo_resource) = client
                .get_resource_value_interpolated::<serde_json::Value>(
                    &delegated_git_repo.resource,
                    Some(job.id.to_string()),
                )
                .await?
            else {
                return Err(windmill_common::error::Error::BadRequest(
                    "Git repository resource is not an object".to_string(),
                ));
            };

            let mut secret_url = git_repo_resource.get("url").and_then(|s| s.as_str()).map(|s| s.to_string())
                .ok_or(anyhow!("Failed to get url from git repo resource, please check that the resource has the correct type (git_repository)"))?;

            #[cfg(feature = "enterprise")]
            let is_github_app = git_repo_resource.get("is_github_app").and_then(|s| s.as_bool())
                .ok_or(anyhow!("Failed to get `is_github_app` field from git repo resource, please check that the resource has the correct type (git_repository)"))?;

            #[cfg(feature = "enterprise")]
            if is_github_app {
                if let Connection::Sql(db) = conn {
                    let token = windmill_common::git_sync_oss::get_github_app_token_internal(
                        db,
                        &client.token,
                    )
                    .await?;
                    secret_url = prepend_token_to_github_url(&secret_url, &token)?;
                } else {
                    return Err(windmill_common::error::Error::BadRequest("Github App authentication is currently unavailable for agent workers. Contact the windmill team to request this feature".to_string()));
                }
            }

            let branch = Some(git_repo_resource.get("branch").and_then(|s| s.as_str()).map(|s| s.to_string())
                .ok_or(anyhow!("Failed to get branch from git repo resource, please check that the resource has the correct type (git_repository)"))?).filter(|s| !s.is_empty());

            let target_path = "delegate_git_repository".to_string();

            let repo = GitRepo {
                url: secret_url,
                commit: delegated_git_repo.commit.clone(),
                branch,
                target_path,
            };
            append_logs(
                &job.id,
                &job.workspace_id,
                format!("\nCloning {}...\n", delegated_git_repo.resource),
                conn,
            )
            .await;
            if let Some(commit) = delegated_git_repo.commit.as_ref() {
                clone_repo_without_history(
                    &repo,
                    commit,
                    job_dir,
                    &job.id,
                    worker_name,
                    conn,
                    mem_peak,
                    canceled_by,
                    &job.workspace_id,
                    occupancy_metrics,
                    git_ssh_cmd,
                )
                .await
                .map_err(|e| anyhow!("Failed to clone git repo `{}`: {e}", repo.url))?;
            } else {
                clone_repo(
                    &repo,
                    job_dir,
                    &job.id,
                    worker_name,
                    conn,
                    mem_peak,
                    canceled_by,
                    &job.workspace_id,
                    occupancy_metrics,
                    git_ssh_cmd,
                )
                .await
                .map_err(|e| anyhow!("Failed to clone git repo `{}`: {e}", repo.url))?;
            }

            append_logs(
                &job.id,
                &job.workspace_id,
                format!(
                    "Cloned {} into {}\n",
                    delegated_git_repo.resource, &repo.target_path
                ),
                conn,
            )
            .await;

            playbook_override = Some(
                delegated_git_repo
                    .playbook
                    .as_ref()
                    .map(|p| format!("{}/{}", &repo.target_path, p)),
            );
        }

        if playbook_override.clone().flatten().is_none() && playbook.is_empty() {
            return Err(windmill_common::error::Error::BadRequest("No playbook was specified. Append a playbook to your script or specify one in the delegate_to_git_repo -> playbook section.".to_string()));
        }

        for repo in &r.git_repos {
            append_logs(
                &job.id,
                &job.workspace_id,
                format!("\nCloning {}...\n", &repo.url),
                conn,
            )
            .await;
            if let Some(full_commit_hash) = req_lockfiles
                .as_ref()
                .and_then(|r| r.git_repos.get(&repo.url))
            {
                clone_repo_without_history(
                    repo,
                    full_commit_hash,
                    job_dir,
                    &job.id,
                    worker_name,
                    conn,
                    mem_peak,
                    canceled_by,
                    &job.workspace_id,
                    occupancy_metrics,
                    git_ssh_cmd,
                )
                .await
                .map_err(|e| anyhow!("Failed to clone git repo `{}`: {e}", repo.url))?;
            } else {
                if req_lockfiles.is_some() {
                    append_logs(
                        &job.id,
                        &job.workspace_id,
                        format!("Warning: `{}` is using latest commit because the lockfile didn't store a commit hash for this repo. Updates to the repo could break the deployed playbook.\n", &repo.url),
                        conn,
                    )
                    .await;
                }
                clone_repo(
                    repo,
                    job_dir,
                    &job.id,
                    worker_name,
                    conn,
                    mem_peak,
                    canceled_by,
                    &job.workspace_id,
                    occupancy_metrics,
                    git_ssh_cmd,
                )
                .await
                .map_err(|e| anyhow!("Failed to clone git repo `{}`: {e}", repo.url))?;
            }

            append_logs(
                &job.id,
                &job.workspace_id,
                format!("Cloned {} into {}\n", &repo.url, &repo.target_path),
                conn,
            )
            .await;
        }

        if let Some(collections) = r.roles_and_collections.as_ref() {
            let empty = String::new();
            let (lockfile, logs) = req_lockfiles
                .as_ref()
                .and_then(|r| {
                    if r.collections_and_roles.is_empty() {
                        None
                    } else {
                        Some((&r.collections_and_roles, &r.collections_and_roles_logs))
                    }
                })
                .unwrap_or((collections, &empty));

            if !logs.is_empty() {
                append_logs(&job.id, &job.workspace_id, logs, conn).await;
            }

            install_galaxy_collections(
                lockfile,
                job_dir,
                &job.id,
                worker_name,
                &job.workspace_id,
                mem_peak,
                canceled_by,
                conn,
                occupancy_metrics,
                git_ssh_cmd,
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

    let vault_password_file_exists = match reqs.as_ref().and_then(|x| x.vault_password.as_ref()) {
        Some(var_path) => {
            let password = client.get_variable_value(&var_path).await?;
            write_file(job_dir, WINDMILL_ANSIBLE_PASSWORD_FILENAME, &password)?;
            true
        }
        None => false,
    };

    create_ansible_cfg(reqs.as_ref(), job_dir, vault_password_file_exists)?;

    let mut reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;
    let additional_python_paths_folders = additional_python_paths.join(":");

    if is_sandboxing_enabled() {
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

    let playbook = playbook_override
        .flatten()
        .unwrap_or("main.yml".to_string());
    let mut cmd_args = vec![playbook.as_str(), "--extra-vars", "@args.json"];
    cmd_args.extend(inventories.iter().map(|s| s.as_str()));
    cmd_args.extend(cmd_options.iter().map(|s| s.as_str()));

    let child = if is_sandboxing_enabled() {
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

        let _file = write_file(job_dir, "wrapper.sh", &wrapper)?;

        #[cfg(unix)]
        _file.metadata()?.permissions().set_mode(0o777);
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
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        let ansible_args: Vec<&str> = cmd_args.iter().map(|s| s.as_ref()).collect();
        let mut ansible_cmd =
            build_command_with_isolation(ANSIBLE_PLAYBOOK_PATH.as_str(), &ansible_args);
        ansible_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        ansible_cmd.env("USERPROFILE", crate::USERPROFILE_ENV.as_str());

        start_child_process(ansible_cmd, ANSIBLE_PLAYBOOK_PATH.as_str(), false).await?
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
        "python run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
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
    client: &AuthedClient,
    conn: &Connection,
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
    append_logs(job_id, w_id, logs, conn).await;

    Ok(nsjail_mounts)
}

async fn get_resource_or_variable_content(
    client: &AuthedClient,
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
