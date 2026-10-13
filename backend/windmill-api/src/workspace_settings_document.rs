/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! The workspace settings document: the workspace's settings in the layout of the `settings.yaml`
//! that `wmill sync` writes, read and applied as one. `wmill sync push`, the settings page's YAML
//! mode and the AI tools all apply through [`apply_settings_document`], so the file layout and
//! the rules for applying it exist once.

use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::PgConnection;
use windmill_api_workspaces::workspaces::{apply_settings_edit, SettingsEdit};
use windmill_common::{
    error::{Error, JsonResult, Result},
    utils::require_admin,
    variables::build_crypt,
    DB,
};

use crate::db::ApiAuthed;

/// Stands for a secret a reader may not see. Sent back unchanged, it keeps the stored value.
pub const SENSITIVE_UNCHANGED: &str = "__SENSITIVE_AND_UNCHANGED__";
use windmill_common::variables::SETTINGS_ENCRYPTED_PREFIX as ENCRYPTED_PREFIX;

const SECRET_KEYS: &[&str] = &["slack_oauth_client_secret"];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SecretMode {
    /// Secrets as [`SENSITIVE_UNCHANGED`], for the UI and AI tools.
    #[default]
    Redacted,
    /// Secrets encrypted with the workspace key, deterministically so that an unchanged secret
    /// exports to the same text and a synced file does not change on every pull.
    Encrypted,
    Plain,
}

#[derive(sqlx::FromRow)]
struct SettingsRow {
    auto_invite: Option<Value>,
    webhook: Option<String>,
    error_handler: Option<Value>,
    success_handler: Option<Value>,
    ai_config: Option<Value>,
    large_file_storage: Option<Value>,
    git_sync: Option<Value>,
    default_app: Option<String>,
    default_scripts: Option<Value>,
    name: Option<String>,
    mute_critical_alerts: Option<bool>,
    color: Option<String>,
    operator_settings: Option<Value>,
    datatable: Option<Value>,
    slack_team_id: Option<String>,
    slack_name: Option<String>,
    slack_command_script: Option<String>,
    slack_oauth_client_id: Option<String>,
    slack_oauth_client_secret: Option<String>,
    ducklake: Option<Value>,
    dbt_warehouses: Option<Value>,
    deploy_ui: Option<Value>,
    teams_command_script: Option<String>,
    public_app_execution_limit_per_minute: Option<i32>,
    guest_access_enabled: Option<bool>,
    guest_jwt_public_key: Option<String>,
    guest_jwt_jwks_url: Option<String>,
    error_handler_fallback_to_instance_alerts: Option<bool>,
    add_admins_and_developers_to_forks: Option<bool>,
}

/// Server-owned git sync state (the webhook secret and hook id, the synced sha, the admin
/// automatic pulls run as, what the credential check observed) never leaves the server: a
/// re-imported workspace must not inherit another install's hook, sync state or pull identity,
/// and in a synced file it would churn for no reason.
pub(crate) fn redact_git_sync(git_sync: Option<Value>) -> Option<Value> {
    let mut git_sync = git_sync?;
    if let Some(repos) = git_sync
        .get_mut("repositories")
        .and_then(|r| r.as_array_mut())
    {
        for repo in repos {
            if let Some(auto_pull) = repo.get_mut("auto_pull").and_then(|a| a.as_object_mut()) {
                for field in [
                    "webhook_secret",
                    "webhook_id",
                    "webhook_error",
                    "last_synced_sha",
                    "last_pull_status",
                    "enabled_by",
                ] {
                    auto_pull.remove(field);
                }
            }
            if let Some(repo) = repo.as_object_mut() {
                repo.remove("credential");
            }
        }
    }
    Some(git_sync)
}

