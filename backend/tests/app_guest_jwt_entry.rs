//! Tests for the guest JWT entry: a guest that enters through a JWT the embedding
//! customer's own backend mints and signs, with no identity-provider round-trip.
//!
//! The key is a per-workspace setting (a PEM public key here), and the token is
//! verified per request against it. A JWT guest is the same identity as a signed-in
//! guest: no `usr` row, no `password` row, no seat, confined to the one app its
//! `app_path` names. These tests pin what a token must carry to be honoured, and the
//! refusals that keep the door narrow: wrong workspace, wrong key, expired, a
//! symmetric algorithm, an email that already has an account, an app not in guest
//! mode, and the workspace switch off.
//!
//! The keys are fixed test vectors (EC P-256, PKCS8), so signing is deterministic and
//! needs no key generation at runtime.

// Built with these like the sibling guest-execution suite: the guest run executes as
// the publisher through EE on-behalf-of code. CI builds with them.
#![cfg(all(feature = "enterprise", feature = "private"))]

use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";
const APP_PATH: &str = "u/test-user/guest_app";
const GUEST_EMAIL: &str = "guest@example.com";

// A P-256 keypair the workspace verifies against (PUB1), and a second private key
// (PRIV2) that it does not, for the wrong-key refusal.
const PRIV1: &str = "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgu27S2DbSwUh8BmQb\n/i4/VhNdoXV7PJekhnoceMULYLihRANCAATMB+rIKHfiJg4JbS+Dh6Or/PMmXMtJ\nlJyOdXI+MsZNqkTCjiBzr/LVi513+/njAqHQ519N/MAow9bH/Y1bH+6C\n-----END PRIVATE KEY-----\n";
const PUB1: &str = "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEzAfqyCh34iYOCW0vg4ejq/zzJlzL\nSZScjnVyPjLGTapEwo4gc6/y1Yudd/v54wKh0OdfTfzAKMPWx/2NWx/ugg==\n-----END PUBLIC KEY-----\n";
const PRIV2: &str = "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgjyhWYyI2+z5zTT0B\neI9EuJJ7v0tcNXhvHrq9y2AG1LihRANCAAS40dEdO+tTffhGt4YQv0dStkd6VcWN\n+CHI9QqZAHAJMsNS3Ld+sZe2M6Of0CNR300QJtfp4UIdEVbXBCIxL1D0\n-----END PRIVATE KEY-----\n";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Serialize)]
struct Claims {
    email: String,
    workspace_id: String,
    app_path: String,
    exp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    nbf: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iat: Option<u64>,
}

impl Claims {
    fn valid() -> Self {
        Claims {
            email: GUEST_EMAIL.to_string(),
            workspace_id: "test-workspace".to_string(),
            app_path: APP_PATH.to_string(),
            exp: now() + 3600,
            nbf: None,
            iat: None,
        }
    }
}

/// Sign as a bearer (`jwt_guest_<jwt>`). `priv_pem`/`alg` let a test sign with the
/// wrong key or a refused algorithm.
fn bearer(claims: &Claims, priv_pem: &str, alg: Algorithm) -> String {
    let key = match alg {
        Algorithm::HS256 => EncodingKey::from_secret(b"a-shared-secret"),
        _ => EncodingKey::from_ec_pem(priv_pem.as_bytes()).unwrap(),
    };
    let jwt = encode(&Header::new(alg), claims, &key).unwrap();
    format!("jwt_guest_{jwt}")
}

async fn enable_guests(port: u16, ws: &str, on: bool) -> anyhow::Result<()> {
    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/{ws}/workspaces/edit_guest_access"
        )),
        ADMIN_TOKEN,
    )
    .json(&json!({ "guest_access_enabled": on }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    Ok(())
}

async fn set_guest_jwt_pem(port: u16, ws: &str, pem: &str) -> anyhow::Result<()> {
    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/{ws}/workspaces/edit_guest_jwt_key"
        )),
        ADMIN_TOKEN,
    )
    .json(&json!({ "public_key": pem }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    Ok(())
}

fn app(path: &str, execution_mode: &str, sandbox: bool) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "App",
        "value": {},
        "policy": {
            "execution_mode": execution_mode,
            "sandbox": sandbox,
            "triggerables_v2": {
                "script/u/test-user/noop": { "static_inputs": {}, "one_of_inputs": {} }
            }
        }
    })
}

