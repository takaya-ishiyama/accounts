use axum::{
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;
use std::future::Future;
use worker::send::SendFuture;

use crate::{
    application::{
        auth::providers::google::GoogleProvider, http::error::AppError,
        usecase::queries::begin_signin::begin_signin,
    },
    infrastructure::{
        env::Environment, oidc_state_store::KvOidcStateStore, random::GetRandomTokenGenerator,
    },
    AppState,
};

#[derive(Deserialize)]
pub struct StartQuery {
    return_to: Option<String>,
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
        let tokens = GetRandomTokenGenerator;
        let provider = GoogleProvider {
            env: &environment,
            tokens: &tokens,
        };
        let oidc_state_store = KvOidcStateStore::new(&kv);
        let redirect = begin_signin(&provider, &oidc_state_store, return_to).await?;
        Ok(Redirect::to(&redirect.url))
    })
}