pub async fn load_settings_document(
    conn: &mut PgConnection,
    db: &DB,
    w_id: &str,
    secrets: SecretMode,
) -> Result<Map<String, Value>> {
    let row = sqlx::query_as::<_, SettingsRow>(
        r#"SELECT
             auto_invite, webhook, error_handler, success_handler, ai_config, large_file_storage,
             git_sync, default_app, default_scripts, workspace.name AS name, mute_critical_alerts,
             color, operator_settings, datatable, slack_team_id, slack_name, slack_command_script,
             slack_oauth_client_id, slack_oauth_client_secret, ducklake, dbt_warehouses, deploy_ui,
             teams_command_script, public_app_execution_limit_per_minute, guest_access_enabled,
             guest_jwt_public_key, guest_jwt_jwks_url, error_handler_fallback_to_instance_alerts,
             add_admins_and_developers_to_forks
         FROM workspace_settings
         LEFT JOIN workspace ON workspace.id = workspace_settings.workspace_id
         WHERE workspace_id = $1"#,
    )
    .bind(w_id)
    .fetch_one(&mut *conn)
    .await?;

    let slack_oauth_client_secret = match (row.slack_oauth_client_secret, secrets) {
        (Some(s), SecretMode::Redacted) if !s.is_empty() => Some(SENSITIVE_UNCHANGED.to_string()),
        (Some(s), SecretMode::Encrypted) if !s.is_empty() => {
            let mc = build_crypt(db, w_id).await?;
            Some(format!(
                "{ENCRYPTED_PREFIX}{}",
                mc.encrypt_str_deterministic(&s)
            ))
        }
        (s, _) => s,
    };

    // In the order `wmill sync pull` has always written settings.yaml. A key that predates the
    // document and is not marked `always` is written only when set, as it always was; a key
    // added since is always written, so a pulled file states it and from then on manages it.
    let entries: [(&str, Option<Value>, bool); 29] = [
        ("auto_invite", row.auto_invite, false),
        ("webhook", row.webhook.map(Value::String), false),
        ("error_handler", row.error_handler, true),
        ("success_handler", row.success_handler, true),
        ("ai_config", row.ai_config, false),
        ("large_file_storage", row.large_file_storage, false),
        ("git_sync", redact_git_sync(row.git_sync), false),
        ("default_app", row.default_app.map(Value::String), false),
        ("default_scripts", row.default_scripts, false),
        ("name", Some(json!(row.name.unwrap_or_default())), true),
        ("mute_critical_alerts", row.mute_critical_alerts.map(Value::Bool), false),
        ("color", row.color.map(Value::String), false),
        ("operator_settings", row.operator_settings, false),
        (
            "datatable",
            windmill_common::workspaces::strip_datatable_permissions(row.datatable),
            false,
        ),
        ("slack_team_id", row.slack_team_id.map(Value::String), false),
        ("slack_name", row.slack_name.map(Value::String), false),
        ("slack_command_script", row.slack_command_script.map(Value::String), false),
        ("slack_oauth_client_id", row.slack_oauth_client_id.map(Value::String), true),
        ("slack_oauth_client_secret", slack_oauth_client_secret.map(Value::String), true),
        ("ducklake", row.ducklake, true),
        ("dbt_warehouses", row.dbt_warehouses, true),
        ("deploy_ui", row.deploy_ui, true),
        ("teams_command_script", row.teams_command_script.map(Value::String), true),
        (
            "public_app_execution_limit_per_minute",
            row.public_app_execution_limit_per_minute.map(|n| json!(n)),
            true,
        ),
        ("guest_access_enabled", Some(json!(row.guest_access_enabled.unwrap_or(false))), true),
        ("guest_jwt_public_key", row.guest_jwt_public_key.map(Value::String), true),
        ("guest_jwt_jwks_url", row.guest_jwt_jwks_url.map(Value::String), true),
        (
            "error_handler_fallback_to_instance_alerts",
            Some(json!(row.error_handler_fallback_to_instance_alerts.unwrap_or(false))),
            true,
        ),
        (
            "add_admins_and_developers_to_forks",
            Some(json!(row.add_admins_and_developers_to_forks.unwrap_or(false))),
            true,
        ),
    ];
    let mut doc = Map::new();
    for (key, value, always) in entries {
        if always || value.is_some() {
            doc.insert(key.to_string(), value.unwrap_or(Value::Null));
        }
    }
    Ok(doc)
}

