use crate::{
    application::{
        account::read_repository::AccountReadError,
        auth::{oidc_state::OidcStateStoreError, session_issuer::SessionIssuerError},
        usecase::commands::bus::CommandBusError,
    },
    domain::auth::provider::ProviderError,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub struct AppError(pub StatusCode, pub String);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "error": self.1 }).to_string();
        (self.0, [("Content-Type", "application/json")], body).into_response()
    }
}

impl From<worker::Error> for AppError {
    fn from(e: worker::Error) -> Self {
        AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}

impl From<ProviderError> for AppError {
    fn from(e: ProviderError) -> Self {
        match e {
            ProviderError::InvalidGrant(msg) => bad_request(msg),
            ProviderError::Internal(msg) => internal(msg),
        }
    }
}

impl From<OidcStateStoreError> for AppError {
    fn from(e: OidcStateStoreError) -> Self {
        internal(e.to_string())
    }
}

impl From<AccountReadError> for AppError {
    fn from(e: AccountReadError) -> Self {
        internal(e.to_string())
    }
}

impl From<SessionIssuerError> for AppError {
    fn from(e: SessionIssuerError) -> Self {
        internal(e.to_string())
    }
}

impl From<CommandBusError> for AppError {
    fn from(e: CommandBusError) -> Self {
        match e {
            CommandBusError::DomainError(msg) => bad_request(msg),
            CommandBusError::Backend(msg) => internal(msg),
        }
    }
}

pub fn bad_request(msg: impl Into<String>) -> AppError {
    AppError(StatusCode::BAD_REQUEST, msg.into())
}

pub fn unauthorized(msg: impl Into<String>) -> AppError {
    AppError(StatusCode::UNAUTHORIZED, msg.into())
}

pub fn internal(msg: impl Into<String>) -> AppError {
    AppError(StatusCode::INTERNAL_SERVER_ERROR, msg.into())
}
