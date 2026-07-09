/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! OAuth2 client and token management for Windmill.
//!
//! This crate provides OAuth2 functionality including:
//! - OAuth2 client configuration and building
//! - Token exchange and refresh
//! - Slack OAuth integration
//! - Client credentials flow support

use std::collections::HashMap;
use std::fmt::Debug;

use anyhow::anyhow;
use hmac::Mac;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use tower_cookies::{Cookie, Cookies};
use windmill_common::error::{self, to_anyhow, Error};
use windmill_common::more_serde::maybe_number_opt;
use windmill_common::oauth2::*;
use windmill_common::utils::now_from_db;
use windmill_common::BASE_URL;

pub type DB = sqlx::Pool<sqlx::Postgres>;

// Re-export oauth2 types that consumers need (also used internally)
pub use oauth2::{
    helpers, AccessToken, AuthType, Client as OClient, RefreshToken, Scope, State, Url,
};

// Re-export reqwest Client (version 0.12 compatible with async-oauth2)
pub use reqwest::Client as HttpClient;

pub use windmill_common::utils::{COOKIE_DOMAIN, IS_SECURE};

lazy_static::lazy_static! {
    /// HTTP client for OAuth operations (reqwest 0.12, compatible with async-oauth2)
    pub static ref OAUTH_HTTP_CLIENT: reqwest::Client = reqwest::ClientBuilder::new()
        .user_agent("windmill/oauth")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create OAuth HTTP client");

    pub static ref OAUTH_CLIENTS: arc_swap::ArcSwap<AllClients> = arc_swap::ArcSwap::from_pointee(AllClients {
        logins: HashMap::new(),
        connects: HashMap::new(),
        slack: None
    });
}

/// OAuth client with associated scopes and configuration
#[derive(Debug, Clone)]
pub struct ClientWithScopes {
    pub display_name: Option<String>,
    pub client: OClient,
    pub scopes: Vec<String>,
    pub extra_params: Option<HashMap<String, String>>,
    pub extra_params_callback: Option<HashMap<String, String>>,
    pub allowed_domains: Option<Vec<String>>,
    pub userinfo_url: Option<String>,
    pub grant_types: Vec<String>,
    /// Resolved token endpoint, exposed so the connect dialog can prefill and
    /// persist it on client-credentials accounts.
    pub token_url: String,
    /// Whether the instance entry carries shared credentials (non-empty id +
    /// secret). Providers without them are bring-your-own only — the connect
    /// dialog lists them under "Others", not "Instance-configured".
    pub has_shared_credentials: bool,
}

/// Map of OAuth client names to their configurations
pub type BasicClientsMap = HashMap<String, ClientWithScopes>;

/// OAuth provider configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthConfig {
    #[serde(default = "empty_auth")]
    pub auth_url: String,
    #[serde(default = "empty_string")]
    pub token_url: String,
    pub userinfo_url: Option<String>,
    pub scopes: Option<Vec<String>>,
    /// Default scopes for the client-credentials (2-legged) flow. These differ
    /// from the authorization-code `scopes` for most providers (member/consent
    /// scopes are invalid in a 2-legged token request), so CC never defaults to
    /// `scopes`. Absent means no default scope — the caller supplies any
    /// provider-specific scopes themselves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc_scopes: Option<Vec<String>>,
    pub extra_params: Option<HashMap<String, String>>,
    pub extra_params_callback: Option<HashMap<String, String>>,
    pub req_body_auth: Option<bool>,
    #[serde(default = "default_grant_types")]
    pub grant_types: Vec<String>,
    /// Optional URL overrides for the provider's sandbox environment. When
    /// present and the admin has configured a `<name>_sandbox` credentials
    /// entry, `build_oauth_clients` registers a second client under that key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<OAuthSandboxOverride>,
    /// Metadata for per-instance OAuth providers (Snowflake, ServiceNow, Coupa,
    /// …) whose authorize/token URLs carry an `{instance}` placeholder filled
    /// from an instance name. The instance-settings UI uses it to build the
    /// per-client `connect_config` for the authorization-code flow; the
    /// client-credentials flow reads its `token_url`/`strip_suffix`/`label`
    /// directly to host-pin the exchange.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_config_template: Option<ConnectConfigTemplate>,
}

/// URL overrides for an OAuth provider's sandbox environment. Inherits
/// scopes, extra_params, etc. from the parent [`OAuthConfig`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OAuthSandboxOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub userinfo_url: Option<String>,
}

/// Metadata for a per-instance OAuth provider. The instance-settings UI renders
/// one generic instance-name input and substitutes `{instance}` into
/// `auth_url`/`token_url` to build the per-client `connect_config`. Adding a new
/// per-instance provider needs only a registry entry carrying this template —
/// no frontend code change. The client-credentials flow additionally reads
/// `token_url`, `strip_suffix`, and `label` from it server-side (see
/// `resolve_cc_token_url_input`) to host-pin the token exchange.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectConfigTemplate {
    /// Properly-cased provider name for the settings dropdown (e.g. "ServiceNow");
    /// the UI falls back to a capitalized registry key when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub label: String,
    pub placeholder: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_url: Option<String>,
    /// Authorize endpoint (with `{instance}`). Absent for client-credentials-only
    /// providers (e.g. Coupa) that have no browser sign-in flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_url: Option<String>,
    pub token_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_body_auth: Option<bool>,
    /// Scopes copied into the built `connect_config` (e.g. NetSuite's
    /// `rest_webservices`). Templated providers default to no scopes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
    /// Key under `connect_config.extra_params` where the instance name is
    /// stored (defaults to `instance`). Snowflake uses `account_identifier` for
    /// backward compatibility with previously-saved configs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_params_key: Option<String>,
    /// Optional host suffix stripped from the input before substitution (e.g.
    /// `.service-now.com`), so the admin can paste a full host or a bare name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_suffix: Option<String>,
    /// Maps OAuth-connected resource arg fields to value templates substituting
    /// `{instance}` (e.g. ServiceNow's `instance_url` ->
    /// `https://{instance}.service-now.com`). Applied by the resource-connect
    /// flow so the created resource carries the instance-specific fields the
    /// scripts need (ServiceNow's token response omits the host).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_mapping: Option<HashMap<String, String>>,
}