#[derive(Deserialize)]
pub struct GetSettingsDocumentQuery {
    #[serde(default)]
    secrets: SecretMode,
}

pub async fn get_settings_document(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(q): Query<GetSettingsDocumentQuery>,
) -> JsonResult<Map<String, Value>> {
    require_admin(authed.is_admin, &authed.username)?;
    if q.secrets == SecretMode::Plain {
        return Err(Error::BadRequest(
            "Plain secrets are only exported by the workspace archive".to_string(),
        ));
    }
    let mut conn = db.acquire().await?;
    Ok(Json(
        load_settings_document(&mut conn, &db, &w_id, q.secrets).await?,
    ))
}

#[derive(Deserialize)]
pub struct ApplySettingsDocument {
    document: Map<String, Value>,
    /// Clear the settings the document omits that `wmill sync push` has always cleared when
    /// settings.yaml omits them. Otherwise a setting the document omits is left as it is.
    #[serde(default)]
    replace: bool,
    #[serde(default)]
    dry_run: bool,
}

#[derive(Serialize, Debug)]
pub struct SettingsChange {
    key: String,
    before: Value,
    after: Value,
}

#[derive(Serialize, Debug, Default)]
pub struct ApplySettingsDocumentResult {
    changes: Vec<SettingsChange>,
    applied: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}

/// Where one planned write goes: a settings endpoint of the workspaces crate, or one of the few
/// that live in this crate.
#[derive(Debug)]
enum Target {
    Edit(SettingsEdit),
    AiConfig,
    MuteCriticalAlerts,
    TeamsCommand,
}

#[derive(Debug)]
struct Write {
    target: Target,
    body: Value,
    /// The document keys this write applies, for error messages.
    keys: Vec<String>,
}

fn is_null(v: Option<&Value>) -> bool {
    v.is_none_or(Value::is_null)
}

/// The endpoints drop a null member rather than store it, so a document stating one would
/// otherwise differ from the stored value on every apply.
fn without_null_members(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(
            m.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), without_null_members(v)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(without_null_members).collect()),
        v => v.clone(),
    }
}

fn same(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (a, b) if is_null(a) && is_null(b) => true,
        (Some(a), Some(b)) => without_null_members(a) == without_null_members(b),
        _ => false,
    }
}

/// Switches the endpoint stores as `false` when cleared, compared as `false` when unset.
const FALSE_WHEN_UNSET: &[&str] = &[
    "mute_critical_alerts",
    "guest_access_enabled",
    "add_admins_and_developers_to_forks",
    "error_handler_fallback_to_instance_alerts",
];

fn null_as_false(v: Option<Value>) -> Option<Value> {
    v.map(|v| if v.is_null() { Value::Bool(false) } else { v })
}

fn redact(key: &str, v: Value) -> Value {
    if SECRET_KEYS.contains(&key) && !v.is_null() {
        json!(SENSITIVE_UNCHANGED)
    } else {
        v
    }
}

/// The auto-invite object without the instance group keys, which a separate endpoint owns.
fn domain_invite(v: Option<&Value>) -> Value {
    let mut m = v.and_then(Value::as_object).cloned().unwrap_or_default();
    m.remove("instance_groups");
    m.remove("instance_groups_roles");
    Value::Object(m)
}

struct Plan {
    current: Map<String, Value>,
    desired: Map<String, Value>,
    replace: bool,
    changes: Vec<SettingsChange>,
    writes: Vec<Write>,
}

