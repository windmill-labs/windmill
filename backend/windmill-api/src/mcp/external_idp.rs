//! MCP access with an access token issued by an external OIDC provider (`mcp_external_idp`).
//!
//! The token names an OAuth client of that provider and the user who signed in through it.
//! The client is mapped to a workspace and a selection of tools; the request then runs as the
//! user's own Windmill account limited to that selection, so a tool is offered only when the
//! client has it and the user can see it. Windmill-issued tokens never reach this path: it is
//! taken only for a JWT whose `iss` is the configured issuer.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
};
use base64::Engine;
use http::StatusCode;
use jsonwebtoken::{jwk::JwkSet, DecodingKey, Validation};
use serde_json::Value;
use windmill_api_auth::{ApiAuthed, OptJobAuthed};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    audit::AuditAuthor,
    db::{GatewayWorkspaceId, DB},
    global_settings::McpExternalIdp,
    utils::HTTP_CLIENT,
};
use windmill_mcp::server::NoEndpointTools;
use windmill_mcp::WorkspaceId;

use super::core::McpSelection;

/// What an MCP client is told to request from the external issuer: standard OIDC scopes,
/// which every provider knows, and which carry the user's email.
pub const REQUESTED_SCOPES: &str = "openid profile email";

/// The issuer configuration when external tokens are on: EE only.
pub fn configured() -> Option<Arc<McpExternalIdp>> {
    #[cfg(feature = "enterprise")]
    {
        windmill_common::global_settings::MCP_EXTERNAL_IDP.load_full()
    }
    #[cfg(not(feature = "enterprise"))]
    {
        None
    }
}

struct Jwks {
    issuer: String,
    keys: JwkSet,
    fetched_at: Instant,
}

/// A token resolved to its Windmill identity, kept until it expires or for a minute at most,
/// so a user's group changes and a removed client still take effect without a new sign-in.
#[derive(Clone)]
struct Resolved {
    authed: ApiAuthed,
    workspace_id: String,
    until: Instant,
}

const RESOLVED_TTL: Duration = Duration::from_secs(60);
/// Keys are fetched again for an unknown `kid` (a rotation), but not more often than this.
const JWKS_REFRESH_MIN_INTERVAL: Duration = Duration::from_secs(30);
/// A key the issuer withdrew stops verifying tokens at most this long after.
const JWKS_MAX_AGE: Duration = Duration::from_secs(600);

lazy_static::lazy_static! {
    static ref JWKS: tokio::sync::RwLock<Option<Jwks>> = tokio::sync::RwLock::new(None);
    static ref RESOLVED: quick_cache::sync::Cache<String, Resolved> =
        quick_cache::sync::Cache::new(1000);
}

/// The `iss` of a JWT, read without verifying it: only used to pick the verification path.
fn unverified_issuer(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    serde_json::from_slice::<Value>(&bytes)
        .ok()?
        .get("iss")?
        .as_str()
        .map(str::to_string)
}

