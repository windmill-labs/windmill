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
    http::{header, Response},
    response::IntoResponse,
};

use hyper::Uri;
use mime_guess::mime;
use rust_embed::RustEmbed;

// static_handler is a handler that serves static files from the
pub async fn static_handler(OriginalUri(original_uri): OriginalUri) -> StaticFile {
    StaticFile(original_uri)
}

#[derive(RustEmbed)]
#[folder = "../../frontend/build/"]
struct Asset;
pub struct StaticFile(Uri);

impl IntoResponse for StaticFile {
    fn into_response(self) -> Response<BoxBody> {
        let path = self.0.path().trim_start_matches('/');
        serve_path(path)
    }
}

const TWO_HUNDRED: &str = "200.html";

fn serve_path(path: &str) -> Response<BoxBody> {
    if path.starts_with("api/") {
        return Response::builder()
            .status(404)
            .body(body::boxed(body::Empty::new()))
            .unwrap();
    }
    match Asset::get(path) {
        Some(content) => {
            let body = body::boxed(body::Full::from(content.data));
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut res = Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
            if mime.as_ref() == mime::APPLICATION_JAVASCRIPT {
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
        None if path.starts_with("_app/") => Response::builder()
            .status(404)
            .body(body::boxed(body::Empty::new()))
            .unwrap(),
        None => serve_path(TWO_HUNDRED),
    }
}
