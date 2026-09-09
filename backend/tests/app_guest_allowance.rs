//! The guest allowance: free up to `FREE_GUESTS_PER_WINDOW` distinct emails over the
//! trailing window. Past it, a hard-capped instance (Community, Pro) refuses a stranger
//! and lets a returning guest back in; a metered one (Enterprise) admits everyone and
//! counts seats. Its own binary: the plan is read from a process-wide key id that this
//! test flips, which no test sharing the process could tolerate.
//!
//! Users from the `base` fixture:
//!   test-user   (admin,     token SECRET_TOKEN)

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::workspaces::{FREE_GUESTS_PER_WINDOW, GUEST_WINDOW_DAYS};
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
/// `private` (the key id) and `enterprise` (the plan read) can meter; every other build
/// is capped whatever this says.
fn set_plan(pro: bool) {
    #[cfg(feature = "private")]
    windmill_common::ee::LICENSE_KEY_ID.store(std::sync::Arc::new(
        if pro { "test_pro" } else { "" }.to_string(),
    ));
    let _ = pro;
}

async fn mint(db: &Pool<Postgres>, email: &str) -> windmill_common::error::Result<String> {
    let mut tx = db.begin().await.unwrap();
    let minted = windmill_api_users::users::create_guest_session_token(
        email,
        "test-workspace",
        APP_PATH,
        &mut tx,
        tower_cookies::Cookies::default(),
    )
    .await;
    tx.commit().await.unwrap();
    minted
}

#[sqlx::test(fixtures("base"))]
async fn the_allowance_caps_strangers_and_meters_an_enterprise_plan(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
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
    // The whole allowance, used yesterday: still in the window, and a day the mint
    // does not write, so a row dated today can only be the mint's own.
    sqlx::query(
        "INSERT INTO guest_activity (email, workspace_id, day)
         SELECT 'g' || i || '@example.com', 'test-workspace', CURRENT_DATE - 1
         FROM generate_series(1, $1) AS i",
    )
    .bind(FREE_GUESTS_PER_WINDOW)
    .execute(&db)
    .await?;

    set_plan(true);
    let refused = mint(&db, "stranger@example.com").await.unwrap_err();
    assert!(
        matches!(&refused, windmill_common::error::Error::PermissionDenied(m)
            if m.contains(&format!("limit of {FREE_GUESTS_PER_WINDOW} guests over {GUEST_WINDOW_DAYS} days"))),
        "a stranger past the allowance is refused with the message the visitor reads: {refused:?}"
    );
    mint(&db, "g1@example.com")
        .await
        .expect("a guest already in the window is let back in");
    let recorded: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM guest_activity
                       WHERE email = 'g1@example.com' AND workspace_id = 'test-workspace'
                         AND day = CURRENT_DATE)",
    )
    .fetch_one(&db)
    .await?;
    assert!(
        recorded,
        "the mint writes today's guest_activity row, the allowance's unit"
    );

    let list: serde_json::Value = authed(
        client().get(format!(
            "http://localhost:{port}/api/users/guests?per_page=5"
        )),
        ADMIN_TOKEN,
    )
    .send()
    .await?
    .json()
    .await?;
    assert_eq!(list["usage"]["guest_count"], FREE_GUESTS_PER_WINDOW);
    assert_eq!(list["usage"]["metered"], false);
    assert_eq!(list["usage"]["guest_seats"], 0);
    assert_eq!(list["guests"].as_array().map(Vec::len), Some(5));
    assert_eq!(list["guests"][0]["workspaces"], json!(["test-workspace"]));
    let usage: serde_json::Value = authed(
        client().get(format!("{ws}/workspaces/guest_usage")),
        ADMIN_TOKEN,
    )
    .send()
    .await?
    .json()
    .await?;
    assert_eq!(usage["guest_count"], FREE_GUESTS_PER_WINDOW);

    #[cfg(all(feature = "private", feature = "enterprise"))]
    {
        set_plan(false);
        mint(&db, "stranger@example.com")
            .await
            .expect("a metered plan admits past the allowance");
        let usage: serde_json::Value = authed(
            client().get(format!("{ws}/workspaces/guest_usage")),
            ADMIN_TOKEN,
        )
        .send()
        .await?
        .json()
        .await?;
        assert_eq!(usage["guest_count"], FREE_GUESTS_PER_WINDOW + 1);
        assert_eq!(usage["metered"], true);
        assert_eq!(usage["billable_guests"], 1);
        assert_eq!(
            usage["guest_seats"], 1,
            "one guest past the allowance is a whole seat"
        );
    }

    Ok(())
}
