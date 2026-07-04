use async_recursion::async_recursion;
#[cfg(feature = "cloud")]
use backon::{ConstantBuilder, Retryable};
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

use crate::{
    error::{self, to_anyhow, Error, Result},
    get_database_url,
    secret_backend::{get_secret_value, is_external_stored_value},
    utils::get_custom_pg_instance_password,
    variables::{build_crypt, decrypt},
    PgDatabase, DB,
};

macro_rules! sqlx_bitflags {
    (
        $flags:ty => $repr:ty
    ) => {
        // ---- Type ----
        impl sqlx::Type<sqlx::Postgres> for $flags {
            fn type_info() -> sqlx::postgres::PgTypeInfo {
                <$repr as sqlx::Type<sqlx::Postgres>>::type_info()
            }
        }

        // ---- Encode ----
        impl<'q> sqlx::Encode<'q, sqlx::Postgres> for $flags {
            fn encode_by_ref(
                &self,
                buf: &mut sqlx::postgres::PgArgumentBuffer,
            ) -> std::result::Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>>
            {
                let bits: $repr = self.bits();
                <$repr as sqlx::Encode<sqlx::Postgres>>::encode(bits, buf)
            }
        }

        // ---- Decode ----
        impl<'r> sqlx::Decode<'r, sqlx::Postgres> for $flags {
            fn decode(
                value: sqlx::postgres::PgValueRef<'r>,
            ) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
                let bits = <$repr as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
                <$flags>::from_bits(bits)
                    .ok_or_else(|| "invalid bitflags value from database".into())
            }
        }
    };
}

// Protection Rules - for fine-grained workspace access control

/// API representation of a protection rule
#[derive(Debug, Clone)]
pub struct ProtectionRuleset {
    pub workspace_id: String,
    pub name: String,
    pub rules: ProtectionRules,
    pub bypass_groups: Vec<String>,
    pub bypass_users: Vec<String>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    // #[sqlx(transparent)]
    pub struct ProtectionRules: i32 {
        const DISABLE_DIRECT_DEPLOYMENT =           1 << 0;
        const DISABLE_WORKSPACE_FORKING =           1 << 1;
        const RESTRICT_DEPLOY_TO_DEPLOYERS =        1 << 2;
        const RESTRICT_ANONYMOUS_APP_DEPLOYMENT =   1 << 3;
    }
}

sqlx_bitflags!(ProtectionRules => i32);

#[derive(Serialize, Deserialize, strum_macros::EnumIter)]
pub enum ProtectionRuleKind {
    DisableDirectDeployment,
    DisableWorkspaceForking,
    RestrictDeployToDeployers,
    RestrictAnonymousAppDeployment,
}

impl ProtectionRuleKind {
    pub const fn flag(&self) -> ProtectionRules {
        match self {
            ProtectionRuleKind::DisableDirectDeployment => {
                ProtectionRules::DISABLE_DIRECT_DEPLOYMENT
            }
            ProtectionRuleKind::DisableWorkspaceForking => {
                ProtectionRules::DISABLE_WORKSPACE_FORKING
            }
            ProtectionRuleKind::RestrictDeployToDeployers => {
                ProtectionRules::RESTRICT_DEPLOY_TO_DEPLOYERS
            }
            ProtectionRuleKind::RestrictAnonymousAppDeployment => {
                ProtectionRules::RESTRICT_ANONYMOUS_APP_DEPLOYMENT
            }
        }
    }

    pub const fn msg(&self) -> &str {
        match self {
            ProtectionRuleKind::DisableDirectDeployment => {
                "Cannot directly deploy in this workspace. Fork or Pull request required."
            }
            ProtectionRuleKind::DisableWorkspaceForking => "Forking this workspace is forbidden",
            ProtectionRuleKind::RestrictDeployToDeployers => {
                "Only workspace admins and members of wm_deployers can deploy to this workspace"
            }
            ProtectionRuleKind::RestrictAnonymousAppDeployment => {
                "Making an app publicly accessible without login (anonymous execution mode) is restricted in this workspace"
            }
        }
    }
}

