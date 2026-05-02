use std::fmt;

pub struct IssuedSession {
    pub jwt: String,
    pub ttl_secs: u64,
}

#[derive(Debug)]
pub enum SessionIssuerError {
    Internal(String),
}

impl fmt::Display for SessionIssuerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

pub trait SessionIssuer {
    fn issue(&self, account_id: &str, audience: &str) -> Result<IssuedSession, SessionIssuerError>;
}
