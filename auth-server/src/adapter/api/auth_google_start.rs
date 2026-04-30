use axum::{
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;
use std::future::Future;
use worker::send::SendFuture;

use crate::{
    application::{http::error::AppError, usecase::queries::auth_google_start::auth_google_start},
    infrastructure::env::Environment,
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
        auth_google_start(&environment, &kv, return_to).await
    })
}
