//! Regression test for cross-site GET CSRF on the job-run endpoints.
//!
//! Seven GET handlers under `/api/w/:workspace/jobs` queue and run a script or flow.
//! The session cookie is `SameSite=Lax`, so a browser attaches it to a cross-site
//! top-level GET navigation, and nothing in the request path looked at `Origin`,
//! `Referer` or `Sec-Fetch-*`: an attacker page could make a logged-in browser execute
//! any deployed runnable with arguments of its choosing. CORS hides the response but
//! not the side effect.
//!
//! `CrossSiteGetGuard` refuses a cross-site GET that authenticates on the session cookie
//! alone. A request carrying its own credential is allowed, which is what keeps the webhook
//! URLs working: a cross-origin `EventSource` cannot set headers, so `?token=` is its only
//! way to reach `run_and_stream`, and a `Lax` cookie never rides an `EventSource` anyway.
//! The one case that does regress is deliberate: a signed-in user clicking a `?token=` link
//! from another site is refused, because `extract_token` gives the cookie precedence and so
//! exempting the parameter would let `?token=junk` reinstate the whole vector.
//!
//! This test pins down:
//!   - every one of the seven GETs rejects a cross-site cookie request (the core fix),
//!   - the cookie outranks a `token` query parameter in `extract_token`, so appending a
//!     junk one does not buy a pass,
//!   - the `Referer` fallback, which is what covers an instance served over plain http:
//!     no `Sec-Fetch-*` header is emitted there, so without it the guard would be inert,
//!   - no over-blocking: same-origin, `none`, a header-less client, a bearer token, a real
//!     `?token=`, a `Referer` matching either `Host` or `X-Forwarded-Host`, and POST on the
//!     same route all get through.
//!
//! The runnables are deliberately absent from the fixture. The guard is an extractor, so
//! it answers before the handler looks anything up, and every request that gets past it
//! fails as not-found instead — which isolates the guard without a worker or a deploy.

use reqwest::StatusCode;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const RUN_GETS: [&str; 7] = [
    "run_wait_result/p/u/test-user/absent",
    "run_wait_result/f/u/test-user/absent",
    "run_wait_result/fv/999",
    "run_and_stream/p/u/test-user/absent",
    "run_and_stream/f/u/test-user/absent",
    "run_and_stream/fv/999",
    "run_and_stream/h/000000000000000f",
];

#[sqlx::test(fixtures("base"))]
async fn test_cross_site_get_cannot_run_jobs(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let origin = format!("http://localhost:{}", server.addr.port());
    let base = format!("{origin}/api/w/test-workspace/jobs");
    let client = reqwest::Client::new();

    // ---- CORE REGRESSION: every job-queuing GET refuses a cross-site cookie request.
    for path in RUN_GETS {
        let resp = client
            .get(format!("{base}/{path}"))
            .header("Cookie", "token=SECRET_TOKEN")
            .header("Sec-Fetch-Site", "cross-site")
            .send()
            .await?;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "{path} must reject a cross-site GET authenticated by the session cookie"
        );
    }

    // A junk `token` query parameter is not an explicit credential: the cookie is what
    // authenticates the request, so the guard must still fire.
    let resp = client
        .get(format!("{base}/{}?token=junk", RUN_GETS[0]))
        .header("Cookie", "token=SECRET_TOKEN")
        .header("Sec-Fetch-Site", "cross-site")
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "a junk `token` query parameter must not bypass the guard"
    );

    // An instance served over plain http gets no `Sec-Fetch-*` header at all — Fetch
    // Metadata rides only on potentially trustworthy URLs — while the cookie still
    // arrives, so `Referer` is the only signal left there.
    let resp = client
        .get(format!("{base}/{}", RUN_GETS[0]))
        .header("Cookie", "token=SECRET_TOKEN")
        .header("Referer", "http://attacker.example/page")
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "a cross-host Referer must stand in for a missing Sec-Fetch-Site"
    );

    // ---- No over-blocking. Each of these authenticates, so a 404 for the absent
    //      runnable is the expected "got past the guard" answer.
    let allowed: [(&str, reqwest::RequestBuilder); 8] = [
        (
            "same-origin",
            client
                .get(format!("{base}/{}", RUN_GETS[0]))
                .header("Cookie", "token=SECRET_TOKEN")
                .header("Sec-Fetch-Site", "same-origin"),
        ),
        (
            "direct navigation (none)",
            client
                .get(format!("{base}/{}", RUN_GETS[0]))
                .header("Cookie", "token=SECRET_TOKEN")
                .header("Sec-Fetch-Site", "none"),
        ),
        (
            "non-browser client (no Sec-Fetch-Site)",
            client
                .get(format!("{base}/{}", RUN_GETS[0]))
                .header("Cookie", "token=SECRET_TOKEN"),
        ),
        (
            "cross-origin bearer token",
            client
                .get(format!("{base}/{}", RUN_GETS[0]))
                .header("Authorization", "Bearer SECRET_TOKEN")
                .header("Sec-Fetch-Site", "cross-site"),
        ),
        (
            "cross-origin webhook URL carrying ?token=",
            client
                .get(format!("{base}/{}?token=SECRET_TOKEN", RUN_GETS[0]))
                .header("Sec-Fetch-Site", "cross-site"),
        ),
        (
            "same-host Referer",
            client
                .get(format!("{base}/{}", RUN_GETS[0]))
                .header("Cookie", "token=SECRET_TOKEN")
                .header("Referer", format!("{origin}/apps")),
        ),
        (
            "Sec-Fetch-Site outranks a cross-host Referer",
            client
                .get(format!("{base}/{}", RUN_GETS[0]))
                .header("Cookie", "token=SECRET_TOKEN")
                .header("Sec-Fetch-Site", "same-origin")
                .header("Referer", "http://attacker.example/page"),
        ),
        (
            "Referer matching X-Forwarded-Host but not Host",
            client
                .get(format!("{base}/{}", RUN_GETS[0]))
                .header("Cookie", "token=SECRET_TOKEN")
                .header("X-Forwarded-Host", "windmill.example, proxy.internal")
                .header("Referer", "https://windmill.example/apps"),
        ),
    ];
    for (name, req) in allowed {
        let resp = req.send().await?;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "{name} must reach the handler"
        );
    }

    // POST is not the CSRF vector — a `SameSite=Lax` cookie is not sent on one — and the
    // stream routes serve both methods off the same handler.
    let resp = client
        .post(format!("{base}/{}", RUN_GETS[3]))
        .header("Cookie", "token=SECRET_TOKEN")
        .header("Sec-Fetch-Site", "cross-site")
        .json(&serde_json::json!({}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "a cross-site POST must reach the handler"
    );

    Ok(())
}