impl Plan {
    /// The document's value for a key that predates the document: under `replace`, an omitted
    /// one counts as `null`, which clears it.
    fn legacy(&self, key: &str) -> Option<Value> {
        match self.desired.get(key) {
            Some(v) => Some(v.clone()),
            None if self.replace => Some(Value::Null),
            None => None,
        }
    }

    /// The document's value for a key that is only managed when present.
    fn managed(&self, key: &str) -> Option<Value> {
        self.desired.get(key).cloned()
    }

    fn differs(&self, key: &str, desired: &Value) -> bool {
        let current = self.current.get(key).cloned();
        let current = if FALSE_WHEN_UNSET.contains(&key) {
            null_as_false(Some(current.unwrap_or(Value::Null)))
        } else {
            current
        };
        !same(current.as_ref(), Some(desired))
    }

    fn record(&mut self, key: &str, after: Value) {
        let before = self.current.get(key).cloned().unwrap_or(Value::Null);
        self.changes.push(SettingsChange {
            key: key.to_string(),
            before: redact(key, before),
            after: redact(key, after),
        });
    }

    /// A write applying the keys recorded since the previous one.
    fn write(&mut self, target: Target, body: Value) {
        let from = self.writes.iter().map(|w| w.keys.len()).sum::<usize>();
        let keys = self.changes[from..].iter().map(|c| c.key.clone()).collect();
        self.writes.push(Write { target, body, keys });
    }

    /// The common case: one key, one endpoint, a body built from the desired value.
    fn simple(
        &mut self,
        key: &str,
        desired: Option<Value>,
        target: Target,
        body: impl FnOnce(Value) -> Value,
    ) {
        if let Some(desired) = desired {
            if self.differs(key, &desired) {
                self.record(key, desired.clone());
                self.write(target, body(desired));
            }
        }
    }
}

fn resolve_secret(
    key: &str,
    v: Value,
    current: Option<&Value>,
    mc: &windmill_common::variables::WorkspaceCrypt,
) -> Result<Value> {
    match v.as_str() {
        Some(SENSITIVE_UNCHANGED) => Ok(current.cloned().unwrap_or(Value::Null)),
        Some(s) if s.starts_with(ENCRYPTED_PREFIX) => mc
            .decrypt_str(&s[ENCRYPTED_PREFIX.len()..])
            .map(Value::String)
            .map_err(|_| {
                Error::BadRequest(format!(
                    "{key} is encrypted with a different workspace key than this workspace's. \
                     Write it in plain text, or sync the workspace key (includeKey) first."
                ))
            }),
        _ => Ok(v),
    }
}

