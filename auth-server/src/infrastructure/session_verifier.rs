use worker::Env;

use crate::{
    application::auth::{
        session,
        session_verifier::{SessionVerifier, SessionVerifierError, VerifiedSession},
    },
    domain::{auth::audience, clock::Clock},
    infrastructure::env::{secret, var},
};

struct SessionVerifierConfig {
    current_private_key: String,
    previous_private_key: Option<String>,
    issuer: String,
    audiences: Vec<String>,
}

pub struct EnvSessionVerifier<'a> {
    config: SessionVerifierConfig,
    clock: &'a dyn Clock,
}

impl<'a> EnvSessionVerifier<'a> {
    pub fn load(env: &Env, clock: &'a dyn Clock) -> worker::Result<Self> {
        let default_audience = var(env, "JWT_DEFAULT_AUDIENCE")?;
        let allowed_raw = env
            .var("ALLOWED_AUDIENCES")
            .ok()
            .map(|v| v.to_string())
            .unwrap_or_else(|| default_audience.clone());
        let audiences = audience::parse(&allowed_raw)
            .map_err(|e| worker::Error::RustError(e.to_string()))?
            .as_slice()
            .to_vec();

        Ok(Self {
            config: SessionVerifierConfig {
                current_private_key: secret(env, "JWT_PRIVATE_KEY")?,
                previous_private_key: secret(env, "JWT_PREVIOUS_PRIVATE_KEY").ok(),
                issuer: var(env, "JWT_ISSUER")?,
                audiences,
            },
            clock,
        })
    }
}

impl SessionVerifier for EnvSessionVerifier<'_> {
    fn verify(&self, token: &str) -> Result<VerifiedSession, SessionVerifierError> {
        let mut keys = vec![self.config.current_private_key.as_str()];
        if let Some(previous) = self.config.previous_private_key.as_deref() {
            keys.push(previous);
        }

        let now_secs = self.clock.now_ms() / 1000;
        let mut last_error = None;
        for audience in &self.config.audiences {
            match session::verify_session(token, &keys, &self.config.issuer, audience, now_secs) {
                Ok(claims) => {
                    return Ok(VerifiedSession {
                        account_id: claims.sub,
                        account_version: claims.account_version,
                    });
                }
                Err(error) => last_error = Some(error),
            }
        }

        Err(SessionVerifierError::Invalid(
            last_error.unwrap_or_else(|| "invalid audience".to_string()),
        ))
    }
}
