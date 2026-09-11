/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::extract::FromRequestParts;
use axum::http::{header, request::Parts, Method};
use axum::Extension;
use url::Url;
use windmill_common::error::Error;
use windmill_common::users::COOKIE_NAME;

use crate::triggers::trigger_helpers::RunnableId;

/// Whether a request is a cross-site GET authenticating on the session cookie alone. A GET
/// handler that runs a script by path resolves it through [`Self::script_runnable`], which
/// refuses a Hub script on such a request.
///
/// The cookie is `SameSite=Lax`, so browsers attach it to cross-site top-level GET
/// navigations. A `hub/` path runs any public Hub script, and an argument written
/// `$var:<path>` or `$res:<path>` is resolved as the caller before the script sees it: such a
/// GET lets any page pick a generic Hub script and hand it the victim's secrets, which the job
/// can then send anywhere.
///
/// Workspace scripts and flows are not refused, by choice, so that GET links to them keep
/// working. That is a scope decision, not a safety property: they still take attacker-chosen
/// arguments, `$var:` and `$res:` included, resolved as the victim. What bounds the exposure
/// is that the attacker needs a runnable path and can only run code the workspace deployed.
///
/// The cookie is the only ambient credential. A bearer header is explicit, and so is the
/// `token` query parameter the webhook URLs carry — a cross-origin `EventSource` has no other
/// way to authenticate, since it cannot set headers. The checks run in `extract_token`'s
/// order, header before cookie, because that is the order it resolves them in: a request
/// carrying both a cookie and `token=` authenticates on the cookie and is therefore still
/// ambient, which is also why a valid `token=` link opened cross-site while signed in is
/// refused.
pub struct CrossSiteGetGuard(Option<CrossSite>);

impl CrossSiteGetGuard {
    pub fn script_runnable(&self, script_path: &str) -> windmill_common::error::Result<RunnableId> {
        let runnable_id = RunnableId::from_script_path(script_path);
        let (Some(signal), RunnableId::HubScript(_)) = (&self.0, &runnable_id) else {
            return Ok(runnable_id);
        };
        // The `Referer` leg is the one that can misfire, on a request that really was
        // same-host: it compares against the hosts the backend can see, and a proxy that
        // rewrites `Host` without setting `X-Forwarded-Host` leaves none of them matching
        // what the browser addressed. Name the comparison so that shows up as a
        // misconfiguration rather than as an unexplained 403.
        if let CrossSite::RefererMismatch { referer, instance_hosts } = signal {
            tracing::warn!(
                referer_host = %referer,
                ?instance_hosts,
                "refusing a cross-site GET Hub script run inferred from Referer; if the request \
                 was same-host, set `X-Forwarded-Host` on the proxy or configure `BASE_URL`"
            );
        }
        Err(Error::PermissionDenied(
            "a cross-site GET request cannot run a Hub script with the session cookie, which takes \
             precedence over a `token` query parameter: pass the token in the `Authorization` \
             header, or open the link from the instance itself or from a browser with no Windmill \
             session"
                .to_string(),
        ))
    }
}

impl<S: Send + Sync> FromRequestParts<S> for CrossSiteGetGuard {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        if parts.method != Method::GET {
            return Ok(CrossSiteGetGuard(None));
        }
        let Some(signal) = cross_site_signal(parts) else {
            return Ok(CrossSiteGetGuard(None));
        };

        let has_bearer = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("Bearer "));
        if has_bearer {
            return Ok(CrossSiteGetGuard(None));
        }

        let has_session_cookie =
            Extension::<tower_cookies::Cookies>::from_request_parts(parts, state)
                .await
                .is_ok_and(|Extension(cookies)| cookies.get(COOKIE_NAME).is_some());
        Ok(CrossSiteGetGuard(has_session_cookie.then_some(signal)))
    }
}

enum CrossSite {
    Declared,
    RefererMismatch { referer: String, instance_hosts: Vec<String> },
}

fn cross_site_signal(parts: &Parts) -> Option<CrossSite> {
    if let Some(site) = parts.headers.get("sec-fetch-site") {
        return site
            .as_bytes()
            .eq_ignore_ascii_case(b"cross-site")
            .then_some(CrossSite::Declared);
    }
    // Fetch Metadata rides only on potentially trustworthy URLs, so an instance served
    // over plain http never receives `Sec-Fetch-Site` (nor does Safari before 16.4) while
    // the cookie, not being `Secure` there either, still arrives. `Referer` is the only
    // other thing a top-level GET navigation carries — `Origin` is not sent on one — so it
    // is all that is left there, and it is weak: the default `strict-origin-when-cross-
    // origin` policy already drops `Referer` on an https-to-http downgrade, so an https
    // attacker page pointing a victim at a plain-http instance sends neither header. This
    // leg catches an http-served attacker page and pre-16.4 Safari on https; the guard is
    // load-bearing on https and best-effort at best on plain http. An absent `Referer`
    // reads as not cross-site, matching how `Sec-Fetch-Site: none` (a bookmark, a typed
    // URL) is treated.
    let referer = referer_host(parts)?;
    let instance_hosts: Vec<String> = instance_hosts(parts).collect();
    (!instance_hosts
        .iter()
        .any(|host| host.eq_ignore_ascii_case(&referer)))
    .then_some(CrossSite::RefererMismatch { referer, instance_hosts })
}

