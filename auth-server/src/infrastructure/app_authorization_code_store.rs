use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};
use worker::Env;

use crate::application::auth::app_authorization_code::{
    AppAuthorizationCodeStore, AppAuthorizationCodeStoreError, AppAuthorizationCodeStoreFuture,
    AppAuthorizationCodeValue,
};

use super::oauth_state_do_client::{load_and_delete_oauth_state, save_oauth_state};

pub struct DoAppAuthorizationCodeStore<'a> {
    pub env: &'a Env,
}

impl AppAuthorizationCodeStore for DoAppAuthorizationCodeStore<'_> {
    fn save<'a>(
        &'a self,
        code: &'a str,
        value: &'a AppAuthorizationCodeValue,
        ttl_secs: u64,
    ) -> AppAuthorizationCodeStoreFuture<'a, ()> {
        Box::pin(async move {
            let value_json = serde_json::to_string(value)
                .map_err(|e| AppAuthorizationCodeStoreError::Backend(e.to_string()))?;
            save_oauth_state(self.env, &code_do_name(code), &value_json, ttl_secs)
                .await
                .map_err(|e| AppAuthorizationCodeStoreError::Backend(e.to_string()))
        })
    }

    fn load_and_delete<'a>(
        &'a self,
        code: &'a str,
    ) -> AppAuthorizationCodeStoreFuture<'a, Option<AppAuthorizationCodeValue>> {
        Box::pin(async move {
            let value_json = load_and_delete_oauth_state(self.env, &code_do_name(code))
                .await
                .map_err(|e| AppAuthorizationCodeStoreError::Backend(e.to_string()))?;
            value_json
                .map(|value_json| {
                    serde_json::from_str::<AppAuthorizationCodeValue>(&value_json)
                        .map_err(|e| AppAuthorizationCodeStoreError::Backend(e.to_string()))
                })
                .transpose()
        })
    }
}

fn code_do_name(code: &str) -> String {
    format!(
        "code:{}",
        URL_SAFE_NO_PAD.encode(Sha256::digest(code.as_bytes()))
    )
}