async fn plan(db: &DB, w_id: &str, mut desired: Map<String, Value>, replace: bool) -> Result<Plan> {
    let current = {
        let mut conn = db.acquire().await?;
        load_settings_document(&mut conn, db, w_id, SecretMode::Plain).await?
    };
    let mc = build_crypt(db, w_id).await?;
    for key in SECRET_KEYS {
        if let Some(v) = desired.remove(*key) {
            desired.insert(
                key.to_string(),
                resolve_secret(key, v, current.get(*key), &mc)?,
            );
        }
    }

    let mut p = Plan { current, desired, replace, changes: vec![], writes: vec![] };

    let webhook = p.legacy("webhook");
    p.simple(
        "webhook",
        webhook,
        Target::Edit(SettingsEdit::Webhook),
        |v| json!({ "webhook": v }),
    );

    if let Some(auto_invite) = p.legacy("auto_invite") {
        let desired_domain = domain_invite(Some(&auto_invite));
        if desired_domain != domain_invite(p.current.get("auto_invite")) {
            if let Some(mode) = desired_domain.get("mode").and_then(Value::as_str) {
                if mode != "invite" && mode != "add" {
                    return Err(Error::BadRequest(
                        r#"Invalid value for auto_invite.mode. Valid values are "invite" and "add""#
                            .to_string(),
                    ));
                }
            }
            let mut after = if auto_invite.is_null() { Value::Null } else { desired_domain.clone() };
            if let (Some(after), Some(cur)) = (
                after.as_object_mut(),
                p.current.get("auto_invite").and_then(Value::as_object),
            ) {
                for k in ["instance_groups", "instance_groups_roles"] {
                    if let Some(v) = cur.get(k) {
                        after.insert(k.to_string(), v.clone());
                    }
                }
            }
            p.record("auto_invite", after);
            let enabled = desired_domain.get("enabled").and_then(Value::as_bool) == Some(true);
            let body = if enabled {
                json!({
                    "operator": desired_domain.get("operator").and_then(Value::as_bool).unwrap_or(false),
                    "invite_all": true,
                    "auto_add": desired_domain.get("mode").and_then(Value::as_str) == Some("add"),
                })
            } else {
                json!({})
            };
            p.write(Target::Edit(SettingsEdit::AutoInvite), body);
        }
        // Clearing an instance group removes the members it granted, so only a document that
        // states the groups manages them.
        if let Some(groups) = auto_invite.get("instance_groups").filter(|g| !g.is_null()) {
            let roles = auto_invite
                .get("instance_groups_roles")
                .filter(|r| !r.is_null())
                .cloned()
                .unwrap_or(json!({}));
            let cur = p.current.get("auto_invite");
            let cur_groups = cur.and_then(|c| c.get("instance_groups"));
            let cur_roles = cur
                .and_then(|c| c.get("instance_groups_roles"))
                .filter(|r| !r.is_null())
                .cloned()
                .unwrap_or(json!({}));
            if !same(Some(groups), cur_groups) || roles != cur_roles {
                p.record(
                    "auto_invite.instance_groups",
                    json!({ "instance_groups": groups, "instance_groups_roles": roles }),
                );
                p.write(
                    Target::Edit(SettingsEdit::InstanceGroups),
                    json!({ "groups": groups, "roles": roles }),
                );
            }
        }
    }

    let ai_config = p.legacy("ai_config");
    p.simple("ai_config", ai_config, Target::AiConfig, |v| {
        if v.is_null() {
            json!({})
        } else {
            v
        }
    });

    let fallback = null_as_false(p.managed("error_handler_fallback_to_instance_alerts"));
    let fallback_changes = fallback
        .as_ref()
        .is_some_and(|f| p.differs("error_handler_fallback_to_instance_alerts", f));
    let error_handler = p.legacy("error_handler");
    let error_handler_changes = error_handler
        .as_ref()
        .is_some_and(|e| p.differs("error_handler", e));
    if error_handler_changes || fallback_changes {
        let handler = if error_handler_changes {
            error_handler.clone().unwrap_or(Value::Null)
        } else {
            p.current
                .get("error_handler")
                .cloned()
                .unwrap_or(Value::Null)
        };
        if error_handler_changes {
            p.record("error_handler", handler.clone());
        }
        if fallback_changes {
            p.record(
                "error_handler_fallback_to_instance_alerts",
                fallback.clone().unwrap_or(Value::Null),
            );
        }
        p.write(
            Target::Edit(SettingsEdit::ErrorHandler),
            json!({
                "path": handler.get("path"),
                "extra_args": handler.get("extra_args"),
                "muted_on_cancel": handler.get("muted_on_cancel").and_then(Value::as_bool).unwrap_or(false),
                "muted_on_user_path": handler.get("muted_on_user_path").and_then(Value::as_bool).unwrap_or(false),
                "fallback_to_instance_alerts": if fallback_changes { fallback.and_then(|f| f.as_bool()) } else { None },
            }),
        );
    }

    let success_handler = p.legacy("success_handler");
    p.simple(
        "success_handler",
        success_handler,
        Target::Edit(SettingsEdit::SuccessHandler),
        |v| json!({ "path": v.get("path"), "extra_args": v.get("extra_args") }),
    );

    let lfs = p.legacy("large_file_storage");
    p.simple(
        "large_file_storage",
        lfs,
        Target::Edit(SettingsEdit::LargeFileStorage),
        |v| json!({ "large_file_storage": v }),
    );

    let git_sync = p.legacy("git_sync");
    p.simple(
        "git_sync",
        git_sync,
        Target::Edit(SettingsEdit::GitSync),
        |v| json!({ "git_sync_settings": v }),
    );

    let default_scripts = p.legacy("default_scripts");
    p.simple(
        "default_scripts",
        default_scripts,
        Target::Edit(SettingsEdit::DefaultScripts),
        |v| v,
    );

    let default_app = p.legacy("default_app");
    p.simple(
        "default_app",
        default_app,
        Target::Edit(SettingsEdit::DefaultApp),
        |v| json!({ "default_app_path": v }),
    );

    // The display name is never applied: settings.yaml is shared across a repo's branches, so
    // applying it would let one workspace's name overwrite another's.

    let mute = null_as_false(p.legacy("mute_critical_alerts"));
    p.simple(
        "mute_critical_alerts",
        mute,
        Target::MuteCriticalAlerts,
        |v| json!({ "mute_critical_alerts": v }),
    );

    // `sync pull` omits the key for a workspace without a color, so an absent or null color
    // means "not managed", never "clear".
    let color = p.managed("color").filter(|c| !c.is_null());
    p.simple(
        "color",
        color,
        Target::Edit(SettingsEdit::Color),
        |v| json!({ "color": v }),
    );

    if let Some(desired) = p.managed("operator_settings").filter(|o| !o.is_null()) {
        // The endpoint keeps the stored value of a write right the document omits, so an
        // older document must not count as differing on those keys.
        let mut compared = desired.clone();
        if let (Some(compared), Some(cur)) = (
            compared.as_object_mut(),
            p.current
                .get("operator_settings")
                .and_then(Value::as_object),
        ) {
            for key in [
                "builder_flows",
                "builder_apps",
                "manage_schedules",
                "manage_triggers",
            ] {
                if !compared.contains_key(key) {
                    if let Some(v) = cur.get(key) {
                        compared.insert(key.to_string(), v.clone());
                    }
                }
            }
        }
        if p.differs("operator_settings", &compared) {
            p.record("operator_settings", compared);
            p.write(Target::Edit(SettingsEdit::OperatorSettings), desired);
        }
    }

    let datatable = p.legacy("datatable");
    p.simple(
        "datatable",
        datatable,
        Target::Edit(SettingsEdit::Datatable),
        |v| json!({ "settings": if v.is_null() { json!({ "datatables": {} }) } else { v } }),
    );

    let slack_command = p.legacy("slack_command_script");
    p.simple(
        "slack_command_script",
        slack_command,
        Target::Edit(SettingsEdit::SlackCommand),
        |v| json!({ "slack_command_script": v }),
    );

    let (client_id, client_secret) = (
        p.legacy("slack_oauth_client_id"),
        p.legacy("slack_oauth_client_secret"),
    );
    if client_id.is_some() || client_secret.is_some() {
        let id = client_id.unwrap_or_else(|| {
            p.current
                .get("slack_oauth_client_id")
                .cloned()
                .unwrap_or(Value::Null)
        });
        let secret = client_secret.unwrap_or_else(|| {
            p.current
                .get("slack_oauth_client_secret")
                .cloned()
                .unwrap_or(Value::Null)
        });
        let id_changes = p.differs("slack_oauth_client_id", &id);
        let secret_changes = p.differs("slack_oauth_client_secret", &secret);
        if id_changes || secret_changes {
            if id_changes {
                p.record("slack_oauth_client_id", id.clone());
            }
            if secret_changes {
                p.record("slack_oauth_client_secret", secret.clone());
            }
            let set = |v: &Value| v.as_str().is_some_and(|s| !s.is_empty());
            if set(&id) && set(&secret) {
                p.write(
                    Target::Edit(SettingsEdit::SetSlackOAuth),
                    json!({ "slack_oauth_client_id": id, "slack_oauth_client_secret": secret }),
                );
            } else {
                p.write(Target::Edit(SettingsEdit::DeleteSlackOAuth), Value::Null);
            }
        }
    }

    // Settings added to the document after `wmill sync` shipped are managed only when the
    // document states them, so a settings.yaml written before them never clears them.
    let ducklake = p.managed("ducklake");
    p.simple(
        "ducklake",
        ducklake,
        Target::Edit(SettingsEdit::Ducklake),
        |v| json!({ "settings": if v.is_null() { json!({ "ducklakes": {} }) } else { v } }),
    );

    let dbt = p.managed("dbt_warehouses");
    p.simple(
        "dbt_warehouses",
        dbt,
        Target::Edit(SettingsEdit::DbtWarehouses),
        |v| json!({ "dbt_warehouses": v }),
    );

    let deploy_ui = p.managed("deploy_ui");
    p.simple(
        "deploy_ui",
        deploy_ui,
        Target::Edit(SettingsEdit::DeployUi),
        |v| json!({ "deploy_ui_settings": v }),
    );

    let teams = p.managed("teams_command_script");
    p.simple(
        "teams_command_script",
        teams,
        Target::TeamsCommand,
        |v| json!({ "teams_command_script": v }),
    );

    let rate = p.managed("public_app_execution_limit_per_minute");
    p.simple(
        "public_app_execution_limit_per_minute",
        rate,
        Target::Edit(SettingsEdit::PublicAppRateLimit),
        |v| json!({ "public_app_execution_limit_per_minute": v }),
    );

    let guest_access = null_as_false(p.managed("guest_access_enabled"));
    p.simple(
        "guest_access_enabled",
        guest_access,
        Target::Edit(SettingsEdit::GuestAccess),
        |v| json!({ "guest_access_enabled": v.as_bool().unwrap_or(false) }),
    );

    let (public_key, jwks_url) = (
        p.managed("guest_jwt_public_key"),
        p.managed("guest_jwt_jwks_url"),
    );
    if public_key.is_some() || jwks_url.is_some() {
        let public_key = public_key.unwrap_or_else(|| {
            p.current
                .get("guest_jwt_public_key")
                .cloned()
                .unwrap_or(Value::Null)
        });
        let jwks_url = jwks_url.unwrap_or_else(|| {
            p.current
                .get("guest_jwt_jwks_url")
                .cloned()
                .unwrap_or(Value::Null)
        });
        let key_changes = p.differs("guest_jwt_public_key", &public_key);
        let url_changes = p.differs("guest_jwt_jwks_url", &jwks_url);
        if key_changes || url_changes {
            if key_changes {
                p.record("guest_jwt_public_key", public_key.clone());
            }
            if url_changes {
                p.record("guest_jwt_jwks_url", jwks_url.clone());
            }
            p.write(
                Target::Edit(SettingsEdit::GuestJwtKey),
                json!({ "public_key": public_key, "jwks_url": jwks_url }),
            );
        }
    }

    let forks = null_as_false(p.managed("add_admins_and_developers_to_forks"));
    p.simple(
        "add_admins_and_developers_to_forks",
        forks,
        Target::Edit(SettingsEdit::AddAdminsAndDevelopersToForks),
        |v| json!({ "add_admins_and_developers_to_forks": v.as_bool().unwrap_or(false) }),
    );

    Ok(p)
}

