use crate::{
    application::{
        account::read_repository::{AccountCurrentRepository, AccountReadRepository, AccountView},
        auth::{
            app_authorization_code::AppAuthorizationCodeStore,
            refresh_token::{RefreshTokenStore, RefreshTokenValue},
            session_issuer::SessionIssuer,
        },
        http::error::{bad_request, AppError},
    },
    domain::{
        auth::{client::AppClients, token::TokenGenerator},
        clock::Clock,
    },
};

pub enum TokenGrant<'a> {
    AuthorizationCode { code: &'a str },
    RefreshToken { refresh_token: &'a str },
}

pub struct TokenExchangeRequest<'a> {
    pub client_id: &'a str,
    pub grant: TokenGrant<'a>,
}

pub struct TokenExchangeConfig {
    pub refresh_token_ttl_secs: u64,
}

pub struct TokenExchangeResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
}

pub async fn exchange_oauth_token(
    app_clients: &AppClients,
    app_codes: &dyn AppAuthorizationCodeStore,
    refresh_tokens: &dyn RefreshTokenStore,
    accounts: &dyn AccountReadRepository,
    current_accounts: &dyn AccountCurrentRepository,
    sessions: &dyn SessionIssuer,
    tokens: &dyn TokenGenerator,
    clock: &dyn Clock,
    config: &TokenExchangeConfig,
    request: TokenExchangeRequest<'_>,
) -> Result<TokenExchangeResponse, AppError> {
    app_clients
        .get(request.client_id)
        .map_err(|_| bad_request("invalid_client"))?;

    match request.grant {
        TokenGrant::AuthorizationCode { code } => {
            let code_value = app_codes
                .load_and_delete(code)
                .await?
                .ok_or_else(|| bad_request("invalid_grant"))?;

            if code_value.client_id != request.client_id || code_value.expires_at <= now_secs(clock)
            {
                return Err(bad_request("invalid_grant"));
            }

            issue_token_pair(
                refresh_tokens,
                sessions,
                tokens,
                clock,
                config.refresh_token_ttl_secs,
                &code_value.account_id,
                code_value.account_version,
                &code_value.client_id,
                &code_value.token_audience,
            )
            .await
        }
        TokenGrant::RefreshToken { refresh_token } => {
            let old_value = refresh_tokens
                .load(refresh_token)
                .await?
                .ok_or_else(|| bad_request("invalid_grant"))?;

            if old_value.client_id != request.client_id || old_value.expires_at <= now_secs(clock) {
                refresh_tokens.delete(refresh_token).await?;
                return Err(bad_request("invalid_grant"));
            }

            let account =
                current_account(accounts, current_accounts, &old_value.account_id).await?;
            let account_version = match account {
                Some(account) if account.status == "Active" => account.version,
                _ => {
                    refresh_tokens.delete(refresh_token).await?;
                    return Err(bad_request("invalid_grant"));
                }
            };

            refresh_tokens.delete(refresh_token).await?;
            issue_token_pair(
                refresh_tokens,
                sessions,
                tokens,
                clock,
                config.refresh_token_ttl_secs,
                &old_value.account_id,
                account_version,
                &old_value.client_id,
                &old_value.token_audience,
            )
            .await
        }
    }
}

async fn current_account(
    accounts: &dyn AccountReadRepository,
    current_accounts: &dyn AccountCurrentRepository,
    account_id: &str,
) -> Result<Option<AccountView>, AppError> {
    if let Some(account) = accounts.get_account(account_id).await? {
        return Ok(Some(account));
    }

    current_accounts
        .get_current_account(account_id)
        .await
        .map_err(Into::into)
}

async fn issue_token_pair(
    refresh_tokens: &dyn RefreshTokenStore,
    sessions: &dyn SessionIssuer,
    tokens: &dyn TokenGenerator,
    clock: &dyn Clock,
    refresh_token_ttl_secs: u64,
    account_id: &str,
    account_version: u64,
    client_id: &str,
    token_audience: &str,
) -> Result<TokenExchangeResponse, AppError> {
    let issued = sessions.issue(account_id, account_version, token_audience)?;
    let refresh_token = tokens.generate(32);
    let now = now_secs(clock);

    refresh_tokens
        .save(
            &refresh_token,
            &RefreshTokenValue {
                account_id: account_id.to_string(),
                account_version,
                client_id: client_id.to_string(),
                token_audience: token_audience.to_string(),
                expires_at: now + refresh_token_ttl_secs as i64,
                created_at: now,
            },
            refresh_token_ttl_secs,
        )
        .await?;

    Ok(TokenExchangeResponse {
        access_token: issued.jwt,
        refresh_token,
        token_type: "Bearer",
        expires_in: issued.ttl_secs,
    })
}

fn now_secs(clock: &dyn Clock) -> i64 {
    clock.now_ms() / 1000
}
