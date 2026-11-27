//! The address a request comes from, for decisions that must not be spoofable (per-IP rate
//! limits) and for the address audit entries record (`request_origin`). Both read it here, so
//! they never disagree about one request.
//!
//! Forwarding headers are believed only when the TCP peer is in `TRUSTED_PROXIES`. Then either
//! `CLIENT_IP_HEADER` names the one header the edge in front writes authoritatively, or
//! `X-Forwarded-For` is read from the right, stopping at the first hop that is not itself a
//! trusted proxy. Everything left of that hop was written by the client and is ignored.
//! `X-Real-IP` is never read implicitly: proxies that do not set it pass the client's own through.

use std::net::{IpAddr, SocketAddr};
use std::sync::LazyLock;

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::{request::Parts, Extensions, HeaderMap, HeaderName};
pub use ipnet::IpNet;

/// Loopback only on a self-hosted instance: it is often reached directly from a private network,
/// whose clients could otherwise name any address they like. A reverse proxy on a private
/// address must be listed in `TRUSTED_PROXIES`, or its own address is taken as the client's.
const DEFAULT_TRUSTED_PROXIES: &[&str] = &["127.0.0.0/8", "::1/128"];
/// Added on cloud, where every request arrives through our own load balancer.
const CLOUD_TRUSTED_PROXIES: &[&str] =
    &["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16", "fc00::/7"];

fn default_trusted_proxies(cloud_hosted: bool) -> String {
    let mut nets = DEFAULT_TRUSTED_PROXIES.to_vec();
    if cloud_hosted {
        nets.extend(CLOUD_TRUSTED_PROXIES);
    }
    nets.join(",")
}

pub struct ClientIpConfig {
    pub trusted: Vec<IpNet>,
    pub header: Option<HeaderName>,
}

/// `TRUSTED_PROXIES`: comma-separated IPs or CIDRs; set but empty, or `none`, trusts no proxy.
/// `CLIENT_IP_HEADER`: a header such as `CloudFront-Viewer-Address`, `CF-Connecting-IP` or
/// `X-Real-IP` that the trusted edge overwrites on every request, read instead of
/// `X-Forwarded-For`.
pub static CLIENT_IP_CONFIG: LazyLock<ClientIpConfig> = LazyLock::new(|| {
    let trusted = match std::env::var("TRUSTED_PROXIES") {
        Ok(v) => parse_trusted_proxies(&v),
        Err(_) => parse_trusted_proxies(&default_trusted_proxies(*crate::worker::CLOUD_HOSTED)),
    };
    let header = std::env::var("CLIENT_IP_HEADER")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .and_then(|v| match HeaderName::try_from(v.trim()) {
            Ok(h) => Some(h),
            Err(_) => {
                tracing::error!("CLIENT_IP_HEADER: {v:?} is not a header name, ignoring it");
                None
            }
        });
    ClientIpConfig { trusted, header }
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
/// peer (a router served without connect info, as in tests or internal dispatch).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClientIp(pub Option<IpAddr>);

impl<S: Send + Sync> FromRequestParts<S> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(ClientIp(client_ip_of_parts(parts)))
    }
}

pub fn client_ip_of_parts(parts: &Parts) -> Option<IpAddr> {
    client_ip_of(&parts.extensions, &parts.headers)
}

/// The same, for a handler holding the whole `Request` rather than its parts.
pub fn client_ip_of(extensions: &Extensions, headers: &HeaderMap) -> Option<IpAddr> {
    let peer = extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())?;
    Some(resolve_client_ip(peer, headers, &CLIENT_IP_CONFIG))
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

/// An address as proxies write it: bare, `ip:port` or `[v6]:port`.
fn parse_addr(bytes: &[u8]) -> Option<IpAddr> {
    let s = std::str::from_utf8(bytes).ok()?.trim();
    s.parse::<IpAddr>()
        .ok()
        .or_else(|| s.parse::<SocketAddr>().ok().map(|a| a.ip()))
        .map(canonical)
}

