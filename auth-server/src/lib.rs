use axum::{routing::get, Router};
use tower_service::Service;
use worker::*;

pub mod adapter;
pub mod application;
pub mod domain;
pub mod infrastructure;

pub use infrastructure::durable_objects::AuthDurableObject;

fn router() -> Router {
    Router::new()
        .route("/", get(root))
        .route("/healthz", get(|| async { "OK" }))
        .route("/me", get(adapter::api::me::router))
        .route(
            "/auth/google/start",
            get(adapter::api::auth_google_start::router),
        )
        .route(
            "/auth/google/callback",
            get(adapter::api::auth_google_callback::router),
        )
        .route("/auth/logout", get(adapter::api::auth_logout::router))
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    Ok(router().call(req).await?)
}

pub async fn root() -> &'static str {
    "Hello Axum!"
}
