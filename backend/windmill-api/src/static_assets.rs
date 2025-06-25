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
        let path = self.0.path().trim_start_matches('/');
        serve_path(path)
    }
}

#[cfg(feature = "static_frontend")]
const TWO_HUNDRED: &str = "200.html";

fn serve_path(path: &str) -> Response<Body> {
    if path.starts_with("api/") {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    #[cfg(feature = "static_frontend")]
    match Asset::get(path) {
        Some(content) => {
            let body = Body::from(content.data);
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut res = Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
            
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
        None => serve_path(TWO_HUNDRED),
    }

    #[cfg(not(feature = "static_frontend"))]
    {
        Response::builder().status(404).body(Body::empty()).unwrap()
    }
}
