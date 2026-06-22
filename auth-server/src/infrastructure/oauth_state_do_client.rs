use serde::Serialize;
use worker::{wasm_bindgen::JsValue, Env, Error, Headers, Method, Request, RequestInit, Result};

#[derive(Serialize)]
struct StoreRequest<'a> {
    value_json: &'a str,
    ttl_secs: u64,
}

pub async fn save_oauth_state(
    env: &Env,
    name: &str,
    value_json: &str,
    ttl_secs: u64,
) -> Result<()> {
    let body = serde_json::to_string(&StoreRequest {
        value_json,
        ttl_secs,
    })
    .map_err(|e| Error::RustError(e.to_string()))?;

    let mut resp = fetch_oauth_state_do(env, name, Method::Post, "/value", Some(body)).await?;
    ensure_success(&mut resp, "save oauth state").await
}

pub async fn load_oauth_state(env: &Env, name: &str) -> Result<Option<String>> {
    let mut resp = fetch_oauth_state_do(env, name, Method::Get, "/value", None).await?;
    read_optional_text(&mut resp, "load oauth state").await
}

pub async fn load_and_delete_oauth_state(env: &Env, name: &str) -> Result<Option<String>> {
    let mut resp = fetch_oauth_state_do(env, name, Method::Post, "/load-and-delete", None).await?;
    read_optional_text(&mut resp, "load and delete oauth state").await
}

pub async fn delete_oauth_state(env: &Env, name: &str) -> Result<()> {
    let mut resp = fetch_oauth_state_do(env, name, Method::Delete, "/value", None).await?;
    ensure_success(&mut resp, "delete oauth state").await
}

async fn fetch_oauth_state_do(
    env: &Env,
    name: &str,
    method: Method,
    path: &str,
    body: Option<String>,
) -> Result<worker::Response> {
    let namespace = env.durable_object("OAUTH_STATE_DO")?;
    let stub = namespace.id_from_name(name)?.get_stub()?;

    let mut req_init = RequestInit::new();
    req_init.with_method(method);

    if let Some(body) = body {
        let headers = Headers::new();
        headers.set("Content-Type", "application/json")?;
        req_init.with_headers(headers);
        req_init.with_body(Some(JsValue::from_str(&body)));
    }

    let req = Request::new_with_init(&format!("https://oauth-state-do{path}"), &req_init)?;
    stub.fetch_with_request(req).await
}

async fn read_optional_text(
    resp: &mut worker::Response,
    operation: &str,
) -> Result<Option<String>> {
    if resp.status_code() == 404 {
        return Ok(None);
    }
    if !(200..300).contains(&resp.status_code()) {
        let text = resp.text().await.unwrap_or_default();
        return Err(Error::RustError(format!(
            "{operation} failed {}: {text}",
            resp.status_code()
        )));
    }
    resp.text().await.map(Some)
}

async fn ensure_success(resp: &mut worker::Response, operation: &str) -> Result<()> {
    if !(200..300).contains(&resp.status_code()) {
        let text = resp.text().await.unwrap_or_default();
        return Err(Error::RustError(format!(
            "{operation} failed {}: {text}",
            resp.status_code()
        )));
    }
    Ok(())
}
