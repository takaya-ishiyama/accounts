use std::{error::Error, fmt};

use crate::domain::account::{account::AccountStatus, identity::IdentityProvider};

#[derive(Debug)]
pub struct ApiError {
    pub code: u16,
    pub message: String,
    pub error: Option<Box<dyn Error>>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An Error Occurred, Please Try Again!")
    }
}

impl ApiError {
    pub fn get_error_message(&self) -> String {
        String::from(&self.message)
    }

    pub fn get_error_code(&self) -> u16 {
        self.code
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum DomainError {
    #[error("identity already linked to this account: {provider:?}/{subject}")]
    IdentityAlreadyLinked {
        provider: IdentityProvider,
        subject: String,
    },
    #[error("cannot unlink last identity from active account")]
    CannotUnlinkLastIdentity,
    #[error("invalid status transition: {from:?} -> {to:?}")]
    InvalidStatusTransition {
        from: AccountStatus,
        to: AccountStatus,
    },
    #[error("email not verified")]
    EmailNotVerified,
    #[error("identity not found: {provider:?}/{subject}")]
    IdentityNotFound {
        provider: IdentityProvider,
        subject: String,
    },
}
