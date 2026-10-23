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
use std::collections::BTreeMap;
use windmill_api_workspaces::workspaces::{apply_settings_edit, SettingsEdit};
use windmill_common::{
    datatable_roles::{
        read_role_catalog, DatatableRoleCluster, ADMIN_DATATABLE_ROLE, NO_DEFAULT_DATATABLE_ROLE,
    },
    error::{Error, JsonResult, Result},
    utils::require_admin,
    variables::build_crypt,
    workspaces::{
        DataTable, DataTablePermissions, ProtectionRuleKind, ProtectionRules,
        DATATABLE_TENANT_WILDCARD, DEV_WORKSPACE_LOCK_RULE_NAME,
    },
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

/// What `datatable_permissions` is resolved against. Role names by catalog id, per cluster: the
/// document names a data table role, the stored
/// permissions its catalog id, which differs from one instance to the next. `None` for a catalog
/// that could not be read, as on an edition without data table roles.
#[derive(Default)]
struct DatatableNames {
    instance: Option<BTreeMap<String, String>>,
    external_instance: Option<BTreeMap<String, String>>,
    /// The `u/`, `g/` and `f/` tenants of the desired document that name no user, group or folder
    /// of the workspace. The endpoint stores such a tenant as a grant that matches nobody, so the
    /// plan is where a typo in a pushed file gets caught.
    unknown_tenants: std::collections::BTreeSet<String>,
}

impl DatatableNames {
    async fn load(db: &DB, w_id: &str, datatable_permissions: &Value) -> Result<Self> {
        let read = |cluster| async move {
            read_role_catalog(db, cluster)
                .await
                .ok()
                .map(|c| c.into_iter().map(|(id, r)| (id, r.name)).collect())
        };
        let tenants = datatable_permissions
            .as_object()
            .into_iter()
            .flat_map(|m| m.values())
            .filter_map(|entry| entry.get("roles")?.as_object())
            .flat_map(|roles| roles.values())
            .filter_map(Value::as_array)
            .flatten()
            .filter_map(Value::as_str);
        let (mut users, mut groups, mut folders) = (vec![], vec![], vec![]);
        for tenant in tenants {
            match tenant.split_once('/') {
                Some(("u", n)) => users.push(n.to_string()),
                Some(("g", n)) => groups.push(n.to_string()),
                Some(("f", n)) => folders.push(n.to_string()),
                _ => {}
            }
        }
        let mut unknown_tenants = std::collections::BTreeSet::new();
        for (prefix, named, query) in [
            (
                "u",
                users,
                "SELECT username FROM usr WHERE workspace_id = $1 AND username = ANY($2)",
            ),
            (
                "g",
                groups,
                "SELECT name FROM group_ WHERE workspace_id = $1 AND name = ANY($2)",
            ),
            (
                "f",
                folders,
                "SELECT name FROM folder WHERE workspace_id = $1 AND name = ANY($2)",
            ),
        ] {
            if named.is_empty() {
                continue;
            }
            let existing: std::collections::HashSet<String> =
                sqlx::query_scalar::<_, String>(query)
                    .bind(w_id)
                    .bind(&named)
                    .fetch_all(db)
                    .await?
                    .into_iter()
                    .collect();
            unknown_tenants.extend(
                named
                    .into_iter()
                    .filter(|n| !existing.contains(n))
                    .map(|n| format!("{prefix}/{n}")),
            );
        }
        Ok(Self {
            instance: read(DatatableRoleCluster::Instance).await,
            external_instance: read(DatatableRoleCluster::ExternalInstance).await,
            unknown_tenants,
        })
    }

    fn of(&self, cluster: DatatableRoleCluster) -> Option<&BTreeMap<String, String>> {
        match cluster {
            DatatableRoleCluster::Instance => self.instance.as_ref(),
            DatatableRoleCluster::ExternalInstance => self.external_instance.as_ref(),
        }
    }
}

/// The cluster whose roles a data table entry can be put under: only an entry that owns a
/// database on a cluster Windmill manages, and is not a clone, which its origin governs.
fn role_cluster_of(entry: &Value) -> Option<DatatableRoleCluster> {
    let entry: DataTable = serde_json::from_value(entry.clone()).ok()?;
    if entry.governed_by.is_some() {
        return None;
    }
    DatatableRoleCluster::of(entry.database?.resource_type)
}

/// Each data table's stored `permissions`, with roles named rather than given by catalog id. A
/// role the catalog no longer has keeps its id, so the document still shows it.
fn datatable_permissions_document(datatable: Option<&Value>, roles: &DatatableNames) -> Value {
    let mut doc = Map::new();
    let entries = datatable
        .and_then(|d| d.get("datatables"))
        .and_then(Value::as_object);
    for (name, entry) in entries.into_iter().flatten() {
        let Some(permissions) = entry
            .get("permissions")
            .filter(|p| !p.is_null())
            .and_then(|p| serde_json::from_value::<DataTablePermissions>(p.clone()).ok())
        else {
            continue;
        };
        let names = role_cluster_of(entry).and_then(|c| roles.of(c));
        let name_of = |id: &str| {
            names
                .and_then(|n| n.get(id))
                .cloned()
                .unwrap_or_else(|| id.to_string())
        };
        let default_role = match permissions.default_role() {
            NO_DEFAULT_DATATABLE_ROLE => Value::Null,
            id => Value::String(name_of(id)),
        };
        let roles: Map<String, Value> = permissions
            .roles
            .iter()
            .map(|(id, t)| (name_of(id), json!(t.tenants)))
            .collect();
        doc.insert(
            name.clone(),
            json!({ "default_role": default_role, "roles": roles }),
        );
    }
    Value::Object(doc)
}

/// Protection rules by name, their rules in a fixed order so a reordered file is no change. The
/// dev workspace lock is left out: the dev workspace setting creates and removes it.
async fn protection_rules_document(conn: &mut PgConnection, w_id: &str) -> Result<Value> {
    let rows = sqlx::query_as::<_, (String, i32, Vec<String>, Vec<String>)>(
        "SELECT name, rules, bypass_groups, bypass_users FROM workspace_protection_rule
         WHERE workspace_id = $1 AND name <> $2 ORDER BY name",
    )
    .bind(w_id)
    .bind(DEV_WORKSPACE_LOCK_RULE_NAME)
    .fetch_all(&mut *conn)
    .await?;
    Ok(Value::Object(
        rows.into_iter()
            .map(|(name, bits, bypass_groups, bypass_users)| {
                let rules = ProtectionRules::from_bits_truncate(bits);
                let rule = json!({
                    "rules": rule_kinds(rules),
                    "bypass_groups": bypass_groups,
                    "bypass_users": bypass_users,
                });
                (name, rule)
            })
            .collect(),
    ))
}

fn rule_kinds(rules: ProtectionRules) -> Value {
    json!(rules.kinds())
}

/// Reads no permission itself: the caller must have checked that the requester is an admin of
/// `w_id`, since `SecretMode::Plain` returns the workspace's secrets in plain text.
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

    let has_permissions = row
        .datatable
        .as_ref()
        .and_then(|d| d.get("datatables"))
        .and_then(Value::as_object)
        .is_some_and(|e| {
            e.values()
                .any(|e| e.get("permissions").is_some_and(|p| !p.is_null()))
        });
    let role_names = if has_permissions {
        DatatableNames::load(db, w_id, &Value::Null).await?
    } else {
        DatatableNames::default()
    };
    let datatable_permissions = datatable_permissions_document(row.datatable.as_ref(), &role_names);
    let protection_rules = protection_rules_document(&mut *conn, w_id).await?;
    let git_sync = redact_git_sync(row.git_sync);
    let git_sync = match fork_parent(&mut *conn, w_id).await? {
        Some(parent) => fork_git_sync_document(git_sync, parent.git_sync),
        None => git_sync,
    };

    // In the order `wmill sync pull` has always written settings.yaml. A key that predates the
    // document and is not marked `always` is written only when set, as it always was; a key
    // added since is always written, so a pulled file states it and from then on manages it.
    let entries: [(&str, Option<Value>, bool); 31] = [
        ("auto_invite", row.auto_invite, false),
        ("webhook", row.webhook.map(Value::String), false),
        ("error_handler", row.error_handler, true),
        ("success_handler", row.success_handler, true),
        ("ai_config", row.ai_config, false),
        ("large_file_storage", row.large_file_storage, false),
        ("git_sync", git_sync, false),
        ("default_app", row.default_app.map(Value::String), false),
        ("default_scripts", row.default_scripts, false),
        ("name", Some(json!(row.name.unwrap_or_default())), true),
        (
            "mute_critical_alerts",
            row.mute_critical_alerts.map(Value::Bool),
            false,
        ),
        ("color", row.color.map(Value::String), false),
        ("operator_settings", row.operator_settings, false),
        (
            "datatable",
            windmill_common::workspaces::strip_datatable_permissions(row.datatable),
            false,
        ),
        ("slack_team_id", row.slack_team_id.map(Value::String), false),
        ("slack_name", row.slack_name.map(Value::String), false),
        (
            "slack_command_script",
            row.slack_command_script.map(Value::String),
            false,
        ),
        (
            "slack_oauth_client_id",
            row.slack_oauth_client_id.map(Value::String),
            true,
        ),
        (
            "slack_oauth_client_secret",
            slack_oauth_client_secret.map(Value::String),
            true,
        ),
        ("ducklake", row.ducklake, true),
        ("dbt_warehouses", row.dbt_warehouses, true),
        ("deploy_ui", row.deploy_ui, true),
        (
            "teams_command_script",
            row.teams_command_script.map(Value::String),
            true,
        ),
        (
            "public_app_execution_limit_per_minute",
            row.public_app_execution_limit_per_minute.map(|n| json!(n)),
            true,
        ),
        (
            "guest_access_enabled",
            Some(json!(row.guest_access_enabled.unwrap_or(false))),
            true,
        ),
        (
            "guest_jwt_public_key",
            row.guest_jwt_public_key.map(Value::String),
            true,
        ),
        (
            "guest_jwt_jwks_url",
            row.guest_jwt_jwks_url.map(Value::String),
            true,
        ),
        (
            "error_handler_fallback_to_instance_alerts",
            Some(json!(row
                .error_handler_fallback_to_instance_alerts
                .unwrap_or(false))),
            true,
        ),
        (
            "add_admins_and_developers_to_forks",
            Some(json!(row
                .add_admins_and_developers_to_forks
                .unwrap_or(false))),
            true,
        ),
        ("datatable_permissions", Some(datatable_permissions), true),
        ("protection_rules", Some(protection_rules), true),
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
/// otherwise differ from the stored value on every apply. A handler's `extra_args` is stored
/// verbatim and its nulls are passed to the job as arguments, so it is compared as is.
fn without_null_members(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(
            m.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| {
                    let v = if k == "extra_args" {
                        v.clone()
                    } else {
                        without_null_members(v)
                    };
                    (k.clone(), v)
                })
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
    warnings: Vec<String>,
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
        self.record_change(key, before, after);
    }

    fn record_change(&mut self, key: &str, before: Value, after: Value) {
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

/// The plain value of a secret the document states, or `None` to keep the stored one: for the
/// placeholder, and for a value encrypted under another workspace's key, which a repo synced to
/// several workspaces carries. That one is reported rather than failing every other setting.
fn resolve_secret(
    key: &str,
    v: Value,
    mc: &windmill_common::variables::WorkspaceCrypt,
    warnings: &mut Vec<String>,
) -> Option<Value> {
    match v.as_str() {
        Some(SENSITIVE_UNCHANGED) => None,
        Some(s) if s.starts_with(ENCRYPTED_PREFIX) => {
            match mc.decrypt_str(&s[ENCRYPTED_PREFIX.len()..]) {
                Ok(plain) => Some(Value::String(plain)),
                Err(_) => {
                    warnings.push(format!(
                        "{key} was left unchanged: it is encrypted with another workspace's key. \
                         Give this workspace its own settings file, export secrets in plain text \
                         (plainSecrets), or share the workspace key (includeKey)."
                    ));
                    None
                }
            }
        }
        _ => Some(v),
    }
}

const KNOWN_KEYS: &[&str] = &[
    "auto_invite",
    "webhook",
    "error_handler",
    "success_handler",
    "ai_config",
    "large_file_storage",
    "git_sync",
    "default_app",
    "default_scripts",
    "name",
    "mute_critical_alerts",
    "color",
    "operator_settings",
    "datatable",
    "slack_team_id",
    "slack_name",
    "slack_command_script",
    "slack_oauth_client_id",
    "slack_oauth_client_secret",
    "ducklake",
    "dbt_warehouses",
    "deploy_ui",
    "teams_command_script",
    "public_app_execution_limit_per_minute",
    "guest_access_enabled",
    "guest_jwt_public_key",
    "guest_jwt_jwks_url",
    "error_handler_fallback_to_instance_alerts",
    "add_admins_and_developers_to_forks",
    "datatable_permissions",
    "protection_rules",
];

fn string_list(v: Option<&Value>, what: &str) -> Result<Value> {
    match v {
        None | Some(Value::Null) => Ok(json!([])),
        Some(Value::Array(a)) if a.iter().all(Value::is_string) => Ok(Value::Array(a.clone())),
        _ => Err(Error::BadRequest(format!(
            "{what} must be a list of strings"
        ))),
    }
}

/// `datatable_permissions` as the endpoint stores it: an omitted default role is `admin`, and
/// `admin` is always one of the roles. A data table stated as null is not under roles, as an
/// omitted one is. A data table's error goes to `errors` and leaves it out of the result, so every
/// data table is checked before the document is refused.
fn normalize_datatable_permissions(v: &Value, errors: &mut Vec<String>) -> Result<Value> {
    let entries = match v {
        Value::Null => return Ok(json!({})),
        Value::Object(m) => m,
        _ => {
            return Err(Error::BadRequest(
                "datatable_permissions must map data tables to their roles".to_string(),
            ))
        }
    };
    let mut out = Map::new();
    for (name, entry) in entries {
        if entry.is_null() {
            continue;
        }
        match normalize_datatable_entry(name, entry) {
            Ok(entry) => {
                out.insert(name.clone(), entry);
            }
            Err(e) => errors.push(error_message(e)),
        }
    }
    Ok(Value::Object(out))
}

fn normalize_datatable_entry(name: &str, entry: &Value) -> Result<Value> {
    let what = format!("datatable_permissions.{name}");
    let entry = entry
        .as_object()
        .ok_or_else(|| Error::BadRequest(format!("{what} must be an object or null")))?;
    let default_role = match entry.get("default_role") {
        None => json!(ADMIN_DATATABLE_ROLE),
        Some(r) if r.is_null() || r.is_string() => r.clone(),
        Some(_) => {
            return Err(Error::BadRequest(format!(
                "{what}.default_role must be a role name or null"
            )))
        }
    };
    let mut roles = Map::new();
    match entry.get("roles") {
        None | Some(Value::Null) => {}
        Some(Value::Object(r)) => {
            for (role, tenants) in r {
                let tenants = string_list(Some(tenants), &format!("{what}.roles.{role}"))?;
                // The endpoint's own check, here so a dry run refuses it too.
                for tenant in tenants.as_array().into_iter().flatten() {
                    let tenant = tenant.as_str().unwrap_or_default();
                    let valid = tenant == DATATABLE_TENANT_WILDCARD
                        || matches!(tenant.split_once('/'),
                                Some(("u" | "g" | "f", rest)) if !rest.is_empty());
                    if !valid {
                        return Err(Error::BadRequest(format!(
                            "Invalid tenant '{tenant}' in {what}.roles.{role}: expected \
                                 'u/<user>', 'g/<group>', 'f/<folder>' or '*'"
                        )));
                    }
                }
                roles.insert(role.clone(), tenants);
            }
        }
        Some(_) => {
            return Err(Error::BadRequest(format!(
                "{what}.roles must map role names to their tenants"
            )))
        }
    }
    roles
        .entry(ADMIN_DATATABLE_ROLE.to_string())
        .or_insert(json!([]));
    if let Some(role) = default_role.as_str() {
        if !roles.contains_key(role) {
            return Err(Error::BadRequest(format!(
                "The default role '{role}' of {what} is not among its roles"
            )));
        }
    }
    Ok(json!({ "default_role": default_role, "roles": roles }))
}

fn error_message(e: Error) -> String {
    match e {
        Error::BadRequest(msg) => msg,
        e => e.to_string(),
    }
}

/// Refuses the document with every data table's error at once.
fn datatable_permissions_errors(errors: Vec<String>) -> Result<()> {
    match errors.len() {
        0 => Ok(()),
        1 => Err(Error::BadRequest(errors.into_iter().next().unwrap())),
        _ => Err(Error::BadRequest(format!(
            "Invalid datatable_permissions, nothing was applied:\n- {}",
            errors.join("\n- ")
        ))),
    }
}

/// `protection_rules` as it is stored: rules in their fixed order, the bypass lists stated. The
/// dev workspace lock is dropped with a warning, since the dev workspace setting owns it.
fn normalize_protection_rules(v: &Value, warnings: &mut Vec<String>) -> Result<Value> {
    let rules = match v {
        Value::Null => return Ok(json!({})),
        Value::Object(m) => m,
        _ => {
            return Err(Error::BadRequest(
                "protection_rules must map rule names to rules".to_string(),
            ))
        }
    };
    let mut out = Map::new();
    for (name, rule) in rules {
        if name == DEV_WORKSPACE_LOCK_RULE_NAME {
            warnings.push(format!(
                "protection_rules.{name} was left unchanged: the dev workspace setting manages it"
            ));
            continue;
        }
        if rule.is_null() {
            continue;
        }
        let what = format!("protection_rules.{name}");
        let rule = rule
            .as_object()
            .ok_or_else(|| Error::BadRequest(format!("{what} must be an object or null")))?;
        let kinds: Vec<ProtectionRuleKind> =
            serde_json::from_value(rule.get("rules").cloned().unwrap_or(json!([])))
                .map_err(|e| Error::BadRequest(format!("Invalid {what}.rules: {e}")))?;
        out.insert(
            name.clone(),
            json!({
                "rules": rule_kinds(ProtectionRules::from(&kinds)),
                "bypass_groups": string_list(rule.get("bypass_groups"), &format!("{what}.bypass_groups"))?,
                "bypass_users": string_list(rule.get("bypass_users"), &format!("{what}.bypass_users"))?,
            }),
        );
    }
    Ok(Value::Object(out))
}

async fn plan(db: &DB, w_id: &str, mut desired: Map<String, Value>, replace: bool) -> Result<Plan> {
    let (mut current, fork) = {
        let mut conn = db.acquire().await?;
        let current = load_settings_document(&mut conn, db, w_id, SecretMode::Plain).await?;
        (current, fork_parent(&mut conn, w_id).await?)
    };
    if let Some(fork) = fork {
        // Diff against what the fork stores, not the parent's view the document shows it.
        let own: Option<Value> =
            sqlx::query_scalar("SELECT git_sync FROM workspace_settings WHERE workspace_id = $1")
                .bind(w_id)
                .fetch_optional(db)
                .await?
                .flatten();
        let own = redact_git_sync(own);
        // A malformed value is left for the endpoint to refuse.
        let stated = match desired.get("git_sync") {
            Some(v) if v.is_object() || v.is_null() => Some(v),
            Some(_) => None,
            None => replace.then_some(&Value::Null),
        };
        if let Some(stated) = stated {
            let applied = fork_git_sync_to_apply(stated, own.as_ref(), fork.is_dev)?;
            desired.insert("git_sync".to_string(), applied);
        }
        match own {
            Some(own) => current.insert("git_sync".to_string(), own),
            None => current.remove("git_sync"),
        };
    }
    let mc = build_crypt(db, w_id).await?;
    let names = match desired.get("datatable_permissions") {
        Some(v) if !v.is_null() => DatatableNames::load(db, w_id, v).await?,
        _ => DatatableNames::default(),
    };
    plan_document(current, desired, replace, &mc, &names)
}

/// A fork's parent, read for its `git_sync`.
struct ForkParent {
    /// As the parent's own document states it: a fork of a fork shares the root's file too.
    git_sync: Option<Value>,
    /// Of the fork itself.
    is_dev: bool,
}

/// `Some` when `w_id` is a fork, by the same test as `reject_parent_only_git_sync_settings_on_fork`.
async fn fork_parent(conn: &mut PgConnection, w_id: &str) -> Result<Option<ForkParent>> {
    // The workspace itself, then its ancestors nearest first; the depth bound only guards
    // against a cycle.
    let mut chain: Vec<(bool, Option<Value>)> = sqlx::query_as(
        "WITH RECURSIVE chain AS (
             SELECT id, parent_workspace_id, is_dev_workspace, 0 AS depth
             FROM workspace WHERE id = $1
             UNION ALL
             SELECT w.id, w.parent_workspace_id, w.is_dev_workspace, c.depth + 1 FROM workspace w
             JOIN chain c ON w.id = c.parent_workspace_id WHERE c.depth < 32
         )
         SELECT c.is_dev_workspace, s.git_sync FROM chain c
         LEFT JOIN workspace_settings s ON s.workspace_id = c.id
         ORDER BY c.depth",
    )
    .bind(w_id)
    .fetch_all(&mut *conn)
    .await?;
    let is_dev = !chain.is_empty() && chain.remove(0).0;
    if chain.is_empty() && !w_id.starts_with(windmill_common::workspaces::WM_FORK_PREFIX) {
        return Ok(None);
    }
    let git_sync = chain.into_iter().rev().fold(None, |above, (_, own)| {
        fork_git_sync_document(redact_git_sync(own), above)
    });
    Ok(Some(ForkParent { git_sync, is_dev }))
}

// A fork shares its parent's settings.yaml (the CLI maps `wm-fork/<branch>/<id>` to the parent's
// entry), so its `git_sync` there is the parent's. Of it, a fork applies only what selects the
// items synced, on the repos it carries: auto-pull, fork PRs and promotion mode are not a fork's
// to take (`reject_parent_only_git_sync_settings_on_fork`), and a field not listed here stays the
// fork's own. The fork's document is the parent's block with the fork's values for these keys,
// the inverse of the apply, so a merge of the fork's branch changes only them in the parent.
const FORK_SYNCED_REPO_KEYS: [&str; 3] = ["settings", "exclude_types_override", "group_by_folder"];

fn repos(git_sync: Option<&Value>) -> &[Value] {
    git_sync
        .and_then(|g| g.get("repositories"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn same_repo<'a>(repos: &'a [Value], repo: &Value) -> Option<&'a Value> {
    let path = repo.get("git_repo_resource_path")?;
    repos
        .iter()
        .find(|r| r.get("git_repo_resource_path") == Some(path))
}

/// The workspace-level filters, for repos without their own `settings`. Synced only while the
/// fork stores them: the save drops them once every repo has its own, so syncing them to a fork
/// without them would rewrite them on every apply.
const FORK_SYNCED_LEGACY_KEYS: [&str; 4] = [
    "include_path",
    "include_type",
    "exclude_path",
    "extra_include_path",
];

/// An unset key and a null one are left as they are: stored blobs differ in which they write, and
/// swapping one for the other would show in settings.yaml as a change.
fn copy_keys(onto: &mut Map<String, Value>, from: &Value, keys: &[&str]) {
    for key in keys {
        if is_null(from.get(*key)) && is_null(onto.get(*key)) {
            continue;
        }
        match from.get(*key) {
            Some(v) => onto.insert(key.to_string(), v.clone()),
            None => onto.remove(*key),
        };
    }
}

/// Sets `onto` to `from`'s values for the synced keys, on each repo `from` also lists.
fn copy_synced_keys(onto: &mut Value, from: &Value, legacy: bool) {
    let from_repos = repos(Some(from));
    let Some(onto) = onto.as_object_mut() else {
        return;
    };
    if legacy {
        copy_keys(onto, from, &FORK_SYNCED_LEGACY_KEYS);
    }
    let Some(repos) = onto.get_mut("repositories").and_then(Value::as_array_mut) else {
        return;
    };
    for repo in repos {
        let Some(src) = same_repo(from_repos, repo) else {
            continue;
        };
        if let Some(repo) = repo.as_object_mut() {
            copy_keys(repo, src, &FORK_SYNCED_REPO_KEYS);
        }
    }
}

fn stores_legacy_filters(own: &Value) -> bool {
    FORK_SYNCED_LEGACY_KEYS
        .iter()
        .any(|k| own.get(*k).is_some_and(|v| !v.is_null()))
}

/// The fork's `git_sync` as its settings.yaml states it.
fn fork_git_sync_document(own: Option<Value>, parent: Option<Value>) -> Option<Value> {
    let Some(mut doc) = parent else {
        return own;
    };
    if let Some(own) = &own {
        copy_synced_keys(&mut doc, own, stores_legacy_filters(own));
    }
    Some(doc)
}

/// What a fork applies of the `git_sync` its settings.yaml states. Only a dev workspace may hold
/// a promotion repo, and one detached from dev still stores its own: it is dropped so the write
/// passes the save's check.
fn fork_git_sync_to_apply(stated: &Value, own: Option<&Value>, is_dev: bool) -> Result<Value> {
    if stated.get("repositories").is_some_and(|r| !r.is_array()) {
        return Err(Error::BadRequest(
            "git_sync.repositories must be a list".to_string(),
        ));
    }
    let Some(mut applied) = own.cloned() else {
        return Ok(Value::Null);
    };
    let legacy = stores_legacy_filters(&applied);
    copy_synced_keys(&mut applied, stated, legacy);
    if !is_dev {
        if let Some(repos) = applied
            .get_mut("repositories")
            .and_then(Value::as_array_mut)
        {
            repos.retain(|r| r.get("use_individual_branch") != Some(&Value::Bool(true)));
        }
    }
    Ok(applied)
}

fn plan_document(
    current: Map<String, Value>,
    mut desired: Map<String, Value>,
    replace: bool,
    mc: &windmill_common::variables::WorkspaceCrypt,
    names: &DatatableNames,
) -> Result<Plan> {
    let mut warnings = vec![];
    let unknown: Vec<&str> = desired
        .keys()
        .map(String::as_str)
        .filter(|k| !KNOWN_KEYS.contains(k))
        .collect();
    if !unknown.is_empty() {
        warnings.push(format!("Ignored unknown settings: {}", unknown.join(", ")));
    }
    for key in ["auto_invite", "error_handler", "success_handler"] {
        if desired
            .get(key)
            .is_some_and(|v| !v.is_null() && !v.is_object())
        {
            return Err(Error::BadRequest(format!(
                "{key} must be an object or null"
            )));
        }
    }
    // The bodies built below coerce a mistyped value to `false` or to a delete, so a wrong type
    // must be refused here rather than reach an endpoint as a destructive write.
    let typed = |key: &str, v: Option<&Value>, ok: fn(&Value) -> bool, ty: &str| match v {
        Some(v) if !v.is_null() && !ok(v) => {
            Err(Error::BadRequest(format!("{key} must be {ty} or null")))
        }
        _ => Ok(()),
    };
    for key in FALSE_WHEN_UNSET {
        typed(key, desired.get(*key), Value::is_boolean, "a boolean")?;
    }
    // The rest are refused by their endpoint anyway, but only after the writes before them
    // went through, and after a dry run reported them as changes.
    for key in [
        "slack_oauth_client_id",
        "slack_oauth_client_secret",
        "webhook",
        "color",
        "default_app",
        "slack_command_script",
        "teams_command_script",
        "guest_jwt_public_key",
        "guest_jwt_jwks_url",
    ] {
        typed(key, desired.get(key), Value::is_string, "a string")?;
    }
    typed(
        "public_app_execution_limit_per_minute",
        desired.get("public_app_execution_limit_per_minute"),
        |v| v.as_i64().is_some_and(|n| i32::try_from(n).is_ok()),
        "a 32-bit integer",
    )?;
    for (parent, key, ok, ty) in [
        (
            "auto_invite",
            "enabled",
            Value::is_boolean as fn(&Value) -> bool,
            "a boolean",
        ),
        ("auto_invite", "operator", Value::is_boolean, "a boolean"),
        ("auto_invite", "mode", Value::is_string, "a string"),
        (
            "error_handler",
            "muted_on_cancel",
            Value::is_boolean,
            "a boolean",
        ),
        (
            "error_handler",
            "muted_on_user_path",
            Value::is_boolean,
            "a boolean",
        ),
    ] {
        let v = desired.get(parent).and_then(|p| p.get(key));
        typed(&format!("{parent}.{key}"), v, ok, ty)?;
    }
    let mut datatable_errors = vec![];
    if let Some(v) = desired.get("datatable_permissions") {
        let v = normalize_datatable_permissions(v, &mut datatable_errors)?;
        desired.insert("datatable_permissions".to_string(), v);
    }
    if let Some(v) = desired.get("protection_rules") {
        let v = normalize_protection_rules(v, &mut warnings)?;
        desired.insert("protection_rules".to_string(), v);
    }
    // The endpoint stores an omitted mute flag as `false`.
    if let Some(handler) = desired
        .get_mut("error_handler")
        .and_then(Value::as_object_mut)
    {
        for key in ["muted_on_cancel", "muted_on_user_path"] {
            if is_null(handler.get(key)) {
                handler.insert(key.to_string(), Value::Bool(false));
            }
        }
    }
    for key in SECRET_KEYS {
        if let Some(v) = desired.remove(*key) {
            if let Some(v) = resolve_secret(key, v, mc, &mut warnings) {
                desired.insert(key.to_string(), v);
            } else if let Some(cur) = current.get(*key) {
                desired.insert(key.to_string(), cur.clone());
            }
        }
    }

    let stated: Vec<String> = desired.keys().cloned().collect();
    let mut p = Plan { current, desired, replace, changes: vec![], writes: vec![], warnings };

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
            let mut after = if auto_invite.is_null() {
                Value::Null
            } else {
                desired_domain.clone()
            };
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

    // Stated as a whole: a data table the map omits is not under roles, a rule it omits is
    // deleted.
    if let Some(desired) = p.managed("datatable_permissions") {
        plan_datatable_permissions(&mut p, desired, names, &mut datatable_errors);
    }
    datatable_permissions_errors(datatable_errors)?;
    if let Some(desired) = p.managed("protection_rules") {
        plan_protection_rules(&mut p, desired);
    }

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
        // At most one of the two is set, so stating one clears the other.
        let other = |stated: &Option<Value>, key: &str| {
            if stated.as_ref().is_some_and(|v| !v.is_null()) {
                Value::Null
            } else {
                p.current.get(key).cloned().unwrap_or(Value::Null)
            }
        };
        let (public_key, jwks_url) = (
            public_key
                .clone()
                .unwrap_or_else(|| other(&jwks_url, "guest_jwt_public_key")),
            jwks_url
                .clone()
                .unwrap_or_else(|| other(&public_key, "guest_jwt_jwks_url")),
        );
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

    // As `wmill sync push` always did: a document whose stated settings all match changes
    // nothing, so a settings.yaml pulled before a setting was turned on in the UI does not
    // clear it just for omitting it.
    let changes_stated = p.changes.iter().any(|c| {
        stated
            .iter()
            .any(|k| c.key == *k || c.key.starts_with(&format!("{k}.")))
    });
    if replace && !changes_stated {
        p.changes.clear();
        p.writes.clear();
    }

    // Refused here rather than by the endpoint, so a dry run already says so.
    let unavailable: Vec<String> = p
        .writes
        .iter()
        .filter(|w| !available_in_this_edition(&w.target))
        .flat_map(|w| w.keys.clone())
        .collect();
    if !unavailable.is_empty() {
        return Err(Error::BadRequest(format!(
            "Only available on Windmill Enterprise Edition: {}",
            unavailable.join(", ")
        )));
    }

    Ok(p)
}

fn object_of(v: Option<&Value>) -> Map<String, Value> {
    v.and_then(Value::as_object).cloned().unwrap_or_default()
}

fn plan_datatable_permissions(
    p: &mut Plan,
    desired: Value,
    names: &DatatableNames,
    errors: &mut Vec<String>,
) {
    let desired = object_of(Some(&desired));
    let current = object_of(p.current.get("datatable_permissions"));
    // The data tables as they will be once the `datatable` write before this one has gone through.
    let entries = object_of(
        p.legacy("datatable")
            .as_ref()
            .or(p.current.get("datatable"))
            .and_then(|d| d.get("datatables")),
    );
    let tables: std::collections::BTreeSet<&String> =
        desired.keys().chain(current.keys()).collect();
    for name in tables {
        let (before, after) = (current.get(name), desired.get(name));
        if same(before, after) {
            continue;
        }
        let entry = entries.get(name);
        // A removed data table takes its roles with it.
        if after.is_none() && entry.is_none() {
            continue;
        }
        let body = match after {
            None => json!({ "datatable": name, "permissioned": false }),
            Some(after) => match datatable_permissions_write(name, entry, before, after, names) {
                Ok(body) => body,
                Err(e) => {
                    errors.extend(e);
                    continue;
                }
            },
        };
        p.record_change(
            &format!("datatable_permissions.{name}"),
            before.cloned().unwrap_or(Value::Null),
            after.cloned().unwrap_or(Value::Null),
        );
        p.write(Target::Edit(SettingsEdit::DatatablePermissions), body);
    }
}

/// The endpoint body that puts `name` under the roles of `after`, or everything wrong with it.
fn datatable_permissions_write(
    name: &str,
    entry: Option<&Value>,
    before: Option<&Value>,
    after: &Value,
    names: &DatatableNames,
) -> std::result::Result<Value, Vec<String>> {
    let Some(cluster) = entry.and_then(role_cluster_of) else {
        return Err(vec![format!(
            "Data table '{name}' cannot be put under roles: only a data table this workspace \
             governs, on a cluster Windmill manages, can"
        )]);
    };
    let Some(catalog) = names.of(cluster) else {
        return Err(vec![
            "Data table roles are unavailable on this instance".to_string()
        ]);
    };
    let ids: BTreeMap<&str, &str> = catalog
        .iter()
        .map(|(id, name)| (name.as_str(), id.as_str()))
        .collect();
    let mut errors = vec![];
    let mut id_of = |role: &str| -> String {
        if role == ADMIN_DATATABLE_ROLE {
            return role.to_string();
        }
        ids.get(role).map(|id| id.to_string()).unwrap_or_else(|| {
            errors.push(format!(
                "Data table '{name}': '{role}' is not a data table role of the {} cluster",
                cluster.as_str()
            ));
            String::new()
        })
    };
    let default_role = match after.get("default_role").and_then(Value::as_str) {
        None => NO_DEFAULT_DATATABLE_ROLE.to_string(),
        Some(role) => id_of(role),
    };
    let roles: Vec<Value> = object_of(after.get("roles"))
        .iter()
        .map(|(role, tenants)| json!({ "id": id_of(role), "tenants": tenants }))
        .collect();
    // Only a tenant this change grants: one already stored that has since stopped naming anybody
    // must not block every later push.
    let granted = |v: Option<&Value>| -> std::collections::BTreeSet<String> {
        object_of(v.and_then(|v| v.get("roles")))
            .values()
            .filter_map(Value::as_array)
            .flatten()
            .filter_map(|t| Some(t.as_str()?.to_string()))
            .collect()
    };
    let already = granted(before);
    for tenant in granted(Some(after)).difference(&already) {
        if names.unknown_tenants.contains(tenant) {
            let kind = match tenant.split_once('/') {
                Some(("u", _)) => "user",
                Some(("g", _)) => "group",
                _ => "folder",
            };
            errors.push(format!(
                "Data table '{name}': tenant '{tenant}' names no {kind} of this workspace"
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(json!({
        "datatable": name,
        "permissioned": true,
        "default_role": default_role,
        "roles": roles,
    }))
}

fn plan_protection_rules(p: &mut Plan, desired: Value) {
    let desired = object_of(Some(&desired));
    let current = object_of(p.current.get("protection_rules"));
    let names: std::collections::BTreeSet<&String> = desired.keys().chain(current.keys()).collect();
    for name in names {
        let (before, after) = (current.get(name), desired.get(name));
        if same(before, after) {
            continue;
        }
        p.record_change(
            &format!("protection_rules.{name}"),
            before.cloned().unwrap_or(Value::Null),
            after.cloned().unwrap_or(Value::Null),
        );
        let rule = |r: &Value, name_key: &str| {
            json!({
                name_key: name,
                "rules": r.get("rules"),
                "bypass_groups": r.get("bypass_groups"),
                "bypass_users": r.get("bypass_users"),
            })
        };
        match (before, after) {
            (None, Some(after)) => p.write(
                Target::Edit(SettingsEdit::CreateProtectionRule),
                rule(after, "name"),
            ),
            (Some(_), Some(after)) => p.write(
                Target::Edit(SettingsEdit::UpdateProtectionRule),
                rule(after, "rule_name"),
            ),
            _ => p.write(
                Target::Edit(SettingsEdit::DeleteProtectionRule),
                json!({ "name": name }),
            ),
        }
    }
}

fn available_in_this_edition(target: &Target) -> bool {
    match target {
        Target::MuteCriticalAlerts
        | Target::Edit(SettingsEdit::DefaultApp)
        | Target::Edit(SettingsEdit::DeployUi) => cfg!(feature = "enterprise"),
        Target::Edit(SettingsEdit::DatatablePermissions) => {
            cfg!(all(feature = "private", feature = "enterprise"))
        }
        Target::TeamsCommand | Target::Edit(SettingsEdit::InstanceGroups) => {
            cfg!(feature = "private")
        }
        _ => true,
    }
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
            let _ = crate::workspaces::edit_copilot_config(a, d, p, Json(body)).await?;
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
            crate::teams_ee::edit_teams_command(a, d, p, Json(body)).await?;
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
    let mut result = ApplySettingsDocumentResult {
        changes: p.changes,
        warnings: p.warnings,
        ..Default::default()
    };
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
            Error::Generic(
                e.status_code(),
                format!(
                    "{e} (settings already applied before this error: {})",
                    written.join(", ")
                ),
            )
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
                "The removed data tables governed data tables in other workspaces, which no longer resolve: {refs}. A superadmin can point them somewhere else."
            ));
        }
        written.extend(keys);
    }
    result.applied = true;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windmill_common::variables::WorkspaceCrypt;

    fn doc(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn keys(p: &Plan) -> Vec<&str> {
        p.changes.iter().map(|c| c.key.as_str()).collect()
    }

    fn current() -> Map<String, Value> {
        doc(json!({
            "name": "ws",
            "webhook": "https://a",
            "large_file_storage": { "type": "S3Storage", "s3_resource_path": "f/s3" },
            "slack_oauth_client_id": "cid",
            "slack_oauth_client_secret": "sekret",
            "guest_jwt_public_key": "PEM",
            "guest_jwt_jwks_url": null,
            "public_app_execution_limit_per_minute": 5,
        }))
    }

    #[test]
    fn replace_clears_an_omitted_setting_only_when_a_stated_one_changes() {
        let mc = WorkspaceCrypt::new("k");
        let unchanged = doc(json!({ "name": "other", "webhook": "https://a" }));
        let p = plan_document(current(), unchanged, true, &mc, &DatatableNames::default()).unwrap();
        assert!(p.writes.is_empty(), "{:?}", keys(&p));

        let changed = doc(json!({ "webhook": "https://b" }));
        let p = plan_document(current(), changed, true, &mc, &DatatableNames::default()).unwrap();
        assert_eq!(
            keys(&p),
            [
                "webhook",
                "large_file_storage",
                "slack_oauth_client_id",
                "slack_oauth_client_secret"
            ]
        );
        assert!(!keys(&p).contains(&"public_app_execution_limit_per_minute"));
    }

    #[test]
    fn secrets_keep_their_value_for_the_placeholder_and_a_foreign_key() {
        let mc = WorkspaceCrypt::new("k");
        let ours = format!(
            "{ENCRYPTED_PREFIX}{}",
            mc.encrypt_str_deterministic("sekret")
        );
        let theirs = format!(
            "{ENCRYPTED_PREFIX}{}",
            WorkspaceCrypt::new("other").encrypt_str_deterministic("x")
        );
        for secret in [SENSITIVE_UNCHANGED.to_string(), ours, theirs.clone()] {
            let p = plan_document(
                current(),
                doc(json!({ "slack_oauth_client_secret": secret })),
                false,
                &mc,
                &DatatableNames::default(),
            )
            .unwrap();
            assert!(p.changes.is_empty(), "{secret}: {:?}", keys(&p));
            assert_eq!(p.warnings.len(), usize::from(secret == theirs));
        }
    }

    #[test]
    fn unset_switches_compare_as_false() {
        let mc = WorkspaceCrypt::new("k");
        let d = doc(json!({ "mute_critical_alerts": false, "guest_access_enabled": null }));
        assert!(
            plan_document(current(), d, false, &mc, &DatatableNames::default())
                .unwrap()
                .changes
                .is_empty()
        );
    }

    #[test]
    fn stating_one_guest_key_clears_the_other() {
        let mc = WorkspaceCrypt::new("k");
        let d = doc(json!({ "guest_jwt_jwks_url": "https://jwks" }));
        let p = plan_document(current(), d, false, &mc, &DatatableNames::default()).unwrap();
        assert_eq!(p.writes.len(), 1);
        assert_eq!(
            p.writes[0].body,
            json!({ "public_key": null, "jwks_url": "https://jwks" })
        );
    }

    #[test]
    fn mistyped_values_are_refused_rather_than_coerced() {
        let mc = WorkspaceCrypt::new("k");
        for d in [
            json!({ "guest_access_enabled": "true" }),
            json!({ "add_admins_and_developers_to_forks": 1 }),
            json!({ "slack_oauth_client_id": 123 }),
            json!({ "error_handler": { "path": "f/x", "muted_on_cancel": "yes" } }),
            json!({ "auto_invite": { "enabled": "true" } }),
            json!({ "webhook": 123 }),
            json!({ "public_app_execution_limit_per_minute": "5" }),
            json!({ "public_app_execution_limit_per_minute": 1.5 }),
            json!({ "public_app_execution_limit_per_minute": 2147483648i64 }),
        ] {
            assert!(
                plan_document(
                    current(),
                    doc(d.clone()),
                    false,
                    &mc,
                    &DatatableNames::default()
                )
                .is_err(),
                "{d}"
            );
        }
    }

    #[test]
    fn a_null_handler_argument_is_a_change() {
        let mc = WorkspaceCrypt::new("k");
        let mut cur = current();
        cur.insert(
            "error_handler".into(),
            json!({ "path": "f/x", "extra_args": {}, "muted_on_cancel": false, "muted_on_user_path": false }),
        );
        let d = doc(json!({ "error_handler": { "path": "f/x", "extra_args": { "input": null } } }));
        let p = plan_document(cur.clone(), d, false, &mc, &DatatableNames::default()).unwrap();
        assert_eq!(keys(&p), ["error_handler"]);

        let d = doc(json!({ "error_handler": { "path": "f/x", "extra_args": {} } }));
        let p = plan_document(cur, d, false, &mc, &DatatableNames::default()).unwrap();
        assert!(p.changes.is_empty(), "{:?}", keys(&p));
    }

    #[cfg(all(feature = "private", feature = "enterprise"))]
    #[test]
    fn datatable_roles_are_named_in_the_document_and_written_by_id() {
        let mc = WorkspaceCrypt::new("k");
        let roles = DatatableNames {
            instance: Some([("r1".to_string(), "readers".to_string())].into()),
            ..Default::default()
        };
        let mut cur = current();
        cur.insert(
            "datatable".into(),
            json!({ "datatables": { "main": { "database": { "resource_type": "instance", "resource_path": "main" } } } }),
        );
        cur.insert(
            "datatable_permissions".into(),
            json!({ "main": { "default_role": "admin", "roles": { "admin": [], "readers": ["g/x"] } } }),
        );

        let d = doc(
            json!({ "datatable_permissions": { "main": { "default_role": "readers", "roles": { "readers": ["g/x", "u/bob"] } } } }),
        );
        let p = plan_document(cur.clone(), d, false, &mc, &roles).unwrap();
        assert_eq!(keys(&p), ["datatable_permissions.main"]);
        assert_eq!(
            p.writes[0].body,
            json!({ "datatable": "main", "permissioned": true, "default_role": "r1",
                    "roles": [{ "id": "r1", "tenants": ["g/x", "u/bob"] }, { "id": "admin", "tenants": [] }] })
        );

        let d = doc(json!({ "datatable_permissions": {} }));
        let p = plan_document(cur.clone(), d, false, &mc, &roles).unwrap();
        assert_eq!(
            p.writes[0].body,
            json!({ "datatable": "main", "permissioned": false })
        );

        let d = doc(json!({ "datatable_permissions": { "main": { "roles": { "writers": [] } } } }));
        assert!(plan_document(cur.clone(), d, false, &mc, &roles).is_err());

        let d = doc(json!({ "datatable_permissions": { "main": { "default_role": "readers" } } }));
        assert!(plan_document(cur.clone(), d, false, &mc, &roles).is_err());

        // A tenant that names nobody is refused when granted, not when already stored, and every
        // data table's errors come back together.
        let strict = DatatableNames {
            instance: roles.instance.clone(),
            unknown_tenants: ["g/x".to_string(), "u/bob".to_string()].into(),
            ..Default::default()
        };
        let d = doc(
            json!({ "datatable_permissions": { "main": { "default_role": "readers", "roles": { "readers": ["g/x"] } } } }),
        );
        assert_eq!(
            keys(&plan_document(cur.clone(), d, false, &mc, &strict).unwrap()),
            ["datatable_permissions.main"]
        );
        let d = doc(json!({ "datatable_permissions": {
            "main": { "roles": { "readers": ["g/x", "u/bob"], "writers": [] } },
            "other": { "roles": { "readers": ["x/y"] } },
        } }));
        let e = plan_document(cur.clone(), d, false, &mc, &strict)
            .err()
            .unwrap()
            .to_string();
        for part in [
            "tenant 'u/bob' names no user",
            "'writers' is not a data table role",
            "Invalid tenant 'x/y'",
        ] {
            assert!(e.contains(part), "{e}");
        }

        // Under replace an omitted `datatable` is cleared, taking its tables' roles with it.
        let d = doc(json!({ "datatable_permissions": {} }));
        let p = plan_document(cur.clone(), d, true, &mc, &roles).unwrap();
        assert!(!p
            .writes
            .iter()
            .any(|w| matches!(w.target, Target::Edit(SettingsEdit::DatatablePermissions))));
        let d = doc(json!({ "datatable_permissions": { "main": {} } }));
        assert!(plan_document(cur, d, true, &mc, &roles).is_err());
    }

    #[test]
    fn protection_rules_are_diffed_by_name() {
        let mc = WorkspaceCrypt::new("k");
        let mut cur = current();
        cur.insert(
            "protection_rules".into(),
            json!({ "old": { "rules": ["DisableWorkspaceForking"], "bypass_groups": [], "bypass_users": [] } }),
        );
        let d = doc(json!({ "protection_rules": {
            "new": { "rules": ["RestrictPublicRunSharing", "DisableDirectDeployment"] },
            "dev_workspace_lock": { "rules": [] },
        } }));
        let p = plan_document(cur, d, false, &mc, &DatatableNames::default()).unwrap();
        assert_eq!(keys(&p), ["protection_rules.new", "protection_rules.old"]);
        assert_eq!(
            p.writes[0].body,
            json!({ "name": "new", "rules": ["DisableDirectDeployment", "RestrictPublicRunSharing"],
                    "bypass_groups": [], "bypass_users": [] })
        );
        assert!(matches!(
            p.writes[1].target,
            Target::Edit(SettingsEdit::DeleteProtectionRule)
        ));
        assert_eq!(p.warnings.len(), 1);
    }

    #[test]
    fn a_fork_applies_only_the_synced_keys_of_the_repos_it_carries() {
        let parent = json!({ "repositories": [
            { "git_repo_resource_path": "f/r", "settings": { "include_path": ["f/**"] },
              "auto_pull": { "enabled": true, "sync_forks": true }, "fork_open_prs": true },
            { "git_repo_resource_path": "f/promo", "use_individual_branch": true },
        ] });
        // A dev workspace's promotion repo is the repo the parent syncs normally.
        let fork = json!({ "repositories": [
            { "git_repo_resource_path": "f/own" },
            { "git_repo_resource_path": "f/r", "settings": { "include_path": ["u/**"] },
              "use_individual_branch": true, "auto_pull": { "enabled": false } },
        ] });
        assert_eq!(
            fork_git_sync_to_apply(&parent, Some(&fork), true).unwrap(),
            json!({ "repositories": [
                { "git_repo_resource_path": "f/own" },
                { "git_repo_resource_path": "f/r", "settings": { "include_path": ["f/**"] },
                  "use_individual_branch": true, "auto_pull": { "enabled": false } },
            ] })
        );

        // The fork's document is the parent's block with the fork's synced keys, and applying it
        // back to the fork is a no-op.
        let shown = fork_git_sync_document(Some(fork.clone()), Some(parent.clone())).unwrap();
        assert_eq!(
            shown,
            json!({ "repositories": [
                { "git_repo_resource_path": "f/r", "settings": { "include_path": ["u/**"] },
                  "auto_pull": { "enabled": true, "sync_forks": true }, "fork_open_prs": true },
                { "git_repo_resource_path": "f/promo", "use_individual_branch": true },
            ] })
        );
        assert_eq!(
            fork_git_sync_to_apply(&shown, Some(&fork), true).unwrap(),
            fork
        );

        // Detached from dev, the fork may no longer hold its promotion repo.
        assert_eq!(
            fork_git_sync_to_apply(&parent, Some(&fork), false).unwrap(),
            json!({ "repositories": [{ "git_repo_resource_path": "f/own" }] })
        );

        // The workspace-level filters follow the file only while the fork stores them.
        let legacy = json!({ "include_path": ["u/**"], "repositories": [] });
        let stated = json!({ "include_path": ["f/**"], "include_type": ["script"],
                             "repositories": [] });
        assert_eq!(
            fork_git_sync_to_apply(&stated, Some(&legacy), false).unwrap(),
            stated
        );
        let migrated = json!({ "repositories": [] });
        assert_eq!(
            fork_git_sync_to_apply(&stated, Some(&migrated), false).unwrap(),
            migrated
        );
        let shown = fork_git_sync_document(Some(legacy.clone()), Some(stated.clone())).unwrap();
        assert_eq!(
            fork_git_sync_to_apply(&shown, Some(&legacy), false).unwrap(),
            legacy
        );

        assert!(
            fork_git_sync_to_apply(&json!({ "repositories": null }), Some(&fork), true).is_err()
        );
        assert_eq!(
            fork_git_sync_to_apply(&parent, None, true).unwrap(),
            Value::Null
        );
    }
}
