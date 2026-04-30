use worker::{Env, Result};

pub fn var(env: &Env, key: &str) -> Result<String> {
    env.var(key).map(|v| v.to_string())
}

pub fn secret(env: &Env, key: &str) -> Result<String> {
    env.secret(key).map(|v| v.to_string())
}
