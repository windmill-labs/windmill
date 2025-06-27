use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use windmill_common::error::JsonResult;


fn trigger_scope_options_grouped() -> Vec<ScopeOption> {
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
                description: Some(format!("{}  {} triggers", label.to_lowercase(), domain.replace('_', " "))),
                requires_resource_path: false,
            });
        }
    }
    out
}

fn get_grouped_scopes() -> GroupedScopesResponse {
    let groups = vec![
        ScopeGroup {
            name: "Global Permissions".to_string(),
            description: Some("System-wide access controls".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "*".to_string(),
                    label: "Administrator".to_string(),
                    description: Some("Full access to all resources and administrative functions".to_string()),
                    requires_resource_path: false,
                }
            ],
        },
        ScopeGroup {
            name: "Scripts & Flows".to_string(),
            description: Some("Access to automation scripts and workflows".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "scripts:read".to_string(),
                    label: "View Scripts".to_string(),
                    description: Some("Read script definitions and metadata".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "scripts:write".to_string(),
                    label: "Manage Scripts".to_string(),
                    description: Some("Create, update, and manage scripts".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "scripts:run".to_string(),
                    label: "Run Scripts".to_string(),
                    description: Some("Run scripts and run automation tasks".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "scripts:delete".to_string(),
                    label: "Delete Scripts".to_string(),
                    description: Some("Remove scripts from the workspace".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "flows:read".to_string(),
                    label: "View Flows".to_string(),
                    description: Some("Read workflow definitions and execution history".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "flows:write".to_string(),
                    label: "Manage Flows".to_string(),
                    description: Some("Create, update, and manage workflows".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "flows:run".to_string(),
                    label: "Run Flows".to_string(),
                    description: Some("Run workflows and orchestrate tasks".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "flows:delete".to_string(),
                    label: "Delete Flows".to_string(),
                    description: Some("Remove workflows from the workspace".to_string()),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeGroup {
            name: "Jobs & Execution".to_string(),
            description: Some("Job management and execution monitoring".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "jobs:read".to_string(),
                    label: "View Jobs".to_string(),
                    description: Some("Read job status, logs, and execution history".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "jobs:write".to_string(),
                    label: "Manage Jobs".to_string(),
                    description: Some("Create, update, and schedule jobs".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "jobs:run".to_string(),
                    label: "Run Jobs".to_string(),
                    description: Some("Run jobs and trigger execution".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "jobs:delete".to_string(),
                    label: "Cancel/Delete Jobs".to_string(),
                    description: Some("Cancel running jobs and delete job records".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "jobs_u:read".to_string(),
                    label: "View User Jobs".to_string(),
                    description: Some("Read user-specific job status and logs".to_string()),
                    requires_resource_path: false,
                },
            ],
        },
        ScopeGroup {
            name: "Applications".to_string(),
            description: Some("User interface applications and dashboards".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "apps:read".to_string(),
                    label: "View Apps".to_string(),
                    description: Some("Read application definitions and configurations".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "apps:write".to_string(),
                    label: "Manage Apps".to_string(),
                    description: Some("Create, update, and configure applications".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "apps:run".to_string(),
                    label: "Use Apps".to_string(),
                    description: Some("Run app components and interact with interfaces".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "apps:delete".to_string(),
                    label: "Delete Apps".to_string(),
                    description: Some("Remove applications from the workspace".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "raw_apps:read".to_string(),
                    label: "View Raw Apps".to_string(),
                    description: Some("Access raw application data and configurations".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "raw_apps:write".to_string(),
                    label: "Manage Raw Apps".to_string(),
                    description: Some("Create and modify raw application data".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "raw_apps:delete".to_string(),
                    label: "Delete Raw Apps".to_string(),
                    description: Some("Remove raw applications from the workspace".to_string()),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeGroup {
            name: "Resources & Configuration".to_string(),
            description: Some("External resources, variables, and workspace configuration".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "resources:read".to_string(),
                    label: "View Resources".to_string(),
                    description: Some("Read external resource configurations and connections".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "resources:write".to_string(),
                    label: "Manage Resources".to_string(),
                    description: Some("Create and update external resource connections".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "resources:delete".to_string(),
                    label: "Delete Resources".to_string(),
                    description: Some("Remove resource connections from the workspace".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "variables:read".to_string(),
                    label: "View Variables".to_string(),
                    description: Some("Read variable values and configurations".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "variables:write".to_string(),
                    label: "Manage Variables".to_string(),
                    description: Some("Create, update, and configure variables".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "variables:delete".to_string(),
                    label: "Delete Variables".to_string(),
                    description: Some("Remove variables from the workspace".to_string()),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeGroup {
            name: "Scheduling & Automation".to_string(),
            description: Some("Scheduled tasks and automated triggers".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "schedules:read".to_string(),
                    label: "View Schedules".to_string(),
                    description: Some("Read scheduled task configurations and history".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "schedules:write".to_string(),
                    label: "Manage Schedules".to_string(),
                    description: Some("Create, update, and configure scheduled tasks".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "schedules:delete".to_string(),
                    label: "Delete Schedules".to_string(),
                    description: Some("Remove scheduled tasks from the workspace".to_string()),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeGroup {
            name: "Triggers & Webhooks".to_string(),
            description: Some("External event triggers and webhook management".to_string()),
            scopes: trigger_scope_options_grouped(),
        },
        ScopeGroup {
            name: "User & Access Management".to_string(),
            description: Some("User accounts, groups, and permission management".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "users:read".to_string(),
                    label: "View Users".to_string(),
                    description: Some("Read user profiles and account information".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "users:write".to_string(),
                    label: "Manage Users".to_string(),
                    description: Some("Invite, update, and manage user accounts".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "users:delete".to_string(),
                    label: "Remove Users".to_string(),
                    description: Some("Delete user accounts from the workspace".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "groups:read".to_string(),
                    label: "View Groups".to_string(),
                    description: Some("Read group configurations and memberships".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "groups:write".to_string(),
                    label: "Manage Groups".to_string(),
                    description: Some("Create, update, and manage user groups".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "groups:delete".to_string(),
                    label: "Delete Groups".to_string(),
                    description: Some("Remove user groups from the workspace".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "folders:read".to_string(),
                    label: "View Folders".to_string(),
                    description: Some("List folders and their contents".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "folders:write".to_string(),
                    label: "Manage Folders".to_string(),
                    description: Some("Create and modify folder contents".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "folders:delete".to_string(),
                    label: "Delete Folders".to_string(),
                    description: Some("Remove folders from the workspace".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "workspaces:read".to_string(),
                    label: "View Workspace Info".to_string(),
                    description: Some("Read workspace metadata and configuration".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "workspaces:admin".to_string(),
                    label: "Workspace Admin".to_string(),
                    description: Some("Full administrative access to workspace settings".to_string()),
                    requires_resource_path: false,
                },
            ],
        },
        ScopeGroup {
            name: "System & Monitoring".to_string(),
            description: Some("System administration and monitoring tools".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "audit:read".to_string(),
                    label: "View Audit Logs".to_string(),
                    description: Some("Access audit trails and system activity logs".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "workers:read".to_string(),
                    label: "View Workers".to_string(),
                    description: Some("Read worker status and execution information".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "workers:admin".to_string(),
                    label: "Manage Workers".to_string(),
                    description: Some("Control worker processes and resource allocation".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "settings:read".to_string(),
                    label: "View Settings".to_string(),
                    description: Some("Read workspace and global configuration settings".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "settings:write".to_string(),
                    label: "Manage Settings".to_string(),
                    description: Some("Update workspace and system configuration".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "configs:read".to_string(),
                    label: "View Configs".to_string(),
                    description: Some("Read system configuration data".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "configs:write".to_string(),
                    label: "Manage Configs".to_string(),
                    description: Some("Update system configuration".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "oauth:read".to_string(),
                    label: "View OAuth".to_string(),
                    description: Some("Read OAuth configurations".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "oauth:write".to_string(),
                    label: "Manage OAuth".to_string(),
                    description: Some("Configure OAuth settings".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "ai:read".to_string(),
                    label: "View AI".to_string(),
                    description: Some("Access AI service configurations".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "ai:write".to_string(),
                    label: "Manage AI".to_string(),
                    description: Some("Configure AI services".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "service_logs:read".to_string(),
                    label: "View Service Logs".to_string(),
                    description: Some("Access system service logs".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "agent_workers:read".to_string(),
                    label: "View Agent Workers".to_string(),
                    description: Some("Monitor agent worker status".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "agent_workers:admin".to_string(),
                    label: "Manage Agent Workers".to_string(),
                    description: Some("Control agent worker processes".to_string()),
                    requires_resource_path: false,
                },
            ],
        },
        ScopeGroup {
            name: "Developer Tools".to_string(),
            description: Some("Development utilities and API access".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "drafts:read".to_string(),
                    label: "View Drafts".to_string(),
                    description: Some("Access saved draft scripts and flows".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "drafts:write".to_string(),
                    label: "Manage Drafts".to_string(),
                    description: Some("Create and update draft versions".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "favorites:read".to_string(),
                    label: "View Favorites".to_string(),
                    description: Some("Access user's favorite items".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "favorites:write".to_string(),
                    label: "Manage Favorites".to_string(),
                    description: Some("Add or remove items from favorites".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "inputs:read".to_string(),
                    label: "View Input Templates".to_string(),
                    description: Some("Access saved input templates and forms".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "openapi:read".to_string(),
                    label: "API Documentation".to_string(),
                    description: Some("Access OpenAPI specification and documentation".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "capture:read".to_string(),
                    label: "View Capture".to_string(),
                    description: Some("Access webhook capture data".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "capture:write".to_string(),
                    label: "Manage Capture".to_string(),
                    description: Some("Configure webhook capture settings".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "job_helpers:read".to_string(),
                    label: "View Job Helpers".to_string(),
                    description: Some("Access job helper functions".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "concurrency:read".to_string(),
                    label: "View Concurrency".to_string(),
                    description: Some("Monitor concurrency groups".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "concurrency:write".to_string(),
                    label: "Manage Concurrency".to_string(),
                    description: Some("Configure concurrency settings".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "oidc:read".to_string(),
                    label: "View OIDC".to_string(),
                    description: Some("Access OpenID Connect configurations".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "oidc:write".to_string(),
                    label: "Manage OIDC".to_string(),
                    description: Some("Configure OpenID Connect settings".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "acls:read".to_string(),
                    label: "View ACLs".to_string(),
                    description: Some("Access granular access control lists".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "acls:write".to_string(),
                    label: "Manage ACLs".to_string(),
                    description: Some("Configure access control lists".to_string()),
                    requires_resource_path: false,
                },
            ],
        },
    ];
    
    GroupedScopesResponse { groups }

}

pub fn global_service() -> Router {
    Router::new()
        .route("/list/grouped_scopes", get(get_grouped_available_scopes))
}

#[derive(Serialize, Deserialize)]
pub struct ScopeOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
    pub requires_resource_path: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ScopeGroup {
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<ScopeOption>,
}

#[derive(Serialize, Deserialize)]
pub struct GroupedScopesResponse {
    pub groups: Vec<ScopeGroup>,
}


async fn get_grouped_available_scopes() -> JsonResult<GroupedScopesResponse> {
    Ok(Json(get_grouped_scopes()))
}
