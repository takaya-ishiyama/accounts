use axum::{
    extract::State,
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
};
use std::future::Future;
use worker::send::SendFuture;

use crate::{
    adapter::api::session_cookie::extract_session_cookie,
    application::{
        http::error::{unauthorized, AppError},
        usecase::queries::current_account::current_account,
    },
    infrastructure::{
        account_read_repository::{D1AccountReadRepository, DoAccountCurrentRepository},
        clock::WorkerClock,
        session_verifier::EnvSessionVerifier,
    },
    AppState,
};

pub fn router(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> SendFuture<impl Future<Output = Result<Response, AppError>>> {
    SendFuture::new(async move {
        let token =
            extract_session_cookie(&headers).ok_or_else(|| unauthorized("missing_session"))?;
        let clock = WorkerClock;
        let sessions = EnvSessionVerifier::load(&state.env, &clock)?;
        let accounts = D1AccountReadRepository { env: &state.env };
        let current_accounts = DoAccountCurrentRepository { env: &state.env };
        let account = current_account(&sessions, &accounts, &current_accounts, token).await?;
        let body = serde_json::to_string(&account).map_err(|e| {
            crate::application::http::error::internal(format!("serialize account response: {e}"))
        })?;

        Ok(([(header::CONTENT_TYPE, "application/json")], body).into_response())
    })
}
