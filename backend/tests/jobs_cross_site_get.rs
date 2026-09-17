//! Regression test for cross-site GET CSRF on the job-run endpoints that can run a Hub script.
//!
//! `run_wait_result/p/{path}` and `run_and_stream/p/{path}` answer GET and, for a `hub/` path,
//! run any public Hub script. The session cookie is `SameSite=Lax`, so a browser attaches it
//! to a cross-site top-level GET navigation, and an argument written `$var:<path>` or
//! `$res:<path>` is resolved as the caller: an attacker page could make a logged-in browser
//! run a generic Hub script and hand it the victim's secrets. CORS hides the response but not
//! the side effect.
//!
//! `CrossSiteGetGuard` refuses such a request. Workspace scripts are deliberately not refused,
//! since they only run code the workspace's own members deployed. A request carrying its own
//! credential is allowed; the one case that regresses is a signed-in user clicking a `?token=`
//! Hub-script link from another site, because `extract_token` gives the cookie precedence and
//! exempting the parameter would let `?token=junk` reinstate the vector.
//!
//! This test pins down:
//!   - both endpoints refuse a cross-site cookie GET to a Hub script (the core fix), whether
//!     `Sec-Fetch-Site` says so or, with no such header (plain http), a cross-host `Referer`,
//!   - a junk `token` query parameter does not buy a pass,
//!   - the scope: the same request to a workspace script is not refused,
//!   - a Hub-script request with its own credential (bearer, or `?token=` and no cookie), or
//!     sent as a POST, gets through.
//!
//! No runnable exists and no Hub is contacted. A request that gets past the guard fails as
//! not-found on a workspace path, and on the non-numeric version in `hub/x/...` for a Hub
//! path, which is rejected while resolving the runnable, before any call to the Hub.

use reqwest::StatusCode;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const HUB_GETS: [&str; 2] = [
    "run_wait_result/p/hub/x/absent",
    "run_and_stream/p/hub/x/absent",
];
const WORKSPACE_GETS: [&str; 2] = [
    "run_wait_result/p/u/test-user/absent",
    "run_and_stream/p/u/test-user/absent",
];

async fn send(req: reqwest::RequestBuilder) -> anyhow::Result<(StatusCode, String)> {
    let resp = req.send().await?;
    let status = resp.status();
    Ok((status, resp.text().await?))
}

#[sqlx::test(fixtures("base"))]
async fn test_cross_site_get_cannot_run_hub_scripts(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace/jobs",
        server.addr.port()
    );
    let client = reqwest::Client::new();
    let cookie_get = |path: &str| {
        client
            .get(format!("{base}/{path}"))
            .header("Cookie", "token=SECRET_TOKEN")
    };

    // ---- CORE REGRESSION: a cross-site cookie GET cannot run a Hub script.
    for path in HUB_GETS {
        let refused = [
            (
                "Sec-Fetch-Site: cross-site",
                cookie_get(path).header("Sec-Fetch-Site", "cross-site"),
            ),
            // Plain http gets no `Sec-Fetch-*` at all, so `Referer` is the only signal left.
            (
                "cross-host Referer with no Sec-Fetch-Site",
                cookie_get(path).header("Referer", "http://attacker.example/page"),
            ),
            // The cookie outranks a `token` query parameter when authenticating.
            (
                "junk ?token= next to the cookie",
                client
                    .get(format!("{base}/{path}?token=junk"))
                    .header("Cookie", "token=SECRET_TOKEN")
                    .header("Sec-Fetch-Site", "cross-site"),
            ),
        ];
        for (name, req) in refused {
            let (status, body) = send(req).await?;
            assert_eq!(
                status,
                StatusCode::FORBIDDEN,
                "{path} [{name}] must be refused: {body}"
            );
        }
    }

    // ---- Scope: the same request to a workspace script is not refused.
    for path in WORKSPACE_GETS {
        let (status, body) = send(cookie_get(path).header("Sec-Fetch-Site", "cross-site")).await?;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "{path} is a workspace script and must reach the handler: {body}"
        );
    }

    // ---- A Hub-script request that carries its own credential, or is a POST, gets through.
    let hub = HUB_GETS[1];
    let allowed = [
        (
            "cross-origin bearer token",
            client
                .get(format!("{base}/{hub}"))
                .header("Authorization", "Bearer SECRET_TOKEN")
                .header("Sec-Fetch-Site", "cross-site"),
        ),
        (
            "cross-origin ?token= with no cookie",
            client
                .get(format!("{base}/{hub}?token=SECRET_TOKEN"))
                .header("Sec-Fetch-Site", "cross-site"),
        ),
        (
            "POST with the cookie",
            client
                .post(format!("{base}/{hub}"))
                .header("Cookie", "token=SECRET_TOKEN")
                .header("Sec-Fetch-Site", "cross-site")
                .json(&serde_json::json!({})),
        ),
    ];
    for (name, req) in allowed {
        let (status, body) = send(req).await?;
        assert!(
            body.contains("Invalid hub script version"),
            "{name} must get past the guard to runnable resolution (got {status}): {body}"
        );
    }

    Ok(())
}
