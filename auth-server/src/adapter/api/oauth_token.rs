use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::future::Future;
use worker::send::SendFuture;

use crate::{
    application::{
        http::error::{bad_request, AppError},
        usecase::commands::exchange_oauth_token::{
            exchange_oauth_token, TokenExchangeConfig, TokenExchangeRequest, TokenExchangeResponse,
            TokenGrant,
        },
    },
    infrastructure::{
        account_read_repository::{D1AccountReadRepository, DoAccountCurrentRepository},
        app_authorization_code_store::DoAppAuthorizationCodeStore,
        clock::WorkerClock,
        env::Environment,
        id_generator::UuidV7Generator,
        random::GetRandomTokenGenerator,
        refresh_token_store::DoRefreshTokenStore,
        session_issuer::EnvSessionIssuer,
    },
    AppState,
};

const DEFAULT_REFRESH_TOKEN_TTL_SECS: u64 = 60 * 60 * 24 * 30;

#[derive(Deserialize)]
pub struct TokenRequest {
    grant_type: String,
    client_id: Option<String>,
    code: Option<String>,
    refresh_token: Option<String>,
}

#[derive(Serialize)]
pub struct TokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: &'static str,
    expires_in: u64,
}

pub fn router(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<TokenRequest>,
) -> SendFuture<impl Future<Output = Result<(HeaderMap, Json<TokenResponse>), AppError>>> {
    SendFuture::new(async move {
        let env = &state.env;
        let environment = Environment::load(env)?;
        let clock = WorkerClock;
        let ids = UuidV7Generator;
        let tokens = GetRandomTokenGenerator;
        let sessions = EnvSessionIssuer::load(env, &clock, &ids)?;
        let app_codes = DoAppAuthorizationCodeStore { env };
        let refresh_tokens = DoRefreshTokenStore { env };
        let accounts = D1AccountReadRepository { env };
        let current_accounts = DoAccountCurrentRepository { env };
        let config = TokenExchangeConfig {
            refresh_token_ttl_secs: env_refresh_token_ttl_secs(env),
        };
        let authenticated_client_id =
            authenticate_client(&environment.app_clients, &headers, req.client_id.as_deref())?;

        let response = exchange_oauth_token(
            &environment.app_clients,
            &app_codes,
            &refresh_tokens,
            &accounts,
            &current_accounts,
            &sessions,
            &tokens,
            &clock,
            &config,
            TokenExchangeRequest {
                client_id: authenticated_client_id,
                grant: parse_grant(&req)?,
            },
        )
        .await?;

        Ok((token_response_headers(), Json(response.into())))
    })
}

fn token_response_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    headers
}

fn authenticate_client<'a>(
    app_clients: &'a crate::domain::auth::client::AppClients,
    headers: &HeaderMap,
    body_client_id: Option<&str>,
) -> Result<&'a str, AppError> {
    let Some((client_id, client_secret)) = basic_client_credentials(headers) else {
        return Err(bad_request("invalid_client"));
    };

    if body_client_id.is_some_and(|body_client_id| body_client_id != client_id) {
        return Err(bad_request("invalid_client"));
    }

    let client = app_clients
        .get(&client_id)
        .map_err(|_| bad_request("invalid_client"))?;
    if !constant_time_eq(client.client_secret.as_bytes(), client_secret.as_bytes()) {
        return Err(bad_request("invalid_client"));
    }

    Ok(&client.client_id)
}

fn basic_client_credentials(headers: &HeaderMap) -> Option<(String, String)> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let encoded = value.strip_prefix("Basic ")?;
    let decoded = STANDARD.decode(encoded).ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    let (client_id, client_secret) = decoded.split_once(':')?;
    if client_id.is_empty() || client_secret.is_empty() {
        return None;
    }
    Some((client_id.to_string(), client_secret.to_string()))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();
    for i in 0..max_len {
        let l = left.get(i).copied().unwrap_or(0);
        let r = right.get(i).copied().unwrap_or(0);
        diff |= (l ^ r) as usize;
    }
    diff == 0
}

fn parse_grant(req: &TokenRequest) -> Result<TokenGrant<'_>, AppError> {
    match req.grant_type.as_str() {
        "authorization_code" => Ok(TokenGrant::AuthorizationCode {
            code: req
                .code
                .as_deref()
                .ok_or_else(|| bad_request("invalid_grant"))?,
        }),
        "refresh_token" => Ok(TokenGrant::RefreshToken {
            refresh_token: req
                .refresh_token
                .as_deref()
                .ok_or_else(|| bad_request("invalid_grant"))?,
        }),
        _ => Err(bad_request("unsupported_grant_type")),
    }
}

impl From<TokenExchangeResponse> for TokenResponse {
    fn from(response: TokenExchangeResponse) -> Self {
        Self {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            token_type: response.token_type,
            expires_in: response.expires_in,
        }
    }
}

fn env_refresh_token_ttl_secs(env: &worker::Env) -> u64 {
    env.var("REFRESH_TOKEN_TTL_SECS")
        .ok()
        .and_then(|v| v.to_string().parse().ok())
        .unwrap_or(DEFAULT_REFRESH_TOKEN_TTL_SECS)
}
