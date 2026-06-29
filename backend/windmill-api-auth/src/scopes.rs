/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use windmill_common::error::{Error, Result};

/// Comprehensive scope system for JWT token authorization
///
/// Scopes follow the format: {domain}:{action}[:{resource}]
/// Examples:
/// - "jobs:read" - Read access to jobs
/// - "scripts:write:f/folder/*" - Write access to scripts in a folder
/// - "*" - Full access (superuser)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeDefinition {
    pub domain: String,
    pub action: String,
    pub kind: Option<String>, // For jobs:run:kind (optional)
    pub resource: Option<Vec<String>>,
}

impl ScopeDefinition {
    pub fn new(
        domain: &str,
        action: &str,
        kind: Option<&str>,
        resource: Option<Vec<String>>,
    ) -> Self {
        Self {
            domain: domain.to_string(),
            action: action.to_string(),
            kind: kind.map(|s| s.to_string()),
            resource: resource,
        }
    }

    pub fn from_scope_string(scope: &str) -> Result<Self> {
        let parts: Vec<&str> = scope.split(':').collect();

        let into_owned_vec = |resources: &str| -> Vec<String> {
            let resources = resources
                .split(",")
                .collect_vec()
                .into_iter()
                .map(ToOwned::to_owned)
                .collect_vec();

            resources
        };

        match parts.len() {
            2 => Ok(Self::new(parts[0], parts[1], None, None)), // domain:action
            3 => {
                if parts[0] == "jobs" && parts[1] == "run" {
                    Ok(Self::new(parts[0], parts[1], Some(parts[2]), None))
                } else {
                    Ok(Self::new(
                        parts[0],
                        parts[1],
                        None,
                        Some(into_owned_vec(parts[2])),
                    ))
                }
            }
            4 => {
                if parts[0] == "jobs" && parts[1] == "run" {
                    Ok(Self::new(
                        parts[0],
                        parts[1],
                        Some(parts[2]),
                        Some(into_owned_vec(parts[3])),
                    ))
                } else {
                    Err(Error::BadRequest(format!(
                        "Invalid 4-part scope: {}",
                        scope
                    )))
                }
            }
            _ => Err(Error::BadRequest(format!(
                "Invalid scope format: {}",
                scope
            ))),
        }
    }

    pub fn as_string(&self) -> String {
        match (&self.kind, &self.resource) {
            (Some(kind), Some(resource)) => {
                format!(
                    "{}:{}:{}:{}",
                    self.domain,
                    self.action,
                    kind,
                    resource.join(",")
                )
            }
            (Some(kind), None) => {
                format!("{}:{}:{}", self.domain, self.action, kind)
            }
            (None, Some(resource)) => {
                format!("{}:{}:{}", self.domain, self.action, resource.join(","))
            }
            (None, None) => format!("{}:{}", self.domain, self.action),
        }
    }

    pub fn includes(&self, other: &ScopeDefinition) -> bool {
        if self.domain != other.domain {
            return false;
        }

        match (self.action.as_str(), other.action.as_str()) {
            (a, b) if (a == "write" && b == "read") || (a == b) => {}
            _ => return false,
        }

        if self.domain == "jobs" && self.action == "run" {
            match (&self.kind, &other.kind) {
                (Some(self_kind), Some(other_kind)) => {
                    if self_kind != other_kind {
                        return false;
                    }
                }
                (Some(_), None) => {
                    return false;
                }
                (None, _) => {
                    return true;
                }
            }
        }

        match (&self.resource, &other.resource) {
            (Some(self_resources), Some(other_resources)) => {
                resources_match(self_resources, other_resources)
            }
            (Some(_), None) => false,
            (None, _) => true,
        }
    }
}

fn resources_match(scope_resources: &[String], accepted_resources: &[String]) -> bool {
    if scope_resources.contains(&"*".to_string()) || accepted_resources.contains(&"*".to_string()) {
        return true;
    }

    if scope_resources.len() <= 4 && accepted_resources.len() <= 4 {
        return resources_match_small(scope_resources, accepted_resources);
    }

    resources_match_large(scope_resources, accepted_resources)
}

fn resources_match_small(scope_resources: &[String], accepted_resources: &[String]) -> bool {
    for required in accepted_resources {
        for scope_resource in scope_resources {
            if resource_matches_pattern(scope_resource, required) {
                return true;
            }
        }
    }
    false
}

fn resources_match_large(scope_resources: &[String], accepted_resources: &[String]) -> bool {
    let mut exact_matches = HashSet::new();
    let mut patterns = Vec::new();

    for scope_resource in scope_resources {
        if scope_resource.contains('*') {
            patterns.push(scope_resource);
        } else {
            exact_matches.insert(scope_resource);
        }
    }

    for accepted_resource in accepted_resources {
        if exact_matches.contains(accepted_resource) {
            return true;
        }

        for pattern in &patterns {
            if resource_matches_pattern(pattern, accepted_resource) {
                return true;
            }
        }
    }

    false
}

fn resource_matches_pattern(scope_resource: &str, accepted_resource: &str) -> bool {
    if scope_resource == accepted_resource {
        return true;
    }

    let matches_wildcard = |pattern: &str, resource: &str| -> bool {
        if !pattern.ends_with("/*") {
            return false;
        }

        let prefix = &pattern[..pattern.len() - 2];

        if !resource.starts_with(prefix) {
            return false;
        }

        // If the resource is exactly the prefix, it matches
        if resource.len() == prefix.len() {
            return true;
        }

        // If the resource is longer, the next character must be '/' for a valid match
        // This prevents "u/user" from matching "u/use/*"
        resource.chars().nth(prefix.len()) == Some('/')
    };

    // Check if either resource is a wildcard pattern and matches the other
    matches_wildcard(scope_resource, accepted_resource)
        || matches_wildcard(accepted_resource, scope_resource)
}

