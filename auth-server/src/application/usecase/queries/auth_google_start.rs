use axum::response::Redirect;

use crate::infrastructure::env::Environment;

pub async fn auth_google_start(env: &Environment) -> Result<Redirect, Box<dyn std::error::Error>> {
    let redirect_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth
        ?response_type=code
        &client_id={}
        &redirect_uri={}
        &scope=openid%20email
        &state=<生成したstate>
        &nonce=<生成したnonce>
        &code_challenge=<SHA256(code_verifier)をBase64URL>
        &code_challenge_method=S256",
        env.google_client_id, env.oauth_redirect_uri
    );
    Ok(Redirect::temporary(&redirect_url))
}
