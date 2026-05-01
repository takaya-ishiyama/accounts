use std::borrow::Cow;

pub fn sanitize(raw: &str) -> Cow<'_, str> {
    let decoded = percent_decode(raw);
    if decoded.starts_with('/') && !decoded.starts_with("//") && !decoded.contains('\\') {
        Cow::Borrowed(raw)
    } else {
        Cow::Owned("/".to_string())
    }
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
}
