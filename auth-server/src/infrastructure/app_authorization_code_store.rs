use worker::kv::KvStore;

use crate::{
    application::auth::app_authorization_code::{
        AppAuthorizationCodeStore, AppAuthorizationCodeStoreError, AppAuthorizationCodeStoreFuture,
        AppAuthorizationCodeValue,
    },
    infrastructure::kv as kv_helper,
};

const TTL_SECS: u64 = 300;

pub struct KvAppAuthorizationCodeStore<'a> {
    kv: &'a KvStore,
}

impl<'a> KvAppAuthorizationCodeStore<'a> {
    pub fn new(kv: &'a KvStore) -> Self {
        Self { kv }
    }
}

impl AppAuthorizationCodeStore for KvAppAuthorizationCodeStore<'_> {
    fn save<'a>(
        &'a self,
        code: &'a str,
        value: &'a AppAuthorizationCodeValue,
    ) -> AppAuthorizationCodeStoreFuture<'a, ()> {
        Box::pin(async move {
            kv_helper::put_json(self.kv, &code_kv_key(code), value, TTL_SECS)
                .await
                .map_err(to_store_error)
        })
    }

    fn load_and_delete<'a>(
        &'a self,
        code: &'a str,
    ) -> AppAuthorizationCodeStoreFuture<'a, Option<AppAuthorizationCodeValue>> {
        Box::pin(async move {
            let key = code_kv_key(code);
            let value = kv_helper::get_json::<AppAuthorizationCodeValue>(self.kv, &key)
                .await
                .map_err(to_store_error)?;
            if value.is_some() {
                kv_helper::delete(self.kv, &key)
                    .await
                    .map_err(to_store_error)?;
            }
            Ok(value)
        })
    }
}

fn code_kv_key(code: &str) -> String {
    format!("oauth:code:{code}")
}

fn to_store_error(error: worker::Error) -> AppAuthorizationCodeStoreError {
    AppAuthorizationCodeStoreError::Backend(error.to_string())
}
