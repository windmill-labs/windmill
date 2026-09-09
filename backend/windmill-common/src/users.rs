/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

lazy_static::lazy_static! {
    /// Matches the non-quoted, non-IP-literal subset of the `proper_email` CHECK constraint
    /// carried by `usr` and `workspace_invite`, so anything accepted here is accepted by those
    /// tables too.
    pub static ref VALID_EMAIL: regex::Regex = regex::Regex::new(
        r"^[A-Za-z0-9!#$%&'*+/=?^_`{|}~-]+(\.[A-Za-z0-9!#$%&'*+/=?^_`{|}~-]+)*@([A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?\.)+[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?$"
    ).unwrap();
}

pub const SUPERADMIN_SECRET_EMAIL: &str = "superadmin_secret@windmill.dev";
pub const SUPERADMIN_NOTIFICATION_EMAIL: &str = "superadmin_notification@windmill.dev";
pub const SUPERADMIN_SYNC_EMAIL: &str = "superadmin_sync@windmill.dev";

pub const COOKIE_NAME: &str = "token";

/// Prefix for user-based permissioned_as values: "u/"
pub const PERMISSIONED_AS_USER_PREFIX: &str = "u/";
/// Prefix for group-based permissioned_as values: "g/"
pub const PERMISSIONED_AS_GROUP_PREFIX: &str = "g/";
/// Prefix for group-based usernames: "group-"
pub const USERNAME_GROUP_PREFIX: &str = "group-";
/// Widest principal a job row can carry (`v2_job.permissioned_as`), which is narrower than the
/// columns runnables and triggers store one in.
pub const PERMISSIONED_AS_MAX_LEN: usize = 55;

/// Whether any account exists for `email`: a `password` row (deactivated ones
/// included, since the sign-in path filters `disabled = false` and a re-enabled
/// account must not read as absent) or a `usr` row in any workspace (what a service
/// account has instead of a password). A guest is someone with none: the single rule
/// that keeps an account holder from ever holding a cheaper guest identity.
///
/// The address is lowercased before the lookup: accounts are stored lowercased, so a
/// mixed-case address would otherwise miss an existing account and be let through. The
/// comparison stays a plain equality (not `lower(email)`), so it uses the email index.
pub async fn has_any_account<'c, E: sqlx::Executor<'c, Database = sqlx::Postgres>>(
    executor: E,
    email: &str,
) -> crate::error::Result<bool> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM password WHERE email = $1)
             OR EXISTS(SELECT 1 FROM usr WHERE email = $1)",
    )
    .bind(email.to_lowercase())
    .fetch_one(executor)
    .await
    .map_err(|e| crate::error::Error::internal_err(format!("checking account for {email}: {e:#}")))
}

/// An email-shaped username is its own principal, which is how a superadmin acting without a
/// `usr` row is named (`usr.username` is constrained to `[\w-]+`, so a member never is). It is
/// decided before the group convention — an address is never a group's username — and one
/// containing `/` is prefixed, since readers split on the first `/` and would otherwise take
/// `g/alice@example.com` for a group.
pub fn username_to_permissioned_as(user: &str) -> String {
    if user.contains('@') {
        return if user.contains('/') {
            format!("{}{}", PERMISSIONED_AS_USER_PREFIX, user)
        } else {
            user.to_string()
        };
    }
    if let Some(group) = user.strip_prefix(USERNAME_GROUP_PREFIX) {
        return format!("{}{}", PERMISSIONED_AS_GROUP_PREFIX, group);
    }
    format!("{}{}", PERMISSIONED_AS_USER_PREFIX, user)
}

/// Borrowed key for zero-allocation cache lookups via `Equivalent<(String, String)>`.
#[derive(Hash)]
struct EmailCacheKey<'a>(&'a str, &'a str);

impl equivalent::Equivalent<(String, String)> for EmailCacheKey<'_> {
    fn equivalent(&self, key: &(String, String)) -> bool {
        self.0 == key.0 && self.1 == key.1
    }
}

lazy_static::lazy_static! {
    static ref EMAIL_CACHE: quick_cache::sync::Cache<(String, String), (String, std::time::Instant)> =
        quick_cache::sync::Cache::new(500);
}

const EMAIL_CACHE_TTL_SECS: u64 = 60;

