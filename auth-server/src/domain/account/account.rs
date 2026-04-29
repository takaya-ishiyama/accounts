use crate::domain::account::{account_detail::AccountDetail, identity::Identity};

#[derive(Debug, Clone)]
enum AccountStatus {
    Active,
    Pending,
    Suspended,
}

#[derive(Debug, Clone)]
pub struct Account {
    id: String,
    status: AccountStatus,
    account_detail: Option<AccountDetail>,
    identities: Vec<Identity>,
    version: u32,
}
