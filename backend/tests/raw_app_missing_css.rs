//! Raw app bundles whose stylesheet is absent rather than empty.
//!
//! `create_app_raw` stores the `css` blob only when the multipart field is
//! present, so an app built from sources with no styles has no `css` row at
//! all. Cross-workspace deploy re-fetches both bundle parts and treats a
//! non-200 as fatal, so 404ing the stylesheet makes such an app un-deployable:
//! absent means empty.
//!
//! This test pins down:
//!   - a bundle stored without a `css` part serves an empty stylesheet,
//!   - a bundle that has styles serves them byte-for-byte, so the tolerance
//!     never blanks a real stylesheet, and
//!   - a missing `js` blob still 404s — the tolerance must not leak to the
//!     half of the bundle whose absence really is broken.

use reqwest::multipart;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn app_part(path: &str) -> anyhow::Result<multipart::Part> {
    let app = serde_json::json!({
        "path": path,
        "summary": "",
        "value": {},
        "policy": { "execution_mode": "publisher" },
        "raw_app": true,
    });
    Ok(multipart::Part::text(app.to_string()).mime_str("application/json")?)
}

#[sqlx::test(fixtures("base"))]
async fn test_raw_app_without_stylesheet_stays_readable(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    let js_body = "console.log('app')";

    // 1. Create a raw app with NO css part at all — what a client that omits
    //    the field produces.
    let form = multipart::Form::new()
        .part("app", app_part("u/test-user/nocss")?)
        .part("js", multipart::Part::text(js_body));
    let resp = authed(
        client().post(format!("{base}/apps/create_raw")),
        "SECRET_TOKEN",
    )
    .multipart(form)
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "creating a raw app without a css part should succeed: {}",
        resp.text().await?
    );

    let secret = authed(
        client().get(format!(
            "{base}/apps/secret_of_latest_version/u/test-user/nocss"
        )),
        "SECRET_TOKEN",
    )
    .send()
    .await?
    .text()
    .await?;

    // 2. The stylesheet is absent, not broken: serve it empty so deploy — which
    //    re-fetches both parts — can carry the app to another workspace.
    let resp = authed(
        client().get(format!("{base}/apps/get_data/v/{secret}.css")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "a bundle with no stylesheet must serve an empty one, not 404"
    );
    assert_eq!(resp.text().await?, "", "the empty stylesheet must be empty");

    // 3. The JavaScript half is untouched by the fallback.
    let resp = authed(
        client().get(format!("{base}/apps/get_data/v/{secret}.js")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await?, js_body);

    // 4. An app that does have styles must still serve them unchanged — the
    //    fallback must never blank a real stylesheet.
    let css_body = "body { color: red }";
    let form = multipart::Form::new()
        .part("app", app_part("u/test-user/withcss")?)
        .part("js", multipart::Part::text(js_body))
        .part("css", multipart::Part::text(css_body));
    let resp = authed(
        client().post(format!("{base}/apps/create_raw")),
        "SECRET_TOKEN",
    )
    .multipart(form)
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);

    let styled_secret = authed(
        client().get(format!(
            "{base}/apps/secret_of_latest_version/u/test-user/withcss"
        )),
        "SECRET_TOKEN",
    )
    .send()
    .await?
    .text()
    .await?;

    let resp = authed(
        client().get(format!("{base}/apps/get_data/v/{styled_secret}.css")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.text().await?,
        css_body,
        "an app with styles must round-trip them intact"
    );

    // 5. A missing `js` blob is a genuinely broken bundle and must still 404.
    //    The API cannot create this state (js is mandatory on write), so drop
    //    the row directly.
    let version_id: i64 = sqlx::query_scalar(
        "SELECT versions[array_upper(versions, 1)] FROM app
         WHERE path = 'u/test-user/nocss' AND workspace_id = 'test-workspace'",
    )
    .fetch_one(&db)
    .await?;
    sqlx::query("DELETE FROM app_bundles WHERE app_version_id = $1 AND file_type = 'js'")
        .bind(version_id)
        .execute(&db)
        .await?;

    let resp = authed(
        client().get(format!("{base}/apps/get_data/v/{secret}.js")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        404,
        "a missing js bundle must still fail loudly — the css tolerance must not leak to it"
    );

    Ok(())
}
