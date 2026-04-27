#[derive(Debug, Clone)]
enum IdentityProvider {
    Google,
    Github,
    Apple,
}

#[derive(Debug, Clone)]
pub struct Identity {
    provider: IdentityProvider,
    subject: String,
    email: String,
    email_verified: bool,
    linked_at: String,
}
