/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{body::Body, extract::OriginalUri, http::Response, response::IntoResponse};

#[cfg(feature = "static_frontend")]
use axum::http::header;
#[cfg(feature = "static_frontend")]
use http::HeaderValue;

#[cfg(feature = "static_frontend")]
use bytes::Bytes;
#[cfg(feature = "static_frontend")]
use dashmap::DashMap;
use hyper::Uri;
#[cfg(feature = "static_frontend")]
use mime_guess::mime;
#[cfg(feature = "static_frontend")]
use rust_embed::RustEmbed;

// Content Security Policy configuration
#[cfg(feature = "static_frontend")]
lazy_static::lazy_static! {
    static ref CSP_POLICY: String = std::env::var("CSP_POLICY").unwrap_or_default();
}

/// A frontend built with `VITE_BASE_URL=/__WM_BASE_PATH__` takes its base path from the
/// `BASE_PATH` env var when served, so one build serves any sub-path.
#[cfg(feature = "static_frontend")]
const BASE_PATH_PLACEHOLDER: &str = "/__WM_BASE_PATH__";

#[cfg(feature = "static_frontend")]
lazy_static::lazy_static! {
    static ref BASE_PATH: String = {
        let path = std::env::var("BASE_PATH").unwrap_or_default();
        match path.trim().trim_matches('/') {
            "" => String::new(),
            path => format!("/{path}"),
        }
    };

    // Decided once from the entry page, so a build without the placeholder pays nothing per request.
    static ref REBASE_ASSETS: bool = {
        let placeholder_build = Asset::get(TWO_HUNDRED).is_some_and(|page| {
            std::str::from_utf8(&page.data).is_ok_and(|html| html.contains(BASE_PATH_PLACEHOLDER))
        });
        if !placeholder_build && !BASE_PATH.is_empty() {
            tracing::warn!(
                "BASE_PATH is ignored: the frontend was not built with VITE_BASE_URL={BASE_PATH_PLACEHOLDER}"
            );
        }
        placeholder_build
    };

    // Embedded assets and `BASE_PATH` are fixed for the life of the process, so each asset is
    // scanned once: `None` records an asset without the placeholder.
    static ref REBASED: DashMap<String, Option<Bytes>> = DashMap::new();
}

// static_handler is a handler that serves static files from the
pub async fn static_handler(OriginalUri(original_uri): OriginalUri) -> StaticFile {
    StaticFile(original_uri)
}

#[cfg(feature = "static_frontend")]
#[derive(RustEmbed)]
#[folder = "${FRONTEND_BUILD_DIR:-../../frontend/build/}"]
struct Asset;
pub struct StaticFile(Uri);

impl IntoResponse for StaticFile {
    fn into_response(self) -> Response<Body> {
        let original_path = self.0.path();
        let query = self.0.query();
        let path = original_path.trim_start_matches('/');
        serve_path(path, original_path, query)
    }
}

#[cfg(feature = "static_frontend")]
const TWO_HUNDRED: &str = "200.html";

/// Check if the original path requires cross-origin isolation headers.
///
/// CANONICAL COEP RATIONALE (the dev-server mirror in `frontend/vite.config.js`
/// points here). Only public apps (`/public/` and custom paths `/a/`) ever get
/// them, and only when they opt in via the `wm_coep` query param: a public (raw)
/// app must set COEP to be embeddable as an iframe inside a cross-origin-isolated
/// page (which requires the embedded document to also set COEP). It is opt-in
/// rather than always-on because COEP `require-corp` blocks subresources without
/// CORP (external image URLs, embeds), and since headers stick to the document,
/// isolating an SPA route leaks into every page reached from it client-side.
#[cfg(feature = "static_frontend")]
fn needs_cross_origin_isolation(original_path: &str, query: Option<&str>) -> bool {
    (original_path.starts_with("/public/") || original_path.starts_with("/a/"))
        && query_has_flag(query, "wm_coep")
}

