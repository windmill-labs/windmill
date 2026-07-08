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
    DatatableMigration,
}

pub const LATEST_GIT_SYNC_SCRIPT_PATH: &str = "hub/28786/sync-script-to-git-repo-windmill";

/// Hub script that applies a repository's state back into a workspace
/// (the repo → Windmill / "pull" direction). Same script the UI runs from
/// `PullWorkspaceModal` with `pull: true`; the slug's `:` is percent-encoded.
pub const GIT_SYNC_PULL_SCRIPT_PATH: &str = "hub/28785/git-sync%3A-init-repository-windmill";

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

/// Split a fork git branch `wm-fork/<base_branch>/<suffix>` into `(base_branch, suffix)`.
///
/// Inverse of the CLI/hub-script `forkBranchName`. The suffix is a workspace id
/// fragment and can't contain `/`, while the base branch may (`release/v2`), so the
/// split is on the last separator. Returns `None` for anything else.
pub fn parse_fork_branch(branch: &str) -> Option<(&str, &str)> {
    let rest = branch.strip_prefix("wm-fork/")?;
    let idx = rest.rfind('/')?;
    let (base, suffix) = (&rest[..idx], &rest[idx + 1..]);
    if base.is_empty() || suffix.is_empty() {
        return None;
    }
    Some((base, suffix))
}

/// Workspace ids that could own a fork branch with this suffix: a generated fork
/// (`wm-fork-<suffix>`, whose branch strips the id prefix) or a dev workspace
/// (prefix-less id used verbatim). Ordered generated-fork first so an ambiguous
/// suffix resolves deterministically.
pub fn fork_branch_workspace_id_candidates(suffix: &str) -> [String; 2] {
    [format!("{WM_FORK_PREFIX}{suffix}"), suffix.to_string()]
}

/// Git branch a dev workspace syncs with: its environment label verbatim, as a
/// first-class top-level branch (`dev`, `staging` — the classic env-branch
/// layout), defaulting to `dev` when the label is unset. Throwaway forks use
/// the namespaced `wm-fork/<base>/<suffix>` form instead.
pub fn dev_workspace_branch(label: Option<&str>) -> String {
    label.filter(|l| !l.is_empty()).unwrap_or("dev").to_string()
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

fn is_false(b: &bool) -> bool {
    !*b
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
    /// Configuration for automatically pulling changes from the git repository
    /// back into the workspace (repo → Windmill direction). Absent means the
    /// reverse direction is not automated (the historical behaviour).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_pull: Option<AutoPullSettings>,
    /// Open a PR when a deploy pushes a `wm_deploy/**` branch of this promotion
    /// repo (app-backed only; runs from the deploy callback so it works without
    /// inbound webhooks). Off by default so upgrades don't change behavior.
    #[serde(default, skip_serializing_if = "is_false")]
    pub promotion_open_prs: bool,
    /// Parent-level: open a PR when a fork of this workspace deploys to its
    /// `wm-fork/**` branch (app-backed only; the fork's deploy callback reads
    /// this from the parent). Off by default.
    #[serde(default, skip_serializing_if = "is_false")]
    pub fork_open_prs: bool,
    /// Server-owned: the last failure opening a PR for a deploy branch of this
    /// repo (e.g. the GitHub App installation hasn't approved the pull-request
    /// permission). Written by the deploy completion hook, cleared on the next
    /// successful PR; never accepted from clients.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_pr_error: Option<String>,
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

/// How auto-pull triggers are delivered for a repository.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AutoPullMode {
    /// Try to create a repo webhook; fall back to polling if the instance is not
    /// reachable from GitHub or the app lacks the webhook permission.
    #[default]
    Auto,
    /// Webhook delivery only (no polling fallback).
    Webhook,
    /// Polling only (`git ls-remote` on an interval).
    Polling,
}

/// Outcome of the most recent auto-pull attempt, surfaced in the UI.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AutoPullStatus {
    /// Commit sha the workspace was last synced to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced_sha: Option<String>,
    /// Unix timestamp (seconds) of the attempt.
    pub at: i64,
    /// Job id of the pull run, if one was enqueued.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<uuid::Uuid>,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Per-repository configuration for automatic repo → Windmill pull sync.
///
/// Stored inside `GitRepositorySettings` (workspace_settings.git_sync JSONB).
/// Webhook fields are populated in phase 2; phase 1 exercises the polling path
/// only, but the full shape is defined up front to avoid a second schema change.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AutoPullSettings {
    /// Default so a server-written status-only blob (fork workspaces, which never
    /// enable auto-pull themselves) parses even without the field.
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub mode: AutoPullMode,
    /// Polling interval in seconds. Defaults to `DEFAULT_AUTO_PULL_POLL_INTERVAL_S`
    /// when polling without an active webhook, relaxed once a webhook is live.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_interval_s: Option<u32>,
    /// Parent-level: also pull each live fork of this workspace from its own
    /// `wm-fork/<branch>/<id>` branch when that branch moves (webhook or poll),
    /// the managed equivalent of the `push-on-merge-to-forks` GitHub Action.
    /// Configured once on the parent; forks never enable auto-pull themselves.
    #[serde(default, skip_serializing_if = "is_false")]
    pub sync_forks: bool,
    /// GitHub repository webhook id (managed-app, phase 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_id: Option<i64>,
    /// HMAC secret for the repo webhook, encrypted at rest (managed-app, phase 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_secret: Option<String>,
    /// Why the repo has no active webhook while one was requested (auto/webhook
    /// mode): instance base URL unset, app missing the webhook permission, etc.
    /// Surfaced in the UI as a "falling back to polling" warning; `None` when the
    /// webhook is live or the repo is polling-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_error: Option<String>,
    /// Last synced commit sha per tracked git ref (e.g. `refs/heads/main`).
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub last_synced_sha: std::collections::HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_pull_status: Option<AutoPullStatus>,
}

// Manual Debug so the HMAC `webhook_secret` (even encrypted) never lands in logs.
impl std::fmt::Debug for AutoPullSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoPullSettings")
            .field("enabled", &self.enabled)
            .field("mode", &self.mode)
            .field("poll_interval_s", &self.poll_interval_s)
            .field("sync_forks", &self.sync_forks)
            .field("webhook_id", &self.webhook_id)
            .field(
                "webhook_secret",
                &self.webhook_secret.as_ref().map(|_| "<redacted>"),
            )
            .field("webhook_error", &self.webhook_error)
            .field("last_synced_sha", &self.last_synced_sha)
            .field("last_pull_status", &self.last_pull_status)
            .finish()
    }
}

/// Default polling interval when a webhook is not active.
pub const DEFAULT_AUTO_PULL_POLL_INTERVAL_S: u32 = 60;

