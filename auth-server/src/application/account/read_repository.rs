use std::{fmt, future::Future, pin::Pin};

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

pub type AccountReadFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, AccountReadError>> + 'a>>;

pub trait AccountReadRepository {
    fn find_account_id<'a>(
        &'a self,
        provider: &'a str,
        subject: &'a str,
    ) -> AccountReadFuture<'a, Option<String>>;
}
