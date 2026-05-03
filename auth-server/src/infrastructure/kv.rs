use serde::{de::DeserializeOwned, Serialize};
use worker::{kv::KvStore, Result};

pub async fn put_json<T: Serialize>(
    kv: &KvStore,
    key: &str,
    value: &T,
    ttl_secs: u64,
) -> Result<()> {
    let json = serde_json::to_string(value).map_err(|e| worker::Error::RustError(e.to_string()))?;
    kv.put(key, json)?
        .expiration_ttl(ttl_secs)
        .execute()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))
}

pub async fn get_json<T: DeserializeOwned>(kv: &KvStore, key: &str) -> Result<Option<T>> {
    match kv.get(key).text().await? {
        Some(s) => {
            let v =
                serde_json::from_str(&s).map_err(|e| worker::Error::RustError(e.to_string()))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

pub async fn delete(kv: &KvStore, key: &str) -> Result<()> {
    kv.delete(key)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))
}
