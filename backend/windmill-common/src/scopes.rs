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

use crate::error::{Error, Result};

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
            // Apps only: `write` can rewrite the app and its policy, so it also covers
            // running its components. Not general — `jobs:write` must not grant
            // `jobs:run`. The resource check below still confines it to the same app.
            ("write", "run") if self.domain == "apps" => {}
            ("write", "cancel") if self.domain == "jobs" => {}
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
            // A requirement naming no path is the whole domain, so only a grant that
            // itself spans every path satisfies it. `*` is that grant — the scope UI
            // accepts it as a resource path and `resources_match` already reads it as
            // everything — while any listed path leaves the collection unauthorized.
            (Some(self_resources), None) => self_resources.iter().any(|r| r == "*"),
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
    /// The `/data_metrics` catalog. Its own domain, NOT an alias of `Scripts`: a
    /// `data_metrics:read` token must reach only this route, never the broader
    /// `/scripts` routes (some of which do no further scope check).
    DataMetrics,
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
    AmqpTriggers,
    SqsTriggers,
    GcpTriggers,
    AzureTriggers,
    PostgresTriggers,
    EmailTriggers,

    // Native trigger domains
    NativeTriggers,
    TriggersHistory,

    // System domains
    Audit,
    Settings,
    Workers,
    ServiceLogs,
    Configs,
    OAuth,
    AI,
    AiEvals, // AI agent eval datasets

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
            Self::DataMetrics => "data_metrics",
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
            Self::AmqpTriggers => "amqp_triggers",
            Self::SqsTriggers => "sqs_triggers",
            Self::GcpTriggers => "gcp_triggers",
            Self::AzureTriggers => "azure_triggers",
            Self::PostgresTriggers => "postgres_triggers",
            Self::EmailTriggers => "email_triggers",
            Self::NativeTriggers => "native_triggers",
            Self::TriggersHistory => "triggers_history",
            Self::Audit => "audit",
            Self::Settings => "settings",
            Self::Workers => "workers",
            Self::ServiceLogs => "service_logs",
            Self::Configs => "configs",
            Self::OAuth => "oauth",
            Self::AI => "ai",
            Self::AiEvals => "ai_evals",
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
            // A distinct domain, not an alias of `scripts` (see the enum variant):
            // a `data_metrics:read` token must not reach the broader /scripts routes.
            "data_metrics" => Some(Self::DataMetrics),
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
            "amqp_triggers" => Some(Self::AmqpTriggers),
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
            "ai_evals" => Some(Self::AiEvals),
            "indexer" | "srch" => Some(Self::Indexer),
            "teams" => Some(Self::Teams),
            "native_triggers" => Some(Self::NativeTriggers),
            "triggers_history" => Some(Self::TriggersHistory),
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
    Read,   // GET operations, list, view
    Write,  // POST, PUT, PATCH, DELETE operations, create, update, delete
    Run,    // Special action for running (scripts, flows, etc.)
    Cancel, // Cancelling jobs (`CANCEL_PATH_ACTIONS`); covered by `jobs:write`
}

impl ScopeAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Run => "run",
            Self::Cancel => "cancel",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "delete" => Some(Self::Write),
            "run" => Some(Self::Run),
            "cancel" => Some(Self::Cancel),
            _ => None,
        }
    }

    /// Check if this action includes another action
    /// Write includes Read
    pub fn includes(&self, other: &ScopeAction) -> bool {
        match (self, other) {
            (ScopeAction::Write, ScopeAction::Read) => true,
            (ScopeAction::Run, ScopeAction::Read) => true,
            (ScopeAction::Write, ScopeAction::Cancel) => true,
            (a, b) => a == b,
        }
    }
}

/// Whether `caller` grants at least everything `requested` grants (directional
/// containment).
///
/// This is intentionally NOT `ScopeDefinition::includes`: that method answers
/// "does this scope grant access to a required action" using OR semantics over
/// resources (any overlap counts, and a `*` on either side matches), which is
/// correct for access checks but unsafe for subset checks — it would let a
/// token scoped to `scripts:read:f/team/a` mint `scripts:read:*` or
/// `scripts:read:f/team/a,f/other/b`. Subset containment instead requires that
/// EVERY requested resource is covered by SOME caller resource.
pub fn scope_contains(caller: &ScopeDefinition, requested: &ScopeDefinition) -> bool {
    if caller.domain != requested.domain {
        return false;
    }

    // write subsumes read; otherwise the action must match exactly.
    match (caller.action.as_str(), requested.action.as_str()) {
        (c, r) if c == r || (c == "write" && r == "read") => {}
        // Apps only: `write` covers `run` (see `ScopeDefinition::includes`), so an
        // app-editor token can mint the narrower run-only credential.
        ("write", "run") if caller.domain == "apps" => {}
        ("write", "cancel") if caller.domain == "jobs" => {}
        _ => return false,
    }

    if caller.domain == "jobs" && caller.action == "run" {
        match (&caller.kind, &requested.kind) {
            (Some(caller_kind), Some(requested_kind)) if caller_kind != requested_kind => {
                return false
            }
            // Caller pinned to a kind, but the request covers any kind.
            (Some(_), None) => return false,
            _ => {}
        }
    }

    match (&caller.resource, &requested.resource) {
        // Caller is unrestricted on resources: covers everything.
        (None, _) => true,
        // Caller is resource-restricted but the request is not: broader, unless the
        // caller lists `*` and so already spans every path. Kept in step with
        // `ScopeDefinition::includes`, which accepts that same grant for a
        // whole-collection read: what a token may exercise, it may also delegate.
        (Some(caller_resources), None) => caller_resources.iter().any(|r| r == "*"),
        (Some(caller_resources), Some(requested_resources)) => {
            resource_set_contains(caller_resources, requested_resources)
        }
    }
}

