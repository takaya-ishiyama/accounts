#[derive(Debug, Clone)]
pub struct VerifiedIdentity {
    pub provider: String,
    pub subject: String,
    pub email: String,
    pub email_verified: bool,
}
