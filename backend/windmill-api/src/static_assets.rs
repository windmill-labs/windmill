/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    body::{self, BoxBody},
    http::{header, Response, Uri, response::Builder},
    response::IntoResponse,
};

use mime_guess::mime;
use rust_embed::RustEmbed;

// static_handler is a handler that serves static files from the
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/').to_string();
    StaticFile(path)
}

#[derive(RustEmbed)]
#[folder = "../../frontend/build/"]
struct Asset;
pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response<BoxBody> {
        let path = self.0.into();
        serve_path(path)
    }
}

fn serve_path(path: String) -> Response<BoxBody> {
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
            res = set_security_headers(res);
            res.body(body).unwrap()
        }
        None if path.as_str().starts_with("_app/") => Response::builder()
            .status(404)
            .body(body::boxed(body::Empty::new()))
            .unwrap(),
        None => serve_path("200.html".to_owned()),
    }
}

fn set_security_headers(mut res: Builder) -> Builder {
    res = res.header("X-XSS-Protection", "1; mode=block");
    res = res.header("X-Frame-Options", "DENY");
    res = res.header("X-Content-Type-Options", "nosniff");
    res = set_content_security_policy(res);

    res
}

fn set_content_security_policy(mut res: Builder) -> Builder {
    let csp = "frame-ancestors 'none'; frame-src 'none'; worker-src 'self'; child-src 'none'; object-src 'none'";
    res = res.header("Content-Security-Policy", csp);

    res
}
