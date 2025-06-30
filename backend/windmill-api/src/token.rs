use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use windmill_common::error::JsonResult;

fn trigger_scope_options_grouped() -> Vec<ScopeOption> {
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

    let mut out = Vec::new();
    for (domain, display_name) in TRIGGER_DOMAINS {
        // Read action
        out.push(ScopeOption {
            value: format!("{domain}:read"),
            label: format!("List & Get {} Triggers", display_name),
            description: Some(format!(
                "List {} triggers and get trigger configurations",
                display_name.to_lowercase()
            )),
            requires_resource_path: false,
        });

        // Write action (includes create, update, run)
        out.push(ScopeOption {
            value: format!("{domain}:write"),
            label: format!("Create, Update & Run {} Triggers", display_name),
            description: Some(format!(
                "Create, update, and run {} triggers",
                display_name.to_lowercase()
            )),
            requires_resource_path: false,
        });

        // Delete action
        out.push(ScopeOption {
            value: format!("{domain}:delete"),
            label: format!("Delete {} Triggers", display_name),
            description: Some(format!(
                "Remove {} triggers from the workspace",
                display_name.to_lowercase()
            )),
            requires_resource_path: false,
        });

        // Admin action (all operations for the trigger domain)
        out.push(ScopeOption {
            value: format!("{domain}:admin"),
            label: format!("Full {} Trigger Administration", display_name),
            description: Some(format!(
                "All operations for {} triggers including system settings",
                display_name.to_lowercase()
            )),
            requires_resource_path: false,
        });
    }
    out
}

