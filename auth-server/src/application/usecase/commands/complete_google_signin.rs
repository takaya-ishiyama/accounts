use crate::{
    application::{
        account::read_repository::AccountReadRepository,
        auth::{
            app_authorization_code::{AppAuthorizationCodeStore, AppAuthorizationCodeValue},
            oidc_state::OidcStateStore,
            session_issuer::SessionIssuer,
        },
        http::error::{bad_request, AppError},
        usecase::commands::{bus::CommandBus, dto::CommandDto},
    },
    domain::{
        auth::{
            client::AppClients,
            provider::{CallbackCtx, IdentityProvider},
            return_to,
            token::TokenGenerator,
        },
        clock::Clock,
        id_generator::IdGenerator,
    },
};

pub struct CompletedGoogleSignin {
    pub redirect_uri: String,
    pub jwt: String,
    pub ttl_secs: u64,
}

pub struct CompleteGoogleSigninConfig {
    pub app_code_ttl_secs: u64,
}

pub async fn complete_google_signin(
    provider: &dyn IdentityProvider,
    oidc_state_store: &dyn OidcStateStore,
    app_codes: &dyn AppAuthorizationCodeStore,
    accounts: &dyn AccountReadRepository,
    commands: &dyn CommandBus,
    sessions: &dyn SessionIssuer,
    clock: &dyn Clock,
    ids: &dyn IdGenerator,
    tokens: &dyn TokenGenerator,
    app_clients: &AppClients,
    config: &CompleteGoogleSigninConfig,
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

    let audience = if !oidc_state.token_audience.is_empty() {
        oidc_state.token_audience.as_str()
    } else if oidc_state.audience.is_empty() {
        default_audience
    } else {
        &oidc_state.audience
    };
    let issued = sessions.issue(&response.account_id, response.account_version, audience)?;
    let return_to = return_to::sanitize(&oidc_state.return_to).into_owned();
    let token_audience = audience.to_string();
    let client_id = oidc_state.client_id;
    let client = app_clients
        .get(&client_id)
        .map_err(|_| bad_request("invalid_client"))?;

    let app_code = tokens.generate(32);
    let now_secs = clock.now_ms() / 1000;
    app_codes
        .save(
            &app_code,
            &AppAuthorizationCodeValue {
                account_id: response.account_id,
                account_version: response.account_version,
                client_id,
                token_audience,
                return_to: return_to.clone(),
                expires_at: now_secs + config.app_code_ttl_secs as i64,
            },
            config.app_code_ttl_secs,
        )
        .await?;

    Ok(CompletedGoogleSignin {
        redirect_uri: redirect_uri_with_code(
            &client.redirect_uri,
            &app_code,
            &return_to,
            &oidc_state.app_state,
        ),
        jwt: issued.jwt,
        ttl_secs: issued.ttl_secs,
    })
}

fn redirect_uri_with_code(
    redirect_uri: &str,
    code: &str,
    return_to: &str,
    app_state: &str,
) -> String {
    let separator = if redirect_uri.contains('?') { '&' } else { '?' };
    let mut uri = format!(
        "{redirect_uri}{separator}code={code}&return_to={}",
        percent_encode(return_to)
    );
    if !app_state.is_empty() {
        uri.push_str("&state=");
        uri.push_str(&percent_encode(app_state));
    }
    uri
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
