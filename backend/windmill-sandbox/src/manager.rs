use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::Mutex;
use uuid::Uuid;
use windmill_common::error::{Error, Result};

use crate::nsjail;
use crate::types::*;

struct SandboxProcess {
    _id: Uuid,
    status: SandboxStatus,
    nsjail_child: Option<tokio::process::Child>,
    nsjail_pid: Option<u32>,
    sandbox_dir: String,
    _config: SandboxConfig,
    _started_at: Option<chrono::DateTime<Utc>>,
    last_activity_at: Option<chrono::DateTime<Utc>>,
    suspended_at: Option<chrono::DateTime<Utc>>,
    _config_path: String,
}

pub struct SandboxManager {
    sandboxes: Arc<Mutex<HashMap<Uuid, SandboxProcess>>>,
    base_dir: String,
}

impl SandboxManager {
    pub fn new(base_dir: String) -> Self {
        std::fs::create_dir_all(&base_dir).ok();
        Self {
            sandboxes: Arc::new(Mutex::new(HashMap::new())),
            base_dir,
        }
    }

    pub async fn create_sandbox(&self, config: SandboxConfig) -> Result<(Uuid, SandboxStatus)> {
        let id = Uuid::new_v4();
        let sandbox_dir = format!("{}/{}", self.base_dir, id);
        std::fs::create_dir_all(&sandbox_dir)
            .map_err(|e| Error::InternalErr(format!("Failed to create sandbox dir: {e}")))?;

        let nsjail_config = nsjail::NsjailConfig {
            sandbox_dir: sandbox_dir.clone(),
            clone_newnet: config.network_enabled,
            clone_newuser: !is_nuser_disabled(),
            memory_limit_bytes: (config.memory_limit_mb as u64) * 1024 * 1024,
            cpu_ms_per_sec: (config.cpu_limit as u32) * 1000,
            disk_limit_bytes: (config.disk_limit_mb as u64) * 1024 * 1024,
            extra_mounts: vec![],
        };

        let config_content = nsjail::render_config(&nsjail_config);
        let config_path = format!("{}/sandbox.config.proto", sandbox_dir);
        std::fs::write(&config_path, &config_content)
            .map_err(|e| Error::InternalErr(format!("Failed to write nsjail config: {e}")))?;

        let env_vars: Vec<(String, String)> = if let Some(obj) = config.env_vars.as_object() {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                .collect()
        } else {
            vec![]
        };

        let child = nsjail::spawn_nsjail(&config_path, &env_vars).await?;
        let nsjail_pid = child.id();

        let process = SandboxProcess {
            _id: id,
            status: SandboxStatus::Running,
            nsjail_child: Some(child),
            nsjail_pid,
            sandbox_dir,
            _config: config,
            _started_at: Some(Utc::now()),
            last_activity_at: Some(Utc::now()),
            suspended_at: None,
            _config_path: config_path,
        };

        self.sandboxes.lock().await.insert(id, process);
        Ok((id, SandboxStatus::Running))
    }

    pub async fn exec(
        &self,
        sandbox_id: Uuid,
        request: ExecRequest,
    ) -> Result<ExecResult> {
        let (child_pid, _sandbox_dir) = {
            let mut sandboxes = self.sandboxes.lock().await;
            let sb = sandboxes
                .get_mut(&sandbox_id)
                .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

            if sb.status != SandboxStatus::Running {
                return Err(Error::BadRequest(format!(
                    "Sandbox is not running (status: {})",
                    sb.status
                )));
            }
            sb.last_activity_at = Some(Utc::now());
            let pid = sb
                .nsjail_pid
                .ok_or_else(|| Error::InternalErr("No PID for sandbox".to_string()))?;
            (pid, sb.sandbox_dir.clone())
        };

        let env_vars: Vec<(String, String)> = request
            .env
            .as_ref()
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let started_at = Utc::now();
        let (exit_code, stdout, stderr) = nsjail::exec_in_sandbox(
            child_pid,
            &request.command,
            request.cwd.as_deref(),
            &env_vars,
            request.timeout_secs.map(|s| s as u32),
        )
        .await?;
        let duration_ms = (Utc::now() - started_at).num_milliseconds();

        Ok(ExecResult {
            exec_id: Uuid::new_v4(),
            exit_code,
            stdout,
            stderr,
            duration_ms,
        })
    }

