use axum::{
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;
use std::future::Future;
use worker::send::SendFuture;

use crate::{
    application::{http::error::AppError, usecase::queries::begin_signin::begin_signin},
    infrastructure::{
        clock::WorkerClock, env::Environment, oidc_state_store::KvOidcStateStore,
        providers::google::GoogleProvider, random::GetRandomTokenGenerator,
    },
    AppState,
};

#[derive(Deserialize)]
pub struct StartQuery {
    return_to: Option<String>,
    client_id: String,
    state: Option<String>,
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

        let provider = GoogleProvider {
            env: &environment,
            kv: &kv,
            tokens: &tokens,
            clock: &clock,
        };
        let oidc_state_store = KvOidcStateStore::new(&kv);
        let redirect = begin_signin(
            &provider,
            &oidc_state_store,
            return_to,
            &query.client_id,
            query.state.as_deref(),
            &environment.app_clients,
        )
        .await?;
        Ok(Redirect::to(&redirect.url))
    })
}
