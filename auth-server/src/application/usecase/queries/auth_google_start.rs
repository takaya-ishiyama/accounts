use axum::response::Redirect;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use worker::kv::KvStore;

use crate::{
    application::{
        auth::oidc_state::{save, OidcStateValue},
        http::error::AppError,
    },
    infrastructure::{env::Environment, random},
};

pub async fn auth_google_start(
    env: &Environment,
    kv: &KvStore,
    return_to: &str,
) -> Result<Redirect, AppError> {
    let state_value = random::random_base64url(32);
    let code_verifier = random::random_base64url(32);
    let nonce = random::random_base64url(32);

    let code_challenge = {
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        URL_SAFE_NO_PAD.encode(hasher.finalize())
    };

    save(
        kv,
        &state_value,
        &OidcStateValue {
            code_verifier,
            nonce: nonce.clone(),
            return_to: sanitize_return_to(return_to),
        },
    )
    .await?;

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
        ?response_type=code\
        &client_id={}\
        &redirect_uri={}\
        &scope=openid%20email\
        &state={state_value}\
        &nonce={nonce}\
        &code_challenge={code_challenge}\
        &code_challenge_method=S256",
        env.google_client_id,
        percent_encode(&env.oauth_redirect_uri),
    );

    Ok(Redirect::to(&auth_url))
}

fn sanitize_return_to(raw: &str) -> String {
    let decoded = percent_decode(raw);
    if decoded.starts_with('/') && !decoded.starts_with("//") && !decoded.contains('\\') {
        raw.to_string()
    } else {
        "/".to_string()
    }
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