/// Relaxed polling interval used as a safety net while a webhook is active.
pub const WEBHOOK_AUTO_PULL_POLL_INTERVAL_S: u32 = 600;

impl AutoPullSettings {
    /// Effective polling interval in seconds, honouring the explicit override and
    /// relaxing to `WEBHOOK_AUTO_PULL_POLL_INTERVAL_S` when a webhook is live.
    pub fn effective_poll_interval_s(&self) -> u32 {
        self.poll_interval_s.unwrap_or_else(|| {
            if self.webhook_id.is_some() {
                WEBHOOK_AUTO_PULL_POLL_INTERVAL_S
            } else {
                DEFAULT_AUTO_PULL_POLL_INTERVAL_S
            }
        })
    }

    /// Whether a freshly observed `(git_ref, head_sha)` warrants enqueuing a pull.
    ///
    /// A trigger (poll or webhook) is only a hint: we pull when auto-pull is
    /// enabled and the observed head differs from the last sha we synced for
    /// that ref. Re-observing the same head (e.g. a redundant poll, or the
    /// commit our own deploy callback just pushed back) is a no-op.
    pub fn should_pull(&self, git_ref: &str, head_sha: &str) -> bool {
        self.enabled && self.last_synced_sha.get(git_ref).map(String::as_str) != Some(head_sha)
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
    /// Whether the SQL-migrations feature is opted in for this data table.
    /// Absent on data tables created before the feature: treated as enabled only
    /// when migrations already exist (see `datatable_migrations_enabled`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub migrations_enabled: Option<bool>,
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

/// Build a self-teaching error for an unresolved `datatable://<name>` reference.
/// The raw "not found" gives the user no way forward — the datatable substrate has
/// no auto-provisioning (unlike a DuckLake catalog), so the fix is always to create
/// one in workspace settings. Surface the available names (to catch typos) and point
/// at the settings page so the message is actionable wherever it bubbles up
/// (pipeline `ATTACH`, schema fetch, postgres executor, ...).
fn datatable_not_found_error(name: &str, datatables: Option<&serde_json::Value>) -> Error {
    let available: Vec<&str> = datatables
        .and_then(|d| d.as_object())
        .map(|o| o.keys().map(String::as_str).collect())
        .unwrap_or_default();

    let hint = if available.is_empty() {
        "No data table is configured in this workspace yet.".to_string()
    } else {
        format!("Configured data tables: {}.", available.join(", "))
    };

    Error::NotFound(format!(
        "Data table '{name}' not found. {hint} \
         Create one in workspace settings under the \"Data tables\" tab \
         (/workspace_settings?tab=windmill_data_tables) — the name \"main\" is the default \
         used by `datatable://main`."
    ))
}

pub async fn get_datatable_resource_from_db_unchecked(
    db: &DB,
    w_id: &str,
    name: &str,
) -> Result<serde_json::Value> {
    let datatables = sqlx::query_scalar!(
        r#"
            SELECT ws.datatable->'datatables' AS datatables
            FROM workspace_settings ws
            WHERE ws.workspace_id = $1
        "#,
        &w_id,
    )
    .fetch_one(db)
    .await
    .map_err(|err| Error::internal_err(format!("getting datatable {name}: {err}")))?;

    let datatable = datatables
        .as_ref()
        .and_then(|d| d.get(name))
        .filter(|v| !v.is_null())
        .ok_or_else(|| datatable_not_found_error(name, datatables.as_ref()))?;
    let datatable = serde_json::from_value::<DataTable>(datatable.clone())?;

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
    /// How this lake behaves when the workspace is a fork/dev workspace. Only meaningful in a
    /// fork's own settings; stamped at fork creation from the user's per-lake choice. Absent =
    /// `Isolated` — the safe default, so forks created before this field existed (and API
    /// callers that omit it) never write the parent's lake.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_behavior: Option<DucklakeForkBehavior>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<DucklakeMaintenance>,
}

/// Per-lake fork data-environment choice, made at fork creation.
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DucklakeForkBehavior {
    /// Fork-scoped namespace + read-defer to the parent (default).
    Isolated,
    /// The fork reads AND WRITES the parent's lake directly — explicit opt-out of isolation
    /// (e.g. a fork meant to run prod-equivalent backfills).
    Shared,
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
///
/// Accepted limitation: a schedule that pre-dated this namespace under a real
/// `ducklake_maintenance` folder keeps running (tick dispatch falls through
/// to its script when its path's lake has no enabled maintenance config, and
/// the settings sync only touches rows derived from config) but stays hidden
/// from list/export and immutable via the schedule API until renamed out of
/// the namespace. Judged unlikely enough to not warrant a discriminator
/// column or a rename migration.
pub const DUCKLAKE_MAINTENANCE_PATH_PREFIX: &str = "f/ducklake_maintenance/";

pub fn ducklake_maintenance_schedule_path(lake: &str) -> String {
    format!("{DUCKLAKE_MAINTENANCE_PATH_PREFIX}{lake}")
}

pub fn lake_from_ducklake_maintenance_path(path: &str) -> Option<&str> {
    path.strip_prefix(DUCKLAKE_MAINTENANCE_PATH_PREFIX)
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
    /// Present when the resolved workspace is a fork/dev workspace. Carries the read-defer
    /// context (ancestor namespaces + tables to expose as views over the direct parent). The
    /// fork *write* redirect is folded into `storage.path`/`extra_args` above, so an agent
    /// worker that predates this field still writes the fork namespace (it only misses the
    /// defer views).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_defer: Option<DucklakeForkDefer>,
}

/// Read-defer context for a fork workspace's ducklake: which ancestor namespaces to attach
/// read-only, and which tables to expose as views over the direct parent because the fork has
/// not materialized them yet. `ancestors` is empty when defer is unavailable (an ancestor no
/// longer defines the lake) — the fork namespace still isolates writes in that case.
#[derive(Deserialize, Serialize)]
pub struct DucklakeForkDefer {
    /// Nearest-first (direct parent … root), each resolved from that workspace's own settings.
    pub ancestors: Vec<DucklakeAncestorAttach>,
    pub defer_tables: Vec<crate::materialization::ForkDeferTable>,
    /// Views currently live in the fork namespace (read from the catalog's `ducklake_view` at
    /// resolution time, lake-internal names). The worker's view→table transition (DROP VIEW
    /// before a managed materialize) keys on THIS, not on recorded materialization status:
    /// after a failed run the status can't distinguish a defer view from a real table, and
    /// `DROP VIEW` against a table (or `CREATE TABLE` against a view) errors — either guess
    /// would wedge the asset until manual repair.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fork_views: Vec<String>,
}

/// Connection data for one ancestor namespace of a fork's ducklake.
#[derive(Deserialize, Serialize)]
pub struct DucklakeAncestorAttach {
    pub workspace_id: String,
    /// DuckDB catalog alias this namespace must be attached under. Persisted defer-view SQL
    /// references it, so it is a pure function of (lake name, ancestor workspace id) and every
    /// session reading those views attaches the ancestor under this exact alias.
    pub alias: String,
    pub catalog: DucklakeCatalog,
    pub catalog_resource: serde_json::Value,
    pub storage: DucklakeStorage,
    /// None = the lake's default metadata schema (the ancestor is a root/non-fork workspace).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_schema: Option<String>,
    /// The ancestor config's own non-reserved ATTACH args (e.g. `ENCRYPTED true`), already
    /// stripped of the fork-owned `METADATA_SCHEMA`/`DATA_PATH`/`OVERRIDE_DATA_PATH` — an
    /// option-dependent lake would otherwise fail its read-only ancestor attach even though
    /// the same lake attaches fine everywhere else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<String>,
}

/// Prefix of fork-scoped ducklake metadata schemas. Cleanup refuses to drop any pg schema not
/// carrying it, mirroring the `wm_fork_` guard on forked datatable databases.
pub const FORK_DUCKLAKE_SCHEMA_PREFIX: &str = "wm_fork_";

/// Bucket-root directory holding all fork namespaces' data files: each fork writes under
/// `{FORK_DUCKLAKE_DATA_DIR}/<fork segment>/<lake path>` where the segment is
/// [`fork_data_dir_segment`] (see [`fork_data_path`] for why it wraps the lake's path instead
/// of nesting under it).
pub const FORK_DUCKLAKE_DATA_DIR: &str = "__wm_forks";

fn mangle_identifier(s: &str, max: usize) -> String {
    s.chars()
        .take(max)
        .map(|c| {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_alphanumeric() {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Deterministic, injective pg-schema name for a fork workspace's namespace of ONE lake:
/// `wm_fork_<mangled wid (≤24)>_<mangled lake (≤16)>_<8-hex sha256>`, ≤58 chars (pg limit 63).
/// Lake-scoped, not just workspace-scoped: two lakes of one workspace may share a catalog
/// database, and a per-workspace schema would merge their namespaces in the fork (tables and
/// snapshots colliding across `ducklake://a/…` and `ducklake://b/…`). The hash keeps distinct
/// (workspace, lake) pairs distinct after mangling. The registry row records the computed name
/// for cleanup, but ATTACH recomputes it — so this function must stay stable across releases
/// or existing forks would silently lose their namespace.
pub fn fork_ducklake_metadata_schema(w_id: &str, lake_name: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = hex::encode(&Sha256::digest(format!("{w_id}\0{lake_name}").as_bytes())[..4]);
    format!(
        "{FORK_DUCKLAKE_SCHEMA_PREFIX}{}_{}_{hash}",
        mangle_identifier(w_id, 24),
        mangle_identifier(lake_name, 16)
    )
}

/// The `METADATA_SCHEMA '<x>'` value carried in a lake config's `extra_args`, if any — the
/// schema the lake's OWN catalog namespace lives in (how one catalog database hosts several
/// lakes). Ancestor read-only attaches must preserve it or they'd bind the wrong namespace.
pub fn extract_metadata_schema_arg(extra_args: &str) -> Option<String> {
    lazy_static::lazy_static! {
        static ref MS: regex::Regex = regex::Regex::new(
            r"(?i)\bMETADATA_SCHEMA\s*(?:'([^']*)'|([A-Za-z0-9_]+))"
        )
        .unwrap();
    }
    // Last occurrence wins, matching DuckDB's duplicate-option semantics.
    MS.captures_iter(extra_args)
        .last()
        .and_then(|c| c.get(1).or_else(|| c.get(2)))
        .map(|m| m.as_str().to_string())
}

/// Deterministic DuckDB attach alias for an ancestor namespace of a fork's lake. Persisted
/// defer-view SQL references it (same stability requirement as
/// [`fork_ducklake_metadata_schema`]).
pub fn fork_ducklake_ancestor_alias(lake_name: &str, ancestor_w_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash =
        hex::encode(&Sha256::digest(format!("{lake_name}\0{ancestor_w_id}").as_bytes())[..4]);
    format!(
        "__wm_dl_{}_{}_{hash}",
        mangle_identifier(lake_name, 20),
        mangle_identifier(ancestor_w_id, 20)
    )
}

/// Strip `METADATA_SCHEMA` / `DATA_PATH` / `OVERRIDE_DATA_PATH` tokens from ducklake ATTACH
/// extra args. In a fork these options are injected by the fork resolution; a user- or
/// settings-supplied duplicate silently wins (DuckDB keeps the last occurrence) and would
/// escape the fork namespace back to the parent's, so they are removed rather than overridden.
pub fn strip_fork_reserved_attach_args(extra_args: &str) -> String {
    lazy_static::lazy_static! {
        static ref RESERVED: regex::Regex = regex::Regex::new(
            r"(?i)\b(METADATA_SCHEMA|DATA_PATH|OVERRIDE_DATA_PATH)\s*('[^']*'|[A-Za-z0-9_]+)"
        )
        .unwrap();
    }
    RESERVED
        .replace_all(extra_args, "")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

lazy_static::lazy_static! {
    /// fork workspace id -> (ancestor chain nearest-first, expiry ts). Empty chain = not a fork.
    /// `parent_workspace_id` only changes on dev-workspace attach/detach, so a short TTL is safe.
    static ref FORK_ANCESTOR_CHAIN_CACHE: Cache<String, (Vec<String>, i64)> = Cache::new(5000);
    /// fork workspace id -> (locations recently upserted into `fork_ducklake_namespace`,
    /// expiry ts), so the registry write doesn't run on every job of a fork. Keyed by
    /// workspace id so cleanup can drop a fork's whole entry: a same-id fork recreated within
    /// the TTL must re-register, or its materializations would carry no registry row and leak
    /// at ITS deletion. TTL'd as a second line of defense for cleanup paths that bypass
    /// `cleanup_fork_ducklake_namespaces` (e.g. manual registry edits).
    static ref FORK_DUCKLAKE_REGISTERED: Cache<String, (std::collections::HashSet<String>, i64)> =
        Cache::new(5000);
}

/// Drop the "already registered" once-cache for a workspace. MUST be called whenever
/// `fork_ducklake_namespace` rows for that workspace are deleted (namespace cleanup, workspace
/// deletion): a surviving entry would make a same-id fork recreated within the TTL skip
/// re-registration, orphaning its namespace at deletion time.
pub fn invalidate_fork_ducklake_registration_cache(w_id: &str) {
    FORK_DUCKLAKE_REGISTERED.remove(w_id);
}

/// Drop the cached ancestor chain for a workspace. MUST be called wherever
/// `parent_workspace_id` lineage changes (fork creation, dev-workspace attach, reparenting
/// rename, deletion): a cached EMPTY chain reads as "not a fork" and would bypass ducklake
/// fork isolation for the TTL — the first jobs after a dev-workspace attach would write the
/// shared lake.
pub fn invalidate_fork_ancestor_chain_cache(w_id: &str) {
    FORK_ANCESTOR_CHAIN_CACHE.remove(w_id);
}

/// Ancestors of `w_id`, nearest-first (direct parent … root). Empty for a non-fork workspace or
/// an unknown id. The depth bound is a cycle-safety backstop, same convention as
/// [`fork_chain_depth`].
///
/// Unauthenticated helper: reads workspace hierarchy for any `w_id`, so callers must already be
/// authorized for that workspace (or run in trusted server-side code).
pub async fn fork_ancestor_chain(db: &crate::DB, w_id: &str) -> Result<Vec<String>> {
    let now = chrono::Utc::now().timestamp();
    if let Some((chain, expiry)) = FORK_ANCESTOR_CHAIN_CACHE.get(w_id) {
        if expiry > now {
            return Ok(chain);
        }
    }
    let chain = sqlx::query_scalar!(
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
            SELECT id AS "id!" FROM chain WHERE depth > 0 ORDER BY depth
        "#,
        w_id
    )
    .fetch_all(db)
    .await
    .map_err(|e| Error::internal_err(format!("resolving fork ancestors of {w_id}: {e:#}")))?;
    FORK_ANCESTOR_CHAIN_CACHE.insert(w_id.to_string(), (chain.clone(), now + 60));
    Ok(chain)
}

pub async fn get_ducklake_from_db_unchecked(
    name: &str,
    w_id: &str,
    db: &DB,
) -> Result<DucklakeWithConnData> {
    let (base, fork_behavior) = ducklake_conn_data(name, w_id, db).await?;
    let chain = fork_ancestor_chain(db, w_id).await?;
    // `Shared` is the explicit fork-creation opt-out of isolation: the fork reads and writes
    // the parent's lake through its own (cloned) config, exactly like a non-fork workspace.
    // An empty chain alone does NOT mean "not a fork": `parent_workspace_id` is ON DELETE SET
    // NULL, so a `wm-fork-*` workspace can outlive its parent — its cloned config still points
    // at the shared lake, so the prefix keeps it isolated (mirrors `workspace_is_fork`). No
    // ancestors ⇒ no defer, write redirect only. Prefix-less dev workspaces can't be orphaned
    // (deletion of their prod is blocked while attached).
    if (chain.is_empty() && !w_id.starts_with(WM_FORK_PREFIX))
        || fork_behavior == Some(DucklakeForkBehavior::Shared)
    {
        return Ok(base);
    }
    fork_scoped_ducklake(name, w_id, base, chain, db).await
}

/// Resolve one workspace's own config for lake `name` — no fork awareness.
async fn ducklake_conn_data(
    name: &str,
    w_id: &str,
    db: &DB,
) -> Result<(DucklakeWithConnData, Option<DucklakeForkBehavior>)> {
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
    let fork_behavior = ducklake.fork_behavior;
    let ducklake = DucklakeWithConnData {
        catalog_resource,
        catalog: ducklake.catalog,
        storage: ducklake.storage,
        extra_args: ducklake.extra_args,
        fork_defer: None,
    };
    Ok((ducklake, fork_behavior))
}

/// The fork's directory segment under `__wm_forks/`: mangled + hashed into ONE path component
/// (same recipe and stability requirement as [`fork_ducklake_metadata_schema`]). Fork/dev
/// workspace ids are only git-branch-safe and may contain `/` (e.g. `wm-fork-a/b`) — used
/// raw, such an id would nest inside the sibling `wm-fork-a`'s prefix and be swept by ITS
/// cleanup. The hash keeps distinct ids distinct after mangling.
pub fn fork_data_dir_segment(fork_w_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = hex::encode(&Sha256::digest(fork_w_id.as_bytes())[..4]);
    format!("{}_{hash}", mangle_identifier(fork_w_id, 40))
}

/// The fork namespace's data path within the same bucket: a bucket-root
/// `__wm_forks/<fork segment>/` prefix wrapping the lake's own path, NOT a sub-path under it —
/// the parent lake's maintenance (snapshot expiry / `ducklake_delete_orphaned_files`) scans
/// everything under the parent's DATA_PATH and would treat live fork files nested there as
/// orphans and delete them.
pub fn fork_data_path(base_path: &str, fork_w_id: &str) -> String {
    let segment = fork_data_dir_segment(fork_w_id);
    let base = base_path.trim_matches('/');
    if base.is_empty() {
        format!("{FORK_DUCKLAKE_DATA_DIR}/{segment}")
    } else {
        format!("{FORK_DUCKLAKE_DATA_DIR}/{segment}/{base}")
    }
}

/// Redirect a fork workspace's lake to its fork-scoped namespace (same catalog DB, fork
/// metadata schema + data sub-path) and assemble the read-defer context over its ancestors.
async fn fork_scoped_ducklake(
    name: &str,
    w_id: &str,
    mut base: DucklakeWithConnData,
    chain: Vec<String>,
    db: &DB,
) -> Result<DucklakeWithConnData> {
    if base.catalog.resource_type == DucklakeCatalogResourceType::Mysql {
        return Err(Error::BadRequest(format!(
            "ducklake {name}: mysql-catalog lakes are not supported in fork workspaces — \
             running against the shared catalog would write the parent workspace's data"
        )));
    }
    let metadata_schema = fork_ducklake_metadata_schema(w_id, name);
    let fork_path = fork_data_path(&base.storage.path, w_id);
    let catalog_identity = ducklake_catalog_identity(&base.catalog);
    register_fork_ducklake_namespace(
        db,
        w_id,
        name,
        &metadata_schema,
        &catalog_identity,
        base.storage.storage.as_deref(),
        &fork_path,
    )
    .await?;

    // Ancestor namespaces, nearest-first, each from its own settings so a fork-side settings
    // edit can't silently repoint what "parent" means. All-or-nothing: a broken link anywhere
    // in the chain disables defer entirely (a defer view over a missing ancestor attach fails
    // at bind time and would kill unrelated jobs), but write isolation still applies.
    let mut ancestors = Vec::with_capacity(chain.len());
    for (i, ancestor_id) in chain.iter().enumerate() {
        match ducklake_conn_data(name, ancestor_id, db).await {
            Ok((mut a, ancestor_behavior)) => {
                // A fork ancestor lives in its own namespace UNLESS its lake is `Shared` —
                // then it never redirected and its data sits at its config's default
                // location, exactly like a root workspace. Chain position alone can't tell
                // the two apart: an orphaned `wm-fork-*` ancestor (its own parent deleted,
                // `parent_workspace_id` SET NULL) ends the chain like a root but its data
                // lives in ITS fork namespace — key on the prefix too, as in resolution.
                let is_isolated_fork = (i + 1 < chain.len()
                    || ancestor_id.starts_with(WM_FORK_PREFIX))
                    && ancestor_behavior != Some(DucklakeForkBehavior::Shared);
                let metadata_schema = if is_isolated_fork {
                    a.storage.path = fork_data_path(&a.storage.path, ancestor_id);
                    Some(fork_ducklake_metadata_schema(ancestor_id, name))
                } else {
                    // Root / shared ancestors live in their config's OWN catalog namespace —
                    // which may be a non-default schema when one catalog database hosts
                    // several lakes (`extra_args METADATA_SCHEMA '…'`). Preserve it, or the
                    // read-only attach would bind the wrong (or a nonexistent) lake.
                    a.extra_args
                        .as_deref()
                        .and_then(extract_metadata_schema_arg)
                };
                let extra_args = a
                    .extra_args
                    .as_deref()
                    .map(strip_fork_reserved_attach_args)
                    .filter(|s| !s.is_empty());
                ancestors.push(DucklakeAncestorAttach {
                    workspace_id: ancestor_id.clone(),
                    alias: fork_ducklake_ancestor_alias(name, ancestor_id),
                    catalog: a.catalog,
                    catalog_resource: a.catalog_resource,
                    storage: a.storage,
                    metadata_schema,
                    extra_args,
                });
            }
            Err(e) => {
                tracing::warn!(
                    "fork {w_id}: ducklake {name} not resolvable in ancestor {ancestor_id} \
                     ({e:#}); read-defer disabled, fork namespace still isolated"
                );
                ancestors.clear();
                break;
            }
        }
    }
    // Drop fork ancestors whose namespace was never bootstrapped (e.g. a fresh intermediate
    // fork that never attached this lake): their READ_ONLY attach would fail the whole job
    // ("DuckLake does not exist" + creation disabled), and a nonexistent namespace can't own
    // tables or be referenced by any persisted view. Checked against the fork's catalog DB —
    // must happen BEFORE defer discovery so `ancestor_idx` values index the filtered list.
    let (existing_schemas, fork_views, fork_tables) =
        inspect_fork_catalog(&base, &metadata_schema, &ancestors, db).await?;
    ancestors.retain(|a| {
        a.metadata_schema
            .as_ref()
            .map_or(true, |s| existing_schemas.contains(s))
    });
    let defer_tables = if ancestors.is_empty() {
        vec![]
    } else {
        // Discovery walks the WHOLE chain: a table only a grandparent materialized has no
        // physical copy in the direct parent (it defers too), so its view must target the
        // nearest ancestor that owns one.
        let ancestor_ids: Vec<String> = ancestors.iter().map(|a| a.workspace_id.clone()).collect();
        let mut tables =
            crate::materialization::list_fork_defer_tables(db, &ancestor_ids, w_id, name).await?;
        // Catalog truth beats recorded status: any table live in the fork namespace is
        // fork-owned, whatever its materialized_partition row says (failed-after-commit,
        // raw SQL) — a defer view over it would silently yield to it.
        tables.retain(|t| !fork_tables.contains(&t.table));
        tables
    };

    base.storage.path = fork_path;
    base.extra_args = Some(match base.extra_args.take() {
        Some(e) => {
            let sanitized = strip_fork_reserved_attach_args(&e);
            if sanitized.is_empty() {
                format!("METADATA_SCHEMA '{metadata_schema}'")
            } else {
                format!("{sanitized}, METADATA_SCHEMA '{metadata_schema}'")
            }
        }
        None => format!("METADATA_SCHEMA '{metadata_schema}'"),
    });
    base.fork_defer = Some(DucklakeForkDefer { ancestors, defer_tables, fork_views });
    Ok(base)
}

/// One round trip to the fork's catalog DB for both namespace introspections: which fork
/// ancestors' metadata schemas actually exist (a fresh intermediate fork may never have
/// bootstrapped its namespace), and the views currently live in THIS fork's namespace
/// (straight from DuckLake's `ducklake_view` metadata — missing metadata tables read as "no
/// views", correct since nothing can exist there). Each ancestor's schema is checked in the
/// ancestor's OWN catalog database — a fork whose catalog drifted away from an ancestor's
/// must not misread that ancestor's (existing, elsewhere) namespace as missing. Ancestors
/// whose catalog is unreachable read as missing — the safe direction: their defer is skipped
/// (loud absent-table reads) rather than emitting a READ_ONLY attach that would fail every
/// job in this fork.
async fn inspect_fork_catalog(
    base: &DucklakeWithConnData,
    metadata_schema: &str,
    ancestors: &[DucklakeAncestorAttach],
    db: &DB,
) -> Result<(
    std::collections::HashSet<String>,
    Vec<String>,
    std::collections::HashSet<String>,
)> {
    // Ancestor fork-schemas grouped by which catalog database they live in.
    let mut groups: std::collections::HashMap<String, (serde_json::Value, Vec<String>)> =
        std::collections::HashMap::new();
    for a in ancestors {
        if let Some(ms) = &a.metadata_schema {
            groups
                .entry(ducklake_catalog_identity(&a.catalog))
                .or_insert_with(|| (a.catalog_resource.clone(), vec![]))
                .1
                .push(ms.clone());
        }
    }

    async fn query_schemas(
        client: &tokio_postgres::Client,
        schemas: Vec<String>,
    ) -> std::result::Result<Vec<String>, tokio_postgres::Error> {
        client
            .query(
                "SELECT schema_name::text FROM information_schema.schemata
                 WHERE schema_name = ANY($1)",
                &[&schemas],
            )
            .await
            .map(|rows| rows.iter().map(|r| r.get::<_, String>(0)).collect())
    }

    let mut existing_schemas: std::collections::HashSet<String> = Default::default();

    // The fork's own catalog: same-catalog ancestor schemas + this fork's live views.
    let pg: crate::PgDatabase =
        serde_json::from_value(base.catalog_resource.clone()).map_err(|e| {
            Error::internal_err(format!("ducklake catalog resource is not postgres: {e}"))
        })?;
    let (client, connection) = pg.connect(Some(db)).await?;
    let join_handle = tokio::spawn(async move { connection.await });
    let same_catalog = groups.remove(&ducklake_catalog_identity(&base.catalog));
    let same_catalog_res = match same_catalog {
        Some((_, schemas)) => query_schemas(&client, schemas).await.map(Some),
        None => Ok(None),
    };
    // Identifier-quoted schema: the name is server-derived (mangled + hashed) but quote anyway.
    let q = format!(
        r#"SELECT CASE WHEN s.schema_name = 'main' THEN v.view_name
                       ELSE s.schema_name || '.' || v.view_name END AS name
           FROM "{ms}".ducklake_view v
           JOIN "{ms}".ducklake_schema s ON s.schema_id = v.schema_id AND s.end_snapshot IS NULL
           WHERE v.end_snapshot IS NULL"#,
        ms = metadata_schema.replace('"', "\"\"")
    );
    let view_rows = client.query(&q, &[]).await;
    // Live TABLES in the fork's namespace, same name shape as the views — the defer list is
    // filtered against them: `CREATE VIEW IF NOT EXISTS` silently yields to an existing
    // table, so emitting a defer view over one would leave reads on the fork table while
    // claiming they defer (recorded status can't tell — a committed write whose data tests
    // failed, or a table left by raw SQL, has no `materialized` row).
    let qt = format!(
        r#"SELECT CASE WHEN s.schema_name = 'main' THEN t.table_name
                       ELSE s.schema_name || '.' || t.table_name END AS name
           FROM "{ms}".ducklake_table t
           JOIN "{ms}".ducklake_schema s ON s.schema_id = t.schema_id AND s.end_snapshot IS NULL
           WHERE t.end_snapshot IS NULL"#,
        ms = metadata_schema.replace('"', "\"\"")
    );
    let table_rows = client.query(&qt, &[]).await;
    drop(client);
    let _ = join_handle.await;

    existing_schemas.extend(
        same_catalog_res
            .map_err(|e| {
                Error::internal_err(format!("checking fork ducklake ancestor schemas: {e}"))
            })?
            .unwrap_or_default(),
    );
    let fork_views = match view_rows {
        Ok(rows) => rows.iter().map(|r| r.get::<_, String>(0)).collect(),
        // 42P01 undefined_table / 3F000 invalid_schema_name: namespace not bootstrapped yet.
        Err(e)
            if e.code().map_or(false, |c| {
                c == &tokio_postgres::error::SqlState::UNDEFINED_TABLE
                    || c == &tokio_postgres::error::SqlState::INVALID_SCHEMA_NAME
            }) =>
        {
            vec![]
        }
        Err(e) => {
            return Err(Error::internal_err(format!(
                "listing fork ducklake views in {metadata_schema}: {e}"
            )))
        }
    };
    let fork_tables = match table_rows {
        Ok(rows) => rows.iter().map(|r| r.get::<_, String>(0)).collect(),
        Err(e)
            if e.code().map_or(false, |c| {
                c == &tokio_postgres::error::SqlState::UNDEFINED_TABLE
                    || c == &tokio_postgres::error::SqlState::INVALID_SCHEMA_NAME
            }) =>
        {
            Default::default()
        }
        Err(e) => {
            return Err(Error::internal_err(format!(
                "listing fork ducklake tables in {metadata_schema}: {e}"
            )))
        }
    };

    // Ancestors living in OTHER catalog databases (this fork's catalog drifted after forking):
    // one connection per distinct catalog, best-effort — an unreachable ancestor catalog only
    // disables that ancestor's defer.
    for (identity, (resource, schemas)) in groups {
        let checked = async {
            let pg: crate::PgDatabase = serde_json::from_value(resource)
                .map_err(|e| Error::internal_err(format!("not a postgres resource: {e}")))?;
            let (client, connection) = pg.connect(Some(db)).await?;
            let join_handle = tokio::spawn(async move { connection.await });
            let res = query_schemas(&client, schemas).await;
            drop(client);
            let _ = join_handle.await;
            res.map_err(|e| Error::internal_err(format!("{e}")))
        }
        .await;
        match checked {
            Ok(found) => existing_schemas.extend(found),
            Err(e) => {
                tracing::warn!(
                    "fork ancestor catalog `{identity}` unreachable while checking ducklake \
                     namespaces ({e:#}); its ancestors' defer is disabled for this resolution"
                );
            }
        }
    }
    Ok((existing_schemas, fork_views, fork_tables))
}

/// Canonical identity of a lake's catalog database (`<resource_type>:<resource_path>`) — used
/// for the cleanup registry and for grouping ancestors by which catalog their namespace lives
/// in. Identifies the database *pointer*, not its (live-resolved) credentials.
pub fn ducklake_catalog_identity(catalog: &DucklakeCatalog) -> String {
    format!(
        "{}:{}",
        catalog.resource_type.as_ref(),
        catalog.resource_path
    )
}

/// Record that a fork attached this lake at this physical location, so fork deletion knows
/// exactly which pg metadata schema and which storage prefixes to clean up. One row per
/// (lake, storage, data path) EVER attached — if the fork's lake settings drift, later
/// attaches add rows rather than replace them, so cleanup covers every prefix the fork wrote.
/// The once-cache is keyed on the full location for the same reason. Re-registering an
/// existing row resets its `schema_dropped` cleanup phase: attaching recreates the metadata
/// schema, so a stale "already dropped" marker would make the eventual cleanup skip a live
/// schema.
async fn register_fork_ducklake_namespace(
    db: &DB,
    w_id: &str,
    name: &str,
    metadata_schema: &str,
    catalog: &str,
    storage: Option<&str>,
    data_path: &str,
) -> Result<()> {
    // '' = default storage (the column is part of the PK, which cannot hold NULL).
    let storage = storage.unwrap_or("");
    // Resolve the logical storage name to its identity NOW: cleanup must delete from the
    // storage that was active when the data was written, not whatever the name points at by
    // deletion time. '' = unresolvable (no LFS configured — the write itself will fail at the
    // proxy, so nothing lands anywhere).
    let storage_ref = fork_storage_ref(db, w_id, storage)
        .await
        .unwrap_or_default();
    let location = format!("{name}\0{catalog}\0{storage}\0{storage_ref}\0{data_path}");
    let now = chrono::Utc::now().timestamp();
    if FORK_DUCKLAKE_REGISTERED
        .get(w_id)
        .is_some_and(|(locations, exp)| exp > now && locations.contains(&location))
    {
        return Ok(());
    }
    sqlx::query!(
        "INSERT INTO fork_ducklake_namespace
           (workspace_id, ducklake_name, metadata_schema, catalog, storage, storage_ref, data_path)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (workspace_id, ducklake_name, catalog, storage, storage_ref, data_path)
         DO UPDATE SET schema_dropped = false",
        w_id,
        name,
        metadata_schema,
        catalog,
        storage,
        &storage_ref,
        data_path,
    )
    .execute(db)
    .await
    .map_err(|e| Error::internal_err(format!("registering fork ducklake namespace: {e:#}")))?;
    let mut locations = FORK_DUCKLAKE_REGISTERED
        .get(w_id)
        .filter(|(_, exp)| *exp > now)
        .map(|(locations, _)| locations)
        .unwrap_or_default();
    locations.insert(location);
    FORK_DUCKLAKE_REGISTERED.insert(w_id.to_string(), (locations, now + 60));
    Ok(())
}

/// Canonical identity of a workspace storage (`<lfs type>:<resource path or root path>`) as
/// configured RIGHT NOW — recorded in the fork namespace registry so cleanup targets the
/// storage the data was actually written to. `storage` = '' for the primary storage, else a
/// `secondary_storage` name. Returns None when no matching storage is configured.
async fn fork_storage_ref(db: &DB, w_id: &str, storage: &str) -> Option<String> {
    let lfs_json = sqlx::query_scalar!(
        "SELECT large_file_storage FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .flatten()?;
    let entry = if storage.is_empty() || storage == "_default_" {
        lfs_json.clone()
    } else {
        lfs_json.get("secondary_storage")?.get(storage)?.clone()
    };
    lfs_entry_storage_ref(&entry)
}

/// The pure part of [`fork_storage_ref`]: one `LargeFileStorage` JSON entry → its canonical
/// `<type>:<path>` descriptor. The `$res:` prefix on resource paths is normalized away (stored
/// configs are inconsistent about it); cleanup re-adds it when resolving.
pub fn lfs_entry_storage_ref(entry: &serde_json::Value) -> Option<String> {
    let typ = entry.get("type")?.as_str()?;
    let path_field = match typ {
        "S3Storage" | "S3AwsOidc" => "s3_resource_path",
        "AzureBlobStorage" | "AzureWorkloadIdentity" => "azure_blob_resource_path",
        "GoogleCloudStorage" => "gcs_resource_path",
        "FilesystemStorage" => "root_path",
        _ => return None,
    };
    let path = entry.get(path_field)?.as_str()?;
    let path = path.strip_prefix("$res:").unwrap_or(path);
    Some(format!("{typ}:{path}"))
}

/// Resolve a `$res:`/`$var:` reference tree to its concrete value (recursively, secrets
/// decrypted). No permission checks — trusted server-side callers only; never echo the result
/// to a user.
pub async fn transform_json_value_unchecked(
    value: &serde_json::Value,
    w_id: &str,
    db: &DB,
) -> Result<serde_json::Value> {
    transform_json_unchecked(value, w_id, db).await
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
    fn test_parse_fork_branch() {
        // Generated fork (`wm-fork-abc`) and dev workspace (`staging`) forms.
        assert_eq!(parse_fork_branch("wm-fork/main/abc"), Some(("main", "abc")));
        assert_eq!(
            parse_fork_branch("wm-fork/main/staging"),
            Some(("main", "staging"))
        );
        // Base branch may itself contain slashes; the suffix never does.
        assert_eq!(
            parse_fork_branch("wm-fork/release/v2/abc"),
            Some(("release/v2", "abc"))
        );
        assert_eq!(parse_fork_branch("main"), None);
        assert_eq!(parse_fork_branch("wm-fork/main"), None);
        assert_eq!(parse_fork_branch("wm-fork/main/"), None);
        assert_eq!(parse_fork_branch("wm-fork//abc"), None);
        assert_eq!(parse_fork_branch("wm_deploy/main/abc"), None);
    }

    #[test]
    fn test_fork_branch_workspace_id_candidates_prefers_generated_fork() {
        let [first, second] = fork_branch_workspace_id_candidates("abc");
        assert_eq!(first, "wm-fork-abc");
        assert_eq!(second, "abc");
    }

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

    fn auto_pull(enabled: bool, synced: &[(&str, &str)]) -> AutoPullSettings {
        AutoPullSettings {
            enabled,
            mode: AutoPullMode::Auto,
            poll_interval_s: None,
            sync_forks: false,
            webhook_id: None,
            webhook_secret: None,
            webhook_error: None,
            last_synced_sha: synced
                .iter()
                .map(|(r, s)| (r.to_string(), s.to_string()))
                .collect(),
            last_pull_status: None,
        }
    }

    #[test]
    fn test_should_pull_on_new_or_changed_sha() {
        let s = auto_pull(true, &[("refs/heads/main", "aaa")]);
        // unchanged head → no pull
        assert!(!s.should_pull("refs/heads/main", "aaa"));
        // moved head → pull
        assert!(s.should_pull("refs/heads/main", "bbb"));
        // never-seen ref → pull
        assert!(s.should_pull("refs/heads/dev", "ccc"));
    }

    #[test]
    fn test_should_pull_respects_enabled_flag() {
        let s = auto_pull(false, &[]);
        assert!(!s.should_pull("refs/heads/main", "bbb"));
    }

    #[test]
    fn test_effective_poll_interval() {
        let mut s = auto_pull(true, &[]);
        assert_eq!(
            s.effective_poll_interval_s(),
            DEFAULT_AUTO_PULL_POLL_INTERVAL_S
        );
        // explicit override wins
        s.poll_interval_s = Some(15);
        assert_eq!(s.effective_poll_interval_s(), 15);
        // with a live webhook and no override, relax to the webhook interval
        s.poll_interval_s = None;
        s.webhook_id = Some(42);
        assert_eq!(
            s.effective_poll_interval_s(),
            WEBHOOK_AUTO_PULL_POLL_INTERVAL_S
        );
    }

    #[test]
    fn test_fork_ducklake_metadata_schema_shape() {
        let s = fork_ducklake_metadata_schema("wm-fork-my-feature-42", "main");
        assert!(s.starts_with(FORK_DUCKLAKE_SCHEMA_PREFIX), "{s}");
        assert!(s.len() <= 63, "pg schema name limit: {s}");
        assert!(
            s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "{s}"
        );
        // Deterministic (persisted view SQL / registry rows depend on it).
        assert_eq!(
            s,
            fork_ducklake_metadata_schema("wm-fork-my-feature-42", "main")
        );
    }

    #[test]
    fn test_fork_ducklake_metadata_schema_injective_after_mangling() {
        // `-` and `_` mangle to the same char; the hash suffix must keep them distinct.
        let a = fork_ducklake_metadata_schema("wm-fork-a-b", "main");
        let b = fork_ducklake_metadata_schema("wm-fork-a_b", "main");
        assert_ne!(a, b);
        // Lake-scoped: two lakes of one workspace may share a catalog database, so their fork
        // namespaces must be distinct schemas.
        assert_ne!(
            fork_ducklake_metadata_schema("wm-fork-a-b", "lake_a"),
            fork_ducklake_metadata_schema("wm-fork-a-b", "lake_b")
        );
        // Long ids truncate to the same mangled prefix; hash must still differ.
        let long_a = fork_ducklake_metadata_schema(&format!("wm-fork-{}x", "a".repeat(40)), "main");
        let long_b = fork_ducklake_metadata_schema(&format!("wm-fork-{}y", "a".repeat(40)), "main");
        assert_ne!(long_a, long_b);
        let long_lake =
            fork_ducklake_metadata_schema(&format!("wm-fork-{}", "a".repeat(42)), &"l".repeat(40));
        assert!(long_a.len() <= 63 && long_b.len() <= 63 && long_lake.len() <= 63);
    }

    #[test]
    fn test_fork_ducklake_ancestor_alias_valid_identifier() {
        let a = fork_ducklake_ancestor_alias("analytics", "wm-fork-dev-1");
        assert!(a.starts_with("__wm_dl_"), "{a}");
        assert!(
            a.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "{a}"
        );
        // Distinct per lake for the same ancestor (a script can attach several lakes).
        assert_ne!(a, fork_ducklake_ancestor_alias("staging", "wm-fork-dev-1"));
        assert_ne!(
            a,
            fork_ducklake_ancestor_alias("analytics", "wm-fork-dev-2")
        );
    }

    #[test]
    fn test_extract_metadata_schema_arg() {
        assert_eq!(
            extract_metadata_schema_arg("METADATA_SCHEMA 'lake_b_ns', ENCRYPTED true"),
            Some("lake_b_ns".to_string())
        );
        assert_eq!(
            extract_metadata_schema_arg("metadata_schema bare_ident"),
            Some("bare_ident".to_string())
        );
        // Last occurrence wins (DuckDB duplicate-option semantics).
        assert_eq!(
            extract_metadata_schema_arg("METADATA_SCHEMA 'a', METADATA_SCHEMA 'b'"),
            Some("b".to_string())
        );
        assert_eq!(extract_metadata_schema_arg("ENCRYPTED true"), None);
    }

    #[test]
    fn test_fork_data_path_prefix_isolation() {
        // Fork/dev ids are git-branch-safe and may contain `/`: `wm-fork-a/b` is a valid id.
        // Its data prefix must NOT nest inside `wm-fork-a`'s, or deleting `wm-fork-a` would
        // sweep the sibling's files via the object-store prefix listing.
        assert!(validate_fork_workspace_id("wm-fork-a/b").is_ok());
        let a = fork_data_path("lake", "wm-fork-a");
        let ab = fork_data_path("lake", "wm-fork-a/b");
        assert!(
            !format!("{ab}/").starts_with(&format!(
                "{FORK_DUCKLAKE_DATA_DIR}/{}/",
                fork_data_dir_segment("wm-fork-a")
            )),
            "{ab} nests under {a}'s cleanup prefix"
        );
        // Single path component: the segment itself contains no separator.
        assert!(!fork_data_dir_segment("wm-fork-a/b").contains('/'));
        // Injective after mangling (`/` and `_` both mangle to `_`).
        assert_ne!(
            fork_data_dir_segment("wm-fork-a/b"),
            fork_data_dir_segment("wm-fork-a_b")
        );
        // Deterministic (registry rows + cleanup guard recompute it).
        assert_eq!(a, fork_data_path("lake", "wm-fork-a"));
        // Empty base path still yields a well-formed prefix.
        assert_eq!(
            fork_data_path("", "wm-fork-a"),
            format!(
                "{FORK_DUCKLAKE_DATA_DIR}/{}",
                fork_data_dir_segment("wm-fork-a")
            )
        );
    }

    #[test]
    fn test_lfs_entry_storage_ref() {
        assert_eq!(
            lfs_entry_storage_ref(&serde_json::json!({
                "type": "S3Storage", "s3_resource_path": "$res:u/admin/minio"
            })),
            Some("S3Storage:u/admin/minio".to_string())
        );
        // The `$res:` prefix is optional in stored configs; normalized either way.
        assert_eq!(
            lfs_entry_storage_ref(&serde_json::json!({
                "type": "AzureBlobStorage", "azure_blob_resource_path": "u/admin/az"
            })),
            Some("AzureBlobStorage:u/admin/az".to_string())
        );
        assert_eq!(
            lfs_entry_storage_ref(&serde_json::json!({
                "type": "FilesystemStorage", "root_path": "/data/lfs"
            })),
            Some("FilesystemStorage:/data/lfs".to_string())
        );
        assert_eq!(
            lfs_entry_storage_ref(&serde_json::json!({"type": "SomethingNew"})),
            None
        );
        assert_eq!(lfs_entry_storage_ref(&serde_json::json!({})), None);
    }

    #[test]
    fn test_strip_fork_reserved_attach_args() {
        // Reserved options removed wherever they appear, others preserved.
        assert_eq!(
            strip_fork_reserved_attach_args("METADATA_SCHEMA 'main', ENCRYPTED true"),
            "ENCRYPTED true"
        );
        assert_eq!(
            strip_fork_reserved_attach_args(
                "ENCRYPTED true, DATA_PATH 's3://b/prod', OVERRIDE_DATA_PATH FALSE"
            ),
            "ENCRYPTED true"
        );
        // Case-insensitive, and quoted values may contain commas/spaces.
        assert_eq!(
            strip_fork_reserved_attach_args("data_path 's3://b/x, y', SNAPSHOT_VERSION 3"),
            "SNAPSHOT_VERSION 3"
        );
        assert_eq!(strip_fork_reserved_attach_args(""), "");
        assert_eq!(
            strip_fork_reserved_attach_args("METADATA_SCHEMA 'wm_fork_evil'"),
            ""
        );
    }

    #[test]
    fn test_datatable_not_found_error_no_datatables() {
        let msg = datatable_not_found_error("main", None).to_string();
        assert!(msg.contains("'main' not found"), "{msg}");
        assert!(msg.contains("No data table is configured"), "{msg}");
        // Always signposts the settings tab so the message is actionable.
        assert!(msg.contains("tab=windmill_data_tables"), "{msg}");
    }

    #[test]
    fn test_datatable_not_found_error_lists_available() {
        let configured = serde_json::json!({ "analytics": {}, "staging": {} });
        let msg = datatable_not_found_error("main", Some(&configured)).to_string();
        assert!(msg.contains("'main' not found"), "{msg}");
        // Surface configured names to catch typos.
        assert!(msg.contains("analytics"), "{msg}");
        assert!(msg.contains("staging"), "{msg}");
        assert!(msg.contains("tab=windmill_data_tables"), "{msg}");
    }
}
