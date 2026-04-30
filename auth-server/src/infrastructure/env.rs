use worker::{Env, Result};

pub fn var(env: &Env, key: &str) -> Result<String> {
    env.var(key).map(|v| v.to_string())
}

pub fn secret(env: &Env, key: &str) -> Result<String> {
    env.secret(key).map(|v| v.to_string())
}

pub struct Environment {
    pub db: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub oauth_redirect_uri: String,
}

impl Environment {
    pub fn load(env: &Env) -> Result<Self> {
        Ok(Self {
            db: var(env, "DB")?,
            google_client_id: secret(env, "GOOGLE_CLIENT_ID")?,
            google_client_secret: secret(env, "GOOGLE_CLIENT_SECRET")?,
            oauth_redirect_uri: var(env, "OAUTH_REDIRECT_URI")?,
        })
    }
}
