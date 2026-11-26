//! Where the request being served came from: the client's IP address and user agent, as
//! recorded on the audit entries it writes.

use std::net::IpAddr;
use std::sync::Arc;

use http::HeaderMap;

use crate::utils::truncate_chars;

const USER_AGENT_MAX_CHARS: usize = 512;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RequestOrigin {
    pub client_ip: Option<IpAddr>,
    pub user_agent: Option<String>,
}

tokio::task_local! {
    static REQUEST_ORIGIN: Arc<RequestOrigin>;
}

/// Run `f` with `origin` as the origin of every audit entry it writes. Task-locals do not
/// cross `tokio::spawn`, so an entry written from a spawned task records no origin.
pub async fn scope<F: std::future::Future>(origin: RequestOrigin, f: F) -> F::Output {
    REQUEST_ORIGIN.scope(Arc::new(origin), f).await
}

/// The origin of the request being served, `None` outside a request.
pub fn current() -> Option<Arc<RequestOrigin>> {
    REQUEST_ORIGIN.try_with(|o| o.clone()).ok()
}

impl RequestOrigin {
    /// `peer` is the address the connection came from; `None` when the request did not arrive
    /// over a socket, and then no header is believed either.
    pub fn from_request(peer: Option<IpAddr>, headers: &HeaderMap) -> Self {
        Self {
            client_ip: peer.map(|p| {
                crate::client_ip::resolve_client_ip(p, headers, &crate::client_ip::CLIENT_IP_CONFIG)
            }),
            user_agent: headers
                .get(http::header::USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| truncate_chars(s, USER_AGENT_MAX_CHARS).to_string()),
        }
    }
}
