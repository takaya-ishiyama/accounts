use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};
use worker::Env;

use crate::application::auth::refresh_token::{
    RefreshTokenStore, RefreshTokenStoreError, RefreshTokenStoreFuture, RefreshTokenValue,
};

use super::oauth_state_do_client::{
    delete_oauth_state, load_and_delete_oauth_state, load_oauth_state, save_oauth_state,
};

pub struct DoRefreshTokenStore<'a> {
    pub env: &'a Env,
}

impl RefreshTokenStore for DoRefreshTokenStore<'_> {
    fn save<'a>(
        &'a self,
        token: &'a str,
        value: &'a RefreshTokenValue,
        ttl_secs: u64,
    ) -> RefreshTokenStoreFuture<'a, ()> {
        Box::pin(async move {
            let value_json = serde_json::to_string(value)
                .map_err(|e| RefreshTokenStoreError::Backend(e.to_string()))?;
            save_oauth_state(self.env, &refresh_do_name(token), &value_json, ttl_secs)
                .await
                .map_err(|e| RefreshTokenStoreError::Backend(e.to_string()))
        })
    }

    fn load<'a>(
        &'a self,
        token: &'a str,
    ) -> RefreshTokenStoreFuture<'a, Option<RefreshTokenValue>> {
        Box::pin(async move {
            let value_json = load_oauth_state(self.env, &refresh_do_name(token))
                .await
                .map_err(|e| RefreshTokenStoreError::Backend(e.to_string()))?;
            value_json
                .map(|value_json| {
                    serde_json::from_str::<RefreshTokenValue>(&value_json)
                        .map_err(|e| RefreshTokenStoreError::Backend(e.to_string()))
                })
                .transpose()
        })
    }

    fn load_and_delete<'a>(
        &'a self,
        token: &'a str,
    ) -> RefreshTokenStoreFuture<'a, Option<RefreshTokenValue>> {
        Box::pin(async move {
            let value_json = load_and_delete_oauth_state(self.env, &refresh_do_name(token))
                .await
                .map_err(|e| RefreshTokenStoreError::Backend(e.to_string()))?;
            value_json
                .map(|value_json| {
                    serde_json::from_str::<RefreshTokenValue>(&value_json)
                        .map_err(|e| RefreshTokenStoreError::Backend(e.to_string()))
                })
                .transpose()
        })
    }

    fn delete<'a>(&'a self, token: &'a str) -> RefreshTokenStoreFuture<'a, ()> {
        Box::pin(async move {
            delete_oauth_state(self.env, &refresh_do_name(token))
                .await
                .map_err(|e| RefreshTokenStoreError::Backend(e.to_string()))
        })
    }
}

fn hash_refresh_token(token: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
}

fn refresh_do_name(token: &str) -> String {
    format!("refresh:{}", hash_refresh_token(token))
}
