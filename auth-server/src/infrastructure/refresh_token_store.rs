use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};
use worker::kv::KvStore;

use crate::{
    application::auth::refresh_token::{
        RefreshTokenStore, RefreshTokenStoreError, RefreshTokenStoreFuture, RefreshTokenValue,
    },
    infrastructure::kv as kv_helper,
};

pub struct KvRefreshTokenStore<'a> {
    kv: &'a KvStore,
}

impl<'a> KvRefreshTokenStore<'a> {
    pub fn new(kv: &'a KvStore) -> Self {
        Self { kv }
    }
}

impl RefreshTokenStore for KvRefreshTokenStore<'_> {
    fn save<'a>(
        &'a self,
        token: &'a str,
        value: &'a RefreshTokenValue,
        ttl_secs: u64,
    ) -> RefreshTokenStoreFuture<'a, ()> {
        Box::pin(async move {
            let token_hash = hash_refresh_token(token);
            kv_helper::put_json(self.kv, &refresh_kv_key(&token_hash), value, ttl_secs)
                .await
                .map_err(to_store_error)
        })
    }

    fn load<'a>(
        &'a self,
        token: &'a str,
    ) -> RefreshTokenStoreFuture<'a, Option<RefreshTokenValue>> {
        Box::pin(async move {
            let token_hash = hash_refresh_token(token);
            kv_helper::get_json::<RefreshTokenValue>(self.kv, &refresh_kv_key(&token_hash))
                .await
                .map_err(to_store_error)
        })
    }

    fn delete<'a>(&'a self, token: &'a str) -> RefreshTokenStoreFuture<'a, ()> {
        Box::pin(async move {
            let token_hash = hash_refresh_token(token);
            kv_helper::delete(self.kv, &refresh_kv_key(&token_hash))
                .await
                .map_err(to_store_error)
        })
    }
}

fn hash_refresh_token(token: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
}

fn refresh_kv_key(token_hash: &str) -> String {
    format!("oauth:refresh:{token_hash}")
}

fn to_store_error(error: worker::Error) -> RefreshTokenStoreError {
    RefreshTokenStoreError::Backend(error.to_string())
}
