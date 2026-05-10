use serde::{Deserialize, Serialize};
use std::{fmt, future::Future, pin::Pin};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenValue {
    pub account_id: String,
    pub account_version: u64,
    pub client_id: String,
    pub token_audience: String,
    pub expires_at: i64,
    pub created_at: i64,
}

#[derive(Debug)]
pub enum RefreshTokenStoreError {
    Backend(String),
}

impl fmt::Display for RefreshTokenStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(msg) => write!(f, "refresh token store error: {msg}"),
        }
    }
}

pub type RefreshTokenStoreFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, RefreshTokenStoreError>> + 'a>>;

pub trait RefreshTokenStore {
    fn save<'a>(
        &'a self,
        token: &'a str,
        value: &'a RefreshTokenValue,
        ttl_secs: u64,
    ) -> RefreshTokenStoreFuture<'a, ()>;

    fn load<'a>(&'a self, token: &'a str)
        -> RefreshTokenStoreFuture<'a, Option<RefreshTokenValue>>;

    fn delete<'a>(&'a self, token: &'a str) -> RefreshTokenStoreFuture<'a, ()>;
}
