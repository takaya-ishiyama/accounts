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
        http::error::{bad_request, internal, AppError},
        usecase::commands::complete_google_signin::{
            complete_google_signin, CompleteGoogleSigninConfig,
        },
    },
    infrastructure::{
        account_read_repository::D1AccountReadRepository,
        app_authorization_code_store::DoAppAuthorizationCodeStore, clock::WorkerClock,
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
        let app_codes = DoAppAuthorizationCodeStore { env };
        let accounts = D1AccountReadRepository { env };
        let commands = DoCommandBus { env };
        let sessions = EnvSessionIssuer::load(env, &clock, &ids)?;
        let config = CompleteGoogleSigninConfig {
            app_code_ttl_secs: env_app_code_ttl_secs(env),
        };

        let completed = complete_google_signin(
            &provider,
            &oidc_state_store,
            &app_codes,
            &accounts,
            &commands,
            &sessions,
            &clock,
            &ids,
            &tokens,
            &environment.app_clients,
            &config,
            code,
            state_key,
            &environment.default_audience,
        )
        .await?;

        redirect_with_session(&completed.redirect_uri, &completed.jwt, completed.ttl_secs)
    })
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

fn env_app_code_ttl_secs(env: &worker::Env) -> u64 {
    env.var("APP_AUTH_CODE_TTL_SECS")
        .ok()
        .and_then(|v| v.to_string().parse().ok())
        .unwrap_or(300)
}
