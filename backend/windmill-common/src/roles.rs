//! Workspace roles: what a non-admin member may author, manage and see.
//!
//! `developer` and `operator` are the default roles. An EE instance defines its own list in the
//! [`WORKSPACE_ROLES_SETTING`] global setting (at least one role), and a workspace may override any
//! role's capabilities, or on EE define roles of its own, in `workspace_settings.role_overrides`.
//! Workspace admin is not a role: it is the `is_admin` flag on top of whichever role a member has,
//! and holds every capability. See `docs/workspace-roles.md`.
//!
//! A member's role is `usr.workspace_role`, or, while that is NULL, `operator` / `developer` from the
//! legacy `usr.operator` flag. `ApiAuthed::is_operator` means "this member's role is restricted",
//! computed at authentication from the role, never from `usr.operator` alone: every gate fast-paths
//! on it, so a stale flag would let a restricted member through.

use std::{collections::HashMap, sync::Arc};

use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};

use crate::{
    db::Authable,
    error::{Error, Result},
    global_settings::WORKSPACE_ROLES_SETTING,
    DB,
};

pub const DEVELOPER_ROLE: &str = "developer";
pub const OPERATOR_ROLE: &str = "operator";

/// How much of a kind of runnable a role may author.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AuthorLevel {
    #[default]
    None,
    /// Assemble already-deployed runnables, with no code of one's own. A composed flow is checked
    /// in full server-side; a composed full-code app is forced into sandbox isolation. See
    /// `docs/operator-builder-rights.md`.
    Compose,
    Full,
}

/// Pages a role sees. Not enforced: hiding a page never stopped an API call, and the enforced
/// rights are the other fields of [`RoleCapabilities`].
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleVisibility {
    pub runs: bool,
    pub schedules: bool,
    pub resources: bool,
    pub variables: bool,
    pub assets: bool,
    pub triggers: bool,
    pub audit_logs: bool,
    pub groups: bool,
    pub folders: bool,
    pub workers: bool,
}

impl RoleVisibility {
    pub const ALL: Self = Self {
        runs: true,
        schedules: true,
        resources: true,
        variables: true,
        assets: true,
        triggers: true,
        audit_logs: true,
        groups: true,
        folders: true,
        workers: true,
    };
    pub const NONE: Self = Self {
        runs: false,
        schedules: false,
        resources: false,
        variables: false,
        assets: false,
        triggers: false,
        audit_logs: false,
        groups: false,
        folders: false,
        workers: false,
    };
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleCapabilities {
    /// Author scripts and run code of one's own on a worker: script previews, dependency jobs,
    /// dynamic selects, resolving a job's code, data table migrations. `Full` flows or apps
    /// require it, since both carry inline code.
    pub code: bool,
    pub flows: AuthorLevel,
    pub apps: AuthorLevel,
    pub manage_schedules: bool,
    pub manage_triggers: bool,
    /// Create a fork of the workspace. The forker is a developer in the fork whatever this role
    /// holds, so granting it to a restricted role hands that member a full-rights copy.
    pub fork: bool,
    pub visibility: RoleVisibility,
}

impl RoleCapabilities {
    pub const FULL: Self = Self {
        code: true,
        flows: AuthorLevel::Full,
        apps: AuthorLevel::Full,
        manage_schedules: true,
        manage_triggers: true,
        fork: true,
        visibility: RoleVisibility::ALL,
    };

    /// The `operator` role as it stood before roles were configurable, and the base any other
    /// role's unset keys fall back to.
    pub const OPERATOR: Self = Self {
        code: false,
        flows: AuthorLevel::None,
        apps: AuthorLevel::None,
        manage_schedules: true,
        manage_triggers: true,
        fork: false,
        visibility: RoleVisibility::ALL,
    };

    /// For a restricted caller whose role cannot be resolved: a guest, a removed member's job, a
    /// role name no longer defined.
    pub const NONE: Self = Self {
        code: false,
        flows: AuthorLevel::None,
        apps: AuthorLevel::None,
        manage_schedules: false,
        manage_triggers: false,
        fork: false,
        visibility: RoleVisibility::NONE,
    };

    /// What a role named `name` holds before any configuration applies.
    pub fn base_for(name: &str) -> Self {
        if name == DEVELOPER_ROLE {
            Self::FULL
        } else {
            Self::OPERATOR
        }
    }

