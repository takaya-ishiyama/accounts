use serde::Deserialize;
use worker::{wasm_bindgen::JsValue, Fetch, Method, Request, RequestInit};

use crate::{
    application::{
        auth::{jwks_google, oidc_state::OidcStateStore, session},
        http::error::{bad_request, internal, AppError},
        usecase::{
            commands::{dispatcher, dto::CommandDto},
            queries::account::find_account_id_by_google_subject,
        },
    },
    domain::auth::return_to,
    infrastructure::{
        env::{secret, var, Environment},
        ids::new_uuid_v7,
        time::now_ms,
    },
};

pub struct CompletedGoogleSignin {
    pub return_to: String,
    pub jwt: String,
    pub ttl_secs: u64,
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    id_token: String,
}

pub async fn complete_google_signin(
    env: &worker::Env,
    code: &str,
    oidc_state_store: &dyn OidcStateStore,
    state_key: &str,
) -> Result<CompletedGoogleSignin, AppError> {
    let environment = Environment::load(env)?;
    let kv = env.kv("KV")?;
    let oidc_state = oidc_state_store
        .load_and_delete(state_key)
        .await?
        .ok_or_else(|| bad_request("invalid_state"))?;

    let token = exchange_code(&environment, code, &oidc_state.code_verifier).await?;
    let kid =
        session::decode_kid(&token.id_token).ok_or_else(|| bad_request("invalid_id_token"))?;
    let jwks = jwks_google::get_jwks(&kv).await?;
    let key =
        jwks_google::find_key(&jwks, &kid).ok_or_else(|| bad_request("unknown_google_key"))?;
    let claims = session::verify_google_id_token(
        &token.id_token,
        &key.n,
        &key.e,
        &environment.google_client_id,
        &oidc_state.nonce,
    )
    .map_err(|_| bad_request("invalid_id_token"))?;

    if !claims.email_verified {
        return Err(bad_request("email_not_verified"));
    }

    let account_id = match find_account_id_by_google_subject(env, &claims.sub).await? {
        Some(id) => id,
        None => new_uuid_v7(),
    };
    let cmd = CommandDto::SignInWithGoogle {
        idempotency_key: new_uuid_v7(),
        subject: claims.sub,
        email: claims.email,
        email_verified: claims.email_verified,
        occurred_at: now_ms(),
    };
    let command_response = dispatcher::dispatch(env, &account_id, &cmd).await?;

    let session_config = SessionConfig::load(env)?;
    let jwt = session::issue_jwt(
        &session_config.private_key,
        "v1",
        &command_response.account_id,
        &session_config.issuer,
        &session_config.audience,
        session_config.ttl_secs,
    )
    .map_err(internal)?;

    Ok(CompletedGoogleSignin {
        return_to: return_to::sanitize(&oidc_state.return_to).into_owned(),
        jwt,
        ttl_secs: session_config.ttl_secs,
    })
}

async fn exchange_code(
    env: &Environment,
    code: &str,
    code_verifier: &str,
) -> Result<GoogleTokenResponse, AppError> {
    let form = form_encode(&[
        ("code", code),
        ("client_id", &env.google_client_id),
        ("client_secret", &env.google_client_secret),
        ("redirect_uri", &env.oauth_redirect_uri),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier),
    ]);

    let headers = worker::Headers::new();
    headers.set("Content-Type", "application/x-www-form-urlencoded")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_headers(headers);
    init.with_body(Some(JsValue::from_str(&form)));

    let req = Request::new_with_init("https://oauth2.googleapis.com/token", &init)?;
    let mut resp = Fetch::Request(req).send().await?;
    if !(200..300).contains(&resp.status_code()) {
        return Err(bad_request("token_exchange_failed"));
    }

    resp.json::<GoogleTokenResponse>()
        .await
        .map_err(|e| internal(e.to_string()))
}

struct SessionConfig {
    private_key: String,
    issuer: String,
    audience: String,
    ttl_secs: u64,
}

impl SessionConfig {
    fn load(env: &worker::Env) -> worker::Result<Self> {
        let ttl_secs = match env.var("SESSION_TTL_SECS") {
            Ok(v) => v.to_string().parse().unwrap_or(3600),
            Err(_) => 3600,
        };
        Ok(Self {
            private_key: secret(env, "JWT_PRIVATE_KEY")?,
            issuer: var(env, "JWT_ISSUER")?,
            audience: var(env, "JWT_DEFAULT_AUDIENCE")?,
            ttl_secs,
        })
    }
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