/// Returns true if `query` contains the given flag key (with or without a
/// value), e.g. `?wm_coep`, `?wm_coep=on`, `?foo=1&wm_coep=1`.
#[cfg(feature = "static_frontend")]
fn query_has_flag(query: Option<&str>, flag: &str) -> bool {
    query.is_some_and(|q| q.split('&').any(|kv| kv.split('=').next() == Some(flag)))
}

/// `data` with the placeholder replaced, or `None` when it is not a text asset carrying it.
#[cfg(feature = "static_frontend")]
fn rebase(data: &[u8], mime: &mime::Mime, base_path: &str) -> Option<Bytes> {
    let is_text = mime.type_() == mime::TEXT
        || mime.subtype() == mime::JAVASCRIPT
        || mime.subtype() == mime::JSON;
    if !is_text {
        return None;
    }
    let text = std::str::from_utf8(data).ok()?;
    text.contains(BASE_PATH_PLACEHOLDER)
        .then(|| Bytes::from(text.replace(BASE_PATH_PLACEHOLDER, base_path)))
}

#[cfg(feature = "static_frontend")]
fn rebased(path: &str, data: &[u8], mime: &mime::Mime) -> Option<Bytes> {
    if !*REBASE_ASSETS {
        return None;
    }
    if let Some(hit) = REBASED.get(path) {
        return hit.value().clone();
    }
    let rebased = rebase(data, mime, &BASE_PATH);
    REBASED.insert(path.to_owned(), rebased.clone());
    rebased
}

fn serve_path(path: &str, original_path: &str, query: Option<&str>) -> Response<Body> {
    if path.starts_with("api/") {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    #[cfg(feature = "static_frontend")]
    match Asset::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let body = match rebased(path, &content.data, &mime) {
                Some(data) => Body::from(data),
                None => Body::from(content.data),
            };
            let mut res = Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");

            if needs_cross_origin_isolation(original_path, query) {
                res = res
                    .header("Cross-Origin-Opener-Policy", "same-origin")
                    .header("Cross-Origin-Embedder-Policy", "require-corp")
                    .header("Cross-Origin-Resource-Policy", "cross-origin");
            }

            // Login and its siblings carry a different `rd` on every page that links
            // to them, so a crawler meets thousands of URLs for one form. The app is
            // client-rendered, so a meta tag only exists after a render pass; the
            // header is seen on the first fetch.
            if original_path.starts_with("/user/") {
                res = res.header("X-Robots-Tag", "noindex, nofollow");
            }

            // The raw-app preview shell evaluates whatever js is posted to it, so it is
            // only ever served opaque-origin, with the deployed app wrapper's exact flags
            // (`get_raw_app_data`). Matched on the embedded file, not the request path,
            // so `//ui_builder/...` is covered. A sandboxed app's editor preview loads it
            // as served, framed with the same flags (`RAW_APP_SANDBOX_FLAGS` in the
            // frontend); an unsandboxed one loads a same-origin blob: copy.
            if path == "ui_builder/app-preview.html" {
                res = res.header(
                    header::CONTENT_SECURITY_POLICY,
                    "sandbox allow-scripts allow-forms allow-popups \
                     allow-popups-to-escape-sandbox allow-downloads allow-modals \
                     allow-top-navigation",
                );
            }

            // Add Content-Security-Policy header for static assets when policy is set
            if !CSP_POLICY.is_empty() {
                if let Ok(header_value) = HeaderValue::try_from(CSP_POLICY.as_str()) {
                    res = res.header("Content-Security-Policy", header_value);
                }
            }
            if mime.as_ref() == mime::APPLICATION_JAVASCRIPT
                || mime.as_ref() == mime::TEXT_JAVASCRIPT
                || path.ends_with(".wasm")
            {
                res = res.header(header::CACHE_CONTROL, "max-age=31536000");
            } else if (mime.type_(), mime.subtype()) == (mime::TEXT, mime::CSS) {
                res = res.header(header::CACHE_CONTROL, "max-age=31536000");
            } else if (mime.type_()) == (mime::IMAGE) || (mime.type_()) == (mime::FONT) {
                res = res.header(header::CACHE_CONTROL, "max-age=31536000");
            } else {
                res = res.header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate");
            }

            res.body(body).unwrap()
        }
        None if path.starts_with("_app/") => {
            Response::builder().status(404).body(Body::empty()).unwrap()
        }
        None => serve_path(TWO_HUNDRED, original_path, query),
    }

    #[cfg(not(feature = "static_frontend"))]
    {
        let _ = (original_path, query); // suppress unused warning
        Response::builder().status(404).body(Body::empty()).unwrap()
    }
}

