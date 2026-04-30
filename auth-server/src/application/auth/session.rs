use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rsa::{pkcs1v15::{SigningKey, VerifyingKey, Signature}, pkcs8::DecodePrivateKey, RsaPrivateKey};
use rsa::signature::{SignatureEncoding, Signer as _, Verifier as _};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::infrastructure::{ids::new_uuid_v7, time::now_ms};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub jti: String,
    pub iat: i64,
    pub exp: i64,
}

pub fn issue_jwt(
    private_key_pem: &str,
    kid: &str,
    sub: &str,
    issuer: &str,
    audience: &str,
    ttl_secs: u64,
) -> Result<String, String> {
    let private_key =
        RsaPrivateKey::from_pkcs8_pem(private_key_pem).map_err(|e| e.to_string())?;
    let signing_key = SigningKey::<Sha256>::new(private_key);

    let now_secs = now_ms() / 1000;
    let jti = new_uuid_v7();

    let header = serde_json::json!({ "alg": "RS256", "typ": "JWT", "kid": kid });
    let payload = serde_json::json!({
        "iss": issuer,
        "sub": sub,
        "aud": audience,
        "jti": jti,
        "iat": now_secs,
        "exp": now_secs + ttl_secs as i64,
    });

    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string());
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string());
    let signing_input = format!("{header_b64}.{payload_b64}");

    let signature = signing_key.sign(signing_input.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

    Ok(format!("{signing_input}.{sig_b64}"))
}

/// Verify a session JWT against our own private keys (current + optional previous).
pub fn verify_session(
    token: &str,
    key_pems: &[&str],
    issuer: &str,
    audience: &str,
) -> Result<JwtClaims, String> {
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err("invalid jwt format".to_string());
    }

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| e.to_string())?;
    let claims: JwtClaims =
        serde_json::from_slice(&payload_bytes).map_err(|e| e.to_string())?;

    let now_secs = now_ms() / 1000;
    if claims.exp <= now_secs {
        return Err("token expired".to_string());
    }
    if claims.iss != issuer {
        return Err("invalid issuer".to_string());
    }
    if claims.aud != audience {
        return Err("invalid audience".to_string());
    }

    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|e| e.to_string())?;
    let sig = Signature::try_from(sig_bytes.as_slice()).map_err(|e| e.to_string())?;

    for pem in key_pems.iter().filter(|p| !p.is_empty()) {
        if let Ok(private_key) = RsaPrivateKey::from_pkcs8_pem(pem) {
            let verifying_key = VerifyingKey::<Sha256>::new(private_key.to_public_key());
            if verifying_key
                .verify(signing_input.as_bytes(), &sig)
                .is_ok()
            {
                return Ok(claims);
            }
        }
    }

    Err("signature verification failed".to_string())
}

/// Verify a Google ID token JWT using an RSA public key (n, e in base64url).
pub fn verify_google_id_token(
    token: &str,
    n_b64: &str,
    e_b64: &str,
    expected_aud: &str,
    expected_nonce: &str,
) -> Result<GoogleIdClaims, String> {
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err("invalid id_token format".to_string());
    }

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| e.to_string())?;
    let claims: GoogleIdClaims =
        serde_json::from_slice(&payload_bytes).map_err(|e| e.to_string())?;

    let now_secs = now_ms() / 1000;
    if claims.exp <= now_secs {
        return Err("id_token expired".to_string());
    }
    if claims.iss != "https://accounts.google.com" && claims.iss != "accounts.google.com" {
        return Err("invalid iss".to_string());
    }
    if claims.aud != expected_aud {
        return Err("invalid aud".to_string());
    }
    if claims.nonce.as_deref() != Some(expected_nonce) {
        return Err("nonce mismatch".to_string());
    }

    // Reconstruct public key from JWK n/e components
    let n_bytes = URL_SAFE_NO_PAD.decode(n_b64).map_err(|e| e.to_string())?;
    let e_bytes = URL_SAFE_NO_PAD.decode(e_b64).map_err(|e| e.to_string())?;
    let n = rsa::BigUint::from_bytes_be(&n_bytes);
    let e = rsa::BigUint::from_bytes_be(&e_bytes);
    let public_key = rsa::RsaPublicKey::new(n, e).map_err(|e| e.to_string())?;

    let verifying_key = VerifyingKey::<Sha256>::new(public_key);
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|e| e.to_string())?;
    let sig = Signature::try_from(sig_bytes.as_slice()).map_err(|e| e.to_string())?;

    verifying_key
        .verify(signing_input.as_bytes(), &sig)
        .map_err(|e| e.to_string())?;

    Ok(claims)
}

/// Decode a JWT header without verification to extract `kid`.
pub fn decode_kid(token: &str) -> Option<String> {
    let header_b64 = token.split('.').next()?;
    let bytes = URL_SAFE_NO_PAD.decode(header_b64).ok()?;
    let header: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    header.get("kid")?.as_str().map(|s| s.to_string())
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleIdClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub exp: i64,
    pub nonce: Option<String>,
}
