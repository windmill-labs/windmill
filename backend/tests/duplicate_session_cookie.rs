//! A browser can send two `token` cookies: this instance's, and one a parent-domain instance set
//! with `Domain=` (app.windmill.dev on `.windmill.dev`, seen from a tenant subdomain). Safari
//! sends the host's cookie first and Chrome last, so the request must authenticate on whichever
//! is a live session here, in either order.

use reqwest::StatusCode;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn test_duplicate_session_cookie_uses_the_live_one(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let client = reqwest::Client::new();
    let url = format!(
        "http://localhost:{}/api/workspaces/users",
        server.addr.port()
    );
    let status = |cookie: &'static str| {
        let req = client.get(&url).header("Cookie", cookie);
        async move { anyhow::Ok(req.send().await?.status()) }
    };

    assert_eq!(
        status("token=FOREIGN_INSTANCE_TOKEN; token=SECRET_TOKEN").await?,
        StatusCode::OK
    );
    assert_eq!(
        status("token=SECRET_TOKEN; token=FOREIGN_INSTANCE_TOKEN").await?,
        StatusCode::OK
    );
    assert_eq!(
        status("token=FOREIGN_INSTANCE_TOKEN; token=OTHER_FOREIGN_TOKEN").await?,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}
