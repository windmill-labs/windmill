//! Audit entries for writes to instance settings and worker group configs, whichever way
//! they arrive: an API request, or a declarative sync that no user request stands behind.

use std::collections::HashMap;

use windmill_common::{
    audit::AuditAuthor, error::Result, instance_config::ConfigWriteObserver,
    users::SUPERADMIN_SYNC_EMAIL, DB,
};

use crate::{
    audit_oss::{audit_log, AuditAuthorable},
    ActionKind,
};

/// The declarative writer behind an entry, recorded as its `source` parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclarativeSource {
    /// `windmill sync-config <file>`
    SyncConfig,
    /// The Kubernetes operator's ConfigMap sync.
    Operator,
}

impl DeclarativeSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeclarativeSource::SyncConfig => "sync-config",
            DeclarativeSource::Operator => "operator",
        }
    }
}

/// Who a declarative sync's writes are attributed to. It runs on direct database
/// credentials, so there is no user to name.
pub fn declarative_sync_author() -> AuditAuthor {
    AuditAuthor {
        username: "instance_config_sync".to_string(),
        email: SUPERADMIN_SYNC_EMAIL.to_string(),
        username_override: None,
        token_prefix: None,
    }
}

fn parameters<'a>(
    value: Option<&'a str>,
    source: Option<DeclarativeSource>,
) -> Option<HashMap<&'a str, &'a str>> {
    let parameters: HashMap<&str, &str> = value
        .map(|v| ("value", v))
        .into_iter()
        .chain(source.map(|s| ("source", s.as_str())))
        .collect();
    (!parameters.is_empty()).then_some(parameters)
}

/// A setting is recorded by name, with its value only when that is a boolean or a number.
/// Any other shape can carry a credential (a registry URL, a webhook, a header map), and no
/// list of secret-bearing settings stays complete as settings are added.
///
/// AUTHORIZATION: records a write, checks nothing. The caller must have established that
/// `author` may change instance settings before performing the write this records.
pub async fn audit_setting_write(
    db: &DB,
    author: &impl AuditAuthorable,
    key: &str,
    value: Option<&serde_json::Value>,
    source: Option<DeclarativeSource>,
) -> Result<()> {
    let deleted = match value {
        None | Some(serde_json::Value::Null) => true,
        Some(serde_json::Value::String(s)) => s.trim().is_empty(),
        Some(_) => false,
    };
    let recorded = match value {
        Some(v @ (serde_json::Value::Bool(_) | serde_json::Value::Number(_))) => {
            Some(v.to_string())
        }
        _ => None,
    };
    audit_log(
        db,
        author,
        if deleted {
            "global_settings.delete"
        } else {
            "global_settings.update"
        },
        if deleted {
            ActionKind::Delete
        } else {
            ActionKind::Update
        },
        "global",
        Some(key),
        parameters(recorded.as_deref(), source),
    )
    .await
}

/// A worker group config is recorded by name only: its env vars and init script are free
/// text that can carry a credential.
///
/// AUTHORIZATION: records a write, checks nothing. The caller must have established that
/// `author` may change worker group configs before performing the write this records.
pub async fn audit_worker_config_write(
    db: &DB,
    author: &impl AuditAuthorable,
    group: &str,
    deleted: bool,
    source: Option<DeclarativeSource>,
) -> Result<()> {
    let (operation, action_kind) = if deleted {
        ("worker_config.delete", ActionKind::Delete)
    } else {
        ("worker_config.update", ActionKind::Update)
    };
    audit_log(
        db,
        author,
        operation,
        action_kind,
        "global",
        Some(&format!("worker__{group}")),
        parameters(None, source),
    )
    .await
}

/// Audits each write of an instance config apply as `author`'s.
///
/// AUTHORIZATION: as [`audit_setting_write`], for every write it is told of.
pub struct InstanceConfigAuditor<'a, A> {
    pub db: &'a DB,
    pub author: &'a A,
    /// `None` for a write made through the API.
    pub source: Option<DeclarativeSource>,
}

impl<A: AuditAuthorable + Sync> ConfigWriteObserver for InstanceConfigAuditor<'_, A> {
    async fn setting_written(&self, key: &str, value: Option<&serde_json::Value>) -> Result<()> {
        audit_setting_write(self.db, self.author, key, value, self.source).await
    }

    async fn worker_config_written(&self, group: &str, deleted: bool) -> Result<()> {
        audit_worker_config_write(self.db, self.author, group, deleted, self.source).await
    }
}