    /// Whether a member of this role is gated at all. Every gate skips an unrestricted caller
    /// without reading its role, which is what `ApiAuthed::is_operator` carries. Visibility is
    /// left out: it only hides pages, and must not turn a full author into an operator for the
    /// raw `is_operator` checks, job permissions and credential caches keyed on it.
    pub fn is_restricted(&self) -> bool {
        Self { visibility: RoleVisibility::ALL, ..*self } != Self::FULL
    }

    /// Whether the role authors deployable artifacts and so costs a full seat rather than half.
    /// Composing counts: there is no half-author.
    pub fn is_author(&self) -> bool {
        self.code || self.flows != AuthorLevel::None || self.apps != AuthorLevel::None
    }

    pub fn has_manage(&self, kind: crate::workspaces::ManageKind) -> bool {
        match kind {
            crate::workspaces::ManageKind::Schedules => self.manage_schedules,
            crate::workspaces::ManageKind::Triggers => self.manage_triggers,
        }
    }

    pub fn level(&self, kind: crate::workspaces::BuilderKind) -> AuthorLevel {
        match kind {
            crate::workspaces::BuilderKind::Flows => self.flows,
            crate::workspaces::BuilderKind::Apps => self.apps,
        }
    }

    fn apply(&mut self, patch: &RoleCapabilitiesPatch) {
        if let Some(v) = patch.code {
            self.code = v;
        }
        if let Some(v) = patch.flows {
            self.flows = v;
        }
        if let Some(v) = patch.apps {
            self.apps = v;
        }
        if let Some(v) = patch.manage_schedules {
            self.manage_schedules = v;
        }
        if let Some(v) = patch.manage_triggers {
            self.manage_triggers = v;
        }
        if let Some(v) = patch.fork {
            self.fork = v;
        }
        if let Some(v) = &patch.visibility {
            v.apply(&mut self.visibility);
        }
    }

    /// A `Full` flow or app carries inline code, so granting one without `code` would be a way
    /// around refusing `code`.
    pub fn validate(&self, role: &str) -> Result<()> {
        if !self.code && (self.flows == AuthorLevel::Full || self.apps == AuthorLevel::Full) {
            return Err(Error::BadRequest(format!(
                "role `{role}`: full flow or app authoring requires the code capability"
            )));
        }
        Ok(())
    }
}

/// A partial [`RoleCapabilities`]: an unset key falls through to the layer below (workspace
/// override, then instance definition, then [`RoleCapabilities::base_for`]). Stored settings are
/// patches rather than full values so that a key added later reads as "never configured".
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct RoleCapabilitiesPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flows: Option<AuthorLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apps: Option<AuthorLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manage_schedules: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manage_triggers: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<RoleVisibilityPatch>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct RoleVisibilityPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runs: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedules: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggers: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_logs: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groups: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folders: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workers: Option<bool>,
}

impl RoleVisibilityPatch {
    fn apply(&self, v: &mut RoleVisibility) {
        macro_rules! set {
            ($($f:ident),*) => { $( if let Some(x) = self.$f { v.$f = x; } )* };
        }
        set!(
            runs, schedules, resources, variables, assets, triggers, audit_logs, groups, folders,
            workers
        );
    }
}

/// The smallest patch that turns `base` into `target`: only the keys that differ.
pub fn diff_capabilities(
    base: &RoleCapabilities,
    target: &RoleCapabilities,
) -> RoleCapabilitiesPatch {
    fn d<T: PartialEq + Copy>(b: T, t: T) -> Option<T> {
        (b != t).then_some(t)
    }
    let (b, t) = (&base.visibility, &target.visibility);
    let visibility = RoleVisibilityPatch {
        runs: d(b.runs, t.runs),
        schedules: d(b.schedules, t.schedules),
        resources: d(b.resources, t.resources),
        variables: d(b.variables, t.variables),
        assets: d(b.assets, t.assets),
        triggers: d(b.triggers, t.triggers),
        audit_logs: d(b.audit_logs, t.audit_logs),
        groups: d(b.groups, t.groups),
        folders: d(b.folders, t.folders),
        workers: d(b.workers, t.workers),
    };
    RoleCapabilitiesPatch {
        code: d(base.code, target.code),
        flows: d(base.flows, target.flows),
        apps: d(base.apps, target.apps),
        manage_schedules: d(base.manage_schedules, target.manage_schedules),
        manage_triggers: d(base.manage_triggers, target.manage_triggers),
        fork: d(base.fork, target.fork),
        visibility: (visibility != RoleVisibilityPatch::default()).then_some(visibility),
    }
}

