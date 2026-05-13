use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::future::Future;
use worker::send::SendFuture;

use crate::{
    application::{
        http::error::{bad_request, AppError},
        usecase::commands::complete_google_signin::{
            complete_google_signin, CompleteGoogleSigninConfig,
        },
    },
    infrastructure::{
        account_read_repository::D1AccountReadRepository,
        app_authorization_code_store::DoAppAuthorizationCodeStore, clock::WorkerClock,
        command_bus::DoCommandBus, env::Environment, id_generator::UuidV7Generator,
        oidc_state_store::KvOidcStateStore, providers::google::GoogleProvider,
        random::GetRandomTokenGenerator,
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
        let config = CompleteGoogleSigninConfig {
            app_code_ttl_secs: env_app_code_ttl_secs(env),
        };

        let completed = complete_google_signin(
            &provider,
            &oidc_state_store,
            &app_codes,
            &accounts,
            &commands,
            &clock,
            &ids,
            &tokens,
            &environment.app_clients,
            &config,
            code,
            state_key,
        )
        .await?;

        Ok(Redirect::to(&completed.redirect_uri).into_response())
    })
}

fn env_app_code_ttl_secs(env: &worker::Env) -> u64 {
    env.var("APP_AUTH_CODE_TTL_SECS")
        .ok()
        .and_then(|v| v.to_string().parse().ok())
        .unwrap_or(300)
}
