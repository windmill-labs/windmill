use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use windmill_common::error::JsonResult;


fn trigger_scope_options() -> Vec<ScopeOption> {
    const TRIGGER_DOMAINS: &[&str] = &[
        "http_triggers",
        "websocket_triggers",
        "kafka_triggers",
        "nats_triggers",
        "mqtt_triggers",
        "sqs_triggers",
        "gcp_triggers",
        "postgres_triggers",
    ];

    const ACTIONS: &[(&str, &str)] = &[
        ("read", "Read"),
        ("write", "Create, update or run"),
        ("delete", "Delete"),
        ("admin", "Full admin access to"),
    ];

    let mut out = Vec::new();
    for domain in TRIGGER_DOMAINS {
        for (action, label) in ACTIONS {
            out.push(ScopeOption {
                value: format!("{domain}:{action}"),
                label: format!("{label} {}", domain.replace('_', " ")),
            });
        }
    }
    out
}

fn get_all_scopes() -> Vec<ScopeOption> {
    let mut scopes = vec![
        ScopeOption { value: "*".to_string(), label: "Full access to all resources".to_string() },
        ScopeOption { value: "jobs:read".into(), label: "Read jobs".into() },
        ScopeOption { value: "jobs:write".into(), label: "Create or update jobs".into() },
        ScopeOption { value: "jobs:execute".into(), label: "Execute jobs".into() },
        ScopeOption { value: "jobs:delete".into(), label: "Delete jobs".into() },
        ScopeOption { value: "jobs:admin".into(), label: "Full admin access to jobs".into() },
        ScopeOption { value: "scripts:read".into(), label: "Read scripts".into() },
        ScopeOption { value: "scripts:write".into(), label: "Create or update scripts".into() },
        ScopeOption { value: "scripts:execute".into(), label: "Execute scripts".into() },
        ScopeOption { value: "scripts:delete".into(), label: "Delete scripts".into() },
        ScopeOption { value: "scripts:admin".into(), label: "Full admin access to scripts".into() },
        ScopeOption { value: "flows:read".into(), label: "Read flows".into() },
        ScopeOption { value: "flows:write".into(), label: "Create or update flows".into() },
        ScopeOption { value: "flows:execute".into(), label: "Execute flows".into() },
        ScopeOption { value: "flows:delete".into(), label: "Delete flows".into() },
        ScopeOption { value: "flows:admin".into(), label: "Full admin access to flows".into() },
        ScopeOption { value: "apps:read".into(), label: "Read apps".into() },
        ScopeOption { value: "apps:write".into(), label: "Create or update apps".into() },
        ScopeOption { value: "apps:execute".into(), label: "Execute apps".into() },
        ScopeOption { value: "apps:delete".into(), label: "Delete apps".into() },
        ScopeOption { value: "apps:admin".into(), label: "Full admin access to apps".into() },
        ScopeOption { value: "variables:read".into(), label: "Read variables".into() },
        ScopeOption { value: "variables:write".into(), label: "Create or update variables".into() },
        ScopeOption { value: "variables:delete".into(), label: "Delete variables".into() },
        ScopeOption {
            value: "variables:admin".into(),
            label: "Full admin access to variables".into(),
        },
        ScopeOption { value: "resources:read".into(), label: "Read external resources".into() },
        ScopeOption { value: "resources:write".into(), label: "Create or update resources".into() },
        ScopeOption { value: "resources:delete".into(), label: "Delete resources".into() },
        ScopeOption {
            value: "resources:admin".into(),
            label: "Full admin access to resources".into(),
        },
        ScopeOption { value: "schedules:read".into(), label: "Read schedules".into() },
        ScopeOption { value: "schedules:write".into(), label: "Create or update schedules".into() },
        ScopeOption { value: "schedules:delete".into(), label: "Delete schedules".into() },
        ScopeOption {
            value: "schedules:admin".into(),
            label: "Full admin access to schedules".into(),
        },
        ScopeOption {
            value: "folders:read".into(),
            label: "List folders and their contents".into(),
        },
        ScopeOption {
            value: "folders:write".into(),
            label: "Create or modify folder contents".into(),
        },
        ScopeOption { value: "folders:delete".into(), label: "Delete folders".into() },
        ScopeOption { value: "folders:admin".into(), label: "Full admin access to folders".into() },
        ScopeOption {
            value: "users:read".into(),
            label: "Read user profiles and permissions".into(),
        },
        ScopeOption { value: "users:write".into(), label: "Invite or modify users".into() },
        ScopeOption { value: "users:delete".into(), label: "Delete users".into() },
        ScopeOption { value: "users:admin".into(), label: "Full admin access to users".into() },
        ScopeOption { value: "groups:read".into(), label: "Read groups".into() },
        ScopeOption { value: "groups:write".into(), label: "Create or modify groups".into() },
        ScopeOption { value: "groups:delete".into(), label: "Delete groups".into() },
        ScopeOption { value: "groups:admin".into(), label: "Full admin access to groups".into() },
        ScopeOption { value: "workspaces:read".into(), label: "Read workspace metadata".into() },
        ScopeOption {
            value: "workspaces:admin".into(),
            label: "Admin access to the workspace".into(),
        },
        ScopeOption {
            value: "settings:read".into(),
            label: "Read global or workspace settings".into(),
        },
        ScopeOption { value: "settings:write".into(), label: "Modify settings".into() },
        ScopeOption {
            value: "settings:admin".into(),
            label: "Full admin access to settings".into(),
        },
        ScopeOption { value: "audit:read".into(), label: "View audit logs".into() },
        ScopeOption { value: "workers:read".into(), label: "Read worker info".into() },
        ScopeOption { value: "workers:admin".into(), label: "Manage workers".into() },
        ScopeOption { value: "configs:read".into(), label: "Read configuration".into() },
        ScopeOption { value: "configs:write".into(), label: "Update configuration".into() },
        ScopeOption {
            value: "integrations:read".into(),
            label: "Read third-party integrations".into(),
        },
        ScopeOption { value: "integrations:write".into(), label: "Configure integrations".into() },
        ScopeOption { value: "oauth:read".into(), label: "Read OAuth clients".into() },
        ScopeOption { value: "oauth:write".into(), label: "Manage OAuth clients".into() },
        ScopeOption { value: "ai:read".into(), label: "Read AI provider config".into() },
        ScopeOption { value: "ai:write".into(), label: "Manage AI providers".into() },
        ScopeOption {
            value: "embeddings:read".into(),
            label: "Read embeddings configuration".into(),
        },
        ScopeOption { value: "embeddings:write".into(), label: "Update embedding settings".into() },
        ScopeOption {
            value: "job_metrics:read".into(),
            label: "Read job execution statistics".into(),
        },
        ScopeOption {
            value: "job_helpers:read".into(),
            label: "Access helper utilities for jobs".into(),
        },
        ScopeOption {
            value: "concurrency:read".into(),
            label: "Read concurrency group settings".into(),
        },
        ScopeOption { value: "oidc:read".into(), label: "Read OIDC configuration".into() },
        ScopeOption { value: "oidc:write".into(), label: "Update OIDC configuration".into() },
        ScopeOption { value: "openapi:read".into(), label: "Access OpenAPI documentation".into() },
        ScopeOption { value: "capture:read".into(), label: "Access webhook captures".into() },
        ScopeOption {
            value: "capture:write".into(),
            label: "Replay or inspect captured data".into(),
        },
        ScopeOption { value: "drafts:read".into(), label: "Read saved drafts".into() },
        ScopeOption { value: "drafts:write".into(), label: "Update or create drafts".into() },
        ScopeOption { value: "favorites:read".into(), label: "Read favorites".into() },
        ScopeOption { value: "favorites:write".into(), label: "Add or remove favorites".into() },
        ScopeOption { value: "inputs:read".into(), label: "Read input templates".into() },
        ScopeOption { value: "acls:read".into(), label: "Read access control lists".into() },
        ScopeOption { value: "acls:write".into(), label: "Manage access control lists".into() },
        ScopeOption { value: "acls:admin".into(), label: "Full admin access to ACLs".into() },
        ScopeOption { value: "raw_apps:read".into(), label: "Read raw application data".into() },
        ScopeOption { value: "raw_apps:write".into(), label: "Manage raw applications".into() },
        ScopeOption { value: "raw_apps:admin".into(), label: "Full admin access to raw apps".into() },
        ScopeOption { value: "tokens:read".into(), label: "Read API tokens".into() },
        ScopeOption { value: "tokens:write".into(), label: "Create and manage API tokens".into() },
        ScopeOption { value: "tokens:delete".into(), label: "Delete API tokens".into() },
        ScopeOption { value: "tokens:admin".into(), label: "Full admin access to tokens".into() },
        ScopeOption { value: "inkeep:read".into(), label: "Read Inkeep integration".into() },
        ScopeOption { value: "inkeep:write".into(), label: "Manage Inkeep integration".into() },
        ScopeOption { value: "saml:read".into(), label: "Read SAML configuration".into() },
        ScopeOption { value: "saml:write".into(), label: "Manage SAML authentication".into() },
        ScopeOption { value: "scim:read".into(), label: "Read SCIM configuration".into() },
        ScopeOption { value: "scim:write".into(), label: "Manage SCIM user provisioning".into() },
        ScopeOption { value: "agent_workers:read".into(), label: "Read agent workers status".into() },
        ScopeOption { value: "agent_workers:write".into(), label: "Manage agent workers".into() },
        ScopeOption { value: "agent_workers:admin".into(), label: "Full admin access to agent workers".into() },
        ScopeOption { value: "search:read".into(), label: "Access search and indexing".into() },
        ScopeOption { value: "search:write".into(), label: "Manage search indices".into() },
    ];

    scopes.extend(trigger_scope_options());
    scopes
}

pub fn global_service() -> Router {
    Router::new().route("/list/scopes", get(get_all_available_scope))
}

#[derive(Serialize, Deserialize)]
pub struct ScopeOption {
    pub value: String,
    pub label: String,
}

async fn get_all_available_scope() -> JsonResult<Vec<ScopeOption>> {
    Ok(Json(get_all_scopes()))
}