async fn fetch_jwks(issuer: &str) -> anyhow::Result<JwkSet> {
    let discovery: Value = HTTP_CLIENT
        .get(format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let jwks_uri = discovery
        .get("jwks_uri")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("discovery document of {issuer} has no jwks_uri"))?;
    Ok(HTTP_CLIENT
        .get(jwks_uri)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

async fn decoding_key(
    issuer: &str,
    kid: &str,
    header_alg: jsonwebtoken::Algorithm,
) -> anyhow::Result<(DecodingKey, jsonwebtoken::Algorithm)> {
    let find = |jwks: &Option<Jwks>| {
        jwks.as_ref()
            .filter(|j| j.issuer == issuer && j.fetched_at.elapsed() < JWKS_MAX_AGE)
            .and_then(|j| j.keys.find(kid).cloned())
    };
    let mut jwk = find(&*JWKS.read().await);
    if jwk.is_none() {
        let mut guard = JWKS.write().await;
        jwk = find(&*guard);
        let stale = guard.as_ref().is_none_or(|j| {
            j.issuer != issuer || j.fetched_at.elapsed() > JWKS_REFRESH_MIN_INTERVAL
        });
        if jwk.is_none() && stale {
            *guard = Some(Jwks {
                issuer: issuer.to_string(),
                keys: fetch_jwks(issuer).await?,
                fetched_at: Instant::now(),
            });
            jwk = find(&*guard);
        }
    }
    let jwk = jwk.ok_or_else(|| anyhow::anyhow!("no key {kid} at {issuer}"))?;
    // Some issuers (Entra) publish keys without `alg`: the token's own header then names it,
    // but never an HMAC one, whose "key" would be the public JWK itself.
    use jsonwebtoken::Algorithm::{HS256, HS384, HS512};
    let alg = windmill_common::jwt::jwk_algorithm(&jwk)
        .or(Some(header_alg).filter(|a| !matches!(a, HS256 | HS384 | HS512)))
        .ok_or_else(|| anyhow::anyhow!("key {kid} names no signing algorithm"))?;
    Ok((DecodingKey::from_jwk(&jwk)?, alg))
}

async fn verify(idp: &McpExternalIdp, token: &str) -> anyhow::Result<(Value, u64)> {
    let header = jsonwebtoken::decode_header(token)?;
    let kid = header
        .kid
        .ok_or_else(|| anyhow::anyhow!("token has no kid"))?;
    let (key, alg) = decoding_key(&idp.issuer, &kid, header.alg).await?;
    let mut validation = Validation::new(alg);
    validation.set_issuer(&[&idp.issuer]);
    validation.validate_nbf = true;
    match &idp.audience {
        Some(aud) => validation.set_audience(&[aud]),
        None => validation.validate_aud = false,
    }
    let claims = jsonwebtoken::decode::<Value>(token, &key, &validation)?.claims;
    // Without an audience to check, an ID token of the same client would pass for an access
    // token: refuse what the issuer marks as one.
    if claims.get("typ").and_then(Value::as_str) == Some("ID") {
        anyhow::bail!("an ID token is not an access token");
    }
    let exp = claims.get("exp").and_then(Value::as_u64).unwrap_or(0);
    Ok((claims, exp))
}

/// Why a verified token was not served. Answered with 403, not 401: signing in again with
/// the same client and account would not change the answer.
struct Refusal {
    client: Option<String>,
    email: Option<String>,
    workspace_id: Option<String>,
    reason: String,
}

fn claim_str(claims: &Value, name: &str) -> Option<String> {
    claims
        .get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

async fn resolve(
    db: &DB,
    idp: &McpExternalIdp,
    claims: &Value,
    url_workspace: Option<&str>,
) -> Result<(ApiAuthed, String), Refusal> {
    let client_id = claim_str(claims, &idp.client_claim);
    let email = claim_str(claims, &idp.email_claim)
        .map(|e| windmill_common::oauth2::normalize_external_email(&e));
    let refuse = |workspace_id: Option<&str>, reason: String| Refusal {
        client: client_id.clone(),
        email: email.clone(),
        workspace_id: workspace_id.map(str::to_string),
        reason,
    };

    let client = client_id
        .as_deref()
        .and_then(|id| idp.client(id))
        .ok_or_else(|| {
            refuse(
                url_workspace,
                format!(
                    "OAuth client {} is not configured for MCP access",
                    client_id.as_deref().unwrap_or("(none)")
                ),
            )
        })?;
    let w_id = client.workspace_id.as_str();
    if url_workspace.is_some_and(|w| w != w_id) {
        return Err(refuse(
            url_workspace,
            format!(
                "OAuth client {} serves workspace {w_id} only",
                client.client_id
            ),
        ));
    }
    // A client-credentials token carries the client alone. A client acts only for a user,
    // within that user's own rights.
    let email = email.clone().ok_or_else(|| {
        refuse(
            Some(w_id),
            format!(
                "the token carries no `{}` claim: a client can only act for a signed-in user",
                idp.email_claim
            ),
        )
    })?;
    if claims.get("email_verified").and_then(Value::as_bool) == Some(false) {
        return Err(refuse(
            Some(w_id),
            format!("{email} is not verified at the issuer"),
        ));
    }

    sync_groups(db, claims, &email).await.map_err(|e| {
        refuse(
            Some(w_id),
            format!("could not apply the user's groups: {e}"),
        )
    })?;

    let username = sqlx::query_scalar!(
        "SELECT username FROM usr WHERE workspace_id = $1 AND email = $2 AND NOT disabled",
        w_id,
        &email
    )
    .fetch_optional(db)
    .await
    .map_err(|e| refuse(Some(w_id), e.to_string()))?
    .ok_or_else(|| {
        refuse(
            Some(w_id),
            format!(
                "{email} is not a member of workspace {w_id}. Users must sign in to Windmill once \
                 before using an MCP client"
            ),
        )
    })?;

    // Uncached, unlike `fetch_api_authed`: the groups just reconciled from the token must be
    // the ones this identity carries, not a copy from before the change.
    let mut authed: ApiAuthed = windmill_common::auth::fetch_authed_from_permissioned_as(
        &format!("u/{username}"),
        &email,
        w_id,
        db,
    )
    .await
    .map_err(|e| refuse(Some(w_id), e.to_string()))?
    .into();
    authed.scopes = Some(client.scopes.clone());
    authed.mcp_client = Some(client.client_id.clone());
    Ok((authed, w_id.to_string()))
}

/// Applies the token's groups claim to the user's IdP-managed instance groups, as an SSO
/// login does, so the rights the token carries are the ones checked.
#[cfg(all(feature = "enterprise", feature = "private", feature = "oauth2"))]
async fn sync_groups(db: &DB, claims: &Value, email: &str) -> windmill_common::error::Result<()> {
    let Some(claim) = crate::oauth2_ee::sso_groups_claim(db).await? else {
        return Ok(());
    };
    let values = match claims.get(&claim) {
        Some(Value::Array(v)) => Some(
            v.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect(),
        ),
        Some(Value::String(s)) => Some(vec![s.clone()]),
        _ => None,
    };
    let Some(groups) =
        crate::oauth2_ee::actionable_idp_groups(values, email, &claim, "MCP access token")
    else {
        return Ok(());
    };
    // Only an existing account: the reconcile would otherwise write memberships for an
    // address that has none.
    let exists = sqlx::query_scalar!("SELECT 1 FROM password WHERE email = $1", email)
        .fetch_optional(db)
        .await?
        .is_some();
    if exists {
        let mut tx = db.begin().await?;
        crate::scim_ee::reconcile_sso_instance_groups(email, &groups, &mut tx).await?;
        tx.commit().await?;
    }
    Ok(())
}

#[cfg(not(all(feature = "enterprise", feature = "private", feature = "oauth2")))]
async fn sync_groups(
    _db: &DB,
    _claims: &Value,
    _email: &str,
) -> windmill_common::error::Result<()> {
    Ok(())
}

async fn audit_refusal(db: &DB, refusal: &Refusal) {
    let email = refusal.email.clone().unwrap_or_default();
    let author =
        AuditAuthor { username: email.clone(), email, username_override: None, token_prefix: None };
    let client = refusal.client.clone().unwrap_or_default();
    let params = HashMap::from([
        ("mcp_client", client.as_str()),
        ("outcome", "refused"),
        ("reason", refusal.reason.as_str()),
    ]);
    if let Err(e) = audit_log(
        db,
        &author,
        "mcp.external_token",
        ActionKind::Execute,
        refusal.workspace_id.as_deref().unwrap_or("global"),
        Some(&client),
        Some(params),
    )
    .await
    {
        tracing::error!("Could not audit a refused MCP token: {e:#}");
    }
}

/// Records a tool call made through an external client, allowed or refused, with both
/// identities: the user as author, the client in the parameters.
pub async fn audit_tool_call(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    tool: &str,
    refusal: Option<&str>,
) {
    let Some(client) = authed.mcp_client.as_deref() else {
        return;
    };
    let outcome = if refusal.is_some() { "refused" } else { "ok" };
    windmill_common::feature_usage::log_feature_usage("mcp_external_client", "tool_call", outcome);
    let mut params = HashMap::from([("mcp_client", client), ("outcome", outcome)]);
    if let Some(reason) = refusal {
        params.insert("reason", reason);
    }
    if let Err(e) = audit_log(
        db,
        authed,
        "mcp.tool_call",
        ActionKind::Execute,
        w_id,
        Some(tool),
        Some(params),
    )
    .await
    {
        tracing::error!("Could not audit an MCP tool call: {e:#}");
    }
}

fn bearer(request: &Request) -> Option<&str> {
    request
        .headers()
        .get(http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

/// Authenticates a request carrying a token of the configured issuer, ahead of the usual
/// token lookup, and binds it to its client's workspace as a workspace-bound selection token
/// is. Every other request passes through untouched.
pub async fn external_idp_auth(
    Extension(db): Extension<DB>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(idp) = configured() else {
        return next.run(request).await;
    };
    let Some(token) = bearer(&request).map(str::to_string) else {
        return next.run(request).await;
    };
    if unverified_issuer(&token).as_deref() != Some(idp.issuer.as_str()) {
        return next.run(request).await;
    }
    let url_workspace = request
        .extensions()
        .get::<WorkspaceId>()
        .map(|w| w.0.clone());

    let cache_key = windmill_common::auth::hash_token(&token);
    let resolved = match RESOLVED
        .get(&cache_key)
        .filter(|r| r.until > Instant::now())
    {
        Some(r) => r,
        None => {
            let (claims, exp) = match verify(&idp, &token).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::debug!("External MCP token refused: {e:#}");
                    return (StatusCode::UNAUTHORIZED, "Invalid or expired access token")
                        .into_response();
                }
            };
            match resolve(&db, &idp, &claims, url_workspace.as_deref()).await {
                Ok((authed, workspace_id)) => {
                    let exp = std::time::UNIX_EPOCH + Duration::from_secs(exp);
                    let left = exp
                        .duration_since(std::time::SystemTime::now())
                        .unwrap_or_default();
                    let r = Resolved {
                        authed,
                        workspace_id,
                        until: Instant::now() + left.min(RESOLVED_TTL),
                    };
                    RESOLVED.insert(cache_key, r.clone());
                    r
                }
                Err(refusal) => {
                    audit_refusal(&db, &refusal).await;
                    return (StatusCode::FORBIDDEN, refusal.reason).into_response();
                }
            }
        }
    };
    if url_workspace
        .as_deref()
        .is_some_and(|w| w != resolved.workspace_id)
    {
        return (
            StatusCode::FORBIDDEN,
            format!(
                "This OAuth client serves workspace {} only",
                resolved.workspace_id
            ),
        )
            .into_response();
    }

    let extensions = request.extensions_mut();
    extensions.insert(OptJobAuthed { job_id: None, authed: resolved.authed.clone() });
    extensions.insert(resolved.authed);
    extensions.insert(GatewayWorkspaceId(resolved.workspace_id.clone()));
    extensions.insert(WorkspaceId(resolved.workspace_id));
    extensions.insert(McpSelection);
    extensions.insert(NoEndpointTools);
    next.run(request).await
}
