use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountDetail {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    pub(crate) primary_email: String,
}

impl AccountDetail {
    pub fn primary_email(&self) -> &str {
        &self.primary_email
    }
}
