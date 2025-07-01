use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use windmill_common::error::JsonResult;

#[derive(Default, Serialize, Deserialize)]
pub struct ScopeOption {
    pub value: String,
    pub label: String,
    pub requires_resource_path: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ScopeDomain {
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<ScopeOption>,
}

fn trigger_scope_domains() -> Vec<ScopeDomain> {
    const TRIGGER_DOMAINS: &[(&str, &str)] = &[
        ("http_triggers", "HTTP"),
        ("websocket_triggers", "WebSocket"),
        ("kafka_triggers", "Kafka"),
        ("nats_triggers", "NATS"),
        ("mqtt_triggers", "MQTT"),
        ("sqs_triggers", "AWS SQS"),
        ("gcp_triggers", "GCP Pub/Sub"),
        ("postgres_triggers", "PostgreSQL"),
    ];

    let mut domains = Vec::new();
    for (domain, display_name) in TRIGGER_DOMAINS {
        domains.push(ScopeDomain {
            name: format!("{} Triggers", display_name),
            description: Some(format!("{} trigger management", display_name)),
            scopes: vec![
                ScopeOption {
                    value: format!("{domain}:read"),
                    label: format!("Read {} Triggers", display_name),
                    ..Default::default()
                },
                ScopeOption {
                    value: format!("{domain}:write"),
                    label: format!("Write {} Triggers", display_name),
                    ..Default::default()
                },
            ],
        });
    }
    domains
}

fn get_scopes() -> Vec<ScopeDomain> {
    let mut groups = vec![
        ScopeDomain {
            name: "Scripts".to_string(),
            description: Some("Access to automation scripts and workflows".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "scripts:read".to_string(),
                    label: "Read Scripts".to_string(),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "scripts:write".to_string(),
                    label: "Write Scripts".to_string(),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeDomain {
            name: "Flows".to_string(),
            description: Some("Access to automation scripts and workflows".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "flows:read".to_string(),
                    label: "Read Flows".to_string(),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "flows:write".to_string(),
                    label: "Write Flows".to_string(),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeDomain {
            name: "Jobs".to_string(),
            description: Some("Job management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "jobs:read".to_string(),
                    label: "Read Jobs".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "jobs:run".to_string(),
                    label: "Run Jobs".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Apps".to_string(),
            description: Some("App management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "apps:read".to_string(),
                    label: "Read Apps".to_string(),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "apps:write".to_string(),
                    label: "Write Apps".to_string(),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "apps:run".to_string(),
                    label: "Run App Components".to_string(),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeDomain {
            name: "RawApps".to_string(),
            description: Some("Raw app management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "raw_apps:read".to_string(),
                    label: "Read Raw Apps".to_string(),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "raw_apps:write".to_string(),
                    label: "Write Raw Apps".to_string(),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeDomain {
            name: "Resources".to_string(),
            description: Some("Resource management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "resources:read".to_string(),
                    label: "Read Resources".to_string(),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "resources:write".to_string(),
                    label: "Write Resources".to_string(),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeDomain {
            name: "Variables".to_string(),
            description: None,
            scopes: vec![
                ScopeOption {
                    value: "variables:read".to_string(),
                    label: "Read Variables".to_string(),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "variables:write".to_string(),
                    label: "Write Variables".to_string(),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeDomain {
            name: "Schedules".to_string(),
            description: Some("Scheduled tasks and automated triggers".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "schedules:read".to_string(),
                    label: "Read Schedules".to_string(),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "schedules:write".to_string(),
                    label: "Write Schedules".to_string(),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeDomain {
            name: "Users".to_string(),
            description: Some("User account management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "users:read".to_string(),
                    label: "Read Users".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "users:write".to_string(),
                    label: "Write Users".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Groups".to_string(),
            description: Some("Group management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "groups:read".to_string(),
                    label: "Read Groups".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "groups:write".to_string(),
                    label: "Write Groups".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Folders".to_string(),
            description: Some("Folder management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "folders:read".to_string(),
                    label: "Read Folders".to_string(),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "folders:write".to_string(),
                    label: "Write Folders".to_string(),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeDomain {
            name: "Workspaces".to_string(),
            description: Some("Workspace management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "workspaces:read".to_string(),
                    label: "Read Workspaces".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "workspaces:write".to_string(),
                    label: "Write Workspaces".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Audit".to_string(),
            description: Some("Audit log management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "audit:read".to_string(),
                    label: "Read Audit Logs".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "audit:write".to_string(),
                    label: "Write Audit Logs".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Workers".to_string(),
            description: Some("Worker management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "workers:read".to_string(),
                    label: "Read Workers".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "workers:write".to_string(),
                    label: "Write Workers".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Settings".to_string(),
            description: Some("System settings management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "settings:read".to_string(),
                    label: "Read Settings".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "settings:write".to_string(),
                    label: "Write Settings".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Service Logs".to_string(),
            description: Some("Service log management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "service_logs:read".to_string(),
                    label: "Read Service Logs".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "service_logs:write".to_string(),
                    label: "Write Service Logs".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Configs".to_string(),
            description: Some("Configuration management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "configs:read".to_string(),
                    label: "Read Configs".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "configs:write".to_string(),
                    label: "Write Configs".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "OAuth".to_string(),
            description: Some("OAuth management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "oauth:read".to_string(),
                    label: "Read OAuth".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "oauth:write".to_string(),
                    label: "Write OAuth".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "AI".to_string(),
            description: Some("AI feature management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "ai:read".to_string(),
                    label: "Read AI".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "ai:write".to_string(),
                    label: "Write AI".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Agent Workers".to_string(),
            description: Some("Agent worker management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "agent_workers:read".to_string(),
                    label: "Read Agent Workers".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "agent_workers:write".to_string(),
                    label: "Write Agent Workers".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Drafts".to_string(),
            description: Some("Draft management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "drafts:read".to_string(),
                    label: "Read Drafts".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "drafts:write".to_string(),
                    label: "Write Drafts".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Favorites".to_string(),
            description: Some("Favorite items management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "favorites:read".to_string(),
                    label: "Read Favorites".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "favorites:write".to_string(),
                    label: "Write Favorites".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Inputs".to_string(),
            description: Some("Input management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "inputs:read".to_string(),
                    label: "Read Inputs".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "inputs:write".to_string(),
                    label: "Write Inputs".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Job Helpers".to_string(),
            description: Some("Job helper utilities".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "job_helpers:read".to_string(),
                    label: "Read Job Helpers".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "job_helpers:write".to_string(),
                    label: "Write Job Helpers".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "OpenAPI".to_string(),
            description: Some("OpenAPI documentation management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "openapi:read".to_string(),
                    label: "Read OpenAPI".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "openapi:write".to_string(),
                    label: "Write OpenAPI".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Capture".to_string(),
            description: Some("Request capture management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "capture:read".to_string(),
                    label: "Read Capture".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "capture:write".to_string(),
                    label: "Write Capture".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Concurrency Groups".to_string(),
            description: Some("Concurrency group management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "concurrency_groups:read".to_string(),
                    label: "Read Concurrency Groups".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "concurrency_groups:write".to_string(),
                    label: "Write Concurrency Groups".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "OIDC".to_string(),
            description: Some("OIDC management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "oidc:read".to_string(),
                    label: "Read OIDC".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "oidc:write".to_string(),
                    label: "Write OIDC".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "ACLs".to_string(),
            description: Some("Access Control List management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "acls:read".to_string(),
                    label: "Read ACLs".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "acls:write".to_string(),
                    label: "Write ACLs".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Indexer".to_string(),
            description: Some("Search indexer management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "indexer:read".to_string(),
                    label: "Read Indexer".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "indexer:write".to_string(),
                    label: "Write Indexer".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Teams".to_string(),
            description: Some("Team management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "teams:read".to_string(),
                    label: "Read Teams".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "teams:write".to_string(),
                    label: "Write Teams".to_string(),
                    ..Default::default()
                },
            ],
        },
        ScopeDomain {
            name: "Git Sync".to_string(),
            description: Some("Git synchronization management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "git_sync:read".to_string(),
                    label: "Read Git Sync".to_string(),
                    ..Default::default()
                },
                ScopeOption {
                    value: "git_sync:write".to_string(),
                    label: "Write Git Sync".to_string(),
                    ..Default::default()
                },
            ],
        },
    ];

    // Add trigger domains
    groups.extend(trigger_scope_domains());

    groups
}

pub fn global_service() -> Router {
    Router::new().route("/list/scopes", get(get_all_available_scopes))
}

async fn get_all_available_scopes() -> JsonResult<Vec<ScopeDomain>> {
    Ok(Json(get_scopes()))
}