// ─────────────────────────────────────────────────────────────────
// Route-level scope checking
// ─────────────────────────────────────────────────────────────────

/// Available scope domains (top-level API categories)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeDomain {
    // Core resource domains
    Jobs,
    Scripts,
    Flows,
    FlowConversations,
    Apps,
    Variables,
    Resources,
    Schedules,
    Folders,
    Users,
    Groups,
    Workspaces,

    // Trigger domains
    HttpTriggers,
    WebsocketTriggers,
    KafkaTriggers,
    NatsTriggers,
    MqttTriggers,
    SqsTriggers,
    GcpTriggers,
    AzureTriggers,
    PostgresTriggers,
    EmailTriggers,

    // Native trigger domains
    NativeTriggers,

    // System domains
    Audit,
    Settings,
    Workers,
    ServiceLogs,
    Configs,
    OAuth,
    AI,
    AiSkills,

    Indexer,
    Teams,   // Microsoft Teams integration
    GitSync, // Git synchronization

    // Special domains
    Capture,           // Webhook capture
    Drafts,            // Draft resources
    Favorites,         // User favorites
    Inputs,            // Input templates
    JobHelpers,        // Job helper functions
    ConcurrencyGroups, // Concurrency groups
    Oidc,              // OpenID Connect
    Openapi,           // OpenAPI generation

    // Additional domains
    Acls,         // Granular access control lists
    RawApps,      // Raw application data
    AgentWorkers, // Agent workers management
    Mcp,          // MCP
    Docs,         // Self-hosted documentation search (read-only)
}

impl ScopeDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Jobs => "jobs",
            Self::Scripts => "scripts",
            Self::Flows => "flows",
            Self::FlowConversations => "flow_conversations",
            Self::Apps => "apps",
            Self::Variables => "variables",
            Self::Resources => "resources",
            Self::Schedules => "schedules",
            Self::Folders => "folders",
            Self::Users => "users",
            Self::Groups => "groups",
            Self::Workspaces => "workspaces",
            Self::HttpTriggers => "http_triggers",
            Self::WebsocketTriggers => "websocket_triggers",
            Self::KafkaTriggers => "kafka_triggers",
            Self::NatsTriggers => "nats_triggers",
            Self::MqttTriggers => "mqtt_triggers",
            Self::SqsTriggers => "sqs_triggers",
            Self::GcpTriggers => "gcp_triggers",
            Self::AzureTriggers => "azure_triggers",
            Self::PostgresTriggers => "postgres_triggers",
            Self::EmailTriggers => "email_triggers",
            Self::NativeTriggers => "native_triggers",
            Self::Audit => "audit",
            Self::Settings => "settings",
            Self::Workers => "workers",
            Self::ServiceLogs => "service_logs",
            Self::Configs => "configs",
            Self::OAuth => "oauth",
            Self::AI => "ai",
            Self::AiSkills => "ai_skills",
            Self::Capture => "capture",
            Self::Drafts => "drafts",
            Self::Favorites => "favorites",
            Self::Inputs => "inputs",
            Self::JobHelpers => "job_helpers",
            Self::ConcurrencyGroups => "concurrency_groups",
            Self::Oidc => "oidc",
            Self::Openapi => "openapi",
            Self::Acls => "acls",
            Self::RawApps => "raw_apps",
            Self::AgentWorkers => "agent_workers",
            Self::Indexer => "indexer",
            Self::Teams => "teams",
            Self::GitSync => "git_sync",
            Self::Mcp => "mcp",
            Self::Docs => "docs",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "jobs" | "jobs_u" => Some(Self::Jobs),
            "scripts" => Some(Self::Scripts),
            "flows" => Some(Self::Flows),
            "flow_conversations" => Some(Self::FlowConversations),
            "apps" | "apps_u" => Some(Self::Apps),
            "variables" => Some(Self::Variables),
            "resources" => Some(Self::Resources),
            "schedules" => Some(Self::Schedules),
            "folders" => Some(Self::Folders),
            "users" => Some(Self::Users),
            "groups" => Some(Self::Groups),
            "workspaces" => Some(Self::Workspaces),
            "http_triggers" => Some(Self::HttpTriggers),
            "websocket_triggers" => Some(Self::WebsocketTriggers),
            "kafka_triggers" => Some(Self::KafkaTriggers),
            "nats_triggers" => Some(Self::NatsTriggers),
            "mqtt_triggers" => Some(Self::MqttTriggers),
            "sqs_triggers" => Some(Self::SqsTriggers),
            "gcp_triggers" => Some(Self::GcpTriggers),
            "azure_triggers" => Some(Self::AzureTriggers),
            "postgres_triggers" => Some(Self::PostgresTriggers),
            "email_triggers" => Some(Self::EmailTriggers),
            "audit" => Some(Self::Audit),
            "settings" => Some(Self::Settings),
            "workers" => Some(Self::Workers),
            "service_logs" => Some(Self::ServiceLogs),
            "configs" => Some(Self::Configs),
            "oauth" => Some(Self::OAuth),
            "ai" => Some(Self::AI),
            "ai_skills" => Some(Self::AiSkills),
            "indexer" | "srch" => Some(Self::Indexer),
            "teams" => Some(Self::Teams),
            "native_triggers" => Some(Self::NativeTriggers),
            "git_sync" | "github_app" => Some(Self::GitSync),
            "capture" => Some(Self::Capture),
            "drafts" => Some(Self::Drafts),
            "favorites" => Some(Self::Favorites),
            "inputs" => Some(Self::Inputs),
            "job_helpers" => Some(Self::JobHelpers),
            "concurrency_groups" => Some(Self::ConcurrencyGroups),
            "oidc" => Some(Self::Oidc),
            "openapi" => Some(Self::Openapi),
            "acls" => Some(Self::Acls),
            "raw_apps" => Some(Self::RawApps),
            "agent_workers" => Some(Self::AgentWorkers),
            "mcp" => Some(Self::Mcp),
            "docs" => Some(Self::Docs),
            _ => None,
        }
    }
}

