pub struct AuthorizationRequest<'a> {
    pub authorization_endpoint: &'a str,
    pub client_id: &'a str,
    pub redirect_uri: &'a str,
    pub scope: &'a str,
    pub state: &'a str,
    pub nonce: &'a str,
    pub code_challenge: &'a str,
}

pub fn build_url(req: &AuthorizationRequest<'_>) -> String {
    format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}&code_challenge={}&code_challenge_method=S256",
        req.authorization_endpoint,
        req.client_id,
        percent_encode(req.redirect_uri),
        percent_encode(req.scope),
        req.state,
        req.nonce,
        req.code_challenge,
    )
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_required_params() {
        let url = build_url(&AuthorizationRequest {
            authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth",
            client_id: "client123",
            redirect_uri: "https://example.com/callback",
            scope: "openid email",
            state: "state_val",
            nonce: "nonce_val",
            code_challenge: "challenge_val",
        });
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=client123"));
        assert!(url.contains("state=state_val"));
        assert!(url.contains("nonce=nonce_val"));
        assert!(url.contains("code_challenge=challenge_val"));
        assert!(url.contains("code_challenge_method=S256"));
    }
}