async fn create_app(port: u16, ws: &str, v: serde_json::Value) -> anyhow::Result<()> {
    let resp = authed(
        client().post(format!("http://localhost:{port}/api/w/{ws}/apps/create")),
        ADMIN_TOKEN,
    )
    .json(&v)
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    Ok(())
}

fn whoami(port: u16, ws: &str, token: &str) -> reqwest::RequestBuilder {
    authed(
        client().get(format!("http://localhost:{port}/api/w/{ws}/users/whoami")),
        token,
    )
}

/// A valid guest JWT opens its app, runs a component as the publisher, reads the run
/// back, reports `role: guest`, and leaves exactly one `guest_activity` row however
/// many requests it makes.
#[sqlx::test(fixtures("base"))]
async fn a_valid_guest_jwt_opens_its_app(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = "test-workspace";

    enable_guests(port, ws, true).await?;
    set_guest_jwt_pem(port, ws, PUB1).await?;
    let resp = authed(
        client().post(format!("http://localhost:{port}/api/w/{ws}/scripts/create")),
        ADMIN_TOKEN,
    )
    .json(&json!({
        "path": "u/test-user/noop",
        "summary": "",
        "description": "",
        "content": "echo 42",
        "language": "bash",
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    create_app(port, ws, app(APP_PATH, "guest", false)).await?;

    // A distinct email: the activity write is deduplicated by a process-global cache
    // keyed on email, workspace and day, and other tests in this binary share the
    // guest email, so the count below is only this test's if its email is its own.
    let mut claims = Claims::valid();
    claims.email = "activity-guest@example.com".to_string();
    let token = bearer(&claims, PRIV1, Algorithm::ES256);

    let resp = whoami(port, ws, &token).send().await?;
    assert_eq!(resp.status(), 200, "guest JWT must authenticate");
    let me: serde_json::Value = resp.json().await?;
    assert_eq!(me["role"], json!("guest"), "must read as a guest");
    assert_eq!(me["operator"], json!(true));
    assert_eq!(me["is_admin"], json!(false));

    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/{ws}/apps_u/execute_component/{APP_PATH}"
        )),
        &token,
    )
    .json(&json!({ "component": "a", "path": "script/u/test-user/noop", "args": {} }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let job_id = resp.text().await?;

    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/{ws}/jobs_u/getupdate/{job_id}"
        )),
        &token,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "the guest that started the run must read it back: {}",
        resp.text().await?
    );

    // Several requests, one row: the write is cached per email, workspace and day.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM guest_activity WHERE email = $1 AND workspace_id = $2 AND jwt_entry",
    )
    .bind(&claims.email)
    .bind(ws)
    .fetch_one(&db)
    .await?;
    assert_eq!(count, 1, "a JWT guest must leave exactly one activity row");

    Ok(())
}

