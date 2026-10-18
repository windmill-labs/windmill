//! Agent worker authentication with a workload identity token: a JWT the platform the agent
//! runs on issues to it (a Kubernetes projected service account token, a SPIFFE JWT-SVID, ...),
//! verified against an issuer the superadmin trusts in `agent_workload_identity_issuers`.
//! The worker group and tags come from the first rule whose claims the token matches, never
//! from the token's own say, so the tag checks the static agent token path applies hold as is.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::{jwk::Jwk, Algorithm, DecodingKey, Validation};
use quick_cache::sync::Cache;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::guest_jwt::{jwk_algorithms, parse_jwks_keys, read_capped_body};
use crate::instance_config::{AgentWorkloadIdentityIssuer, AgentWorkloadIdentityRule};

/// A projected service account token is about 1 KB; refused past this before any work.
const MAX_TOKEN_LEN: usize = 16 * 1024;
const JWKS_TTL: Duration = Duration::from_secs(15 * 60);
/// An unknown `kid` refetches the key set, at most this often per source, so tokens carrying
/// made-up `kid`s cannot turn into one outbound fetch each.
const JWKS_MIN_REFETCH: Duration = Duration::from_secs(30);

/// Why a token was refused. `issuer` is set once the token named a trusted one. The reason
/// goes to the server log and the audit log, never to the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub issuer: Option<String>,
    pub reason: String,
}

impl Refusal {
    fn new(issuer: Option<&str>, reason: impl Into<String>) -> Self {
        Self { issuer: issuer.map(str::to_string), reason: reason.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadIdentity {
    pub issuer: String,
    pub subject: Option<String>,
    pub worker_group: String,
    pub tags: Vec<String>,
    pub exp: u64,
}

/// Rejects what would make a rule set ambiguous or a rule match more than intended. Runtime
/// matching skips a rule without claims too, so a setting planted around this check cannot
/// admit every token of an issuer.
pub fn validate_issuers(
    issuers: &[AgentWorkloadIdentityIssuer],
) -> std::result::Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for (i, cfg) in issuers.iter().enumerate() {
        let at = format!("issuer #{} ({})", i + 1, cfg.issuer);
        if cfg.issuer.trim().is_empty() {
            return Err(format!("issuer #{}: issuer is required", i + 1));
        }
        if !seen.insert(cfg.issuer.as_str()) {
            return Err(format!("{at}: listed more than once"));
        }
        if cfg.audience.trim().is_empty() {
            return Err(format!("{at}: audience is required"));
        }
        match (&cfg.jwks, &cfg.jwks_uri) {
            (Some(jwks), _) if !jwks.trim().is_empty() => {
                parse_jwks_keys(jwks.as_bytes()).map_err(|e| format!("{at}: inline JWKS: {e}"))?;
            }
            (_, Some(uri)) if !uri.trim().is_empty() => {
                if !(uri.starts_with("https://") || uri.starts_with("http://")) {
                    return Err(format!("{at}: jwks_uri must be an http(s) URL"));
                }
            }
            _ => {
                if !(cfg.issuer.starts_with("https://") || cfg.issuer.starts_with("http://")) {
                    return Err(format!(
                        "{at}: without jwks_uri or an inline JWKS, the issuer must be an http(s) URL \
                         serving OIDC discovery"
                    ));
                }
            }
        }
        if cfg.rules.is_empty() {
            return Err(format!("{at}: at least one rule is required"));
        }
        for (j, rule) in cfg.rules.iter().enumerate() {
            let at = format!("{at}, rule #{}", j + 1);
            if rule.claims.is_empty() || rule.claims.keys().any(|k| k.trim().is_empty()) {
                return Err(format!("{at}: at least one claim to match is required"));
            }
            if rule.worker_group.trim().is_empty()
                || rule.worker_group.contains(char::is_whitespace)
            {
                return Err(format!("{at}: worker group is required and has no spaces"));
            }
            if rule.tags.is_empty() || rule.tags.iter().any(|t| t.trim().is_empty()) {
                return Err(format!("{at}: at least one non-empty tag is required"));
            }
        }
    }
    Ok(())
}

/// The value at a dot separated claim path. At each level the longest key that prefixes the
/// remaining path is tried first, so `kubernetes.io.namespace` reaches `namespace` under the
/// `kubernetes.io` key.
fn claim_at<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let obj = value.as_object()?;
    if let Some(v) = obj.get(path) {
        return Some(v);
    }
    path.match_indices('.')
        .rev()
        .find_map(|(i, _)| claim_at(obj.get(&path[..i])?, &path[i + 1..]))
}

fn claim_matches(claims: &Value, path: &str, expected: &str) -> bool {
    match claim_at(claims, path) {
        Some(Value::String(s)) => s == expected,
        Some(v @ (Value::Number(_) | Value::Bool(_))) => v.to_string() == expected,
        _ => false,
    }
}

pub fn matching_rule<'a>(
    rules: &'a [AgentWorkloadIdentityRule],
    claims: &Value,
) -> Option<&'a AgentWorkloadIdentityRule> {
    rules.iter().find(|rule| {
        !rule.claims.is_empty()
            && rule
                .claims
                .iter()
                .all(|(path, expected)| claim_matches(claims, path, expected))
    })
}