fn get_grouped_scopes() -> GroupedScopesResponse {
    let groups = vec![
        ScopeGroup {
            name: "Scripts & Flows".to_string(),
            description: Some("Access to automation scripts and workflows".to_string()),
            scopes: vec![
                ScopeOption {
                    value: "scripts:read".to_string(),
                    label: "List and Get Scripts".to_string(),
                    description: Some("Read script definitions and metadata".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "scripts:write".to_string(),
                    label: "Create and Update Scripts".to_string(),
                    description: Some("Create, update, and manage scripts".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "scripts:run".to_string(),
                    label: "Run Scripts".to_string(),
                    description: Some("Run scripts and automation tasks".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "scripts:delete".to_string(),
                    label: "Archive and Delete Scripts".to_string(),
                    description: Some("Remove scripts from the workspace".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "scripts:admin".to_string(),
                    label: "Full Script Administration".to_string(),
                    description: Some(
                        "Complete access to all script operations and system settings".to_string(),
                    ),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "flows:read".to_string(),
                    label: "List and Get Flows".to_string(),
                    description: Some(
                        "Read workflow definitions and execution history".to_string(),
                    ),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "flows:write".to_string(),
                    label: "Create and Update Flows".to_string(),
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
                    label: "Archive and Delete Flows".to_string(),
                    description: Some("Remove workflows from the workspace".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "flows:admin".to_string(),
                    label: "Full Flow Administration".to_string(),
                    description: Some(
                        "Complete access to all flow operations and system settings".to_string(),
                    ),
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
                    label: "List Jobs, Get Status & Logs".to_string(),
                    description: Some("Read job status, logs, and execution history".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "jobs:write".to_string(),
                    label: "Create, Update & Cancel Jobs".to_string(),
                    description: Some("Create, update, cancel and schedule jobs".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "jobs:run".to_string(),
                    label: "Run & Restart Jobs".to_string(),
                    description: Some("Run jobs and trigger job execution".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "jobs:delete".to_string(),
                    label: "Cancel/Delete Jobs".to_string(),
                    description: Some("Cancel running jobs and delete job records".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "jobs:admin".to_string(),
                    label: "Full Job Administration".to_string(),
                    description: Some(
                        "Complete access to all job operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "jobs_u:read".to_string(),
                    label: "Public Job Status & Logs".to_string(),
                    description: Some(
                        "Access job status, logs, and results without full authentication"
                            .to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "jobs_u:run".to_string(),
                    label: "Resume & Cancel Public Jobs".to_string(),
                    description: Some("Run public jobs with limited permissions".to_string()),
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
                    label: "List and Get Apps".to_string(),
                    description: Some(
                        "Read application definitions and configurations".to_string(),
                    ),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "apps:write".to_string(),
                    label: "Create and Update Apps".to_string(),
                    description: Some("Create, update, and configure applications".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "apps:run".to_string(),
                    label: "Run App Components".to_string(),
                    description: Some(
                        "Run app components and interact with interfaces".to_string(),
                    ),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "apps:delete".to_string(),
                    label: "Delete Apps".to_string(),
                    description: Some("Remove applications from the workspace".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "apps:admin".to_string(),
                    label: "Full App Administration".to_string(),
                    description: Some(
                        "Complete access to all app operations and system settings".to_string(),
                    ),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "raw_apps:read".to_string(),
                    label: "List and Get Raw Apps".to_string(),
                    description: Some("Access raw application data and configurations".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "raw_apps:write".to_string(),
                    label: "Create and Update Raw Apps".to_string(),
                    description: Some("Create and modify raw application data".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "raw_apps:delete".to_string(),
                    label: "Delete Raw Apps".to_string(),
                    description: Some("Remove raw applications from the workspace".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "raw_apps:admin".to_string(),
                    label: "Full Raw App Administration".to_string(),
                    description: Some(
                        "Complete access to all raw app operations and system settings".to_string(),
                    ),
                    requires_resource_path: true,
                },
            ],
        },
        ScopeGroup {
            name: "Resources & Configuration".to_string(),
            description: Some(
                "External resources, variables, and workspace configuration".to_string(),
            ),
            scopes: vec![
                ScopeOption {
                    value: "resources:read".to_string(),
                    label: "List and Get Resources".to_string(),
                    description: Some(
                        "Read external resource configurations and connections".to_string(),
                    ),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "resources:write".to_string(),
                    label: "Create and Update Resources".to_string(),
                    description: Some(
                        "Create and update external resource connections".to_string(),
                    ),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "resources:delete".to_string(),
                    label: "Delete Resources".to_string(),
                    description: Some("Remove resource connections from the workspace".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "resources:admin".to_string(),
                    label: "Full Resource Administration".to_string(),
                    description: Some(
                        "Complete access to all resource operations and system settings"
                            .to_string(),
                    ),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "variables:read".to_string(),
                    label: "List and Get Variables".to_string(),
                    description: Some("Read variable values and configurations".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "variables:write".to_string(),
                    label: "Create and Update Variables".to_string(),
                    description: Some("Create, update, and configure variables".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "variables:delete".to_string(),
                    label: "Delete Variables".to_string(),
                    description: Some("Remove variables from the workspace".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "variables:admin".to_string(),
                    label: "Full Variable Administration".to_string(),
                    description: Some(
                        "Complete access to all variable operations and system settings"
                            .to_string(),
                    ),
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
                    label: "List and Get Schedules".to_string(),
                    description: Some("Read scheduled task configurations and history".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "schedules:write".to_string(),
                    label: "Create and Update Schedules".to_string(),
                    description: Some("Create, update, and configure scheduled tasks".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "schedules:delete".to_string(),
                    label: "Delete Schedules".to_string(),
                    description: Some("Remove scheduled tasks from the workspace".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "schedules:admin".to_string(),
                    label: "Full Schedule Administration".to_string(),
                    description: Some(
                        "Complete access to all schedule operations and system settings"
                            .to_string(),
                    ),
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
                    label: "List Users & Get Profile Info".to_string(),
                    description: Some("Read user profiles and account information".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "users:write".to_string(),
                    label: "Invite, Update & Manage Users".to_string(),
                    description: Some("Invite, update, and manage user accounts".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "users:delete".to_string(),
                    label: "Delete User Accounts".to_string(),
                    description: Some("Delete user accounts from the workspace".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "users:admin".to_string(),
                    label: "Full User Administration".to_string(),
                    description: Some(
                        "Complete access to all user operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "groups:read".to_string(),
                    label: "List Groups & Get Memberships".to_string(),
                    description: Some("Read group configurations and memberships".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "groups:write".to_string(),
                    label: "Create Groups & Manage Memberships".to_string(),
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
                    value: "groups:admin".to_string(),
                    label: "Full Group Administration".to_string(),
                    description: Some(
                        "Complete access to all group operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "folders:read".to_string(),
                    label: "List Folders & Get Permissions".to_string(),
                    description: Some("List folders and their contents".to_string()),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "folders:write".to_string(),
                    label: "Create Folders & Update Metadata".to_string(),
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
                    value: "folders:admin".to_string(),
                    label: "Full Folder Administration".to_string(),
                    description: Some(
                        "Complete access to all folder operations and system settings".to_string(),
                    ),
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "workspaces:read".to_string(),
                    label: "Get Workspace Settings & Invites".to_string(),
                    description: Some("Read workspace metadata and configuration".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "workspaces:write".to_string(),
                    label: "Update Workspace Settings & Invite Users".to_string(),
                    description: Some("Update workspace settings and configuration".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "workspaces:delete".to_string(),
                    label: "Archive Workspace & Delete Invites".to_string(),
                    description: Some("Archive workspace and delete invites".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "workspaces:admin".to_string(),
                    label: "Full Workspace Administration".to_string(),
                    description: Some(
                        "Complete access to all workspace operations and system settings"
                            .to_string(),
                    ),
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
                    label: "List & Get Audit Logs".to_string(),
                    description: Some("Access audit trails and system activity logs".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "audit:write".to_string(),
                    label: "Create Audit Log Entries".to_string(),
                    description: Some("Create audit log entries".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "audit:delete".to_string(),
                    label: "Delete Old Audit Logs".to_string(),
                    description: Some("Delete old audit logs per retention policies".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "audit:admin".to_string(),
                    label: "Full Audit Administration".to_string(),
                    description: Some(
                        "Complete access to all audit operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "workers:read".to_string(),
                    label: "List Workers & Get Status".to_string(),
                    description: Some("Read worker status and execution information".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "workers:write".to_string(),
                    label: "Update Worker Configuration".to_string(),
                    description: Some("Update worker configuration".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "workers:admin".to_string(),
                    label: "Full Worker Administration".to_string(),
                    description: Some(
                        "Complete access to all worker operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "settings:read".to_string(),
                    label: "Get Workspace & Global Settings".to_string(),
                    description: Some(
                        "Read workspace and global configuration settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "settings:write".to_string(),
                    label: "Update Workspace & System Configuration".to_string(),
                    description: Some("Update workspace and system configuration".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "settings:admin".to_string(),
                    label: "Full Settings Administration".to_string(),
                    description: Some(
                        "Complete access to all settings operations and system configuration"
                            .to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "service_logs:read".to_string(),
                    label: "List Log Files & Get Service Logs".to_string(),
                    description: Some("Access system service logs".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "service_logs:admin".to_string(),
                    label: "Full Service Log Administration".to_string(),
                    description: Some(
                        "Complete access to all service log operations and system settings"
                            .to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "configs:read".to_string(),
                    label: "Get System Configuration".to_string(),
                    description: Some("Read system configuration data".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "configs:write".to_string(),
                    label: "Update System Configuration".to_string(),
                    description: Some("Update system configuration".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "configs:admin".to_string(),
                    label: "Full Config Administration".to_string(),
                    description: Some(
                        "Complete access to all config operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "oauth:read".to_string(),
                    label: "List OAuth Clients & Get Configuration".to_string(),
                    description: Some("Read OAuth configurations".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "oauth:write".to_string(),
                    label: "Create & Update OAuth Clients".to_string(),
                    description: Some("Create and update OAuth clients and settings".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "oauth:delete".to_string(),
                    label: "Delete OAuth Clients".to_string(),
                    description: Some("Delete OAuth clients".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "oauth:admin".to_string(),
                    label: "Full OAuth Administration".to_string(),
                    description: Some(
                        "Complete access to all OAuth operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "ai:read".to_string(),
                    label: "Get AI Configuration".to_string(),
                    description: Some("Access AI service configurations".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "ai:write".to_string(),
                    label: "Update AI Settings & Submit Requests".to_string(),
                    description: Some("Update AI settings and submit requests".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "ai:admin".to_string(),
                    label: "Full AI Administration".to_string(),
                    description: Some(
                        "Complete access to all AI operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "agent_workers:read".to_string(),
                    label: "List Agent Workers & Get Status".to_string(),
                    description: Some("List agent workers and get agent status".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "agent_workers:write".to_string(),
                    label: "Create Agent Tokens & Update Configuration".to_string(),
                    description: Some("Create agent tokens and update configuration".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "agent_workers:delete".to_string(),
                    label: "Remove Agents & Blacklist Tokens".to_string(),
                    description: Some("Remove agents and blacklist tokens".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "agent_workers:admin".to_string(),
                    label: "Full Agent Worker Administration".to_string(),
                    description: Some(
                        "Complete access to all agent worker operations and system settings"
                            .to_string(),
                    ),
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
                    label: "List & Get Draft Content".to_string(),
                    description: Some("List drafts and get draft content".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "drafts:write".to_string(),
                    label: "Create & Update Draft Versions".to_string(),
                    description: Some("Create and update draft versions".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "drafts:delete".to_string(),
                    label: "Delete Drafts".to_string(),
                    description: Some("Remove draft versions".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "drafts:admin".to_string(),
                    label: "Full Draft Administration".to_string(),
                    description: Some(
                        "Complete access to all draft operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "favorites:write".to_string(),
                    label: "Star & Unstar Items".to_string(),
                    description: Some("Add or remove items from favorites".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "inputs:read".to_string(),
                    label: "List Input Templates & Get Schemas".to_string(),
                    description: Some("Access saved input templates and forms".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "inputs:write".to_string(),
                    label: "Create & Update Input Templates".to_string(),
                    description: Some("Create and update input templates".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "inputs:delete".to_string(),
                    label: "Delete Input Templates".to_string(),
                    description: Some("Remove input templates".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "inputs:admin".to_string(),
                    label: "Full Input Administration".to_string(),
                    description: Some(
                        "Complete access to all input operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "job_helpers:read".to_string(),
                    label: "Get Job Helper Data & Query Files".to_string(),
                    description: Some("Get job helper data and query stored files".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "job_helpers:write".to_string(),
                    label: "Store Files & Update Helper Configuration".to_string(),
                    description: Some("Store files and update helper configuration".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "job_helpers:delete".to_string(),
                    label: "Delete Job Helpers".to_string(),
                    description: Some("Delete stored files".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "job_helpers:admin".to_string(),
                    label: "Full Job Helper Administration".to_string(),
                    description: Some(
                        "Complete access to all job helper operations and system settings"
                            .to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "openapi:read".to_string(),
                    label: "Get OpenAPI Specifications".to_string(),
                    description: Some("Get OpenAPI specifications and generate docs".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "openapi:write".to_string(),
                    label: "Generate & Download API Documentation".to_string(),
                    description: Some("Update and generate API documentation".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "openapi:admin".to_string(),
                    label: "Full OpenAPI Administration".to_string(),
                    description: Some(
                        "Complete access to all OpenAPI operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "capture:read".to_string(),
                    label: "List Captured Requests & Get Data".to_string(),
                    description: Some("Access webhook capture data".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "capture:write".to_string(),
                    label: "Create Capture Configurations".to_string(),
                    description: Some("Configure webhook capture settings".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "capture:delete".to_string(),
                    label: "Delete Capture".to_string(),
                    description: Some("Remove webhook capture configurations".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "capture:admin".to_string(),
                    label: "Full Capture Administration".to_string(),
                    description: Some(
                        "Complete access to all capture operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "concurrency_groups:read".to_string(),
                    label: "List Concurrency Groups & Get Status".to_string(),
                    description: Some("Monitor concurrency groups".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "concurrency_groups:write".to_string(),
                    label: "Create & Update Concurrency Groups".to_string(),
                    description: Some("Create and update concurrency groups".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "concurrency_groups:delete".to_string(),
                    label: "Delete Concurrency Groups".to_string(),
                    description: Some("Delete concurrency groups".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "concurrency_groups:admin".to_string(),
                    label: "Full Concurrency Groups Administration".to_string(),
                    description: Some(
                        "Complete access to all concurrency groups operations and system settings"
                            .to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "oidc:read".to_string(),
                    label: "Get OIDC Configuration & Status".to_string(),
                    description: Some("Get OIDC configuration and view status".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "oidc:write".to_string(),
                    label: "Update OIDC Settings & Generate Tokens".to_string(),
                    description: Some("Update OIDC settings and generate tokens".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "oidc:admin".to_string(),
                    label: "Full OIDC Administration".to_string(),
                    description: Some(
                        "Complete access to all OIDC operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "acls:read".to_string(),
                    label: "Get Access Control Lists".to_string(),
                    description: Some("Access granular access control lists".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "acls:write".to_string(),
                    label: "Add & Remove ACL Entries".to_string(),
                    description: Some("Configure access control lists".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "acls:delete".to_string(),
                    label: "Delete ACLs".to_string(),
                    description: Some("Remove access control list entries".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "acls:admin".to_string(),
                    label: "Full ACL Administration".to_string(),
                    description: Some(
                        "Complete access to all ACL operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "indexer:read".to_string(),
                    label: "Search Jobs & Service Logs".to_string(),
                    description: Some("Search through job execution and service log indices".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "indexer:delete".to_string(),
                    label: "Delete Search Indices".to_string(),
                    description: Some("Delete and rebuild search indices".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "indexer:admin".to_string(),
                    label: "Full Search Index Administration".to_string(),
                    description: Some(
                        "Complete access to all search index operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "teams:write".to_string(),
                    label: "Sync Teams & Send Notifications".to_string(),
                    description: Some("Sync teams data and send activity notifications".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "teams:admin".to_string(),
                    label: "Full Teams Administration".to_string(),
                    description: Some(
                        "Complete access to all Teams operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "git_sync:read".to_string(),
                    label: "List Git Repositories & Export Installations".to_string(),
                    description: Some("Read git sync configurations and export installation data".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "git_sync:write".to_string(),
                    label: "Install & Configure Git Sync".to_string(),
                    description: Some("Install GitHub apps, configure git sync, and import installations".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "git_sync:delete".to_string(),
                    label: "Delete Git Installations".to_string(),
                    description: Some("Remove git sync installations".to_string()),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "git_sync:admin".to_string(),
                    label: "Full Git Sync Administration".to_string(),
                    description: Some(
                        "Complete access to all git sync operations and system settings".to_string(),
                    ),
                    requires_resource_path: false,
                },
            ],
        },
    ];

    GroupedScopesResponse { groups }
}

pub fn global_service() -> Router {
    Router::new().route("/list/grouped_scopes", get(get_grouped_available_scopes))
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