/// Available scope actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeAction {
    Read,  // GET operations, list, view
    Write, // POST, PUT, PATCH, DELETE operations, create, update, delete
    Run,   // Special action for running (scripts, flows, etc.)
}

impl ScopeAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Run => "run",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "delete" => Some(Self::Write),
            "run" => Some(Self::Run),
            _ => None,
        }
    }

    /// Check if this action includes another action
    /// Write includes Read
    pub fn includes(&self, other: &ScopeAction) -> bool {
        match (self, other) {
            (ScopeAction::Write, ScopeAction::Read) => true,
            (ScopeAction::Run, ScopeAction::Read) => true,
            (a, b) => a == b,
        }
    }
}

pub fn check_route_access(
    token_scopes: &[String],
    route_path: &str,
    http_method: &str,
) -> Result<()> {
    // Map HTTP method to scope action (considering route context)
    let required_action = map_http_method_to_action(http_method, route_path);

    // Find the domain and kind for this route
    let (required_domain, required_kind, route_suffix) = extract_domain_from_route(route_path)?;

    // App embed tokens (sentinel) carry broad read scopes (`jobs:read`,
    // `users:read`, `folders:read`) that exist only for a handful of routes. The
    // whole `/users`, `/folders` and `/jobs` routers are CORS-enabled for the
    // opaque app iframe, so default-deny everything in those domains except the
    // intended routes — otherwise the token could enumerate/export workspace data.
    if has_app_embed_sentinel(Some(token_scopes)) {
        if let Some(suffix) = route_suffix.as_deref() {
            if app_embed_route_denied(required_domain, suffix) {
                return Err(Error::PermissionDenied(
                    "Access denied. App embed token cannot access this route.".to_string(),
                ));
            }
            // The by-id job cancel is a POST (write) that the token's `jobs:read`
            // wouldn't satisfy, but cancelling the app's own component runs is
            // intended (most components supersede an in-flight run on re-run). Permit
            // it here; `cancel_job_api` confines it to jobs the app launched
            // (created_by == viewer). A read_only token is still rejected by the
            // separate read-only check.
            if suffix.starts_with("jobs_u/queue/cancel/") {
                return Ok(());
            }
        }
    }

    // MCP scopes (mcp:all, mcp:favorites, mcp:hub:*, etc.) use a custom format
    // that doesn't fit the standard domain:action model. Verify the token has at
    // least one mcp: scope; MCP handlers do their own fine-grained checking.
    if required_domain == ScopeDomain::Mcp {
        let is_scoped_token = token_scopes
            .iter()
            .any(|s| !s.starts_with("if_jobs:filter_tags:"));
        if !is_scoped_token {
            return Ok(());
        }
        if token_scopes.iter().any(|s| s.starts_with("mcp:")) {
            return Ok(());
        }
        return Err(Error::PermissionDenied(
            "Access denied. Required scope: mcp:*".to_string(),
        ));
    }

    // tracing::error!("Checking route access {:?} {:?} {:?} {:?}", required_action, required_domain, required_kind, route_suffix);
    let mut is_scoped_token = false;
    // Check if any token scope grants the required access
    for scope_str in token_scopes {
        if !scope_str.starts_with("if_jobs:filter_tags:") {
            if let Ok(scope) = ScopeDefinition::from_scope_string(scope_str) {
                // tracing::error!("Checking scope {:?} for required domain {:?} and action {:?} and kind {:?} and route suffix {:?}", scope, required_domain, required_action, required_kind, route_suffix);
                if scope_grants_access(
                    &scope,
                    required_domain,
                    required_action,
                    required_kind.as_deref(),
                    route_suffix.as_deref(),
                )? {
                    // tracing::error!("Scope grants access: {:?}", scope);
                    return Ok(());
                }
            }
            if !is_scoped_token {
                is_scoped_token = true;
            }
        }
    }

    //Edge case for backward compatibility, if only scopes defined was filter tag then don't treat this we don't treat the token
    //as a restricted token
    if !is_scoped_token {
        return Ok(());
    }
    let scope_display = if let Some(kind) = required_kind {
        format!(
            "{}:{}:{}",
            required_domain.as_str(),
            required_action.as_str(),
            kind
        )
    } else {
        format!("{}:{}", required_domain.as_str(), required_action.as_str())
    };

    Err(Error::PermissionDenied(format!(
        "Access denied. Required scope: {}",
        scope_display
    )))
}

const SCRIPT_JOBS: [&'static str; 8] = [
    "jobs/run/p",
    "jobs/run/h",
    "jobs/run_wait_result/p",
    "jobs/run_wait_result/h",
    "jobs/run/preview_bundle",
    "jobs/run/preview",
    "jobs/run_and_stream/p",
    "jobs/run_and_stream/h",
];