/// Every resource in `requested` must be covered by some resource in `caller`.
fn resource_set_contains(caller: &[String], requested: &[String]) -> bool {
    if caller.iter().any(|r| r == "*") {
        return true;
    }
    requested
        .iter()
        .all(|req| req != "*" && caller.iter().any(|c| resource_covers(c, req)))
}

/// Directional: does the single caller resource pattern cover `requested`?
/// `caller` may be an exact path or a `<prefix>/*` subtree wildcard; `requested`
/// may itself be a subtree wildcard, in which case the whole requested subtree
/// must fall within the caller's subtree.
fn resource_covers(caller: &str, requested: &str) -> bool {
    if caller == requested {
        return true;
    }
    let Some(prefix) = caller.strip_suffix("/*") else {
        // An exact caller resource only covers itself (handled above).
        return false;
    };
    let requested_base = requested.strip_suffix("/*").unwrap_or(requested);
    requested_base == prefix
        || (requested_base.starts_with(prefix)
            && requested_base.as_bytes().get(prefix.len()) == Some(&b'/'))
}

/// Stands in for an empty set of job token scopes. A token with `scopes: Some([])` is
/// unrestricted (an empty list is read as "no scopes defined"), so a job whose effective
/// scopes are empty carries this entry instead: it does not parse, and an unparseable
/// entry marks a token as scoped while granting nothing.
pub const NO_API_ACCESS_SCOPE: &str = "no_api_access";

/// Validates and normalizes a script or flow `job_token_scopes` setting. `[]` is valid and
/// leaves the job only the runtime routes about itself.
pub fn validate_job_token_scopes(scopes: &[String]) -> Result<Vec<String>> {
    let mut normalized: Vec<String> = Vec::with_capacity(scopes.len());
    for scope in scopes {
        let scope = scope.trim();
        // MCP scopes follow the MCP runtime's own grammar, and a list of only
        // `if_jobs:filter_tags` entries reads as unscoped: neither can cap a job token.
        if scope.starts_with("mcp:") || scope.starts_with("if_jobs:") {
            return Err(Error::BadRequest(format!(
                "Job token scopes cannot include '{scope}'"
            )));
        }
        let parsed = ScopeDefinition::from_scope_string(scope)?;
        if ScopeDomain::from_str(&parsed.domain).is_none()
            || ScopeAction::from_str(&parsed.action).is_none()
        {
            return Err(Error::BadRequest(format!(
                "Unknown job token scope '{scope}'"
            )));
        }
        if !normalized.iter().any(|s| s == scope) {
            normalized.push(scope.to_string());
        }
    }
    Ok(normalized)
}

/// The scopes a pushed job's token is restricted to: what `ceiling` (the scopes of the job
/// or token the push acts for) and `own` (the target's `job_token_scopes` setting) both
/// grant. `None` on both sides means unrestricted.
///
/// Keeps every entry of either side contained by some entry of the other. Overlaps that
/// neither side contains whole (two different globs) are dropped, which can only narrow.
pub fn intersect_job_token_scopes(
    ceiling: Option<&[String]>,
    own: Option<&[String]>,
) -> Option<Vec<String>> {
    match (ceiling, own) {
        (None, None) => None,
        (Some(c), None) => Some(c.to_vec()),
        (None, Some(o)) => Some(o.to_vec()),
        (Some(c), Some(o)) => {
            let parse = |scopes: &[String]| -> Vec<Option<ScopeDefinition>> {
                scopes
                    .iter()
                    .map(|s| ScopeDefinition::from_scope_string(s).ok())
                    .collect()
            };
            let (pc, po) = (parse(c), parse(o));
            let mut out: Vec<String> = vec![];
            let mut keep_covered =
                |raw: &[String], parsed: &[Option<ScopeDefinition>], by: &[Option<ScopeDefinition>]| {
                    for (s, p) in raw.iter().zip(parsed) {
                        let Some(p) = p else { continue };
                        if by.iter().flatten().any(|b| scope_contains(b, p))
                            && !out.iter().any(|x| x == s)
                        {
                            out.push(s.clone());
                        }
                    }
                };
            keep_covered(o, &po, &pc);
            keep_covered(c, &pc, &po);
            Some(out)
        }
    }
}

