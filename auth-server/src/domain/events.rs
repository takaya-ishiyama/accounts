use serde::{Deserialize, Serialize};

use crate::domain::account::identity::{Identity, IdentityProvider};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(tag = "type")]
pub enum EventPayload {
    AccountRegistered { primary_email: String, identity: Identity },
    GoogleIdentityLinked { identity: Identity },
    IdentityUnlinked { provider: IdentityProvider, subject: String },
    EmailVerified { provider: IdentityProvider, subject: String },
    PrimaryEmailChanged { new_email: String },
    AccountSuspended,
    AccountReactivated,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct EventEnvelope {
    pub event_id: String,
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub schema_version: u32,
    pub version: u64,
    pub occurred_at: i64,
    pub command_id: Option<String>,
    pub payload: EventPayload,
}