async fn execute(authed: &ApiAuthed, db: &DB, w_id: &str, write: Write) -> Result<Value> {
    let (a, d, p) = (
        authed.clone(),
        Extension(db.clone()),
        Path(w_id.to_string()),
    );
    let body_err = |e: serde_json::Error| {
        Error::BadRequest(format!("Invalid value for {:?}: {e}", write.target))
    };
    match write.target {
        Target::Edit(SettingsEdit::AutoInvite) => {
            match apply_settings_edit(
                a.clone(),
                db.clone(),
                w_id.to_string(),
                SettingsEdit::AutoInvite,
                write.body.clone(),
            )
            .await
            {
                // Inviting every domain is refused on cloud, where only the workspace's own
                // domain can be auto-invited.
                Err(_) if write.body.get("invite_all") == Some(&json!(true)) => {
                    let mut body = write.body;
                    body["invite_all"] = json!(false);
                    apply_settings_edit(
                        a,
                        db.clone(),
                        w_id.to_string(),
                        SettingsEdit::AutoInvite,
                        body,
                    )
                    .await
                }
                r => r,
            }
        }
        Target::Edit(edit) => {
            apply_settings_edit(a, db.clone(), w_id.to_string(), edit, write.body).await
        }
        Target::AiConfig => {
            let body = serde_json::from_value(write.body).map_err(body_err)?;
            let _ = crate::workspaces::edit_copilot_config(a, d, p, authed.clone(), Json(body)).await?;
            Ok(Value::Null)
        }
        #[cfg(feature = "enterprise")]
        Target::MuteCriticalAlerts => {
            let body = serde_json::from_value(write.body).map_err(body_err)?;
            crate::workspaces::mute_critical_alerts(d, p, a, Json(body)).await?;
            Ok(Value::Null)
        }
        #[cfg(not(feature = "enterprise"))]
        Target::MuteCriticalAlerts => Err(crate::workspaces::mute_critical_alerts().await),
        #[cfg(feature = "private")]
        Target::TeamsCommand => {
            let body = serde_json::from_value(write.body).map_err(body_err)?;
            crate::teams_ee::edit_teams_command(a, d, p, authed.clone(), Json(body)).await?;
            Ok(Value::Null)
        }
        #[cfg(not(feature = "private"))]
        Target::TeamsCommand => {
            crate::teams_oss::edit_teams_command().await?;
            Ok(Value::Null)
        }
    }
}

