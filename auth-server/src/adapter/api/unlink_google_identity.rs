use axum::{
    extract::{Query, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::future::Future;
use worker::send::SendFuture;

use crate::{
    adapter::api::session_cookie::extract_session_cookie,
    application::{
        http::error::{bad_request, internal, unauthorized, AppError},
        usecase::commands::unlink_google_identity::unlink_google_identity,
    },
    infrastructure::{
        clock::WorkerClock, command_bus::DoCommandBus, id_generator::UuidV7Generator,
        session_verifier::EnvSessionVerifier,
    },
    AppState,
};

#[derive(Deserialize)]
pub struct UnlinkQuery {
    subject: Option<String>,
}

pub fn router(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<UnlinkQuery>,
) -> SendFuture<impl Future<Output = Result<Response, AppError>>> {
    SendFuture::new(async move {
        let token =
            extract_session_cookie(&headers).ok_or_else(|| unauthorized("missing_session"))?;
        let subject = query
            .subject
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| bad_request("missing_subject"))?;

        let clock = WorkerClock;
        let ids = UuidV7Generator;
        let sessions = EnvSessionVerifier::load(&state.env, &clock)?;
        let commands = DoCommandBus { env: &state.env };
        let response =
            unlink_google_identity(&sessions, &commands, &clock, &ids, token, subject).await?;
        let body = serde_json::to_string(&response)
            .map_err(|e| internal(format!("serialize unlink response: {e}")))?;

        Ok(([(header::CONTENT_TYPE, "application/json")], body).into_response())
    })
}
