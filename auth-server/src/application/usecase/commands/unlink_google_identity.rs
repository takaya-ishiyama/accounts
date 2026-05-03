use crate::{
    application::{
        auth::session_verifier::SessionVerifier,
        http::error::AppError,
        usecase::commands::{
            bus::CommandBus,
            dto::{CommandDto, CommandResponse},
        },
    },
    domain::{account::identity::IdentityProvider, clock::Clock, id_generator::IdGenerator},
};

pub async fn unlink_google_identity(
    sessions: &dyn SessionVerifier,
    commands: &dyn CommandBus,
    clock: &dyn Clock,
    ids: &dyn IdGenerator,
    token: &str,
    subject: &str,
) -> Result<CommandResponse, AppError> {
    let session = sessions.verify(token)?;
    let cmd = CommandDto::UnlinkIdentity {
        idempotency_key: ids.new_id(),
        provider: IdentityProvider::Google,
        subject: subject.to_string(),
        occurred_at: clock.now_ms(),
    };
    Ok(commands.dispatch(&session.account_id, &cmd).await?)
}