/// The refusals that keep the door narrow. Each presents a bearer on the workspace's
/// own `whoami`, which the arm reaches only after every gate, so a 401 is the arm
/// saying no rather than a handler.
#[sqlx::test(fixtures("base"))]
async fn guest_jwt_refusals(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = "test-workspace";

    enable_guests(port, ws, true).await?;
    set_guest_jwt_pem(port, ws, PUB1).await?;
    create_app(port, ws, app(APP_PATH, "guest", false)).await?;
    create_app(port, ws, app("u/test-user/members_app", "publisher", false)).await?;

    // Positive control: a token valid against this exact fixture is admitted. Without it a
    // broken setup would 401 every bearer below and the whole suite would pass vacuously.
    let control = whoami(port, ws, &bearer(&Claims::valid(), PRIV1, Algorithm::ES256))
        .send()
        .await?;
    assert_eq!(control.status(), 200, "{}", control.text().await?);

    // wrong workspace: the claim must name the route's workspace.
    let mut c = Claims::valid();
    c.workspace_id = "other-ws".to_string();
    let wrong_ws = bearer(&c, PRIV1, Algorithm::ES256);

    // wrong key: signed with a key the workspace does not hold.
    let wrong_key = bearer(&Claims::valid(), PRIV2, Algorithm::ES256);

    // expired, past the verifier's clock-skew leeway.
    let mut c = Claims::valid();
    c.exp = now() - 120;
    let expired = bearer(&c, PRIV1, Algorithm::ES256);

    // a symmetric algorithm is never accepted.
    let hs256 = bearer(&Claims::valid(), PRIV1, Algorithm::HS256);

    // an email that already has an account is refused, not downgraded.
    let mut c = Claims::valid();
    c.email = "test@windmill.dev".to_string();
    let has_account = bearer(&c, PRIV1, Algorithm::ES256);

    // an app not in guest mode.
    let mut c = Claims::valid();
    c.app_path = "u/test-user/members_app".to_string();
    let not_guest_app = bearer(&c, PRIV1, Algorithm::ES256);

    // an existing account addressed in a different case still counts as an account:
    // the base fixture holds `test@windmill.dev`.
    let mut c = Claims::valid();
    c.email = "Test@Windmill.Dev".to_string();
    let mixed_case_account = bearer(&c, PRIV1, Algorithm::ES256);

    // a lifetime past the 24h cap, even with a valid signature.
    let mut c = Claims::valid();
    c.exp = now() + 25 * 3600;
    let over_lifetime_cap = bearer(&c, PRIV1, Algorithm::ES256);

    // an email with no `@` would become the guest's username and could be read as a
    // `u/<user>` or `g/<group>` principal; refused.
    let mut c = Claims::valid();
    c.email = "group-admins".to_string();
    let group_shaped_email = bearer(&c, PRIV1, Algorithm::ES256);

    // an email longer than the `guest_activity.email` column: refused before auth, so a
    // guest is never admitted without the activity row and audit event the count needs.
    let mut c = Claims::valid();
    c.email = format!("{}@example.com", "a".repeat(250));
    let oversized_email = bearer(&c, PRIV1, Algorithm::ES256);

    // an app_path carrying a scope metacharacter would widen the guest's scopes.
    let mut c = Claims::valid();
    c.app_path = "u/test-user/*".to_string();
    let wildcard_app_path = bearer(&c, PRIV1, Algorithm::ES256);

    // a valid, signed token past the length cap: without the cap it would deserialize into
    // GuestJwtClaims (the extra claim ignored) and verify, so this pins the length check.
    let mut payload = serde_json::to_value(Claims::valid()).unwrap();
    payload["padding"] = serde_json::json!("a".repeat(9000));
    let big_jwt = encode(
        &Header::new(Algorithm::ES256),
        &payload,
        &EncodingKey::from_ec_pem(PRIV1.as_bytes()).unwrap(),
    )
    .unwrap();
    let oversized_token = format!("jwt_guest_{big_jwt}");

    // a repeated prefix must not strip down to a valid short token that verifies and is then
    // cached under the full bearer key (trim_start_matches would; strip_prefix must not).
    let repeated_prefix = format!(
        "jwt_guest_{}",
        bearer(&Claims::valid(), PRIV1, Algorithm::ES256)
    );

    for (label, token) in [
        ("wrong workspace", wrong_ws),
        ("wrong key", wrong_key),
        ("expired", expired),
        ("HS256", hs256),
        ("email with an account", has_account),
        ("app not in guest mode", not_guest_app),
        ("mixed-case account", mixed_case_account),
        ("over the 24h lifetime cap", over_lifetime_cap),
        ("group-shaped email", group_shaped_email),
        ("oversized email", oversized_email),
        ("wildcard app_path", wildcard_app_path),
        ("oversized token", oversized_token),
        ("repeated prefix", repeated_prefix),
    ] {
        let resp = whoami(port, ws, &token).send().await?;
        assert_eq!(resp.status(), 401, "{label} must be refused");
    }

    Ok(())
}

