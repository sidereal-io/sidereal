//! The session cookie, by hand: one unsigned cookie needs no cookie crate.
use axum::http::{header::COOKIE, HeaderMap};
use sidereal_core::access::SESSION_LIFETIME;

pub const NAME: &str = "sidereal_session";

/// The `Set-Cookie` value that hands the browser its token. `secure` says
/// whether the request arrived over HTTPS.
pub fn session(token: &str, secure: bool) -> String {
    build(token, SESSION_LIFETIME.as_secs(), secure)
}

/// The `Set-Cookie` value that makes the browser forget its token.
pub fn cleared(secure: bool) -> String {
    build("", 0, secure)
}

fn build(value: &str, max_age: u64, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{NAME}={value}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age}{secure}")
}

/// The session token from the request's `Cookie` headers, if any.
///
/// Browsers separate pairs with `; `, so each pair is trimmed before it is
/// split on its first `=`. A token is base64url and holds no `=`.
pub fn token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find_map(|(name, value)| (name == NAME).then_some(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes(value: &str) -> Vec<&str> {
        value.split("; ").collect()
    }

    fn headers(values: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append(COOKIE, value.parse().unwrap());
        }
        headers
    }

    #[test]
    fn the_session_cookie_is_scoped_to_the_browser() {
        let cookie = session("abc", false);
        let parts = attributes(&cookie);
        assert_eq!(parts[0], "sidereal_session=abc");
        for attribute in ["HttpOnly", "SameSite=Lax", "Path=/", "Max-Age=2592000"] {
            assert!(parts.contains(&attribute), "{attribute} in {cookie}");
        }
    }

    #[test]
    fn the_session_cookie_is_secure_only_over_https() {
        assert!(!attributes(&session("abc", false)).contains(&"Secure"));
        assert!(attributes(&session("abc", true)).contains(&"Secure"));
    }

    #[test]
    fn the_cleared_cookie_is_empty_and_expired() {
        let cookie = cleared(false);
        let parts = attributes(&cookie);
        assert_eq!(parts[0], "sidereal_session=");
        assert!(parts.contains(&"Max-Age=0"));
        assert!(parts.contains(&"Path=/"));
    }

    #[test]
    fn the_token_is_found_among_other_cookies() {
        for (case, values) in [
            ("alone", &["sidereal_session=abc"][..]),
            ("last", &["theme=dark; sidereal_session=abc"]),
            ("first", &["sidereal_session=abc; theme=dark"]),
            ("no space", &["theme=dark;sidereal_session=abc"]),
            ("second header", &["theme=dark", "sidereal_session=abc"]),
        ] {
            assert_eq!(token(&headers(values)), Some("abc"), "{case}");
        }
    }

    #[test]
    fn a_similar_name_is_not_the_token() {
        for values in [
            &["xsidereal_session=abc"][..],
            &["sidereal_session_old=abc"],
            &["theme=dark"],
            &[],
        ] {
            assert_eq!(token(&headers(values)), None, "{values:?}");
        }
    }
}
