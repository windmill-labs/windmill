//! A readable companion to the HttpOnly session cookie, so the frontend can tell whether the
//! session it rendered for last is still the one the browser holds before `whoami` answers.
//! It is derived from the token and cannot be turned back into it, so it grants nothing.
//!
//! Mirrored from the `Set-Cookie` headers rather than set beside each `token` cookie: every
//! site that sets or clears the session, EE ones included, must move the hint with it, or the
//! frontend would render one user's cached data under another's session.

use axum::{extract::Request, middleware::Next, response::Response};
use http::{header::SET_COOKIE, HeaderValue};
use sha2::{Digest, Sha256};
use tower_cookies::Cookie;
use windmill_common::users::COOKIE_NAME;

pub(crate) const SESSION_HINT_COOKIE: &str = "wm_session";

fn session_hint(token: &str) -> String {
    let digest = Sha256::new()
        .chain_update(b"windmill-session-hint\0")
        .chain_update(token.as_bytes())
        .finalize();
    hex::encode(&digest[..12])
}

/// The hint cookie for a `token` cookie being set (or cleared, when its value is empty), with
/// the same scope and lifetime.
fn hint_for(session: &Cookie) -> Cookie<'static> {
    let value = if session.value().is_empty() {
        String::new()
    } else {
        session_hint(session.value())
    };
    let mut hint = Cookie::new(SESSION_HINT_COOKIE, value);
    if let Some(path) = session.path() {
        hint.set_path(path.to_string());
    }
    if let Some(domain) = session.domain() {
        hint.set_domain(domain.to_string());
    }
    if let Some(expires) = session.expires() {
        hint.set_expires(expires);
    }
    if let Some(max_age) = session.max_age() {
        hint.set_max_age(max_age);
    }
    if let Some(secure) = session.secure() {
        hint.set_secure(secure);
    }
    if let Some(same_site) = session.same_site() {
        hint.set_same_site(same_site);
    }
    hint.set_http_only(false);
    hint
}

/// Must wrap `CookieManagerLayer`, which writes the `Set-Cookie` headers this reads.
pub(crate) async fn mirror_session_cookie(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let hints: Vec<HeaderValue> = res
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|s| Cookie::parse(s.to_owned()).ok())
        .filter(|c| c.name() == COOKIE_NAME)
        .filter_map(|c| HeaderValue::from_str(&hint_for(&c).to_string()).ok())
        .collect();
    for hint in hints {
        res.headers_mut().append(SET_COOKIE, hint);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hint_follows_the_session_cookie_and_hides_the_token() {
        let set = Cookie::parse(
            "token=secret_token_value; Path=/; Domain=example.com; Secure; HttpOnly; \
             SameSite=Lax; Expires=Wed, 21 Oct 2037 07:28:00 GMT",
        )
        .unwrap();
        let hint = hint_for(&set);
        assert_eq!(hint.name(), SESSION_HINT_COOKIE);
        assert!(!hint.value().contains("secret_token_value"));
        assert_eq!(hint.value(), session_hint("secret_token_value"));
        assert_ne!(hint.value(), session_hint("another_token"));
        assert_eq!(hint.http_only(), Some(false));
        assert_eq!(hint.path(), Some("/"));
        assert_eq!(hint.domain(), Some("example.com"));
        assert_eq!(hint.expires(), set.expires());
        assert_eq!(hint.secure(), Some(true));

        // A logout clears the session: the hint goes with it.
        let cleared = Cookie::parse("token=; Path=/; Max-Age=0").unwrap();
        let hint = hint_for(&cleared);
        assert_eq!(hint.value(), "");
        assert_eq!(hint.max_age(), cleared.max_age());
    }
}
