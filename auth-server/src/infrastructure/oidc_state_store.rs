use worker::kv::KvStore;

use crate::{
    application::auth::oidc_state::{
        OidcStateStore, OidcStateStoreError, OidcStateStoreFuture, OidcStateValue,
    },
    infrastructure::kv as kv_helper,
};

const TTL_SECS: u64 = 600; // 10 minutes

pub struct KvOidcStateStore<'a> {
    kv: &'a KvStore,
}

impl<'a> KvOidcStateStore<'a> {
    pub fn new(kv: &'a KvStore) -> Self {
        Self { kv }
    }
}

impl OidcStateStore for KvOidcStateStore<'_> {
    fn save<'a>(
        &'a self,
        state_key: &'a str,
        value: &'a OidcStateValue,
    ) -> OidcStateStoreFuture<'a, ()> {
        Box::pin(async move {
            kv_helper::put_json(self.kv, &oidc_kv_key(state_key), value, TTL_SECS)
                .await
                .map_err(to_store_error)
        })
    }

    fn load_and_delete<'a>(
        &'a self,
        state_key: &'a str,
    ) -> OidcStateStoreFuture<'a, Option<OidcStateValue>> {
        Box::pin(async move {
            let key = oidc_kv_key(state_key);
            let value = kv_helper::get_json::<OidcStateValue>(self.kv, &key)
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

fn oidc_kv_key(state: &str) -> String {
    format!("oidc:state:{state}")
}

fn to_store_error(error: worker::Error) -> OidcStateStoreError {
    OidcStateStoreError::Backend(error.to_string())
}