impl OAuthConfig {
    /// Returns a copy of this config with sandbox URL overrides applied and
    /// the nested `sandbox` field cleared. Returns `None` if no overrides are
    /// set.
    pub fn as_sandbox(&self) -> Option<OAuthConfig> {
        let sb = self.sandbox.as_ref()?;
        let mut out = self.clone();
        out.sandbox = None;
        if let Some(u) = &sb.auth_url {
            out.auth_url = u.clone();
        }
        if let Some(u) = &sb.token_url {
            out.token_url = u.clone();
        }
        if sb.userinfo_url.is_some() {
            out.userinfo_url = sb.userinfo_url.clone();
        }
        Some(out)
    }
}

/// Suffix appended to a provider name to identify its sandbox variant in the
/// instance credentials map and in `account.client`.
pub const SANDBOX_SUFFIX: &str = "_sandbox";

/// Strips [`SANDBOX_SUFFIX`] from a client name, returning the canonical
/// provider name. Returns the input unchanged if no suffix is present.
pub fn canonical_provider_name(client_name: &str) -> &str {
    client_name
        .strip_suffix(SANDBOX_SUFFIX)
        .unwrap_or(client_name)
}

/// Resolves a registry [`OAuthConfig`] for `client_name`, transparently
/// applying the `sandbox` override block when the name carries the sandbox
/// suffix (e.g. `docusign_sandbox` resolves to `docusign` with sandbox URLs
/// applied). Used so callers don't need to know whether a name is a sandbox
/// variant before looking it up.
pub fn resolve_registry_config(
    static_configs: &HashMap<String, OAuthConfig>,
    client_name: &str,
) -> Option<OAuthConfig> {
    if let Some(cfg) = static_configs.get(client_name) {
        return Some(cfg.clone());
    }
    if client_name.ends_with(SANDBOX_SUFFIX) {
        return static_configs
            .get(canonical_provider_name(client_name))
            .and_then(|cfg| cfg.as_sandbox());
    }
    None
}

/// OAuth client credentials
#[derive(Clone, Serialize, Deserialize)]
pub struct OAuthClient {
    #[serde(default = "empty_string")]
    pub id: String,
    #[serde(default = "empty_string")]
    pub secret: String,
    #[serde(default, deserialize_with = "windmill_common::utils::empty_as_none")]
    pub display_name: Option<String>,
    pub allowed_domains: Option<Vec<String>>,
    pub connect_config: Option<OAuthConfig>,
    pub login_config: Option<OAuthConfig>,
    pub tenant: Option<String>,
    #[serde(default = "default_grant_types")]
    pub grant_types: Vec<String>,
}

impl std::fmt::Debug for OAuthClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthClient")
            .field("id", &self.id)
            .field("secret", &"***")
            .field("display_name", &self.display_name)
            .field("allowed_domains", &self.allowed_domains)
            .field("connect_config", &self.connect_config)
            .field("login_config", &self.login_config)
            .field("tenant", &self.tenant)
            .field("grant_types", &self.grant_types)
            .finish()
    }
}

fn empty_string() -> String {
    "".to_string()
}

/// Placeholder authorize URL for providers that only support the
/// client-credentials grant (the authorize endpoint is never used by it).
pub const MISSING_AUTH_URL: &str = "https://missing-auth-url";

fn empty_auth() -> String {
    MISSING_AUTH_URL.to_string()
}

fn default_grant_types() -> Vec<String> {
    vec!["authorization_code".to_string()]
}

/// Container for all OAuth clients (login, connect, and slack)
#[derive(Debug)]
pub struct AllClients {
    pub logins: BasicClientsMap,
    pub connects: BasicClientsMap,
    pub slack: Option<OClient>,
}

/// Slack token response from OAuth flow
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SlackTokenResponse {
    pub access_token: AccessToken,
    // Optional because Enterprise Grid installs can omit `team`. Callers should surface
    // a clear error if this is None rather than unwrapping.
    #[serde(default)]
    pub team: Option<SlackTeam>,
}

/// Standard OAuth token response
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TokenResponse {
    pub access_token: AccessToken,
    #[serde(deserialize_with = "maybe_number_opt")]
    #[serde(default)]
    pub expires_in: Option<u64>,
    pub refresh_token: Option<RefreshToken>,
    #[serde(deserialize_with = "helpers::deserialize_space_delimited_vec")]
    #[serde(serialize_with = "helpers::serialize_space_delimited_vec")]
    #[serde(default)]
    pub scope: Option<Vec<Scope>>,
}

/// Slack team info from OAuth v2 response
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SlackTeam {
    pub id: String,
    pub name: String,
}

/// OAuth callback parameters
#[derive(Deserialize)]
pub struct OAuthCallback {
    pub code: String,
    pub state: String,
}

/// Build a basic OAuth client from configuration
pub fn build_basic_client(
    name: String,
    config: OAuthConfig,
    client_params: OAuthClient,
    login: bool,
    base_url: &str,
    override_callback: Option<String>,
) -> error::Result<(String, OClient)> {
    let auth_url = Url::parse(&config.auth_url)
        .map_err(|e| anyhow!("Invalid authorization endpoint URL: {e}"))?;
    let token_url =
        Url::parse(&config.token_url).map_err(|e| anyhow!("Invalid token endpoint URL: {e}"))?;

    let redirect_url = if login {
        format!("{base_url}/user/login_callback/{name}")
    } else if let Some(callback) = override_callback {
        callback
    } else {
        format!("{base_url}/oauth/callback/{name}")
    };

    let mut client = OClient::new(client_params.id, auth_url, token_url);
    if config.req_body_auth.unwrap_or(false) {
        client.set_auth_type(AuthType::RequestBody);
    }
    client.set_client_secret(client_params.secret.clone());
    client.set_redirect_url(
        Url::parse(&redirect_url).map_err(|e| anyhow!("Invalid redirect URL: {e}"))?,
    );

    Ok((name.to_string(), client))
}