impl From<&Vec<ProtectionRuleKind>> for ProtectionRules {
    fn from(value: &Vec<ProtectionRuleKind>) -> Self {
        let mut r = ProtectionRules::empty();
        for rule in value {
            r = r | rule.flag();
        }
        r
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct WorkspaceGitSyncSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_type: Option<Vec<ObjectType>>,
    pub repositories: Vec<GitRepositorySettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_include_path: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct WorkspaceDeploymentUISettings {
    pub include_path: Vec<String>,
    pub include_type: Vec<ObjectType>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ObjectType {
    Script,
    Flow,
    App,
    Folder,
    Resource,
    Variable,
    Secret,
    Schedule,
    ResourceType,
    User,
    Group,
    Trigger,
    Settings,
    Key,
    WorkspaceDependencies,
}

pub const LATEST_GIT_SYNC_SCRIPT_PATH: &str = "hub/28719/sync-script-to-git-repo-windmill";

/// Prefix used to identify fork workspaces. A workspace whose id starts with this string is a
/// fork of another workspace.
pub const WM_FORK_PREFIX: &str = "wm-fork-";

/// Validate that a fork workspace id is safe to interpolate into a git branch name.
///
/// The id is appended verbatim to a branch like `wm-fork/<original_branch>/<id>`,
/// so it must satisfy `git check-ref-format` rules. We validate synchronously at the API
/// layer because the actual branch creation runs in a deferred git-sync worker job — without
/// this check, the API returns 200 and the failure only surfaces later in the worker.
pub fn validate_fork_workspace_id(id: &str) -> error::Result<()> {
    validate_workspace_branch_id(id, true)
}

/// Like [`validate_fork_workspace_id`] but does not require the `wm-fork-` prefix. Used for dev
/// workspaces, whose id is an ordinary (prefix-less) workspace id but must still be git-branch-safe
/// because it is interpolated into a `wm-fork/<original_branch>/<id>` branch name like any fork.
pub fn validate_dev_workspace_id(id: &str) -> error::Result<()> {
    validate_workspace_branch_id(id, false)
}

/// The `workspace.name` column is `character varying(50)`, so a name longer than 50 characters
/// triggers a raw `value too long for type character varying(50)` SQL error on insert. Validate
/// up front to return a clear message instead.
pub fn validate_workspace_name(name: &str) -> error::Result<()> {
    if name.chars().count() > 50 {
        return Err(Error::BadRequest(format!(
            "Workspace name is too long ({} chars). Maximum length is 50 characters.",
            name.chars().count()
        )));
    }
    Ok(())
}

fn validate_workspace_branch_id(id: &str, require_fork_prefix: bool) -> error::Result<()> {
    if id.is_empty() {
        return Err(Error::BadRequest(
            "Workspace id cannot be empty".to_string(),
        ));
    }

    if require_fork_prefix && !id.starts_with(WM_FORK_PREFIX) {
        return Err(Error::BadRequest(format!(
            "The id `{}` is invalid for a forked workspace. It should be prefixed by {}",
            id, WM_FORK_PREFIX
        )));
    }

    if id.len() > 50 {
        return Err(Error::BadRequest(format!(
            "Workspace id `{}` is too long ({} chars). Maximum length is 50 characters.",
            id,
            id.len()
        )));
    }

    let reject = |reason: &str| {
        Err::<(), _>(Error::BadRequest(format!(
            "Fork workspace id `{}` is invalid: {} (must be a valid git branch name component)",
            id, reason
        )))
    };

    if id.ends_with('.') {
        return reject("cannot end with '.'");
    }
    if id.ends_with(".lock") {
        return reject("cannot end with '.lock'");
    }
    if id.contains("..") {
        return reject("cannot contain '..'");
    }
    if id.contains("@{") {
        return reject("cannot contain '@{'");
    }
    if id.contains("//") {
        return reject("cannot contain '//'");
    }
    for ch in id.chars() {
        match ch {
            ':' | '~' | '^' | '?' | '*' | '[' | '\\' | ' ' => {
                return reject(&format!("contains forbidden character '{}'", ch));
            }
            c if c.is_ascii_control() || c == '\u{7f}' => {
                return reject("contains a control character");
            }
            _ => {}
        }
    }
    // Each slash-separated component cannot start with '.' or end with '.lock'.
    for component in id.split('/') {
        if component.starts_with('.') {
            return reject("a path component cannot start with '.'");
        }
        if component.ends_with(".lock") {
            return reject("a path component cannot end with '.lock'");
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitRepositorySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_types_override: Option<Vec<ObjectType>>,
    /// None means auto-managed: always use LATEST_GIT_SYNC_SCRIPT_PATH.
    /// Some(path) means pinned to a specific script.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    pub git_repo_resource_path: String,
    pub use_individual_branch: Option<bool>,
    pub group_by_folder: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<GitSyncSettings>,
}

impl GitRepositorySettings {
    pub fn effective_script_path(&self) -> &str {
        self.script_path
            .as_deref()
            .unwrap_or(LATEST_GIT_SYNC_SCRIPT_PATH)
    }

    pub fn is_script_meets_min_version(&self, min_version: u32) -> error::Result<bool> {
        let path = self.effective_script_path();
        // example: "hub/28102/sync-script-to-git-repo-windmill"
        let current = path
            .split("/") // -> ["hub" "28102" "sync-script-to-git-repo-windmill"]
            .skip(1) // omit "hub"
            .next() // get numeric id
            .ok_or(Error::InternalErr(format!(
                "cannot get script version id from: {}",
                path
            )))?
            .parse()
            .unwrap_or_else(|e| {
                tracing::warn!("cannot get script version id from: {}. e: {e}", path);

                u32::MAX
            });

        Ok(current >= min_version) // this works on assumption that all scripts in hub have sequential ids
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitSyncSettings {
    pub include_path: Vec<String>,
    pub include_type: Vec<ObjectType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_include_path: Option<Vec<String>>,
}

impl Default for GitSyncSettings {
    fn default() -> Self {
        Self {
            include_path: Vec::new(),
            include_type: Vec::new(),
            exclude_path: None,
            extra_include_path: None,
        }
    }
}

#[derive(Clone)]
pub struct TeamPlanStatus {
    pub premium: bool,
    pub is_past_due: bool,
    pub max_tolerated_executions: Option<i32>,
}

lazy_static::lazy_static! {
    pub static ref TEAM_PLAN_CACHE: Cache<String, TeamPlanStatus> = Cache::new(5000);
    // Value: (rate_limit, cached_at_timestamp)
    pub static ref PUBLIC_APP_RATE_LIMIT_CACHE: Cache<String, (Option<i32>, i64)> = Cache::new(1000);
}

#[cfg(feature = "cloud")]
lazy_static::lazy_static! {
    // Maps a workspace id to its root (billing) ancestor. Value: (root_id, expiry_timestamp).
    // Reparenting (attach/detach dev) is rare and self-heals via the 60s TTL, so a brief stale
    // mapping only mis-attributes usage for <60s across other instances.
    pub static ref BILLING_WORKSPACE_CACHE: Cache<String, (String, i64)> = Cache::new(5000);
}

/// Resolve the "billing" workspace for `w_id`: the root ancestor of the fork/dev chain (the
/// workspace whose plan and usage a fork draws from). Returns `w_id` unchanged for a standalone
/// workspace, an unknown id, or a (malformed) cyclic chain.
///
/// Unauthenticated metering helper: it only reads the parent chain and returns another workspace id,
/// so callers must already be authorized for `w_id` (or run in trusted server-side code); `w_id` is
/// expected to be a server-side id, not raw user input.
#[cfg(feature = "cloud")]
pub async fn get_billing_workspace_id(db: &crate::DB, w_id: &str) -> Result<String> {
    let now = chrono::Utc::now().timestamp();
    if let Some((root, expiry)) = BILLING_WORKSPACE_CACHE.get(w_id) {
        if expiry > now {
            return Ok(root);
        }
    }

    // The depth bound is a cycle-safety backstop kept well above the enforced `MAX_FORK_DEPTH`, so a
    // truncated (root-not-found) result — which would fall back to `w_id` and mis-attribute billing —
    // is unreachable for any real hierarchy; only a malformed cycle could hit it.
    let root = sqlx::query_scalar!(
        r#"
            WITH RECURSIVE chain AS (
                SELECT id, parent_workspace_id, 0 AS depth
                FROM workspace WHERE id = $1
                UNION ALL
                SELECT w.id, w.parent_workspace_id, chain.depth + 1
                FROM workspace w
                JOIN chain ON w.id = chain.parent_workspace_id
                WHERE chain.depth < 20
            )
            SELECT id AS "id!" FROM chain WHERE parent_workspace_id IS NULL LIMIT 1
        "#,
        w_id
    )
    .fetch_optional(db)
    .await
    .map_err(|e| Error::internal_err(format!("resolving billing workspace for {w_id}: {e:#}")))?
    .unwrap_or_else(|| w_id.to_string());

    BILLING_WORKSPACE_CACHE.insert(w_id.to_string(), (root.clone(), now + 60));
    Ok(root)
}

/// Invalidate the billing-workspace mapping for a workspace (call after reparenting it).
#[cfg(feature = "cloud")]
pub fn invalidate_billing_workspace_cache(w_id: &str) {
    BILLING_WORKSPACE_CACHE.remove(w_id);
}

/// Invalidate the cached team-plan (premium/past-due) status for a workspace. `TEAM_PLAN_CACHE` has
/// no TTL — it's only evicted by the premium-change NOTIFY — so call this when a workspace id is
/// permanently deleted, otherwise a reused id could inherit the old workspace's premium status.
#[cfg(feature = "cloud")]
pub fn invalidate_team_plan_cache(w_id: &str) {
    TEAM_PLAN_CACHE.remove(w_id);
}

/// Depth of `w_id` in its fork chain: 0 for a root (no parent), 1 for a direct fork, and so on. Walks
/// the parent chain up to the root. The recursion bound is a cycle-safety backstop set well above the
/// enforced `MAX_FORK_DEPTH`; a (malformed) cyclic chain saturates it and so reads as "too deep",
/// which safely rejects rather than allows.
///
/// Unauthenticated helper: reads workspace hierarchy for any `w_id`, so callers must already be
/// authorized for that workspace (or run in trusted server-side code).
pub async fn fork_chain_depth(db: &crate::DB, w_id: &str) -> Result<i64> {
    let depth = sqlx::query_scalar!(
        r#"
            WITH RECURSIVE chain AS (
                SELECT id, parent_workspace_id, 0 AS depth
                FROM workspace WHERE id = $1
                UNION ALL
                SELECT w.id, w.parent_workspace_id, chain.depth + 1
                FROM workspace w
                JOIN chain ON w.id = chain.parent_workspace_id
                WHERE chain.depth < 20
            )
            SELECT COALESCE(MAX(depth), 0)::bigint AS "depth!" FROM chain
        "#,
        w_id
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::internal_err(format!("computing fork depth for {w_id}: {e:#}")))?;
    Ok(depth)
}

/// Height of the fork subtree rooted at `w_id`: 0 when it has no live child forks, 1 with direct
/// children, and so on. Used so that attaching a candidate which already has its own child forks can't
/// push the family past the depth limit.
///
/// Unauthenticated helper: reads workspace hierarchy for any `w_id`, so callers must already be
/// authorized for that workspace (or run in trusted server-side code).
pub async fn fork_subtree_height(db: &crate::DB, w_id: &str) -> Result<i64> {
    // The `deleted` filter is applied in the outer aggregation (not the recursive step, matching
    // count_workspace_forks) so a live descendant under a soft-deleted intermediate is still measured
    // at its true depth rather than pruned — otherwise the height could be underestimated and let the
    // resulting chain exceed the depth limit.
    let height = sqlx::query_scalar!(
        r#"
            WITH RECURSIVE tree AS (
                SELECT id, deleted, 0 AS depth FROM workspace WHERE id = $1
                UNION ALL
                SELECT w.id, w.deleted, tree.depth + 1 FROM workspace w
                JOIN tree ON w.parent_workspace_id = tree.id
                WHERE tree.depth < 20
            )
            SELECT COALESCE(MAX(depth) FILTER (WHERE NOT deleted), 0)::bigint AS "height!" FROM tree
        "#,
        w_id
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::internal_err(format!("computing fork subtree height for {w_id}: {e:#}")))?;
    Ok(height)
}

/// Ids of every fork/dev workspace anywhere under `w_id` (excludes `w_id` itself), including live
/// descendants beneath a soft-deleted intermediate. Used to invalidate per-workspace caches for a
/// whole subtree after its ancestor is reparented.
///
/// Unauthenticated helper: reads workspace hierarchy for any `w_id`, so callers must already be
/// authorized for that workspace (or run in trusted server-side code).
pub async fn list_fork_descendants(db: &crate::DB, w_id: &str) -> Result<Vec<String>> {
    let ids = sqlx::query_scalar!(
        r#"
            WITH RECURSIVE tree AS (
                SELECT id, 0 AS depth FROM workspace WHERE id = $1
                UNION ALL
                SELECT w.id, tree.depth + 1 FROM workspace w
                JOIN tree ON w.parent_workspace_id = tree.id
                WHERE tree.depth < 20
            )
            SELECT id AS "id!" FROM tree WHERE id != $1
        "#,
        w_id
    )
    .fetch_all(db)
    .await
    .map_err(|e| Error::internal_err(format!("listing fork descendants of {w_id}: {e:#}")))?;
    Ok(ids)
}

/// Count non-deleted fork/dev workspaces anywhere under `root` (excludes `root` itself).
///
/// Unauthenticated metering helper: it reads workspace hierarchy for any `root` id, so callers must
/// already be authorized for that workspace (or run in trusted server-side code). `root` is expected
/// to be a server-resolved id, never raw user input.
#[cfg(feature = "cloud")]
pub async fn count_workspace_forks(db: &crate::DB, root: &str) -> Result<i64> {
    // The `deleted` filter is on the outer SELECT (not the recursive step) so that a live sub-fork
    // whose intermediate parent was soft-deleted is still counted rather than pruned with it. The
    // depth bound is a cycle-safety backstop kept well above the enforced `MAX_FORK_DEPTH`, so
    // descendants are never silently dropped from the cap count.
    let count = sqlx::query_scalar!(
        r#"
            WITH RECURSIVE tree AS (
                SELECT id, deleted, 0 AS depth FROM workspace WHERE id = $1
                UNION ALL
                SELECT w.id, w.deleted, tree.depth + 1 FROM workspace w
                JOIN tree ON w.parent_workspace_id = tree.id
                WHERE tree.depth < 20
            )
            SELECT COUNT(DISTINCT id) AS "count!" FROM tree WHERE id != $1 AND NOT deleted
        "#,
        root
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::internal_err(format!("counting forks of {root}: {e:#}")))?;
    Ok(count)
}

/// Approximate paid seats of a workspace as `ceil(developers + operators/2)`, excluding disabled and
/// service-account members. Reuses billing's author/operator weighting, but counts provisioned
/// members rather than the active-user population billing meters, so it only ever loosens the fork
/// cap (never blocks a paid seat) — good enough for a soft guardrail.
///
/// Unauthenticated metering helper: reads member counts for any `w_id`, so callers must already be
/// authorized for that workspace (or run in trusted server-side code).
#[cfg(feature = "cloud")]
pub async fn count_paid_seats(db: &crate::DB, w_id: &str) -> Result<i64> {
    let row = sqlx::query!(
        r#"SELECT
            COUNT(*) FILTER (WHERE NOT operator AND NOT disabled AND NOT is_service_account) AS "developers!",
            COUNT(*) FILTER (WHERE operator AND NOT disabled AND NOT is_service_account) AS "operators!"
        FROM usr WHERE workspace_id = $1"#,
        w_id
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::internal_err(format!("counting paid seats of {w_id}: {e:#}")))?;
    Ok(((row.developers as f64) + 0.5 * (row.operators as f64)).ceil() as i64)
}

#[cfg(feature = "cloud")]
pub async fn get_team_plan_status(_db: &crate::DB, _w_id: &str) -> Result<TeamPlanStatus> {
    // A fork/dev workspace draws its plan from the root (billing) workspace. Resolve to the root and
    // key the cache by it: the premium-change NOTIFY is keyed by the workspace whose premium row
    // changed (the root), so keying by root keeps invalidation correct and lets forks share it.
    let billing_w_id = get_billing_workspace_id(_db, _w_id).await?;
    let cached = TEAM_PLAN_CACHE.get(&billing_w_id);
    if let Some(cached) = cached {
        return Ok(cached);
    }

    let team_plan_info = (|| async {
        sqlx::query_as!(
            TeamPlanStatus,
            r#"
                SELECT
                    w.premium,
                    COALESCE(cw.is_past_due, false) as "is_past_due!",
                    cw.max_tolerated_executions
                FROM
                    workspace w
                    LEFT JOIN cloud_workspace_settings cw ON cw.workspace_id = w.id
                WHERE
                    w.id = $1
            "#,
            billing_w_id
        )
        .fetch_optional(_db)
        .await
    })
    .retry(
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_secs(5))
            .with_max_times(10),
    )
    .notify(|err, dur| {
        tracing::error!(
            "Failed to get team plan status for workspace {billing_w_id} (will retry in {dur:?}): {err:#}"
        );
    })
    .await
    .map_err(|err| {
        Error::internal_err(format!(
            "Failed to get team plan status for workspace {billing_w_id} after 10 retries: {err:#}"
        ))
    })?
    .unwrap_or_else(|| TeamPlanStatus {
        premium: false,
        is_past_due: false,
        max_tolerated_executions: None,
    });

    TEAM_PLAN_CACHE.insert(billing_w_id, team_plan_info.clone());

    Ok(team_plan_info)
}

// Protection Rules Cache

lazy_static::lazy_static! {
    pub static ref PROTECTION_RULES_CACHE: Cache<String, (std::sync::Arc<Vec<ProtectionRuleset>>, i64)> = Cache::new(100);
}

/// Get all protection rules for a workspace with caching (60s TTL)
pub async fn get_protection_rules(
    workspace_id: &str,
    db: &DB,
) -> Result<std::sync::Arc<Vec<ProtectionRuleset>>> {
    let now = chrono::Utc::now().timestamp();

    // Check cache and expiry
    if let Some((cached_rules, expiry)) = PROTECTION_RULES_CACHE.get(workspace_id) {
        if expiry > now {
            return Ok(cached_rules);
        }
    }

    // Query database
    let rulesets = sqlx::query_as!(
        ProtectionRuleset,
        r#"
            SELECT
                workspace_id,
                name,
                rules as "rules: ProtectionRules",
                bypass_groups,
                bypass_users
            FROM workspace_protection_rule
            WHERE workspace_id = $1
            ORDER BY name
        "#,
        workspace_id
    )
    .fetch_all(db)
    .await
    .map_err(|e| Error::internal_err(format!("Failed to fetch protection rules: {}", e)))?;

    // Cache with 60s TTL
    let arc_rules = std::sync::Arc::new(rulesets);
    let expiry = now + 60;
    PROTECTION_RULES_CACHE.insert(workspace_id.to_string(), (arc_rules.clone(), expiry));

    Ok(arc_rules)
}

/// Invalidate the protection rules cache for a workspace
pub fn invalidate_protection_rules_cache(workspace_id: &str) {
    PROTECTION_RULES_CACHE.remove(workspace_id);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleCheckResult {
    Allowed,
    Blocked(String),
}

/// Check if a user can bypass a protection rule
///
/// Returns `Allowed` if:
/// - User is in the rule's bypass users list (u/<username>)
/// - User's group is in the rule's bypass groups list (g/<groupname>)
///
/// Returns `Blocked` if:
/// - User is not in bypass lists
///
/// Returns `Err` if the rule is not found
/// Reserved protection-rule name applied to a prod workspace paired with a dev workspace. It carries
/// `DisableDirectDeployment` + `DisableWorkspaceForking` and is auto-managed by the dev-workspace
/// feature (applied on pairing, removed on detach).
pub const DEV_WORKSPACE_LOCK_RULE_NAME: &str = "dev_workspace_lock";

pub async fn check_user_against_rule(
    workspace_id: &str,
    rule: &ProtectionRuleKind,
    username: &str,
    user_groups: &[String],
    is_admin: bool,
    db: &DB,
) -> Result<RuleCheckResult> {
    if is_admin {
        return Ok(RuleCheckResult::Allowed);
    }

    // wm_deployers members implicitly satisfy RestrictDeployToDeployers,
    // regardless of per-ruleset bypass configuration.
    if matches!(rule, ProtectionRuleKind::RestrictDeployToDeployers)
        && user_groups.iter().any(|g| g == crate::WM_DEPLOYERS_GROUP)
    {
        return Ok(RuleCheckResult::Allowed);
    }

    let rulesets = get_protection_rules(workspace_id, db).await?;

    for ruleset in rulesets.iter() {
        if ruleset.rules.contains(rule.flag()) {
            if ruleset.bypass_users.iter().any(|u| u == username)
                || ruleset
                    .bypass_groups
                    .iter()
                    .any(|g| user_groups.contains(g))
            {
                continue;
            }
            return Ok(RuleCheckResult::Blocked(format!(
                "Ruleset {} of {} blocked this action: {}",
                ruleset.name,
                workspace_id,
                rule.msg()
            )));
        }
    }

    Ok(RuleCheckResult::Allowed)
}

/// Check all deploy-gating protection rules at once.
///
/// Evaluates `DisableDirectDeployment` first (so its message wins when both
/// rules would block), then `RestrictDeployToDeployers`. Returns the first
/// `Blocked` result, or `Allowed` if neither rule blocks.
///
/// Use this at every item create/update endpoint that participates in the
/// deploy/merge flow so a single call enforces both rules consistently.
pub async fn check_deploy_rules(
    workspace_id: &str,
    username: &str,
    user_groups: &[String],
    is_admin: bool,
    db: &DB,
) -> Result<RuleCheckResult> {
    for rule in [
        ProtectionRuleKind::DisableDirectDeployment,
        ProtectionRuleKind::RestrictDeployToDeployers,
    ] {
        let res = check_user_against_rule(workspace_id, &rule, username, user_groups, is_admin, db)
            .await?;
        if matches!(res, RuleCheckResult::Blocked(_)) {
            return Ok(res);
        }
    }
    Ok(RuleCheckResult::Allowed)
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DataTableForkBehavior {
    SchemaOnly,
    SchemaAndData,
    KeepOriginal,
}

impl Default for DataTableForkBehavior {
    fn default() -> Self {
        DataTableForkBehavior::KeepOriginal
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DataTable {
    pub database: DataTableDatabase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<DataTableForkedFrom>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DataTableForkedFrom {
    /// Schema snapshot at fork time
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DataTableDatabase {
    pub resource_type: DataTableCatalogResourceType,
    pub resource_path: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum DataTableCatalogResourceType {
    #[strum(serialize = "postgres")]
    Postgresql,
    Instance,
}

pub async fn get_datatable_resource_from_db_unchecked(
    db: &DB,
    w_id: &str,
    name: &str,
) -> Result<serde_json::Value> {
    let datatable = sqlx::query_scalar!(
        r#"
            SELECT ws.datatable->'datatables'->$2 AS config
            FROM workspace_settings ws
            WHERE ws.workspace_id = $1
        "#,
        &w_id,
        name
    )
    .fetch_one(db)
    .await
    .map_err(|err| Error::internal_err(format!("getting datatable {name}: {err}")))?
    .ok_or_else(|| Error::internal_err(format!("datatable {name} not found")))?;
    let datatable = serde_json::from_value::<DataTable>(datatable)?;

    let db_resource = if datatable.database.resource_type == DataTableCatalogResourceType::Instance
    {
        let mut pg_creds = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?;
        pg_creds.dbname = datatable.database.resource_path.clone();
        pg_creds.user = Some("custom_instance_user".to_string());
        pg_creds.password = Some(get_custom_pg_instance_password(&db).await?);
        serde_json::to_value(&pg_creds)
            .map_err(|e| Error::internal_err(format!("Error serializing pg creds: {}", e)))?
    } else {
        transform_json_unchecked(
            &serde_json::Value::String(format!("$res:{}", datatable.database.resource_path)),
            w_id,
            db,
        )
        .await?
    };

    Ok(db_resource)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Ducklake {
    pub catalog: DucklakeCatalog,
    pub storage: DucklakeStorage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<DucklakeMaintenance>,
}

/// Scheduled maintenance for a ducklake (enterprise): snapshot expiry,
/// adjacent-file compaction and orphaned-file cleanup, run as a managed
/// per-lake schedule. Not mirrored in `instance_config::Ducklake`:
/// instance-level lakes have no workspace to schedule into.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DucklakeMaintenance {
    pub enabled: bool,
    /// Cron (v2/croner, seconds optional). None → daily at 03:00 UTC with a
    /// deterministic per-(workspace, lake) minute offset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    /// Snapshot retention window in days (default 7). Snapshots older than
    /// this are expired: time-travel reads (`AT (VERSION => n)`) older than
    /// the window stop working. 0 keeps only the current snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_days: Option<u32>,
    /// Merge adjacent small parquet files (default true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compaction: Option<bool>,
    /// Delete orphaned files older than max(retention, 1 day) (default true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphan_cleanup: Option<bool>,
}

impl DucklakeMaintenance {
    pub const DEFAULT_RETENTION_DAYS: u32 = 7;

    pub fn retention_days(&self) -> u32 {
        self.retention_days.unwrap_or(Self::DEFAULT_RETENTION_DAYS)
    }
    pub fn compaction(&self) -> bool {
        self.compaction.unwrap_or(true)
    }
    pub fn orphan_cleanup(&self) -> bool {
        self.orphan_cleanup.unwrap_or(true)
    }
}

/// Reserved schedule path namespace for managed ducklake maintenance
/// schedules. Must satisfy the `schedule.path` CHECK constraint
/// (`^[ufg](\/[\w-]+){2,}$`), hence the `f/` prefix; the folder itself never
/// exists. The schedule API rejects user mutations under this prefix and the
/// list/export endpoints filter it out — the lifecycle is owned by the
/// workspace ducklake settings.
pub const DUCKLAKE_MAINTENANCE_PATH_PREFIX: &str = "f/ducklake_maintenance/";

pub fn ducklake_maintenance_schedule_path(lake: &str) -> String {
    format!("{DUCKLAKE_MAINTENANCE_PATH_PREFIX}{lake}")
}

pub fn lake_from_ducklake_maintenance_path(path: &str) -> Option<&str> {
    path.strip_prefix(DUCKLAKE_MAINTENANCE_PATH_PREFIX)
}

/// Default maintenance cadence: daily at 03:00-03:59 UTC, minute picked
/// deterministically per (workspace, lake) so all lakes of an instance don't
/// fire in the same second.
pub fn default_ducklake_maintenance_cron(w_id: &str, lake: &str) -> String {
    let mut hash: u32 = 5381;
    for b in w_id.bytes().chain("/".bytes()).chain(lake.bytes()) {
        hash = hash.wrapping_mul(33).wrapping_add(b as u32);
    }
    format!("0 {} 3 * * *", hash % 60)
}

/// Lake names are interpolated into `ATTACH 'ducklake://<name>'`, generated
/// maintenance SQL and the reserved schedule path (CHECK-constrained to
/// `[\w-]+` segments), so they must stay to this charset.
pub fn is_valid_ducklake_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Raw per-lake config (no credential interpolation). This does not check for
/// any permission (like [`get_ducklake_from_db_unchecked`]): callers are
/// internal (schedule-tick payload construction) and must not expose the
/// config to a user. Generic over the executor so callers inside a
/// transaction (e.g. `push_scheduled_job` during `change_workspace_id`) see
/// uncommitted settings.
pub async fn get_ducklake_raw_unchecked<'c, E: sqlx::PgExecutor<'c>>(
    executor: E,
    w_id: &str,
    name: &str,
) -> Result<Option<Ducklake>> {
    let config = sqlx::query_scalar!(
        r#"
            SELECT ws.ducklake->'ducklakes'->$2 AS config
            FROM workspace_settings ws
            WHERE ws.workspace_id = $1
        "#,
        &w_id,
        name
    )
    .fetch_optional(executor)
    .await
    .map_err(|err| Error::internal_err(format!("getting ducklake {name}: {err}")))?
    .flatten();

    match config {
        Some(c) => Ok(Some(serde_json::from_value::<Ducklake>(c)?)),
        None => Ok(None),
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DucklakeCatalog {
    pub resource_type: DucklakeCatalogResourceType,
    pub resource_path: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DucklakeStorage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    pub path: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum DucklakeCatalogResourceType {
    #[strum(serialize = "postgres")]
    Postgresql,
    Mysql,
    Instance,
}

#[derive(Deserialize, Serialize)]
pub struct DucklakeWithConnData {
    pub catalog: DucklakeCatalog,
    pub catalog_resource: serde_json::Value,
    pub storage: DucklakeStorage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<String>,
}

pub async fn get_ducklake_from_db_unchecked(
    name: &str,
    w_id: &str,
    db: &DB,
) -> Result<DucklakeWithConnData> {
    let ducklake = sqlx::query_scalar!(
        r#"
            SELECT ws.ducklake->'ducklakes'->$2 AS config
            FROM workspace_settings ws
            WHERE ws.workspace_id = $1
        "#,
        &w_id,
        name
    )
    .fetch_one(db)
    .await
    .map_err(|err| Error::internal_err(format!("getting ducklake {name}: {err}")))?
    .ok_or_else(|| Error::internal_err(format!("ducklake {name} not found")))?;

    let ducklake = serde_json::from_value::<Ducklake>(ducklake)?;

    let catalog_resource =
        if ducklake.catalog.resource_type == DucklakeCatalogResourceType::Instance {
            let mut pg_creds = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?;
            pg_creds.dbname = ducklake.catalog.resource_path.clone();
            pg_creds.user = Some("custom_instance_user".to_string());
            pg_creds.password = Some(get_custom_pg_instance_password(&db).await?);
            serde_json::to_value(&pg_creds)
                .map_err(|e| Error::internal_err(format!("Error serializing pg creds: {}", e)))?
        } else {
            transform_json_unchecked(
                &serde_json::Value::String(format!("$res:{}", ducklake.catalog.resource_path)),
                w_id,
                db,
            )
            .await?
        };
    let ducklake = DucklakeWithConnData {
        catalog_resource,
        catalog: ducklake.catalog,
        storage: ducklake.storage,
        extra_args: ducklake.extra_args,
    };
    Ok(ducklake)
}

// This does not check for any permission. Should never be displayed to a user.
#[async_recursion]
async fn transform_json_unchecked(
    value: &serde_json::Value,
    w_id: &str,
    db: &DB,
) -> Result<serde_json::Value> {
    let value = match value {
        serde_json::Value::Object(map) => {
            let mut transformed_map = serde_json::Map::new();
            for (key, val) in map {
                let transformed_val = transform_json_unchecked(val, w_id, db).await?;
                transformed_map.insert(key.clone(), serde_json::to_value(transformed_val)?);
            }
            serde_json::Value::Object(transformed_map)
        }
        serde_json::Value::Array(arr) => {
            let mut transformed_array = Vec::new();
            for val in arr {
                let transformed_val = transform_json_unchecked(val, w_id, db).await?;
                transformed_array.push(serde_json::to_value(transformed_val)?);
            }
            serde_json::Value::Array(transformed_array)
        }
        serde_json::Value::String(s) if s.starts_with("$res:") => {
            let resource = sqlx::query_scalar!(
                "SELECT value AS \"value!: _\" FROM resource WHERE workspace_id = $1 AND path = $2",
                &w_id,
                &s[5..]
            )
            .fetch_one(db)
            .await
            .map_err(to_anyhow)?;
            transform_json_unchecked(&resource, w_id, db).await?
        }
        serde_json::Value::String(s) if s.starts_with("$var:") => {
            let (value, is_secret): (String, bool) = sqlx::query_as(
                "SELECT value, is_secret FROM variable WHERE workspace_id = $1 AND path = $2",
            )
            .bind(&w_id)
            .bind(&s[5..])
            .fetch_one(db)
            .await
            .map_err(to_anyhow)?;
            let value = if is_secret {
                if is_external_stored_value(&value) {
                    get_secret_value(db, w_id, &s[5..], &value).await?
                } else {
                    let mc = build_crypt(&db, &w_id).await?;
                    decrypt(&mc, value).map_err(|e| {
                        Error::internal_err(format!("Error decrypting variable {}: {}", &s, e))
                    })?
                }
            } else {
                value
            };
            serde_json::Value::String(value)
        }
        s @ serde_json::Value::String(_) => s.clone(),
        x => x.clone(),
    };

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_fork_workspace_id_accepts_valid() {
        validate_fork_workspace_id("wm-fork-test-allow").unwrap();
        validate_fork_workspace_id("wm-fork-my_workspace.42").unwrap();
        validate_fork_workspace_id("wm-fork-a").unwrap();
    }

    #[test]
    fn test_validate_fork_workspace_id_rejects_missing_prefix() {
        assert!(validate_fork_workspace_id("not-a-fork").is_err());
        assert!(validate_fork_workspace_id("wm-fork").is_err());
    }

    #[test]
    fn test_validate_fork_workspace_id_rejects_git_unsafe_chars() {
        for bad in [
            "wm-fork-test:allow",
            "wm-fork-test allow",
            "wm-fork-test~allow",
            "wm-fork-test^allow",
            "wm-fork-test?allow",
            "wm-fork-test*allow",
            "wm-fork-test[allow",
            "wm-fork-test\\allow",
            "wm-fork-test\nallow",
        ] {
            assert!(
                validate_fork_workspace_id(bad).is_err(),
                "expected `{}` to be rejected",
                bad
            );
        }
    }

    #[test]
    fn test_validate_fork_workspace_id_rejects_git_unsafe_sequences() {
        assert!(validate_fork_workspace_id("wm-fork-foo..bar").is_err());
        assert!(validate_fork_workspace_id("wm-fork-foo@{bar").is_err());
        assert!(validate_fork_workspace_id("wm-fork-foo//bar").is_err());
        assert!(validate_fork_workspace_id("wm-fork-foo.").is_err());
        assert!(validate_fork_workspace_id("wm-fork-foo.lock").is_err());
        assert!(validate_fork_workspace_id("wm-fork-foo/.bar").is_err());
        assert!(validate_fork_workspace_id("wm-fork-foo/bar.lock").is_err());
    }

    #[test]
    fn test_validate_dev_workspace_id_accepts_prefixless_valid() {
        // Dev workspaces use ordinary, prefix-less ids but must stay git-branch-safe.
        validate_dev_workspace_id("dev").unwrap();
        validate_dev_workspace_id("my-dev-workspace").unwrap();
        validate_dev_workspace_id("staging.42").unwrap();
        // The fork prefix is allowed but not required.
        validate_dev_workspace_id("wm-fork-dev").unwrap();
    }

    #[test]
    fn test_validate_dev_workspace_id_rejects_empty_and_git_unsafe() {
        assert!(validate_dev_workspace_id("").is_err());
        assert!(validate_dev_workspace_id("dev workspace").is_err());
        assert!(validate_dev_workspace_id("dev..staging").is_err());
        assert!(validate_dev_workspace_id("dev/.x").is_err());
        assert!(validate_dev_workspace_id("dev.lock").is_err());
    }

    #[test]
    fn test_validate_fork_workspace_id_rejects_empty() {
        assert!(validate_fork_workspace_id("").is_err());
    }

    #[test]
    fn test_validate_fork_workspace_id_rejects_too_long() {
        let long_id = format!("wm-fork-{}", "a".repeat(43));
        assert!(validate_fork_workspace_id(&long_id).is_err());
    }
}