/// The header's `alg`, read raw: `jsonwebtoken` cannot parse `none`, and a refusal should say
/// which algorithm was offered rather than that the header is malformed.
fn header_alg(token: &str) -> std::result::Result<Algorithm, String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let header = token.split('.').next().unwrap_or_default();
    let header: Value = URL_SAFE_NO_PAD
        .decode(header.trim_end_matches('='))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .ok_or("malformed JWT header")?;
    let alg = header.get("alg").and_then(Value::as_str).unwrap_or("");
    match alg.parse::<Algorithm>() {
        Ok(
            a @ (Algorithm::RS256
            | Algorithm::RS384
            | Algorithm::RS512
            | Algorithm::PS256
            | Algorithm::PS384
            | Algorithm::PS512
            | Algorithm::ES256
            | Algorithm::ES384),
        ) => Ok(a),
        _ => Err(format!(
            "alg {alg:?} is not accepted, only RS*, PS* and ES*"
        )),
    }
}

/// Verify `token` against the trusted `issuers` and map it to a worker group and tags.
pub async fn verify(
    issuers: &[AgentWorkloadIdentityIssuer],
    token: &str,
) -> std::result::Result<WorkloadIdentity, Refusal> {
    if token.len() > MAX_TOKEN_LEN {
        return Err(Refusal::new(None, "token is too long"));
    }
    let alg = header_alg(token).map_err(|r| Refusal::new(None, r))?;
    let kid = jsonwebtoken::decode_header(token)
        .map_err(|e| Refusal::new(None, format!("malformed JWT header: {e}")))?
        .kid;
    let unverified: Value = crate::jwt::decode_without_verify(token)
        .map_err(|e| Refusal::new(None, format!("malformed JWT payload: {e}")))?;
    let iss = unverified.get("iss").and_then(Value::as_str).unwrap_or("");
    let cfg = issuers
        .iter()
        .find(|c| c.issuer == iss)
        .ok_or_else(|| Refusal::new(None, format!("issuer {iss:?} is not trusted")))?;
    let refuse = |reason: String| Refusal::new(Some(&cfg.issuer), reason);

    let keys = issuer_keys(cfg, kid.as_deref())
        .await
        .map_err(|e| refuse(format!("signing keys unavailable: {e}")))?;
    let candidates: Vec<&Jwk> = match &kid {
        Some(kid) => keys.get(kid).into_iter().collect(),
        None => keys.values().collect(),
    };
    let candidates: Vec<&Jwk> = candidates
        .into_iter()
        .filter(|jwk| jwk_algorithms(jwk).is_some_and(|algs| algs.contains(&alg)))
        .collect();
    if candidates.is_empty() {
        return Err(refuse(match &kid {
            Some(kid) => format!("no {alg:?} signing key with kid {kid:?} in the issuer's JWKS"),
            None => format!("no {alg:?} signing key in the issuer's JWKS"),
        }));
    }

    let mut validation = Validation::new(alg);
    validation.set_audience(&[&cfg.audience]);
    validation.set_issuer(&[&cfg.issuer]);
    validation.set_required_spec_claims(&["exp", "iss", "aud"]);
    validation.validate_nbf = true;
    let mut last_err = None;
    let mut claims = None;
    for jwk in candidates {
        let key = match DecodingKey::from_jwk(jwk) {
            Ok(key) => key,
            Err(e) => {
                last_err = Some(e.to_string());
                continue;
            }
        };
        match jsonwebtoken::decode::<Value>(token, &key, &validation) {
            Ok(data) => {
                claims = Some(data.claims);
                break;
            }
            Err(e) => last_err = Some(describe_jwt_error(&e, &cfg.audience)),
        }
    }
    let claims = claims.ok_or_else(|| refuse(last_err.unwrap_or_default()))?;

    let subject = claims
        .get("sub")
        .and_then(Value::as_str)
        .map(str::to_string);
    let rule = matching_rule(&cfg.rules, &claims).ok_or_else(|| {
        refuse(format!(
            "no rule matches the token (sub {})",
            subject.as_deref().unwrap_or("<none>")
        ))
    })?;
    Ok(WorkloadIdentity {
        issuer: cfg.issuer.clone(),
        subject,
        worker_group: rule.worker_group.clone(),
        tags: rule.tags.clone(),
        exp: claims.get("exp").and_then(Value::as_u64).unwrap_or(0),
    })
}