/// Build a Slack OAuth client with custom credentials
pub async fn build_slack_client(
    client_id: &str,
    client_secret: &str,
    _workspace_id: &str,
) -> error::Result<OClient> {
    let auth_url = Url::parse("https://slack.com/oauth/v2/authorize")
        .map_err(|e| anyhow!("Invalid Slack authorization URL: {e}"))?;
    let token_url = Url::parse("https://slack.com/api/oauth.v2.access")
        .map_err(|e| anyhow!("Invalid Slack token URL: {e}"))?;

    let base_url = (**BASE_URL.load()).clone();
    let redirect_url = format!("{}/oauth/callback_slack", base_url);

    let mut client = OClient::new(client_id.to_string(), auth_url, token_url);
    client.set_client_secret(client_secret.to_string());
    client.set_redirect_url(
        Url::parse(&redirect_url).map_err(|e| anyhow!("Invalid redirect URL: {e}"))?,
    );

    Ok(client)
}

/// Build OAuth client for client credentials flow with resource-level credentials.
///
/// No instance-level entry is required: the provider endpoint config resolves
/// from the instance `oauths` entry when one exists, else from the static
/// registry, else is synthesized from the token URL override alone. Returns the
/// built client together with the resolved [`OAuthConfig`] so callers can reuse
/// its scopes / `extra_params_callback`.
pub async fn build_client_credentials_oauth_client(
    db: &DB,
    client_name: &str,
    client_id: &str,
    client_secret: &str,
    resolved_token_url: Option<&str>,
    connect_configs_json: &str,
) -> error::Result<(OClient, OAuthConfig)> {
    use windmill_common::global_settings::{load_value_from_global_settings, OAUTH_SETTING};

    let oauths = load_value_from_global_settings(db, OAUTH_SETTING).await?;
    let instance_entry: Option<OAuthClient> = oauths
        .as_ref()
        .and_then(|o| o.get(client_name))
        .and_then(|v| match serde_json::from_value(v.clone()) {
            Ok(entry) => Some(entry),
            Err(e) => {
                tracing::warn!(
                    client = %client_name,
                    "Invalid instance OAuth entry, falling back to static registry: {e}"
                );
                None
            }
        });

    let resolve_from_registry = |client_name: &str| -> error::Result<Option<OAuthConfig>> {
        let static_configs =
            serde_json::from_str::<HashMap<String, OAuthConfig>>(connect_configs_json).map_err(
                |e| error::Error::InternalErr(format!("Failed to parse oauth_connect.json: {}", e)),
            )?;
        Ok(resolve_registry_config(&static_configs, client_name))
    };

    // A token URL alone is enough for client credentials: providers that only
    // support this grant have no authorize endpoint to configure.
    let instance_connect_config = instance_entry
        .as_ref()
        .and_then(|e| e.connect_config.clone())
        .filter(|c| !c.token_url.is_empty())
        .map(|mut c| {
            if c.auth_url.is_empty() {
                c.auth_url = empty_auth();
            }
            c
        });

    let from_instance = instance_connect_config.is_some();
    let mut connect_config = match instance_connect_config {
        Some(config) => config,
        None => resolve_from_registry(client_name)?.ok_or_else(|| {
            error::Error::BadRequest(format!(
                "No token URL available for '{}': not found in instance OAuth settings or static \
                 config",
                client_name
            ))
        })?,
    };

    // Registry providers default their client-credentials scopes from `cc_scopes`,
    // never the authorization-code `scopes` (which several providers reject for a
    // 2-legged token request). Instance-configured entries keep their admin-set
    // scopes untouched.
    if !from_instance {
        connect_config.scopes = connect_config.cc_scopes.clone();
    }

    let caller_supplied_creds = !client_id.is_empty() && !client_secret.is_empty();

    // Apply the resolved concrete token URL. Instance-templated providers (e.g.
    // Coupa) carry an empty or `{instance}`-templated token URL in their registry
    // config; the resolved value (host-pinned for instance-name connections,
    // persisted on the row for refresh) is what completes it. For bring-your-own
    // connections this value may instead be a caller-supplied override — safe
    // because only the caller's own credentials are ever sent to it.
    if let Some(url) = resolved_token_url {
        connect_config.token_url = url.to_string();
    }
    if connect_config.token_url.is_empty() {
        return Err(error::Error::BadRequest(format!(
            "No token URL configured for '{}'",
            client_name
        )));
    }

    // Fall back to the instance entry's own credentials when the caller supplies
    // none: the shared instance-level client-credentials setup, where an admin
    // configures one service-account client for everyone and the secret never
    // leaves the server. Only entries that explicitly enable the
    // client_credentials grant qualify, so an authorization-code-only client's
    // secret is never reused for this flow.
    let instance_cc_creds = instance_entry.as_ref().filter(|e| {
        e.grant_types.iter().any(|g| g == "client_credentials")
            && !e.id.is_empty()
            && !e.secret.is_empty()
    });
    // All-or-nothing: use the caller's credentials only when both id and secret
    // are present, otherwise fall back entirely to the instance entry. Never mix
    // a caller-supplied id with the admin secret (or vice versa).
    let (resolved_client_id, resolved_client_secret) = if caller_supplied_creds {
        (client_id.to_string(), client_secret.to_string())
    } else {
        instance_cc_creds
            .map(|e| (e.id.clone(), e.secret.clone()))
            .unwrap_or_default()
    };

    let resource_oauth_client = OAuthClient {
        id: resolved_client_id,
        secret: resolved_client_secret,
        allowed_domains: instance_entry
            .as_ref()
            .and_then(|e| e.allowed_domains.clone()),
        connect_config: Some(connect_config.clone()),
        login_config: instance_entry.as_ref().and_then(|e| e.login_config.clone()),
        display_name: instance_entry.as_ref().and_then(|e| e.display_name.clone()),
        grant_types: instance_entry
            .as_ref()
            .map(|e| e.grant_types.clone())
            .unwrap_or_else(default_grant_types),
        tenant: instance_entry.as_ref().and_then(|e| e.tenant.clone()),
    };

    let base_url = (**BASE_URL.load()).clone();
    let (_, client) = build_basic_client(
        client_name.to_string(),
        connect_config.clone(),
        resource_oauth_client,
        false,
        &base_url,
        None,
    )?;

    Ok((client, connect_config))
}