/// `CloudFront-Viewer-Address` always ends in the source port, after an unbracketed IPv6
/// address. Read generically, `2001:db8::1:4430` is itself a valid address, so each source port
/// would name a different client.
fn parse_addr_then_port(bytes: &[u8]) -> Option<IpAddr> {
    let s = std::str::from_utf8(bytes).ok()?.trim();
    let (ip, port) = s.rsplit_once(':')?;
    port.parse::<u16>().ok()?;
    let ip = ip
        .strip_prefix('[')
        .and_then(|i| i.strip_suffix(']'))
        .unwrap_or(ip);
    ip.parse::<IpAddr>().ok().map(canonical)
}

pub fn resolve_client_ip(peer: IpAddr, headers: &HeaderMap, cfg: &ClientIpConfig) -> IpAddr {
    let peer = canonical(peer);
    if !is_trusted(peer, &cfg.trusted) {
        return peer;
    }
    if let Some(name) = &cfg.header {
        let mut values = headers.get_all(name).iter();
        return match (values.next(), values.next()) {
            (Some(v), None) if name.as_str() == "cloudfront-viewer-address" => {
                parse_addr_then_port(v.as_bytes()).unwrap_or(peer)
            }
            (Some(v), None) => parse_addr(v.as_bytes()).unwrap_or(peer),
            _ => peer,
        };
    }
    // Split on raw bytes: a field that is not valid UTF-8 must stay in the list as a hop we cannot
    // read, not vanish and let the fields around it shift into its place.
    let hops: Vec<&[u8]> = headers
        .get_all("x-forwarded-for")
        .iter()
        .flat_map(|v| v.as_bytes().split(|b| *b == b','))
        .collect();
    let mut client = peer;
    for hop in hops.iter().rev() {
        // A hop we cannot read ends what can be believed: the last trusted address stands.
        let Some(ip) = parse_addr(hop) else {
            return client;
        };
        client = ip;
        if !is_trusted(ip, &cfg.trusted) {
            return ip;
        }
    }
    client
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(pairs: &[(&'static str, &[u8])]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.append(*k, HeaderValue::from_bytes(v).unwrap());
        }
        h
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    fn cfg(trusted: &str, header: Option<&'static str>) -> ClientIpConfig {
        ClientIpConfig {
            trusted: parse_trusted_proxies(trusted),
            header: header.map(HeaderName::from_static),
        }
    }

    /// Cloud's default, which trusts the private ranges most cases here put a proxy on.
    fn defaults() -> ClientIpConfig {
        cfg(&default_trusted_proxies(true), None)
    }

    #[test]
    fn self_hosted_default_trusts_only_loopback() {
        let c = cfg(&default_trusted_proxies(false), None);
        let h = headers(&[("x-forwarded-for", b"5.5.5.5")]);
        assert_eq!(resolve_client_ip(ip("10.0.0.1"), &h, &c), ip("10.0.0.1"));
        assert_eq!(resolve_client_ip(ip("127.0.0.1"), &h, &c), ip("5.5.5.5"));
    }

    #[test]
    fn untrusted_peer_ignores_forwarding_headers() {
        let h = headers(&[("x-forwarded-for", b"10.0.0.9")]);
        assert_eq!(
            resolve_client_ip(ip("1.2.3.4"), &h, &defaults()),
            ip("1.2.3.4")
        );
        let c = cfg("10.0.0.0/8", Some("cf-connecting-ip"));
        let h = headers(&[("cf-connecting-ip", b"5.5.5.5")]);
        assert_eq!(resolve_client_ip(ip("1.2.3.4"), &h, &c), ip("1.2.3.4"));
    }

    #[test]
    fn forwarded_for_is_read_from_the_right() {
        // A client-written leftmost entry is skipped: the proxy appended the real address.
        let h = headers(&[("x-forwarded-for", b"6.6.6.6, 5.5.5.5, 10.0.0.2")]);
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &defaults()),
            ip("5.5.5.5")
        );
        let h = headers(&[
            ("x-forwarded-for", b"6.6.6.6"),
            ("x-forwarded-for", b"5.5.5.5"),
        ]);
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &defaults()),
            ip("5.5.5.5")
        );
    }

    #[test]
    fn real_ip_is_not_read_implicitly() {
        let h = headers(&[("x-real-ip", b"5.5.5.5")]);
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &defaults()),
            ip("10.0.0.1")
        );
    }

    #[test]
    fn unreadable_hop_stops_at_the_last_trusted_address() {
        let h = headers(&[("x-forwarded-for", b"5.5.5.5, garbage, 10.0.0.2")]);
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &defaults()),
            ip("10.0.0.2")
        );
        // A non-UTF-8 field is a hop too, not a field to drop.
        let h = headers(&[
            ("x-forwarded-for", b"6.6.6.6"),
            ("x-forwarded-for", b"\x80, 10.0.0.2"),
        ]);
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &defaults()),
            ip("10.0.0.2")
        );
    }

    #[test]
    fn a_configured_cdn_range_is_walked_through() {
        let c = cfg("10.0.0.0/8, 130.176.0.0/16", None);
        let h = headers(&[("x-forwarded-for", b"5.5.5.5, 130.176.1.1")]);
        assert_eq!(resolve_client_ip(ip("10.0.0.1"), &h, &c), ip("5.5.5.5"));
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &defaults()),
            ip("130.176.1.1")
        );
    }

    #[test]
    fn configured_header_replaces_forwarded_for() {
        let c = cfg("10.0.0.0/8", Some("cloudfront-viewer-address"));
        let h = headers(&[
            ("x-forwarded-for", b"6.6.6.6"),
            (
                "cloudfront-viewer-address",
                b"2001:0db8:85a3:0000:0000:8a2e:0370:7334:46532",
            ),
        ]);
        assert_eq!(
            resolve_client_ip(ip("10.0.0.1"), &h, &c),
            ip("2001:db8:85a3::8a2e:370:7334")
        );
        let h = headers(&[("cloudfront-viewer-address", b"5.5.5.5:443")]);
        assert_eq!(resolve_client_ip(ip("10.0.0.1"), &h, &c), ip("5.5.5.5"));
        // Compressed IPv6 with a short port also parses as a bare address: the port must go.
        for port in ["4430", "4431"] {
            let v = format!("2001:db8::1:{port}");
            let h = headers(&[("cloudfront-viewer-address", v.as_bytes())]);
            assert_eq!(resolve_client_ip(ip("10.0.0.1"), &h, &c), ip("2001:db8::1"));
        }
        // Absent, repeated or unreadable: the peer, never a guess.
        let h = headers(&[("x-forwarded-for", b"6.6.6.6")]);
        assert_eq!(resolve_client_ip(ip("10.0.0.1"), &h, &c), ip("10.0.0.1"));
        let h = headers(&[
            ("cloudfront-viewer-address", b"5.5.5.5:1"),
            ("cloudfront-viewer-address", b"6.6.6.6:1"),
        ]);
        assert_eq!(resolve_client_ip(ip("10.0.0.1"), &h, &c), ip("10.0.0.1"));
    }

    #[test]
    fn mapped_v4_is_v4_and_none_trusts_no_proxy() {
        assert_eq!(
            resolve_client_ip(ip("::ffff:1.2.3.4"), &HeaderMap::new(), &defaults()),
            ip("1.2.3.4")
        );
        let h = headers(&[("x-forwarded-for", b"5.5.5.5")]);
        assert_eq!(
            resolve_client_ip(ip("127.0.0.1"), &h, &cfg("none", None)),
            ip("127.0.0.1")
        );
        assert_eq!(
            resolve_client_ip(ip("127.0.0.1"), &h, &cfg("", None)),
            ip("127.0.0.1")
        );
    }
}