/// One entry of the instance's [`WORKSPACE_ROLES_SETTING`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RoleDefinition {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub capabilities: RoleCapabilitiesPatch,
}

/// One entry of `workspace_settings.role_overrides`, keyed by role name. Naming a role the
/// instance does not define adds a workspace-only role.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct RoleOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub capabilities: RoleCapabilitiesPatch,
}

pub type RoleOverrides = HashMap<String, RoleOverride>;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoleSource {
    Instance,
    Workspace,
}

/// A role as it applies in one workspace.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EffectiveRole {
    pub name: String,
    pub label: String,
    pub capabilities: RoleCapabilities,
    pub source: RoleSource,
    /// Whether the workspace overrides this role.
    pub overridden: bool,
}

/// Whether stored instance roles and workspace-only roles apply: on any EE build. A CE build has
/// `developer` and `operator`, which a workspace may still override.
fn custom_roles_built() -> bool {
    cfg!(feature = "enterprise")
}

/// Whether instance roles and workspace-only roles may be changed: an EE build whose license
/// verified. A lapsed license freezes them instead of dropping them, since falling back to the
/// defaults would hand a restricted `developer` everything. The `cfg` is load-bearing: CE's
/// `LICENSE_KEY_VALID` is always `true`.
pub fn custom_roles_enabled() -> bool {
    #[cfg(feature = "enterprise")]
    {
        crate::ee_oss::LICENSE_KEY_VALID.load(std::sync::atomic::Ordering::Relaxed)
    }
    #[cfg(not(feature = "enterprise"))]
    {
        false
    }
}

pub fn default_instance_roles() -> Vec<RoleDefinition> {
    vec![
        RoleDefinition {
            name: DEVELOPER_ROLE.to_string(),
            label: Some("Developer".to_string()),
            capabilities: RoleCapabilitiesPatch::default(),
        },
        RoleDefinition {
            name: OPERATOR_ROLE.to_string(),
            label: Some("Operator".to_string()),
            capabilities: RoleCapabilitiesPatch::default(),
        },
    ]
}

/// Role names appear in `u/`-free contexts only (settings, group mappings, the `usr` row), but are
/// also shown as-is and compared verbatim, so they are kept to a slug.
pub fn validate_role_name(name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && name.len() <= 50
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        && name != "admin";
    if ok {
        Ok(())
    } else {
        Err(Error::BadRequest(format!(
            "invalid role name `{name}`: use 1-50 lowercase letters, digits, `_` or `-`, and not `admin`"
        )))
    }
}