/// Shared instance-level client-credentials for `client_name`: the `(id, secret,
/// token_url)` from its instance `oauths` entry, but only when that entry both
/// declares the `client_credentials` grant and carries non-empty credentials.
/// Lets the connect flow use one admin-configured service-account client instead
/// of asking each user for their own.
///
/// # Authorization
/// Returns the admin's shared service-account secret, so callers MUST first
/// verify the caller's authorization to use it (workspace membership plus
/// read-write access — operators and read-only tokens are excluded). This helper
/// performs no authorization itself.
pub async fn resolve_instance_cc_credentials(
    db: &DB,
    client_name: &str,
) -> error::Result<Option<(String, String, Option<String>)>> {
    use windmill_common::global_settings::{load_value_from_global_settings, OAUTH_SETTING};

    let oauths = load_value_from_global_settings(db, OAUTH_SETTING).await?;
    let entry: Option<OAuthClient> = oauths
        .as_ref()
        .and_then(|o| o.get(client_name))
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    Ok(entry.and_then(|e| {
        let cc_grant = e.grant_types.iter().any(|g| g == "client_credentials");
        if cc_grant && !e.id.is_empty() && !e.secret.is_empty() {
            // Token URL from the entry's connect_config (built by instance settings
            // from the connect_config_template), so the account row is
            // self-contained for refresh.
            let token_url = e
                .connect_config
                .as_ref()
                .map(|c| c.token_url.clone())
                .filter(|u| !u.is_empty());
            Some((e.id, e.secret, token_url))
        } else {
            None
        }
    }))
}

/// Resolve the concrete client-credentials token URL for a bring-your-own
/// connection. The caller never supplies a token URL: it always comes from the
/// built-in registry, so the exchange host can never be redirected.
///
/// Supported only for registry providers. For one whose CC token URL carries an
/// `{instance}` placeholder (Coupa, ServiceNow, …) — declared in its
/// `connect_config_template` — the caller supplies only an instance name,
/// validated as a bare hostname label and substituted into the fixed-host
/// template. A fixed-host registry provider uses its registry token URL directly.
/// A custom resource type (no registry entry) is rejected: there is no known host
/// to send credentials to.
pub fn resolve_cc_token_url_input(
    connect_configs_json: &str,
    client_name: &str,
    caller_instance: Option<&str>,
) -> error::Result<String> {
    let Some(cfg) = serde_json::from_str::<HashMap<String, OAuthConfig>>(connect_configs_json)
        .ok()
        .and_then(|m| resolve_registry_config(&m, client_name))
    else {
        return Err(error::Error::BadRequest(format!(
            "Client credentials with your own credentials are only supported for built-in OAuth \
             providers, not '{client_name}'. Configure shared credentials on the instance OAuth \
             entry instead."
        )));
    };

    // Instance-templated providers carry the `{instance}` token URL (and its
    // label/strip_suffix) in `connect_config_template`; fixed-host providers use
    // the plain `token_url`.
    let tmpl = cfg.connect_config_template.as_ref();
    let template = tmpl
        .map(|t| t.token_url.clone())
        .filter(|u| !u.is_empty())
        .or_else(|| Some(cfg.token_url.clone()).filter(|u| !u.is_empty()))
        .ok_or_else(|| {
            error::Error::BadRequest(format!("No token URL is configured for '{client_name}'"))
        })?;

    if !template.contains("{instance}") {
        // Fixed-host registry provider: its registry token URL is authoritative.
        return Ok(template);
    }

    // Structural host-pinning guard: only substitute when `{instance}` is the
    // leftmost host label of a fixed-host template (`scheme://{instance}.fixed-host/…`).
    // The hostname-label validation below keeps the value clean, but only this
    // check guarantees the substituted value can never change the registrable
    // domain — so a malformed template (e.g. `https://{instance}/token`) can't turn
    // the caller's instance name into a full attacker-controlled host (SSRF /
    // credential exfiltration). The template is a code-reviewed registry file, so a
    // violation is a programming error.
    let placeholder = "{instance}";
    let idx = template.find(placeholder).unwrap();
    let after = &template[idx + placeholder.len()..];
    if !template[..idx].ends_with("://") || !after.starts_with('.') {
        return Err(error::Error::InternalErr(format!(
            "Invalid instance-templated token URL for '{client_name}': {{instance}} must be the \
             leftmost host label (scheme://{{instance}}.fixed-host/…)"
        )));
    }

    let raw = caller_instance
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            error::Error::BadRequest(format!(
                "{} is required for {client_name}",
                tmpl.map(|t| t.label.as_str()).unwrap_or("An instance name")
            ))
        })?;
    // Strip an optional known host suffix so the user can paste a full host or a
    // bare name, then accept only a hostname label — never any character that
    // could move the host out of the template's domain.
    let value = tmpl
        .and_then(|t| t.strip_suffix.as_deref())
        .and_then(|sfx| raw.strip_suffix(sfx))
        .unwrap_or(raw)
        .trim_end_matches('.');
    let valid = !value.is_empty()
        && !value.starts_with(['-', '.'])
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.');
    if !valid {
        return Err(error::Error::BadRequest(format!(
            "invalid instance name '{raw}' for {client_name}"
        )));
    }
    Ok(template.replace("{instance}", value))
}

/// Whether a built-in provider's client-credentials token URL is host-pinned via
/// an `{instance}` template (e.g. servicenow, snowflake, coupa). Such providers
/// only accept an instance name substituted into a fixed-host template, so a
/// free-form caller token URL override must be rejected for them — otherwise the
/// exchange host could be redirected, which is exactly what the template pins.
/// Fixed-host registry providers and custom (non-registry) providers return
/// `false`: an override is allowed there.
pub fn is_instance_templated_cc(connect_configs_json: &str, client_name: &str) -> bool {
    serde_json::from_str::<HashMap<String, OAuthConfig>>(connect_configs_json)
        .ok()
        .and_then(|m| resolve_registry_config(&m, client_name))
        .map(|cfg| {
            cfg.connect_config_template
                .as_ref()
                .map(|t| t.token_url.clone())
                .filter(|u| !u.is_empty())
                .unwrap_or(cfg.token_url)
                .contains("{instance}")
        })
        .unwrap_or(false)
}

