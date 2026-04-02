/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

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

pub fn username_to_permissioned_as(user: &str) -> String {
    if user.contains('@') {
        user.to_string()
    } else if let Some(group) = user.strip_prefix(USERNAME_GROUP_PREFIX) {
        format!("{}{}", PERMISSIONED_AS_GROUP_PREFIX, group)
    } else {
        format!("{}{}", PERMISSIONED_AS_USER_PREFIX, user)
    }
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

/// Get email from permissioned_as string.
/// - "u/{username}" → lookup email from usr table (cached)
/// - "g/{group}" → "group-{group}@windmill.dev"
/// - raw email → return as-is
pub async fn get_email_from_permissioned_as(
    permissioned_as: &str,
    workspace_id: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> crate::error::Result<String> {
    if let Some(username) = permissioned_as.strip_prefix(PERMISSIONED_AS_USER_PREFIX) {
        let lookup = EmailCacheKey(workspace_id, username);
        if let Some((email, cached_at)) = EMAIL_CACHE.get(&lookup) {
            if cached_at.elapsed().as_secs() < EMAIL_CACHE_TTL_SECS {
                return Ok(email);
            }
        }
        let email = sqlx::query_scalar!(
            "SELECT email FROM usr WHERE username = $1 AND workspace_id = $2",
            username,
            workspace_id
        )
        .fetch_optional(db)
        .await?;
        let email = match email {
            Some(e) => e,
            None => {
                // User not in workspace — check instance-level password table.
                // This handles super admins who can access any workspace without
                // a usr record; without the real email, the super_admin check in
                // fetch_authed_from_permissioned_as would fail.
                sqlx::query_scalar!("SELECT email FROM password WHERE username = $1", username)
                    .fetch_optional(db)
                    .await?
                    .unwrap_or_else(|| format!("{}@unknown.windmill.dev", username))
            }
        };
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
        assert_eq!(username_to_permissioned_as("group-all"), "g/all");
        assert_eq!(username_to_permissioned_as("group-my-team"), "g/my-team");
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