/// Every host a legitimate same-host request can name. `Host` alone is not enough: a
/// reverse proxy that forwards without preserving it (nginx `proxy_pass` with no
/// `proxy_set_header Host $host`) hands the backend the upstream's name, which no browser
/// `Referer` will ever match. None of these is browser-settable on a navigation — a
/// navigation carries no custom headers, and `BASE_URL` is instance config — so widening
/// the accepted set costs nothing.
fn instance_hosts(parts: &Parts) -> impl Iterator<Item = String> {
    let base_url = windmill_common::BASE_URL.load();
    [
        request_host(parts),
        header_host(parts, "x-forwarded-host"),
        Url::parse(base_url.as_str())
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned)),
    ]
    .into_iter()
    .flatten()
}

fn referer_host(parts: &Parts) -> Option<String> {
    let referer = parts.headers.get(header::REFERER)?.to_str().ok()?;
    Url::parse(referer).ok()?.host_str().map(str::to_owned)
}

fn request_host(parts: &Parts) -> Option<String> {
    if let Some(host) = parts.uri.host() {
        return Some(host.to_owned());
    }
    header_host(parts, header::HOST)
}

fn header_host(parts: &Parts, name: impl header::AsHeaderName) -> Option<String> {
    host_of(parts.headers.get(name)?.to_str().ok()?)
}

/// The host in a `Host`-shaped header value: `host[:port]`, where `host` may be a bracketed
/// IPv6 literal, and where a chain of proxies appends to `X-Forwarded-Host` so only the
/// first entry is the one the browser addressed. The port is split off by the URL parser
/// rather than by hand-rolling the bracket rules.
fn host_of(value: &str) -> Option<String> {
    let host = value.split(',').next()?.trim();
    Url::parse(&format!("http://{host}"))
        .ok()?
        .host_str()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{cross_site_signal, host_of, CrossSite};
    use axum::http::{request::Parts, Request};

    fn parts(headers: &[(&str, &str)]) -> Parts {
        let mut req = Request::get("/api/w/ws/jobs/run_wait_result/p/hub/1/x");
        for (name, value) in headers {
            req = req.header(*name, *value);
        }
        req.body(()).unwrap().into_parts().0
    }

    #[test]
    fn sec_fetch_site_decides_when_present() {
        let declared = |site| cross_site_signal(&parts(&[("sec-fetch-site", site)]));
        assert!(matches!(declared("cross-site"), Some(CrossSite::Declared)));
        for site in ["same-origin", "same-site", "none"] {
            assert!(declared(site).is_none(), "{site} is not cross-site");
        }
        // The header outranks a `Referer` that disagrees with it.
        let with_referer = parts(&[
            ("sec-fetch-site", "same-origin"),
            ("host", "windmill.example"),
            ("referer", "https://attacker.example/page"),
        ]);
        assert!(cross_site_signal(&with_referer).is_none());
    }

    #[test]
    fn referer_stands_in_when_sec_fetch_site_is_absent() {
        let signal = |headers: &[(&str, &str)]| cross_site_signal(&parts(headers));
        assert!(matches!(
            signal(&[
                ("host", "windmill.example"),
                ("referer", "https://attacker.example/p")
            ]),
            Some(CrossSite::RefererMismatch { .. })
        ));
        // Ports differ between the frontend and the API, and do not make a request cross-site.
        assert!(signal(&[
            ("host", "windmill.example:8000"),
            ("referer", "http://windmill.example:3000/apps"),
        ])
        .is_none());
        // A proxy that rewrote `Host` but forwarded the public name.
        assert!(signal(&[
            ("host", "windmill-server.internal"),
            ("x-forwarded-host", "windmill.example"),
            ("referer", "https://windmill.example/apps"),
        ])
        .is_none());
        assert!(signal(&[("host", "windmill.example")]).is_none());
    }

    #[test]
    fn host_of_strips_port_brackets_and_proxy_chain() {
        assert_eq!(host_of("windmill.example"), Some("windmill.example".into()));
        assert_eq!(
            host_of("windmill.example:8000"),
            Some("windmill.example".into())
        );
        assert_eq!(host_of("[::1]:8000"), Some("[::1]".into()));
        assert_eq!(host_of("[::1]"), Some("[::1]".into()));
        assert_eq!(
            host_of("windmill.example, proxy.internal"),
            Some("windmill.example".into())
        );
        assert_eq!(host_of(""), None);
        assert_eq!(host_of("not a host"), None);
    }
}
