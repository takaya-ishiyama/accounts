use crate::domain::account::identity::Identity;

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
    primary_email: String,
    identities: Vec<Identity>,
    version: u32,
}