/// Validates the `job_token_scopes` of every step and agent tool of a flow, returning whether
/// any of them sets one.
pub fn validate_flow_step_job_token_scopes(value: &crate::flows::FlowValue) -> Result<bool> {
    let mut any = false;
    let mut check = |module: &crate::flows::FlowModule| -> anyhow::Result<()> {
        if let Some(scopes) = &module.job_token_scopes {
            any = true;
            validate_job_token_scopes(scopes)
                .map_err(|e| anyhow::anyhow!("step {}: {e}", module.id))?;
        }
        Ok(())
    };
    let extra: Vec<crate::flows::FlowModule> = value
        .failure_module
        .iter()
        .chain(value.preprocessor_module.iter())
        .map(|m| (**m).clone())
        .collect();
    crate::flows::FlowModule::traverse_modules(&value.modules, &mut check)
        .and_then(|()| crate::flows::FlowModule::traverse_modules(&extra, &mut check))
        .map_err(|e| Error::BadRequest(e.to_string()))?;
    Ok(any)
}

/// Counts a deploy of a script or flow that restricts its job token, keyed by which shape
/// of restriction it picked, so take-up of the presets can be told from custom lists.
pub fn log_job_token_scopes_deploy(kind: &'static str, scopes: Option<&[String]>) {
    let Some(scopes) = scopes else { return };
    let shape = match scopes {
        [] => "no_api",
        [only] if only == "oidc:write" => "oidc_only",
        _ => "custom",
    };
    crate::feature_usage::log_feature_usage(
        "job_token_scopes",
        "deploy",
        &format!("{kind}:{shape}"),
    );
}

/// The `scopes` claim of a job token minted from the job's effective scopes.
pub fn job_token_jwt_scopes(effective: Option<Vec<String>>) -> Option<Vec<String>> {
    match effective {
        Some(s) if s.is_empty() => Some(vec![NO_API_ACCESS_SCOPE.to_string()]),
        s => s,
    }
}

#[cfg(test)]
mod job_token_scopes_tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn intersect_never_widens() {
        assert_eq!(intersect_job_token_scopes(None, None), None);
        let oidc = v(&["oidc:write"]);
        assert_eq!(intersect_job_token_scopes(Some(&oidc), None), Some(oidc.clone()));
        assert_eq!(intersect_job_token_scopes(None, Some(&oidc)), Some(oidc.clone()));
        // A child setting asking for more than its parent holds gets only the common part.
        assert_eq!(
            intersect_job_token_scopes(
                Some(&oidc),
                Some(&v(&["oidc:write", "variables:read"]))
            ),
            Some(oidc.clone())
        );
        // Narrower entries win from either side.
        assert_eq!(
            intersect_job_token_scopes(
                Some(&v(&["variables:write:f/a/*"])),
                Some(&v(&["variables:read:f/a/b", "scripts:read"]))
            ),
            Some(v(&["variables:read:f/a/b"]))
        );
        assert_eq!(
            intersect_job_token_scopes(Some(&v(&["jobs:run"])), Some(&v(&["oidc:write"]))),
            Some(vec![])
        );
        // The empty-set marker grants nothing to intersect with.
        assert_eq!(
            intersect_job_token_scopes(Some(&v(&[NO_API_ACCESS_SCOPE])), Some(&oidc)),
            Some(vec![])
        );
    }

    #[test]
    fn empty_effective_scopes_mint_a_scoped_token() {
        assert_eq!(
            job_token_jwt_scopes(Some(vec![])),
            Some(v(&[NO_API_ACCESS_SCOPE]))
        );
        assert_eq!(job_token_jwt_scopes(None), None);
    }

    #[test]
    fn validate_rejects_unusable_scopes() {
        assert!(validate_job_token_scopes(&v(&["mcp:all"])).is_err());
        assert!(validate_job_token_scopes(&v(&["if_jobs:filter_tags:a"])).is_err());
        assert!(validate_job_token_scopes(&v(&["nope:write"])).is_err());
        assert!(validate_job_token_scopes(&v(&["*"])).is_err());
        assert_eq!(
            validate_job_token_scopes(&v(&["oidc:write", " oidc:write"])).unwrap(),
            v(&["oidc:write"])
        );
    }
}
