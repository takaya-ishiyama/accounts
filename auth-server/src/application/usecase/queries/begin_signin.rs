use crate::{
    application::{
        auth::oidc_state::{OidcStateStore, OidcStateValue},
        http::error::{bad_request, AppError},
    },
    domain::auth::{
        client::AppClients,
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
    client_id: &str,
    app_state: Option<&str>,
    app_clients: &AppClients,
) -> Result<SignInRedirect, AppError> {
    let client = app_clients
        .get(client_id)
        .map_err(|_| bad_request("invalid_client"))?;
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
                audience: client.token_audience.clone(),
                client_id: client.client_id.clone(),
                token_audience: client.token_audience.clone(),
                app_state: app_state.unwrap_or_default().to_string(),
            },
        )
        .await?;

    Ok(SignInRedirect {
        url: resp.authorization_url,
    })
}
