use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
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

pub trait IdentityProvider {
    fn id(&self) -> &'static str;
    fn start(&self, ctx: &StartCtx<'_>) -> Result<StartResponse, ProviderError>;
}
