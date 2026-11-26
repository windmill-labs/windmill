//! Where the request being served came from: the client's IP address and user agent, as
//! recorded on the audit entries it writes.

use std::net::IpAddr;
use std::sync::{Arc, LazyLock};

use http::HeaderMap;
use ipnet::IpNet;

use crate::utils::truncate_chars;
use crate::worker::CLOUD_HOSTED;

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

/// The proxies whose forwarding headers are believed. `TRUSTED_PROXIES` (comma-separated
/// addresses or CIDRs, empty for none) replaces the default: loopback, plus the private
/// ranges on cloud, where every request arrives through our own load balancer. Private
/// ranges are not trusted by default elsewhere: a self-hosted instance is often reached
/// directly from a private network, and its clients could then name any address they like.
static TRUSTED_PROXIES: LazyLock<Vec<IpNet>> =
    LazyLock::new(|| match std::env::var("TRUSTED_PROXIES") {
        Ok(v) => parse_trusted_proxies(&v),
        Err(_) => {
            let mut nets = vec!["127.0.0.0/8", "::1/128"];
            if *CLOUD_HOSTED {
                nets.extend(["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16", "fc00::/7"]);
            }
            nets.into_iter().filter_map(|n| n.parse().ok()).collect()
        }
    });

fn parse_trusted_proxies(v: &str) -> Vec<IpNet> {
    v.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| {
            let parsed = s
                .parse::<IpNet>()
                .ok()
                .or_else(|| s.parse::<IpAddr>().ok().map(IpNet::from));
            if parsed.is_none() {
                tracing::warn!("Ignoring invalid TRUSTED_PROXIES entry: {s}");
            }
            parsed
        })
        .collect()
}

impl RequestOrigin {
    /// `peer` is the address the connection came from; `None` when the request did not arrive
    /// over a socket, and then no header is believed either.
    pub fn from_request(peer: Option<IpAddr>, headers: &HeaderMap) -> Self {
        Self {
            client_ip: peer.map(|p| client_ip(p, headers, &TRUSTED_PROXIES)),
            user_agent: headers
                .get(http::header::USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| truncate_chars(s, USER_AGENT_MAX_CHARS).to_string()),
        }
    }
}

fn parse_hop(s: &str) -> Option<IpAddr> {
    let s = s.trim();
    s.parse::<IpAddr>()
        .ok()
        .or_else(|| s.parse::<std::net::SocketAddr>().ok().map(|a| a.ip()))
        .map(|ip| ip.to_canonical())
}

/// The first address that is not a trusted proxy, walking `X-Forwarded-For` back from the
/// peer: each trusted hop vouches for the address before it, an untrusted one for nothing,
/// so entries left of it are whatever the client wrote. `X-Real-IP` is read only when there
/// is no `X-Forwarded-For`.
fn client_ip(peer: IpAddr, headers: &HeaderMap, trusted: &[IpNet]) -> IpAddr {
    let is_trusted = |ip: &IpAddr| trusted.iter().any(|n| n.contains(ip));
    let mut current = peer.to_canonical();
    if !is_trusted(&current) {
        return current;
    }
    // Bytes that are not UTF-8 are kept as entries that fail to parse, so they stop the walk
    // rather than vanish and let the client's own entries, or X-Real-IP, stand in.
    let values: Vec<String> = headers
        .get_all("x-forwarded-for")
        .iter()
        .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned())
        .collect();
    let hops: Vec<&str> = values.iter().flat_map(|v| v.split(',')).collect();
    if values.is_empty() {
        if let Some(ip) = headers
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(parse_hop)
        {
            return ip;
        }
        return current;
    }
    for hop in hops.iter().rev() {
        if !is_trusted(&current) {
            break;
        }
        match parse_hop(hop) {
            Some(ip) => current = ip,
            None => break,
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.append(*k, v.parse().unwrap());
        }
        h
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn forwarded_addresses_are_believed_only_through_trusted_hops() {
        let trusted = parse_trusted_proxies("10.0.0.0/8, 127.0.0.1");
        let xff = headers(&[("x-forwarded-for", "6.6.6.6, 1.2.3.4, 10.0.0.2")]);

        // An untrusted peer's headers say nothing.
        assert_eq!(client_ip(ip("8.8.8.8"), &xff, &trusted), ip("8.8.8.8"));
        // Trusted peer and trusted hop: the client is the first untrusted address, and the
        // entry the client itself wrote left of it is ignored.
        assert_eq!(client_ip(ip("10.0.0.1"), &xff, &trusted), ip("1.2.3.4"));
        // Every hop trusted: the leftmost one.
        let all_trusted = headers(&[("x-forwarded-for", "10.1.1.1, 10.0.0.2")]);
        assert_eq!(
            client_ip(ip("127.0.0.1"), &all_trusted, &trusted),
            ip("10.1.1.1")
        );
        // An unparseable hop stops the walk at the last address vouched for.
        let garbage = headers(&[("x-forwarded-for", "1.2.3.4, unknown")]);
        assert_eq!(
            client_ip(ip("10.0.0.1"), &garbage, &trusted),
            ip("10.0.0.1")
        );
        // A hop that is not UTF-8 stops the walk too, and does not let X-Real-IP in.
        let mut binary = headers(&[("x-real-ip", "5.5.5.5")]);
        binary.append(
            "x-forwarded-for",
            http::HeaderValue::from_bytes(b"\x80, 1.2.3.4").unwrap(),
        );
        assert_eq!(client_ip(ip("10.0.0.1"), &binary, &trusted), ip("1.2.3.4"));
        let mut binary_last = headers(&[("x-real-ip", "5.5.5.5")]);
        binary_last.append(
            "x-forwarded-for",
            http::HeaderValue::from_bytes(b"1.2.3.4, \x80").unwrap(),
        );
        assert_eq!(client_ip(ip("10.0.0.1"), &binary_last, &trusted), ip("10.0.0.1"));
        // X-Real-IP, only from a trusted peer and only without X-Forwarded-For.
        let real = headers(&[("x-real-ip", "5.5.5.5")]);
        assert_eq!(client_ip(ip("10.0.0.1"), &real, &trusted), ip("5.5.5.5"));
        assert_eq!(client_ip(ip("8.8.8.8"), &real, &trusted), ip("8.8.8.8"));
        // Nothing trusted.
        assert_eq!(client_ip(ip("10.0.0.1"), &xff, &[]), ip("10.0.0.1"));
    }
}
