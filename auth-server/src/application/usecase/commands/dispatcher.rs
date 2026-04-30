use serde::Serialize;
use worker::{wasm_bindgen::JsValue, Env, Result};

use crate::application::commands::dto::{CommandDto, CommandResponse};

#[derive(Serialize)]
struct DoCommandRequest<'a> {
    account_id: &'a str,
    command: &'a CommandDto,
}

/// Routes a command to the per-account AuthDurableObject instance.
/// For new accounts pass a freshly-minted UUIDv7 as `account_id`.
/// For existing accounts pass the known account ID (looked up from D1).
pub async fn dispatch(env: &Env, account_id: &str, cmd: &CommandDto) -> Result<CommandResponse> {
    let namespace = env.durable_object("AUTH_DO")?;
    let stub = namespace.id_from_name(account_id)?.get_stub()?;

    let body = serde_json::to_string(&DoCommandRequest {
        account_id,
        command: cmd,
    })
    .map_err(|e| worker::Error::RustError(e.to_string()))?;

    let headers = worker::Headers::new();
    headers.set("Content-Type", "application/json")?;

    let mut req_init = worker::RequestInit::new();
    req_init.with_method(worker::Method::Post);
    req_init.with_body(Some(JsValue::from_str(&body)));
    req_init.with_headers(headers);

    let req = worker::Request::new_with_init("https://do/command", &req_init)?;
    let mut resp = stub.fetch_with_request(req).await?;

    if resp.status_code() == 422 {
        let text = resp.text().await.unwrap_or_default();
        return Err(worker::Error::RustError(format!("domain_error:{text}")));
    }
    if !(200..300).contains(&resp.status_code()) {
        let text = resp.text().await.unwrap_or_default();
        return Err(worker::Error::RustError(format!(
            "DO command failed {}: {text}",
            resp.status_code()
        )));
    }

    resp.json::<CommandResponse>()
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))
}
