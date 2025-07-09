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
                    label: "Read".to_string(),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: format!("{domain}:write"),
                    label: "Write".to_string(),
                    requires_resource_path: true,
                },
            ],
        });
    }
    domains
}

fn standard_scope_domains() -> Vec<ScopeDomain> {
    const STANDARD_DOMAINS: &[(&str, &str, &str, bool)] = &[
        (
            "scripts",
            "Scripts",
            "Access to automation scripts and workflows",
            true,
        ),
        (
            "flows",
            "Flows",
            "Access to automation scripts and workflows",
            true,
        ),
        ("apps", "Apps", "App management", true),
        ("raw_apps", "RawApps", "Raw app management", true),
        ("resources", "Resources", "Resource management", true),
        ("variables", "Variables", "", true),
        (
            "schedules",
            "Schedules",
            "Scheduled tasks and automated triggers",
            true,
        ),
        ("folders", "Folders", "Folder management", true),
        ("users", "Users", "User account management", false),
        ("groups", "Groups", "Group management", false),
        ("workspaces", "Workspaces", "Workspace management", false),
        ("audit", "Audit", "Audit log management", false),
        ("workers", "Workers", "Worker management", false),
        ("settings", "Settings", "System settings management", false),
        (
            "service_logs",
            "Service Logs",
            "Service log management",
            false,
        ),
        ("configs", "Configs", "Configuration management", false),
        ("oauth", "OAuth", "OAuth management", false),
        ("ai", "AI", "AI feature management", false),
        (
            "agent_workers",
            "Agent Workers",
            "Agent worker management",
            false,
        ),
        ("drafts", "Drafts", "Draft management", false),
        ("favorites", "Favorites", "Favorite items management", false),
        ("inputs", "Inputs", "Input management", false),
        ("job_helpers", "Job Helpers", "Job helper utilities", false),
        (
            "openapi",
            "OpenAPI",
            "OpenAPI documentation management",
            false,
        ),
        ("capture", "Capture", "Request capture management", false),
        (
            "concurrency_groups",
            "Concurrency Groups",
            "Concurrency group management",
            false,
        ),
        ("oidc", "OIDC", "OIDC management", false),
        ("acls", "ACLs", "Access Control List management", false),
        ("indexer", "Indexer", "Search indexer management", false),
        ("teams", "Teams", "Team management", false),
        (
            "git_sync",
            "Git Sync",
            "Git synchronization management",
            false,
        ),
    ];

    let mut domains = Vec::new();
    for (domain_key, domain_name, description, requires_resource_path) in STANDARD_DOMAINS {
        domains.push(ScopeDomain {
            name: domain_name.to_string(),
            description: if description.is_empty() {
                None
            } else {
                Some(description.to_string())
            },
            scopes: vec![
                ScopeOption {
                    value: format!("{domain_key}:read"),
                    label: "Read".to_string(),
                    requires_resource_path: *requires_resource_path,
                },
                ScopeOption {
                    value: format!("{domain_key}:write"),
                    label: "Write".to_string(),
                    requires_resource_path: *requires_resource_path,
                },
            ],
        });
    }
    domains
}

fn get_scopes() -> Vec<ScopeDomain> {
    let mut groups = vec![ScopeDomain {
        name: "Jobs".to_string(),
        description: Some("Job management".to_string()),
        scopes: vec![
            ScopeOption {
                value: "jobs:read".to_string(),
                label: "Read".to_string(),
                requires_resource_path: false,
            },
            ScopeOption {
                value: "jobs:write".to_string(),
                label: "Write".to_string(),
                requires_resource_path: false,
            },
            ScopeOption {
                value: "jobs:run:scripts".to_string(),
                label: "Run scripts".to_string(),
                requires_resource_path: true,
            },
            ScopeOption {
                value: "jobs:run:flows".to_string(),
                label: "Run flows".to_string(),
                requires_resource_path: true,
            },
        ],
    }];

    groups.extend(standard_scope_domains());
    groups.extend(trigger_scope_domains());

    groups
}

pub fn global_service() -> Router {
    Router::new().route("/list/scopes", get(get_all_available_scopes))
}

async fn get_all_available_scopes() -> JsonResult<Vec<ScopeDomain>> {
    Ok(Json(get_scopes()))
}
