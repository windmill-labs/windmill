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
    git_sync_oss::{prepend_token_to_github_url, sanitize_git_url},
    worker::{
        is_allowed_file_location, split_python_requirements, to_raw_value, write_file,
        write_file_at_user_defined_location, Connection, PyVAlias, WORKER_CONFIG,
    },
};
use windmill_queue::MiniPulledJob;

use windmill_parser_yaml::{
    validate_vault_id, AnsibleRequirements, GitRepo, PreexistingAnsibleInventory,
    ResourceOrVariablePath,
};
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    bash_executor::BIN_BASH,
    common::{
        build_command_with_isolation, check_executor_binary_exists, get_reserved_variables,
        read_and_check_result, render_nsjail_rlimit_as, resolve_nsjail_timeout,
        resolve_nsjail_tmp_mount_block, start_child_process, transform_json, OccupancyMetrics,
    },
    handle_child::handle_child,
    is_sandboxing_enabled,
    python_executor::{create_dependencies_dir, handle_python_reqs, uv_pip_compile},
    DISABLE_NUSER, GIT_PATH, HOME_ENV, NSJAIL_ANSIBLE_RLIMIT_AS_MB, NSJAIL_PATH, PATH_ENV,
    PROXY_ENVS, PY_INSTALL_DIR, TZ_ENV,
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

const DELEGATE_GIT_REPO_TARGET: &str = "delegate_git_repository";

/// Usable bytes in `sockaddr_un.sun_path` (108 minus the NUL). An ABI constant, not a
/// filesystem limit — which is why only the socket breaks while every regular file in the
/// same job dir is fine.
const AF_UNIX_PATH_LIMIT: usize = 107;

/// Root for the per-job dir in which ansible's persistent-connection plugins
/// (`network_cli`, `httpapi`, `netconf`) bind their unix socket, named after a digest of the
/// connection. `sockaddr_un.sun_path` caps the whole socket path at [`AF_UNIX_PATH_LIMIT`],
/// which the job dir alone already exhausts, so the socket dir must stay short and cannot
/// live under `ANSIBLE_HOME` (which Windmill pins into the job dir).
///
/// Fixed, and directly under `/tmp`, for two reasons that are easy to undo by accident:
/// `/tmp`'s sticky bit is what stops another uid renaming our root away, the one property
/// [`prepare_socket_root`] needs from a parent; and every component of a fixed path is one
/// nobody can point elsewhere, so trusting the root does not mean trusting an ancestor
/// chain. Notably NOT under `WINDMILL_DIR`: the shipped image chmods that tree to a
/// non-sticky 0777 so any UID can write it (`Dockerfile`, "Make directories
/// world-accessible for any UID"), which is exactly the parent an attacker can swap entries
/// in.
const PERSISTENT_CONTROL_PATH_ROOT: &str = "/tmp/wm-pc";

/// The budget this whole change exists to protect: root + `/` + a 32-char job uuid + `/` +
/// a socket name, allowing a full 40-char sha1 (ansible truncates it far shorter today, but
/// a custom control path may not).
const _: () = assert!(PERSISTENT_CONTROL_PATH_ROOT.len() + 1 + 32 + 1 + 40 <= AF_UNIX_PATH_LIMIT);

/// Cleared when the root cannot be trusted (see [`prepare_socket_root`]), which makes jobs
/// stop naming it and fall back to ansible's own `{ANSIBLE_HOME}/pc` default — inside the
/// job dir, so worker-owned. Network playbooks then fail on the path length as they did
/// before this dir existed, which beats handing an attacker the socket a device session
/// runs over. Defaults to trusted: the check runs at worker start, before any job.
static SOCKET_ROOT_TRUSTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);

/// Socket dir for `job_id`, or `None` when the root is untrusted. Per-job on purpose:
/// socket names hash host+credentials, so concurrent jobs sharing a dir would reuse each
/// other's connection daemon.
fn persistent_control_path_dir(job_id: &Uuid) -> Option<String> {
    SOCKET_ROOT_TRUSTED
        .load(std::sync::atomic::Ordering::Relaxed)
        .then(|| format!("{PERSISTENT_CONTROL_PATH_ROOT}/{}", job_id.simple()))
}

/// Whether `name` is one this module could have created, i.e. `Uuid::simple` (32 hex, no
/// hyphens). Belt to the root check's braces: nothing else should ever be in there.
fn is_persistent_control_path_dir_name(name: &str) -> bool {
    name.len() == 32 && Uuid::try_parse(name).is_ok()
}

/// Removes the job's socket dir on the way out. It lives outside `job_dir`, so the
/// worker's job-dir sweep does not cover it.
struct PersistentControlPathGuard(String);

impl Drop for PersistentControlPathGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Claim the socket-dir root at worker start, then reap dirs left behind by workers that
/// died before their guard could run.
#[cfg(unix)]
pub async fn prepare_persistent_control_path_root() {
    // A play holds its socket dir for as long as it runs, touching the mtime only when
    // connections open, so anything younger than the longest permitted job may still be
    // live — including on another worker sharing this host.
    let stale_after = std::time::Duration::from_secs(
        windmill_common::worker::MAX_TIMEOUT.saturating_add(24 * 60 * 60),
    );
    prepare_socket_root(PERSISTENT_CONTROL_PATH_ROOT, stale_after).await
}

