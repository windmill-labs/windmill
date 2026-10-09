/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_common::error::{Error, Result};

pub use windmill_common::scopes::{ScopeAction, ScopeDefinition, ScopeDomain};

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

    // A guest session carries the same broad read scopes as an embed token and for
    // the same handful of routes, so it gets the same default-deny.
    if has_guest_sentinel(Some(token_scopes)) {
        if let Some(suffix) = route_suffix.as_deref() {
            if guest_route_denied(required_domain, suffix) {
                return Err(Error::PermissionDenied(format!(
                    "a guest session cannot access {route_path}"
                )));
            }
            // Same rationale as the embed branch: re-running a component supersedes
            // its in-flight run, and `cancel_job_api` confines this to the caller's
            // own jobs.
            if suffix.starts_with("jobs_u/queue/cancel/") {
                return Ok(());
            }
        }
    }

    // Each declared scope must grant what its prompt said and no more:
    // `jobs:run` only deployed runnables, `users:read` only the viewer's identity.
    if has_raw_app_sdk_sentinel(Some(token_scopes)) {
        if let Some(suffix) = route_suffix.as_deref() {
            if is_request_supplied_code_route(suffix) {
                return Err(Error::PermissionDenied(
                    "Access denied. A raw app frontend SDK token cannot run request-supplied code."
                        .to_string(),
                ));
            }
            if required_domain == ScopeDomain::Users && suffix != "users/whoami" {
                return Err(Error::PermissionDenied(
                    "Access denied. A raw app frontend SDK token can only read the viewer's own identity."
                        .to_string(),
                ));
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

/// Runs of a saved AI agent: the server builds the run from the stored agent, so the route is
/// scoped by the agent's path like a deployed runnable's, as `jobs:run:agents:<path>`.
const AGENT_JOBS: [&'static str; 1] = ["jobs/run/agent"];

lazy_static::lazy_static! {
    static ref RUN_PATH_ACTIONS: Vec<&'static str> = {
        let mut v = vec!["jobs/resume/", "jobs/run/batch_rerun_jobs", "jobs/run/workflow_as_code", "jobs/run/dependencies","jobs/run/flow_dependencies", "apps_u/execute_component", "apps_u/upload_s3_file"];

        v.extend(SCRIPT_JOBS);
        v.extend(FLOW_JOBS);
        v.extend(AGENT_JOBS);
        v
    };
}

/// The job routes a `jobs:cancel` scope reaches, as workspaced route suffixes. A
/// path-scoped `jobs:cancel:<paths>` is resource-blind here like every scope; the
/// handlers confine it through `job_cancel_path_confinement`, so a cancel route added
/// here without that check would serve a path-scoped token every job.
const CANCEL_PATH_ACTIONS: [&'static str; 4] = [
    "jobs_u/queue/cancel/",
    "jobs_u/queue/force_cancel/",
    "jobs_u/queue/cancel_persistent/",
    "jobs/queue/cancel_selection",
];

fn is_cancel_route(method: &str, route_path: &str) -> bool {
    if !method.eq_ignore_ascii_case("POST") {
        return false;
    }
    let mut parts = route_path.splitn(5, '/');
    let (Some(""), Some("api"), Some("w"), Some(_), Some(suffix)) = (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) else {
        return false;
    };
    CANCEL_PATH_ACTIONS.iter().any(|p| suffix.starts_with(p))
}

fn map_http_method_to_action(method: &str, route_path: &str) -> ScopeAction {
    if is_cancel_route(method, route_path) {
        return ScopeAction::Cancel;
    }
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

/// Checks the route path to determine the runnable kind ("agents", "flows" or "scripts").
///
/// The order of checks is important:
/// - Flow-related paths are checked first to avoid false positives, as some flow paths
///   (e.g., `/run_preview_flow`) share prefixes with script paths (e.g., `/run_preview`).
///
/// Returns the kind based on the match, or `None` if no match is found.
fn determine_kind_from_route(route_path: &str) -> Option<String> {
    if route_path.starts_with("jobs") {
        // Preview/bundle runs execute arbitrary code with no deployed path, so
        // their handlers require the broad `jobs:run` scope: they must carry no
        // kind, else the derived scope is narrower than the handler demands.
        // Anchor to the endpoint segment so by-path runs of a deployed runnable
        // whose path contains "preview" (e.g. `run/p/f/team/preview_report`) are
        // still classified by their kind.
        if route_path.starts_with("jobs/run/preview")
            || route_path.starts_with("jobs/run_wait_result/preview")
        {
            return None;
        }
        if AGENT_JOBS.iter().any(|path| route_path.starts_with(path)) {
            return Some("agents".to_string());
        }
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

/// The reads a `jobs:run` scope implies: following, by id, a run the token started.
/// Every entry is keyed by a job id and confines an authenticated caller to its own
/// runnable — through `require_job_read_access`, through its own `jobs:run:flows:<path>`
/// check, or, where an approval token or resume secret bypasses that gate, through a
/// direct `require_job_within_read_scope`. The one exception is
/// `jobs_u/get_root_job_id/`, which has no check at all but discloses only flow lineage,
/// to anyone, authenticated or not. Workspace-wide enumeration (`jobs/list`, counts,
/// exports) and credential minting (`job_view_token`) are deliberately absent — those are
/// `jobs:read`. Keep by-id read routes here in sync as they are added, or a run token
/// loses the ability to follow its own run through them.
const RUN_WHITELISTED_GET_PATHS: [&'static str; 32] = [
    "jobs_u/get_flow/",
    "jobs_u/get_root_job_id/",
    "jobs_u/get/",
    "jobs_u/get_logs/",
    "jobs_u/get_completed_logs_tail/",
    "jobs_u/get_flow_all_logs/",
    "jobs_u/get_flow_all_logs_structured/",
    "jobs_u/get_flow_all_results/",
    "jobs_u/get_args/",
    "jobs_u/get_flow_debug_info/",
    "jobs_u/completed/get/",
    "jobs_u/completed/get_result/",
    "jobs_u/completed/get_result_maybe/",
    "jobs_u/completed/get_timing/",
    "jobs_u/dispatch_events/",
    "jobs_u/getupdate/",
    "jobs_u/getupdate_sse/",
    "jobs_u/get_log_file/",
    "jobs/run_progress/",
    "jobs/dbt_graph/",
    "jobs/dbt_resumable/",
    "jobs/dbt_resumable_script/p/",
    "jobs/result_by_id/",
    "jobs/resume_urls/",
    "jobs/flow/user_states/",
    "jobs/job_signature/",
    "jobs/wac_approval_urls/",
    "jobs/completed/get/",
    "jobs/completed/get_result/",
    "jobs/completed/get_result_maybe/",
    "jobs/completed/get_timing/",
    "jobs/get_otel_traces/",
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

/// Sentinel in a guest session token: someone the identity provider authenticated
/// who is a member of no workspace. Grants nothing itself — it only confines the
/// session to the app surface, the same way `app_embed` does. What makes a session a
/// guest at all is the server-minted label
/// [`windmill_common::auth::GUEST_SESSION_LABEL`]; a forged sentinel here can only
/// narrow its own token.
pub const GUEST_SENTINEL: &str = "guest";

/// True if a token is a guest session, whose scopes are its entire grant: it has no ACL
/// of its own, so every ACL check denies it unaided.
pub fn has_guest_sentinel(scopes: Option<&[String]>) -> bool {
    scopes.is_some_and(|s| s.iter().any(|x| x == GUEST_SENTINEL))
}

/// `scopes` with the guest sentinel present exactly once.
pub fn with_guest_sentinel(mut scopes: Vec<String>) -> Vec<String> {
    if !scopes.iter().any(|x| x == GUEST_SENTINEL) {
        scopes.push(GUEST_SENTINEL.to_string());
    }
    scopes
}

/// Scopes a guest session carries. The broad-looking reads are narrowed to a route
/// allowlist by the sentinel (`guest_route_denied`), plus the two path-scoped app
/// grants. A guest has no `usr` row, so this list is the whole of what it can do. The
/// single source both the mint (a signed-in guest) and the JWT auth arm build from.
///
/// The sentinel here only narrows. A signed-in guest is made one by the server-minted
/// label; a JWT guest has no label, so for it the sentinel is what governs.
pub fn guest_session_scopes(app_path: &str) -> windmill_common::error::Result<Vec<String>> {
    // The path is spliced into a scope, whose grammar reserves `:`, `,`, `*` and a leading
    // `/`; app paths may otherwise carry spaces and `@`, so guard only those reserved chars.
    if !windmill_common::auth::is_scope_literal_path(app_path) {
        return Err(windmill_common::error::Error::BadRequest(format!(
            "app path {app_path} is empty or cannot be scoped: `:`, `,` and `*` are reserved \
             in scopes, and a leading `/` never matches a route"
        )));
    }
    Ok(vec![
        GUEST_SENTINEL.to_string(),
        "jobs:read".to_string(),
        "resources:run".to_string(),
        "users:read".to_string(),
        "folders:read".to_string(),
        format!("apps:read:{app_path}"),
        format!("apps:run:{app_path}"),
    ])
}

/// Sentinel in raw-app SDK tokens. Grants nothing; `check_route_access` uses it
/// to narrow the declared scopes to what the viewer's prompt promised.
pub const RAW_APP_SDK_SENTINEL: &str = "raw_app_sdk";

pub fn has_raw_app_sdk_sentinel(scopes: Option<&[String]>) -> bool {
    scopes.is_some_and(|s| s.iter().any(|x| x == RAW_APP_SDK_SENTINEL))
}

/// Endpoints that run code the caller supplies or names by job id (the latter
/// with no ownership check). Their jobs get an unscoped credential as the viewer,
/// so reaching one would make a captured SDK token a full account takeover.
fn is_request_supplied_code_route(suffix: &str) -> bool {
    // Prefixes, so the `_async` variants are covered too.
    const CODE_ROUTES: [&str; 10] = [
        "jobs/run/preview",
        "jobs/run_inline/preview",
        "jobs/run_wait_result/preview",
        "jobs/run/preview_bundle",
        "jobs/run/preview_flow",
        "jobs/run_wait_result/preview_flow",
        "jobs/run/dependencies",
        "jobs/run/flow_dependencies",
        "jobs/run/workflow_as_code",
        "jobs/restart/f",
    ];
    CODE_ROUTES.iter().any(|p| suffix.starts_with(p))
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

/// Routes a guest session is denied: the app-embed allowlist, plus the embed-token
/// mint. A guest session is the *embedder* — the viewer's own browser rendering the
/// app page — not the app's own JS, and the page mints the iframe's token from it.
///
/// Everything else stays default-denied, so a guest reaches the app it was let in
/// for and nothing around it.
fn guest_route_denied(domain: ScopeDomain, suffix: &str) -> bool {
    if domain == ScopeDomain::Apps && suffix.starts_with("apps_u/embed_token") {
        return false;
    }
    app_embed_route_denied(domain, suffix)
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

/// The by-id and list routes a path-scoped `jobs:read:<paths>` reaches, as workspaced
/// route suffixes. Every other read in the domain is refused to it: counts, exports,
/// uuid listings, queue positions and dispatch edges aggregate over the workspace, and
/// the view-token and resume-url routes mint a credential (`approval_info` withholds
/// its view token from a confined caller for the same reason).
///
/// Each entry confines the caller in its handler, through `require_job_read_access` or
/// a direct `require_job_within_read_scope`. `jobs/result_by_id/` is absent because it
/// resolves a node up through the enclosing flows, past the job the check admitted.
/// A route added here without that check serves a path-scoped token every job.
const READ_PATH_SCOPED_BY_ID_PATHS: [&'static str; 25] = [
    "jobs_u/get/",
    "jobs_u/get_logs/",
    "jobs_u/get_completed_logs_tail/",
    "jobs_u/get_flow_all_logs/",
    "jobs_u/get_flow_all_logs_structured/",
    "jobs_u/get_flow_all_results/",
    "jobs_u/get_args/",
    "jobs_u/get_flow_debug_info/",
    "jobs_u/completed/get/",
    "jobs_u/completed/get_result/",
    "jobs_u/completed/get_result_maybe/",
    "jobs_u/completed/get_timing/",
    "jobs_u/dispatch_events/",
    "jobs_u/getupdate/",
    "jobs_u/getupdate_sse/",
    "jobs_u/get_log_file/",
    "jobs_u/get_flow/",
    "jobs_u/flow/approval_info/",
    "jobs/run_progress/",
    "jobs/flow/user_states/",
    "jobs/completed/get/",
    "jobs/completed/get_result/",
    "jobs/completed/get_result_maybe/",
    "jobs/completed/get_timing/",
    "jobs/get_otel_traces/",
];

/// Matched whole: `jobs/list` is also a prefix of the uuid listings. Their handlers
/// filter in SQL to [`job_list_path_filter`].
const READ_PATH_SCOPED_LIST_PATHS: [&'static str; 3] =
    ["jobs/list", "jobs/completed/list", "jobs/queue/list"];

fn is_read_path_scoped_route(suffix: &str) -> bool {
    READ_PATH_SCOPED_LIST_PATHS.contains(&suffix)
        || READ_PATH_SCOPED_BY_ID_PATHS
            .iter()
            .any(|p| suffix.starts_with(p))
}

/// The paths of a `jobs:read:<paths>` scope, or `None` when it spans every job: no
/// path part, or `*` among them.
fn job_read_scope_paths(scope: &ScopeDefinition) -> Option<&[String]> {
    scope
        .resource
        .as_deref()
        .filter(|paths| !paths.iter().any(|p| p == "*"))
}

/// The runnables a token's job reads are confined to: those its `jobs:run` scopes may
/// start, and those its `jobs:read:<paths>` scopes name.
pub struct JobReadConfinement {
    run: Vec<ScopeDefinition>,
    read_paths: crate::ScopePathFilter,
}

impl JobReadConfinement {
    /// Whether a job that ran `runnable_path` as `kind` (`scripts`, `flows` or `agents`)
    /// is inside the confinement. A read path admits every kind; a run scope only its own.
    pub fn admits(&self, kind: &str, runnable_path: &str) -> bool {
        if self.read_paths.allows(runnable_path) {
            return true;
        }
        let required = ScopeDefinition::new(
            ScopeDomain::Jobs.as_str(),
            ScopeAction::Run.as_str(),
            Some(kind),
            Some(vec![runnable_path.to_string()]),
        );
        self.run.iter().any(|scope| scope.includes(&required))
    }

    /// Whether a `jobs:read:<paths>` scope is part of the confinement.
    pub fn has_read_paths(&self) -> bool {
        matches!(&self.read_paths, crate::ScopePathFilter::Restricted { exact, prefix }
            if !exact.is_empty() || !prefix.is_empty())
    }
}

fn push_read_paths(paths: &[String], exact: &mut Vec<String>, prefix: &mut Vec<String>) {
    for path in paths {
        match path.strip_suffix("/*") {
            Some(p) => prefix.push(p.to_string()),
            None => exact.push(path.clone()),
        }
    }
}

/// The paths a token's job lists are filtered to. Decided by its read grants alone,
/// unlike [`job_read_confinement`]: a `jobs:run` scope grants no enumeration, so even a
/// bare one beside a `jobs:read:<paths>` leaves the lists on those paths. Unrestricted
/// for an unscoped token, a `jobs:read` naming no path (or `*`), and `jobs:write`.
pub fn job_list_path_filter(scopes: Option<&[String]>) -> crate::ScopePathFilter {
    let mut exact = Vec::new();
    let mut prefix = Vec::new();
    let mut scoped = false;
    for scope in scopes
        .unwrap_or_default()
        .iter()
        .filter(|s| !s.starts_with("if_jobs:filter_tags:"))
    {
        scoped = true;
        let Ok(scope) = ScopeDefinition::from_scope_string(scope) else {
            continue;
        };
        if ScopeDomain::from_str(&scope.domain) != Some(ScopeDomain::Jobs) {
            continue;
        }
        match ScopeAction::from_str(&scope.action) {
            Some(ScopeAction::Read) => match job_read_scope_paths(&scope) {
                Some(paths) => push_read_paths(paths, &mut exact, &mut prefix),
                None => return crate::ScopePathFilter::AllowAll,
            },
            Some(ScopeAction::Write) => return crate::ScopePathFilter::AllowAll,
            _ => continue,
        }
    }
    if scoped {
        crate::ScopePathFilter::Restricted { exact, prefix }
    } else {
        crate::ScopePathFilter::AllowAll
    }
}

/// What a token's job reads are confined to, or `None` when they are not confined to
/// particular runnables.
///
/// A run scope is what the trigger UI mints per script or flow and hands to a webhook
/// caller / CI job: it may start the runnables it names and follow those runs, so its
/// by-id job reads must stay within what it can start. Both the path
/// (`jobs:run:flows:f/team/etl`) and the kind-only (`jobs:run:scripts`, which legacy
/// `jobs:runscript` tokens carry) forms confine, since `ScopeDefinition::includes`
/// already matches a candidate `jobs:run:<kind>:<path>` against either.
///
/// A `jobs:read:<paths>` scope reads the runs of the runnables at those paths and
/// nothing else. The two kinds of grant add up: a job either admits is readable.
///
/// Returns `None` — unconfined — when the token is effectively unscoped, or carries a
/// jobs scope that grants every job read in its own right: a `jobs:read` naming no path
/// (or `*`), a bare `jobs:run` (it can start anything, so confining its reads to "what
/// it may run" would restrict nothing), or `jobs:write`, whose paths are ignored on
/// reads as they are on every job write.
pub fn job_read_confinement(scopes: Option<&[String]>) -> Option<JobReadConfinement> {
    let mut run = Vec::new();
    let mut exact = Vec::new();
    let mut prefix = Vec::new();
    let mut confined = false;
    for scope in scopes?
        .iter()
        .filter(|s| !s.starts_with("if_jobs:filter_tags:"))
    {
        let Ok(scope) = ScopeDefinition::from_scope_string(scope) else {
            continue;
        };
        if ScopeDomain::from_str(&scope.domain) != Some(ScopeDomain::Jobs) {
            continue;
        }
        match ScopeAction::from_str(&scope.action) {
            Some(ScopeAction::Run) if scope.kind.is_some() || scope.resource.is_some() => {
                confined = true;
                run.push(scope)
            }
            Some(ScopeAction::Read) => {
                let Some(paths) = job_read_scope_paths(&scope) else {
                    return None;
                };
                confined = true;
                push_read_paths(paths, &mut exact, &mut prefix);
            }
            // Grants no reads (see `scope_grants_access`), so it neither confines nor
            // frees them.
            Some(ScopeAction::Cancel) => continue,
            Some(_) => return None,
            None => continue,
        }
    }
    confined.then_some(JobReadConfinement {
        run,
        read_paths: crate::ScopePathFilter::Restricted { exact, prefix },
    })
}

/// The `jobs:cancel:<paths>` scopes a token's cancels are confined to, or `None` when
/// they are not confined: an unscoped token, or one whose cancel grant is not
/// path-scoped (`jobs:cancel`, or `jobs:write`, which stays resource-blind here as on
/// every other job write).
///
/// A job is within the confinement when its own `runnable_path` or that of any of its
/// `parent_job` ancestors matches (see `cancel_confinement_admits`), so a token scoped
/// to a flow can cancel the flow's steps.
pub fn job_cancel_path_confinement(scopes: Option<&[String]>) -> Option<Vec<ScopeDefinition>> {
    let mut confinement = Vec::new();
    for scope in scopes?
        .iter()
        .filter(|s| !s.starts_with("if_jobs:filter_tags:"))
    {
        let Ok(scope) = ScopeDefinition::from_scope_string(scope) else {
            continue;
        };
        if ScopeDomain::from_str(&scope.domain) != Some(ScopeDomain::Jobs) {
            continue;
        }
        match ScopeAction::from_str(&scope.action) {
            Some(ScopeAction::Cancel) if scope.resource.is_some() => confinement.push(scope),
            Some(ScopeAction::Cancel | ScopeAction::Write) => return None,
            _ => continue,
        }
    }
    (!confinement.is_empty()).then_some(confinement)
}

/// Whether the token holds a `jobs:cancel` scope, path-scoped or not.
pub fn has_job_cancel_grant(scopes: Option<&[String]>) -> bool {
    scopes.is_some_and(|scopes| {
        scopes.iter().any(|s| {
            ScopeDefinition::from_scope_string(s).is_ok_and(|s| {
                ScopeDomain::from_str(&s.domain) == Some(ScopeDomain::Jobs)
                    && ScopeAction::from_str(&s.action) == Some(ScopeAction::Cancel)
            })
        })
    })
}

/// Whether a job of `runnable_path` is inside a [`job_cancel_path_confinement`] set.
pub fn cancel_confinement_admits(confinement: &[ScopeDefinition], runnable_path: &str) -> bool {
    let required = ScopeDefinition::new(
        ScopeDomain::Jobs.as_str(),
        ScopeAction::Cancel.as_str(),
        None,
        Some(vec![runnable_path.to_string()]),
    );
    confinement.iter().any(|scope| scope.includes(&required))
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

    // Apps `write` covers `run` (see `ScopeDefinition::includes`). Like every domain
    // here this layer is resource-blind; the Run handlers path-check the app.
    if scope_domain == ScopeDomain::Apps
        && scope_action == ScopeAction::Write
        && required_action == ScopeAction::Run
    {
        return Ok(true);
    }

    // `jobs:run` is a grant to *start* a runnable. The only reads it implies are the
    // by-id routes a caller needs to follow the run it started
    // (`RUN_WHITELISTED_GET_PATHS`) — never workspace-wide enumeration (`jobs/list`,
    // counts, exports), which is what `jobs:read` is for. Those by-id reads are in turn
    // confined to the runnable a path-scoped token names, by `require_job_read_access`.
    // `ScopeAction::Run.includes(&Read)` (which exists so `apps:run` can fetch the app
    // it runs) must not reach this domain, so decide it here rather than falling
    // through to the hierarchy below.
    if scope_domain == ScopeDomain::Jobs
        && scope_action == ScopeAction::Run
        && required_action == ScopeAction::Read
    {
        return Ok(route_path.is_some_and(|p| {
            RUN_WHITELISTED_GET_PATHS
                .iter()
                .any(|path| p.starts_with(path))
        }));
    }

    // A path-scoped `jobs:read:<paths>` is the one job scope this layer does not treat
    // as resource-blind: it reaches only the routes whose handlers confine it to those
    // paths, so a job read route is refused to it until it is listed there.
    if scope_domain == ScopeDomain::Jobs
        && scope_action == ScopeAction::Read
        && required_action == ScopeAction::Read
        && job_read_scope_paths(scope).is_some()
    {
        return Ok(route_path.is_some_and(is_read_path_scoped_route));
    }

    if !scope_action.includes(&required_action) {
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

/// The workspace-less routes a job token (`$WM_TOKEN`) may still read. A route qualifies
/// only when it answers from the caller's own account, from the request body, or with
/// content identical for every workspace (the Hub proxy, the documentation) — never
/// naming another workspace, and never disclosing instance configuration. `usage` reads
/// the caller's own row; `email` and `allowed_domain_auto_invite` are derived from the
/// token itself and touch no table.
///
/// Three instance settings are on the list. `get_global_setting` exempts a handful of keys
/// from its own super-admin gate, these among them, so each is already readable by every
/// authenticated user; each is here because the CLI reads it from a job:
/// `automate_username_creation` before creating a user during a git-sync push, `uid` and
/// `hub_base_url` when a job runs `wmill hub pull`, as an edited copy of the retired
/// `u/admin/hub_sync` still does (servers sync resource types themselves). The other
/// ungated keys have no such caller, so they stay confined — being ungated earns a key
/// nothing on its own. Listing a gated key earns it nothing either: `require_super_admin`
/// refuses every job token, so `hub_api_secret`, which that pull reads for a private Hub,
/// stays out of a job's reach whatever this list says.
///
/// Deliberately absent, as each crosses that line: `users/list_invites` (returns the
/// workspace ids the identity was invited to), `users/tokens/list` (credential metadata
/// of the borrowed identity), `users/exists/{email}` (an oracle over arbitrary
/// addresses, not the caller's own), and `workspaces/list` / `workspaces/users`.
///
/// Read methods only — a mutating handler added on one of these paths must be
/// reconsidered rather than inherit the grant.
fn is_global_read_open_to_job_token(route_path: &str) -> bool {
    matches!(
        route_path,
        "/api/users/whoami"
            | "/api/users/email"
            | "/api/users/usage"
            | "/api/users/tutorial_progress"
            | "/api/workspaces/allowed_domain_auto_invite"
            | "/api/settings/global/automate_username_creation"
            | "/api/settings/global/uid"
            | "/api/settings/global/hub_base_url"
            | "/api/docs/search"
            | "/api/docs/page"
            | "/api/embeddings/query_hub_scripts"
    ) || route_path.starts_with("/api/integrations/hub/")
        || route_path.starts_with("/api/scripts/hub/")
        || route_path.starts_with("/api/flows/hub/")
        || route_path.starts_with("/api/apps/hub/")
}

/// The workspace-less POSTs a job token keeps. Each takes a `POST` for the sake of a
/// request body rather than to commit anything of consequence: none writes Windmill state
/// outside the caller's own account. The object-storage probe does write to the store the
/// body names — see its entry. What each may *read* is bounded per entry below — the
/// workspace-existence check answers for any id, the rest only from the body or the
/// caller's own row:
/// - a resource editor's object-storage "Test connection" runs as a preview job that POSTs
///   the storage config (`TestConnection.svelte`). It puts and deletes an object to prove
///   the credentials work, so it does write — but only to the store the body names, and a
///   failure between the two can leave that object behind. No Windmill state of any
///   workspace is touched.
/// - `wmill workspace add`, how a job points the CLI at its own instance, checks the
///   workspace exists before accepting the credentials — the git-sync hub scripts run
///   exactly this. `workspace` carries no row-level security, so the bare boolean it
///   answers is instance-wide rather than membership-filtered; what it discloses is
///   only whether a workspace id is taken.
/// - the cron preview computes the next occurrences of the expression in the body. It
///   takes no `ApiAuthed` and opens no transaction, so it returns nothing the caller did
///   not send.
/// - tutorial progress upserts a UI bitfield keyed on the caller's own email. Its path
///   serves a `GET` too, which the read list above carries.
const GLOBAL_WRITES_OPEN_TO_JOB_TOKEN: [&str; 4] = [
    "/api/settings/test_object_storage_config",
    "/api/workspaces/exists",
    "/api/schedules/preview",
    "/api/users/tutorial_progress",
];

/// Confines a job token (`$WM_TOKEN`) to routes that name a workspace. It is minted
/// for one job in one workspace yet carries that job's full user privileges, so on an
/// instance-wide route it would mint a permanent workspace-less API token, read
/// worker-group configuration, or manage global users. The token lookup already
/// rejects a workspace-bound API token on those routes; this is the same rule for
/// job tokens.
///
/// Keyed on the job token specifically: an MCP token is workspace-bound too, but
/// deliberately publishes instance-wide tools. Callers pass only routes whose path
/// carries no workspace.
pub fn check_job_token_for_global_route(route_path: &str, http_method: &str) -> Result<()> {
    let is_read = map_http_method_to_action(http_method, route_path) == ScopeAction::Read;
    if (is_read && is_global_read_open_to_job_token(route_path))
        || (http_method.eq_ignore_ascii_case("POST")
            && GLOBAL_WRITES_OPEN_TO_JOB_TOKEN.contains(&route_path))
    {
        Ok(())
    } else {
        Err(Error::PermissionDenied(format!(
            "A job token ($WM_TOKEN) is confined to the workspace of its job and cannot be used \
             on {route_path}, which is not workspace-scoped. Use an API token created for the \
             user instead."
        )))
    }
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

/// Routes the runtime of a job calls about that job alone: progress, its root id, its
/// resume and approval urls, a workflow-as-code checkpoint or task. A job token restricted
/// by `job_token_scopes` keeps these whatever its scopes, or the job could not run at all,
/// but only for its own job: the job id in the path must be the token's.
pub fn is_own_job_runtime_route(route_path: &str, http_method: &str, job_id: uuid::Uuid) -> bool {
    let Some(rest) = route_path.strip_prefix("/api/w/") else {
        return false;
    };
    let Some((_workspace, rest)) = rest.split_once('/') else {
        return false;
    };
    let segments: Vec<&str> = rest.split('/').collect();
    let (method_ok, id_index) = match segments.as_slice() {
        ["jobs_u", "get" | "get_root_job_id", ..] => (http_method == "GET", 2),
        ["job_metrics", "set_progress", ..] => (http_method == "POST", 2),
        ["job_metrics", "get_progress", ..] => (http_method == "GET", 2),
        ["jobs" | "jobs_u", "resume_urls" | "wac_approval_urls", ..] => (http_method == "GET", 2),
        ["jobs", "worker_is_draining", ..] => (http_method == "GET", 2),
        ["jobs", "wac", "inline_checkpoint", ..] => (http_method == "POST", 3),
        ["jobs", "run", "workflow_as_code", ..] => (http_method == "POST", 3),
        _ => return false,
    };
    method_ok
        && segments
            .get(id_index)
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .is_some_and(|id| id == job_id)
}

/// Routes reading the state of a flow run: step results (`results.x` in input transforms,
/// loop and branch results) and the run's user state. The flow orchestrator evaluates a
/// step's inputs with the token of the step that just finished, and the SDK reads user state
/// at the root job, so a restricted job token keeps these for every job of its own run.
/// Returns the job id the path names; the caller checks it against the token's lineage.
pub fn flow_run_read_route_job(route_path: &str, http_method: &str) -> Option<uuid::Uuid> {
    let rest = route_path.strip_prefix("/api/w/")?;
    let (_workspace, rest) = rest.split_once('/')?;
    let segments: Vec<&str> = rest.split('/').collect();
    let id = match segments.as_slice() {
        ["jobs" | "jobs_u", "result_by_id", id, ..] if http_method == "GET" => id,
        ["jobs_u", "completed", "get_result" | "get_result_maybe", id, ..]
            if http_method == "GET" =>
        {
            id
        }
        ["jobs" | "jobs_u", "flow", "user_states", id, ..]
            if http_method == "GET" || http_method == "POST" =>
        {
            id
        }
        _ => return None,
    };
    uuid::Uuid::parse_str(id).ok()
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
    fn own_job_runtime_routes_admit_only_the_tokens_job() {
        let own = uuid::Uuid::new_v4();
        let other = uuid::Uuid::new_v4();
        let ok = |path: &str, method: &str| is_own_job_runtime_route(path, method, own);
        assert!(ok(
            &format!("/api/w/ws/job_metrics/set_progress/{own}"),
            "POST"
        ));
        assert!(ok(
            &format!("/api/w/ws/jobs_u/get_root_job_id/{own}"),
            "GET"
        ));
        assert!(ok(&format!("/api/w/ws/jobs/resume_urls/{own}/0"), "GET"));
        assert!(ok(
            &format!("/api/w/ws/jobs/wac/inline_checkpoint/{own}"),
            "POST"
        ));
        assert!(ok(
            &format!("/api/w/ws/jobs/run/workflow_as_code/{own}/main"),
            "POST"
        ));
        assert!(!ok(
            &format!("/api/w/ws/job_metrics/set_progress/{other}"),
            "POST"
        ));
        assert!(!ok(&format!("/api/w/ws/jobs_u/get/{own}"), "POST"));
        assert!(!ok(&format!("/api/w/ws/jobs/run/p/{own}"), "POST"));
        assert!(!ok("/api/w/ws/variables/get_value/u/admin/secret", "GET"));
        let run = |path: &str, method: &str| flow_run_read_route_job(path, method);
        assert_eq!(
            run(&format!("/api/w/ws/jobs/result_by_id/{other}/b"), "GET"),
            Some(other)
        );
        assert_eq!(
            run(
                &format!("/api/w/ws/jobs_u/completed/get_result/{other}"),
                "GET"
            ),
            Some(other)
        );
        assert_eq!(
            run(
                &format!("/api/w/ws/jobs/flow/user_states/{other}/k"),
                "POST"
            ),
            Some(other)
        );
        assert_eq!(
            run(
                &format!("/api/w/ws/jobs_u/completed/delete/{other}"),
                "POST"
            ),
            None
        );
    }

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
    fn data_metrics_is_its_own_domain_not_a_scripts_alias() {
        // `data_metrics` must be a distinct domain: a token scoped to it must reach
        // only the data_metrics route, never the broader /scripts routes (some of
        // which do no further scope check). Regression for a privilege escalation.
        assert_eq!(
            ScopeDomain::from_str("data_metrics"),
            Some(ScopeDomain::DataMetrics)
        );
        let dm = vec!["data_metrics:read".to_string()];
        assert!(check_route_access(&dm, "/api/w/test/data_metrics/list", "GET").is_ok());
        assert!(check_route_access(&dm, "/api/w/test/scripts/list", "GET").is_err());
        assert!(check_route_access(&dm, "/api/w/test/scripts/raw/h/abc.ts", "GET").is_err());
        // Conversely a scripts token does not reach the data_metrics route.
        let sc = vec!["scripts:read".to_string()];
        assert!(check_route_access(&sc, "/api/w/test/data_metrics/list", "GET").is_err());
    }

    /// `apps_u/execute_component` (and the S3 upload the same components drive) is a
    /// Run action, so a scoped token needs `apps:run`. `apps:write` must keep reaching
    /// it too: it can rewrite the app and its policy, so withholding execution from it
    /// protects nothing while breaking every app-scoped token.
    #[test]
    fn apps_run_routes_accept_run_and_write_scopes() {
        let execute = "/api/w/test/apps_u/execute_component/u/admin/app";
        for scope in ["apps:run", "apps:write"] {
            assert!(
                check_route_access(&[scope.to_string()], execute, "POST").is_ok(),
                "{scope} must reach execute_component"
            );
        }
        assert!(check_route_access(&["apps:read".to_string()], execute, "POST").is_err());
        // The write-satisfies-run allowance is confined to the apps domain.
        assert!(check_route_access(
            &["jobs:write".to_string()],
            "/api/w/test/jobs/run/p/u/admin/script",
            "POST"
        )
        .is_err());
    }

    #[test]
    fn jobs_run_reads_are_limited_to_the_by_id_poll_routes() {
        let job = "/api/w/test/jobs_u/completed/get_result/019ff012-6b1e-0d6b-fc0d-0c85d34d9cec";
        let list = "/api/w/test/jobs/list";
        for scope in ["jobs:run", "jobs:run:scripts:u/admin/script"] {
            // Following the run it started stays available...
            assert!(
                check_route_access(&[scope.to_string()], job, "GET").is_ok(),
                "{scope} must reach the by-id job poll routes"
            );
            // ...but a run grant is not a licence to enumerate the workspace's jobs.
            assert!(
                check_route_access(&[scope.to_string()], list, "GET").is_err(),
                "{scope} must not reach jobs/list"
            );
        }
        assert!(check_route_access(&["jobs:read".to_string()], list, "GET").is_ok());
    }

    #[test]
    fn run_scopes_confine_job_reads_by_kind_and_path() {
        let confinement =
            job_read_confinement(Some(&["jobs:run:flows:f/team/*".to_string()])).unwrap();
        assert!(confinement.admits("flows", "f/team/etl"));
        // Right path, wrong kind — a script named like the flow is not the flow.
        assert!(!confinement.admits("scripts", "f/team/etl"));
        assert!(!confinement.admits("flows", "f/other/etl"));

        // A kind-only scope confines to that kind, at any path.
        let kind_only = job_read_confinement(Some(&["jobs:run:scripts".to_string()])).unwrap();
        assert!(kind_only.admits("scripts", "u/admin/anything"));
        assert!(!kind_only.admits("flows", "f/team/etl"));

        // Scopes that grant job reads in their own right leave reads unconfined.
        for scopes in [
            vec!["jobs:read".to_string()],
            vec!["jobs:run".to_string()],
            vec![
                "jobs:run:scripts:u/admin/script".to_string(),
                "jobs:read".to_string(),
            ],
            vec!["if_jobs:filter_tags:deno".to_string()],
        ] {
            assert!(
                job_read_confinement(Some(&scopes)).is_none(),
                "{scopes:?} must not confine job reads"
            );
        }
        assert!(job_read_confinement(None).is_none());
    }

    #[test]
    fn jobs_read_paths_confine_reads_and_reach_only_confined_routes() {
        let scopes = |s: &[&str]| s.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let scoped = scopes(&["jobs:read:f/served/*,u/svc/report"]);
        let confinement = job_read_confinement(Some(&scoped)).unwrap();
        // Any kind of runnable at a named path.
        assert!(confinement.admits("flows", "f/served/etl"));
        assert!(confinement.admits("scripts", "u/svc/report"));
        assert!(!confinement.admits("scripts", "f/other/etl"));
        assert!(!confinement.admits("scripts", "f/served_not/etl"));
        let lists = |s: &[&str], path: &str| job_list_path_filter(Some(&scopes(s))).allows(path);
        assert!(lists(
            &["jobs:read:f/served/*,u/svc/report"],
            "f/served/etl"
        ));
        assert!(!lists(
            &["jobs:read:f/served/*,u/svc/report"],
            "f/other/etl"
        ));

        // A run scope adds the runnables it may start to the by-id reads, and nothing to
        // the lists, not even a bare one that leaves the by-id reads unconfined.
        let both = scopes(&["jobs:read:f/served/*", "jobs:run:scripts:f/other/etl"]);
        let confinement = job_read_confinement(Some(&both)).unwrap();
        assert!(confinement.admits("scripts", "f/other/etl"));
        assert!(!confinement.admits("flows", "f/other/etl"));
        for run in ["jobs:run:scripts:f/other/etl", "jobs:run"] {
            assert!(
                !lists(&["jobs:read:f/served/*", run], "f/other/etl"),
                "{run}"
            );
        }
        for unfiltered in [
            &["jobs:read"][..],
            &["jobs:read:f/served/*", "jobs:write"],
            &["if_jobs:filter_tags:deno"],
        ] {
            assert!(lists(unfiltered, "f/other/etl"), "{unfiltered:?}");
        }

        for unconfined in [
            &["jobs:read:*"][..],
            &["jobs:read:f/served/*", "jobs:read"],
            &["jobs:read:f/served/*", "jobs:write"],
            &["jobs:write:f/served/*"],
        ] {
            assert!(
                job_read_confinement(Some(&scopes(unconfined))).is_none(),
                "{unconfined:?}"
            );
        }

        let id = "0190f4c2-0000-7000-8000-000000000000";
        for route in [
            format!("jobs_u/get/{id}"),
            format!("jobs_u/completed/get_result/{id}"),
            format!("jobs_u/getupdate_sse/{id}"),
            format!("jobs_u/flow/approval_info/{id}"),
            "jobs/list".to_string(),
            "jobs/completed/list".to_string(),
            "jobs/queue/list".to_string(),
        ] {
            let route = format!("/api/w/ws/{route}");
            assert!(
                check_route_access(&scoped, &route, "GET").is_ok(),
                "{route}"
            );
        }
        // Reads that aggregate over the workspace, resolve past the job they name, or
        // mint a credential stay with a `jobs:read` that names no path.
        for route in [
            "jobs/list_filtered_uuids".to_string(),
            "jobs/queue/list_filtered_uuids".to_string(),
            "jobs/queue/count".to_string(),
            "jobs/completed/count".to_string(),
            "jobs/completed/count_jobs".to_string(),
            "jobs/completed/export".to_string(),
            "jobs/queue/export".to_string(),
            "jobs/asset_dispatch_edges".to_string(),
            format!("jobs/queue/scheduled_for/{id}"),
            format!("jobs/result_by_id/{id}/a"),
            format!("jobs/job_view_token/{id}"),
            format!("jobs_u/get_root_job_id/{id}"),
        ] {
            let route = format!("/api/w/ws/{route}");
            assert!(
                check_route_access(&scoped, &route, "GET").is_err(),
                "{route}"
            );
            assert!(
                check_route_access(&scopes(&["jobs:read"]), &route, "GET").is_ok(),
                "{route}"
            );
            assert!(
                check_route_access(&scopes(&["jobs:read:*"]), &route, "GET").is_ok(),
                "{route}"
            );
        }
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
        // Test canonical string conversion
        assert_eq!(ScopeDomain::Acls.as_str(), "acls");
        assert_eq!(ScopeDomain::RawApps.as_str(), "raw_apps");
        assert_eq!(ScopeDomain::AgentWorkers.as_str(), "agent_workers");
        assert_eq!(
            ScopeDomain::FlowConversations.as_str(),
            "flow_conversations"
        );
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

    // Whole-collection reads (the workspace export, `apps:read`, ...) require the
    // domain with no path. Only a grant spanning every path may satisfy that.
    #[test]
    fn test_unqualified_requirement_needs_a_whole_domain_grant() {
        let unqualified = ScopeDefinition::new("resources", "read", None, None);

        let wildcard = ScopeDefinition::new("resources", "read", None, Some(vec!["*".to_string()]));
        assert!(wildcard.includes(&unqualified));

        let path_scoped = ScopeDefinition::new(
            "resources",
            "read",
            None,
            Some(vec!["f/team/db".to_string(), "u/alice/db".to_string()]),
        );
        assert!(!path_scoped.includes(&unqualified));
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
        assert_eq!(
            scope_for_route("POST", "/api/w/ws/jobs/run/agent/u/x/y").as_deref(),
            Some("jobs:run:agents")
        );

        // Preview/bundle runs have no deployed path and their handlers require the
        // broad `jobs:run` scope, so the derived scope must not carry a kind.
        for path in [
            "/api/w/ws/jobs/run/preview",
            "/api/w/ws/jobs/run/preview_bundle",
            "/api/w/ws/jobs/run/preview_flow",
            "/api/w/ws/jobs/run_wait_result/preview",
            "/api/w/ws/jobs/run_wait_result/preview_flow",
        ] {
            assert_eq!(
                scope_for_route("POST", path).as_deref(),
                Some("jobs:run"),
                "preview route {path} must derive the broad jobs:run scope"
            );
        }

        // By-path runs of a deployed runnable whose path contains "preview" must
        // still derive their kind (not be swept into the broad jobs:run above),
        // otherwise a `jobs:run:scripts:*`/`jobs:run:flows:*` token is denied.
        assert_eq!(
            scope_for_route("POST", "/api/w/ws/jobs/run/p/u/alice/preview_report").as_deref(),
            Some("jobs:run:scripts")
        );
        assert_eq!(
            scope_for_route(
                "POST",
                "/api/w/ws/jobs/run_wait_result/p/f/team/preview_report"
            )
            .as_deref(),
            Some("jobs:run:scripts")
        );
        assert_eq!(
            scope_for_route("POST", "/api/w/ws/jobs/run/f/f/team/preview_report").as_deref(),
            Some("jobs:run:flows")
        );

        // The minted scope actually satisfies the route check it targets.
        let s = scope_for_route("POST", "/api/w/ws/variables/create").unwrap();
        assert!(check_route_access(&[s], "/api/w/ws/variables/create", "POST").is_ok());

        // Unknown route -> None so the caller fails closed.
        assert!(scope_for_route("GET", "/healthz").is_none());
    }

    #[test]
    fn jobs_cancel_grants_only_the_cancel_routes() {
        let cancel = vec!["jobs:cancel:f/served/*".to_string()];
        let id = "0190f4c2-0000-7000-8000-000000000000";
        for route in [
            format!("/api/w/ws/jobs_u/queue/cancel/{id}"),
            format!("/api/w/ws/jobs_u/queue/force_cancel/{id}"),
            "/api/w/ws/jobs_u/queue/cancel_persistent/f/served/s".to_string(),
            "/api/w/ws/jobs/queue/cancel_selection".to_string(),
        ] {
            assert!(
                check_route_access(&cancel, &route, "POST").is_ok(),
                "{route}"
            );
            assert!(
                check_route_access(&["jobs:write".to_string()], &route, "POST").is_ok(),
                "{route}"
            );
            assert!(
                check_route_access(&["jobs:read".to_string()], &route, "POST").is_err(),
                "{route}"
            );
        }
        for (route, method) in [
            (format!("/api/w/ws/jobs_u/get/{id}"), "GET"),
            ("/api/w/ws/jobs/list".to_string(), "GET"),
            (format!("/api/w/ws/jobs/flow/resume_suspended/{id}"), "POST"),
            (format!("/api/w/ws/jobs/queue/run_now/{id}"), "POST"),
            ("/api/w/ws/jobs/run/p/f/served/s".to_string(), "POST"),
        ] {
            assert!(
                check_route_access(&cancel, &route, method).is_err(),
                "{route}"
            );
        }
    }

    #[test]
    fn jobs_cancel_path_confinement() {
        let scopes = |s: &[&str]| s.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let conf =
            job_cancel_path_confinement(Some(&scopes(&["jobs:cancel:f/served/*,u/svc/kill_me"])))
                .unwrap();
        assert!(cancel_confinement_admits(&conf, "f/served/etl"));
        assert!(cancel_confinement_admits(&conf, "u/svc/kill_me"));
        assert!(!cancel_confinement_admits(&conf, "f/other/etl"));
        assert!(!cancel_confinement_admits(&conf, "f/served_not/etl"));

        // A cancel grant that is not path-scoped leaves cancels unconfined.
        for s in [
            &["jobs:cancel"][..],
            &["jobs:cancel:f/a/*", "jobs:write"],
            &[],
        ] {
            assert!(
                job_cancel_path_confinement(Some(&scopes(s))).is_none(),
                "{s:?}"
            );
        }
        // A cancel scope neither confines nor frees a run token's reads.
        let run = scopes(&["jobs:run:flows:f/a/b", "jobs:cancel"]);
        assert!(job_read_confinement(Some(&run)).is_some());
    }
}