const FLOW_JOBS: [&'static str; 6] = [
    "jobs/run/f",
    "jobs/run_wait_result/f",
    "jobs/run/preview_flow",
    "jobs/restart/f",
    "jobs/flow/resume",
    "jobs/run_and_stream/f",
];

lazy_static::lazy_static! {
    static ref RUN_PATH_ACTIONS: Vec<&'static str> = {
        let mut v = vec!["jobs/resume/", "jobs/run/batch_rerun_jobs", "jobs/run/workflow_as_code", "jobs/run/dependencies","jobs/run/flow_dependencies", "apps_u/execute_component", "apps_u/upload_s3_file"];

        v.extend(SCRIPT_JOBS);
        v.extend(FLOW_JOBS);
        v
    };
}

fn map_http_method_to_action(method: &str, route_path: &str) -> ScopeAction {
    if RUN_PATH_ACTIONS
        .iter()
        .any(|run_path| route_path.contains(run_path))
    {
        return ScopeAction::Run;
    }

    match method.to_uppercase().as_str() {
        "GET" | "HEAD" | "OPTIONS" => ScopeAction::Read,
        "POST" | "PUT" | "PATCH" | "DELETE" => ScopeAction::Write,
        _ => ScopeAction::Read,
    }
}

/// Checks the route path to determine the runnable kind (either "flows" or "scripts").
///
/// The order of checks is important:
/// - Flow-related paths are checked first to avoid false positives, as some flow paths
///   (e.g., `/run_preview_flow`) share prefixes with script paths (e.g., `/run_preview`).
///
/// Returns `"flows"` or `"scripts"` based on the match, or `None` if no match is found.
fn determine_kind_from_route(route_path: &str) -> Option<String> {
    if route_path.starts_with("jobs") {
        if FLOW_JOBS.iter().any(|path| route_path.starts_with(path)) {
            return Some("flows".to_string());
        } else if SCRIPT_JOBS.iter().any(|path| route_path.starts_with(path)) {
            return Some("scripts".to_string());
        }
    }
    None
}

fn extract_domain_from_route(
    route_path: &str,
) -> Result<(ScopeDomain, Option<String>, Option<String>)> {
    // Examples:
    // - /api/w/workspace/jobs/123 -> jobs domain (workspaced)
    // - /api/teams/sync -> teams domain (global)
    // - /api/srch/index/search -> indexer domain (global)
    let parts: Vec<&str> = route_path.split('/').collect();

    let (domain, kind, route_suffix) = if parts.len() >= 5 && parts[1] == "api" && parts[2] == "w" {
        let domain_part = parts[4];
        let route_suffix = &parts[4..].join("/");

        let domain = ScopeDomain::from_str(domain_part);

        let kind = determine_kind_from_route(&route_suffix);

        (domain, kind, Some(route_suffix.to_owned()))
    } else if parts.len() >= 3 && parts[1] == "api" {
        (
            ScopeDomain::from_str(parts[2]),
            None,
            Some(parts[2..].join("/")),
        )
    } else {
        (None, None, None)
    };

    if let Some(domain) = domain {
        // tracing::error!("Extracted domain {:?} from route {:?} with kind {:?} and route suffix {:?}", domain, route_path, kind, route_suffix);
        return Ok((domain, kind, route_suffix));
    }

    Err(Error::BadRequest(format!(
        "Could not extract domain from route: {}",
        route_path
    )))
}

const RUN_WHITELISTED_GET_PATHS: [&'static str; 20] = [
    "jobs_u/get_flow/",
    "jobs_u/get_root_job_id/",
    "jobs_u/get/",
    "jobs_u/get_logs/",
    "jobs_u/get_flow_all_logs/",
    "jobs_u/get_args/",
    "jobs_u/get_flow_debug_info/",
    "jobs_u/completed/get/",
    "jobs_u/completed/get_result/",
    "jobs_u/completed/get_result_maybe/",
    "jobs_u/getupdate/",
    "jobs_u/getupdate_sse/",
    "jobs_u/get_log_file/",
    "jobs/result_by_id/",
    "jobs/resume_urls/",
    "jobs/flow/user_states/",
    "jobs/job_signature/",
    "jobs/completed/get/",
    "jobs/completed/get_result/",
    "jobs/completed/get_result_maybe/",
];

/// Sentinel scope in app embed tokens. Grants nothing itself; `check_route_access`
/// uses it to deny the workspace-wide job enumeration routes `jobs:read` would
/// otherwise reach, so an embedded app reads only jobs it launched (by id).
pub const APP_EMBED_SENTINEL: &str = "app_embed";

/// True if a token's scopes include the app-embed sentinel (a sandboxed app iframe
/// token). Such tokens carry the viewer's identity but represent untrusted app JS,
/// so several handlers confine them to the app's own resources/runs.
pub fn has_app_embed_sentinel(scopes: Option<&[String]>) -> bool {
    scopes.is_some_and(|s| s.iter().any(|x| x == APP_EMBED_SENTINEL))
}