#[cfg(all(test, feature = "static_frontend"))]
mod tests {
    use super::*;

    #[test]
    fn test_rebase() {
        let html = br#"src="/__WM_BASE_PATH__/_app/a.js" base="/__WM_BASE_PATH__""#;
        let rebased = |base: &str, mime: &mime::Mime| rebase(html, mime, base);

        assert_eq!(
            rebased("/windmill", &mime::TEXT_HTML).as_deref(),
            Some(br#"src="/windmill/_app/a.js" base="/windmill""#.as_slice())
        );
        assert_eq!(
            rebased("", &mime::TEXT_HTML).as_deref(),
            Some(br#"src="/_app/a.js" base="""#.as_slice())
        );
        // binary assets and text without the placeholder are left to be served as embedded
        assert_eq!(rebased("/windmill", &mime::APPLICATION_OCTET_STREAM), None);
        assert_eq!(
            rebase(b"no placeholder", &mime::TEXT_HTML, "/windmill"),
            None
        );
    }

    #[test]
    fn test_query_has_flag() {
        assert!(query_has_flag(Some("wm_coep"), "wm_coep"));
        assert!(query_has_flag(Some("wm_coep=on"), "wm_coep"));
        assert!(query_has_flag(Some("foo=1&wm_coep=1"), "wm_coep"));
        assert!(query_has_flag(Some("wm_coep&foo=1"), "wm_coep"));
        assert!(!query_has_flag(Some("wm_coepx=1"), "wm_coep"));
        assert!(!query_has_flag(Some("foo=wm_coep"), "wm_coep"));
        assert!(!query_has_flag(Some(""), "wm_coep"));
        assert!(!query_has_flag(None, "wm_coep"));
    }

    #[test]
    fn test_needs_cross_origin_isolation() {
        // the raw app editor, its UI builder frames and the viewer are never isolated
        assert!(!needs_cross_origin_isolation("/apps_raw/edit/foo", None));
        assert!(!needs_cross_origin_isolation("/apps_raw/add", None));
        assert!(!needs_cross_origin_isolation(
            "/ui_builder/index.html",
            None
        ));
        assert!(!needs_cross_origin_isolation(
            "/apps_raw/get/u/foo/bar",
            None
        ));

        // public apps (and custom paths) are isolated only when they opt in via wm_coep
        assert!(needs_cross_origin_isolation(
            "/public/ws/secret",
            Some("wm_coep")
        ));
        assert!(needs_cross_origin_isolation(
            "/public/ws/secret",
            Some("wm_coep=on")
        ));
        assert!(needs_cross_origin_isolation(
            "/a/ws/my/path",
            Some("wm_coep=on")
        ));
        assert!(!needs_cross_origin_isolation("/public/ws/secret", None));
        assert!(!needs_cross_origin_isolation("/a/ws/my/path", None));
        assert!(!needs_cross_origin_isolation(
            "/public/ws/secret",
            Some("foo=1")
        ));

        // unrelated paths never get the headers
        assert!(!needs_cross_origin_isolation(
            "/apps/get/foo",
            Some("wm_coep")
        ));
        // `/api/` must not be caught by the `/a/` prefix
        assert!(!needs_cross_origin_isolation(
            "/api/version",
            Some("wm_coep")
        ));
        assert!(!needs_cross_origin_isolation("/", None));
    }
}