/// Reject a root that another local user could control, and mark it untrusted so jobs stop
/// naming it. Returns without sweeping in that case.
///
/// SECURITY: the root sits in a world-writable `/tmp`, so a local user who wins the race to
/// create it owns the parent of every job's socket dir —
/// enough to hand ansible a socket of their choosing (a device session, credentials and
/// all, runs over it), or to swap in a symlink and redirect the sweep's path-based
/// `remove_dir_all` onto a target of their choosing, as the worker's uid. Three things must
/// hold: the root is a real directory (`symlink_metadata` reports the link's own type
/// without following it, so `is_dir()` cannot be satisfied by a symlink), we own it and
/// nobody else can write it, and its parent cannot be used to replace it — which needs the
/// parent either not writable by others, or sticky, since the sticky bit is exactly what
/// stops a non-owner renaming an entry out of a shared dir. The root is validated after the
/// create attempt, never before: anything else races whoever creates it first.
#[cfg(unix)]
async fn prepare_socket_root(root: &str, stale_after: std::time::Duration) {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let untrusted = |reason: String| {
        tracing::error!(
            "Refusing to use the ansible persistent-connection socket root at {root}: {reason}. \
             Ansible network playbooks on this host will keep failing with `AF_UNIX path too \
             long` until this is resolved."
        );
        SOCKET_ROOT_TRUSTED.store(false, std::sync::atomic::Ordering::Relaxed);
    };

    if let Some(parent) = std::path::Path::new(root).parent() {
        // The worker's own dir, but a fresh install may not have created it yet.
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            return untrusted(format!("its parent {} is unusable: {e}", parent.display()));
        }
        // Resolved, not `symlink_metadata`: what matters is the mode of the directory the
        // entries actually live in, and a symlinked parent is normal (macOS `/tmp`).
        match tokio::fs::metadata(parent).await {
            Ok(meta) => {
                let mode = meta.permissions().mode();
                if mode & 0o022 != 0 && mode & 0o1000 == 0 {
                    return untrusted(format!(
                        "its parent {} is writable by other users and not sticky (mode={:o}), \
                         so they could replace the root",
                        parent.display(),
                        mode & 0o7777
                    ));
                }
            }
            Err(e) => return untrusted(format!("cannot stat its parent: {e}")),
        }
    }

    // Non-recursive on purpose: `recursive` reports success for a path that already
    // exists, which under a sticky parent (where others may still *create* the
    // not-yet-existing `pc`, only not rename ours away) would hand us whatever another uid
    // raced into place. Create-or-EEXIST, then validate whatever is actually there.
    match tokio::fs::DirBuilder::new().mode(0o700).create(root).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return untrusted(format!("it could not be created: {e}")),
    }

    match tokio::fs::symlink_metadata(root).await {
        Ok(meta) if meta.is_dir() => {
            let mode = meta.permissions().mode();
            // Trusted means usable as well as safe: without owner rwx ansible cannot create
            // its per-job dir, and a trusted-but-unusable root would hand every network
            // playbook a permission error instead of the working fallback.
            if meta.uid() != nix::unistd::Uid::effective().as_raw()
                || mode & 0o022 != 0
                || mode & 0o700 != 0o700
            {
                return untrusted(format!(
                    "it is not owned by this worker, is writable by others, or is not \
                     writable by us (uid={}, mode={:o})",
                    meta.uid(),
                    mode & 0o7777
                ));
            }
        }
        Ok(_) => {
            return untrusted(
                "it is not a directory (possibly a symlink planted by another local user)"
                    .to_string(),
            )
        }
        Err(e) => return untrusted(format!("it could not be stat'd: {e}")),
    }

    let Ok(mut entries) = tokio::fs::read_dir(root).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        // Only reap what we could have created. Nothing else should ever be in a root we
        // made 0700 ourselves, but this is a recursive delete running as the worker's uid:
        // cheap to bound by name, expensive to get wrong.
        if !entry
            .file_name()
            .to_str()
            .is_some_and(is_persistent_control_path_dir_name)
        {
            continue;
        }
        // `DirEntry::metadata` does not traverse symlinks, so a planted link is never
        // followed here either.
        let stale = match entry.metadata().await.and_then(|m| m.modified()) {
            Ok(modified) => modified.elapsed().is_ok_and(|e| e > stale_after),
            Err(_) => false,
        };
        if stale {
            let _ = tokio::fs::remove_dir_all(entry.path()).await;
        }
    }
}

lazy_static::lazy_static! {
    static ref TEMPLATE_RE: regex::Regex = regex::Regex::new(r"\{\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}\}").unwrap();
}

/// Substitute `{{ arg_name }}` placeholders with values from `args`.
/// Strings are used raw; numbers/bools are stringified. Other types are rejected.
fn interpolate_template(
    template: &str,
    args: Option<&HashMap<String, Box<RawValue>>>,
    field_name: &str,
) -> error::Result<String> {
    let mut last_err: Option<error::Error> = None;
    let result = TEMPLATE_RE.replace_all(template, |caps: &regex::Captures| {
        let name = &caps[1];
        let raw = args.and_then(|a| a.get(name));
        let Some(raw) = raw else {
            last_err = Some(error::Error::BadRequest(format!(
                "`{}` references `{{{{ {} }}}}` but no such argument was provided",
                field_name, name
            )));
            return String::new();
        };
        let json: serde_json::Value = match serde_json::from_str(raw.get()) {
            Ok(v) => v,
            Err(e) => {
                last_err = Some(error::Error::BadRequest(format!(
                    "`{}` could not parse argument `{}` as JSON: {e}",
                    field_name, name
                )));
                return String::new();
            }
        };
        match json {
            serde_json::Value::String(s) => s,
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => {
                last_err = Some(error::Error::BadRequest(format!(
                    "`{}` references `{{{{ {} }}}}` but the argument is null",
                    field_name, name
                )));
                String::new()
            }
            _ => {
                last_err = Some(error::Error::BadRequest(format!(
                    "`{}` references `{{{{ {} }}}}` but the argument is not a primitive (string/number/bool)",
                    field_name, name
                )));
                String::new()
            }
        }
    });
    if let Some(e) = last_err {
        return Err(e);
    }
    Ok(result.into_owned())
}

