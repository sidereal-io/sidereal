//! The scheme and the client address of a request, believed from forwarded
//! headers only when the peer is a trusted proxy.
use super::AccessError;
use crate::config::TrustedProxies;
use axum::{
    extract::{ConnectInfo, Request, State},
    http::HeaderMap,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub const FORWARDED_FOR: &str = "x-forwarded-for";
pub const FORWARDED_PROTO: &str = "x-forwarded-proto";

/// What the server resolved about a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Forwarded {
    /// Whether the request reached the trusted proxy over HTTPS.
    pub https: bool,
    /// The client address. `None` when the nearest untrusted
    /// `X-Forwarded-For` entry is not an address.
    pub client: Option<IpAddr>,
}

/// Hand every request its `Forwarded`, or refuse forwarded headers from a
/// peer that is not a trusted proxy. The refusal logs nothing and holds
/// nothing from the request, because its source is not trusted.
pub async fn layer(
    State(trusted): State<TrustedProxies>,
    mut request: Request,
    next: Next,
) -> Response {
    // No `ConnectInfo` means a wiring mistake or an in-process test. Such a
    // peer is never trusted, even by a list that covers `0.0.0.0`.
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(peer)| peer.ip());
    let trusted_peer = peer.is_some_and(|peer| trusted.contains(peer));
    let headers = request.headers();
    if !trusted_peer
        && (headers.contains_key(FORWARDED_FOR) || headers.contains_key(FORWARDED_PROTO))
    {
        return AccessError::UntrustedProxy.into_response();
    }
    let forwarded = match peer {
        Some(peer) => resolve(peer, headers, &trusted),
        None => Forwarded {
            https: false,
            client: Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        },
    };
    request.extensions_mut().insert(forwarded);
    next.run(request).await
}

pub fn resolve(peer: IpAddr, headers: &HeaderMap, trusted: &TrustedProxies) -> Forwarded {
    let peer = peer.to_canonical();
    if !trusted.contains(peer) {
        return Forwarded {
            https: false,
            client: Some(peer),
        };
    }
    Forwarded {
        https: https(headers),
        client: client(peer, headers, trusted),
    }
}

/// One header holding exactly `https`. A list, from one header or several,
/// is not believed.
fn https(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(FORWARDED_PROTO).iter();
    match (values.next(), values.next()) {
        (Some(value), None) => value
            .to_str()
            .is_ok_and(|value| value.trim().eq_ignore_ascii_case("https")),
        _ => false,
    }
}

