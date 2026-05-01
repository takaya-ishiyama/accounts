use crate::{
    application::{
        auth::oidc_state::{OidcStateStore, OidcStateValue},
        http::error::AppError,
    },
    domain::auth::{
        provider::{IdentityProvider, StartCtx},
        return_to,
    },
};

pub struct SignInRedirect {
    pub url: String,
}

pub async fn begin_signin(
    provider: &dyn IdentityProvider,
    oidc_state_store: &dyn OidcStateStore,
    return_to_raw: &str,
) -> Result<SignInRedirect, AppError> {
    let sanitized = return_to::sanitize(return_to_raw).into_owned();

    let resp = provider.start(&StartCtx {
        return_to: &sanitized,
    })?;

    oidc_state_store
        .save(
            &resp.state_key,
            &OidcStateValue {
                code_verifier: resp.code_verifier,
                nonce: resp.nonce,
                return_to: sanitized,
            },
        )
        .await?;

    Ok(SignInRedirect {
        url: resp.authorization_url,
    })
}
