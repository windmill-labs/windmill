//! The address a request comes from, for decisions that must not be spoofable (per-IP rate
//! limits, token IP allowlists).
//!
//! Forwarding headers are believed only as far as they were written by a trusted proxy: the
//! peer of the TCP connection must be in `TRUSTED_PROXIES`, and `X-Forwarded-For` is read from
//! the right, stopping at the first hop that is not itself a trusted proxy. Everything left of
//! that hop was written by the client and is ignored.

use std::net::{IpAddr, SocketAddr};
use std::sync::LazyLock;

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::{request::Parts, HeaderMap};
use ipnet::IpNet;

/// Loopback, private and link-local networks: where a reverse proxy in front of Windmill
/// usually connects from. Anything else in front (a CDN, a public load balancer) must be listed
/// in `TRUSTED_PROXIES`, or its own address is taken as the client's.
const DEFAULT_TRUSTED_PROXIES: &[&str] = &[
    "127.0.0.0/8",
    "::1/128",
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "169.254.0.0/16",
    "fc00::/7",
    "fe80::/10",
];

/// `TRUSTED_PROXIES`: comma-separated IPs or CIDRs. Set but empty, or `none`, trusts no proxy, so
/// the connection's peer is always the client.
pub static TRUSTED_PROXIES: LazyLock<Vec<IpNet>> =
    LazyLock::new(|| match std::env::var("TRUSTED_PROXIES") {
        Ok(v) => parse_trusted_proxies(&v),
        Err(_) => DEFAULT_TRUSTED_PROXIES
            .iter()
            .map(|s| s.parse().expect("default trusted proxy is a valid CIDR"))
            .collect(),
    });

fn parse_trusted_proxies(v: &str) -> Vec<IpNet> {
    if v.trim().eq_ignore_ascii_case("none") {
        return vec![];
    }
    v.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| {
            let parsed = s
                .parse::<IpNet>()
                .ok()
                .or_else(|| s.parse::<IpAddr>().ok().map(IpNet::from));
            if parsed.is_none() {
                tracing::error!("TRUSTED_PROXIES: ignoring invalid entry {s:?}");
            }
            parsed
        })
        .collect()
}

/// The client address of a request: `None` when the server was not given the connection's
/// peer (a router served without connect info, as in tests).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClientIp(pub Option<IpAddr>);

impl<S: Send + Sync> FromRequestParts<S> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(ClientIp(client_ip_of_parts(parts)))
    }
}

pub fn client_ip_of_parts(parts: &Parts) -> Option<IpAddr> {
    let peer = parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())?;
    Some(resolve_client_ip(peer, &parts.headers, &TRUSTED_PROXIES))
}

/// An IPv4 client of a dual-stack listener arrives as `::ffff:a.b.c.d`; compare it as IPv4 so
/// IPv4 networks match it.
pub fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        v4 => v4,
    }
}

fn is_trusted(ip: IpAddr, trusted: &[IpNet]) -> bool {
    trusted.iter().any(|n| n.contains(&ip))
}

/// An `X-Forwarded-For` entry: a bare address, or one with a port as some proxies write it.
fn parse_hop(s: &str) -> Option<IpAddr> {
    let s = s.trim();
    s.parse::<IpAddr>()
        .ok()
        .or_else(|| s.parse::<SocketAddr>().ok().map(|a| a.ip()))
        .map(canonical)
}

pub fn resolve_client_ip(peer: IpAddr, headers: &HeaderMap, trusted: &[IpNet]) -> IpAddr {
    let peer = canonical(peer);
    if !is_trusted(peer, trusted) {
        return peer;
    }
    // Several X-Forwarded-For headers are one list, in order.
    let hops: Vec<&str> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .collect();
    if !hops.is_empty() {
        let mut client = peer;
        for hop in hops.iter().rev() {
            // A hop we cannot read ends what can be believed: the last trusted address stands.
            let Some(ip) = parse_hop(hop) else {
                return client;
            };
            client = ip;
            if !is_trusted(ip, trusted) {
                return ip;
            }
        }
        return client;
    }
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_hop)
        .unwrap_or(peer)
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

    fn defaults() -> Vec<IpNet> {
        parse_trusted_proxies(&DEFAULT_TRUSTED_PROXIES.join(","))
    }

    #[test]
    fn untrusted_peer_ignores_forwarding_headers() {
        let h = headers(&[("x-forwarded-for", "10.0.0.9"), ("x-real-ip", "10.0.0.9")]);
        assert_eq!(
            resolve_client_ip(ip("1.2.3.4"), &h, &defaults()),
            ip("1.2.3.4")
        );
    }

    #[test]
    fn forwarded_for_is_read_from_the_right() {
        // A client-written leftmost entry is skipped: the proxy appended the real address.
        let h = headers(&[("x-forwarded-for", "6.6.6.6, 5.5.5.5, 10.0.0.2")]);
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &defaults()),
            ip("5.5.5.5")
        );
        let h = headers(&[
            ("x-forwarded-for", "6.6.6.6"),
            ("x-forwarded-for", "5.5.5.5"),
        ]);
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &defaults()),
            ip("5.5.5.5")
        );
    }

    #[test]
    fn a_configured_cdn_range_is_walked_through() {
        let trusted = parse_trusted_proxies("10.0.0.0/8, 130.176.0.0/16");
        let h = headers(&[("x-forwarded-for", "5.5.5.5, 130.176.1.1")]);
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &trusted),
            ip("5.5.5.5")
        );
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &defaults()),
            ip("130.176.1.1")
        );
    }

    #[test]
    fn unreadable_hop_stops_at_the_last_trusted_address() {
        let h = headers(&[("x-forwarded-for", "5.5.5.5, garbage, 10.0.0.2")]);
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &defaults()),
            ip("10.0.0.2")
        );
    }

    #[test]
    fn real_ip_only_without_forwarded_for_and_mapped_v4_is_v4() {
        let h = headers(&[("x-real-ip", "5.5.5.5")]);
        assert_eq!(
            resolve_client_ip(ip("::ffff:10.0.0.1"), &h, &defaults()),
            ip("5.5.5.5")
        );
        assert_eq!(
            resolve_client_ip(ip("::ffff:1.2.3.4"), &HeaderMap::new(), &defaults()),
            ip("1.2.3.4")
        );
    }

    #[test]
    fn none_trusts_no_proxy() {
        let h = headers(&[("x-forwarded-for", "5.5.5.5")]);
        assert_eq!(
            resolve_client_ip(ip("127.0.0.1"), &h, &parse_trusted_proxies("none")),
            ip("127.0.0.1")
        );
        assert_eq!(
            resolve_client_ip(ip("127.0.0.1"), &h, &parse_trusted_proxies("")),
            ip("127.0.0.1")
        );
    }
}
