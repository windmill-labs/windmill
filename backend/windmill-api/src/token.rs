use axum::{routing::get, Json, Router};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use windmill_common::error::JsonResult;

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ScopeOption {
    pub value: String,
    pub label: String,
    pub requires_resource_path: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScopeDomain {
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<ScopeOption>,
}

fn build_trigger_scope_domains() -> Vec<ScopeDomain> {
    const TRIGGER_DOMAINS: &[(&str, &str)] = &[
        ("http_triggers", "HTTP"),
        ("websocket_triggers", "WebSocket"),
        ("kafka_triggers", "Kafka"),
        ("nats_triggers", "NATS"),
        ("mqtt_triggers", "MQTT"),
        ("sqs_triggers", "AWS SQS"),
        ("gcp_triggers", "GCP Pub/Sub"),
        ("azure_triggers", "Azure Event Grid"),
        ("postgres_triggers", "PostgreSQL"),
        ("email_triggers", "Email"),
    ];

    TRIGGER_DOMAINS
        .iter()
        .map(|(domain, display_name)| ScopeDomain {
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
        })
        .collect()
}

fn build_standard_scope_domains() -> Vec<ScopeDomain> {
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
        (
            "flow_conversations",
            "Flow Conversations",
            "Flow conversation management",
            false,
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
        ("ai_skills", "AI Skills", "AI skill management", false),
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
        (
            "native_triggers",
            "Native Triggers",
            "Native triggers management",
            true,
        ),
    ];

    STANDARD_DOMAINS
        .iter()
        .map(|(key, name, desc, req)| ScopeDomain {
            name: name.to_string(),
            description: if desc.is_empty() {
                None
            } else {
                Some(desc.to_string())
            },
            scopes: vec![
                ScopeOption {
                    value: format!("{key}:read"),
                    label: "Read".to_string(),
                    requires_resource_path: *req,
                },
                ScopeOption {
                    value: format!("{key}:write"),
                    label: "Write".to_string(),
                    requires_resource_path: *req,
                },
            ],
        })
        .collect()
}

lazy_static! {
    static ref ALL_SCOPES: Vec<ScopeDomain> = {
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

        // Read-only: `/api/docs/*` exposes only GET routes, so there is no
        // `docs:write`. Kept out of build_standard_scope_domains (which mints a
        // read+write pair) for that reason.
        groups.push(ScopeDomain {
            name: "Documentation".to_string(),
            description: Some("Read-only documentation search".to_string()),
            scopes: vec![ScopeOption {
                value: "docs:read".to_string(),
                label: "Read".to_string(),
                requires_resource_path: false,
            }],
        });

        // Read-only: the `data_metrics/list` route is the only surface and the
        // catalog is written at deploy, never through a token. Path-selectable
        // because the route filters rows by the caller's `data_metrics:read` path
        // grants. Its own domain, not a `scripts` alias, so a metrics token can't
        // reach `/scripts` routes.
        groups.push(ScopeDomain {
            name: "Data Metrics".to_string(),
            description: Some("Read-only access to declared measures and dimensions".to_string()),
            scopes: vec![ScopeOption {
                value: "data_metrics:read".to_string(),
                label: "Read".to_string(),
                requires_resource_path: true,
            }],
        });

        groups.extend(build_standard_scope_domains());
        groups.extend(build_trigger_scope_domains());

        groups
    };
}

pub fn global_service() -> Router {
    Router::new().route("/list/scopes", get(get_all_available_scopes))
}

async fn get_all_available_scopes() -> JsonResult<Vec<ScopeDomain>> {
    Ok(Json(ALL_SCOPES.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The token-scope picker is driven by this catalog, so a scope that is
    /// enforced but absent here can't be granted through the supported UI.
    #[test]
    fn docs_read_scope_is_exposed_read_only() {
        let values: Vec<&str> = ALL_SCOPES
            .iter()
            .flat_map(|d| d.scopes.iter())
            .map(|s| s.value.as_str())
            .collect();
        assert!(
            values.contains(&"docs:read"),
            "docs:read must be selectable"
        );
        assert!(!values.contains(&"docs:write"), "docs has no write surface");
    }

    /// The `data_metrics` route enforces its own scope domain, so `data_metrics:read`
    /// must be grantable here or no token can ever reach it. It is read-only (the
    /// catalog is written at deploy) and path-selectable.
    #[test]
    fn data_metrics_read_scope_is_exposed_read_only_and_path_selectable() {
        let opt = ALL_SCOPES
            .iter()
            .flat_map(|d| d.scopes.iter())
            .find(|s| s.value == "data_metrics:read")
            .expect("data_metrics:read must be selectable");
        assert!(
            opt.requires_resource_path,
            "data_metrics:read is path-scoped"
        );
        assert!(
            !ALL_SCOPES
                .iter()
                .flat_map(|d| d.scopes.iter())
                .any(|s| s.value == "data_metrics:write"),
            "data_metrics has no write surface"
        );
    }
}
