use std::{future::Future, pin::Pin};
use thiserror::Error;

use crate::domain::auth::identity::VerifiedIdentity;

pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ProviderError>> + 'a>>;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("invalid grant: {0}")]
    InvalidGrant(String),
    #[error("provider error: {0}")]
    Internal(String),
}

pub struct StartCtx<'a> {
    pub return_to: &'a str,
}

pub struct StartResponse {
    pub authorization_url: String,
    pub state_key: String,
    pub code_verifier: String,
    pub nonce: String,
}

pub struct CallbackCtx<'a> {
    pub code: &'a str,
    pub code_verifier: &'a str,
    pub nonce: &'a str,
}

pub trait IdentityProvider {
    fn id(&self) -> &'static str;
    fn start(&self, ctx: &StartCtx<'_>) -> Result<StartResponse, ProviderError>;
    fn callback<'a>(&'a self, ctx: &'a CallbackCtx<'a>) -> ProviderFuture<'a, VerifiedIdentity>;
}
