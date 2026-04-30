use serde::{Deserialize, Serialize};
use worker::{kv::KvStore, Result};

use crate::infrastructure::kv as kv_helper;

const TTL_SECS: u64 = 600; // 10 minutes

#[derive(Serialize, Deserialize, Clone)]
pub struct OidcStateValue {
    pub code_verifier: String,
    pub nonce: String,
    pub return_to: String,
}

pub async fn save(kv: &KvStore, state_key: &str, value: &OidcStateValue) -> Result<()> {
    kv_helper::put_json(kv, &oidc_kv_key(state_key), value, TTL_SECS).await
}

pub async fn load_and_delete(kv: &KvStore, state_key: &str) -> Result<Option<OidcStateValue>> {
    let key = oidc_kv_key(state_key);
    let value = kv_helper::get_json::<OidcStateValue>(kv, &key).await?;
    if value.is_some() {
        kv_helper::delete(kv, &key).await?;
    }
    Ok(value)
}

fn oidc_kv_key(state: &str) -> String {
    format!("oidc:state:{state}")
}