/// Routes an app embed token (sentinel) is denied. Its broad scopes (`apps:run`,
/// `jobs:read`, `users:read`, `folders:read`) exist only for a fixed set of routes a
/// running app uses, but the whole `/apps`, `/jobs`, `/users`, `/folders` routers are
/// CORS-enabled for the opaque app iframe. Default-deny those domains via an explicit
/// allowlist so the token can't reach workspace inventory, counts, exports, or
/// capability-minting routes (job signatures / resume URLs).
fn app_embed_route_denied(domain: ScopeDomain, suffix: &str) -> bool {
    match domain {
        ScopeDomain::Apps => !app_embed_apps_route_allowed(suffix),
        ScopeDomain::Jobs => !app_embed_job_route_allowed(suffix),
        ScopeDomain::Users => suffix != "users/whoami",
        ScopeDomain::Folders => suffix != "folders/listnames",
        _ => false,
    }
}

/// App routes a running app uses: its own definition (`apps/get/p/<path>`, further
/// path-scoped by `apps:read:<path>`) and the public app-serving endpoints
/// (`apps_u/*`: public_app, public_resource, get_data, and the path-taking
/// `execute_component` / `download_s3_file`, which re-check `apps:run|read:<path>`
/// in their handlers so they stay confined to this app). Everything else in the
/// domain — workspace app inventory (`exists`, `custom_path_exists`, `list`,
/// `list_paths*`, `secret_of`, history, management) — is denied.
fn app_embed_apps_route_allowed(suffix: &str) -> bool {
    // The embed-token mint endpoints live under `apps_u/` but they create
    // credentials. A running app never calls them — the trusted embedder session/JWT
    // mints the token and hands it to the iframe — so deny them here, otherwise an
    // app embed token could renew itself indefinitely past the 12h expiry.
    if suffix.starts_with("apps_u/embed_token") {
        return false;
    }
    suffix.starts_with("apps/get/p/") || suffix.starts_with("apps_u/")
}

/// Job routes a running app uses (the by-id poll/cancel surface driven by the
/// frontend JobLoader). Everything else in the jobs domain — enumeration, counts,
/// exports, and the `job_signature`/`resume_urls` capability-minting routes — is
/// denied. By-id reads are further confined to the app's own runs by
/// `require_job_read_access` (the `app_embed` cutoff).
fn app_embed_job_route_allowed(suffix: &str) -> bool {
    // `get_root_job_id` is intentionally absent: its handler has no access check at
    // all (returns any job's root id by id) and the app never calls it, so denying
    // it costs nothing and avoids leaking a foreign job's flow lineage.
    const ALLOWED: [&str; 15] = [
        "jobs_u/get/",
        "jobs_u/getupdate/",
        "jobs_u/getupdate_sse/",
        "jobs_u/get_logs/",
        "jobs_u/get_completed_logs_tail/",
        "jobs_u/get_args/",
        "jobs_u/get_flow/",
        "jobs_u/get_flow_all_logs/",
        "jobs_u/get_flow_debug_info/",
        "jobs_u/get_log_file/",
        "jobs_u/completed/get/",
        "jobs_u/completed/get_result/",
        "jobs_u/completed/get_result_maybe/",
        "jobs_u/completed/get_timing/",
        "jobs_u/queue/cancel/",
    ];
    ALLOWED.iter().any(|p| suffix.starts_with(p))
}

/// Resource routes a metadata-only `resources:run` scope (app embed tokens) may
/// GET: pickers (`/list`) and type schemas. Excludes every value-returning route
/// (`get`, `get_value`, `get_value_interpolated`, `list_search`) so resource
/// values — which can hold credentials — are never exposed.
fn resource_metadata_route_allowed(suffix: &str) -> bool {
    suffix == "resources/list"
        || suffix.starts_with("resources/list_names/")
        || suffix.starts_with("resources/exists/")
        || suffix.starts_with("resources/type/")
}

fn scope_grants_access(
    scope: &ScopeDefinition,
    required_domain: ScopeDomain,
    required_action: ScopeAction,
    required_kind: Option<&str>,
    route_path: Option<&str>,
) -> Result<bool> {
    // Check domain match
    let scope_domain = ScopeDomain::from_str(&scope.domain)
        .ok_or_else(|| Error::BadRequest(format!("Invalid scope domain: {}", scope.domain)))?;

    if scope_domain != required_domain {
        return Ok(false);
    }

    // Check action match (with hierarchical permissions)
    let scope_action = ScopeAction::from_str(&scope.action)
        .ok_or_else(|| Error::BadRequest(format!("Invalid scope action: {}", scope.action)))?;

    // App embed tokens carry `resources:run`: metadata-only resource access via
    // default-deny + allowlist (so a new value route is never exposed by accident).
    // See `resource_metadata_route_allowed`.
    if scope_domain == ScopeDomain::Resources && scope_action == ScopeAction::Run {
        return Ok(required_action == ScopeAction::Read
            && route_path.is_some_and(resource_metadata_route_allowed));
    }

    if !scope_action.includes(&required_action)
        && !(scope_domain == ScopeDomain::Jobs
            && required_action == ScopeAction::Read
            && route_path.is_some_and(|p| {
                RUN_WHITELISTED_GET_PATHS
                    .iter()
                    .any(|path| p.starts_with(path))
            }))
    {
        return Ok(false);
    }

    if scope_domain == ScopeDomain::Jobs && required_action == ScopeAction::Run {
        match (&scope.kind, required_kind) {
            (Some(scope_kind), Some(req_kind)) => {
                if scope_kind != req_kind {
                    return Ok(false);
                }
            }
            (None, _) => {}
            (Some(_), None) => {
                return Ok(false);
            }
        }
    }

    // No resource specified means access to entire domain
    Ok(true)
}

