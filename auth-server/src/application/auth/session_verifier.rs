use std::fmt;

#[derive(Debug, Clone)]
pub struct VerifiedSession {
    pub account_id: String,
    pub account_version: u64,
}

#[derive(Debug)]
pub enum SessionVerifierError {
    Invalid(String),
    Internal(String),
}

impl fmt::Display for SessionVerifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(msg) => write!(f, "{msg}"),
            Self::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

pub trait SessionVerifier {
    fn verify(&self, token: &str) -> Result<VerifiedSession, SessionVerifierError>;
}
