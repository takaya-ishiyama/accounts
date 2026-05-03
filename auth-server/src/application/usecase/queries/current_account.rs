use crate::application::{
    account::read_repository::{AccountReadRepository, AccountView},
    auth::session_verifier::SessionVerifier,
    http::error::{unauthorized, AppError},
};

pub async fn current_account(
    sessions: &dyn SessionVerifier,
    accounts: &dyn AccountReadRepository,
    token: &str,
) -> Result<AccountView, AppError> {
    let session = sessions.verify(token)?;
    accounts
        .get_account(&session.account_id)
        .await?
        .ok_or_else(|| unauthorized("account_not_found"))
}
