use serde::{Deserialize, Serialize};

use crate::domain::account::identity::IdentityProvider;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum CommandDto {
    SignInWithGoogle {
        idempotency_key: String,
        subject: String,
        email: String,
        email_verified: bool,
        occurred_at: i64,
    },
    LinkGoogleIdentity {
        idempotency_key: String,
        subject: String,
        email: String,
        email_verified: bool,
        occurred_at: i64,
    },
    UnlinkIdentity {
        idempotency_key: String,
        provider: IdentityProvider,
        subject: String,
        occurred_at: i64,
    },
    SuspendAccount {
        idempotency_key: String,
        occurred_at: i64,
    },
    ReactivateAccount {
        idempotency_key: String,
        occurred_at: i64,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandResponse {
    pub account_id: String,
    pub events_emitted: usize,
}

impl CommandDto {
    pub fn idempotency_key(&self) -> &str {
        match self {
            Self::SignInWithGoogle { idempotency_key, .. } => idempotency_key,
            Self::LinkGoogleIdentity { idempotency_key, .. } => idempotency_key,
            Self::UnlinkIdentity { idempotency_key, .. } => idempotency_key,
            Self::SuspendAccount { idempotency_key, .. } => idempotency_key,
            Self::ReactivateAccount { idempotency_key, .. } => idempotency_key,
        }
    }
}
