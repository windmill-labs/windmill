/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Deploying from the UI into a workspace of another Windmill instance.
//!
//! The browser runs the same deploy it runs between two workspaces of this instance, and sends
//! every call aimed at the target through [`proxy`], which forwards it to the remote workspace
//! with the caller's own token for that instance. The remote authorizes and audits the deploy as
//! that person, so nothing here needs to know what a deploy is made of.

use std::time::Duration;

use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Extension, OriginalUri, Path},
    http::{header, HeaderMap, Method, StatusCode},
    response::Response,
    routing::{any, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use magic_crypt::MagicCrypt256;
use regex::Regex;
use serde::{Deserialize, Serialize};
use windmill_api_auth::{is_effectively_unscoped, ApiAuthed};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    error::{error_source_chain, Error, Result},
    ssrf::validate_url_for_ssrf,
    utils::{configure_client, rd_string, require_admin},
    variables::{build_crypt, decrypt, encrypt},
    worker::CLOUD_HOSTED,
    DB,
};

lazy_static! {
    static ref REMOTE_WORKSPACE_ID: Regex = Regex::new("^[a-zA-Z0-9_-]{1,50}$").unwrap();
    /// Shared so that a deploy's calls reuse connections. A cloud instance instead builds a client
    /// per request, pinned to the addresses it just validated for that target.
    static ref REMOTE_CLIENT: reqwest::Client = remote_client_builder().build().unwrap();
}

const PROXY_PREFIX: &str = "/remote_deploy/proxy/";

pub fn workspaced_service(proxy_body_limit: usize) -> Router {
    Router::new()
        .route("/target", get(get_target).post(set_target))
        .route("/connect", post(connect))
        .route("/disconnect", post(disconnect))
        // `{key}` is the caller's `proxy_key`. The session cookie is `SameSite=Lax`, so it rides a
        // top-level navigation from any site: without a value such a link cannot know, it could
        // spend the stored token, to run something on the remote or to render its content on this
        // origin. `Sec-Fetch-Site` is no substitute, since plain-http instances never receive it.
        .route(
            "/proxy/{key}/{*rest}",
            any(proxy).layer(DefaultBodyLimit::max(proxy_body_limit)),
        )
}

/// A request URI as logs should record it: with the proxy key masked, since that key is what keeps
/// a link from spending a stored remote token.
pub struct RedactedUri<'a>(pub &'a axum::http::Uri);

impl std::fmt::Display for RedactedUri<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Some((head, keyed)) = self.0.path().split_once(PROXY_PREFIX) else {
            return self.0.fmt(f);
        };
        let rest = keyed.find('/').map_or("", |i| &keyed[i..]);
        write!(f, "{head}{PROXY_PREFIX}***{rest}")?;
        match self.0.query() {
            Some(query) => write!(f, "?{query}"),
            None => Ok(()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemoteDeployTarget {
    /// Root URL of the remote instance, without `/api`.
    pub base_url: String,
    pub workspace_id: String,
}

impl RemoteDeployTarget {
    fn normalized(self) -> Result<Self> {
        let base_url = self.base_url.trim().trim_end_matches('/').to_string();
        let parsed = url::Url::parse(&base_url)
            .map_err(|e| Error::BadRequest(format!("Invalid remote instance URL: {e}")))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(Error::BadRequest(
                "The remote instance URL must be an http(s) URL with a host".to_string(),
            ));
        }
        if !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(Error::BadRequest(
                "The remote instance URL must not carry credentials, a query or a fragment"
                    .to_string(),
            ));
        }
        let workspace_id = self.workspace_id.trim().to_string();
        if !REMOTE_WORKSPACE_ID.is_match(&workspace_id) {
            return Err(Error::BadRequest(format!(
                "Invalid remote workspace id: {workspace_id}"
            )));
        }
        Ok(Self { base_url, workspace_id })
    }

    fn api_url(&self, suffix: &str) -> String {
        format!("{}/api/w/{}/{suffix}", self.base_url, self.workspace_id)
    }
}

