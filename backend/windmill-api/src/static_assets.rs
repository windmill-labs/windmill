/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    body::{self, BoxBody},
    http::{header, response::Builder, Response, Uri},
    response::IntoResponse,
    Extension,
};

use crate::{CloudHosted, IsSecure};
use mime_guess::mime;
use rust_embed::RustEmbed;
use std::sync::Arc;

// static_handler is a handler that serves static files from the
pub async fn static_handler(
    uri: Uri,
    Extension(is_secure): Extension<Arc<IsSecure>>,
    Extension(is_cloud_hosted): Extension<Arc<CloudHosted>>,
) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/').to_string();
    StaticFile(path, is_secure.0, is_cloud_hosted.0)
}

#[derive(RustEmbed)]
#[folder = "../../frontend/build/"]
struct Asset;
pub struct StaticFile<T>(pub T, pub bool, pub bool);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response<BoxBody> {
        let path = self.0.into();
        let can_set_security_headers = self.1 && self.2;
        serve_path(path, can_set_security_headers)
    }
}

fn serve_path(path: String, can_set_security_headers: bool) -> Response<BoxBody> {
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
            } else if (mime.type_()) == (mime::IMAGE) {
                res = res.header(header::CACHE_CONTROL, "max-age=31536000");
            } else {
                res = res.header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate");
            }

            if can_set_security_headers {
                res = set_security_headers(res);
            }
            res.body(body).unwrap()
        }
        None if path.as_str().starts_with("_app/") => Response::builder()
            .status(404)
            .body(body::boxed(body::Empty::new()))
            .unwrap(),
        None => serve_path("200.html".to_owned(), can_set_security_headers),
    }
}

fn set_security_headers(mut res: Builder) -> Builder {
    res = res.header("X-Frame-Options", "DENY");
    res = res.header("X-Content-Type-Options", "nosniff");

    if let Ok(csp) = std::env::var("SERVE_CSP") {
        res = res.header("Content-Security-Policy", csp);
    }

    res
}