/// Reject absolute paths and `..` segments to prevent escaping the cloned repo directory.
fn validate_relative_path(path: &str, field_name: &str) -> error::Result<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(error::Error::BadRequest(format!(
            "`{}` resolved to an empty path",
            field_name
        )));
    }
    let p = std::path::Path::new(trimmed);
    for component in p.components() {
        match component {
            // RootDir catches leading `/` or `\`; Prefix catches Windows drive
            // letters and UNC paths. `Path::is_absolute()` alone misses
            // RootDir-only paths on Windows (e.g. `/etc/passwd`).
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(error::Error::BadRequest(format!(
                    "`{}` must be a relative path inside the cloned repo, got: {}",
                    field_name, trimmed
                )));
            }
            std::path::Component::ParentDir => {
                return Err(error::Error::BadRequest(format!(
                    "`{}` must not contain `..` segments, got: {}",
                    field_name, trimmed
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

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
        .load()
        .additional_python_paths
        .clone()
        .unwrap_or_else(|| vec![]);

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

    run_galaxy_install_from_requirements(
        "requirements.yml",
        job_dir,
        job_id,
        worker_name,
        w_id,
        mem_peak,
        canceled_by,
        conn,
        occupancy_metrics,
        git_ssh_cmd,
    )
    .await
}

/// Run `ansible-galaxy role install -r <path>` then `ansible-galaxy collection install -r <path>`.
/// `requirements_path` is relative to `job_dir`.
async fn run_galaxy_install_from_requirements(
    requirements_path: &str,
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
            requirements_path,
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
            requirements_path,
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

/// Look for `requirements.yml`, `collections/requirements.yml`, and `roles/requirements.yml`
/// inside a cloned repo (relative to `job_dir`) and run ansible-galaxy install on each one found.
async fn install_requirements_from_cloned_repo(
    repo_target: &str,
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
    let candidates = [
        "requirements.yml",
        "requirements.yaml",
        "collections/requirements.yml",
        "collections/requirements.yaml",
        "roles/requirements.yml",
        "roles/requirements.yaml",
    ];
    let mut found: Vec<String> = vec![];
    for candidate in candidates {
        let abs = std::path::Path::new(job_dir)
            .join(repo_target)
            .join(candidate);
        if abs.is_file() {
            found.push(format!("{}/{}", repo_target, candidate));
        }
    }

    if found.is_empty() {
        append_logs(
            job_id,
            w_id,
            format!(
                "\nNo requirements.yml found in `{}`, skipping repo dependency install.\n",
                repo_target
            ),
            conn,
        )
        .await;
        return Ok(());
    }

    append_logs(
        job_id,
        w_id,
        format!(
            "\n\n--- INSTALLING REPO REQUIREMENTS ({}) ---\n",
            found.join(", ")
        ),
        conn,
    )
    .await;

    for path in &found {
        run_galaxy_install_from_requirements(
            path,
            job_dir,
            job_id,
            worker_name,
            w_id,
            mem_peak,
            canceled_by,
            conn,
            occupancy_metrics,
            git_ssh_cmd,
        )
        .await?;
    }

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
            sanitize_git_url(&repo.url)
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
    job_id: &Uuid,
) -> error::Result<()> {
    let mut passwords_cfg = String::new();
    if vault_password_file_exists {
        passwords_cfg.push_str(&format!(
            "vault_password_file = {WINDMILL_ANSIBLE_PASSWORD_FILENAME}\n"
        ));
    }
    if let Some(vault_ids) = reqs.as_ref().map(|r| &r.vault_id) {
        if !vault_ids.is_empty() {
            // Defense in depth: entries are validated at parse time, but re-check here
            // since they are interpolated raw into ansible.cfg (config-directive injection).
            for vault_id in vault_ids {
                validate_vault_id(vault_id)?;
            }
            let password_files = vault_ids.join(",");

            passwords_cfg.push_str(&format!("vault_identity_list = {password_files}\n"));
        }
    }
    let persistent_cfg = persistent_control_path_dir(job_id)
        .map(|dir| format!("[persistent_connection]\ncontrol_path_dir = {dir}\n"))
        .unwrap_or_default();
    let ansible_cfg_content = format!(
        r#"
[defaults]
collections_path = ./
roles_path = ./roles
home={job_dir}/.ansible
local_tmp={job_dir}/.ansible/tmp
remote_tmp={job_dir}/.ansible/tmp
{passwords_cfg}
{persistent_cfg}"#
    );

    write_file(job_dir, "ansible.cfg", &ansible_cfg_content)?;

    Ok(())
}

/// The section a header line opens, if it is one. Mirrors configparser's `SECTCRE`
/// (`\[(?P<header>.+)\]`, matched not fullmatched, `.+` greedy): the name runs to the
/// *last* `]`, and anything after it — an inline comment, say — is ignored.
fn parse_ansible_cfg_section_header(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix('[')?;
    let end = rest.rfind(']')?;
    Some(rest[..end].trim())
}

/// Read a colon-separated path list (e.g. `roles_path`, `collections_path`) from
/// the `[defaults]` section of an ansible.cfg. Returns the raw entries as written,
/// unresolved. Deliberately minimal: no inline-comment or continuation handling,
/// which ansible's configparser also does not apply to these values. `key` and `=`
/// or `:` as the delimiter are both matched (Python configparser accepts either),
/// with the value itself split on `:` (`os.pathsep`).
fn parse_ansible_cfg_path_list(content: &str, key: &str) -> Option<Vec<String>> {
    let mut in_defaults = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(section) = parse_ansible_cfg_section_header(trimmed) {
            in_defaults = section.eq_ignore_ascii_case("defaults");
            continue;
        }
        if !in_defaults || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        // configparser delimits key/value on the first `=` or `:`, whichever
        // comes first; the remaining `:` in the value are path separators.
        let sep = trimmed.find('=').into_iter().chain(trimmed.find(':')).min();
        if let Some(sep) = sep {
            let (k, rest) = trimmed.split_at(sep);
            let v = &rest[1..];
            if k.trim().eq_ignore_ascii_case(key) {
                let entries: Vec<String> = v
                    .split(':')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
                return (!entries.is_empty()).then_some(entries);
            }
        }
    }
    None
}

/// Whether `section` declares `key` in an ansible.cfg. Same deliberately minimal
/// parsing as [`parse_ansible_cfg_path_list`], for a scalar key in a named section.
fn ansible_cfg_declares(content: &str, section: &str, key: &str) -> bool {
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(header) = parse_ansible_cfg_section_header(trimmed) {
            in_section = header.eq_ignore_ascii_case(section);
            continue;
        }
        if !in_section || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let sep = trimmed.find('=').into_iter().chain(trimmed.find(':')).min();
        if let Some(sep) = sep {
            if trimmed[..sep].trim().eq_ignore_ascii_case(key) {
                return true;
            }
        }
    }
    false
}

/// Prepend Windmill's dependency install dir to the repo cfg's declared path list.
/// Relative entries from the repo cfg are resolved against `cfg_dir` to match how
/// ansible resolves them relative to the config file's own directory.
fn resolve_and_prepend_path(
    base: String,
    repo_entries: Option<Vec<String>>,
    cfg_dir: &str,
) -> String {
    let mut paths = vec![base];
    if let Some(entries) = repo_entries {
        for e in entries {
            if e.starts_with('/') || e.starts_with('~') {
                paths.push(e);
            } else {
                paths.push(format!("{cfg_dir}/{e}"));
            }
        }
    }
    paths.join(":")
}

/// Build the environment overrides applied when delegating to a git repo that
/// ships its own ansible.cfg. See the call site for the layering rationale.
async fn build_ansible_cfg_override_envs(
    cfg_path: &str,
    job_dir: &str,
    vault_password_file_exists: bool,
    reqs: Option<&AnsibleRequirements>,
    job_id: &Uuid,
) -> error::Result<Vec<(String, String)>> {
    let mut envs = vec![
        ("ANSIBLE_CONFIG".to_string(), cfg_path.to_string()),
        // Runtime-bound: reference the ephemeral job dir, cannot be set statically.
        ("ANSIBLE_HOME".to_string(), format!("{job_dir}/.ansible")),
        (
            "ANSIBLE_LOCAL_TEMP".to_string(),
            format!("{job_dir}/.ansible/tmp"),
        ),
        (
            "ANSIBLE_REMOTE_TEMP".to_string(),
            format!("{job_dir}/.ansible/tmp"),
        ),
    ];

    // Vault: Windmill manages the secret, so its config wins over the repo cfg.
    if vault_password_file_exists {
        envs.push((
            "ANSIBLE_VAULT_PASSWORD_FILE".to_string(),
            format!("{job_dir}/{WINDMILL_ANSIBLE_PASSWORD_FILENAME}"),
        ));
    }
    if let Some(vault_ids) = reqs.map(|r| &r.vault_id).filter(|v| !v.is_empty()) {
        // Defense in depth: entries are validated at parse time, but re-check
        // here since they are interpolated raw into the env value.
        for vault_id in vault_ids {
            validate_vault_id(vault_id)?;
        }
        envs.push((
            "ANSIBLE_VAULT_IDENTITY_LIST".to_string(),
            vault_ids.join(","),
        ));
    }

    // Dependency search paths: additive. Windmill installs galaxy roles into
    // `{job_dir}/roles` and collections into `{job_dir}`; prepend those to the
    // repo cfg's declared paths so both Windmill-installed and repo deps resolve.
    let cfg_dir = std::path::Path::new(cfg_path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or(job_dir);
    let cfg_content = tokio::fs::read_to_string(cfg_path).await.map_err(|e| {
        windmill_common::error::Error::internal_err(format!(
            "Failed to read delegated ansible.cfg at `{cfg_path}`: {e}"
        ))
    })?;

    // Persistent-connection socket dir: only a default. Unlike ANSIBLE_HOME this value is
    // not runtime-bound, so a repo that picks its own dir keeps it.
    if !ansible_cfg_declares(&cfg_content, "persistent_connection", "control_path_dir") {
        if let Some(dir) = persistent_control_path_dir(job_id) {
            envs.push(("ANSIBLE_PERSISTENT_CONTROL_PATH_DIR".to_string(), dir));
        }
    }

    envs.push((
        "ANSIBLE_ROLES_PATH".to_string(),
        resolve_and_prepend_path(
            format!("{job_dir}/roles"),
            parse_ansible_cfg_path_list(&cfg_content, "roles_path"),
            cfg_dir,
        ),
    ));
    envs.push((
        "ANSIBLE_COLLECTIONS_PATH".to_string(),
        resolve_and_prepend_path(
            job_dir.to_string(),
            // Also probe the deprecated plural ini alias; env vars replace (not
            // merge) the cfg value, so a repo using it would otherwise be dropped.
            parse_ansible_cfg_path_list(&cfg_content, "collections_path")
                .or_else(|| parse_ansible_cfg_path_list(&cfg_content, "collections_paths")),
            cfg_dir,
        ),
    ));

    Ok(envs)
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

    let cmd_options: Vec<String> = if let Some(r) = reqs.as_ref() {
        let mut opts = r.options.clone();
        if let Some(limit) = opts.limit.as_ref() {
            opts.limit = Some(interpolate_template(
                limit,
                interpolated_args.as_ref(),
                "options.limit",
            )?);
        }
        get_cmd_options(opts)
    } else {
        vec![]
    };

    let mut inventories: Vec<String> = reqs
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
    // Absolute path of a repo-provided `ansible.cfg` to use as the effective
    // config, set when `delegate_to_git_repo.ansible_cfg` is provided.
    let mut ansible_config_override: Option<String> = None;

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
            let interpolated_playbook = delegated_git_repo
                .playbook
                .as_ref()
                .map(|p| -> error::Result<String> {
                    let p = interpolate_template(
                        p,
                        interpolated_args.as_ref(),
                        "delegate_to_git_repo.playbook",
                    )?;
                    validate_relative_path(&p, "delegate_to_git_repo.playbook")?;
                    Ok(p)
                })
                .transpose()?;
            let interpolated_commit = delegated_git_repo
                .commit
                .as_ref()
                .map(|c| {
                    interpolate_template(
                        c,
                        interpolated_args.as_ref(),
                        "delegate_to_git_repo.commit",
                    )
                })
                .transpose()?;
            let interpolated_inventories_location = delegated_git_repo
                .inventories_location
                .as_ref()
                .map(|p| -> error::Result<String> {
                    let p = interpolate_template(
                        p,
                        interpolated_args.as_ref(),
                        "delegate_to_git_repo.inventories_location",
                    )?;
                    validate_relative_path(&p, "delegate_to_git_repo.inventories_location")?;
                    Ok(p)
                })
                .transpose()?;

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

            let target_path = DELEGATE_GIT_REPO_TARGET.to_string();

            let repo = GitRepo {
                url: secret_url,
                commit: interpolated_commit.clone(),
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
            if let Some(commit) = interpolated_commit.as_ref() {
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
                .map_err(|e| {
                    anyhow!(
                        "Failed to clone git repo `{}`: {e}",
                        sanitize_git_url(&repo.url)
                    )
                })?;
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
                .map_err(|e| {
                    anyhow!(
                        "Failed to clone git repo `{}`: {e}",
                        sanitize_git_url(&repo.url)
                    )
                })?;
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

            playbook_override =
                Some(interpolated_playbook.map(|p| format!("{}/{}", &repo.target_path, p)));

            if let Some(inv) = interpolated_inventories_location {
                inventories.push("-i".to_string());
                inventories.push(format!("{}/{}", &repo.target_path, inv));
            }

            if let Some(cfg_rel) = delegated_git_repo.ansible_cfg.as_ref() {
                let cfg_rel = interpolate_template(
                    cfg_rel,
                    interpolated_args.as_ref(),
                    "delegate_to_git_repo.ansible_cfg",
                )?;
                validate_relative_path(&cfg_rel, "delegate_to_git_repo.ansible_cfg")?;
                let cfg_path = format!("{}/{}/{}", job_dir, &repo.target_path, cfg_rel);
                if !tokio::fs::try_exists(&cfg_path).await.unwrap_or(false) {
                    return Err(windmill_common::error::Error::BadRequest(format!(
                        "delegate_to_git_repo.ansible_cfg: no ansible.cfg found in the cloned repo at `{}/{}`",
                        &repo.target_path, cfg_rel
                    )));
                }
                ansible_config_override = Some(cfg_path);
            }

            if delegated_git_repo.install_requirements {
                install_requirements_from_cloned_repo(
                    &repo.target_path,
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

        if playbook_override.clone().flatten().is_none() && playbook.is_empty() {
            return Err(windmill_common::error::Error::BadRequest("No playbook was specified. Append a playbook to your script or specify one in the delegate_to_git_repo -> playbook section.".to_string()));
        }

        for repo in &r.git_repos {
            append_logs(
                &job.id,
                &job.workspace_id,
                format!("\nCloning {}...\n", sanitize_git_url(&repo.url)),
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
                .map_err(|e| {
                    anyhow!(
                        "Failed to clone git repo `{}`: {e}",
                        sanitize_git_url(&repo.url)
                    )
                })?;
            } else {
                if req_lockfiles.is_some() {
                    append_logs(
                        &job.id,
                        &job.workspace_id,
                        format!("Warning: `{}` is using latest commit because the lockfile didn't store a commit hash for this repo. Updates to the repo could break the deployed playbook.\n", sanitize_git_url(&repo.url)),
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
                .map_err(|e| {
                    anyhow!(
                        "Failed to clone git repo `{}`: {e}",
                        sanitize_git_url(&repo.url)
                    )
                })?;
            }

            append_logs(
                &job.id,
                &job.workspace_id,
                format!(
                    "Cloned {} into {}\n",
                    sanitize_git_url(&repo.url),
                    &repo.target_path
                ),
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

    create_ansible_cfg(reqs.as_ref(), job_dir, vault_password_file_exists, &job.id)?;
    let _control_path_guard = persistent_control_path_dir(&job.id).map(PersistentControlPathGuard);

    // When the run delegates to a git repo that ships its own ansible.cfg, that
    // file becomes the effective config (ansible loads exactly one config file and
    // does not merge). These env vars layer Windmill's runtime-bound settings back
    // on top — env vars outrank ansible.cfg. Only applied on the non-sandboxed
    // path: git-repo delegation clones into `job_dir` which the nsjail profile does
    // not mount, so it already requires DISABLE_NSJAIL.
    let ansible_env_overrides = match ansible_config_override.as_ref() {
        Some(cfg_path) => {
            if is_sandboxing_enabled() {
                tracing::warn!(
                    "delegate_to_git_repo.ansible_cfg is set but sandboxing is enabled; \
                     git-repo delegation requires DISABLE_NSJAIL, the ansible.cfg override \
                     will not take effect"
                );
            }
            build_ansible_cfg_override_envs(
                cfg_path,
                job_dir,
                vault_password_file_exists,
                reqs.as_ref(),
                &job.id,
            )
            .await?
        }
        None => vec![],
    };

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
        let nsjail_timeout =
            resolve_nsjail_timeout(conn, &job.workspace_id, job.id, job.timeout).await;
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_ANSIBLE_CONTENT
                .replace(
                    "{RLIMIT_AS}",
                    &render_nsjail_rlimit_as(NSJAIL_ANSIBLE_RLIMIT_AS_MB.as_deref(), 4096),
                )
                .replace("{PY_INSTALL_DIR}", &*PY_INSTALL_DIR)
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{SHARED_DEPENDENCIES}", shared_deps.as_str())
                .replace("{FILE_RESOURCES}", nsjail_extra_mounts.join("\n").as_str())
                .replace(
                    "{ADDITIONAL_PYTHON_PATHS}",
                    additional_python_paths_folders.as_str(),
                )
                .replace(
                    "{TMP_MOUNT_BLOCK}",
                    &resolve_nsjail_tmp_mount_block(job_dir).await,
                )
                .replace("{TIMEOUT}", &nsjail_timeout),
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
            .envs(ansible_env_overrides)
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

    if let Some(limit) = r.limit {
        ret.push("--limit".to_string());
        ret.push(limit);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args_from_json(v: serde_json::Value) -> HashMap<String, Box<RawValue>> {
        let serde_json::Value::Object(map) = v else {
            panic!("expected object");
        };
        map.into_iter()
            .map(|(k, v)| (k, RawValue::from_string(v.to_string()).unwrap()))
            .collect()
    }

    #[test]
    fn test_interpolate_template_string() {
        let args = args_from_json(serde_json::json!({"playbook": "site.yml"}));
        let out =
            interpolate_template("playbooks/{{ playbook }}", Some(&args), "playbook").unwrap();
        assert_eq!(out, "playbooks/site.yml");
    }

    #[test]
    fn test_interpolate_template_number_and_bool() {
        let args = args_from_json(serde_json::json!({"n": 42, "b": true}));
        let out = interpolate_template("{{ n }}-{{ b }}", Some(&args), "x").unwrap();
        assert_eq!(out, "42-true");
    }

    #[test]
    fn test_interpolate_template_no_placeholders() {
        let out = interpolate_template("plain.yml", None, "playbook").unwrap();
        assert_eq!(out, "plain.yml");
    }

    #[test]
    fn test_interpolate_template_missing_arg_errors() {
        let args = args_from_json(serde_json::json!({}));
        let err = interpolate_template("{{ missing }}", Some(&args), "playbook").unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn test_interpolate_template_object_arg_errors() {
        let args = args_from_json(serde_json::json!({"o": {"k": "v"}}));
        let err = interpolate_template("{{ o }}", Some(&args), "x").unwrap_err();
        assert!(err.to_string().contains("not a primitive"));
    }

    #[test]
    fn test_validate_relative_path_ok() {
        validate_relative_path("playbooks/site.yml", "playbook").unwrap();
        validate_relative_path("./site.yml", "playbook").unwrap();
        validate_relative_path("a/b/c.yml", "playbook").unwrap();
    }

    #[test]
    fn test_validate_relative_path_rejects_absolute() {
        // `/etc/passwd` isn't `is_absolute()` on Windows (no drive prefix), but
        // its leading RootDir still escapes the cloned repo, so reject it on
        // every platform.
        assert!(validate_relative_path("/etc/passwd", "playbook").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn test_validate_relative_path_rejects_windows_absolute() {
        assert!(validate_relative_path("\\etc\\passwd", "playbook").is_err());
        assert!(validate_relative_path("C:\\Windows\\System32", "playbook").is_err());
    }

    #[test]
    fn test_validate_relative_path_rejects_parent_dir() {
        assert!(validate_relative_path("../escape.yml", "playbook").is_err());
        assert!(validate_relative_path("a/../../escape.yml", "playbook").is_err());
    }

    #[test]
    fn test_validate_relative_path_rejects_empty() {
        assert!(validate_relative_path("", "playbook").is_err());
        assert!(validate_relative_path("   ", "playbook").is_err());
    }

    #[test]
    fn test_create_ansible_cfg_writes_valid_vault_id() {
        let dir = tempfile::tempdir().unwrap();
        let job_dir = dir.path().to_str().unwrap();
        let reqs = AnsibleRequirements {
            vault_id: vec!["dev@vault_pass.txt".to_string()],
            ..Default::default()
        };
        create_ansible_cfg(Some(&reqs), job_dir, false, &Uuid::new_v4()).unwrap();
        let cfg = std::fs::read_to_string(dir.path().join("ansible.cfg")).unwrap();
        assert!(cfg.contains("vault_identity_list = dev@vault_pass.txt"));
        assert!(!cfg.contains("library"));
    }

    /// The socket ansible binds under `control_path_dir` must fit `sun_path` (107
    /// usable bytes), which the job dir alone blows past — hence a short dir outside
    /// `ANSIBLE_HOME`.
    #[test]
    fn test_create_ansible_cfg_control_path_dir_fits_af_unix_limit() {
        let dir = tempfile::tempdir().unwrap();
        let job_dir = dir.path().to_str().unwrap();
        let job_id = Uuid::new_v4();
        let flag = TrustFlag::lock();
        flag.set(true);
        create_ansible_cfg(None, job_dir, false, &job_id).unwrap();

        let cfg = std::fs::read_to_string(dir.path().join("ansible.cfg")).unwrap();
        let control_path_dir = persistent_control_path_dir(&job_id).unwrap();
        assert!(cfg.contains("[persistent_connection]"));
        assert!(cfg.contains(&format!("control_path_dir = {control_path_dir}")));
        // The whole point: the socket dir must escape the job dir, whose length is what
        // blows the budget.
        assert!(!control_path_dir.starts_with(job_dir));

        // dir + `/` + socket name, budgeted at a full 40-char sha1 (ansible truncates
        // it far shorter today, but a custom control path may not).
        assert!(
            control_path_dir.len() + 1 + 40 <= AF_UNIX_PATH_LIMIT,
            "socket path would exceed sun_path: {control_path_dir}"
        );
    }

    #[test]
    fn test_create_ansible_cfg_rejects_vault_id_injection() {
        let dir = tempfile::tempdir().unwrap();
        let job_dir = dir.path().to_str().unwrap();
        let reqs = AnsibleRequirements {
            vault_id: vec!["default@/tmp/wm/x\nlibrary = /tmp/wm/evil_modules".to_string()],
            ..Default::default()
        };
        // Defense-in-depth boundary: a poisoned entry must error before any config is written.
        assert!(create_ansible_cfg(Some(&reqs), job_dir, false, &Uuid::new_v4()).is_err());
        assert!(!dir.path().join("ansible.cfg").exists());
    }

    #[test]
    fn test_parse_ansible_cfg_path_list() {
        let cfg = "\
[defaults]
roles_path = roles:extra/roles
collections_path=/opt/collections
host_key_checking = False

[inventory]
roles_path = ignored/section
";
        assert_eq!(
            parse_ansible_cfg_path_list(cfg, "roles_path"),
            Some(vec!["roles".to_string(), "extra/roles".to_string()])
        );
        assert_eq!(
            parse_ansible_cfg_path_list(cfg, "collections_path"),
            Some(vec!["/opt/collections".to_string()])
        );
        // Keys only in another section are not picked up.
        assert_eq!(parse_ansible_cfg_path_list(cfg, "library"), None);
    }

    #[test]
    fn test_parse_ansible_cfg_path_list_ignores_comments() {
        let cfg = "\
[defaults]
# roles_path = commented
; roles_path = also_commented
";
        assert_eq!(parse_ansible_cfg_path_list(cfg, "roles_path"), None);
    }

    #[test]
    fn test_parse_ansible_cfg_path_list_colon_delimiter() {
        // configparser accepts `:` as a key/value delimiter, and the value can
        // itself be a `:`-separated list.
        let cfg = "\
[defaults]
roles_path: my_roles
collections_path : a/col:b/col
";
        assert_eq!(
            parse_ansible_cfg_path_list(cfg, "roles_path"),
            Some(vec!["my_roles".to_string()])
        );
        assert_eq!(
            parse_ansible_cfg_path_list(cfg, "collections_path"),
            Some(vec!["a/col".to_string(), "b/col".to_string()])
        );
    }

    #[test]
    fn test_resolve_and_prepend_path() {
        // No repo entries: only Windmill's install dir.
        assert_eq!(
            resolve_and_prepend_path("/job/roles".to_string(), None, "/job/repo"),
            "/job/roles"
        );
        // Relative repo entries resolve against the cfg dir; absolute/~ kept as-is.
        assert_eq!(
            resolve_and_prepend_path(
                "/job/roles".to_string(),
                Some(vec![
                    "roles".to_string(),
                    "/abs/roles".to_string(),
                    "~/r".to_string()
                ]),
                "/job/repo/config"
            ),
            "/job/roles:/job/repo/config/roles:/abs/roles:~/r"
        );
    }

    fn ansible_playbook_available() -> bool {
        std::process::Command::new("ansible-playbook")
            .arg("--version")
            .output()
            .is_ok()
    }

    /// End-to-end: with a delegated repo that ships its own `ansible.cfg` pointing
    /// `roles_path` at an in-repo directory, the override env vars must make the
    /// real `ansible-playbook` resolve a role it otherwise cannot. Requires the
    /// `ansible-playbook` binary; self-skips when absent (e.g. standard CI). Run on
    /// a worker devbox with `cargo test -p windmill-worker --features python`.
    #[tokio::test]
    async fn test_ansible_cfg_override_resolves_repo_roles_e2e() {
        if !ansible_playbook_available() {
            eprintln!(
                "SKIP test_ansible_cfg_override_resolves_repo_roles_e2e: ansible-playbook not found on PATH"
            );
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let job_dir = dir.path().to_str().unwrap();
        let repo = dir.path().join(DELEGATE_GIT_REPO_TARGET);
        let role_tasks = repo.join("my_roles/greet/tasks");
        std::fs::create_dir_all(&role_tasks).unwrap();

        std::fs::write(
            repo.join("ansible.cfg"),
            "[defaults]\nroles_path = my_roles\n",
        )
        .unwrap();
        std::fs::write(
            role_tasks.join("main.yml"),
            "- debug:\n    msg: \"hello from greet role\"\n",
        )
        .unwrap();
        let play = "- hosts: localhost\n  connection: local\n  gather_facts: false\n  roles:\n    - greet\n";
        std::fs::write(repo.join("play.yml"), play).unwrap();

        // Windmill's own generated cfg (the negative-control config that exists today).
        create_ansible_cfg(None, job_dir, false, &Uuid::new_v4()).unwrap();

        let playbook = format!("{DELEGATE_GIT_REPO_TARGET}/play.yml");
        let run = |envs: Vec<(String, String)>| {
            std::process::Command::new("ansible-playbook")
                .arg(&playbook)
                .current_dir(job_dir)
                .envs(envs)
                .output()
                .unwrap()
        };

        // Negative control: today's behavior (Windmill cfg via cwd, no override) —
        // the role lives in the repo subdir and is not found.
        let cfg_path = repo.join("ansible.cfg");
        let before = run(vec![]);
        assert!(
            !before.status.success(),
            "without the override the repo role must NOT resolve; stdout={}",
            String::from_utf8_lossy(&before.stdout)
        );

        // With the override: ANSIBLE_CONFIG points at the repo cfg and roles_path
        // is honored, so the role runs.
        let envs = build_ansible_cfg_override_envs(
            cfg_path.to_str().unwrap(),
            job_dir,
            false,
            None,
            &Uuid::new_v4(),
        )
        .await
        .unwrap();
        let after = run(envs);
        let stdout = String::from_utf8_lossy(&after.stdout);
        assert!(
            after.status.success() && stdout.contains("hello from greet role"),
            "with the override the repo role must resolve; status={:?} stdout={stdout} stderr={}",
            after.status,
            String::from_utf8_lossy(&after.stderr)
        );
    }

    #[tokio::test]
    async fn test_build_ansible_cfg_override_envs() {
        let dir = tempfile::tempdir().unwrap();
        let job_dir = dir.path().to_str().unwrap();
        let repo_dir = dir.path().join("delegate_git_repository");
        std::fs::create_dir_all(&repo_dir).unwrap();
        let cfg_path = repo_dir.join("ansible.cfg");
        std::fs::write(&cfg_path, "[defaults]\nroles_path = my_roles\n").unwrap();
        let cfg_path = cfg_path.to_str().unwrap();

        let reqs = AnsibleRequirements {
            vault_id: vec!["dev@vault_pass.txt".to_string()],
            ..Default::default()
        };
        let job_id = Uuid::new_v4();
        let flag = TrustFlag::lock();
        flag.set(true);
        let envs = build_ansible_cfg_override_envs(cfg_path, job_dir, true, Some(&reqs), &job_id)
            .await
            .unwrap();
        let map: std::collections::HashMap<_, _> = envs.into_iter().collect();

        assert_eq!(
            map.get("ANSIBLE_CONFIG").map(|s| s.as_str()),
            Some(cfg_path)
        );
        // The repo cfg declares no control_path_dir, so Windmill's short default applies.
        assert_eq!(
            map.get("ANSIBLE_PERSISTENT_CONTROL_PATH_DIR"),
            persistent_control_path_dir(&job_id).as_ref()
        );
        assert_eq!(
            map.get("ANSIBLE_HOME"),
            Some(&format!("{job_dir}/.ansible"))
        );
        assert_eq!(
            map.get("ANSIBLE_VAULT_PASSWORD_FILE"),
            Some(&format!("{job_dir}/{WINDMILL_ANSIBLE_PASSWORD_FILENAME}"))
        );
        assert_eq!(
            map.get("ANSIBLE_VAULT_IDENTITY_LIST").map(|s| s.as_str()),
            Some("dev@vault_pass.txt")
        );
        // Windmill's `{job_dir}/roles` is prepended to the repo cfg's own path,
        // which is resolved against the cfg directory.
        assert_eq!(
            map.get("ANSIBLE_ROLES_PATH"),
            Some(&format!(
                "{job_dir}/roles:{}/my_roles",
                repo_dir.to_str().unwrap()
            ))
        );
        // No collections_path in the repo cfg → only Windmill's job dir.
        assert_eq!(
            map.get("ANSIBLE_COLLECTIONS_PATH").map(|s| s.as_str()),
            Some(job_dir)
        );
    }

    #[tokio::test]
    async fn test_build_ansible_cfg_override_envs_collections_paths_alias() {
        let dir = tempfile::tempdir().unwrap();
        let job_dir = dir.path().to_str().unwrap();
        let repo_dir = dir.path().join("delegate_git_repository");
        std::fs::create_dir_all(&repo_dir).unwrap();
        let cfg_path = repo_dir.join("ansible.cfg");
        // Deprecated plural alias must still be picked up so the repo's collections
        // are not silently dropped when the env override replaces the cfg value.
        std::fs::write(&cfg_path, "[defaults]\ncollections_paths = my_cols\n").unwrap();

        let envs = build_ansible_cfg_override_envs(
            cfg_path.to_str().unwrap(),
            job_dir,
            false,
            None,
            &Uuid::new_v4(),
        )
        .await
        .unwrap();
        let map: std::collections::HashMap<_, _> = envs.into_iter().collect();
        assert_eq!(
            map.get("ANSIBLE_COLLECTIONS_PATH"),
            Some(&format!("{job_dir}:{}/my_cols", repo_dir.to_str().unwrap()))
        );
    }

    #[tokio::test]
    async fn test_build_ansible_cfg_override_envs_keeps_user_control_path_dir() {
        let dir = tempfile::tempdir().unwrap();
        let job_dir = dir.path().to_str().unwrap();
        let repo_dir = dir.path().join(DELEGATE_GIT_REPO_TARGET);
        std::fs::create_dir_all(&repo_dir).unwrap();
        let cfg_path = repo_dir.join("ansible.cfg");
        std::fs::write(
            &cfg_path,
            "[defaults]\nroles_path = my_roles\n\n[persistent_connection]\ncontrol_path_dir = /tmp/my_pc\n",
        )
        .unwrap();

        let envs = build_ansible_cfg_override_envs(
            cfg_path.to_str().unwrap(),
            job_dir,
            false,
            None,
            &Uuid::new_v4(),
        )
        .await
        .unwrap();
        let map: std::collections::HashMap<_, _> = envs.into_iter().collect();
        assert_eq!(map.get("ANSIBLE_PERSISTENT_CONTROL_PATH_DIR"), None);
    }

    /// `SOCKET_ROOT_TRUSTED` is process-global and cargo runs tests in parallel: hold this
    /// while reading or flipping it, and the default is restored on the way out.
    struct TrustFlag(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

    impl TrustFlag {
        fn lock() -> Self {
            static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
            Self(LOCK.lock().unwrap_or_else(|e| e.into_inner()))
        }
        fn set(&self, trusted: bool) {
            SOCKET_ROOT_TRUSTED.store(trusted, std::sync::atomic::Ordering::Relaxed);
        }
        fn get(&self) -> bool {
            SOCKET_ROOT_TRUSTED.load(std::sync::atomic::Ordering::Relaxed)
        }
    }

    impl Drop for TrustFlag {
        fn drop(&mut self) {
            SOCKET_ROOT_TRUSTED.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    #[cfg(unix)]
    fn backdate(path: &std::path::Path, age: std::time::Duration) {
        let times = std::fs::FileTimes::new().set_modified(std::time::SystemTime::now() - age);
        std::fs::File::open(path).unwrap().set_times(times).unwrap();
    }

    /// The sweep only reaps what no live job can own: a play may hold its socket dir for
    /// the whole of MAX_TIMEOUT without touching the mtime again. And it only ever touches
    /// names it could have created itself.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_prepare_socket_root_sweeps_only_stale_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("wm-pc");
        std::fs::create_dir(&root).unwrap();

        let stale = root.join(Uuid::new_v4().simple().to_string());
        let live = root.join(Uuid::new_v4().simple().to_string());
        let foreign = root.join("someone-elses-data");
        for p in [&stale, &live, &foreign] {
            std::fs::create_dir(p).unwrap();
        }
        backdate(&stale, std::time::Duration::from_secs(48 * 60 * 60));
        backdate(&live, std::time::Duration::from_secs(12 * 60 * 60));
        backdate(&foreign, std::time::Duration::from_secs(48 * 60 * 60));

        let _flag = TrustFlag::lock();
        prepare_socket_root(
            root.to_str().unwrap(),
            std::time::Duration::from_secs(24 * 60 * 60),
        )
        .await;

        assert!(!stale.exists(), "dir older than the cutoff must be reaped");
        assert!(live.exists(), "a dir a live job may still own must be kept");
        assert!(
            foreign.exists(),
            "a stale dir we never created must be left alone"
        );
    }

    #[test]
    fn test_is_persistent_control_path_dir_name() {
        assert!(is_persistent_control_path_dir_name(
            &Uuid::new_v4().simple().to_string()
        ));
        // Hyphenated form is not what we create, so it is not ours to delete.
        assert!(!is_persistent_control_path_dir_name(
            &Uuid::new_v4().to_string()
        ));
        assert!(!is_persistent_control_path_dir_name("someone-elses-data"));
        assert!(!is_persistent_control_path_dir_name(""));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_prepare_socket_root_creates_root_private() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("wm-pc");
        let flag = TrustFlag::lock();
        flag.set(true);
        prepare_socket_root(root.to_str().unwrap(), std::time::Duration::from_secs(1)).await;

        let meta = std::fs::metadata(&root).unwrap();
        assert!(meta.is_dir());
        // Owning the root 0700 is what stops another local user replacing it later.
        assert_eq!(meta.permissions().mode() & 0o777, 0o700);
        assert!(flag.get(), "a root we created ourselves is trusted");
    }

    /// The root must be validated *after* the create attempt, not before: under a sticky
    /// parent another uid may still win the race to create the not-yet-existing `pc`
    /// (sticky stops them renaming ours away, not creating it first), and a create that
    /// tolerates `AlreadyExists` would otherwise hand us their directory unchecked.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_prepare_socket_root_validates_raced_creation() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("windmill");
        std::fs::create_dir(&parent).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o1777)).unwrap();

        // Stand in for the racer's dir: present before we look, and not exclusively ours.
        let root = parent.join("pc");
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o777)).unwrap();

        let flag = TrustFlag::lock();
        flag.set(true);
        prepare_socket_root(root.to_str().unwrap(), std::time::Duration::from_secs(1)).await;

        assert!(
            !flag.get(),
            "a root raced into place under a sticky parent must not be trusted"
        );
    }

    /// Safe but unusable is still not trusted: ansible cannot create its per-job dir under
    /// a root we cannot write, and naming it anyway would swap the working fallback for a
    /// permission error on every network playbook.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_prepare_socket_root_refuses_unwritable_root() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("wm-pc");
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o500)).unwrap();

        let flag = TrustFlag::lock();
        flag.set(true);
        prepare_socket_root(root.to_str().unwrap(), std::time::Duration::from_secs(1)).await;

        assert!(!flag.get(), "a root we cannot write must not be trusted");
        // Let the tempdir clean itself up.
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    }

    /// A root we do not exclusively own may have been pre-planted by another local user,
    /// who then controls the parent of every job's socket dir — and could swap a symlink
    /// in after this check, redirecting the sweep's path-based `remove_dir_all`.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_prepare_socket_root_refuses_world_writable_root() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("wm-pc");
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o777)).unwrap();

        // UUID-named, so survival proves the trust check stopped the sweep rather than the
        // name filter.
        let stale = root.join(Uuid::new_v4().simple().to_string());
        std::fs::create_dir(&stale).unwrap();
        backdate(&stale, std::time::Duration::from_secs(48 * 60 * 60));

        let _flag = TrustFlag::lock();
        prepare_socket_root(
            root.to_str().unwrap(),
            std::time::Duration::from_secs(24 * 60 * 60),
        )
        .await;

        assert!(
            stale.exists(),
            "must not sweep a root that others can write to"
        );
    }

    /// The root must hang off `/tmp`, whose sticky bit is what protects it. The trap this
    /// guards: the shipped image chmods the whole `WINDMILL_DIR` tree to a non-sticky 0777
    /// so any UID can write it, so parenting the root there would make it untrusted and
    /// silently disable this fix in the standard image while every local test still passed.
    #[test]
    fn test_control_path_root_hangs_off_tmp() {
        assert_eq!(
            std::path::Path::new(PERSISTENT_CONTROL_PATH_ROOT).parent(),
            Some(std::path::Path::new("/tmp"))
        );
    }

    /// A parent that others can write (and that is not sticky) lets them rename the root
    /// away and drop a symlink in its place after the checks — so the root cannot be
    /// trusted no matter how it currently looks.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_prepare_socket_root_refuses_writable_parent() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("windmill");
        let root = parent.join("pc");
        std::fs::create_dir_all(&root).unwrap();
        // UUID-named, so survival proves the trust check stopped the sweep rather than the
        // name filter.
        let stale = root.join(Uuid::new_v4().simple().to_string());
        std::fs::create_dir(&stale).unwrap();
        backdate(&stale, std::time::Duration::from_secs(48 * 60 * 60));
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o777)).unwrap();

        let flag = TrustFlag::lock();
        flag.set(true);
        prepare_socket_root(
            root.to_str().unwrap(),
            std::time::Duration::from_secs(24 * 60 * 60),
        )
        .await;

        assert!(stale.exists(), "must not sweep under a replaceable parent");
        assert!(
            !flag.get(),
            "an untrusted root must be marked so jobs stop naming it"
        );
    }

    /// A sticky parent (like /tmp itself) is fine: the sticky bit is what stops a
    /// non-owner renaming our root out of it.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_prepare_socket_root_accepts_sticky_world_writable_parent() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("windmill");
        let root = parent.join("pc");
        std::fs::create_dir_all(&parent).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o1777)).unwrap();

        let flag = TrustFlag::lock();
        flag.set(true);
        prepare_socket_root(
            root.to_str().unwrap(),
            std::time::Duration::from_secs(24 * 60 * 60),
        )
        .await;

        assert!(root.is_dir(), "root must be created under a sticky parent");
        assert!(flag.get());
    }

    /// Fail closed: when the root is untrusted the cfg must not name it, so ansible falls
    /// back to its own `{ANSIBLE_HOME}/pc` default inside the worker-owned job dir.
    #[test]
    fn test_create_ansible_cfg_omits_untrusted_control_path_dir() {
        let dir = tempfile::tempdir().unwrap();
        let job_dir = dir.path().to_str().unwrap();

        let flag = TrustFlag::lock();
        flag.set(false);
        create_ansible_cfg(None, job_dir, false, &Uuid::new_v4()).unwrap();

        let cfg = std::fs::read_to_string(dir.path().join("ansible.cfg")).unwrap();
        assert!(!cfg.contains("control_path_dir"));
        assert!(!cfg.contains("[persistent_connection]"));
    }

    /// A symlinked root must never be swept: `remove_dir_all` through it would delete
    /// whatever the link points at, as the worker's uid.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_prepare_socket_root_refuses_symlinked_root() {
        let dir = tempfile::tempdir().unwrap();
        let victim = dir.path().join("victim");
        // UUID-named, so survival proves the symlink was not followed rather than the name
        // filter sparing it.
        let victim_child = victim.join(Uuid::new_v4().simple().to_string());
        std::fs::create_dir_all(&victim_child).unwrap();
        backdate(&victim_child, std::time::Duration::from_secs(48 * 60 * 60));

        let root = dir.path().join("wm-pc");
        std::os::unix::fs::symlink(&victim, &root).unwrap();

        let _flag = TrustFlag::lock();
        prepare_socket_root(
            root.to_str().unwrap(),
            std::time::Duration::from_secs(24 * 60 * 60),
        )
        .await;

        assert!(
            victim_child.exists(),
            "sweep must not follow a symlinked root"
        );
    }

    /// configparser matches `\[(?P<header>.+)\]` without anchoring the end of the line, so
    /// a header with anything trailing it is still that section — and missing it here
    /// would silently override the user's own control_path_dir.
    #[test]
    fn test_ansible_cfg_section_header_with_trailing_text() {
        assert_eq!(
            parse_ansible_cfg_section_header("[persistent_connection] ; note"),
            Some("persistent_connection")
        );
        assert_eq!(parse_ansible_cfg_section_header("not a header"), None);
        // Greedy `.+` runs to the last `]`.
        assert_eq!(parse_ansible_cfg_section_header("[a]b]"), Some("a]b"));

        assert!(ansible_cfg_declares(
            "[persistent_connection] ; note\ncontrol_path_dir = /tmp/mine\n",
            "persistent_connection",
            "control_path_dir"
        ));
        assert_eq!(
            parse_ansible_cfg_path_list("[defaults] # note\nroles_path = my_roles\n", "roles_path"),
            Some(vec!["my_roles".to_string()])
        );
    }

    #[test]
    fn test_ansible_cfg_declares_scoped_to_section() {
        let cfg = "\
[defaults]
control_path_dir = /wrong/section

[persistent_connection]
# control_path_dir = /commented
connect_timeout = 30
";
        assert!(!ansible_cfg_declares(
            cfg,
            "persistent_connection",
            "control_path_dir"
        ));
        assert!(ansible_cfg_declares(
            cfg,
            "persistent_connection",
            "connect_timeout"
        ));
    }
}
