use axum::{
    extract::{Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::future::Future;
use worker::send::SendFuture;

use crate::{
    application::{
        auth::app_authorization_code::{AppAuthorizationCodeStore, AppAuthorizationCodeValue},
        http::error::{bad_request, internal, AppError},
        usecase::commands::complete_google_signin::complete_google_signin,
    },
    domain::{auth::token::TokenGenerator, clock::Clock},
    infrastructure::{
        account_read_repository::D1AccountReadRepository,
        app_authorization_code_store::KvAppAuthorizationCodeStore, clock::WorkerClock,
        command_bus::DoCommandBus, env::Environment, id_generator::UuidV7Generator,
        oidc_state_store::KvOidcStateStore, providers::google::GoogleProvider,
        random::GetRandomTokenGenerator, session_issuer::EnvSessionIssuer,
    },
    AppState,
};

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

pub fn router(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> SendFuture<impl Future<Output = Result<Response, AppError>>> {
    SendFuture::new(async move {
        let env = &state.env;

        if let Some(error) = query.error {
            return Err(bad_request(error));
        }

        let code = query
            .code
            .as_deref()
            .ok_or_else(|| bad_request("missing_code"))?;
        let state_key = query
            .state
            .as_deref()
            .ok_or_else(|| bad_request("missing_state"))?;

        let environment = Environment::load(env)?;
        let kv = env.kv("KV")?;
        let clock = WorkerClock;
        let ids = UuidV7Generator;
        let tokens = GetRandomTokenGenerator;
        let provider = GoogleProvider {
            env: &environment,
            kv: &kv,
            tokens: &tokens,
            clock: &clock,
        };
        let oidc_state_store = KvOidcStateStore::new(&kv);
        let accounts = D1AccountReadRepository { env };
        let commands = DoCommandBus { env };
        let sessions = EnvSessionIssuer::load(env, &clock, &ids)?;
        let app_codes = KvAppAuthorizationCodeStore::new(&kv);

        let completed = complete_google_signin(
            &provider,
            &oidc_state_store,
            &accounts,
            &commands,
            &sessions,
            &clock,
            &ids,
            code,
            state_key,
            &environment.default_audience,
        )
        .await?;

        let client = environment
            .app_clients
            .get(&completed.client_id)
            .map_err(|_| bad_request("invalid_client"))?;
        let app_code = tokens.generate(32);
        let now_secs = clock.now_ms() / 1000;
        let app_return_to = completed.return_to.clone();
        app_codes
            .save(
                &app_code,
                &AppAuthorizationCodeValue {
                    account_id: completed.account_id,
                    account_version: completed.account_version,
                    client_id: completed.client_id,
                    token_audience: completed.token_audience,
                    return_to: completed.return_to,
                    expires_at: now_secs + 300,
                },
            )
            .await?;

        let redirect_uri = redirect_uri_with_code(&client.redirect_uri, &app_code, &app_return_to);
        Ok(redirect_with_session(
            &redirect_uri,
            &completed.jwt,
            completed.ttl_secs,
        )?)
    })
}

fn redirect_uri_with_code(redirect_uri: &str, code: &str, return_to: &str) -> String {
    let separator = if redirect_uri.contains('?') { '&' } else { '?' };
    format!(
        "{redirect_uri}{separator}code={code}&return_to={}",
        percent_encode(return_to)
    )
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

fn redirect_with_session(location: &str, jwt: &str, ttl_secs: u64) -> Result<Response, AppError> {
    let cookie =
        format!("__Host-session={jwt}; Max-Age={ttl_secs}; Path=/; Secure; HttpOnly; SameSite=Lax");
    let mut response = StatusCode::FOUND.into_response();
    response.headers_mut().insert(
        header::LOCATION,
        HeaderValue::from_str(location).map_err(|e| internal(e.to_string()))?,
    );
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| internal(e.to_string()))?,
    );
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    Ok(response)
}
