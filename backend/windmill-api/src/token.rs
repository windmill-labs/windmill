use std::sync::Arc;

use axum::{extract::Query, routing::get, Extension, Json, Router};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use windmill_api_auth::{
    auth::{check_route_layer, maybe_get_workspace_id_from_path},
    check_scopes, AuthCache, OptJobAuthed, Tokened,
};
use windmill_common::{
    auth::{hash_token, is_session_label, is_super_admin_email, JWTAuthClaims},
    client_ip::ClientIp,
    error::{Error, JsonResult},
    jwt,
    users::SUPERADMIN_SECRET_EMAIL,
    DB,
};

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
        ("amqp_triggers", "AMQP"),
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
        (
            "ai_evals",
            "AI Evals",
            "AI agent eval datasets and standalone runs",
            false,
        ),
        (
            "slack_agents",
            "Slack Agents",
            "AI agents answering in Slack channels",
            false,
        ),
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
        .map(|(key, name, desc, req)| {
            let mut scopes = vec![
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
            ];
            // `apps_u/execute_component` and `apps_u/upload_s3_file` are classified as
            // Run actions, so running a deployed app's components needs `apps:run`:
            // without it here no supported token can be granted that access.
            if *key == "apps" {
                scopes.push(ScopeOption {
                    value: "apps:run".to_string(),
                    label: "Run".to_string(),
                    requires_resource_path: *req,
                });
            }
            ScopeDomain {
                name: name.to_string(),
                description: if desc.is_empty() {
                    None
                } else {
                    Some(desc.to_string())
                },
                scopes,
            }
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
                    requires_resource_path: true,
                },
                ScopeOption {
                    value: "jobs:write".to_string(),
                    label: "Write".to_string(),
                    requires_resource_path: false,
                },
                ScopeOption {
                    value: "jobs:cancel".to_string(),
                    label: "Cancel".to_string(),
                    requires_resource_path: true,
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
                ScopeOption {
                    value: "jobs:run:dependencies".to_string(),
                    label: "Generate dependency locks".to_string(),
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

        // Read-only: `trigger_history` is append-only and written by the server
        // alone, so there is no `triggers_history:write`. Its own domain rather
        // than a `schedules`/`*_triggers` alias: one listing spans every kind,
        // and a history row quotes the whole trigger row (a schedule's `args`
        // included), so reading it is an explicit grant rather than a side
        // effect of being able to read the trigger. Path-selectable because the
        // route filters rows by the caller's path grants.
        groups.push(ScopeDomain {
            name: "Trigger History".to_string(),
            description: Some(
                "Read-only access to the modification history of schedules and triggers"
                    .to_string(),
            ),
            scopes: vec![ScopeOption {
                value: "triggers_history:read".to_string(),
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
    Router::new()
        .route("/list/scopes", get(get_all_available_scopes))
        .route("/introspect", get(introspect_token))
}

#[derive(Deserialize)]
struct IntrospectQuery {
    workspace: Option<String>,
    path: Option<String>,
    method: Option<String>,
}

#[derive(Serialize)]
struct TokenIntrospection {
    kind: &'static str,
    email: String,
    username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expiration: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scopes: Option<Vec<String>>,
    read_only: bool,
    /// The token does not carry its owner's workspace admin rights.
    admin_withheld: bool,
    /// The token reaches superadmin routes: its owner is a superadmin and the token neither
    /// withholds that nor is a job's.
    super_admin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_ips: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace: Option<TokenWorkspaceAccess>,
    #[serde(skip_serializing_if = "Option::is_none")]
    route: Option<TokenRouteCheck>,
}

#[derive(Serialize)]
struct TokenWorkspaceAccess {
    workspace_id: String,
    usable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_admin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_operator: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    groups: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    folders: Option<Vec<FolderAccess>>,
}

#[derive(Serialize)]
struct FolderAccess {
    name: String,
    write: bool,
    owner: bool,
}

#[derive(Serialize)]
struct TokenRouteCheck {
    method: String,
    path: String,
    allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// The scope a run-by-path endpoint (`windmill-api/src/jobs.rs`) requires of the runnable once
/// the route layer has admitted the token, which only knows that it may run something. Routes
/// added there with their own `check_scopes` belong here too, or the verdict over-reports.
fn run_by_path_scope(path: &str) -> Option<String> {
    let (_, suffix) = path.strip_prefix("/api/w/")?.split_once('/')?;
    let (endpoint, rest) = suffix.strip_prefix("jobs/")?.split_once('/')?;
    if !matches!(
        endpoint,
        "run" | "run_wait_result" | "run_and_stream" | "run_inline"
    ) {
        return None;
    }
    let kind = match rest.split_once('/')? {
        ("p", runnable) => ("scripts", runnable),
        ("f", runnable) if endpoint != "run_inline" => ("flows", runnable),
        _ => return None,
    };
    Some(format!("jobs:run:{}:{}", kind.0, kind.1))
}

/// Describes only the credential presenting the request (`Tokened`), never one named in the
/// request, so it cannot be used to look up another person's token. It resolves the token
/// itself rather than through `ApiAuthed`: the route layer would refuse a scoped token here,
/// and a workspace-bound token resolves on no route outside its workspace.
async fn introspect_token(
    Extension(db): Extension<DB>,
    Extension(cache): Extension<Arc<AuthCache>>,
    Tokened { token }: Tokened,
    ClientIp(client_ip): ClientIp,
    Query(q): Query<IntrospectQuery>,
) -> JsonResult<TokenIntrospection> {
    let unauthorized = || Error::NotAuthorized("Unauthorized".to_string());
    let row = if token.starts_with("jwt_") {
        None
    } else {
        sqlx::query!(
            "SELECT owner, email, label, token_prefix, created_at, expiration, workspace_id,
                scopes, read_only, super_admin, admin, allowed_ips::text[] AS allowed_ips
             FROM token
             WHERE token_hash = $1 AND (expiration > NOW() OR expiration IS NULL)",
            hash_token(&token)
        )
        .fetch_optional(&db)
        .await?
    };

    let mut out = if let Some(row) = row {
        let allowed =
            windmill_api_auth::token_ip_allowlist_oss::load_allowed_ips(row.allowed_ips.clone());
        if !windmill_api_auth::client_ip_allowed(allowed.as_deref(), client_ip) {
            return Err(unauthorized());
        }
        let email = row.email.ok_or_else(unauthorized)?;
        let is_session = is_session_label(row.label.as_deref());
        TokenIntrospection {
            kind: if is_session { "session" } else { "api_token" },
            username: row
                .owner
                .as_deref()
                .and_then(|o| o.split_once('/'))
                .map(|(_, name)| name.to_string())
                .unwrap_or_else(|| email.clone()),
            email,
            label: row.label,
            token_prefix: Some(row.token_prefix),
            created_at: Some(row.created_at),
            expiration: row.expiration,
            workspace_id: row.workspace_id,
            workspace_ids: None,
            scopes: row.scopes,
            read_only: row.read_only,
            admin_withheld: !row.admin,
            // Not withheld yet; whether the owner holds it is settled below. A session
            // always stands for the account as it is.
            super_admin: row.super_admin || is_session,
            // What the build enforces: a CE build ignores a stored list.
            allowed_ips: allowed.map(|nets| nets.iter().map(|n| n.to_string()).collect()),
            job_id: None,
            workspace: None,
            route: None,
        }
    } else {
        // A guest JWT resolves only inside its workspace.
        let requested = q.workspace.clone().filter(|w| !w.is_empty());
        let resolved = match cache.get_opt_job_authed(None, &token, client_ip).await {
            Some(resolved) => Some(resolved),
            None if requested.is_some() => {
                cache.get_opt_job_authed(requested, &token, client_ip).await
            }
            None => None,
        };
        let OptJobAuthed { authed, job_id } = resolved.ok_or_else(unauthorized)?;
        let mut out = TokenIntrospection {
            kind: if authed.email == SUPERADMIN_SECRET_EMAIL {
                "superadmin_secret"
            } else {
                "external"
            },
            email: authed.email,
            username: authed.username,
            label: None,
            token_prefix: None,
            created_at: None,
            expiration: None,
            workspace_id: None,
            workspace_ids: None,
            scopes: authed.scopes,
            read_only: authed.read_only,
            admin_withheld: authed.admin_withheld,
            super_admin: !authed.super_admin_withheld,
            allowed_ips: None,
            job_id,
            workspace: None,
            route: None,
        };
        if let Some(jwt) = token
            .strip_prefix("jwt_")
            .filter(|_| !token.starts_with("jwt_ext_"))
        {
            out.kind = "jwt";
            if let Ok(claims) = jwt::decode_with_internal_secret::<JWTAuthClaims>(jwt).await {
                out.label = claims.label;
                out.workspace_id = claims.workspace_id;
                out.workspace_ids = claims.workspace_ids;
                out.expiration = chrono::DateTime::from_timestamp(claims.exp as i64, 0);
            }
        }
        out
    };
    // The same three conditions `require_super_admin` checks.
    out.super_admin =
        out.super_admin && out.job_id.is_none() && is_super_admin_email(&db, &out.email).await?;

    if let Some(w_id) = q.workspace.as_deref().filter(|w| !w.is_empty()) {
        let resolved = cache
            .get_opt_job_authed(Some(w_id.to_string()), &token, client_ip)
            .await;
        out.workspace = Some(match resolved {
            Some(OptJobAuthed { authed, .. }) => TokenWorkspaceAccess {
                workspace_id: w_id.to_string(),
                usable: true,
                is_admin: Some(authed.is_admin),
                is_operator: Some(authed.is_operator),
                groups: Some(authed.groups),
                folders: Some(
                    authed
                        .folders
                        .into_iter()
                        .map(|(name, write, owner)| FolderAccess { name, write, owner })
                        .collect(),
                ),
            },
            None => TokenWorkspaceAccess {
                workspace_id: w_id.to_string(),
                usable: false,
                is_admin: None,
                is_operator: None,
                groups: None,
                folders: None,
            },
        });
    }

    if let Some(path) = q.path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        let path = if path.starts_with("/api/") {
            path.to_string()
        } else if let Some(w_id) = q.workspace.as_deref().filter(|w| !w.is_empty()) {
            format!("/api/w/{w_id}/{}", path.trim_start_matches('/'))
        } else {
            return Err(Error::BadRequest(
                "a relative path needs a workspace; pass a full /api/... path".to_string(),
            ));
        };
        let path = path
            .split(['?', '#'])
            .next()
            .unwrap_or_default()
            .to_string();
        let method = q.method.as_deref().unwrap_or("GET").to_ascii_uppercase();
        let path_vec: Vec<&str> = path.split('/').collect();
        let route_workspace = maybe_get_workspace_id_from_path(&path_vec);
        let resolved = cache
            .get_opt_job_authed(route_workspace.clone(), &token, client_ip)
            .await;
        let verdict = match resolved {
            None => Err(match route_workspace.as_deref() {
                Some(w_id) => format!("The token cannot be used in workspace {w_id}."),
                None => "The token cannot be used outside a workspace.".to_string(),
            }),
            Some(mut opt_job_authed) => async {
                check_route_layer(
                    &db,
                    &mut opt_job_authed,
                    route_workspace.as_deref(),
                    &path,
                    &method,
                )
                .await?;
                if let Some(scope) = run_by_path_scope(&path) {
                    check_scopes(&opt_job_authed.authed, || scope)?;
                }
                Ok::<_, Error>(())
            }
            .await
            .map_err(|e| e.to_string()),
        };
        out.route =
            Some(TokenRouteCheck { method, path, allowed: verdict.is_ok(), reason: verdict.err() });
    }

    Ok(Json(out))
}

async fn get_all_available_scopes() -> JsonResult<Vec<ScopeDomain>> {
    Ok(Json(ALL_SCOPES.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_by_path_scope_names_the_runnable() {
        let scope = |p: &str| run_by_path_scope(p);
        assert_eq!(
            scope("/api/w/ws/jobs/run_wait_result/p/f/ci/build").as_deref(),
            Some("jobs:run:scripts:f/ci/build")
        );
        assert_eq!(
            scope("/api/w/ws/jobs/run/f/u/me/flow").as_deref(),
            Some("jobs:run:flows:u/me/flow")
        );
        assert_eq!(scope("/api/w/ws/jobs/run/h/abc"), None);
        assert_eq!(scope("/api/w/ws/jobs/run_inline/f/u/me/flow"), None);
        assert_eq!(scope("/api/w/ws/scripts/list"), None);
    }

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

    /// Running a deployed app's components is enforced as a Run action, so `apps:run`
    /// must be selectable here or no supported token can be granted that access.
    #[test]
    fn apps_run_scope_is_exposed_and_path_selectable() {
        let apps = ALL_SCOPES
            .iter()
            .find(|d| d.name == "Apps")
            .expect("Apps domain must exist");
        let opt = apps
            .scopes
            .iter()
            .find(|s| s.value == "apps:run")
            .expect("apps:run must be selectable");
        assert!(opt.requires_resource_path, "apps:run is path-scoped");
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