/// The nearest `X-Forwarded-For` entry that is not a trusted proxy, read from
/// the last. Each proxy appends, so only the entries to the right of an
/// untrusted one were written by proxies the server trusts.
fn client(peer: IpAddr, headers: &HeaderMap, trusted: &TrustedProxies) -> Option<IpAddr> {
    for value in headers.get_all(FORWARDED_FOR).iter().rev() {
        // Split the raw bytes: text the walk never reaches may hold bytes
        // that `to_str` refuses, and must not hide the client.
        for entry in value.as_bytes().rsplit(|&byte| byte == b',') {
            let Ok(entry) = std::str::from_utf8(entry) else {
                return None;
            };
            match entry.trim().parse::<IpAddr>() {
                Ok(address) if trusted.contains(address) => continue,
                Ok(address) => return Some(address.to_canonical()),
                Err(_) => return None,
            }
        }
    }
    Some(peer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::trusted_proxies;
    use axum::http::HeaderValue;
    use std::ffi::OsStr;

    fn ip(value: &str) -> IpAddr {
        value.parse().unwrap()
    }

    fn trusting(value: &str) -> TrustedProxies {
        trusted_proxies(Some(OsStr::new(value))).unwrap()
    }

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.append(*name, value.parse().unwrap());
        }
        headers
    }

    fn from(peer: &str, trusted: &str, pairs: &[(&'static str, &str)]) -> Forwarded {
        resolve(ip(peer), &headers(pairs), &trusting(trusted))
    }

    #[test]
    fn an_untrusted_peer_is_plain_http_from_itself() {
        let resolved = from(
            "192.168.1.20",
            "172.18.0.1",
            &[(FORWARDED_PROTO, "https"), (FORWARDED_FOR, "203.0.113.9")],
        );
        assert_eq!(
            resolved,
            Forwarded {
                https: false,
                client: Some(ip("192.168.1.20")),
            }
        );
    }

    #[test]
    fn a_trusted_proxy_forwards_the_scheme_and_the_client() {
        let resolved = from(
            "172.18.0.1",
            "172.18.0.1",
            &[(FORWARDED_PROTO, "https"), (FORWARDED_FOR, "203.0.113.9")],
        );
        assert_eq!(
            resolved,
            Forwarded {
                https: true,
                client: Some(ip("203.0.113.9")),
            }
        );
    }

    #[test]
    fn the_proto_is_https_ignoring_case() {
        for value in ["HTTPS", "Https", " https "] {
            let resolved = from("172.18.0.1", "172.18.0.1", &[(FORWARDED_PROTO, value)]);
            assert!(resolved.https, "{value:?}");
        }
    }

    #[test]
    fn any_other_proto_is_plain_http() {
        for pairs in [
            &[(FORWARDED_PROTO, "http")][..],
            &[(FORWARDED_PROTO, "https, http")],
            &[(FORWARDED_PROTO, "https,https")],
            &[(FORWARDED_PROTO, "https"), (FORWARDED_PROTO, "https")],
            &[(FORWARDED_PROTO, "")],
            &[],
        ] {
            assert!(!from("172.18.0.1", "172.18.0.1", pairs).https, "{pairs:?}");
        }
    }

    #[test]
    fn a_spoofed_entry_behind_the_proxy_is_not_the_client() {
        let resolved = from(
            "172.18.0.1",
            "172.18.0.1",
            &[(FORWARDED_FOR, "10.0.0.1, 203.0.113.9")],
        );
        assert_eq!(resolved.client, Some(ip("203.0.113.9")));
    }

    #[test]
    fn a_chain_of_trusted_proxies_is_skipped() {
        let resolved = from(
            "172.18.0.1",
            "172.18.0.0/16",
            &[(FORWARDED_FOR, "203.0.113.9, 172.18.0.5")],
        );
        assert_eq!(resolved.client, Some(ip("203.0.113.9")));
    }

    #[test]
    fn repeated_headers_are_read_as_one_list() {
        let resolved = from(
            "172.18.0.1",
            "172.18.0.0/16",
            &[
                (FORWARDED_FOR, "203.0.113.9"),
                (FORWARDED_FOR, "172.18.0.5"),
            ],
        );
        assert_eq!(resolved.client, Some(ip("203.0.113.9")));
    }

    #[test]
    fn every_entry_trusted_leaves_the_peer_as_the_client() {
        let resolved = from(
            "172.18.0.1",
            "172.18.0.0/16",
            &[(FORWARDED_FOR, "172.18.0.5")],
        );
        assert_eq!(resolved.client, Some(ip("172.18.0.1")));
    }

    #[test]
    fn no_forwarded_for_leaves_the_peer_as_the_client() {
        let resolved = from("172.18.0.1", "172.18.0.1", &[(FORWARDED_PROTO, "https")]);
        assert_eq!(resolved.client, Some(ip("172.18.0.1")));
    }

    #[test]
    fn an_unparsable_entry_is_the_client() {
        for value in [
            "203.0.113.9, garbage",
            "203.0.113.9, 203.0.113.10:4000",
            "203.0.113.9, ",
        ] {
            let resolved = from("172.18.0.1", "172.18.0.1", &[(FORWARDED_FOR, value)]);
            assert_eq!(resolved.client, None, "{value:?}");
        }
    }

    /// A request from a trusted proxy whose `X-Forwarded-For` is these raw
    /// bytes, which need not be text.
    fn forwarded_for_bytes(value: &[u8]) -> Forwarded {
        let mut headers = HeaderMap::new();
        headers.insert(FORWARDED_FOR, HeaderValue::from_bytes(value).unwrap());
        resolve(ip("172.18.0.1"), &headers, &trusting("172.18.0.1"))
    }

    #[test]
    fn bytes_left_of_the_client_do_not_hide_it() {
        let resolved = forwarded_for_bytes(b"\xff, 203.0.113.9");
        assert_eq!(resolved.client, Some(ip("203.0.113.9")));
    }

    #[test]
    fn a_nearest_entry_that_is_not_text_is_the_client() {
        let resolved = forwarded_for_bytes(b"203.0.113.9, \xff");
        assert_eq!(resolved.client, None);
    }

    #[test]
    fn mapped_addresses_are_canonical() {
        let resolved = from(
            "::ffff:172.18.0.1",
            "172.18.0.1",
            &[(FORWARDED_FOR, "::ffff:203.0.113.9, ::ffff:172.18.0.1")],
        );
        assert_eq!(resolved.client, Some(ip("203.0.113.9")));
    }
}