/// Parses and validates a [`WORKSPACE_ROLES_SETTING`] value: at least one role, unique valid
/// names, capabilities that pass [`RoleCapabilities::validate`].
pub fn parse_instance_roles(value: &serde_json::Value) -> Result<Vec<RoleDefinition>> {
    let roles: Vec<RoleDefinition> = serde_json::from_value(value.clone())
        .map_err(|e| Error::BadRequest(format!("invalid workspace roles: {e}")))?;
    if roles.is_empty() {
        return Err(Error::BadRequest(
            "at least one workspace role must be defined".to_string(),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for r in &roles {
        validate_role_name(&r.name)?;
        if !seen.insert(r.name.as_str()) {
            return Err(Error::BadRequest(format!("duplicate role `{}`", r.name)));
        }
        let mut caps = RoleCapabilities::base_for(&r.name);
        caps.apply(&r.capabilities);
        caps.validate(&r.name)?;
    }
    Ok(roles)
}

/// The workspace's roles: the instance's, in their order and with any override applied, then the
/// workspace-only ones by name. Order is precedence where one is needed (instance group mapping).
pub fn effective_roles(
    instance: &[RoleDefinition],
    overrides: &RoleOverrides,
) -> Result<Vec<EffectiveRole>> {
    let mut out = Vec::with_capacity(instance.len() + overrides.len());
    for def in instance {
        let mut caps = RoleCapabilities::base_for(&def.name);
        caps.apply(&def.capabilities);
        let ovr = overrides.get(&def.name);
        if let Some(o) = ovr {
            caps.apply(&o.capabilities);
        }
        out.push(EffectiveRole {
            label: ovr
                .and_then(|o| o.label.clone())
                .or_else(|| def.label.clone())
                .unwrap_or_else(|| def.name.clone()),
            name: def.name.clone(),
            capabilities: caps,
            source: RoleSource::Instance,
            overridden: ovr.is_some(),
        });
    }
    if custom_roles_built() {
        let mut extra: Vec<_> = overrides
            .iter()
            .filter(|(n, _)| !instance.iter().any(|d| &d.name == *n))
            .collect();
        extra.sort_by(|a, b| a.0.cmp(b.0));
        for (name, o) in extra {
            let mut caps = RoleCapabilities::base_for(name);
            caps.apply(&o.capabilities);
            out.push(EffectiveRole {
                name: name.clone(),
                label: o.label.clone().unwrap_or_else(|| name.clone()),
                capabilities: caps,
                source: RoleSource::Workspace,
                overridden: true,
            });
        }
    }
    Ok(out)
}

/// Validates a workspace's `role_overrides` against the instance roles.
pub fn validate_overrides(instance: &[RoleDefinition], overrides: &RoleOverrides) -> Result<()> {
    for name in overrides.keys() {
        validate_role_name(name)?;
        if !custom_roles_built() && !instance.iter().any(|d| &d.name == name) {
            return Err(Error::BadRequest(format!(
                "role `{name}` is not defined: adding workspace roles requires Windmill Enterprise Edition"
            )));
        }
    }
    for r in effective_roles(instance, overrides)? {
        r.capabilities.validate(&r.name)?;
    }
    Ok(())
}

/// Refuses, while [`custom_roles_enabled`] is false, a `role_overrides` write that adds, edits or
/// relabels a workspace-only role. Removing one, and overriding instance roles, stay allowed.
pub fn check_frozen_overrides(
    instance: &[RoleDefinition],
    previous: &RoleOverrides,
    next: &RoleOverrides,
) -> Result<()> {
    if custom_roles_enabled() {
        return Ok(());
    }
    for (name, o) in next {
        if !instance.iter().any(|d| &d.name == name) && previous.get(name) != Some(o) {
            return Err(Error::BadRequest(format!(
                "changing workspace role `{name}` requires a valid Windmill Enterprise Edition license"
            )));
        }
    }
    Ok(())
}

/// The role a member gets when nothing names one (a legacy non-operator payload, an unmapped
/// instance group, an admin's underlying role): `developer` while it is defined, else the first
/// role, since an instance may drop `developer` and `set_workspace_role` refuses unknown names.
pub fn default_role_name<'a>(names: impl IntoIterator<Item = &'a str>) -> String {
    let mut first = None;
    for name in names {
        if name == DEVELOPER_ROLE {
            return DEVELOPER_ROLE.to_string();
        }
        first.get_or_insert(name);
    }
    first.unwrap_or(DEVELOPER_ROLE).to_string()
}

/// The role a `usr` row holds: its `workspace_role`, or the legacy one its `operator` flag names.
pub fn resolve_role_name(workspace_role: Option<String>, operator: bool) -> String {
    workspace_role.unwrap_or_else(|| {
        if operator {
            OPERATOR_ROLE.to_string()
        } else {
            DEVELOPER_ROLE.to_string()
        }
    })
}

/// SQL for the role a `usr` / `workspace_invite` row aliased `t` holds; mirrors
/// [`resolve_role_name`].
pub const ROLE_NAME_SQL: &str =
    "COALESCE(t.workspace_role, CASE WHEN t.operator THEN 'operator' ELSE 'developer' END)";

lazy_static::lazy_static! {
    static ref INSTANCE_ROLES: Cache<(), (Arc<Vec<RoleDefinition>>, i64)> = Cache::new(1);
    static ref WORKSPACE_ROLES: Cache<String, (Arc<Vec<EffectiveRole>>, i64)> = Cache::new(1000);
    static ref MEMBER_ROLE: Cache<(String, String), (Option<String>, i64)> = Cache::new(10000);
}

const ROLE_CACHE_TTL_SECS: i64 = 60;

/// The instance's role list. A CE build, or an instance that never configured one, has the
/// default roles. Cached 60s, dropped on every replica when the setting changes.
pub async fn instance_roles(db: &DB) -> Result<Arc<Vec<RoleDefinition>>> {
    if let Some(roles) = cached_instance_roles() {
        return Ok(roles);
    }
    let mut conn = db.acquire().await?;
    instance_roles_on(&mut conn).await
}

fn cached_instance_roles() -> Option<Arc<Vec<RoleDefinition>>> {
    let now = chrono::Utc::now().timestamp();
    INSTANCE_ROLES
        .get(&())
        .filter(|(_, expiry)| *expiry > now)
        .map(|(r, _)| r)
}

async fn instance_roles_on(conn: &mut sqlx::PgConnection) -> Result<Arc<Vec<RoleDefinition>>> {
    if let Some(roles) = cached_instance_roles() {
        return Ok(roles);
    }
    let roles = if custom_roles_built() {
        let stored = sqlx::query_scalar!(
            "SELECT value FROM global_settings WHERE name = $1",
            WORKSPACE_ROLES_SETTING
        )
        .fetch_optional(&mut *conn)
        .await?;
        match stored {
            Some(v) => parse_instance_roles(&v).unwrap_or_else(|e| {
                tracing::error!("ignoring invalid {WORKSPACE_ROLES_SETTING} setting: {e:#}");
                default_instance_roles()
            }),
            None => default_instance_roles(),
        }
    } else {
        default_instance_roles()
    };
    let roles = Arc::new(roles);
    INSTANCE_ROLES.insert(
        (),
        (
            roles.clone(),
            chrono::Utc::now().timestamp() + ROLE_CACHE_TTL_SECS,
        ),
    );
    Ok(roles)
}

pub async fn load_role_overrides<'c, E: sqlx::PgExecutor<'c>>(
    db: E,
    workspace_id: &str,
) -> Result<RoleOverrides> {
    let v: Option<Option<serde_json::Value>> = sqlx::query_scalar!(
        "SELECT role_overrides FROM workspace_settings WHERE workspace_id = $1",
        workspace_id
    )
    .fetch_optional(db)
    .await?;
    match v.flatten() {
        Some(v) => serde_json::from_value(v).map_err(|e| {
            Error::internal_err(format!("invalid role overrides of {workspace_id}: {e:#}"))
        }),
        None => Ok(RoleOverrides::new()),
    }
}

fn cached_workspace_roles(workspace_id: &str) -> Option<Arc<Vec<EffectiveRole>>> {
    let now = chrono::Utc::now().timestamp();
    WORKSPACE_ROLES
        .get(workspace_id)
        .filter(|(_, expiry)| *expiry > now)
        .map(|(r, _)| r)
}

/// The roles of `workspace_id`. Cached 60s; `role_overrides` writes and the instance setting drop
/// the entry on every replica.
///
/// Call it before opening an RLS transaction: on a cache miss it takes a connection from the root
/// pool, and a second pooled connection held alongside a transaction self-deadlocks on a
/// one-connection pool. A caller that already holds a connection uses [`workspace_roles_on`].
pub async fn workspace_roles(db: &DB, workspace_id: &str) -> Result<Arc<Vec<EffectiveRole>>> {
    if let Some(roles) = cached_workspace_roles(workspace_id) {
        return Ok(roles);
    }
    let mut conn = db.acquire().await?;
    workspace_roles_on(&mut conn, workspace_id).await
}

pub async fn workspace_roles_on(
    conn: &mut sqlx::PgConnection,
    workspace_id: &str,
) -> Result<Arc<Vec<EffectiveRole>>> {
    if let Some(roles) = cached_workspace_roles(workspace_id) {
        return Ok(roles);
    }
    let instance = instance_roles_on(&mut *conn).await?;
    let overrides = load_role_overrides(&mut *conn, workspace_id).await?;
    let roles = Arc::new(effective_roles(&instance, &overrides)?);
    WORKSPACE_ROLES.insert(
        workspace_id.to_string(),
        (
            roles.clone(),
            chrono::Utc::now().timestamp() + ROLE_CACHE_TTL_SECS,
        ),
    );
    Ok(roles)
}

pub async fn find_role(db: &DB, workspace_id: &str, name: &str) -> Result<Option<EffectiveRole>> {
    Ok(workspace_roles(db, workspace_id)
        .await?
        .iter()
        .find(|r| r.name == name)
        .cloned())
}

/// Capabilities of role `name` in `workspace_id`; [`RoleCapabilities::NONE`] for a name no longer
/// defined, so a deleted role fails closed.
pub async fn role_capabilities_by_name(
    db: &DB,
    workspace_id: &str,
    name: &str,
) -> Result<RoleCapabilities> {
    Ok(find_role(db, workspace_id, name)
        .await?
        .map(|r| r.capabilities)
        .unwrap_or(RoleCapabilities::NONE))
}

/// Whether a member holding the role of this `usr` row is restricted: what authentication puts in
/// `ApiAuthed::is_operator`. Runs on the caller's connection, which authentication already holds.
pub async fn member_is_restricted(
    conn: &mut sqlx::PgConnection,
    workspace_id: &str,
    workspace_role: Option<String>,
    operator: bool,
) -> Result<bool> {
    let name = resolve_role_name(workspace_role, operator);
    Ok(workspace_roles_on(conn, workspace_id)
        .await?
        .iter()
        .find(|r| r.name == name)
        .map(|r| r.capabilities)
        .unwrap_or(RoleCapabilities::NONE)
        .is_restricted())
}

/// [`member_is_restricted`] for a caller holding no connection.
pub async fn member_is_restricted_db(
    db: &DB,
    workspace_id: &str,
    workspace_role: Option<String>,
    operator: bool,
) -> Result<bool> {
    let name = resolve_role_name(workspace_role, operator);
    Ok(role_capabilities_by_name(db, workspace_id, &name)
        .await?
        .is_restricted())
}

async fn member_role(db: &DB, workspace_id: &str, username: &str) -> Result<Option<String>> {
    let now = chrono::Utc::now().timestamp();
    let key = (workspace_id.to_string(), username.to_string());
    if let Some((role, expiry)) = MEMBER_ROLE.get(&key) {
        if expiry > now {
            return Ok(role);
        }
    }
    let row = sqlx::query!(
        "SELECT workspace_role, operator FROM usr WHERE workspace_id = $1 AND username = $2",
        workspace_id,
        username
    )
    .fetch_optional(db)
    .await?;
    let role = row.map(|r| resolve_role_name(r.workspace_role, r.operator));
    MEMBER_ROLE.insert(key, (role.clone(), now + ROLE_CACHE_TTL_SECS));
    Ok(role)
}

/// What the caller may do in `workspace_id`. An admin, and any caller authentication did not mark
/// restricted, holds everything; a restricted caller holds its role's capabilities, or none if it
/// has no membership (a guest, a removed member's job). `workspace_id` MUST be the route's.
///
/// Call it before opening an RLS transaction; see [`workspace_roles`].
pub async fn caller_capabilities<A: Authable + ?Sized>(
    db: &DB,
    workspace_id: &str,
    authed: &A,
) -> Result<RoleCapabilities> {
    if authed.is_admin() || !authed.is_operator() {
        return Ok(RoleCapabilities::FULL);
    }
    match member_role(db, workspace_id, authed.username()).await? {
        Some(role) => role_capabilities_by_name(db, workspace_id, &role).await,
        None => Ok(RoleCapabilities::NONE),
    }
}

pub fn invalidate_workspace_roles_cache(workspace_id: &str) {
    WORKSPACE_ROLES.remove(workspace_id);
}

pub const ROLE_OVERRIDES_CHANGE_CHANNEL: &str = "notify_role_overrides_change";

/// Tells every replica to drop its cached roles for the workspace. Call it in the transaction
/// that writes `role_overrides`: a replica that missed it keeps authorizing whatever the workspace
/// just withdrew until its entry expires.
pub async fn notify_role_overrides_change(
    conn: &mut sqlx::PgConnection,
    workspace_id: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO notify_event (channel, payload) VALUES ($1, $2)")
        .bind(ROLE_OVERRIDES_CHANGE_CHANNEL)
        .bind(workspace_id)
        .execute(conn)
        .await?;
    Ok(())
}

pub fn invalidate_all_roles_cache() {
    INSTANCE_ROLES.clear();
    WORKSPACE_ROLES.clear();
    MEMBER_ROLE.clear();
}

pub fn invalidate_member_role_cache(workspace_id: &str, username: &str) {
    MEMBER_ROLE.remove(&(workspace_id.to_string(), username.to_string()));
}

/// Refuses removing role `name` while a member, a pending invite, the domain auto-invite or an
/// instance group mapping still names it: in `w_id`, or, for an instance role (`w_id = None`), in
/// any workspace, and while any workspace still overrides it.
pub async fn ensure_role_unused(db: &DB, w_id: Option<&str>, name: &str) -> Result<()> {
    let users = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM usr t WHERE ($1::text IS NULL OR t.workspace_id = $1) AND
            COALESCE(t.workspace_role, CASE WHEN t.operator THEN 'operator' ELSE 'developer' END) = $2",
        w_id,
        name
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);
    let invites = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM workspace_invite t WHERE ($1::text IS NULL OR t.workspace_id = $1) AND
            COALESCE(t.workspace_role, CASE WHEN t.operator THEN 'operator' ELSE 'developer' END) = $2",
        w_id,
        name
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);
    let settings = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM workspace_settings WHERE ($1::text IS NULL OR workspace_id = $1) AND (
            jsonb_typeof(auto_invite) = 'object' AND COALESCE(auto_invite->>'role',
                CASE WHEN (auto_invite->>'operator')::boolean THEN 'operator' ELSE 'developer' END) = $2
            OR EXISTS (SELECT 1 FROM jsonb_each_text(
                CASE WHEN jsonb_typeof(auto_invite->'instance_groups_roles') = 'object'
                     THEN auto_invite->'instance_groups_roles' ELSE '{}'::jsonb END) g
                WHERE g.value = $2))",
        w_id,
        name
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);
    if users + invites + settings > 0 {
        return Err(Error::BadRequest(format!(
            "role `{name}` is still held by {users} member(s) and {invites} pending invite(s), or named by {settings} workspace auto-invite or instance group mapping(s): reassign them first"
        )));
    }
    // An override left behind would silently re-base onto `base_for` instead of the instance
    // definition it was written against.
    if w_id.is_none() {
        let overriding = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM workspace_settings WHERE COALESCE(role_overrides ? $1, false)",
            name
        )
        .fetch_one(db)
        .await?
        .unwrap_or(0);
        if overriding > 0 {
            return Err(Error::BadRequest(format!(
                "role `{name}` is overridden in {overriding} workspace(s): remove those overrides first"
            )));
        }
    }
    Ok(())
}

