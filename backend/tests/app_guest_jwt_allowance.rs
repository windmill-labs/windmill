//! The guest allowance reached through a guest JWT (`jwt_guest_`). Its own binary
//! because `set_plan` flips a process-global license key, which a test sharing the
//! process could not tolerate (see `app_guest_allowance.rs`).
//!
//! Users from the `base` fixture:
//!   test-user   (admin,     token SECRET_TOKEN)

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::workspaces::FREE_GUESTS_PER_WINDOW;
use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";
const APP_PATH: &str = "u/test-user/guest_app";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

/// Community and Pro are capped, Enterprise is metered. Only a build with both
/// `private` and `enterprise` can meter; every other build is capped whatever this says.
fn set_plan(pro: bool) {
    #[cfg(feature = "private")]
    windmill_common::ee::LICENSE_KEY_ID.store(std::sync::Arc::new(
        if pro { "test_pro" } else { "" }.to_string(),
    ));
    let _ = pro;
}

const JWT_PUB: &str = "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEzAfqyCh34iYOCW0vg4ejq/zzJlzL\nSZScjnVyPjLGTapEwo4gc6/y1Yudd/v54wKh0OdfTfzAKMPWx/2NWx/ugg==\n-----END PUBLIC KEY-----\n";
const JWT_PRIV: &str = "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgu27S2DbSwUh8BmQb\n/i4/VhNdoXV7PJekhnoceMULYLihRANCAATMB+rIKHfiJg4JbS+Dh6Or/PMmXMtJ\nlJyOdXI+MsZNqkTCjiBzr/LVi513+/njAqHQ519N/MAow9bH/Y1bH+6C\n-----END PRIVATE KEY-----\n";

fn guest_jwt(email: &str) -> String {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;
    let claims = json!({
        "email": email,
        "workspace_id": "test-workspace",
        "app_path": APP_PATH,
        "exp": exp,
    });
    let jwt = encode(
        &Header::new(Algorithm::ES256),
        &claims,
        &EncodingKey::from_ec_pem(JWT_PRIV.as_bytes()).unwrap(),
    )
    .unwrap();
    format!("jwt_guest_{jwt}")
}

/// A JWT guest is subject to the same allowance as a signed-in one. Past the cap on a
/// capped instance, a stranger's JWT is refused (the auth arm returns 401; the visitor
/// message is only logged, since the arm cannot carry it), while a guest already in the
/// window is let back in.
#[sqlx::test(fixtures("base"))]
async fn a_guest_jwt_is_capped_like_a_signed_in_guest(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    authed(
        client().post(format!("{ws}/workspaces/edit_guest_access")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "guest_access_enabled": true }))
    .send()
    .await?;
    let resp = authed(
        client().post(format!("{ws}/workspaces/edit_guest_jwt_key")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "public_key": JWT_PUB }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": APP_PATH,
            "summary": "Guest app",
            "value": {},
            "policy": { "execution_mode": "guest", "triggerables_v2": {} }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    // The whole allowance, used today (g1..gN).
    sqlx::query(
        "INSERT INTO guest_activity (email, workspace_id, day)
         SELECT 'g' || i || '@example.com', 'test-workspace', CURRENT_DATE
         FROM generate_series(1, $1) AS i",
    )
    .bind(FREE_GUESTS_PER_WINDOW)
    .execute(&db)
    .await?;
    set_plan(true);

    let resp = authed(
        client().get(format!("{ws}/users/whoami")),
        &guest_jwt("stranger@example.com"),
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 401, "a stranger's JWT is refused past the cap");

    let resp = authed(
        client().get(format!("{ws}/users/whoami")),
        &guest_jwt("g1@example.com"),
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "a returning guest's JWT is admitted: {}",
        resp.text().await?
    );

    Ok(())
}
