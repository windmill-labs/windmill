/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    body::{self, BoxBody},
    extract::OriginalUri,
    http::{header, response::Builder, Response},
    response::IntoResponse,
    Extension,
};

use crate::{CloudHosted, ContentSecurityPolicy, IsSecure};
use mime_guess::mime;
use rust_embed::RustEmbed;
use std::sync::Arc;

// static_handler is a handler that serves static files from the
pub async fn static_handler(
    Extension(is_secure): Extension<Arc<IsSecure>>,
    Extension(is_cloud_hosted): Extension<Arc<CloudHosted>>,
    Extension(csp): Extension<Arc<ContentSecurityPolicy>>,
    OriginalUri(original_uri): OriginalUri,
) -> StaticFile {
    let path = original_uri.path().trim_start_matches('/').to_string();
    StaticFile(path, is_secure.0, is_cloud_hosted.0, csp)
}

#[derive(RustEmbed)]
#[folder = "../../frontend/build/"]
struct Asset;
pub struct StaticFile(
    pub String,
    pub bool,
    pub bool,
    pub Arc<ContentSecurityPolicy>,
);

impl IntoResponse for StaticFile {
    fn into_response(self) -> Response<BoxBody> {
        let path = self.0;
        let can_set_security_headers = self.1 && self.2;
        let csp = self.3;
        serve_path(path, can_set_security_headers, csp)
    }
}

fn serve_path(
    path: String,
    can_set_security_headers: bool,
    csp: Arc<ContentSecurityPolicy>,
) -> Response<BoxBody> {
    if path.starts_with("api/") {
        return Response::builder()
            .status(404)
            .body(body::boxed(body::Empty::new()))
            .unwrap();
    }
    match Asset::get(path.as_str()) {
        Some(content) => {
            let body = body::boxed(body::Full::from(content.data));
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut res = Response::builder().header(header::CONTENT_TYPE, mime.as_ref());
            if mime.as_ref() == mime::APPLICATION_JAVASCRIPT {
                res = res.header(header::CACHE_CONTROL, "max-age=31536000");
            } else if (mime.type_(), mime.subtype()) == (mime::TEXT, mime::CSS) {
                res = res.header(header::CACHE_CONTROL, "max-age=31536000");
            } else if (mime.type_()) == (mime::IMAGE) || (mime.type_()) == (mime::FONT) {
                res = res.header(header::CACHE_CONTROL, "max-age=31536000");
            } else {
                res = res.header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate");
            }

            if can_set_security_headers {
                res = set_security_headers(res, csp);
            }
            res.body(body).unwrap()
        }
        None if path.as_str().starts_with("_app/") => Response::builder()
            .status(404)
            .body(body::boxed(body::Empty::new()))
            .unwrap(),
        None => serve_path("200.html".to_owned(), can_set_security_headers, csp),
    }
}

fn set_security_headers(mut res: Builder, csp: Arc<ContentSecurityPolicy>) -> Builder {
    res = res.header("X-Frame-Options", "DENY");
    res = res.header("X-Content-Type-Options", "nosniff");

    if !csp.0.is_empty() {
        res = res.header("Content-Security-Policy", &csp.0);
    }

    res
}