/// Resolve the concrete bring-your-own client-credentials token URL for any
/// provider, never from a caller-supplied URL:
/// - **Built-in registry providers** resolve from the registry via
///   [`resolve_cc_token_url_input`] (host-pinned from the caller's instance name
///   for instance-templated ones).
/// - **Custom providers configured at the instance level** use the admin's
///   `connect_config.token_url`. The caller has no instance template to fill, so
///   an instance name is rejected.
///
/// This is the single entry point the connect/account-creation handlers should
/// use so both resolve identically.
pub async fn resolve_cc_token_url(
    db: &DB,
    client_name: &str,
    caller_instance: Option<&str>,
    connect_configs_json: &str,
) -> error::Result<String> {
    use windmill_common::global_settings::{load_value_from_global_settings, OAUTH_SETTING};

    let supports_cc =
        |grant_types: &[String]| grant_types.iter().any(|g| g == "client_credentials");

    let registry_cfg = serde_json::from_str::<HashMap<String, OAuthConfig>>(connect_configs_json)
        .ok()
        .and_then(|m| resolve_registry_config(&m, client_name));
    if let Some(cfg) = registry_cfg {
        // Built-in provider: only honor it for client credentials if it actually
        // declares that grant, so an authorization-code-only provider can't be
        // driven through the CC API.
        if !supports_cc(&cfg.grant_types) {
            return Err(error::Error::BadRequest(format!(
                "'{client_name}' is not enabled for the client_credentials grant"
            )));
        }
        return resolve_cc_token_url_input(connect_configs_json, client_name, caller_instance);
    }

    // Custom (non-registry) provider: the token URL comes from the admin's
    // instance connect_config (an admin-configured, trusted host), never the
    // caller. The instance entry must also enable the client-credentials grant.
    let entry: Option<OAuthClient> = load_value_from_global_settings(db, OAUTH_SETTING)
        .await?
        .as_ref()
        .and_then(|o| o.get(client_name))
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let instance_token_url = entry
        .as_ref()
        .filter(|e| supports_cc(&e.grant_types))
        .and_then(|e| e.connect_config.clone())
        .map(|c| c.token_url)
        .filter(|u| !u.is_empty());
    match instance_token_url {
        Some(_)
            if caller_instance
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false) =>
        {
            Err(error::Error::BadRequest(format!(
                "An instance name only applies to built-in instance-templated providers, not \
                 '{client_name}'"
            )))
        }
        Some(url) => Ok(url),
        None => Err(error::Error::BadRequest(format!(
            "Client credentials with your own credentials require '{client_name}' to be a built-in \
             OAuth provider or an instance entry that enables the client_credentials grant"
        ))),
    }
}

/// Exchange authorization code for tokens
pub async fn exchange_code<T: DeserializeOwned>(
    callback: OAuthCallback,
    cookies: &Cookies,
    client: OClient,
    extra_params: Option<HashMap<String, String>>,
    http_client: &reqwest::Client,
) -> error::Result<T> {
    let name = if COOKIE_DOMAIN.is_some() {
        "csrf_domain"
    } else {
        "csrf"
    };
    let csrf_state = cookies
        .get(name)
        .map(|x| x.value_trimmed().to_string())
        .unwrap_or("".to_string());
    if callback.state != csrf_state {
        return Err(error::Error::BadRequest("csrf did not match".to_string()));
    }

    let mut token_url = client.exchange_code(callback.code);

    if let Some(extra_params) = extra_params {
        for (key, value) in extra_params {
            token_url = token_url.param(key, value)
        }
    }

    token_url
        .with_client(http_client)
        .execute::<T>()
        .await
        .map_err(|e| error::Error::InternalErr(format!("{:?}", e)))
}

/// Internal token exchange implementation
pub async fn exchange_token(
    client: OClient,
    refresh_token: &str,
    grant_type: &str,
    extra_params_callback: Option<&HashMap<String, String>>,
    http_client: &reqwest::Client,
    scopes: Option<&[String]>,
) -> Result<TokenResponse, Error> {
    let token_json = match grant_type {
        "authorization_code" | "" => {
            let mut request = client.exchange_refresh_token(&RefreshToken::from(refresh_token));
            if let Some(scopes) = scopes {
                if !scopes.is_empty() {
                    request = request.param("scope", scopes.join(" "));
                }
            }
            request
                .with_client(http_client)
                .execute::<serde_json::Value>()
                .await
                .map_err(to_anyhow)?
        }
        "client_credentials" => {
            let mut token_request = client.exchange_client_credentials();

            if let Some(extra_params) = extra_params_callback {
                for (key, value) in extra_params.iter() {
                    token_request = token_request.param(key.clone(), value.clone());
                }
            }

            token_request
                .with_client(http_client)
                .execute::<serde_json::Value>()
                .await
                .map_err(to_anyhow)?
        }
        _ => {
            return Err(Error::BadRequest(format!(
                "Unsupported grant type: {}",
                grant_type
            )))
        }
    };

    let token = serde_json::from_value::<TokenResponse>(token_json.clone()).map_err(|e| {
        Error::BadConfig(format!(
            "Error deserializing response as a new token: {e}\nresponse:{token_json}"
        ))
    })?;
    Ok(token)
}

/// Pre-fetched account fields needed for token refresh.
pub struct OAuthAccountInfo {
    pub client: String,
    pub refresh_token: String,
    pub grant_type: String,
    pub cc_client_id: Option<String>,
    pub cc_client_secret: Option<String>,
    pub cc_token_url: Option<String>,
    pub scopes: Option<Vec<String>>,
}