#[derive(Serialize)]
struct RemoteDeployConnection {
    remote_email: String,
    /// Goes in every proxy URL, see [`workspaced_service`].
    proxy_key: String,
    connected_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct RemoteDeployStatus {
    target: Option<RemoteDeployTarget>,
    /// The caller's own connection to `target`.
    connection: Option<RemoteDeployConnection>,
}

/// A stored token acts on another instance as its owner, with none of the restrictions of the
/// credential that reaches it. Only the owner acting directly with full rights may use or replace
/// it: a job token runs as whoever a `wm_deployers` member pointed `on_behalf_of` at, and a
/// scoped or read-only token was never granted this.
fn require_own_credentials(authed: &ApiAuthed) -> Result<()> {
    if authed.job_id.is_some()
        || authed.read_only
        || !is_effectively_unscoped(authed.scopes.as_deref())
    {
        return Err(Error::PermissionDenied(
            "Deploying to a remote instance requires your own session or an unscoped token, \
             not a job, scoped or read-only token"
                .to_string(),
        ));
    }
    Ok(())
}

async fn load_target(db: &DB, w_id: &str) -> Result<Option<RemoteDeployTarget>> {
    let target = sqlx::query_scalar!(
        "SELECT remote_deploy_target FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(db)
    .await?
    .flatten();
    target
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| Error::internal_err(format!("reading the remote deploy target: {e}")))
}

async fn require_target(db: &DB, w_id: &str) -> Result<RemoteDeployTarget> {
    load_target(db, w_id).await?.ok_or_else(|| {
        Error::BadRequest(format!(
            "Workspace {w_id} has no remote deploy target. An admin sets it in the workspace \
             settings, under Dev workspace"
        ))
    })
}

fn remote_client_builder() -> reqwest::ClientBuilder {
    configure_client(reqwest::ClientBuilder::new())
        .user_agent("windmill/remote-deploy")
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
}

async fn remote_client(target: &RemoteDeployTarget) -> Result<reqwest::Client> {
    // A workspace admin chose the URL. A self-hosted instance may deploy into its own private
    // network, as it may reach a private git remote; a cloud workspace must not.
    if !*CLOUD_HOSTED {
        return Ok(REMOTE_CLIENT.clone());
    }
    validate_url_for_ssrf(&target.base_url)
        .await?
        .apply_dns_pinning(remote_client_builder())
        .build()
        .map_err(|e| Error::internal_err(format!("building the remote deploy client: {e}")))
}

fn unreachable(target: &RemoteDeployTarget, e: reqwest::Error) -> Error {
    Error::BadGateway(format!(
        "Could not reach the remote instance {}: {}",
        target.base_url,
        error_source_chain(&e)
    ))
}

/// Responses carrying a `proxy_key` must not be kept by any cache between here and the browser.
const NO_STORE: [(header::HeaderName, &str); 1] = [(header::CACHE_CONTROL, "no-store")];

struct StoredConnection {
    token: String,
    remote_email: String,
    proxy_key: String,
    connected_at: DateTime<Utc>,
}

/// The caller's connection to `target`, if they may still use it.
///
/// `connect` takes no lock against what clears these rows (a removal from the workspace, a target
/// change, a key rotation): writers take those rows in every order, so any lock it held could close
/// a deadlock. A row can therefore land just after one of them ran, and is voided here instead. It
/// counts only for the target it was granted for, if connected since the setting last changed (so
/// pointing the setting back does not revive it); for a superadmin or a membership that began no
/// later than the connect (so a re-add does not either); and while it decrypts.
async fn load_connection(
    db: &DB,
    w_id: &str,
    email: &str,
    target: &RemoteDeployTarget,
) -> Result<Option<StoredConnection>> {
    let Some(row) = sqlx::query_as!(
        StoredConnection,
        "SELECT t.token, t.remote_email, t.proxy_key, t.connected_at FROM remote_deploy_token t
         WHERE t.workspace_id = $1 AND t.email = $2 AND t.base_url = $3
           AND t.remote_workspace_id = $4
           AND NOT EXISTS (SELECT 1 FROM workspace_settings s WHERE s.workspace_id = t.workspace_id
                             AND s.remote_deploy_target_changed_at > t.connected_at)
           AND (EXISTS (SELECT 1 FROM usr u WHERE u.workspace_id = t.workspace_id
                          AND u.email = t.email AND u.created_at <= t.connected_at)
                OR EXISTS (SELECT 1 FROM password p WHERE p.email = t.email AND p.super_admin))",
        w_id,
        email,
        &target.base_url,
        &target.workspace_id
    )
    .fetch_optional(db)
    .await?
    else {
        return Ok(None);
    };
    match decrypt(&build_crypt(db, w_id).await?, row.token) {
        Ok(token) => Ok(Some(StoredConnection { token, ..row })),
        Err(e) => {
            tracing::warn!("remote deploy token of {email} in {w_id} does not decrypt: {e}");
            Ok(None)
        }
    }
}

async fn get_target(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> Result<(
    [(header::HeaderName, &'static str); 1],
    Json<RemoteDeployStatus>,
)> {
    let target = load_target(&db, &w_id).await?;
    // The connection carries the proxy key. A credential that may not use the proxy (see
    // `require_own_credentials`) must not read it either: it could hand its owner a working link.
    let connection = match &target {
        Some(target) if require_own_credentials(&authed).is_ok() => {
            load_connection(&db, &w_id, &authed.email, target)
                .await?
                .map(|c| RemoteDeployConnection {
                    remote_email: c.remote_email,
                    proxy_key: c.proxy_key,
                    connected_at: c.connected_at,
                })
        }
        _ => None,
    };
    Ok((NO_STORE, Json(RemoteDeployStatus { target, connection })))
}

#[derive(Deserialize)]
struct SetRemoteDeployTarget {
    target: Option<RemoteDeployTarget>,
}

async fn set_target(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(request): Json<SetRemoteDeployTarget>,
) -> Result<String> {
    if !cfg!(feature = "enterprise") {
        return Err(Error::BadRequest(
            "Deploying to another instance is only available on Windmill Enterprise Edition"
                .to_string(),
        ));
    }
    // Everyone who connects afterwards hands their token to this URL.
    require_own_credentials(&authed)?;
    require_admin(authed.is_admin, &authed.username)?;
    let target = request
        .target
        .map(RemoteDeployTarget::normalized)
        .transpose()?;

    let mut tx = db.begin().await?;
    let (base_url, remote_workspace_id) = match &target {
        Some(t) => (Some(t.base_url.as_str()), Some(t.workspace_id.as_str())),
        None => (None, None),
    };
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.edit_remote_deploy_target",
        ActionKind::Update,
        &w_id,
        None,
        Some(
            [
                ("base_url", base_url.unwrap_or("")),
                ("remote_workspace_id", remote_workspace_id.unwrap_or("")),
            ]
            .into(),
        ),
    )
    .await?;
    let target_json = target
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| Error::internal_err(e.to_string()))?;
    sqlx::query!(
        "UPDATE workspace_settings SET remote_deploy_target = $1::jsonb,
             remote_deploy_target_changed_at = CASE
                 WHEN remote_deploy_target IS DISTINCT FROM $1::jsonb THEN now()
                 ELSE remote_deploy_target_changed_at END
         WHERE workspace_id = $2",
        target_json,
        &w_id
    )
    .execute(&mut *tx)
    .await?;
    // A token is only ever sent to the target it was granted for, so the ones for any other
    // target are dead credentials.
    sqlx::query!(
        "DELETE FROM remote_deploy_token WHERE workspace_id = $1
           AND ($2::text IS NULL OR base_url <> $2 OR remote_workspace_id <> $3)",
        &w_id,
        base_url,
        remote_workspace_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(match target {
        Some(t) => format!(
            "Workspace {w_id} deploys to {} on {}",
            t.workspace_id, t.base_url
        ),
        None => format!("Removed the remote deploy target of workspace {w_id}"),
    })
}

#[derive(Deserialize)]
struct ConnectRequest {
    token: String,
    /// The target the caller got the token for, as the drawer showed it.
    target: RemoteDeployTarget,
}

#[derive(Deserialize)]
struct RemoteWhoami {
    email: String,
}

async fn connect(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(request): Json<ConnectRequest>,
) -> Result<(
    [(header::HeaderName, &'static str); 1],
    Json<RemoteDeployConnection>,
)> {
    require_own_credentials(&authed)?;
    let target = require_target(&db, &w_id).await?;
    // A token is only ever sent to the instance it was meant for. Without this, re-pointing the
    // target between the user getting a token and posting it here would hand it to the new one;
    // after this check the token still goes to `target` only, never to what the setting became.
    let requested = request.target.normalized()?;
    if requested.base_url != target.base_url || requested.workspace_id != target.workspace_id {
        return Err(Error::BadRequest(
            "The remote deploy target changed since you started connecting; reload and connect \
             again"
                .to_string(),
        ));
    }
    let token = request.token.trim();
    if token.is_empty() {
        return Err(Error::BadRequest("The token is empty".to_string()));
    }

    let response = remote_client(&target)
        .await?
        .get(target.api_url("users/whoami"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| unreachable(&target, e))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(Error::BadRequest(format!(
            "{} did not accept this token for workspace {} ({status}): {body}",
            target.base_url, target.workspace_id
        )));
    }
    let whoami: RemoteWhoami = response.json().await.map_err(|e| {
        Error::BadGateway(format!(
            "{} did not answer like a Windmill instance: {}",
            target.base_url,
            error_source_chain(&e)
        ))
    })?;

    let proxy_key = rd_string(32);
    let mc = build_crypt(&db, &w_id).await?;
    // No lock against removals, target changes or key rotations: `load_connection` voids a row
    // that lands after one of them.
    let mut tx = db.begin().await?;
    let connected_at = sqlx::query_scalar!(
        "INSERT INTO remote_deploy_token
             (workspace_id, email, base_url, remote_workspace_id, token, remote_email, proxy_key)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (workspace_id, email) DO UPDATE SET
             base_url = EXCLUDED.base_url, remote_workspace_id = EXCLUDED.remote_workspace_id,
             token = EXCLUDED.token, remote_email = EXCLUDED.remote_email,
             proxy_key = EXCLUDED.proxy_key, connected_at = now()
         RETURNING connected_at",
        &w_id,
        &authed.email,
        &target.base_url,
        &target.workspace_id,
        encrypt(&mc, token),
        &whoami.email,
        &proxy_key
    )
    .fetch_one(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.remote_deploy_connect",
        ActionKind::Create,
        &w_id,
        Some(&whoami.email),
        Some([("base_url", target.base_url.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    Ok((
        NO_STORE,
        Json(RemoteDeployConnection { remote_email: whoami.email, proxy_key, connected_at }),
    ))
}

async fn disconnect(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> Result<String> {
    require_own_credentials(&authed)?;
    let mut tx = db.begin().await?;
    sqlx::query!(
        "DELETE FROM remote_deploy_token WHERE workspace_id = $1 AND email = $2",
        &w_id,
        &authed.email
    )
    .execute(&mut *tx)
    .await?;
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.remote_deploy_disconnect",
        ActionKind::Delete,
        &w_id,
        None,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(format!(
        "Disconnected from the remote deploy target of {w_id}"
    ))
}

/// The remote URL for the request path after the proxy prefix and key, as the client sent it.
///
/// `url` rewrites a path while parsing it: it resolves dot segments (`%2e` spellings included)
/// and turns backslashes into slashes. Either would reach a route other than the one this
/// instance authorized — outside the remote workspace, or a route its read-only check does not
/// recognize — so a path the parser would change is refused rather than forwarded.
fn forwarded_url(
    target: &RemoteDeployTarget,
    original_path: &str,
    query: Option<&str>,
) -> Result<url::Url> {
    let suffix = original_path
        .split_once(PROXY_PREFIX)
        .and_then(|(_, keyed)| keyed.split_once('/'))
        .map(|(_, suffix)| suffix)
        .unwrap_or_default();
    let invalid = || Error::BadRequest(format!("Invalid remote deploy path: {suffix}"));
    let base_path = url::Url::parse(&target.base_url)
        .map_err(|_| invalid())?
        .path()
        .trim_end_matches('/')
        .to_string();
    let mut url = url::Url::parse(&target.api_url(suffix)).map_err(|_| invalid())?;
    let expected_path = format!("{base_path}/api/w/{}/{suffix}", target.workspace_id);
    if suffix.is_empty() || url.path() != expected_path {
        return Err(invalid());
    }
    url.set_query(query);
    Ok(url)
}

async fn proxy(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, key, _rest)): Path<(String, String, String)>,
    OriginalUri(uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response> {
    require_own_credentials(&authed)?;
    let target = require_target(&db, &w_id).await?;
    let url = forwarded_url(&target, uri.path(), uri.query())?;
    let stored = load_connection(&db, &w_id, &authed.email, &target)
        .await?
        .ok_or_else(|| {
            Error::BadRequest(format!(
                "Connect to {} with your own token before deploying there",
                target.base_url
            ))
        })?;
    if !constant_time_eq::constant_time_eq(key.as_bytes(), stored.proxy_key.as_bytes()) {
        return Err(Error::BadRequest(
            "This remote deploy link is not yours or is out of date; reload the page".to_string(),
        ));
    }

    // Only what describes the payload: the caller's cookie and token belong to this instance.
    let mut request = remote_client(&target)
        .await?
        .request(method, url)
        .bearer_auth(stored.token)
        .body(body);
    for name in [header::CONTENT_TYPE, header::ACCEPT] {
        if let Some(value) = headers.get(&name) {
            request = request.header(name, value.clone());
        }
    }
    let response = request.send().await.map_err(|e| unreachable(&target, e))?;

    let status = response.status();
    // Passed through, a 401 would read as this instance's session having expired and log the
    // user out of it.
    if status == StatusCode::UNAUTHORIZED {
        let body = response.text().await.unwrap_or_default();
        return Err(Error::BadGateway(format!(
            "{} rejected your stored token: {body}",
            target.base_url
        )));
    }
    if status.is_redirection() {
        let location = response
            .headers()
            .get(header::LOCATION)
            .and_then(|l| l.to_str().ok())
            .unwrap_or_default()
            .to_string();
        return Err(Error::BadGateway(format!(
            "{} redirected to {location}. Set the remote deploy target to the URL it redirects to",
            target.base_url
        )));
    }
    // The body is the remote's and may be anything its users stored (an uploaded file, a job
    // result typed `text/html`); served from this origin it must never render as a page here.
    let mut builder = Response::builder()
        .status(status)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(
            header::CONTENT_SECURITY_POLICY,
            "sandbox; default-src 'none'",
        );
    if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
        builder = builder.header(header::CONTENT_TYPE, content_type);
    }
    builder
        .body(Body::from_stream(response.bytes_stream()))
        .map_err(|e| Error::internal_err(format!("building the proxied response: {e}")))
}

/// Move the stored remote tokens to a new workspace key. A token that no longer decrypts is
/// dropped: its owner connects again.
pub(crate) async fn reencrypt_tokens(
    conn: &mut sqlx::PgConnection,
    w_id: &str,
    old: &MagicCrypt256,
    new: &MagicCrypt256,
) -> Result<()> {
    let rows = sqlx::query!(
        "SELECT email, token FROM remote_deploy_token WHERE workspace_id = $1 FOR UPDATE",
        w_id
    )
    .fetch_all(&mut *conn)
    .await?;
    for row in rows {
        match decrypt(old, row.token) {
            Ok(plain) => {
                sqlx::query!(
                    "UPDATE remote_deploy_token SET token = $1
                     WHERE workspace_id = $2 AND email = $3",
                    encrypt(new, &plain),
                    w_id,
                    row.email
                )
                .execute(&mut *conn)
                .await?;
            }
            Err(_) => {
                sqlx::query!(
                    "DELETE FROM remote_deploy_token WHERE workspace_id = $1 AND email = $2",
                    w_id,
                    row.email
                )
                .execute(&mut *conn)
                .await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarded_url_refuses_paths_the_parser_rewrites() {
        let target = RemoteDeployTarget {
            base_url: "https://prod.example.com/windmill".to_string(),
            workspace_id: "prod".to_string(),
        };
        let prefix = "/api/w/dev/remote_deploy/proxy/someKey";
        let url = forwarded_url(
            &target,
            &format!("{prefix}/scripts/get/p/f/team/my%20script"),
            Some("with_starred_info=true"),
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://prod.example.com/windmill/api/w/prod/scripts/get/p/f/team/my%20script?with_starred_info=true"
        );
        for path in [
            format!("{prefix}/../../users/list"),
            format!("{prefix}/%2e%2e/%2e%2e/users/list"),
            format!("{prefix}/jobs\\run_wait_result\\p/f/team/action"),
            prefix.to_string(),
        ] {
            assert!(
                forwarded_url(&target, &path, None).is_err(),
                "{path} should be refused"
            );
        }
    }

    #[test]
    fn redacted_uri_masks_the_proxy_key() {
        let proxied: axum::http::Uri = "/api/w/dev/remote_deploy/proxy/secretKey/flows/get/f/a?x=1"
            .parse()
            .unwrap();
        assert_eq!(
            RedactedUri(&proxied).to_string(),
            "/api/w/dev/remote_deploy/proxy/***/flows/get/f/a?x=1"
        );
        let other: axum::http::Uri = "/api/w/dev/flows/list?x=1".parse().unwrap();
        assert_eq!(RedactedUri(&other).to_string(), "/api/w/dev/flows/list?x=1");
    }

    #[test]
    fn target_normalization() {
        let target = RemoteDeployTarget {
            base_url: "  https://prod.example.com/  ".to_string(),
            workspace_id: " prod ".to_string(),
        }
        .normalized()
        .unwrap();
        assert_eq!(target.base_url, "https://prod.example.com");
        assert_eq!(
            target.api_url("flows/create"),
            "https://prod.example.com/api/w/prod/flows/create"
        );

        for (base_url, workspace_id) in [
            ("ftp://prod.example.com", "prod"),
            ("https://user:pass@prod.example.com", "prod"),
            ("https://prod.example.com?token=x", "prod"),
            ("https://prod.example.com", "prod/../admins"),
        ] {
            assert!(
                RemoteDeployTarget {
                    base_url: base_url.to_string(),
                    workspace_id: workspace_id.to_string(),
                }
                .normalized()
                .is_err(),
                "{base_url} {workspace_id} should be refused"
            );
        }
    }
}
