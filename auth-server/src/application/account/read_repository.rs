use std::{fmt, future::Future, pin::Pin};

use serde::Serialize;

#[derive(Debug)]
pub enum AccountReadError {
    Backend(String),
}

impl fmt::Display for AccountReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(msg) => write!(f, "account read backend error: {msg}"),
        }
    }
}

pub type AccountReadFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, AccountReadError>> + 'a>>;

#[derive(Debug, Clone, Serialize)]
pub struct AccountView {
    pub id: String,
    pub status: String,
    pub primary_email: String,
    pub identities: Vec<IdentityView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdentityView {
    pub provider: String,
    pub subject: String,
    pub email: String,
    pub email_verified: bool,
}

pub trait AccountReadRepository {
    fn find_account_id<'a>(
        &'a self,
        provider: &'a str,
        subject: &'a str,
    ) -> AccountReadFuture<'a, Option<String>>;

    fn get_account<'a>(&'a self, account_id: &'a str)
        -> AccountReadFuture<'a, Option<AccountView>>;
}
