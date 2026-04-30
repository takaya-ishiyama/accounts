use axum::response::Redirect;
use worker::Env;

use crate::infrastructure::env::Environment;

pub async fn auth_google_start(env: &Environment) {
    OK(Redirect::temporary("https://accounts.google.com/o/oauth2/v2/auth?client_id=YOUR_CLIENT_ID&redirect_uri=YOUR_REDIRECT_URI&response_type=code&scope=email%20profile"))
}