/// Resolve a workspace-scoped username to its email.
///
/// Members are found in `usr`. A superadmin acting in a workspace they are *not*
/// a member of has no `usr` row; they carry either their instance-derived
/// username (`password.username`, when `automate_username_creation` is enabled)
/// or their email (when it is disabled), so fall back to `password` on both,
/// gated on `super_admin` since only superadmins can act without membership.
/// Returns `None` when the username resolves to nobody.
pub async fn resolve_username_to_email<'c>(
    workspace_id: &str,
    username: &str,
    db: impl sqlx::PgExecutor<'c>,
) -> crate::error::Result<Option<String>> {
    Ok(sqlx::query_scalar!(
        "SELECT COALESCE(
            (SELECT email FROM usr WHERE workspace_id = $1 AND username = $2),
            (SELECT email FROM password WHERE (username = $2 OR email = $2) AND super_admin = true)
        )",
        workspace_id,
        username
    )
    .fetch_optional(db)
    .await?
    .flatten())
}

/// Whether a permissioned_as names something that exists in this workspace.
///
/// An existence probe over the non-RLS pool, no authorization of its own: callers must
/// already be authorized for `w_id`. Same contract for [`permissioned_as_from_email`] and
/// [`resolve_username_to_email`].
///
/// Accepts the three forms `username_to_permissioned_as` can produce: `u/{username}`,
/// `g/{group}`, and a bare address when the username is itself email-shaped. Anything else
/// is malformed — `fetch_authed_from_permissioned_as` would take its least-privileged
/// branch rather than fail, so callers reject instead of storing it.
///
/// Decided prefix first, like `fetch_authed_from_permissioned_as` and
/// [`get_email_from_permissioned_as`]: a group name may contain `@` while a username never
/// carries a `u/`/`g/` prefix, so a bare address is only ever what is left over. Validating
/// by a different rule than dispatch reads by is what lets a stored identity run as someone
/// else.
pub async fn permissioned_as_exists(
    workspace_id: &str,
    permissioned_as: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> crate::error::Result<bool> {
    if let Some(username) = permissioned_as.strip_prefix(PERMISSIONED_AS_USER_PREFIX) {
        return Ok(resolve_username_to_email(workspace_id, username, db)
            .await?
            .is_some());
    }
    if let Some(group) = permissioned_as.strip_prefix(PERMISSIONED_AS_GROUP_PREFIX) {
        return Ok(sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM group_ WHERE workspace_id = $1 AND name = $2)",
            workspace_id,
            group
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false));
    }
    // The bare form names an account acting without a `usr` row, not anyone who merely holds
    // that address: `u/{username}` is the canonical principal for a member, and the bare branch
    // of `fetch_authed_from_permissioned_as` grants neither their groups nor their folders.
    // Anything unprefixed that is not an address at all is malformed.
    if !permissioned_as.contains('@') {
        return Ok(false);
    }
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS(
            SELECT 1 FROM usr WHERE workspace_id = $1 AND username = $2
            UNION ALL
            SELECT 1 FROM password WHERE email = $2 AND super_admin
        )",
        workspace_id,
        permissioned_as
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false))
}

/// Drop a cached address so a transactional email change is visible immediately.
///
/// The address is derived at dispatch and feeds the instance-superadmin check and
/// `email_to_igroup`, so serving a stale one would run jobs with the wrong authorization
/// for up to the cache TTL.
pub fn invalidate_email_cache(workspace_id: &str, username: &str) {
    EMAIL_CACHE.remove(&(workspace_id.to_string(), username.to_string()));
}

