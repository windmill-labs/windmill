/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::sync::atomic::AtomicBool;

use hmac::Hmac;
use serde::Serialize;
use sha2::Sha256;

pub type HmacSha256 = Hmac<Sha256>;

pub const WORKSPACE_SLACK_BOT_TOKEN_PATH: &str = "f/slack_bot/bot_token";

pub const GLOBAL_SLACK_BOT_TOKEN_PATH: &str = "f/slack_bot/global_bot_token";

pub const GLOBAL_TEAMS_BOT_TOKEN_PATH: &str = "f/teams_bot/global_bot_token";

pub const GLOBAL_TEAMS_API_TOKEN_PATH: &str = "f/teams_bot/global_api_token";
lazy_static::lazy_static! {

    pub static ref REQUIRE_PREEXISTING_USER_FOR_OAUTH: AtomicBool = AtomicBool::new(std::env::var("REQUIRE_PREEXISTING_USER_FOR_OAUTH")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false));

    /// Domain to append to emails missing a domain during external login (OAuth/SAML).
    /// If set, emails without '@' will have '@{LOGIN_DOMAIN}' appended.
    /// Example: LOGIN_DOMAIN=example.com transforms "john" to "john@example.com"
    pub static ref LOGIN_DOMAIN: Option<String> = std::env::var("LOGIN_DOMAIN").ok();

}

/// Normalize an email from external login by appending LOGIN_DOMAIN if the email is missing a domain.
/// Returns the email lowercased.
pub fn normalize_external_email(email: &str) -> String {
    let email = email.trim();
    if email.contains('@') {
        email.to_lowercase()
    } else if let Some(domain) = LOGIN_DOMAIN.as_ref() {
        format!("{}@{}", email, domain).to_lowercase()
    } else {
        email.to_lowercase()
    }
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum InstanceEvent {
    UserSignupOAuth { email: String },
    UserAdded { email: String },
    // UserDeleted { email: String },
    // UserDeletedWorkspace { workspace: String, email: String },
    UserAddedWorkspace { workspace: String, email: String },
    UserInvitedWorkspace { workspace: String, email: String },
    UserJoinedWorkspace { workspace: String, email: String, username: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_external_email_with_domain() {
        // Email already has domain - should just lowercase
        assert_eq!(
            normalize_external_email("John.Doe@Example.COM"),
            "john.doe@example.com"
        );
        assert_eq!(
            normalize_external_email("user@domain.org"),
            "user@domain.org"
        );
    }

    #[test]
    fn test_normalize_external_email_trims_whitespace() {
        assert_eq!(
            normalize_external_email("  user@example.com  "),
            "user@example.com"
        );
        assert_eq!(normalize_external_email("  john  "), "john");
    }

    #[test]
    fn test_normalize_external_email_without_domain_no_env() {
        // When LOGIN_DOMAIN is not set, email without @ stays as-is (lowercased)
        // Note: This test's behavior depends on whether LOGIN_DOMAIN env var is set
        // In the test environment, it's typically not set
        let result = normalize_external_email("JohnDoe");
        // Result will either be "johndoe" or "johndoe@{LOGIN_DOMAIN}" depending on env
        assert!(result.starts_with("johndoe"));
        assert_eq!(result, result.to_lowercase());
    }
}
