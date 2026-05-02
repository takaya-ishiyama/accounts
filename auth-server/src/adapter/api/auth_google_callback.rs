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
        usecase::commands::complete_google_signin::complete_google_signin,
    },
    infrastructure::oidc_state_store::KvOidcStateStore,
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
        let e = &state.env;

        if let Some(error) = query.error {
            return Err(bad_request(error));
        }
        let kv = e.kv("KV")?;

        let code = query
            .code
            .as_deref()
            .ok_or_else(|| bad_request("missing_code"))?;
        let state_key = query
            .state
            .as_deref()
            .ok_or_else(|| bad_request("missing_state"))?;

        let oidc_state_store = KvOidcStateStore::new(&kv);

        let completed =
            complete_google_signin(&state.env, code, &oidc_state_store, state_key).await?;

        Ok(redirect_with_session(
            &completed.return_to,
            &completed.jwt,
            completed.ttl_secs,
        )?)
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
