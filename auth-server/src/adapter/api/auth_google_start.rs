use crate::{
    application::usecase::queries::auth_google_start::auth_google_start,
    infrastructure::env::Environment, AppState,
};
use axum::{extract::State, response::Redirect};

pub async fn router(State(state): State<AppState>) {
    let _env = &state.env;
    let env = Environment::load(_env);
    auth_google_start(&env).await.unwrap();
}
