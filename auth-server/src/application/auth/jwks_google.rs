use serde::{Deserialize, Serialize};
use worker::{kv::KvStore, Fetch, Method, Request, Result};

use crate::infrastructure::kv as kv_helper;

const CACHE_KEY: &str = "google:jwks";
const CACHE_TTL: u64 = 3600; // 1 hour

#[derive(Serialize, Deserialize, Clone)]
pub struct GoogleJwks {
    pub keys: Vec<GoogleJwk>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GoogleJwk {
    pub kty: String,
    pub kid: String,
    pub n: String,
    pub e: String,
    #[serde(default)]
    pub alg: String,
}

pub async fn get_jwks(kv: &KvStore) -> Result<GoogleJwks> {
    // Try cache first
    if let Some(cached) = kv_helper::get_json::<GoogleJwks>(kv, CACHE_KEY).await? {
        return Ok(cached);
    }

    // Fetch from Google
    let req = Request::new("https://www.googleapis.com/oauth2/v3/certs", Method::Get)?;
    let mut resp = Fetch::Request(req).send().await?;
    let jwks: GoogleJwks = resp
        .json()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;

    // Cache it
    let _ = kv_helper::put_json(kv, CACHE_KEY, &jwks, CACHE_TTL).await;

    Ok(jwks)
}

pub fn find_key<'a>(jwks: &'a GoogleJwks, kid: &str) -> Option<&'a GoogleJwk> {
    jwks.keys.iter().find(|k| k.kid == kid)
}