/// Enforces a token's `read_only` flag: only methods classified as `Read`
/// (GET/HEAD/OPTIONS) are allowed. Run actions and mutating methods are
/// rejected. Independent of `scopes`.
pub fn check_read_only_for_route(route_path: &str, http_method: &str) -> Result<()> {
    if map_http_method_to_action(http_method, route_path) == ScopeAction::Read {
        Ok(())
    } else {
        Err(Error::PermissionDenied(
            "Token is read-only. Mutating endpoints are not allowed.".to_string(),
        ))
    }
}

/// The minimal scope string that grants access to exactly `{method} {path}`, as
/// `check_route_access` would require it. Used to mint a least-privilege JWT for
/// a single proxied request (the MCP endpoint proxy), so the minted token can do
/// only that one operation rather than acting as a blank check.
///
/// `path` is the request path (e.g. `/api/w/{workspace}/variables/get/...`).
/// Returns `None` if the route's domain can't be determined — the caller should
/// then fail closed.
pub fn scope_for_route(method: &str, path: &str) -> Option<String> {
    let action = map_http_method_to_action(method, path);
    let (domain, kind, _suffix) = extract_domain_from_route(path).ok()?;
    Some(match (domain, action, kind) {
        (ScopeDomain::Jobs, ScopeAction::Run, Some(kind)) => format!("jobs:run:{}", kind),
        (domain, action, _) => format!("{}:{}", domain.as_str(), action.as_str()),
    })
}

