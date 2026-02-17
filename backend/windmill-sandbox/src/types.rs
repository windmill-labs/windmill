use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "sandbox_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SandboxStatus {
    Creating,
    Running,
    Suspended,
    Stopped,
    Error,
}

impl std::fmt::Display for SandboxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxStatus::Creating => write!(f, "creating"),
            SandboxStatus::Running => write!(f, "running"),
            SandboxStatus::Suspended => write!(f, "suspended"),
            SandboxStatus::Stopped => write!(f, "stopped"),
            SandboxStatus::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default)]
    pub mode: SandboxMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_secs: Option<i32>,
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: i32,
    #[serde(default = "default_memory_limit_mb")]
    pub memory_limit_mb: i32,
    #[serde(default = "default_disk_limit_mb")]
    pub disk_limit_mb: i32,
    #[serde(default)]
    pub env_vars: serde_json::Value,
    #[serde(default)]
    pub labels: serde_json::Value,
    #[serde(default)]
    pub mounts: serde_json::Value,
    #[serde(default)]
    pub network_enabled: bool,
    #[serde(default)]
    pub ephemeral: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_stop_after_secs: Option<i32>,
}

fn default_cpu_limit() -> i32 {
    1
}
fn default_memory_limit_mb() -> i32 {
    512
}
fn default_disk_limit_mb() -> i32 {
    1024
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SandboxMode {
    #[default]
    Embedded,
    Remote,
}

impl std::fmt::Display for SandboxMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxMode::Embedded => write!(f, "embedded"),
            SandboxMode::Remote => write!(f, "remote"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInfo {
    pub id: Uuid,
    pub workspace_id: String,
    pub status: SandboxStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub labels: serde_json::Value,
    pub mode: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_activity_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspended_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub ephemeral: bool,
    pub cpu_limit: i32,
    pub memory_limit_mb: i32,
    pub network_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecRequest {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub exec_id: Uuid,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
}