    pub async fn suspend(&self, sandbox_id: Uuid) -> Result<SandboxStatus> {
        let mut sandboxes = self.sandboxes.lock().await;
        let sb = sandboxes
            .get_mut(&sandbox_id)
            .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

        if sb.status != SandboxStatus::Running {
            return Err(Error::BadRequest(format!(
                "Can only suspend a running sandbox (status: {})",
                sb.status
            )));
        }

        let pid = sb
            .nsjail_pid
            .ok_or_else(|| Error::InternalErr("No PID for sandbox".to_string()))?;

        nsjail::send_signal(pid, nix::sys::signal::Signal::SIGSTOP)?;
        sb.status = SandboxStatus::Suspended;
        sb.suspended_at = Some(Utc::now());
        Ok(SandboxStatus::Suspended)
    }

    pub async fn resume(&self, sandbox_id: Uuid) -> Result<SandboxStatus> {
        let mut sandboxes = self.sandboxes.lock().await;
        let sb = sandboxes
            .get_mut(&sandbox_id)
            .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

        if sb.status != SandboxStatus::Suspended {
            return Err(Error::BadRequest(format!(
                "Can only resume a suspended sandbox (status: {})",
                sb.status
            )));
        }

        let pid = sb
            .nsjail_pid
            .ok_or_else(|| Error::InternalErr("No PID for sandbox".to_string()))?;

        nsjail::send_signal(pid, nix::sys::signal::Signal::SIGCONT)?;
        sb.status = SandboxStatus::Running;
        sb.suspended_at = None;
        sb.last_activity_at = Some(Utc::now());
        Ok(SandboxStatus::Running)
    }

    pub async fn terminate(&self, sandbox_id: Uuid) -> Result<SandboxStatus> {
        let mut sandboxes = self.sandboxes.lock().await;
        let sb = sandboxes
            .get_mut(&sandbox_id)
            .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

        if sb.status == SandboxStatus::Stopped {
            return Ok(SandboxStatus::Stopped);
        }

        if let Some(pid) = sb.nsjail_pid {
            let _ = nsjail::send_signal(pid, nix::sys::signal::Signal::SIGTERM);
        }

        if let Some(ref mut child) = sb.nsjail_child {
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                child.wait(),
            )
            .await;
            child.kill().await.ok();
        }

        sb.status = SandboxStatus::Stopped;
        sb.nsjail_child = None;

        let _ = std::fs::remove_dir_all(&sb.sandbox_dir);

        Ok(SandboxStatus::Stopped)
    }

    pub async fn status(&self, sandbox_id: Uuid) -> Result<SandboxStatus> {
        let sandboxes = self.sandboxes.lock().await;
        let sb = sandboxes
            .get(&sandbox_id)
            .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;
        Ok(sb.status)
    }

    pub async fn write_file(
        &self,
        sandbox_id: Uuid,
        path: &str,
        content: &[u8],
    ) -> Result<()> {
        let sandbox_dir = {
            let mut sandboxes = self.sandboxes.lock().await;
            let sb = sandboxes
                .get_mut(&sandbox_id)
                .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;
            if sb.status == SandboxStatus::Stopped {
                return Err(Error::BadRequest("Sandbox is stopped".to_string()));
            }
            sb.last_activity_at = Some(Utc::now());
            sb.sandbox_dir.clone()
        };
        nsjail::write_to_sandbox(&sandbox_dir, path, content)
    }

    pub async fn read_file(&self, sandbox_id: Uuid, path: &str) -> Result<String> {
        let sandbox_dir = {
            let sandboxes = self.sandboxes.lock().await;
            let sb = sandboxes
                .get(&sandbox_id)
                .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;
            if sb.status == SandboxStatus::Stopped {
                return Err(Error::BadRequest("Sandbox is stopped".to_string()));
            }
            sb.sandbox_dir.clone()
        };
        nsjail::read_from_sandbox(&sandbox_dir, path)
    }

    pub async fn list_sandboxes(&self) -> Vec<(Uuid, SandboxStatus)> {
        let sandboxes = self.sandboxes.lock().await;
        sandboxes
            .iter()
            .map(|(id, sb)| (*id, sb.status))
            .collect()
    }

    pub async fn cleanup_all(&self) {
        let ids: Vec<Uuid> = {
            let sandboxes = self.sandboxes.lock().await;
            sandboxes.keys().cloned().collect()
        };
        for id in ids {
            let _ = self.terminate(id).await;
        }
        let mut sandboxes = self.sandboxes.lock().await;
        sandboxes.clear();
    }

    pub async fn remove_stopped(&self, sandbox_id: Uuid) -> Result<()> {
        let mut sandboxes = self.sandboxes.lock().await;
        let sb = sandboxes
            .get(&sandbox_id)
            .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;
        if sb.status != SandboxStatus::Stopped {
            return Err(Error::BadRequest(
                "Can only remove stopped sandboxes".to_string(),
            ));
        }
        sandboxes.remove(&sandbox_id);
        Ok(())
    }
}

fn is_nuser_disabled() -> bool {
    std::env::var("DISABLE_NUSER")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}
