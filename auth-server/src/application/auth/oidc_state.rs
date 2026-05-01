use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};

#[derive(Serialize, Deserialize, Clone)]
pub struct OidcStateValue {
    pub code_verifier: String,
    pub nonce: String,
    pub return_to: String,
}

pub type OidcStateStoreResult<T> = Result<T, OidcStateStoreError>;
pub type OidcStateStoreFuture<'a, T> = Pin<Box<dyn Future<Output = OidcStateStoreResult<T>> + 'a>>;

#[derive(Debug, thiserror::Error)]
pub enum OidcStateStoreError {
    #[error("oidc state store error: {0}")]
    Backend(String),
}

pub trait OidcStateStore {
    fn save<'a>(
        &'a self,
        state_key: &'a str,
        value: &'a OidcStateValue,
    ) -> OidcStateStoreFuture<'a, ()>;

    fn load_and_delete<'a>(
        &'a self,
        state_key: &'a str,
    ) -> OidcStateStoreFuture<'a, Option<OidcStateValue>>;
}