/// The workspace switch gates a JWT guest exactly as it gates a signed-in one, at the
/// auth door, so turning guests off closes the JWT entry too.
#[sqlx::test(fixtures("base"))]
async fn guest_jwt_needs_the_workspace_switch(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = "test-workspace";

    set_guest_jwt_pem(port, ws, PUB1).await?;
    create_app(port, ws, app(APP_PATH, "guest", false)).await?;
    let token = bearer(&Claims::valid(), PRIV1, Algorithm::ES256);

    // Switch off (the default): refused.
    let resp = whoami(port, ws, &token).send().await?;
    assert_eq!(
        resp.status(),
        401,
        "a JWT guest must be refused while guests are off"
    );

    // Switch on: through.
    enable_guests(port, ws, true).await?;
    let resp = whoami(port, ws, &token).send().await?;
    assert_eq!(
        resp.status(),
        200,
        "with guests on, the JWT guest is admitted"
    );

    // Off again: closed on the next request.
    enable_guests(port, ws, false).await?;
    let resp = whoami(port, ws, &token).send().await?;
    assert_eq!(
        resp.status(),
        401,
        "turning guests off closes the JWT guest again"
    );

    Ok(())
}

/// A guest JWT is pinned to the workspace its claim names, so it authenticates on no
/// workspace-less route: the arm has no workspace to check the claim against.
#[sqlx::test(fixtures("base"))]
async fn guest_jwt_rejected_on_workspaceless_route(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = "test-workspace";

    enable_guests(port, ws, true).await?;
    set_guest_jwt_pem(port, ws, PUB1).await?;
    create_app(port, ws, app(APP_PATH, "guest", false)).await?;
    let token = bearer(&Claims::valid(), PRIV1, Algorithm::ES256);

    let resp = authed(
        client().get(format!("http://localhost:{port}/api/users/tokens/list")),
        &token,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "a guest JWT must not authenticate on a workspace-less route"
    );

    Ok(())
}

/// An embed token a JWT guest mints for a sandboxed app is capped at the JWT's own
/// expiry: a JWT has no token row, so the cap is carried through the auth cache. It
/// must not outlive the JWT, which is the guest's only revocation.
#[sqlx::test(fixtures("base"))]
async fn a_guest_jwt_derived_embed_token_is_capped(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = "test-workspace";

    enable_guests(port, ws, true).await?;
    set_guest_jwt_pem(port, ws, PUB1).await?;
    create_app(port, ws, app(APP_PATH, "guest", true)).await?;
    let secret: String = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/{ws}/apps/secret_of/{APP_PATH}"
        )),
        ADMIN_TOKEN,
    )
    .send()
    .await?
    .text()
    .await?;

    let claims = Claims::valid();
    let jwt_exp = claims.exp;
    let token = bearer(&claims, PRIV1, Algorithm::ES256);

    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/{ws}/apps_u/embed_token/{secret}"
        )),
        &token,
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let body: serde_json::Value = resp.json().await?;
    let child_exp: chrono::DateTime<chrono::Utc> = body["expiration"]
        .as_str()
        .and_then(|e| e.parse().ok())
        .expect("mint must return the token's expiration");
    assert!(
        child_exp.timestamp() as u64 <= jwt_exp,
        "the derived embed token ({child_exp}) must not outlive the JWT (exp {jwt_exp})"
    );

    // And it resolves as a guest.
    let embed = body["token"].as_str().expect("mint must return a token");
    let resp = whoami(port, ws, embed).send().await?;
    assert_eq!(resp.status(), 200);
    let me: serde_json::Value = resp.json().await?;
    assert_eq!(me["role"], json!("guest"));

    Ok(())
}

/// A workspace with no guest key of its own falls back to the instance issuer
/// (`JWT_EXT_JWKS_URL`), so an operator running one issuer configures it once. Verified as a
/// guest here in CE; a full login from that issuer stays EE (`jwt_ext_`).
#[sqlx::test(fixtures("base"))]
async fn no_workspace_key_falls_back_to_the_instance_issuer(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use windmill_common::guest_jwt::{key_source, GuestJwtKeySource};
    let url = "https://issuer.example.com/jwks.json";
    unsafe { std::env::set_var("JWT_EXT_JWKS_URL", url) };
    let src = key_source(&db, "test-workspace").await;
    unsafe { std::env::remove_var("JWT_EXT_JWKS_URL") };
    assert!(
        matches!(src?, Some(GuestJwtKeySource::JwksUrl(u)) if u == url),
        "no workspace key falls back to the instance issuer"
    );
    Ok(())
}