/// Checks a write of the `workspace_roles` setting (`None` or null resets to the defaults) before
/// it is stored. Every writer of the setting must call it, the declarative config sync included.
pub async fn validate_instance_roles_write(
    db: &DB,
    value: Option<&serde_json::Value>,
) -> Result<()> {
    let current = instance_roles(db).await?;
    let next = match value {
        None | Some(serde_json::Value::Null) => default_instance_roles(),
        Some(v) => parse_instance_roles(v)?,
    };
    // Resetting stays open so a downgraded instance can always drop the setting.
    if !custom_roles_enabled() && *current != next && next != default_instance_roles() {
        return Err(Error::BadRequest(
            "changing workspace roles requires a valid Windmill Enterprise Edition license"
                .to_string(),
        ));
    }
    for removed in current
        .iter()
        .filter(|r| !next.iter().any(|n| n.name == r.name))
    {
        ensure_role_unused(db, None, &removed.name).await?;
    }
    // An override valid on the old definitions can merge into an invalid role on the new ones
    // (`flows: full` kept while the instance withdraws `code`), which the gates would honour.
    let stored: Vec<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT workspace_id, role_overrides FROM workspace_settings WHERE role_overrides IS NOT NULL",
    )
    .fetch_all(db)
    .await?;
    for (w, overrides) in stored {
        let overrides: RoleOverrides = serde_json::from_value(overrides)
            .map_err(|e| Error::internal_err(format!("invalid role overrides of {w}: {e:#}")))?;
        validate_overrides(&next, &overrides).map_err(|e| {
            Error::BadRequest(format!(
                "workspace {w}'s role overrides would become invalid: {e}"
            ))
        })?;
    }
    Ok(())
}

