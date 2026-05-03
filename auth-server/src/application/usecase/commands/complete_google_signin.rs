use crate::{
    application::{
        account::read_repository::AccountReadRepository,
        auth::{oidc_state::OidcStateStore, session_issuer::SessionIssuer},
        http::error::{bad_request, AppError},
        usecase::commands::{bus::CommandBus, dto::CommandDto},
    },
    domain::{
        auth::provider::{CallbackCtx, IdentityProvider},
        clock::Clock,
        id_generator::IdGenerator,
    },
};

pub struct CompletedGoogleSignin {
    pub return_to: String,
    pub jwt: String,
    pub ttl_secs: u64,
}

pub async fn complete_google_signin(
    provider: &dyn IdentityProvider,
    oidc_state_store: &dyn OidcStateStore,
    accounts: &dyn AccountReadRepository,
    commands: &dyn CommandBus,
    sessions: &dyn SessionIssuer,
    clock: &dyn Clock,
    ids: &dyn IdGenerator,
    code: &str,
    state_key: &str,
    default_audience: &str,
) -> Result<CompletedGoogleSignin, AppError> {
    let oidc_state = oidc_state_store
        .load_and_delete(state_key)
        .await?
        .ok_or_else(|| bad_request("invalid_state"))?;

    let identity = provider
        .callback(&CallbackCtx {
            code,
            code_verifier: &oidc_state.code_verifier,
            nonce: &oidc_state.nonce,
        })
        .await?;

    if !identity.email_verified {
        return Err(bad_request("email_not_verified"));
    }

    let account_id = accounts
        .find_account_id(&identity.provider, &identity.subject)
        .await?
        .unwrap_or_else(|| ids.new_id());

    let cmd = CommandDto::SignInWithGoogle {
        idempotency_key: ids.new_id(),
        subject: identity.subject,
        email: identity.email,
        email_verified: identity.email_verified,
        occurred_at: clock.now_ms(),
    };
    let response = commands.dispatch(&account_id, &cmd).await?;

    let audience = if oidc_state.audience.is_empty() {
        default_audience
    } else {
        &oidc_state.audience
    };
    let issued = sessions.issue(&response.account_id, audience)?;

    // return_to was already validated in begin_signin and stored server-side in KV;
    // re-sanitizing here would incorrectly reject allowed absolute URLs from dev environments.
    Ok(CompletedGoogleSignin {
        return_to: oidc_state.return_to,
        jwt: issued.jwt,
        ttl_secs: issued.ttl_secs,
    })
}
