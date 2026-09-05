//! The guest JWT contract: what a token minted by an embedding customer's own backend
//! must carry to open one guest-mode app, and how it is verified against the workspace's
//! configured key (or, off cloud, the instance issuer). Deliberately narrower than the external JWT scheme
//! (`jwt_ext_`), whose claims can assert admin, groups and folders: a guest key can
//! only ever mint guests, whatever the token says.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::{
    jwk::{AlgorithmParameters, Jwk, PublicKeyUse},
    Algorithm, DecodingKey, Validation,
};
use quick_cache::sync::Cache;
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::DB;

/// A token is honoured at most this long past its issue, however far its `exp` lies:
/// a guest's expiry is its only revocation, and a long-lived token minted by mistake
/// would otherwise stay valid until it leaked.
pub const MAX_LIFETIME_SECS: u64 = 24 * 60 * 60;

/// Bearer prefix. Stateless: no `token` row. Verified against the workspace's key (or, off
/// cloud, the instance issuer when the workspace set none) and resolved in the auth cache,
/// whose entry is short-lived (not the token's full `exp`) so a rotated key revokes within
/// minutes. See the arm in `windmill-api-auth`.
pub const BEARER_PREFIX: &str = "jwt_guest_";

const RSA_ALGORITHMS: [Algorithm; 6] = [
    Algorithm::RS256,
    Algorithm::RS384,
    Algorithm::RS512,
    Algorithm::PS256,
    Algorithm::PS384,
    Algorithm::PS512,
];
const EC_ALGORITHMS: [Algorithm; 2] = [Algorithm::ES256, Algorithm::ES384];

/// Every claim honoured. Extra claims are ignored; a missing one refuses the token.
/// `app_path` is mandatory: a token opens that one app, exactly as a signed-in guest
/// session does.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GuestJwtClaims {
    pub email: String,
    pub workspace_id: String,
    pub app_path: String,
    pub exp: u64,
    pub nbf: Option<u64>,
    pub iat: Option<u64>,
}

/// How a guest JWT is verified: the workspace's configured key (a PEM public key or a JWKS
/// URL), or, off cloud, the instance issuer (`JWT_EXT_JWKS_URL`) when the workspace set none —
/// see `key_source`. When neither is set the JWT is refused whatever it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestJwtKeySource {
    Pem(String),
    JwksUrl(String),
}

