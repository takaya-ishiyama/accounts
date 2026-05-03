use std::borrow::Cow;

pub fn sanitize(raw: &str) -> Cow<'_, str> {
    if is_safe_path(raw) {
        Cow::Borrowed(raw)
    } else {
        Cow::Owned("/".to_string())
    }
}

/// Like `sanitize`, but also accepts absolute URLs whose origin is in `allowed_origins`.
/// Use this in development/staging where the auth-server and the app run on different hosts.
pub fn sanitize_with_allowed_origins<'a>(raw: &'a str, allowed_origins: &[&str]) -> Cow<'a, str> {
    if is_safe_path(raw) {
        return Cow::Borrowed(raw);
    }
    if !allowed_origins.is_empty() {
        if let Some(origin) = extract_origin(raw) {
            if allowed_origins.iter().any(|&o| o == origin) {
                return Cow::Borrowed(raw);
            }
        }
    }
    Cow::Owned("/".to_string())
}

fn is_safe_path(s: &str) -> bool {
    let decoded = percent_decode(s);
    decoded.starts_with('/') && !decoded.starts_with("//") && !decoded.contains('\\')
}

fn extract_origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let host_end = rest
        .find(['/', '?', '#'])
        .unwrap_or(rest.len());
    Some(format!("{}://{}", scheme, &rest[..host_end]))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(b) = hex_byte(bytes[i + 1], bytes[i + 2]) {
                out.push(b as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn hex_byte(hi: u8, lo: u8) -> Option<u8> {
    Some((hex_val(hi)? << 4) | hex_val(lo)?)
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_paths() {
        assert_eq!(sanitize("/").as_ref(), "/");
        assert_eq!(sanitize("/dashboard").as_ref(), "/dashboard");
        assert_eq!(sanitize("/path/to/page?q=1").as_ref(), "/path/to/page?q=1");
    }

    #[test]
    fn rejects_protocol_relative() {
        assert_eq!(sanitize("//evil.com").as_ref(), "/");
    }

    #[test]
    fn rejects_double_encoded_protocol_relative() {
        // %2F%2F at start decodes to //evil.com — same-origin bypass attempt
        assert_eq!(sanitize("%2F%2Fevil.com").as_ref(), "/");
    }

    #[test]
    fn rejects_backslash() {
        assert_eq!(sanitize("/p\\ath").as_ref(), "/");
    }

    #[test]
    fn rejects_absolute_url() {
        assert_eq!(sanitize("https://evil.com").as_ref(), "/");
    }

    #[test]
    fn allowed_origin_accepts_absolute_url() {
        let allowed = ["http://localhost:5173"];
        assert_eq!(
            sanitize_with_allowed_origins("http://localhost:5173/api/auth/callback", &allowed).as_ref(),
            "http://localhost:5173/api/auth/callback"
        );
    }

    #[test]
    fn allowed_origin_rejects_unknown_absolute_url() {
        let allowed = ["http://localhost:5173"];
        assert_eq!(
            sanitize_with_allowed_origins("https://evil.com/steal", &allowed).as_ref(),
            "/"
        );
    }

    #[test]
    fn allowed_origin_still_accepts_relative_paths() {
        let allowed = ["http://localhost:5173"];
        assert_eq!(
            sanitize_with_allowed_origins("/dashboard", &allowed).as_ref(),
            "/dashboard"
        );
    }

    #[test]
    fn extract_origin_works() {
        assert_eq!(
            extract_origin("http://localhost:5173/api/auth/callback"),
            Some("http://localhost:5173".to_string())
        );
        assert_eq!(
            extract_origin("https://example.com/path?q=1"),
            Some("https://example.com".to_string())
        );
        assert_eq!(extract_origin("/relative"), None);
    }
}