/// Side effects of a stored `workspace_roles` write, owed by every writer. The seat table is read
/// by billing on every replica, so it is rebuilt here rather than on the notify that drops the
/// role caches.
pub async fn after_instance_roles_write(db: &DB) -> Result<()> {
    invalidate_all_roles_cache();
    let mut tx = db.begin().await?;
    sync_role_seats(&mut tx, None).await?;
    tx.commit().await?;
    Ok(())
}

/// Rewrites `role_seat`, the per-role author flag billing SQL joins on, from the current role
/// configuration: the instance rows (`workspace_id = ''`) and those of every workspace with an
/// override, or only `workspace_id`'s when given. Run after any write to either setting; billing
/// reads nothing else about roles. Reads on `tx` only: a second pooled connection held alongside
/// it self-deadlocks on a one-connection pool.
pub async fn sync_role_seats<'c>(
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    workspace_id: Option<&str>,
) -> Result<()> {
    let instance = instance_roles_on(&mut **tx).await?;
    let workspaces: Vec<(String, Option<serde_json::Value>)> = match workspace_id {
        Some(w) => sqlx::query_as(
            "SELECT $1::text, (SELECT role_overrides FROM workspace_settings WHERE workspace_id = $1)",
        )
        .bind(w)
        .fetch_all(&mut **tx)
        .await?,
        None => {
            sqlx::query("DELETE FROM role_seat").execute(&mut **tx).await?;
            let rows = effective_roles(&instance, &RoleOverrides::new())?;
            write_role_seats(tx, "", &rows).await?;
            sqlx::query_as(
                "SELECT workspace_id, role_overrides FROM workspace_settings WHERE role_overrides IS NOT NULL",
            )
            .fetch_all(&mut **tx)
            .await?
        }
    };
    for (w, overrides) in workspaces {
        sqlx::query("DELETE FROM role_seat WHERE workspace_id = $1")
            .bind(&w)
            .execute(&mut **tx)
            .await?;
        let overrides: RoleOverrides = match overrides {
            Some(v) => serde_json::from_value(v).map_err(|e| {
                Error::internal_err(format!("invalid role overrides of {w}: {e:#}"))
            })?,
            None => continue,
        };
        let rows: Vec<_> = effective_roles(&instance, &overrides)?
            .into_iter()
            .filter(|r| r.overridden)
            .collect();
        write_role_seats(tx, &w, &rows).await?;
    }
    Ok(())
}

