use worker::Env;

use crate::{
    application::auth::{
        session,
        session_issuer::{IssuedSession, SessionIssuer, SessionIssuerError},
    },
    domain::{clock::Clock, id_generator::IdGenerator},
    infrastructure::env::{secret, var},
};

struct SessionConfig {
    private_key: String,
    issuer: String,
    ttl_secs: u64,
}

pub struct EnvSessionIssuer<'a> {
    config: SessionConfig,
    clock: &'a dyn Clock,
    ids: &'a dyn IdGenerator,
}

impl<'a> EnvSessionIssuer<'a> {
    pub fn load(env: &Env, clock: &'a dyn Clock, ids: &'a dyn IdGenerator) -> worker::Result<Self> {
        let ttl_secs = env
            .var("SESSION_TTL_SECS")
            .ok()
            .and_then(|v| v.to_string().parse().ok())
            .unwrap_or(3600);
        Ok(Self {
            config: SessionConfig {
                private_key: secret(env, "JWT_PRIVATE_KEY")?,
                issuer: var(env, "JWT_ISSUER")?,
                ttl_secs,
            },
            clock,
            ids,
        })
    }
}

impl SessionIssuer for EnvSessionIssuer<'_> {
    fn issue(
        &self,
        account_id: &str,
        account_version: u64,
        audience: &str,
    ) -> Result<IssuedSession, SessionIssuerError> {
        let now_secs = self.clock.now_ms() / 1000;
        let jti = self.ids.new_id();
        let jwt = session::issue_jwt(
            &self.config.private_key,
            "v1",
            account_id,
            &self.config.issuer,
            audience,
            account_version,
            self.config.ttl_secs,
            now_secs,
            jti,
        )
        .map_err(SessionIssuerError::Internal)?;
        Ok(IssuedSession {
            jwt,
            ttl_secs: self.config.ttl_secs,
        })
    }
}
