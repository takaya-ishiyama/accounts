use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

use crate::{
    adapter::api::session_cookie::SESSION_COOKIE,
    application::http::error::{internal, AppError},
};

pub async fn router() -> Result<Response, AppError> {
    let mut response = StatusCode::NO_CONTENT.into_response();
    let cookie = format!("{SESSION_COOKIE}=; Max-Age=0; Path=/; Secure; HttpOnly; SameSite=Lax");
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| internal(e.to_string()))?,
    );
    Ok(response)
}
