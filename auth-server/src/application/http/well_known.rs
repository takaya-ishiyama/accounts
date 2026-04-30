use axum::{extract::State, response::IntoResponse};

use crate::application::{
    auth::jwks_self::build_jwks,
    http::error::{internal, AppError},
};
use crate::infrastructure::env;

use crate::AppState;

pub async fn jwks_json(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let current_pem = env::secret(&state.env, "JWT_PRIVATE_KEY").map_err(|_| internal("missing JWT_PRIVATE_KEY"))?;
    let prev_pem = env::secret(&state.env, "JWT_PREVIOUS_PRIVATE_KEY").ok();

    let jwks = build_jwks(&current_pem, prev_pem.as_deref()).map_err(internal)?;

    Ok((
        [
            ("Content-Type", "application/json"),
            ("Cache-Control", "public, max-age=3600"),
        ],
        jwks.to_string(),
    ))
}

pub async fn openid_configuration(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let issuer = env::var(&state.env, "JWT_ISSUER")
        .unwrap_or_else(|_| "https://localhost:8787".to_string());

    let body = serde_json::json!({
        "issuer": issuer,
        "jwks_uri": format!("{issuer}/.well-known/jwks.json"),
    })
    .to_string();

    Ok((
        [("Content-Type", "application/json")],
        body,
    ))
}