fn describe_jwt_error(e: &jsonwebtoken::errors::Error, audience: &str) -> String {
    use jsonwebtoken::errors::ErrorKind;
    match e.kind() {
        ErrorKind::InvalidSignature => "signature does not verify".to_string(),
        ErrorKind::ExpiredSignature => "token is expired".to_string(),
        ErrorKind::ImmatureSignature => "token is not valid yet (nbf)".to_string(),
        ErrorKind::InvalidAudience => format!("aud does not include {audience:?}"),
        ErrorKind::InvalidIssuer => "iss does not match".to_string(),
        ErrorKind::MissingRequiredClaim(c) => format!("required claim {c} is missing"),
        _ => format!("token refused: {e}"),
    }
}

struct CachedKeys {
    keys: Arc<HashMap<String, Jwk>>,
    fetched_at: Instant,
}

lazy_static::lazy_static! {
    static ref KEYS_CACHE: Cache<String, Arc<CachedKeys>> = Cache::new(64);
    /// Fetches are rare (a refresh every `JWKS_TTL`, or a rotated-in `kid`); serializing them
    /// keeps a burst of tokens from fetching the same set once each.
    static ref FETCH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::new(());
}

async fn issuer_keys(
    cfg: &AgentWorkloadIdentityIssuer,
    kid: Option<&str>,
) -> Result<Arc<HashMap<String, Jwk>>> {
    if let Some(jwks) = cfg.jwks.as_deref().filter(|j| !j.trim().is_empty()) {
        return Ok(Arc::new(parse_jwks_keys(jwks.as_bytes())?));
    }
    let cache_key = format!("{}\n{}", cfg.issuer, cfg.jwks_uri.as_deref().unwrap_or(""));
    let usable = |e: &CachedKeys, max_age: Duration| {
        e.fetched_at.elapsed() < max_age && kid.map_or(true, |k| e.keys.contains_key(k))
    };
    let cached = KEYS_CACHE.get(&cache_key);
    if let Some(entry) = cached.as_ref().filter(|e| usable(e, JWKS_TTL)) {
        return Ok(entry.keys.clone());
    }
    let _guard = FETCH_LOCK.lock().await;
    if let Some(entry) = KEYS_CACHE.get(&cache_key) {
        if usable(&entry, JWKS_TTL) || entry.fetched_at.elapsed() < JWKS_MIN_REFETCH {
            return Ok(entry.keys.clone());
        }
    }
    let keys = match fetch_keys(cfg).await {
        Ok(keys) => Arc::new(keys),
        Err(e) => match cached.filter(|e| e.fetched_at.elapsed() < JWKS_TTL) {
            // An unknown kid with an unreachable issuer still verifies against the known keys.
            Some(entry) => return Ok(entry.keys.clone()),
            None => return Err(e),
        },
    };
    KEYS_CACHE.insert(
        cache_key,
        Arc::new(CachedKeys { keys: keys.clone(), fetched_at: Instant::now() }),
    );
    Ok(keys)
}