async fn write_role_seats<'c>(
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    workspace_id: &str,
    roles: &[EffectiveRole],
) -> Result<()> {
    let names: Vec<&str> = roles.iter().map(|r| r.name.as_str()).collect();
    let authors: Vec<bool> = roles.iter().map(|r| r.capabilities.is_author()).collect();
    sqlx::query(
        "INSERT INTO role_seat (workspace_id, role, author)
         SELECT $1, n, a FROM unnest($2::text[], $3::bool[]) AS x(n, a)",
    )
    .bind(workspace_id)
    .bind(&names)
    .bind(&authors)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// SQL for "the membership row aliased `t` consumes a full (author) seat": an admin always does,
/// whatever role it holds underneath; otherwise from `role_seat`, the workspace's row for the role,
/// else the instance's, else the legacy rule that only a developer is an author. `t` carries
/// `workspace_id`, `is_admin`, `operator` and `workspace_role`. Shared by every seat count so the
/// displayed, enforced and invoiced numbers agree.
pub const AUTHOR_SEAT_SQL: &str = "t.is_admin OR COALESCE(
        (SELECT rs.author FROM role_seat rs WHERE rs.workspace_id = t.workspace_id
            AND rs.role = COALESCE(t.workspace_role, CASE WHEN t.operator THEN 'operator' ELSE 'developer' END)),
        (SELECT rs.author FROM role_seat rs WHERE rs.workspace_id = ''
            AND rs.role = COALESCE(t.workspace_role, CASE WHEN t.operator THEN 'operator' ELSE 'developer' END)),
        NOT t.operator)";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_legacy_roles() {
        let roles = effective_roles(&default_instance_roles(), &RoleOverrides::new()).unwrap();
        assert_eq!(roles[0].capabilities, RoleCapabilities::FULL);
        assert!(!roles[0].capabilities.is_restricted());
        assert_eq!(roles[1].capabilities, RoleCapabilities::OPERATOR);
        assert!(roles[1].capabilities.is_restricted());
        assert!(!roles[1].capabilities.is_author());

        let hidden_page = RoleCapabilities {
            visibility: RoleVisibility { workers: false, ..RoleVisibility::ALL },
            ..RoleCapabilities::FULL
        };
        assert!(!hidden_page.is_restricted());
    }

    #[test]
    fn override_layers_over_instance_definition() {
        let instance: Vec<RoleDefinition> = serde_json::from_value(serde_json::json!([
            {"name": "developer"},
            {"name": "operator", "capabilities": {"manage_triggers": false, "visibility": {"runs": false}}}
        ]))
        .unwrap();
        let overrides: RoleOverrides = serde_json::from_value(serde_json::json!({
            "operator": {"capabilities": {"flows": "compose", "visibility": {"workers": false}}}
        }))
        .unwrap();
        let op = effective_roles(&instance, &overrides).unwrap().remove(1);
        assert!(op.overridden);
        assert_eq!(op.capabilities.flows, AuthorLevel::Compose);
        assert!(!op.capabilities.manage_triggers);
        assert!(!op.capabilities.visibility.runs);
        assert!(!op.capabilities.visibility.workers);
        assert!(op.capabilities.visibility.folders);
        assert!(op.capabilities.is_author());
    }

    #[test]
    fn full_authoring_requires_code() {
        let err = parse_instance_roles(&serde_json::json!([
            {"name": "builder", "capabilities": {"flows": "full"}}
        ]));
        assert!(err.is_err());
        assert!(parse_instance_roles(&serde_json::json!([])).is_err());
        assert!(parse_instance_roles(&serde_json::json!([{"name": "admin"}])).is_err());
    }
}
