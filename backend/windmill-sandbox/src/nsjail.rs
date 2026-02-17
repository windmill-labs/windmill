use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;
use windmill_common::error::{Error, Result};

const NSJAIL_CONFIG_TEMPLATE: &str = include_str!("../../windmill-worker/nsjail/sandbox.config.proto");

pub struct NsjailConfig {
    pub sandbox_dir: String,
    pub clone_newnet: bool,
    pub clone_newuser: bool,
    pub memory_limit_bytes: u64,
    pub cpu_ms_per_sec: u32,
    pub disk_limit_bytes: u64,
    pub extra_mounts: Vec<MountConfig>,
}

pub struct MountConfig {
    pub src: String,
    pub dst: String,
    pub rw: bool,
}

fn nsjail_path() -> String {
    std::env::var("NSJAIL_PATH").unwrap_or_else(|_| "nsjail".to_string())
}

pub fn render_config(config: &NsjailConfig) -> String {
    let extra_mounts = config
        .extra_mounts
        .iter()
        .map(|m| {
            format!(
                "mount {{\n    src: \"{}\"\n    dst: \"{}\"\n    is_bind: true\n    rw: {}\n}}",
                m.src, m.dst, m.rw
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    NSJAIL_CONFIG_TEMPLATE
        .replace("{SANDBOX_DIR}", &config.sandbox_dir)
        .replace("{CLONE_NEWNET}", &(!config.clone_newnet).to_string())
        .replace("{CLONE_NEWUSER}", &config.clone_newuser.to_string())
        .replace("{MEMORY_LIMIT}", &config.memory_limit_bytes.to_string())
        .replace("{CPU_MS_PER_SEC}", &config.cpu_ms_per_sec.to_string())
        .replace("{DISK_LIMIT}", &config.disk_limit_bytes.to_string())
        .replace("{IFACE_NO_LO}", &(!config.clone_newnet).to_string())
        .replace("{EXTRA_MOUNTS}", &extra_mounts)
}

pub async fn spawn_nsjail(
    config_path: &str,
    env_vars: &[(String, String)],
) -> Result<tokio::process::Child> {
    let mut cmd = Command::new(nsjail_path());
    cmd.arg("--config")
        .arg(config_path)
        .arg("--")
        .arg("/bin/bash")
        .arg("--login")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    for (k, v) in env_vars {
        cmd.env(k, v);
    }

    cmd.spawn()
        .map_err(|e| Error::InternalErr(format!("Failed to spawn nsjail: {e}")))
}

pub async fn exec_in_sandbox(
    child_pid: u32,
    command: &str,
    cwd: Option<&str>,
    env_vars: &[(String, String)],
    timeout_secs: Option<u32>,
) -> Result<(i32, String, String)> {
    let mut cmd = Command::new("nsenter");
    cmd.arg("--target")
        .arg(child_pid.to_string())
        .arg("--mount")
        .arg("--pid")
        .arg("--")
        .arg("/bin/bash")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    for (k, v) in env_vars {
        cmd.env(k, v);
    }

    let timeout = timeout_secs.unwrap_or(300);

    let child = cmd
        .spawn()
        .map_err(|e| Error::InternalErr(format!("Failed to exec nsenter: {e}")))?;

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout as u64),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| Error::ExecutionErr(format!("Command timed out after {timeout}s")))?
    .map_err(|e| Error::InternalErr(format!("Failed to wait for nsenter: {e}")))?;

    let exit_code = result.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

    Ok((exit_code, stdout, stderr))
}

pub fn send_signal(pid: u32, signal: nix::sys::signal::Signal) -> Result<()> {
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid as i32),
        signal,
    )
    .map_err(|e| Error::InternalErr(format!("Failed to send signal {signal} to PID {pid}: {e}")))
}

pub fn write_to_sandbox(sandbox_dir: &str, file_path: &str, content: &[u8]) -> Result<()> {
    let full_path = Path::new(sandbox_dir).join(
        file_path
            .strip_prefix('/')
            .unwrap_or(file_path),
    );
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::InternalErr(format!("Failed to create directory: {e}")))?;
    }
    std::fs::write(&full_path, content)
        .map_err(|e| Error::InternalErr(format!("Failed to write file: {e}")))
}

pub fn read_from_sandbox(sandbox_dir: &str, file_path: &str) -> Result<String> {
    let full_path = Path::new(sandbox_dir).join(
        file_path
            .strip_prefix('/')
            .unwrap_or(file_path),
    );
    std::fs::read_to_string(&full_path)
        .map_err(|e| Error::NotFound(format!("File not found or unreadable: {e}")))
}
