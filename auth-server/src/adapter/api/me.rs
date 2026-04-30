use crate::{application::usecase::commands::dispatcher::dispatch, AppState};
use axum::extract::State;

pub async fn router(State(state): State<AppState>) -> &'static str {
    let _env = &state.env;
    "Hello"
}
