use axum::{
    http::{header, HeaderValue},
    routing::{get, post},
    Router,
};
use tower_service::Service;
use worker::*;

pub mod adapter;
pub mod application;
pub mod domain;
pub mod infrastructure;

pub use infrastructure::durable_objects::AuthDurableObject;

#[derive(Clone)]
pub struct AppState {
    pub env: Env,
}

fn router(env: Env) -> Router {
    let state = AppState { env };
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
        .route("/auth/logout", post(adapter::api::auth_logout::router))
        .route(
            "/me/identities/google/unlink",
            post(adapter::api::unlink_google_identity::router),
        )
        .route(
            "/.well-known/jwks.json",
            get(application::http::well_known::jwks_json),
        )
        .route(
            "/.well-known/openid-configuration",
            get(application::http::well_known::openid_configuration),
        )
        .with_state(state)
}

#[event(queue)]
async fn queue(
    batch: MessageBatch<domain::events::EventEnvelope>,
    env: Env,
    _ctx: Context,
) -> Result<()> {
    application::projections::consumer::handle_batch(batch, &env).await
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    let mut response = router(env).call(req).await?;
    apply_security_headers(&mut response);
    Ok(response)
}

pub async fn root() -> &'static str {
    "Hello Axum!"
}

fn apply_security_headers(response: &mut axum::http::Response<axum::body::Body>) {
    let headers = response.headers_mut();
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'"),
    );
    headers
        .entry(header::CACHE_CONTROL)
        .or_insert(HeaderValue::from_static("no-store"));
}
