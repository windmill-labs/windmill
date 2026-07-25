//! Git clone helpers shared by the runtimes that execute code from an external
//! repo (Ansible playbooks, dbt projects). Nothing here is runtime-specific:
//! given a `GitRepo` and an SSH command, clone it into the job dir and report
//! the commit that was checked out.

use std::path::PathBuf;
use std::process::Stdio;

use anyhow::anyhow;
use tokio::process::Command;
use uuid::Uuid;
use windmill_common::error;
use windmill_common::git_sync_oss::sanitize_git_url;
use windmill_common::worker::{is_allowed_file_location, Connection};
use windmill_parser_yaml::GitRepo;
use windmill_queue::CanceledBy;

use crate::common::{start_child_process, OccupancyMetrics};
use crate::handle_child::handle_child;
use crate::{GIT_PATH, PATH_ENV, PROXY_ENVS, TZ_ENV};

pub async fn clone_repo(
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

pub async fn clone_repo_without_history(
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
            sanitize_git_url(&repo.url)
        ))?
        .split_whitespace()
        .next()
        .map(|s| s.to_string())
        .ok_or(anyhow!("Unexpected output format for git ls-remote"))?)
}
