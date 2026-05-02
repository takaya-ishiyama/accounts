use serde::Deserialize;
use worker::{kv::KvStore, wasm_bindgen::JsValue, Fetch, Method, Request, RequestInit};

use crate::{
    application::auth::{jwks_google, session},
    domain::auth::{
        authorization::{build_url, AuthorizationRequest},
        identity::VerifiedIdentity,
        pkce::code_challenge_s256,
        provider::{
            CallbackCtx, IdentityProvider, ProviderError, ProviderFuture, StartCtx, StartResponse,
        },
        token::TokenGenerator,
    },
    domain::clock::Clock,
    infrastructure::env::Environment,
};

const GOOGLE_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_SCOPE: &str = "openid email";

pub struct GoogleProvider<'a> {
    pub env: &'a Environment,
    pub kv: &'a KvStore,
    pub tokens: &'a dyn TokenGenerator,
    pub clock: &'a dyn Clock,
}

impl IdentityProvider for GoogleProvider<'_> {
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

    fn callback<'a>(&'a self, ctx: &'a CallbackCtx<'a>) -> ProviderFuture<'a, VerifiedIdentity> {
        Box::pin(async move {
            let token = exchange_code(self.env, ctx.code, ctx.code_verifier)
                .await
                .map_err(ProviderError::InvalidGrant)?;

            let kid = session::decode_kid(&token.id_token)
                .ok_or_else(|| ProviderError::InvalidGrant("invalid_id_token".to_string()))?;

            let jwks = jwks_google::get_jwks(self.kv)
                .await
                .map_err(|e| ProviderError::Internal(e.to_string()))?;

            let key = jwks_google::find_key(&jwks, &kid)
                .ok_or_else(|| ProviderError::InvalidGrant("unknown_google_key".to_string()))?;

            let now_secs = self.clock.now_ms() / 1000;
            let claims = session::verify_google_id_token(
                &token.id_token,
                &key.n,
                &key.e,
                &self.env.google_client_id,
                ctx.nonce,
                now_secs,
            )
            .map_err(|e| ProviderError::InvalidGrant(e))?;

            Ok(VerifiedIdentity {
                provider: "google".to_string(),
                subject: claims.sub,
                email: claims.email,
                email_verified: claims.email_verified,
            })
        })
    }
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    id_token: String,
}

async fn exchange_code(
    env: &Environment,
    code: &str,
    code_verifier: &str,
) -> Result<GoogleTokenResponse, String> {
    let form = form_encode(&[
        ("code", code),
        ("client_id", &env.google_client_id),
        ("client_secret", &env.google_client_secret),
        ("redirect_uri", &env.oauth_redirect_uri),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier),
    ]);

    let headers = worker::Headers::new();
    headers
        .set("Content-Type", "application/x-www-form-urlencoded")
        .map_err(|e| e.to_string())?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_headers(headers);
    init.with_body(Some(JsValue::from_str(&form)));

    let req = Request::new_with_init("https://oauth2.googleapis.com/token", &init)
        .map_err(|e| e.to_string())?;
    let mut resp = Fetch::Request(req)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !(200..300).contains(&resp.status_code()) {
        return Err("token_exchange_failed".to_string());
    }

    resp.json::<GoogleTokenResponse>()
        .await
        .map_err(|e| e.to_string())
}

fn form_encode(params: &[(&str, &str)]) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", encode_form_component(k), encode_form_component(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn encode_form_component(value: &str) -> String {
    let mut encoded = String::new();
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(b as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{b:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_encode_escapes_reserved_chars() {
        assert_eq!(
            form_encode(&[
                ("code", "a/b c"),
                ("redirect_uri", "https://example.com/cb?x=1")
            ]),
            "code=a%2Fb+c&redirect_uri=https%3A%2F%2Fexample.com%2Fcb%3Fx%3D1"
        );
    }
}