/// Inverse of [`get_email_from_permissioned_as`]: the principal an on-behalf-of email
/// names in this workspace, for callers that supply the email alone.
///
/// Mirrors [`resolve_username_to_email`] branch for branch, including its fallback to
/// `password` — a superadmin acting in a workspace they are not a member of has no `usr`
/// row, and dropping them here would hand their runnables back to the deployer while
/// keeping their superadmin email.
///
/// `None` when the email names nobody at all — an address outside the workspace that is
/// not a superadmin's, or a group that no longer exists. Callers then leave the identity
/// unrecorded rather than storing a principal that cannot authenticate.
///
/// Reads through the non-RLS pool and authorizes nothing: callers must already be authorized
/// for `workspace_id`.
pub async fn permissioned_as_from_email(
    workspace_id: &str,
    email: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> crate::error::Result<Option<String>> {
    let mut conn = db.acquire().await?;
    // A real account always wins: the synthetic group namespace below is not reserved,
    // so a user may legitimately hold a `group-*@windmill.dev` address, and resolving it
    // to the like-named group would hand their runnables that group's folder access.
    if let Some(username) = sqlx::query_scalar!(
        "SELECT COALESCE(
            (SELECT username FROM usr WHERE workspace_id = $1 AND email = $2),
            (SELECT COALESCE(username, email) FROM password WHERE email = $2 AND super_admin = true)
        )",
        workspace_id,
        email
    )
    .fetch_optional(&mut *conn)
    .await?
    .flatten()
    {
        return Ok(Some(username_to_permissioned_as(&username)));
    }
    // Groups have no address of their own; `get_email_from_permissioned_as` mints this
    // synthetic one, so it is the only form that can be read back as a group.
    let Some(group) = email
        .strip_prefix(USERNAME_GROUP_PREFIX)
        .and_then(|rest| rest.strip_suffix("@windmill.dev"))
    else {
        return Ok(None);
    };
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM group_ WHERE workspace_id = $1 AND name = $2)",
        workspace_id,
        group
    )
    .fetch_one(&mut *conn)
    .await?
    .unwrap_or(false);
    Ok(exists.then(|| format!("{}{}", PERMISSIONED_AS_GROUP_PREFIX, group)))
}

/// Get email from permissioned_as string.
/// - "u/{username}" → resolve via [`resolve_username_to_email`] (cached)
/// - "g/{group}" → "group-{group}@windmill.dev"
/// - raw email → return as-is
pub async fn get_email_from_permissioned_as<'c>(
    permissioned_as: &str,
    workspace_id: &str,
    db: impl sqlx::PgExecutor<'c>,
) -> crate::error::Result<String> {
    get_email_from_permissioned_as_inner(permissioned_as, workspace_id, db, true).await
}

/// [`get_email_from_permissioned_as`] without the address cache. Nothing evicts that cache
/// across processes, so for a minute after an email change it still serves the old address —
/// fine where the address only labels something on screen, wrong where it decides whether a
/// write is accepted or is copied onto a job row that outlives the window.
///
/// Reads through the non-RLS pool and authorizes nothing, like the cached one: callers must
/// already be authorized for `workspace_id`.
pub async fn get_email_from_permissioned_as_uncached<'c>(
    permissioned_as: &str,
    workspace_id: &str,
    db: impl sqlx::PgExecutor<'c>,
) -> crate::error::Result<String> {
    get_email_from_permissioned_as_inner(permissioned_as, workspace_id, db, false).await
}

async fn get_email_from_permissioned_as_inner<'c>(
    permissioned_as: &str,
    workspace_id: &str,
    db: impl sqlx::PgExecutor<'c>,
    use_cache: bool,
) -> crate::error::Result<String> {
    if let Some(username) = permissioned_as.strip_prefix(PERMISSIONED_AS_USER_PREFIX) {
        if use_cache {
            let lookup = EmailCacheKey(workspace_id, username);
            if let Some((email, cached_at)) = EMAIL_CACHE.get(&lookup) {
                if cached_at.elapsed().as_secs() < EMAIL_CACHE_TTL_SECS {
                    return Ok(email);
                }
            }
        }
        let email = resolve_username_to_email(workspace_id, username, db)
            .await?
            .unwrap_or_else(|| format!("{}@unknown.windmill.dev", username));
        let key = (workspace_id.to_string(), username.to_string());
        EMAIL_CACHE.insert(key, (email.clone(), std::time::Instant::now()));
        Ok(email)
    } else if let Some(group) = permissioned_as.strip_prefix(PERMISSIONED_AS_GROUP_PREFIX) {
        Ok(format!("{}{}@windmill.dev", USERNAME_GROUP_PREFIX, group))
    } else {
        // raw email
        Ok(permissioned_as.to_string())
    }
}

/// Compute the highest-precedence workspace role for a user across all their instance groups.
///
/// Precedence: admin (3) > developer (2) > operator (1).
/// Returns `(best_group_name, is_admin, is_operator)`.
pub fn compute_highest_workspace_role(
    user_igroups: &[String],
    ws_configured_groups: &[String],
    ws_roles: &std::collections::HashMap<String, String>,
) -> (String, bool, bool) {
    let mut best_group = String::new();
    let mut best_precedence = 0u8;

    for group in user_igroups {
        if !ws_configured_groups.contains(group) {
            continue;
        }
        let default_role = "developer".to_string();
        let role = ws_roles.get(group).unwrap_or(&default_role);
        let precedence = match role.as_str() {
            "admin" => 3u8,
            "operator" => 1,
            _ => 2,
        };
        if precedence > best_precedence {
            best_precedence = precedence;
            best_group = group.clone();
        }
    }

    let default_role = "developer".to_string();
    let best_role_str = ws_roles.get(&best_group).unwrap_or(&default_role);
    let (is_admin, is_operator) = match best_role_str.as_str() {
        "admin" => (true, false),
        "operator" => (false, true),
        _ => (false, false),
    };

    (best_group, is_admin, is_operator)
}

