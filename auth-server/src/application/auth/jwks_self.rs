use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rsa::{pkcs8::DecodePrivateKey, traits::PublicKeyParts, RsaPrivateKey};
use serde_json::{json, Value};

pub fn build_jwks(current_pem: &str, previous_pem: Option<&str>) -> Result<Value, String> {
    let mut keys: Vec<Value> = Vec::new();

    keys.push(pem_to_jwk(current_pem, "v1")?);

    if let Some(prev) = previous_pem.filter(|p| !p.is_empty()) {
        keys.push(pem_to_jwk(prev, "v0")?);
    }

    Ok(json!({ "keys": keys }))
}

fn pem_to_jwk(pem: &str, kid: &str) -> Result<Value, String> {
    let private_key = RsaPrivateKey::from_pkcs8_pem(pem).map_err(|e| e.to_string())?;
    let public_key = private_key.to_public_key();

    let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());

    Ok(json!({
        "kty": "RSA",
        "use": "sig",
        "alg": "RS256",
        "kid": kid,
        "n": n,
        "e": e,
    }))
}
