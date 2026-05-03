use axum::http::HeaderMap;

pub const SESSION_COOKIE: &str = "__Host-session";

pub fn extract_session_cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(find_session_cookie)
}

fn find_session_cookie(header: &str) -> Option<&str> {
    header.split(';').find_map(|part| {
        let trimmed = part.trim();
        let (name, value) = trimmed.split_once('=')?;
        (name == SESSION_COOKIE && !value.is_empty()).then_some(value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{header, HeaderMap, HeaderValue};

    #[test]
    fn extracts_session_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("a=1; __Host-session=token; b=2"),
        );

        assert_eq!(extract_session_cookie(&headers), Some("token"));
    }
}