/// The issuer is superadmin-configured and trusted like `JWT_EXT_JWKS_URL`, so it is fetched
/// with the permissive client: an in-cluster or self-signed issuer has to work.
async fn fetch_keys(cfg: &AgentWorkloadIdentityIssuer) -> Result<HashMap<String, Jwk>> {
    let jwks_uri = match cfg.jwks_uri.as_deref().filter(|u| !u.trim().is_empty()) {
        Some(uri) => uri.to_string(),
        None => {
            let discovery = format!(
                "{}/.well-known/openid-configuration",
                cfg.issuer.trim_end_matches('/')
            );
            let doc: Value = serde_json::from_slice(&get(&discovery).await?)
                .map_err(|e| Error::BadRequest(format!("OIDC discovery is not JSON: {e}")))?;
            doc.get("jwks_uri")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::BadRequest("OIDC discovery has no jwks_uri".to_string()))?
                .to_string()
        }
    };
    parse_jwks_keys(&get(&jwks_uri).await?)
}

async fn get(url: &str) -> Result<Vec<u8>> {
    let resp = crate::utils::HTTP_CLIENT_PERMISSIVE
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| Error::BadRequest(format!("fetching {url}: {e}")))?;
    read_capped_body(resp).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header};
    use serde_json::json;
    use std::collections::BTreeMap;

    // Throwaway P-256 keys, used nowhere else. `JWKS` publishes the first one.
    const PRIV: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgu7FqG4gEcV8logVI
B6/Lc5aFA8jxKmix7Jdcr/acid2hRANCAATV+8XhZlMNXkRAy7FZ4nbQkBOechNv
a2U/COEBOOauFRMd0NB5bi2DAqO1r/h7eN7fLMGYLXVPlSvNdJuYL4F8
-----END PRIVATE KEY-----";
    const OTHER_PRIV: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgcU92tN6g4nOPoxQm
