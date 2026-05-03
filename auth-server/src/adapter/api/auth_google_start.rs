use axum::{
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;
use std::future::Future;
use worker::send::SendFuture;

use crate::{
    application::{
        http::error::{bad_request, internal, AppError},
        usecase::queries::begin_signin::begin_signin,
    },
    domain::auth::audience::AudienceError,
    infrastructure::{
        clock::WorkerClock, env::Environment, oidc_state_store::KvOidcStateStore,
        providers::google::GoogleProvider, random::GetRandomTokenGenerator,
    },
    AppState,
};

#[derive(Deserialize)]
pub struct StartQuery {
    return_to: Option<String>,
    audience: Option<String>,
}

pub fn router(
    State(state): State<AppState>,
    Query(query): Query<StartQuery>,
) -> SendFuture<impl Future<Output = Result<Redirect, AppError>>> {
    SendFuture::new(async move {
        let e = &state.env;
        let environment = Environment::load(e)?;
        let kv = e.kv("KV")?;
        let return_to = query.return_to.as_deref().unwrap_or("/");
        let clock = WorkerClock;
        let tokens = GetRandomTokenGenerator;

        let audience = environment
            .allowed_audiences
            .select(query.audience.as_deref(), &environment.default_audience)
            .map_err(|e| match e {
                AudienceError::Unknown(_) => bad_request("invalid_audience"),
                _ => internal(e.to_string()),
            })?;

        let provider = GoogleProvider {
            env: &environment,
            kv: &kv,
            tokens: &tokens,
            clock: &clock,
        };
        let oidc_state_store = KvOidcStateStore::new(&kv);
        let redirect = begin_signin(&provider, &oidc_state_store, return_to, audience).await?;
        Ok(Redirect::to(&redirect.url))
    })
}
