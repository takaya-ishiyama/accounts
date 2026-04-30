use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentityProvider {
    Google,
    Github,
    Apple,
}

impl IdentityProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::Github => "Github",
            Self::Apple => "Apple",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Google" => Some(Self::Google),
            "Github" => Some(Self::Github),
            "Apple" => Some(Self::Apple),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub(crate) provider: IdentityProvider,
    pub(crate) subject: String,
    pub(crate) email: String,
    pub(crate) email_verified: bool,
    pub(crate) linked_at: i64,
}

impl Identity {
    pub fn new_google(subject: String, email: String, email_verified: bool, linked_at: i64) -> Self {
        Self {
            provider: IdentityProvider::Google,
            subject,
            email,
            email_verified,
            linked_at,
        }
    }

    pub fn provider(&self) -> &IdentityProvider {
        &self.provider
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn email_verified(&self) -> bool {
        self.email_verified
    }

    pub fn linked_at(&self) -> i64 {
        self.linked_at
    }
}