6KLBG0mtRL4gZ6/BNcrNPXiRammhRANCAASu64yo/1u80XgfncT4r9QMORMHZjDM
Sa6vBkm9YSs4MMV6K5vCIyow+i712HgUL/FNZ55GJVwXF77JffommMAk
-----END PRIVATE KEY-----";
    const JWKS: &str = r#"{"keys":[{"kty":"EC","crv":"P-256","alg":"ES256","use":"sig","kid":"k1",
        "x":"1fvF4WZTDV5EQMuxWeJ20JATnnITb2tlPwjhATjmrhU",
        "y":"Ex3Q0HluLYMCo7Wv-Ht43t8swZgtdU-VK810m5gvgXw"}]}"#;
    const ISS: &str = "https://kubernetes.default.svc.cluster.local";

    fn issuer() -> AgentWorkloadIdentityIssuer {
        AgentWorkloadIdentityIssuer {
            issuer: ISS.to_string(),
            jwks_uri: None,
            jwks: Some(JWKS.to_string()),
            audience: "windmill".to_string(),
            rules: vec![
                AgentWorkloadIdentityRule {
                    claims: BTreeMap::from([(
                        "sub".to_string(),
                        "system:serviceaccount:agents:gpu".to_string(),
                    )]),
                    worker_group: "gpu".to_string(),
                    tags: vec!["gpu".to_string()],
                },
                AgentWorkloadIdentityRule {
                    claims: BTreeMap::from([(
                        "kubernetes.io.namespace".to_string(),
                        "agents".to_string(),
                    )]),
                    worker_group: "agents".to_string(),
                    tags: vec!["dbt".to_string(), "etl".to_string()],
                },
            ],
        }
    }

    fn claims(sub: &str, ns: &str) -> Value {
        let now = jsonwebtoken::get_current_timestamp();
        json!({
            "iss": ISS, "aud": ["windmill"], "sub": sub, "exp": now + 600, "iat": now, "nbf": now,
            "kubernetes.io": {"namespace": ns, "serviceaccount": {"name": "x"}},
        })
    }

    fn sign(claims: &Value, pem: &str, alg: Algorithm) -> String {
        let mut header = Header::new(alg);
        header.kid = Some("k1".to_string());
        jsonwebtoken::encode(
            &header,
            claims,
            &EncodingKey::from_ec_pem(pem.as_bytes()).unwrap(),
        )
        .unwrap()
    }

    async fn reason(token: &str) -> String {
        verify(&[issuer()], token).await.unwrap_err().reason
    }

    #[tokio::test]
    async fn first_matching_rule_decides_group_and_tags() {
        let id = verify(
            &[issuer()],
            &sign(
                &claims("system:serviceaccount:agents:gpu", "agents"),
                PRIV,
                Algorithm::ES256,
            ),
        )
        .await
        .unwrap();
        assert_eq!(
            (id.worker_group.as_str(), id.tags),
            ("gpu", vec!["gpu".to_string()])
        );
        // A dotted key (`kubernetes.io`) is reached through a nested path.
        let id = verify(
            &[issuer()],
            &sign(
                &claims("system:serviceaccount:agents:etl", "agents"),
                PRIV,
                Algorithm::ES256,
            ),
        )
        .await
        .unwrap();
        assert_eq!(id.worker_group, "agents");
        assert!(reason(&sign(
            &claims("system:serviceaccount:other:etl", "other"),
            PRIV,
            Algorithm::ES256
        ))
        .await
        .starts_with("no rule matches"));
    }

    #[tokio::test]
    async fn refuses_wrong_iss_aud_exp_key_and_alg() {
        let ok = claims("system:serviceaccount:agents:gpu", "agents");
        let with = |k: &str, v: Value| {
            let mut c = ok.clone();
            c[k] = v;
            sign(&c, PRIV, Algorithm::ES256)
        };
        assert!(reason(&with("iss", json!("https://evil")))
            .await
            .contains("not trusted"));
        assert!(reason(&with("aud", json!("other"))).await.contains("aud"));
        assert!(reason(&with("exp", json!(1_000_000)))
            .await
            .contains("expired"));
        assert!(reason(&sign(&ok, OTHER_PRIV, Algorithm::ES256))
            .await
            .contains("signature"));

        let hs = jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &ok,
            &EncodingKey::from_secret(b"s"),
        )
        .unwrap();
        assert!(reason(&hs).await.contains("not accepted"));
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        let none = format!(
            "{}.{}.",
            URL_SAFE_NO_PAD.encode(br#"{"alg":"none","kid":"k1"}"#),
            URL_SAFE_NO_PAD.encode(ok.to_string())
        );
        assert!(reason(&none).await.contains("not accepted"));
    }

    #[test]
    fn rule_without_claims_never_matches() {
        let rules = vec![AgentWorkloadIdentityRule {
            claims: BTreeMap::new(),
            worker_group: "g".to_string(),
            tags: vec!["t".to_string()],
        }];
        assert!(matching_rule(&rules, &json!({"sub": "x"})).is_none());
        let mut cfg = issuer();
        cfg.rules = rules;
        assert!(validate_issuers(&[cfg]).is_err());
    }
}
