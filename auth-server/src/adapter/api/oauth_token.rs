use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::future::Future;
use worker::send::SendFuture;

use crate::{
    application::{
        account::read_repository::AccountReadRepository,
        account::read_repository::{AccountCurrentRepository, AccountView},
        auth::{
            app_authorization_code::AppAuthorizationCodeStore,
            refresh_token::{RefreshTokenStore, RefreshTokenValue},
            session_issuer::SessionIssuer,
        },
        http::error::{bad_request, AppError},
    },
    domain::{auth::token::TokenGenerator, clock::Clock},
    infrastructure::{
        account_read_repository::{D1AccountReadRepository, DoAccountCurrentRepository},
        app_authorization_code_store::KvAppAuthorizationCodeStore,
        clock::WorkerClock,
        env::Environment,
        id_generator::UuidV7Generator,
        random::GetRandomTokenGenerator,
        refresh_token_store::{hash_refresh_token, KvRefreshTokenStore},
        session_issuer::EnvSessionIssuer,
    },
    AppState,
};

const DEFAULT_REFRESH_TOKEN_TTL_SECS: u64 = 60 * 60 * 24 * 30;

#[derive(Deserialize)]
pub struct TokenRequest {
    grant_type: String,
    client_id: String,
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
    Json(req): Json<TokenRequest>,
) -> SendFuture<impl Future<Output = Result<Json<TokenResponse>, AppError>>> {
    SendFuture::new(async move {
        let env = &state.env;
        let environment = Environment::load(env)?;
        environment
            .app_clients
            .get(&req.client_id)
            .map_err(|_| bad_request("invalid_client"))?;

        let kv = env.kv("KV")?;
        let clock = WorkerClock;
        let ids = UuidV7Generator;
        let tokens = GetRandomTokenGenerator;
        let sessions = EnvSessionIssuer::load(env, &clock, &ids)?;
        let refresh_tokens = KvRefreshTokenStore::new(&kv);

        match req.grant_type.as_str() {
            "authorization_code" => {
                let code = req
                    .code
                    .as_deref()
                    .ok_or_else(|| bad_request("invalid_grant"))?;
                let app_codes = KvAppAuthorizationCodeStore::new(&kv);
                let code_value = app_codes
                    .load_and_delete(code)
                    .await?
                    .ok_or_else(|| bad_request("invalid_grant"))?;

                if code_value.client_id != req.client_id
                    || code_value.expires_at <= now_secs(&clock)
                {
                    return Err(bad_request("invalid_grant"));
                }

                issue_token_response(
                    &sessions,
                    &refresh_tokens,
                    &tokens,
                    &clock,
                    env_refresh_token_ttl_secs(env),
                    &code_value.account_id,
                    code_value.account_version,
                    &code_value.client_id,
                    &code_value.token_audience,
                )
                .await
            }
            "refresh_token" => {
                let refresh_token = req
                    .refresh_token
                    .as_deref()
                    .ok_or_else(|| bad_request("invalid_grant"))?;
                let token_hash = hash_refresh_token(refresh_token);
                let old_value = refresh_tokens
                    .load(&token_hash)
                    .await?
                    .ok_or_else(|| bad_request("invalid_grant"))?;

                if old_value.client_id != req.client_id || old_value.expires_at <= now_secs(&clock)
                {
                    refresh_tokens.delete(&token_hash).await?;
                    return Err(bad_request("invalid_grant"));
                }

                let account = current_account(env, &old_value.account_id).await?;
                let account_version = match account {
                    Some(account) if account.status == "Active" => account.version,
                    _ => {
                        refresh_tokens.delete(&token_hash).await?;
                        return Err(bad_request("invalid_grant"));
                    }
                };

                refresh_tokens.delete(&token_hash).await?;
                issue_token_response(
                    &sessions,
                    &refresh_tokens,
                    &tokens,
                    &clock,
                    env_refresh_token_ttl_secs(env),
                    &old_value.account_id,
                    account_version,
                    &old_value.client_id,
                    &old_value.token_audience,
                )
                .await
            }
            _ => Err(bad_request("unsupported_grant_type")),
        }
    })
}

async fn current_account(
    env: &worker::Env,
    account_id: &str,
) -> Result<Option<AccountView>, AppError> {
    let d1 = D1AccountReadRepository { env };
    if let Some(account) = d1.get_account(account_id).await? {
        return Ok(Some(account));
    }

    let current = DoAccountCurrentRepository { env };
    current
        .get_current_account(account_id)
        .await
        .map_err(Into::into)
}

async fn issue_token_response(
    sessions: &dyn SessionIssuer,
    refresh_tokens: &dyn RefreshTokenStore,
    tokens: &dyn TokenGenerator,
    clock: &dyn Clock,
    refresh_token_ttl_secs: u64,
    account_id: &str,
    account_version: u64,
    client_id: &str,
    token_audience: &str,
) -> Result<Json<TokenResponse>, AppError> {
    let issued = sessions.issue(account_id, account_version, token_audience)?;
    let refresh_token = tokens.generate(32);
    let refresh_hash = hash_refresh_token(&refresh_token);
    let now = now_secs(clock);

    refresh_tokens
        .save(
            &refresh_hash,
            &RefreshTokenValue {
                token_hash: refresh_hash.clone(),
                account_id: account_id.to_string(),
                account_version,
                client_id: client_id.to_string(),
                token_audience: token_audience.to_string(),
                expires_at: now + refresh_token_ttl_secs as i64,
                created_at: now,
                rotated_at: None,
            },
            refresh_token_ttl_secs,
        )
        .await?;

    Ok(Json(TokenResponse {
        access_token: issued.jwt,
        refresh_token,
        token_type: "Bearer",
        expires_in: issued.ttl_secs,
    }))
}

fn env_refresh_token_ttl_secs(env: &worker::Env) -> u64 {
    env.var("REFRESH_TOKEN_TTL_SECS")
        .ok()
        .and_then(|v| v.to_string().parse().ok())
        .unwrap_or(DEFAULT_REFRESH_TOKEN_TTL_SECS)
}

fn now_secs(clock: &dyn Clock) -> i64 {
    clock.now_ms() / 1000
}
