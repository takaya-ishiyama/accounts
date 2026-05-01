use crate::domain::auth::{
    authorization::{build_url, AuthorizationRequest},
    pkce::code_challenge_s256,
    provider::{IdentityProvider, ProviderError, StartCtx, StartResponse},
    token::TokenGenerator,
};
use crate::infrastructure::env::Environment;

const GOOGLE_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_SCOPE: &str = "openid email";

pub struct GoogleProvider<'a> {
    pub env: &'a Environment,
    pub tokens: &'a dyn TokenGenerator,
}

impl<'a> IdentityProvider for GoogleProvider<'a> {
    fn id(&self) -> &'static str {
        "google"
    }

    fn start(&self, _ctx: &StartCtx<'_>) -> Result<StartResponse, ProviderError> {
        let state_key = self.tokens.generate(32);
        let code_verifier = self.tokens.generate(32);
        let nonce = self.tokens.generate(32);
        let code_challenge = code_challenge_s256(&code_verifier);

        let authorization_url = build_url(&AuthorizationRequest {
            authorization_endpoint: GOOGLE_AUTH_ENDPOINT,
            client_id: &self.env.google_client_id,
            redirect_uri: &self.env.oauth_redirect_uri,
            scope: GOOGLE_SCOPE,
            state: &state_key,
            nonce: &nonce,
            code_challenge: &code_challenge,
        });

        Ok(StartResponse {
            authorization_url,
            state_key,
            code_verifier,
            nonce,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::auth::token::TokenGenerator;

    struct FakeTokens;
    impl TokenGenerator for FakeTokens {
        fn generate(&self, _byte_len: usize) -> String {
            "fixed_token".to_string()
        }
    }

    #[test]
    fn start_builds_valid_authorization_url() {
        let env = Environment {
            google_client_id: "client123".to_string(),
            google_client_secret: "secret".to_string(),
            oauth_redirect_uri: "https://example.com/auth/google/callback".to_string(),
        };
        let tokens = FakeTokens;
        let provider = GoogleProvider { env: &env, tokens: &tokens };

        let resp = provider
            .start(&StartCtx { return_to: "/dashboard" })
            .unwrap();

        assert!(resp.authorization_url.contains("client_id=client123"));
        assert!(resp.authorization_url.contains("code_challenge_method=S256"));
        assert!(resp.authorization_url.contains("state=fixed_token"));
        assert!(resp.authorization_url.contains("nonce=fixed_token"));
        assert_eq!(resp.state_key, "fixed_token");
        assert_eq!(resp.code_verifier, "fixed_token");
        assert_eq!(resp.nonce, "fixed_token");
    }
}
