use worker::{Env, Result};

use crate::domain::auth::audience::{self, AllowedAudiences};

pub fn var(env: &Env, key: &str) -> Result<String> {
    env.var(key).map(|v| v.to_string())
}

pub fn secret(env: &Env, key: &str) -> Result<String> {
    env.secret(key).map(|v| v.to_string())
}

pub struct Environment {
    pub google_client_id: String,
    pub google_client_secret: String,
    pub oauth_redirect_uri: String,
    pub default_audience: String,
    pub allowed_audiences: AllowedAudiences,
}

impl Environment {
    pub fn load(env: &Env) -> Result<Self> {
        let default_audience = var(env, "JWT_DEFAULT_AUDIENCE")?;
        let allowed_raw = env
            .var("ALLOWED_AUDIENCES")
            .ok()
            .map(|v| v.to_string())
            .unwrap_or_else(|| default_audience.clone());
        let allowed_audiences =
            audience::parse(&allowed_raw).map_err(|e| worker::Error::RustError(e.to_string()))?;

        Ok(Self {
            google_client_id: var(env, "GOOGLE_CLIENT_ID")?,
            google_client_secret: secret(env, "GOOGLE_CLIENT_SECRET")?,
            oauth_redirect_uri: var(env, "OAUTH_REDIRECT_URI")?,
            default_audience,
            allowed_audiences,
        })
    }
}