/// Applies `document` as a list of writes, each through the endpoint that owns the setting, in
/// the order `wmill sync push` has always written them. The writes are not one transaction: a
/// failing one stops the apply, and the error names the settings already written.
pub async fn apply_settings_document(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<ApplySettingsDocument>,
) -> JsonResult<ApplySettingsDocumentResult> {
    require_admin(authed.is_admin, &authed.username)?;
    let p = plan(&db, &w_id, req.document, req.replace).await?;
    let mut result = ApplySettingsDocumentResult { changes: p.changes, ..Default::default() };
    if req.dry_run || p.writes.is_empty() {
        return Ok(Json(result));
    }
    let mut written: Vec<String> = vec![];
    for write in p.writes {
        let keys = write.keys.clone();
        let response = execute(&authed, &db, &w_id, write).await.map_err(|e| {
            if written.is_empty() {
                return e;
            }
            let applied = format!(
                " (settings already applied before this error: {})",
                written.join(", ")
            );
            match e {
                Error::BadRequest(m) => Error::BadRequest(format!("{m}{applied}")),
                e => Error::BadRequest(format!("{e}{applied}")),
            }
        })?;
        if let Some(stranded) = response
            .get("stranded_references")
            .and_then(Value::as_array)
            .filter(|s| !s.is_empty())
        {
            let refs = stranded
                .iter()
                .filter_map(|r| {
                    Some(format!(
                        "{}/{}",
                        r.get("workspace_id")?.as_str()?,
                        r.get("datatable")?.as_str()?
                    ))
                })
                .collect::<Vec<_>>()
                .join(", ");
            result.warnings.push(format!(
                "Removed data tables governed data tables in other workspaces, which no longer resolve: {refs}. A superadmin can point them somewhere else."
            ));
        }
        written.extend(keys);
    }
    result.applied = true;
    Ok(Json(result))
}
