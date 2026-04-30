use serde::{Deserialize, Serialize};

use crate::domain::account::{account_detail::AccountDetail, identity::{Identity, IdentityProvider}};
use crate::domain::events::{EventEnvelope, EventPayload};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountStatus {
    Active,
    Pending,
    Suspended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub(crate) id: String,
    pub(crate) status: AccountStatus,
    pub(crate) account_detail: Option<AccountDetail>,
    pub(crate) identities: Vec<Identity>,
    pub(crate) version: u64,
}

impl Account {
    pub(crate) fn empty() -> Self {
        Self {
            id: String::new(),
            status: AccountStatus::Pending,
            account_detail: None,
            identities: Vec::new(),
            version: 0,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn status(&self) -> &AccountStatus {
        &self.status
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn identities(&self) -> &[Identity] {
        &self.identities
    }

    pub fn account_detail(&self) -> Option<&AccountDetail> {
        self.account_detail.as_ref()
    }

    pub fn find_identity(&self, provider: &IdentityProvider, subject: &str) -> Option<&Identity> {
        self.identities.iter().find(|i| i.provider() == provider && i.subject() == subject)
    }

    pub(crate) fn apply(mut self, ev: &EventEnvelope) -> Self {
        match &ev.payload {
            EventPayload::AccountRegistered { primary_email, identity } => {
                self.id = ev.aggregate_id.clone();
                self.status = AccountStatus::Active;
                self.account_detail = Some(AccountDetail {
                    id: ev.aggregate_id.clone(),
                    name: None,
                    primary_email: primary_email.clone(),
                });
                self.identities.push(identity.clone());
            }
            EventPayload::GoogleIdentityLinked { identity } => {
                self.identities.push(identity.clone());
            }
            EventPayload::IdentityUnlinked { provider, subject } => {
                self.identities.retain(|i| !(i.provider() == provider && i.subject() == subject));
            }
            EventPayload::EmailVerified { provider, subject } => {
                if let Some(identity) = self.identities.iter_mut().find(|i| i.provider() == provider && i.subject() == subject) {
                    identity.email_verified = true;
                }
            }
            EventPayload::PrimaryEmailChanged { new_email } => {
                if let Some(detail) = &mut self.account_detail {
                    detail.primary_email = new_email.clone();
                }
            }
            EventPayload::AccountSuspended => {
                self.status = AccountStatus::Suspended;
            }
            EventPayload::AccountReactivated => {
                self.status = AccountStatus::Active;
            }
        }
        self.version = ev.version;
        self
    }
}
