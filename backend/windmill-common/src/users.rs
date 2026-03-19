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

pub fn username_to_permissioned_as(user: &str) -> String {
    if user.contains('@') {
        user.to_string()
    } else if let Some(group) = user.strip_prefix("group-") {
        format!("g/{}", group)
    } else {
        format!("u/{}", user)
    }
}

lazy_static::lazy_static! {
    /// Cache key is "workspace_id/username" to avoid allocating a tuple of two Strings on every lookup.
    static ref EMAIL_CACHE: quick_cache::sync::Cache<String, (String, std::time::Instant)> =
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
    if let Some(username) = permissioned_as.strip_prefix("u/") {
        let key = format!("{}/{}", workspace_id, username);
        if let Some((email, cached_at)) = EMAIL_CACHE.get(&key) {
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
        .await?
        .unwrap_or_else(|| format!("{}@windmill.dev", username));
        EMAIL_CACHE.insert(key, (email.clone(), std::time::Instant::now()));
        Ok(email)
    } else if let Some(group) = permissioned_as.strip_prefix("g/") {
        Ok(format!("group-{}@windmill.dev", group))
    } else {
        // raw email
        Ok(permissioned_as.to_string())
    }
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
}