/// Helper function to check if scopes allow access to a route
pub fn check_scopes_for_route(
    token_scopes: Option<&[String]>,
    route_path: &str,
    http_method: &str,
) -> Result<()> {
    // If no scopes defined, allow access (backward compatibility)
    let scopes = match token_scopes {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(()),
    };

    check_route_access(scopes, route_path, http_method)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_definition_parsing() {
        let scope = ScopeDefinition::from_scope_string("jobs:read").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "read");
        assert_eq!(scope.kind, None);
        assert_eq!(scope.resource, None);

        let scope = ScopeDefinition::from_scope_string("jobs:run:scripts:f/folder/*").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "run");
        assert_eq!(scope.kind, Some("scripts".to_string()));
        assert_eq!(scope.resource, Some(vec!["f/folder/*".to_string()]));

        // Test jobs:run:kind parsing
        let scope = ScopeDefinition::from_scope_string("jobs:run:scripts").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "run");
        assert_eq!(scope.kind, Some("scripts".to_string()));
        assert_eq!(scope.resource, None);

        // Test jobs:run:kind:resource parsing
        let scope = ScopeDefinition::from_scope_string("jobs:run:flows:f/folder/*").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "run");
        assert_eq!(scope.kind, Some("flows".to_string()));
        assert_eq!(scope.resource, Some(vec!["f/folder/*".to_string()]));

        // Test comma-separated resources parsing
        let scope =
            ScopeDefinition::from_scope_string("scripts:read:path1,path2,f/folder/*").unwrap();
        assert_eq!(scope.domain, "scripts");
        assert_eq!(scope.action, "read");
        assert_eq!(scope.kind, None);
        assert_eq!(
            scope.resource,
            Some(vec![
                "path1".to_string(),
                "path2".to_string(),
                "f/folder/*".to_string()
            ])
        );
    }

    #[test]
    fn test_scope_action_hierarchy() {
        assert!(ScopeAction::Write.includes(&ScopeAction::Read));
        assert!(!ScopeAction::Read.includes(&ScopeAction::Write));
        assert!(ScopeAction::Run.includes(&ScopeAction::Read));
        assert!(!ScopeAction::Run.includes(&ScopeAction::Write));
    }

    #[test]
    fn test_route_domain_extraction() {
        let (domain, kind, route_suffix) =
            extract_domain_from_route("/api/w/test_workspace/jobs/123").unwrap();
        assert_eq!(domain, ScopeDomain::Jobs);
        assert_eq!(kind, None);
        assert_eq!(route_suffix, Some("jobs/123".to_string()));

        let (domain, kind, route_suffix) =
            extract_domain_from_route("/api/w/test_workspace/scripts/test_script").unwrap();
        assert_eq!(domain, ScopeDomain::Scripts);
        assert_eq!(kind, None);
        assert_eq!(route_suffix, Some("scripts/test_script".to_string()));

        let (domain, kind, route_suffix) =
            extract_domain_from_route("/api/w/test_workspace/flow_conversations/list").unwrap();
        assert_eq!(domain, ScopeDomain::FlowConversations);
        assert_eq!(kind, None);
        assert_eq!(route_suffix, Some("flow_conversations/list".to_string()));

        let (domain, kind, route_suffix) =
            extract_domain_from_route("/api/w/test_workspace/ai_skills/list").unwrap();
        assert_eq!(domain, ScopeDomain::AiSkills);
        assert_eq!(kind, None);
        assert_eq!(route_suffix, Some("ai_skills/list".to_string()));
    }

    #[test]
    fn test_check_read_only_for_route() {
        // Plain GETs pass.
        assert!(check_read_only_for_route("/api/w/x/scripts/list", "GET").is_ok());
        assert!(check_read_only_for_route("/api/w/x/scripts/get/foo", "HEAD").is_ok());
        assert!(check_read_only_for_route("/api/w/x/anything", "OPTIONS").is_ok());

        // Mutating methods are rejected.
        assert!(check_read_only_for_route("/api/w/x/scripts/create", "POST").is_err());
        assert!(check_read_only_for_route("/api/w/x/scripts/update", "PUT").is_err());
        assert!(check_read_only_for_route("/api/w/x/scripts/delete", "DELETE").is_err());
        assert!(check_read_only_for_route("/api/w/x/scripts/patch", "PATCH").is_err());

        // Run paths are rejected even on GET (map_http_method_to_action elevates
        // them to Run via RUN_PATH_ACTIONS).
        assert!(check_read_only_for_route("/api/w/x/jobs/run/p/f/foo", "GET").is_err());
        assert!(check_read_only_for_route("/api/w/x/jobs/run/p/f/foo", "POST").is_err());

        // OAuth/registration endpoints under /api/mcp/* must NOT be exempted by
        // the auth middleware — they go through this check on the gateway side
        // because they can mint non-read-only tokens. The middleware decides
        // which paths to exempt; this helper is method-only, so we just assert
        // that mutating methods still fail.
        assert!(
            check_read_only_for_route("/api/mcp/gateway/oauth/server/approve", "POST").is_err()
        );
    }

    #[test]
    fn test_specific_scope_access() {
        let scopes = vec!["jobs:read".to_string()];

        assert!(check_route_access(&scopes, "/api/w/test_workspace/jobs/123", "GET").is_ok());

        // DELETE now requires write permission, so it should still fail with read-only scope
        assert!(check_route_access(&scopes, "/api/w/test_workspace/jobs/123", "DELETE").is_err());
    }

    #[test]
    fn test_new_domain_parsing() {
        // Test that new domains are properly parsed
        assert_eq!(ScopeDomain::from_str("acls"), Some(ScopeDomain::Acls));
        assert_eq!(
            ScopeDomain::from_str("raw_apps"),
            Some(ScopeDomain::RawApps)
        );
        assert_eq!(
            ScopeDomain::from_str("agent_workers"),
            Some(ScopeDomain::AgentWorkers)
        );
        assert_eq!(
            ScopeDomain::from_str("flow_conversations"),
            Some(ScopeDomain::FlowConversations)
        );
        assert_eq!(
            ScopeDomain::from_str("ai_skills"),
            Some(ScopeDomain::AiSkills)
        );

        // Test canonical string conversion
        assert_eq!(ScopeDomain::Acls.as_str(), "acls");
        assert_eq!(ScopeDomain::RawApps.as_str(), "raw_apps");
        assert_eq!(ScopeDomain::AgentWorkers.as_str(), "agent_workers");
        assert_eq!(
            ScopeDomain::FlowConversations.as_str(),
            "flow_conversations"
        );
        assert_eq!(ScopeDomain::AiSkills.as_str(), "ai_skills");
    }

    #[test]
    fn test_ai_skills_scope_access() {
        let read_scopes = vec!["ai_skills:read".to_string()];
        assert!(
            check_route_access(&read_scopes, "/api/w/test_workspace/ai_skills/list", "GET").is_ok()
        );
        assert!(check_route_access(
            &read_scopes,
            "/api/w/test_workspace/ai_skills/get/foo",
            "GET"
        )
        .is_ok());
        assert!(check_route_access(
            &read_scopes,
            "/api/w/test_workspace/ai_skills/upload",
            "POST"
        )
        .is_err());

        let write_scopes = vec!["ai_skills:write".to_string()];
        assert!(check_route_access(
            &write_scopes,
            "/api/w/test_workspace/ai_skills/upload",
            "POST"
        )
        .is_ok());
        assert!(check_route_access(
            &write_scopes,
            "/api/w/test_workspace/ai_skills/delete/foo",
            "DELETE"
        )
        .is_ok());
    }

    #[test]
    fn test_flow_conversations_scope_access() {
        let read_scopes = vec!["flow_conversations:read".to_string()];
        assert!(check_route_access(
            &read_scopes,
            "/api/w/test_workspace/flow_conversations/list",
            "GET"
        )
        .is_ok());
        assert!(check_route_access(
            &read_scopes,
            "/api/w/test_workspace/flow_conversations/123/messages",
            "GET"
        )
        .is_ok());
        assert!(check_route_access(
            &read_scopes,
            "/api/w/test_workspace/flow_conversations/delete/123",
            "DELETE"
        )
        .is_err());

        let write_scopes = vec!["flow_conversations:write".to_string()];
        assert!(check_route_access(
            &write_scopes,
            "/api/w/test_workspace/flow_conversations/delete/123",
            "DELETE"
        )
        .is_ok());
    }

    #[test]
    fn test_resource_array_matching() {
        // Test wildcard access
        let scope_all = ScopeDefinition::new("scripts", "read", None, Some(vec!["*".to_string()]));
        let required = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["path1".to_string(), "path2".to_string()]),
        );
        assert!(scope_all.includes(&required));

        // Test exact matches
        let scope_exact = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["path1".to_string(), "path2".to_string()]),
        );
        let required_subset =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["path1".to_string()]));
        assert!(scope_exact.includes(&required_subset));

        // Test partial match - should grant access if ANY required resource matches
        let scope_limited =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["path1".to_string()]));
        let required_partial = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["path1".to_string(), "path2".to_string()]),
        );
        assert!(scope_limited.includes(&required_partial)); // path1 matches, so access granted

        // Test no match - scope doesn't cover any of the required resources
        let scope_different =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["path3".to_string()]));
        let required_no_match = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["path1".to_string(), "path2".to_string()]),
        );
        assert!(!scope_different.includes(&required_no_match));

        // Test pattern matching
        let scope_pattern = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["f/folder/*".to_string()]),
        );
        let required_in_folder = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["f/folder/script1".to_string()]),
        );
        assert!(scope_pattern.includes(&required_in_folder));

        // Test mixed patterns and exact matches
        let scope_mixed = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["exact_path".to_string(), "f/folder/*".to_string()]),
        );
        let required_mixed1 = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["exact_path".to_string()]),
        );
        let required_mixed2 = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["f/folder/script2".to_string()]),
        );
        assert!(scope_mixed.includes(&required_mixed1));
        assert!(scope_mixed.includes(&required_mixed2));
    }

    #[test]
    fn test_efficiency_small_vs_large_arrays() {
        // Test small array optimization path
        let scope_small = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["path1".to_string(), "path2".to_string()]),
        );
        let required_small =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["path1".to_string()]));
        assert!(scope_small.includes(&required_small));

        // Test large array optimization path
        let large_scope_vec: Vec<String> = (0..10).map(|i| format!("path{}", i)).collect();
        let scope_large = ScopeDefinition::new("scripts", "read", None, Some(large_scope_vec));
        let required_large =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["path5".to_string()]));
        assert!(scope_large.includes(&required_large));
    }

    #[test]
    fn test_user_example_case() {
        let user_scope =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["u/dieri/*".to_string()]));
        let required_mixed = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["u/dadad/wqdq".to_string(), "u/*".to_string()]),
        );
        assert!(user_scope.includes(&required_mixed));

        let scope_specific = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["folder/file1".to_string()]),
        );
        let required_multi = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["folder/file1".to_string(), "other/file2".to_string()]),
        );
        assert!(scope_specific.includes(&required_multi));

        let scope_broad =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["u/*".to_string()]));
        let required_specific = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["u/dieri/script.py".to_string()]),
        );
        assert!(scope_broad.includes(&required_specific));

        let scope_specific_path = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["u/dieri/script.py".to_string()]),
        );
        let required_broad =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["u/*".to_string()]));
        assert!(scope_specific_path.includes(&required_broad));
    }

    #[test]
    fn test_mcp_scope_bypass_blocked_without_mcp_scope() {
        // A token with only jobs:read should NOT be able to access MCP endpoints
        let scopes = vec!["jobs:read".to_string()];
        assert!(check_route_access(&scopes, "/api/w/test_workspace/mcp/something", "GET").is_err());
        assert!(
            check_route_access(&scopes, "/api/w/test_workspace/mcp/something", "POST").is_err()
        );
    }

    #[test]
    fn test_mcp_scope_allowed_with_mcp_scope() {
        // A token with mcp:all should access MCP endpoints
        let scopes = vec!["mcp:all".to_string()];
        assert!(check_route_access(&scopes, "/api/w/test_workspace/mcp/something", "GET").is_ok());

        // mcp:favorites should also work
        let scopes = vec!["mcp:favorites".to_string()];
        assert!(check_route_access(&scopes, "/api/w/test_workspace/mcp/something", "POST").is_ok());

        // mcp:scripts:path should also work
        let scopes = vec!["mcp:scripts:u/admin/script1".to_string()];
        assert!(check_route_access(&scopes, "/api/w/test_workspace/mcp/something", "GET").is_ok());
    }

    #[test]
    fn test_mcp_scope_filter_tags_only_treated_as_unrestricted() {
        // Token with only filter_tags is not considered scoped — should be allowed
        let scopes = vec!["if_jobs:filter_tags:tag1".to_string()];
        assert!(check_route_access(&scopes, "/api/w/test_workspace/mcp/something", "GET").is_ok());
    }

    #[test]
    fn test_mcp_scope_mixed_scopes_without_mcp() {
        // Token with multiple non-MCP scopes should be denied
        let scopes = vec!["jobs:read".to_string(), "scripts:write".to_string()];
        assert!(check_route_access(&scopes, "/api/w/test_workspace/mcp/something", "GET").is_err());
    }

    #[test]
    fn test_mcp_scope_mixed_scopes_with_mcp() {
        // Token with MCP scope + other scopes should be allowed for MCP
        let scopes = vec!["jobs:read".to_string(), "mcp:all".to_string()];
        assert!(check_route_access(&scopes, "/api/w/test_workspace/mcp/something", "GET").is_ok());
    }

    #[test]
    fn test_scope_for_route() {
        // The minted scope must be exactly what check_route_access requires for
        // the same route, so a JWT carrying it passes for that one route only.
        assert_eq!(
            scope_for_route("GET", "/api/w/ws/variables/get/u/x/y").as_deref(),
            Some("variables:read")
        );
        assert_eq!(
            scope_for_route("POST", "/api/w/ws/variables/create").as_deref(),
            Some("variables:write")
        );
        assert_eq!(
            scope_for_route("DELETE", "/api/w/ws/resources/delete/u/x/y").as_deref(),
            Some("resources:write")
        );
        // jobs run paths carry the runnable kind.
        assert_eq!(
            scope_for_route("POST", "/api/w/ws/jobs/run/p/u/x/y").as_deref(),
            Some("jobs:run:scripts")
        );
        assert_eq!(
            scope_for_route("POST", "/api/w/ws/jobs/run/f/u/x/y").as_deref(),
            Some("jobs:run:flows")
        );

        // The minted scope actually satisfies the route check it targets.
        let s = scope_for_route("POST", "/api/w/ws/variables/create").unwrap();
        assert!(check_route_access(&[s], "/api/w/ws/variables/create", "POST").is_ok());

        // Unknown route -> None so the caller fails closed.
        assert!(scope_for_route("GET", "/healthz").is_none());
    }
}