pub fn truncate_token(token: &str) -> String {
    if token.len() > 10 {
        let mut s = token[..10].to_owned();
        s.push_str("*****");
        s
    } else {
        token.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_username_to_permissioned_as() {
        assert_eq!(username_to_permissioned_as("alice"), "u/alice");
        assert_eq!(
            username_to_permissioned_as("alice@example.com"),
            "alice@example.com"
        );
        assert_eq!(
            username_to_permissioned_as("g/alice@example.com"),
            "u/g/alice@example.com"
        );
        // The `group-` convention is for usernames, and an address is never one.
        assert_eq!(
            username_to_permissioned_as("group-ops/alice@example.com"),
            "u/group-ops/alice@example.com"
        );
        assert_eq!(
            username_to_permissioned_as("group-ops@example.com"),
            "group-ops@example.com"
        );
        assert_eq!(username_to_permissioned_as("group-all"), "g/all");
        assert_eq!(username_to_permissioned_as("group-my-team"), "g/my-team");
    }

    #[test]
    fn test_valid_email() {
        for email in [
            "alice@example.com",
            "alice.bob+tag@sub.example.co.uk",
            "a_b-c!#$%&'*+/=?^_`{|}~@example.com",
        ] {
            assert!(VALID_EMAIL.is_match(email), "{email} should be valid");
        }
        for email in [
            "alice",
            "alice@example",
            "alice@@example.com",
            "alice @example.com",
            "alice@example.com\nbob@example.com",
            "",
        ] {
            assert!(!VALID_EMAIL.is_match(email), "{email} should be invalid");
        }
    }

    #[test]
    fn test_compute_highest_workspace_role_admin_wins() {
        let user_groups = vec!["ops".to_string(), "admins".to_string()];
        let ws_groups = vec!["ops".to_string(), "admins".to_string()];
        let mut roles = std::collections::HashMap::new();
        roles.insert("ops".to_string(), "operator".to_string());
        roles.insert("admins".to_string(), "admin".to_string());

        let (group, is_admin, is_operator) =
            compute_highest_workspace_role(&user_groups, &ws_groups, &roles);
        assert_eq!(group, "admins");
        assert!(is_admin);
        assert!(!is_operator);
    }

    #[test]
    fn test_compute_highest_workspace_role_developer_over_operator() {
        let user_groups = vec!["devs".to_string(), "ops".to_string()];
        let ws_groups = vec!["devs".to_string(), "ops".to_string()];
        let mut roles = std::collections::HashMap::new();
        roles.insert("devs".to_string(), "developer".to_string());
        roles.insert("ops".to_string(), "operator".to_string());

        let (group, is_admin, is_operator) =
            compute_highest_workspace_role(&user_groups, &ws_groups, &roles);
        assert_eq!(group, "devs");
        assert!(!is_admin);
        assert!(!is_operator);
    }

    #[test]
    fn test_compute_highest_workspace_role_skips_unconfigured_groups() {
        let user_groups = vec!["admins".to_string(), "other".to_string()];
        let ws_groups = vec!["ops".to_string()]; // admins not configured for this workspace
        let mut roles = std::collections::HashMap::new();
        roles.insert("admins".to_string(), "admin".to_string());
        roles.insert("ops".to_string(), "operator".to_string());

        let (group, is_admin, is_operator) =
            compute_highest_workspace_role(&user_groups, &ws_groups, &roles);
        // No user groups match ws_configured_groups, so best_group stays empty
        assert_eq!(group, "");
        assert!(!is_admin);
        assert!(!is_operator);
    }

    #[test]
    fn test_compute_highest_workspace_role_defaults_to_developer() {
        let user_groups = vec!["team".to_string()];
        let ws_groups = vec!["team".to_string()];
        let roles = std::collections::HashMap::new(); // no role configured → developer

        let (group, is_admin, is_operator) =
            compute_highest_workspace_role(&user_groups, &ws_groups, &roles);
        assert_eq!(group, "team");
        assert!(!is_admin);
        assert!(!is_operator);
    }
}