/// The instance-wide external JWT issuer (`JWT_EXT_JWKS_URL`, also used by `jwt_ext_`). Read
/// fresh rather than cached so it is testable and picks up config regardless of init order.
fn instance_ext_jwks_url() -> Option<String> {
    std::env::var("JWT_EXT_JWKS_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

pub async fn key_source(db: &DB, w_id: &str) -> Result<Option<GuestJwtKeySource>> {
    let row = sqlx::query!(
        "SELECT guest_jwt_public_key, guest_jwt_jwks_url FROM workspace_settings WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(db)
    .await
    .map_err(|e| Error::internal_err(format!("reading guest JWT key of {w_id}: {e:#}")))?;
    let per_workspace = row.and_then(|r| match (r.guest_jwt_public_key, r.guest_jwt_jwks_url) {
        (Some(pem), _) => Some(GuestJwtKeySource::Pem(pem)),
        (None, Some(url)) => Some(GuestJwtKeySource::JwksUrl(url)),
        (None, None) => None,
    });
    if per_workspace.is_some() {
        return Ok(per_workspace);
    }
    // No workspace key: fall back to the instance issuer, so an operator running one issuer for
    // both `jwt_ext_` and guests configures it once. Verifying it (and granting a *guest*) is
    // done here in CE; granting a full login from it stays EE (`jwt_ext_`). Not on the shared
    // cloud, where one instance issuer must not be trusted to mint guests in every tenant's
    // workspace — there the per-workspace key is the only source.
    if !*crate::worker::CLOUD_HOSTED {
        if let Some(url) = instance_ext_jwks_url() {
            return Ok(Some(GuestJwtKeySource::JwksUrl(url)));
        }
    }
    Ok(None)
}

/// Parse a PEM public key and the algorithms it may verify: RSA keys the RS/PS family,
/// EC keys the ES family. Anything symmetric has no PEM form, so HS* is unreachable
/// from here by construction; the JWKS path refuses it explicitly.
pub fn decoding_key_from_pem(pem: &str) -> Result<(DecodingKey, &'static [Algorithm])> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use spki::der::Decode;
    // The key is admin-set into an unbounded `TEXT` column and reparsed on every guest-JWT
    // request; a well-formed key with an oversized modulus would pass the checks below. Refuse
    // one larger than any real public key before decoding or storing it. Measure the untrimmed
    // input: the endpoint stores what the admin sent, so whitespace padding counts too.
    if pem.len() > MAX_GUEST_PEM_LEN {
        return Err(Error::BadRequest(format!(
            "guest key is longer than {MAX_GUEST_PEM_LEN} bytes"
        )));
    }
    let pem = pem.trim();
    // A verification key must be public. jsonwebtoken 8.3 keys the public/private distinction
    // off the PEM label alone and never inspects the DER, so private material relabelled
    // `PUBLIC KEY` would be stored and then served back through the settings response. Decode
    // the body leniently, as jsonwebtoken does (tolerating any wrapping the strict RFC 7468
    // decoder would refuse), then require it to be a public-key structure: an SPKI (RSA or EC)
    // or a PKCS#1 RSA public key. Private-key DER satisfies neither.
    let der = STANDARD
        .decode(
            pem.lines()
                .filter(|l| !l.trim_start().starts_with("-----"))
                .flat_map(|l| l.split_whitespace())
                .collect::<String>(),
        )
        .map_err(|e| Error::BadRequest(format!("guest key is not valid PEM: {e}")))?;
    let is_public = spki::SubjectPublicKeyInfoRef::from_der(&der).is_ok()
        || pkcs1::RsaPublicKey::from_der(&der).is_ok();
    if !is_public {
        return Err(Error::BadRequest(
            "expected an RSA or EC public key in PEM form (-----BEGIN PUBLIC KEY-----)".to_string(),
        ));
    }
    if let Ok(key) = DecodingKey::from_rsa_pem(pem.as_bytes()) {
        return Ok((key, &RSA_ALGORITHMS));
    }
    if let Ok(key) = DecodingKey::from_ec_pem(pem.as_bytes()) {
        return Ok((key, &EC_ALGORITHMS));
    }
    Err(Error::BadRequest(
        "not an RSA or EC public key in PEM form (expected -----BEGIN PUBLIC KEY-----)".to_string(),
    ))
}

/// The algorithms a JWKS key may verify, or `None` if the key is unusable here: a
/// symmetric key (HS*, a shared secret the embedder would then have to hold), an
/// unsupported family, or a key not marked for signatures. A key that names its `alg`
/// pins that one; an RSA key that omits it accepts the whole RSA family, and an EC key
/// the algorithm its curve implies, mirroring how a PEM key is accepted.
pub fn jwk_algorithms(jwk: &Jwk) -> Option<Vec<Algorithm>> {
    if jwk.common.public_key_use.is_some()
        && jwk.common.public_key_use != Some(PublicKeyUse::Signature)
    {
        return None;
    }
    // A key that lists its operations must allow verifying signatures; otherwise it is
    // published for something else (encryption, key wrapping) and is not ours to use.
    if jwk
        .common
        .key_operations
        .as_ref()
        .is_some_and(|ops| !ops.contains(&jsonwebtoken::jwk::KeyOperations::Verify))
    {
        return None;
    }
    match (&jwk.algorithm, jwk.common.algorithm) {
        (AlgorithmParameters::RSA(_), Some(alg)) if RSA_ALGORITHMS.contains(&alg) => {
            Some(vec![alg])
        }
        (AlgorithmParameters::RSA(_), None) => Some(RSA_ALGORITHMS.to_vec()),
        (AlgorithmParameters::EllipticCurve(_), Some(alg)) if EC_ALGORITHMS.contains(&alg) => {
            Some(vec![alg])
        }
        (AlgorithmParameters::EllipticCurve(p), None) => match p.curve {
            jsonwebtoken::jwk::EllipticCurve::P256 => Some(vec![Algorithm::ES256]),
            jsonwebtoken::jwk::EllipticCurve::P384 => Some(vec![Algorithm::ES384]),
            _ => None,
        },
        _ => None,
    }
}

/// Verify `token` against `key`, honouring only the accepted `algorithms`, and check
/// every claim rule that needs no database: signature, `exp` (mandatory), `nbf` and
/// `iat` when present, the lifetime cap, that the token names `w_id`, that `email` is a
/// valid address bounded to 254 bytes, and that `app_path` carries no scope metacharacter.
pub fn verify(
    token: &str,
    key: &DecodingKey,
    algorithms: &[Algorithm],
    w_id: &str,
) -> Result<GuestJwtClaims> {
    let mut validation = Validation::new(algorithms[0]);
    validation.algorithms = algorithms.to_vec();
    validation.validate_nbf = true;
    let claims = jsonwebtoken::decode::<GuestJwtClaims>(token, key, &validation)
        .map_err(|e| Error::NotAuthorized(format!("guest JWT refused: {e}")))?
        .claims;
    let now = jsonwebtoken::get_current_timestamp();
    if claims.exp > now + MAX_LIFETIME_SECS {
        return Err(Error::NotAuthorized(format!(
            "guest JWT refused: exp is more than {MAX_LIFETIME_SECS} seconds ahead"
        )));
    }
    if let Some(iat) = claims.iat {
        if iat > now + validation.leeway {
            return Err(Error::NotAuthorized(
                "guest JWT refused: iat is in the future".to_string(),
            ));
        }
        if claims.exp.saturating_sub(iat) > MAX_LIFETIME_SECS {
            return Err(Error::NotAuthorized(format!(
                "guest JWT refused: lifetime exceeds {MAX_LIFETIME_SECS} seconds"
            )));
        }
    }
    if claims.workspace_id != w_id {
        return Err(Error::NotAuthorized(
            "guest JWT refused: workspace_id does not match the workspace".to_string(),
        ));
    }
    // The email becomes the guest's username; require the address shape the `usr` table
    // accepts (`VALID_EMAIL`), so it always carries an `@` and `username_to_permissioned_as`
    // can only ever read it as its own principal, never a `u/<user>` or `g/<group>`.
    // Bound it to fit the `guest_activity.email` column: a longer one fails that insert
    // while the guest is admitted uncounted.
    if !crate::users::VALID_EMAIL.is_match(&claims.email) || claims.email.len() > 254 {
        return Err(Error::NotAuthorized(
            "guest JWT refused: email is not a valid, bounded email address".to_string(),
        ));
    }
    // The app path is spliced into `apps:read:<path>` and `apps:run:<path>` scopes, whose
    // grammar reserves `:`, `,`, `*` and a leading `/`; refuse those, the same guard
    // `guest_session_scopes` applies at the mint. App paths may carry spaces and `@`.
    if !crate::auth::is_scope_literal_path(&claims.app_path) {
        return Err(Error::NotAuthorized(
            "guest JWT refused: app_path is empty or cannot be scoped (`:`, `,`, `*` are \
             reserved and a leading `/` never matches a route)"
                .to_string(),
        ));
    }
    Ok(claims)
}

struct JwksEntry {
    keys: Arc<HashMap<String, Jwk>>,
    /// When this entry stops being served and the next request refetches. A good fetch
    /// is served for `JWKS_TTL`, a failed one for `JWKS_NEGATIVE_TTL` (serving the last
    /// good keys if there are any), so an unreachable issuer cannot be turned into one
    /// outbound fetch per request by unauthenticated traffic.
    expires_at: Instant,
    /// When these keys were last fetched successfully. Stale keys are served only within
    /// `JWKS_MAX_STALE` of this, and a stale re-serve preserves it, so a revoked `kid` or an
    /// unreachable issuer stops minting new guest JWTs after a bounded window, not forever.
    fetched_at: Instant,
}

lazy_static::lazy_static! {
    static ref JWKS_CACHE: Cache<String, Arc<JwksEntry>> = Cache::new(200);
    /// Per-URL fetch lock: only one refresh per URL is in flight at a time, so a cold
    /// or stale entry under a burst triggers one fetch, not one per request. This is a
    /// plain map, not a capacity-bounded `Cache`: a `Cache` could evict a lock whose fetch
    /// is still running, and the next request for that URL would then mint a fresh lock and
    /// start a duplicate fetch, so cycling past 200 cold URLs could defeat single-flight and
    /// storm the issuers. `JwksFetchLock` drops each entry once its last holder is gone, so
    /// the map only ever holds the fetches in flight (bounded by concurrent distinct URLs).
    static ref JWKS_FETCH_LOCKS: std::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>> =
        std::sync::Mutex::new(HashMap::new());
}

/// How long a good key set is served before a refresh; also the lag before a
/// rotated-in `kid` is picked up. The cadence of the instance-level external JWKS.
const JWKS_TTL: Duration = Duration::from_secs(15 * 60);
/// How long a failed fetch is remembered before retrying, so an unreachable issuer is
/// hit at most once per this interval however much guest-JWT traffic arrives.
const JWKS_NEGATIVE_TTL: Duration = Duration::from_secs(30);
/// The absolute age past which cached keys are no longer served, even while revalidating:
/// once an issuer has been unreachable (or has revoked a `kid`) for this long, its old keys
/// stop authenticating and the request fails closed rather than trusting them indefinitely.
const JWKS_MAX_STALE: Duration = Duration::from_secs(60 * 60);
/// A JWKS body larger than this is refused rather than buffered: the URL is admin-set
/// but the server it names may be attacker-controlled, and a real key set is a few KB.
const JWKS_MAX_BYTES: usize = 1 << 20;

/// A cache entry retains only the usable signing keys, but nothing else bounds their combined
/// size: the response cap is 1 MiB and `from_jwk` decodes `n`/`e`/`x`/`y` without a length
/// limit, so one entry could retain ~1 MiB (a few hundred MB across the 200-entry LRU). Cap the
/// retained material instead; a real set is a few KB, so this is invisible to a legitimate one.
const JWKS_MAX_RETAINED_BYTES: usize = 64 * 1024;

/// The retained-bytes cap counts key material; this caps the number of keys so the per-key
/// fixed cost (each `Jwk` and its map slot) is bounded too, not just their string content.
const JWKS_MAX_KEYS: usize = 50;

/// Both JWKS caches key on the admin-supplied URL string. The column is unbounded `TEXT`, so
/// without this a workspace admin could grow the caches by the URL bytes alone. Enforced in
/// `fetch_jwks`, which `edit_guest_jwt_key` validates through, so an overlong URL is never
/// stored; the cache only ever sees a URL that was stored, hence a bounded one.
const MAX_JWKS_URL_LEN: usize = 2048;

/// A guest verification key is admin-set into an unbounded `TEXT` column and reparsed on every
/// guest-JWT request. A real public key PEM is under a few KB (RSA-16384 SPKI is ~2.8 KB), so
/// this bounds the stored and reparsed bytes without refusing any real key.
const MAX_GUEST_PEM_LEN: usize = 8 * 1024;

/// A guest JWT is refused past this before any signature work or caching: the auth cache keys
/// on the bearer, so an oversized token (unauthenticated at this point) would otherwise be
/// decoded and, if it verified, cached at its full size. A real JWT is well under this.
pub const MAX_GUEST_JWT_LEN: usize = 8 * 1024;

/// Fetch a JWKS, keeping only the keys usable here. A workspace-admin URL is validated
/// against private ranges and the connect pinned to the validated addresses; redirects are
/// not followed for the same reason. The instance issuer (`JWT_EXT_JWKS_URL`) is exempt from
/// those restrictions — it is operator-configured and trusted (it also backs `jwt_ext_`), so a
/// self-hosted internal (http/private) issuer that works for `jwt_ext_` works for guests too.
/// The body is read with a cap so a hostile endpoint cannot exhaust memory.
pub async fn fetch_jwks(url: &str) -> Result<HashMap<String, Jwk>> {
    use futures::StreamExt;
    if url.len() > MAX_JWKS_URL_LEN {
        return Err(Error::BadRequest(format!(
            "JWKS URL is longer than {MAX_JWKS_URL_LEN} bytes"
        )));
    }
    let resp = if instance_ext_jwks_url().as_deref() == Some(url) {
        // Operator-trusted issuer: fetch it with the same permissive client `jwt_ext_` uses
        // (follows redirects, honors ACCEPT_INVALID_CERTS), so an issuer that works for
        // `jwt_ext_` through a redirect or an approved self-signed cert works for guests too.
        crate::utils::HTTP_CLIENT_PERMISSIVE.get(url).send().await
    } else {
        // Workspace-admin URL: validate (https + private ranges), pin the connect to the
        // validated addresses, and do not follow redirects — all against SSRF.
        let client = crate::ssrf::validate_guest_jwks_url(url)
            .await
            .map_err(|e| Error::BadRequest(format!("JWKS URL is not allowed: {e}")))?
            .apply_dns_pinning(crate::utils::configure_client(reqwest::ClientBuilder::new()))
            .user_agent("windmill/beta")
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| Error::internal_err(format!("building JWKS client: {e}")))?;
        client.get(url).send().await
    }
    .and_then(|r| r.error_for_status())
    .map_err(|e| Error::BadRequest(format!("could not fetch JWKS: {e}")))?;
    let mut stream = resp.bytes_stream();
    let mut body: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| Error::BadRequest(format!("reading JWKS: {e}")))?;
        if body.len() + chunk.len() > JWKS_MAX_BYTES {
            return Err(Error::BadRequest(format!(
                "JWKS is larger than {JWKS_MAX_BYTES} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    parse_jwks_keys(&body)
}

/// The usable signing keys in a JWKS body, by `kid`. Each key is parsed on its own and
/// one that does not model as a JWT key is skipped, not fatal: a set may legitimately
/// carry an encryption key (say `alg: "RSA-OAEP"`, which is not in jsonwebtoken's signing
/// `Algorithm` enum and would fail whole-set deserialization) beside its signing keys.
///
/// A key is kept only if its material actually decodes (`DecodingKey::from_jwk`): jsonwebtoken
/// carries `n`/`e`/`x`/`y` as strings and defers decoding to auth time, so without this a JWKS
/// whose only key is malformed would be accepted at save time and fail every token later.
fn parse_jwks_keys(body: &[u8]) -> Result<HashMap<String, Jwk>> {
    let set: serde_json::Value = serde_json::from_slice(body)
        .map_err(|e| Error::BadRequest(format!("JWKS is not JSON: {e}")))?;
    let entries = set
        .get("keys")
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::BadRequest("JWKS has no `keys` array".to_string()))?;
    let keys: HashMap<String, Jwk> = entries
        .iter()
        .filter_map(|entry| serde_json::from_value::<Jwk>(entry.clone()).ok())
        .filter(|jwk| jwk_algorithms(jwk).is_some())
        .filter(|jwk| DecodingKey::from_jwk(jwk).is_ok())
        .filter_map(|jwk| jwk.common.key_id.clone().map(|kid| (kid, jwk)))
        .collect();
    if keys.is_empty() {
        return Err(Error::BadRequest(
            "JWKS holds no RSA or EC signing key with a kid".to_string(),
        ));
    }
    // Bound the usable keys two ways, both measured after filtering so a large mixed-use set
    // (many encryption keys, few signing) is not refused for its size: their count (the per-key
    // fixed cost) and their combined material bytes.
    if keys.len() > JWKS_MAX_KEYS {
        return Err(Error::BadRequest(format!(
            "JWKS holds more than {JWKS_MAX_KEYS} usable signing keys"
        )));
    }
    let retained: usize = keys
        .values()
        .filter_map(|jwk| serde_json::to_vec(jwk).ok().map(|v| v.len()))
        .sum();
    if retained > JWKS_MAX_RETAINED_BYTES {
        return Err(Error::BadRequest(format!(
            "JWKS signing keys retain more than {JWKS_MAX_RETAINED_BYTES} bytes"
        )));
    }
    Ok(keys)
}

/// A cached entry that holds no keys is a remembered failure; serving it would report
/// an unreachable issuer as an unknown `kid`. Map it to an issuer-unreachable error.
fn servable(entry: Arc<JwksEntry>) -> Result<Arc<JwksEntry>> {
    if entry.keys.is_empty() {
        Err(Error::NotAuthorized(
            "guest JWT refused: the JWKS issuer is unreachable".to_string(),
        ))
    } else {
        Ok(entry)
    }
}

/// A held single-flight lock for one JWKS URL. Dropping it removes the URL from
/// `JWKS_FETCH_LOCKS` once no other holder remains, so the registry never keeps a lock past
/// its fetch and stays bounded by the number of fetches in flight, not by URLs ever seen.
struct JwksFetchLock {
    url: String,
    lock: Arc<tokio::sync::Mutex<()>>,
}

impl JwksFetchLock {
    /// The lock for `url`, created on first use. Callers sharing a URL get the same `Arc`,
    /// so one holds the inner mutex and fetches while the rest wait on it.
    fn acquire(url: &str) -> Self {
        let mut map = JWKS_FETCH_LOCKS.lock().unwrap();
        let lock = map
            .entry(url.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone();
        JwksFetchLock { url: url.to_string(), lock }
    }
}

impl Drop for JwksFetchLock {
    fn drop(&mut self) {
        let mut map = JWKS_FETCH_LOCKS.lock().unwrap();
        // Clones are only taken while holding this same map lock, so the count is stable
        // here: two Arcs (the map's and this one's) means we are the last holder and the
        // entry can go; more means another request still needs it and will remove it in turn.
        if map
            .get(&self.url)
            .is_some_and(|lock| Arc::strong_count(lock) <= 2)
        {
            map.remove(&self.url);
        }
    }
}

/// Refresh a URL's JWKS off the request path, under the single-flight lock. A held
/// lock means a refresh is already running, so this is a no-op. A failed refresh
/// leaves the served stale keys in place rather than dropping them.
fn spawn_jwks_refresh(url: String) {
    tokio::spawn(async move {
        let fetch_lock = JwksFetchLock::acquire(&url);
        let Ok(_guard) = fetch_lock.lock.try_lock() else {
            return;
        };
        match fetch_jwks(&url).await {
            Ok(keys) => {
                let now = Instant::now();
                JWKS_CACHE.insert(
                    url,
                    Arc::new(JwksEntry {
                        keys: Arc::new(keys),
                        expires_at: now + JWKS_TTL,
                        fetched_at: now,
                    }),
                );
            }
            Err(e) => tracing::warn!("guest JWKS background refresh failed for {url}: {e:#}"),
        }
    });
}

/// The workspace's JWKS. A fresh entry is served directly; a stale-but-good one is
/// served while a refresh runs off the request path (`spawn_jwks_refresh`), so a slow
/// issuer never stalls a request. Only a cold or negative entry blocks, under a per-URL
/// lock so a burst triggers one fetch; a failed fetch there caches a short-lived empty
/// entry that reads as "issuer unreachable", so an unreachable issuer is hit at most
/// once per `JWKS_NEGATIVE_TTL`. Fetches follow a schedule, never a per-request,
/// attacker-chosen `kid`.
async fn cached_jwks(url: &str) -> Result<Arc<JwksEntry>> {
    if let Some(entry) = JWKS_CACHE.get(url) {
        // Keys past JWKS_MAX_STALE are never served, even while revalidating: a stale re-serve
        // bumps expires_at but keeps fetched_at, so a persistently failing refresh would
        // otherwise serve revoked keys forever. Too-old keys fall through to the blocking
        // refresh, which fails closed if the issuer is still down.
        if entry.fetched_at.elapsed() < JWKS_MAX_STALE {
            if entry.expires_at > Instant::now() {
                return servable(entry);
            }
            // Stale but still holds good keys: serve them now and refresh off the request
            // path, so a slow or hanging issuer adds no latency. Bump the entry first so the
            // refresh window does not spawn a task per request. A negative (empty) entry
            // falls through to the blocking refresh below.
            if !entry.keys.is_empty() {
                let served = Arc::new(JwksEntry {
                    keys: entry.keys.clone(),
                    expires_at: Instant::now() + JWKS_NEGATIVE_TTL,
                    fetched_at: entry.fetched_at,
                });
                JWKS_CACHE.insert(url.to_string(), served.clone());
                spawn_jwks_refresh(url.to_string());
                return Ok(served);
            }
        }
    }
    // Cold or negative entry, nothing good to serve: block on a single-flight refresh.
    // `JwksFetchLock::acquire` hands cold requests the same lock, so they share one fetch.
    let fetch_lock = JwksFetchLock::acquire(url);
    let _guard = fetch_lock.lock.lock().await;
    // Another task may have refreshed while we waited for the lock (honour the age limit too,
    // so a concurrent stale re-serve of too-old keys is not mistaken for a fresh entry).
    if let Some(entry) = JWKS_CACHE.get(url) {
        if entry.fetched_at.elapsed() < JWKS_MAX_STALE && entry.expires_at > Instant::now() {
            return servable(entry);
        }
    }
    match fetch_jwks(url).await {
        Ok(keys) => {
            let now = Instant::now();
            let entry = Arc::new(JwksEntry {
                keys: Arc::new(keys),
                expires_at: now + JWKS_TTL,
                fetched_at: now,
            });
            JWKS_CACHE.insert(url.to_string(), entry.clone());
            Ok(entry)
        }
        Err(e) => {
            // Reached with no servable keys: either nothing cached, or keys too old to trust.
            // Cache a short negative entry so the next requests do not each refetch, and
            // surface the error, so an issuer that revoked a key or went down fails closed.
            JWKS_CACHE.insert(
                url.to_string(),
                Arc::new(JwksEntry {
                    keys: Arc::new(HashMap::new()),
                    expires_at: Instant::now() + JWKS_NEGATIVE_TTL,
                    fetched_at: Instant::now(),
                }),
            );
            Err(e)
        }
    }
}

/// The key a token's header selects from the workspace's JWKS, by `kid`, and the
/// algorithms it may verify. An unknown `kid` is refused against the cached set rather
/// than triggering a fetch, so varying `kid` cannot drive outbound requests; a
/// genuinely rotated-in key is picked up within `JWKS_TTL`.
pub async fn jwks_key_for(url: &str, token: &str) -> Result<(DecodingKey, Vec<Algorithm>)> {
    let header = jsonwebtoken::decode_header(token)
        .map_err(|e| Error::NotAuthorized(format!("guest JWT refused: {e}")))?;
    let kid = header.kid.ok_or_else(|| {
        Error::NotAuthorized("guest JWT refused: no kid in the header".to_string())
    })?;
    let entry = cached_jwks(url).await?;
    let jwk = entry.keys.get(&kid).ok_or_else(|| {
        Error::NotAuthorized(format!("guest JWT refused: kid {kid} is not in the JWKS"))
    })?;
    let algs = jwk_algorithms(jwk).ok_or_else(|| {
        Error::NotAuthorized(format!("guest JWT refused: kid {kid} is not a signing key"))
    })?;
    let key = DecodingKey::from_jwk(jwk)
        .map_err(|e| Error::internal_err(format!("unusable JWK {kid}: {e}")))?;
    Ok((key, algs))
}

/// Verify `token` for `w_id` against whatever key the workspace configured. A PEM key
/// ignores `kid`; a JWKS selects by it.
pub async fn verify_for_workspace(db: &DB, w_id: &str, token: &str) -> Result<GuestJwtClaims> {
    if token.len() > MAX_GUEST_JWT_LEN {
        return Err(Error::NotAuthorized(format!(
            "guest JWT refused: token is longer than {MAX_GUEST_JWT_LEN} bytes"
        )));
    }
    let Some(source) = key_source(db, w_id).await? else {
        return Err(Error::NotAuthorized(format!(
            "guest JWT refused: workspace {w_id} has no guest JWT key"
        )));
    };
    match source {
        GuestJwtKeySource::Pem(pem) => {
            let (key, algorithms) = decoding_key_from_pem(&pem)?;
            verify(token, &key, algorithms, w_id)
        }
        GuestJwtKeySource::JwksUrl(url) => {
            let (key, algorithms) = jwks_key_for(&url, token).await?;
            verify(token, &key, &algorithms, w_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes the tests that mutate the process-wide `ALLOW_PRIVATE_GUEST_JWKS_URLS`, so a
    /// concurrent run cannot clear it out from under another (mirrors `ssrf.rs`'s test lock).
    static TEST_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn jwk(v: serde_json::Value) -> Jwk {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn rsa_key_with_alg_pins_it() {
        let k = jwk(serde_json::json!({"kty":"RSA","alg":"RS384","n":"aa","e":"AQAB"}));
        assert_eq!(jwk_algorithms(&k), Some(vec![Algorithm::RS384]));
    }

    #[test]
    fn rsa_key_without_alg_takes_the_whole_family() {
        // The bug this pins: an alg-less RSA key must not be forced to RS256, which
        // would reject valid RS384/512 or PS* tokens.
        let k = jwk(serde_json::json!({"kty":"RSA","n":"aa","e":"AQAB"}));
        assert_eq!(jwk_algorithms(&k), Some(RSA_ALGORITHMS.to_vec()));
    }

    #[test]
    fn ec_key_takes_its_curve_algorithm() {
        let k = jwk(serde_json::json!({"kty":"EC","crv":"P-256","x":"aa","y":"bb"}));
        assert_eq!(jwk_algorithms(&k), Some(vec![Algorithm::ES256]));
    }

    #[test]
    fn symmetric_key_is_refused() {
        let k = jwk(serde_json::json!({"kty":"oct","k":"c2VjcmV0"}));
        assert_eq!(jwk_algorithms(&k), None);
    }

    #[test]
    fn a_key_marked_for_encryption_is_refused() {
        let k = jwk(serde_json::json!({"kty":"RSA","use":"enc","n":"aa","e":"AQAB"}));
        assert_eq!(jwk_algorithms(&k), None);
    }

    #[test]
    fn a_mixed_use_jwks_keeps_only_the_signing_keys() {
        // An encryption key (RSA-OAEP is not in jsonwebtoken's signing Algorithm enum)
        // beside a signing key must not fail the whole set. The signing key carries real
        // coordinates so it survives the material check parse_jwks_keys now applies.
        let body = serde_json::json!({
            "keys": [
                {"kty":"RSA","alg":"RSA-OAEP","kid":"enc","use":"enc","n":"aa","e":"AQAB"},
                {"kty":"EC","crv":"P-256","kid":"sig","x":PUB1_X,"y":PUB1_Y}
            ]
        })
        .to_string();
        let keys = parse_jwks_keys(body.as_bytes()).expect("the signing key survives");
        assert!(keys.contains_key("sig"));
        assert!(!keys.contains_key("enc"));
    }

    #[test]
    fn a_jwks_with_only_malformed_key_material_is_refused() {
        // Metadata (kty/alg/use) is fine but `n` is not valid base64url, so the key is
        // unusable. Since it is the only key, configuring this JWKS must fail at save time
        // rather than persist a URL whose tokens all fail later.
        let body = serde_json::json!({
            "keys": [{"kty":"RSA","kid":"k1","n":"not base64url!!","e":"AQAB"}]
        })
        .to_string();
        assert!(parse_jwks_keys(body.as_bytes()).is_err());
    }

    #[test]
    fn too_many_usable_keys_is_refused() {
        // All keys are usable (real coordinates), so this trips the count cap, not the
        // empty-set path. Small material, so it is the count that refuses them, not the bytes.
        let keys: Vec<_> = (0..=JWKS_MAX_KEYS)
            .map(|i| {
                serde_json::json!({"kty":"EC","crv":"P-256","kid":format!("k{i}"),"x":PUB1_X,"y":PUB1_Y})
            })
            .collect();
        let body = serde_json::json!({ "keys": keys }).to_string();
        let err = parse_jwks_keys(body.as_bytes()).unwrap_err().to_string();
        assert!(err.contains("usable signing keys"), "{err}");
    }

    #[test]
    fn keys_retaining_too_many_bytes_is_refused() {
        // Under the key count cap, but the material of these usable RSA keys exceeds the byte
        // cap. `from_jwk` decodes any-length `n`, so this is reachable without the count cap.
        let big_n = "A".repeat(2000);
        let keys: Vec<_> = (0..40)
            .map(|i| serde_json::json!({"kty":"RSA","kid":format!("k{i}"),"n":big_n,"e":"AQAB"}))
            .collect();
        let body = serde_json::json!({ "keys": keys }).to_string();
        let err = parse_jwks_keys(body.as_bytes()).unwrap_err().to_string();
        assert!(err.contains("retain more than"), "{err}");
    }

    #[tokio::test]
    async fn an_overlong_jwks_url_is_refused() {
        // The cache keys on the URL string, so an unbounded URL is refused before it is cached.
        // Assert the length error specifically: a bogus URL would fail the fetch regardless.
        let url = format!(
            "https://issuer.example.com/{}",
            "a".repeat(MAX_JWKS_URL_LEN)
        );
        let err = fetch_jwks(&url).await.unwrap_err().to_string();
        assert!(err.contains("longer than"), "{err}");
    }

    #[test]
    fn an_oversized_pem_is_refused() {
        // A well-formed key body padded past the cap: refused for its length before decoding,
        // so an oversized-but-valid key cannot be stored and reparsed on every request.
        let big = format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----\n",
            "A".repeat(MAX_GUEST_PEM_LEN)
        );
        let err = decoding_key_from_pem(&big).err().unwrap().to_string();
        assert!(err.contains("longer than"), "{err}");
        // Whitespace padding must count: the endpoint stores the untrimmed value, so the cap is
        // measured before trimming rather than on the small trimmed key it would otherwise see.
        let padded = format!("{}{RSA_PUBLIC}", " ".repeat(MAX_GUEST_PEM_LEN));
        let err = decoding_key_from_pem(&padded).err().unwrap().to_string();
        assert!(err.contains("longer than"), "{err}");
    }

    #[test]
    fn key_ops_without_verify_is_refused() {
        let enc = jwk(serde_json::json!({"kty":"RSA","key_ops":["encrypt"],"n":"aa","e":"AQAB"}));
        assert_eq!(jwk_algorithms(&enc), None);
        let ver = jwk(serde_json::json!({"kty":"RSA","key_ops":["verify"],"n":"aa","e":"AQAB"}));
        assert_eq!(jwk_algorithms(&ver), Some(RSA_ALGORITHMS.to_vec()));
    }

    // PUB1's coordinates and its matching PKCS8 private key, for the JWKS-derived
    // verification test.
    const PUB1_X: &str = "zAfqyCh34iYOCW0vg4ejq_zzJlzLSZScjnVyPjLGTao";
    const PUB1_Y: &str = "RMKOIHOv8tWLnXf7-eMCodDnX038wCjD1sf9jVsf7oI";
    const PRIV1: &str = "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgu27S2DbSwUh8BmQb\n/i4/VhNdoXV7PJekhnoceMULYLihRANCAATMB+rIKHfiJg4JbS+Dh6Or/PMmXMtJ\nlJyOdXI+MsZNqkTCjiBzr/LVi513+/njAqHQ519N/MAow9bH/Y1bH+6C\n-----END PRIVATE KEY-----\n";

    #[test]
    fn a_jwks_key_verifies_a_real_token() {
        // The one path the PEM tests do not cover: a key rebuilt from a JWK verifies a
        // token signed by its private half, and the algorithms come from the JWK.
        let jwk = jwk(serde_json::json!({
            "kty": "EC", "crv": "P-256", "kid": "k1", "x": PUB1_X, "y": PUB1_Y
        }));
        let algs = jwk_algorithms(&jwk).expect("EC signing key");
        assert_eq!(algs, vec![Algorithm::ES256]);
        let key = jsonwebtoken::DecodingKey::from_jwk(&jwk).expect("usable JWK");
        let payload = serde_json::json!({
            "email": "g@example.com",
            "workspace_id": "ws",
            "app_path": "u/a/app",
            "exp": jsonwebtoken::get_current_timestamp() + 600,
        });
        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(Algorithm::ES256),
            &payload,
            &jsonwebtoken::EncodingKey::from_ec_pem(PRIV1.as_bytes()).unwrap(),
        )
        .unwrap();
        let out = verify(&token, &key, &algs, "ws").expect("verifies");
        assert_eq!(out.email, "g@example.com");
        // The workspace pin is part of verify.
        assert!(verify(&token, &key, &algs, "other-ws").is_err());
    }

    #[test]
    fn a_non_key_pem_is_rejected() {
        assert!(decoding_key_from_pem(
            "-----BEGIN CERTIFICATE-----\nnope\n-----END CERTIFICATE-----"
        )
        .is_err());
    }

    // A real RSA public key (SPKI). Its private counterpart is RSA_PKCS1_PRIVATE below.
    const RSA_PUBLIC: &str = "-----BEGIN PUBLIC KEY-----\n\
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAx3J0fQcHp2ZlMI4rCVsY\n\
tirATZPWyPD7exoYWPInhV5xjbY2Fe8IVFaZszQcQbCXZjBtFp2fj0tBTow8BeOy\n\
X9LJPyKeho/j68FycuDVg7JCzG0TWtsnh/V23WkrlKIfmMqS3+YUyFavROTcAN1T\n\
5BcFHLAi/4Q2qy0JjXBdZ8avelzZrQ/T67/Kcsoct/pvnEDT2YRsSbA7VMWaxWh8\n\
MYJ7GNV/10YT2c5CBJGSLbyRSVWk2IwfnM9Cl9n/5NE6TkSetYQ2xlqKTONp5W43\n\
UzW1NAeqKCxPQfN/ADjwW18nk2o7xj1kMF4rBlhsTm9ClE71nwi5NxsvMOdVxytZ\n\
ZQIDAQAB\n\
-----END PUBLIC KEY-----\n";

    // A complete, valid PKCS#1 RSA *private* key (the counterpart of RSA_PUBLIC) with its
    // armor relabelled `RSA PUBLIC KEY`. jsonwebtoken's from_rsa_pem accepts it under that
    // label; the structural check refuses it (a 9-field RSAPrivateKey is neither an SPKI nor
    // a 2-field RsaPublicKey). Complete on purpose: malformed DER would fail for the wrong
    // reason and let a real bypass through unnoticed.
    const RSA_PKCS1_PRIVATE_AS_PUBLIC: &str = "-----BEGIN RSA PUBLIC KEY-----\n\
MIIEogIBAAKCAQEAx3J0fQcHp2ZlMI4rCVsYtirATZPWyPD7exoYWPInhV5xjbY2\n\
Fe8IVFaZszQcQbCXZjBtFp2fj0tBTow8BeOyX9LJPyKeho/j68FycuDVg7JCzG0T\n\
Wtsnh/V23WkrlKIfmMqS3+YUyFavROTcAN1T5BcFHLAi/4Q2qy0JjXBdZ8avelzZ\n\
rQ/T67/Kcsoct/pvnEDT2YRsSbA7VMWaxWh8MYJ7GNV/10YT2c5CBJGSLbyRSVWk\n\
2IwfnM9Cl9n/5NE6TkSetYQ2xlqKTONp5W43UzW1NAeqKCxPQfN/ADjwW18nk2o7\n\
xj1kMF4rBlhsTm9ClE71nwi5NxsvMOdVxytZZQIDAQABAoIBAD+IbaQQM7d3Dj/X\n\
4cyyqJ4K40QzFmXfIfTWXLAkv0MkUR7XzsXQ5YHcLkzgCipAwxGp1m4wWs4OJmkL\n\
kek8XatZnYLPl9j8iBmm/zqp9Unk5JNzIYm9KwwLvMgOAvRvaopE6WGKTM9+kYls\n\
L8rUti7/yECZuSRU7Qc9KwBTrWVrXK+RBtBqZYQXb92BFxq0N3Qp+utLNdFcO5sW\n\
7d8gKp3ipQt5z9ZAB2pYMw7ZTzonF4C7HdyrbYXztvYrxuw1imMkQ9iFFhdn3/76\n\
qFR7XwaFrld8DECGaH/652kV6zaSQijbBTeXF4zsgwXY4BHMVmZKaXH5unw8Gbmo\n\
WCoLbLcCgYEA4vTo5kCiKXnlBp3W1Zpg4cml6Wzo0UDldF4kkuXQxhdQA38c0ise\n\
Cocf6qyAqz1L2TxQ/9WCL2oIP1AY9XqnQ0cJtYIosGWORz4tPe67M8inB/GBotFI\n\
pmQNVSIjqbgKVi0x+UzmFjitINFPf461lDdJTwhsv9TQRbHrXErvON8CgYEA4PhV\n\
GYMJu46tqFVtD/koWAQRLmeaZXhxP5lMSmQjdYCa3ys5lccvTlzoF9K8immMQkIx\n\
gyOazmEtFnK4IXmEY1wg2NIHuJM7/maoM2rozbjXBxsYM7Xw2QX7BHXqG6Ia/Bij\n\
ZaRJdumCVRJv7OshQTGuqDIzd3l5WEqg11XYgjsCgYAP3v6Wc3ijm+GXN9x5LYWO\n\
5JIUo8gYMgiZvaejGi0iXSj8RZxXWiqMo+xodc29q9itBVnIuj6TYD/ZZZmJOR2P\n\
R9128vYzd7aeZsu1JAe1VFfR52KgZzBEaoTAKlYCHVujsR9ohqckcKwyulBr5Cfw\n\
iHk47KbmN1SlOw7xclAOUwKBgAuxHEsdIk5bFe9fsTFZU51vaK0uuTl4zvntL6fW\n\
GHms21+p0W5VUcIS1gUW8LGI1r9CzWvxV8RODJfUEnm65QR870AVek0/aajJEQjL\n\
D5pRdutpnxJg7El7JBaRQj95Z0mexi8sIJ1LeXiOYr6/YZUPzfHz2fTlnUbXahCG\n\
55+tAoGAI811NTb7kuuIPYuj4raDW88QVNX2xB3+p9lXGolB4jPgsUEjgSvLgH9S\n\
Q/LwEBiCYVyii8MvWsIZpHvSGyOoty2p19/CAvrAOfpEVlnXQeiX+mh09p1mQbfM\n\
y9rTR828ADcaZ63Ej1oL4GcqmGhODxCLy1YKKcy0FHzChqPMV6g=\n\
-----END RSA PUBLIC KEY-----\n";

    #[test]
    fn a_private_pem_is_refused() {
        // A verification key must be public and must never be stored otherwise: it is served
        // back through the settings response. jsonwebtoken keys the public/private split off
        // the PEM label, so private DER relabelled with a public armor slips a label check;
        // only parsing the DER as a public-key structure (SPKI or PKCS#1 RSA public) refuses
        // it. A real public key still parses, so the guard is not vacuous.
        assert!(decoding_key_from_pem(RSA_PUBLIC).is_ok());
        // The same key with its body on one line (not 64-column wrapped): jsonwebtoken accepts
        // any wrapping, so the guard must too rather than lean on the strict RFC 7468 decoder.
        let body: String = RSA_PUBLIC
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect();
        let one_line = format!("-----BEGIN PUBLIC KEY-----\n{body}\n-----END PUBLIC KEY-----\n");
        assert!(decoding_key_from_pem(&one_line).is_ok());

        assert!(decoding_key_from_pem(PRIV1).is_err());
        // The same PKCS#8 EC private key, relabelled `PUBLIC KEY`.
        assert!(decoding_key_from_pem(&PRIV1.replace("PRIVATE KEY", "PUBLIC KEY")).is_err());
        assert!(decoding_key_from_pem(RSA_PKCS1_PRIVATE_AS_PUBLIC).is_err());
    }

    #[tokio::test]
    async fn a_concurrent_cold_burst_makes_one_jwks_fetch() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::io::AsyncWriteExt;
        let _env = TEST_ENV_LOCK.lock().await;
        // The stub listens on loopback, which SSRF validation refuses without this.
        unsafe { std::env::set_var("ALLOW_PRIVATE_GUEST_JWKS_URLS", "true") };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let body = format!(
            r#"{{"keys":[{{"kty":"EC","crv":"P-256","kid":"k1","x":"{PUB1_X}","y":"{PUB1_Y}"}}]}}"#
        );
        let hits = std::sync::Arc::new(AtomicUsize::new(0));
        let hits_srv = hits.clone();
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                hits_srv.fetch_add(1, Ordering::SeqCst);
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                // Delay so the other callers pile onto the single-flight lock before the
                // leader's fetch returns.
                tokio::time::sleep(Duration::from_millis(150)).await;
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            }
        });
        let url = format!("http://127.0.0.1:{}/jwks.json", addr.port());
        let mut handles = Vec::new();
        for _ in 0..10 {
            let u = url.clone();
            handles.push(tokio::spawn(async move {
                cached_jwks(&u).await.map(|e| e.keys.len())
            }));
        }
        for h in handles {
            assert_eq!(
                h.await.unwrap().unwrap(),
                1,
                "each caller resolves the one key"
            );
        }
        assert_eq!(
            hits.load(Ordering::SeqCst),
            1,
            "single-flight: a concurrent cold burst makes one fetch"
        );
        unsafe { std::env::remove_var("ALLOW_PRIVATE_GUEST_JWKS_URLS") };
    }

    #[tokio::test]
    async fn jwks_fetch_locks_are_shared_and_self_cleaning() {
        // The registry is a plain map, not a capacity-bounded cache: a cache could evict a
        // lock mid-fetch, letting a later request for that URL start a duplicate fetch. Pin
        // both halves of what keeps single-flight intact under many distinct URLs: the same
        // URL hands back one shared lock, and the entry is removed once its last holder drops
        // (so nothing evicts an in-flight lock and the map stays bounded by fetches in flight).
        let url = "https://example.test/jwks-lock-probe.json";
        {
            let a = JwksFetchLock::acquire(url);
            let b = JwksFetchLock::acquire(url);
            assert!(Arc::ptr_eq(&a.lock, &b.lock), "one lock per URL");
            assert!(JWKS_FETCH_LOCKS.lock().unwrap().contains_key(url));
        }
        assert!(
            !JWKS_FETCH_LOCKS.lock().unwrap().contains_key(url),
            "the lock is dropped once idle"
        );
    }

    #[tokio::test]
    async fn stale_jwks_keys_stop_being_served_past_the_grace_window() {
        let _env = TEST_ENV_LOCK.lock().await;
        unsafe { std::env::set_var("ALLOW_PRIVATE_GUEST_JWKS_URLS", "true") };
        // A dead loopback port, so every refresh fails (connection refused).
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let jwk = jwk(serde_json::json!(
            {"kty":"EC","crv":"P-256","kid":"k1","x":PUB1_X,"y":PUB1_Y}
        ));
        let keys: HashMap<String, Jwk> = [("k1".to_string(), jwk)].into_iter().collect();
        let stale = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();

        // Within the grace window, stale keys are still served while a (failing) refresh runs.
        let within = format!("http://127.0.0.1:{port}/within");
        JWKS_CACHE.insert(
            within.clone(),
            Arc::new(JwksEntry {
                keys: Arc::new(keys.clone()),
                expires_at: stale,
                fetched_at: Instant::now(),
            }),
        );
        assert!(
            cached_jwks(&within).await.is_ok(),
            "stale keys within the grace window are still served"
        );

        // Past the grace window, the keys are not served: the failing refresh fails closed.
        let beyond = format!("http://127.0.0.1:{port}/beyond");
        JWKS_CACHE.insert(
            beyond.clone(),
            Arc::new(JwksEntry {
                keys: Arc::new(keys),
                expires_at: stale,
                fetched_at: Instant::now()
                    .checked_sub(JWKS_MAX_STALE + Duration::from_secs(1))
                    .unwrap(),
            }),
        );
        assert!(
            cached_jwks(&beyond).await.is_err(),
            "keys past the grace window fail closed once the refresh fails"
        );
        unsafe { std::env::remove_var("ALLOW_PRIVATE_GUEST_JWKS_URLS") };
    }

    #[tokio::test]
    async fn a_plaintext_http_jwks_url_is_refused_by_default() {
        let _env = TEST_ENV_LOCK.lock().await;
        unsafe { std::env::remove_var("ALLOW_PRIVATE_GUEST_JWKS_URLS") };
        // The JWKS supplies the keys that authenticate guest JWTs; without the operator opt-in,
        // a plaintext URL (which an on-path attacker could replace) is refused for its scheme.
        assert!(matches!(
            crate::ssrf::validate_guest_jwks_url("http://issuer.example.com/jwks.json").await,
            Err(crate::ssrf::SsrfValidationError::HttpsRequired)
        ));
    }

    #[tokio::test]
    async fn the_instance_issuer_bypasses_the_https_and_private_restriction() {
        let _env = TEST_ENV_LOCK.lock().await;
        unsafe { std::env::remove_var("ALLOW_PRIVATE_GUEST_JWKS_URLS") };
        // The instance issuer is operator-trusted (it also backs jwt_ext_), so an http/private
        // URL is not refused for its scheme: it reaches the connect and fails there (dead port),
        // not at validation. A different URL is not the instance issuer and is still refused.
        let instance = "http://127.0.0.1:1/jwks.json";
        unsafe { std::env::set_var("JWT_EXT_JWKS_URL", instance) };
        let trusted = fetch_jwks(instance).await.err().unwrap().to_string();
        let other = fetch_jwks("http://127.0.0.1:1/other.json")
            .await
            .err()
            .unwrap()
            .to_string();
        unsafe { std::env::remove_var("JWT_EXT_JWKS_URL") };
        assert!(
            !trusted.contains("not allowed") && !trusted.contains("must use https"),
            "instance issuer skips validation: {trusted}"
        );
        assert!(
            other.contains("not allowed") || other.contains("https"),
            "a non-instance http url is still refused: {other}"
        );
    }
}