/// Refresh an OAuth token and update the `account` row.
/// Fetches the account from DB, then delegates to `refresh_token_for_account`.
///
/// Returns the freshly minted access token. Persisting it to the secret variable
/// backing the resource is the caller's responsibility (it must route through the
/// configured secret backend — see `store_oauth_token_value` in `windmill-store`).
pub async fn refresh_token<'c>(
    mut tx: Transaction<'c, Postgres>,
    w_id: &str,
    id: i32,
    db: &DB,
    oauth_clients: &AllClients,
    http_client: &reqwest::Client,
    connect_configs_json: &str,
) -> error::Result<String> {
    let account = sqlx::query_as!(
        OAuthAccountInfo,
        "SELECT client, refresh_token, grant_type, cc_client_id, cc_client_secret, cc_token_url, scopes FROM account WHERE workspace_id = $1 AND id = $2",
        w_id,
        id,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let account = windmill_common::utils::not_found_if_none(account, "Account", &id.to_string())?;

    refresh_token_for_account(
        tx,
        w_id,
        id,
        db,
        account,
        oauth_clients,
        http_client,
        connect_configs_json,
    )
    .await
}

/// Refresh an OAuth token given pre-fetched account info (no additional SELECT).
///
/// Exchanges the refresh token, updates the `account` row (`refresh_token`,
/// `expires_at`, `refresh_error`) and returns the new access token. It does NOT
/// persist the token to the secret variable — the caller must do that through the
/// configured secret backend (`store_oauth_token_value`), otherwise an external
/// secret backend would keep serving the stale connect-time token.
pub async fn refresh_token_for_account<'c>(
    mut tx: Transaction<'c, Postgres>,
    w_id: &str,
    id: i32,
    db: &DB,
    account: OAuthAccountInfo,
    oauth_clients: &AllClients,
    http_client: &reqwest::Client,
    connect_configs_json: &str,
) -> error::Result<String> {
    // Instance-configured client: required for authorization_code (the refresh
    // token exchange uses the instance app's credentials). For client_credentials
    // it is resolved inside `build_client_credentials_oauth_client` instead.
    let oauth_client_info = oauth_clients.connects.get(&account.client).cloned();

    let is_client_credentials = account.grant_type == "client_credentials";

    let (mut client, cc_config) = if is_client_credentials {
        // Bring-your-own accounts store their own credentials (and resolved token
        // URL) on the row. Shared instance accounts store none: passing empty
        // credentials makes the builder re-resolve the admin's service-account
        // credentials and token URL from the instance entry on every refresh, so a
        // rotated or removed shared secret takes effect immediately (mirrors the
        // authorization-code model, where the row never holds the app secret).
        let (client_id, client_secret) = match (&account.cc_client_id, &account.cc_client_secret) {
            (Some(id), Some(secret)) => (id.as_str(), secret.as_str()),
            _ => ("", ""),
        };
        let (client, config) = build_client_credentials_oauth_client(
            db,
            &account.client,
            client_id,
            client_secret,
            account.cc_token_url.as_deref(),
            connect_configs_json,
        )
        .await?;
        (client, Some(config))
    } else {
        let info = oauth_client_info
            .as_ref()
            .ok_or_else(|| error::Error::BadRequest("invalid client".to_string()))?;
        (info.client.to_owned(), None)
    };

    // Account-level scopes (when stored) override these defaults. Client-credentials
    // accounts default to the resolved CC config's scopes (`cc_scopes` for registry
    // providers, the admin's instance scopes for custom ones) — never the instance
    // client's authorization-code scopes, which are invalid in a 2-legged request.
    // Authorization-code accounts default to the instance client's scopes.
    let fallback_scopes = if is_client_credentials {
        cc_config
            .as_ref()
            .and_then(|c| c.scopes.clone())
            .unwrap_or_default()
    } else {
        oauth_client_info
            .as_ref()
            .map(|i| i.scopes.clone())
            .unwrap_or_default()
    };
    let effective_scopes = account
        .scopes
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&fallback_scopes);

    if is_client_credentials {
        for scope in effective_scopes.iter() {
            client.add_scope(scope);
        }
    }

    let extra_params_callback = oauth_client_info
        .as_ref()
        .and_then(|i| i.extra_params_callback.clone())
        .or_else(|| {
            cc_config
                .as_ref()
                .and_then(|c| c.extra_params_callback.clone())
        });

    tracing::info!(
        grant_type = %account.grant_type,
        client = %account.client,
        workspace_id = %w_id,
        account_id = %id,
        "Refreshing OAuth token"
    );

    let token = exchange_token(
        client,
        &account.refresh_token,
        &account.grant_type,
        extra_params_callback.as_ref(),
        http_client,
        Some(effective_scopes),
    )
    .await;

    if let Err(token_err) = token {
        sqlx::query!(
            "UPDATE account SET refresh_error = $1 WHERE workspace_id = $2 AND id = $3",
            token_err.alt(),
            w_id,
            id,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Err(error::Error::BadRequest(format!(
            "Error refreshing token: {}",
            token_err.alt()
        )));
    };

    let token = token.unwrap();

    let expires_at = now_from_db(&mut *tx).await?
        + chrono::Duration::try_seconds(
            token
                .expires_in
                .ok_or_else(|| Error::InternalErr("expires_in expected and not found".to_string()))?
                .try_into()
                .unwrap(),
        )
        .unwrap_or_default();
    sqlx::query!(
        "UPDATE account SET refresh_token = $1, expires_at = $2, refresh_error = NULL WHERE workspace_id = $3 AND id = $4",
        token
            .refresh_token
            .map(|x| x.to_string())
            .unwrap_or(account.refresh_token),
        expires_at,
        w_id,
        id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let token_str = token.access_token.to_string();

    tracing::info!(
        grant_type = %account.grant_type,
        client = %account.client,
        workspace_id = %w_id,
        account_id = %id,
        "OAuth token refreshed successfully"
    );

    Ok(token_str)
}

/// Generate OAuth redirect URL with CSRF protection
pub fn oauth_redirect(
    clients: &HashMap<String, ClientWithScopes>,
    client_name: String,
    cookies: Cookies,
    scopes: Option<Vec<String>>,
    extra_params: Option<HashMap<String, String>>,
    is_secure: bool,
) -> error::Result<axum::response::Redirect> {
    let client_w_scopes = clients
        .get(&client_name)
        .ok_or_else(|| error::Error::BadRequest("client not found".to_string()))?;
    let state = State::new_random();
    let mut client = client_w_scopes.client.clone();
    let scopes_iter = if let Some(scopes) = scopes {
        scopes
    } else {
        client_w_scopes.scopes.clone()
    };

    for scope in scopes_iter.iter() {
        client.add_scope(scope);
    }

    let mut auth_url = client.authorize_url(&state);

    if let Some(extra_params) = extra_params {
        let mut query_string = auth_url.query_pairs_mut();
        for (key, value) in extra_params {
            query_string.append_pair(&key, &value);
        }
    }

    set_csrf_cookie(&state, cookies, is_secure);
    Ok(axum::response::Redirect::to(auth_url.as_str()))
}

/// Set CSRF cookie for OAuth state verification
pub fn set_csrf_cookie(state: &State, cookies: Cookies, is_secure: bool) {
    let csrf = state.to_base64();
    let name = if COOKIE_DOMAIN.is_some() {
        "csrf_domain".to_string()
    } else {
        "csrf".to_string()
    };
    let mut cookie = Cookie::new(name, csrf);
    cookie.set_secure(is_secure);
    cookie.set_same_site(Some(tower_cookies::cookie::SameSite::Lax));
    cookie.set_http_only(true);
    cookie.set_path("/");
    if COOKIE_DOMAIN.is_some() {
        cookie.set_domain(COOKIE_DOMAIN.clone().unwrap());
    }
    cookies.add(cookie);
}

/// Slack signature verifier for webhook authentication
#[derive(Clone, Debug)]
pub struct SlackVerifier {
    mac: HmacSha256,
}

impl SlackVerifier {
    pub fn new<S: AsRef<[u8]>>(secret: S) -> anyhow::Result<SlackVerifier> {
        HmacSha256::new_from_slice(secret.as_ref())
            .map(|mac| SlackVerifier { mac })
            .map_err(|_| anyhow::anyhow!("invalid secret"))
    }

    pub fn verify(&self, ts: &str, body: &str, exp_sig: &str) -> anyhow::Result<()> {
        let basestring = format!("v0:{}:{}", ts, body);
        let mut mac = self.mac.clone();

        mac.update(basestring.as_bytes());
        let sig = format!("v0={}", hex::encode(mac.finalize().into_bytes()));
        if sig != exp_sig {
            Err(anyhow::anyhow!("signature mismatch"))?;
        }
        Ok(())
    }
}

/// Fetch user info from OAuth provider
pub async fn http_get_user_info<T: DeserializeOwned>(
    http_client: &reqwest::Client,
    url: &str,
    token: &str,
) -> error::Result<T> {
    let res = http_client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(to_anyhow)
        .map_err(|e| error::Error::InternalErr(format!("failed to fetch user info: {}", e)))?;
    if !res.status().is_success() {
        tracing::debug!(
            "The bearer token of the failed oauth user info exchange is: {}",
            token
        );
        return Err(error::Error::BadConfig(format!(
            "The user info endpoint responded with non 200: {}\n{}\n{}",
            res.status(),
            res.headers()
                .iter()
                .map(|x| format!("{}: {}", x.0.as_str(), x.1.to_str().unwrap_or_default()))
                .collect::<Vec<_>>()
                .join("\n"),
            res.text().await.unwrap_or_default(),
        )));
    }
    Ok(res.json::<T>().await.map_err(to_anyhow).map_err(|e| {
        error::Error::InternalErr(format!("failed to decode json from user info: {}", e))
    })?)
}

/// GitHub email info response
#[derive(Deserialize)]
pub struct GHEmailInfo {
    pub email: String,
    pub verified: bool,
    pub primary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_verifier() {
        let verifier = SlackVerifier::new("test_secret").unwrap();
        assert!(verifier.verify("123", "body", "wrong_sig").is_err());
    }

    #[test]
    fn canonical_provider_name_strips_sandbox_suffix() {
        assert_eq!(canonical_provider_name("docusign_sandbox"), "docusign");
        assert_eq!(canonical_provider_name("docusign"), "docusign");
        assert_eq!(canonical_provider_name(""), "");
        // Only strips the suffix once; trailing suffix on already-canonical name.
        assert_eq!(
            canonical_provider_name("foo_sandbox_sandbox"),
            "foo_sandbox"
        );
    }

    fn sample_oauth_config(with_sandbox: bool) -> OAuthConfig {
        OAuthConfig {
            auth_url: "https://account.example.com/oauth/auth".to_string(),
            token_url: "https://account.example.com/oauth/token".to_string(),
            userinfo_url: Some("https://account.example.com/userinfo".to_string()),
            scopes: Some(vec!["signature".to_string()]),
            cc_scopes: None,
            extra_params: None,
            extra_params_callback: None,
            req_body_auth: None,
            grant_types: default_grant_types(),
            sandbox: with_sandbox.then(|| OAuthSandboxOverride {
                auth_url: Some("https://account-d.example.com/oauth/auth".to_string()),
                token_url: Some("https://account-d.example.com/oauth/token".to_string()),
                userinfo_url: None,
            }),
            connect_config_template: None,
        }
    }

    #[test]
    fn as_sandbox_returns_none_when_no_override() {
        assert!(sample_oauth_config(false).as_sandbox().is_none());
    }

    #[test]
    fn as_sandbox_overlays_urls_and_inherits_rest() {
        let resolved = sample_oauth_config(true).as_sandbox().unwrap();
        // URLs overridden by sandbox block
        assert_eq!(
            resolved.auth_url,
            "https://account-d.example.com/oauth/auth"
        );
        assert_eq!(
            resolved.token_url,
            "https://account-d.example.com/oauth/token"
        );
        // userinfo_url not in override → inherits from parent
        assert_eq!(
            resolved.userinfo_url,
            Some("https://account.example.com/userinfo".to_string())
        );
        // Scopes/grant_types inherited from parent
        assert_eq!(resolved.scopes, Some(vec!["signature".to_string()]));
        assert_eq!(resolved.grant_types, default_grant_types());
        // Nested sandbox field cleared on the resolved config
        assert!(resolved.sandbox.is_none());
    }

    #[test]
    fn resolve_registry_config_direct_lookup() {
        let mut registry = HashMap::new();
        registry.insert("docusign".to_string(), sample_oauth_config(true));

        let resolved = resolve_registry_config(&registry, "docusign").unwrap();
        assert_eq!(resolved.auth_url, "https://account.example.com/oauth/auth");
        // Direct lookup returns the entry as-is (sandbox block still attached).
        assert!(resolved.sandbox.is_some());
    }

    #[test]
    fn resolve_registry_config_sandbox_fallback() {
        let mut registry = HashMap::new();
        registry.insert("docusign".to_string(), sample_oauth_config(true));

        let resolved = resolve_registry_config(&registry, "docusign_sandbox").unwrap();
        // Sandbox-suffixed lookup resolves to parent's sandbox-overlaid config.
        assert_eq!(
            resolved.auth_url,
            "https://account-d.example.com/oauth/auth"
        );
        assert!(resolved.sandbox.is_none());
    }

    #[test]
    fn resolve_registry_config_missing_returns_none() {
        let registry: HashMap<String, OAuthConfig> = HashMap::new();
        assert!(resolve_registry_config(&registry, "docusign").is_none());
        assert!(resolve_registry_config(&registry, "docusign_sandbox").is_none());
    }

    #[test]
    fn resolve_registry_config_sandbox_without_block_returns_none() {
        let mut registry = HashMap::new();
        // Parent exists but has no sandbox override.
        registry.insert("docusign".to_string(), sample_oauth_config(false));
        assert!(resolve_registry_config(&registry, "docusign_sandbox").is_none());
    }

    const CC_REGISTRY: &str = r#"{
        "coupa": {
            "grant_types": ["client_credentials"],
            "connect_config_template": {
                "label": "Coupa instance",
                "placeholder": "x",
                "token_url": "https://{instance}.coupahost.com/oauth2/token",
                "strip_suffix": ".coupahost.com"
            }
        },
        "servicenow": {
            "grant_types": ["authorization_code", "client_credentials"],
            "connect_config_template": {
                "label": "ServiceNow instance",
                "placeholder": "dev12345",
                "auth_url": "https://{instance}.service-now.com/oauth_auth.do",
                "token_url": "https://{instance}.service-now.com/oauth_token.do",
                "strip_suffix": ".service-now.com"
            }
        },
        "visma": {
            "auth_url": "https://connect.visma.com/connect/authorize",
            "token_url": "https://connect.visma.com/connect/token",
            "grant_types": ["authorization_code", "client_credentials"]
        },
        "bad_host_tpl": {
            "grant_types": ["client_credentials"],
            "connect_config_template": {
                "label": "x", "placeholder": "x",
                "token_url": "https://{instance}/token"
            }
        },
        "bad_mid_tpl": {
            "grant_types": ["client_credentials"],
            "connect_config_template": {
                "label": "x", "placeholder": "x",
                "token_url": "https://api.{instance}.evil.com/token"
            }
        }
    }"#;

    #[test]
    fn cc_token_url_templated_substitutes_instance() {
        let url = resolve_cc_token_url_input(CC_REGISTRY, "coupa", Some("acme")).unwrap();
        assert_eq!(url, "https://acme.coupahost.com/oauth2/token");
    }

    #[test]
    fn cc_token_url_templated_from_connect_config_template() {
        // ServiceNow's CC token URL comes from its connect_config_template.
        let url = resolve_cc_token_url_input(CC_REGISTRY, "servicenow", Some("dev99")).unwrap();
        assert_eq!(url, "https://dev99.service-now.com/oauth_token.do");
    }

    #[test]
    fn cc_token_url_strips_known_host_suffix() {
        let url =
            resolve_cc_token_url_input(CC_REGISTRY, "coupa", Some("acme.coupahost.com")).unwrap();
        assert_eq!(url, "https://acme.coupahost.com/oauth2/token");
    }

    #[test]
    fn cc_token_url_rejects_instance_that_escapes_the_host() {
        // A '/' (or any non-hostname char) must not let the caller move the host
        // out of the template's domain.
        assert!(resolve_cc_token_url_input(CC_REGISTRY, "coupa", Some("evil.com/oauth")).is_err());
        assert!(resolve_cc_token_url_input(CC_REGISTRY, "coupa", Some("a@b")).is_err());
    }

    #[test]
    fn cc_token_url_requires_instance_when_templated() {
        assert!(resolve_cc_token_url_input(CC_REGISTRY, "coupa", None).is_err());
    }

    #[test]
    fn cc_token_url_fixed_host_uses_registry_url() {
        let url = resolve_cc_token_url_input(CC_REGISTRY, "visma", None).unwrap();
        assert_eq!(url, "https://connect.visma.com/connect/token");
    }

    #[test]
    fn cc_token_url_rejects_custom_provider() {
        // No registry entry: bring-your-own client credentials are not allowed.
        assert!(resolve_cc_token_url_input(CC_REGISTRY, "my_custom_thing", Some("acme")).is_err());
    }

    #[test]
    fn cc_token_url_rejects_template_not_in_subdomain_position() {
        // `{instance}` must be the leftmost host label of a fixed-host template, so
        // a malformed template can't let the instance value control the host.
        assert!(resolve_cc_token_url_input(CC_REGISTRY, "bad_host_tpl", Some("evil.com")).is_err());
        assert!(resolve_cc_token_url_input(CC_REGISTRY, "bad_mid_tpl", Some("evil")).is_err());
    }

    #[test]
    fn instance_templated_cc_true_for_templated_providers() {
        // Host-pinned via `{instance}`: a bring-your-own token URL override must be
        // refused for these (only the instance-name path may set their URL).
        assert!(is_instance_templated_cc(CC_REGISTRY, "coupa"));
        assert!(is_instance_templated_cc(CC_REGISTRY, "servicenow"));
    }

    #[test]
    fn instance_templated_cc_false_for_fixed_host_and_unknown() {
        // Fixed-host registry provider and custom (non-registry) provider both allow
        // an override, so neither is reported as instance-templated.
        assert!(!is_instance_templated_cc(CC_REGISTRY, "visma"));
        assert!(!is_instance_templated_cc(CC_REGISTRY, "my_custom_thing"));
    }
}
