use serde::{Deserialize, Serialize};
use std::{fmt, future::Future, pin::Pin};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppAuthorizationCodeValue {
    pub account_id: String,
    pub account_version: u64,
    pub client_id: String,
    pub token_audience: String,
    pub return_to: String,
    pub expires_at: i64,
}

#[derive(Debug)]
pub enum AppAuthorizationCodeStoreError {
    Backend(String),
}

impl fmt::Display for AppAuthorizationCodeStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(msg) => write!(f, "app authorization code store error: {msg}"),
        }
    }
}

pub type AppAuthorizationCodeStoreFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, AppAuthorizationCodeStoreError>> + 'a>>;

pub trait AppAuthorizationCodeStore {
    fn save<'a>(
        &'a self,
        code: &'a str,
        value: &'a AppAuthorizationCodeValue,
        ttl_secs: u64,
    ) -> AppAuthorizationCodeStoreFuture<'a, ()>;

    fn load_and_delete<'a>(
        &'a self,
        code: &'a str,
    ) -> AppAuthorizationCodeStoreFuture<'a, Option<AppAuthorizationCodeValue>>;
}
