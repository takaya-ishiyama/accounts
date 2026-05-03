use serde::Serialize;
use worker::{wasm_bindgen::JsValue, Env};

use crate::application::usecase::commands::{
    bus::{CommandBus, CommandBusError, CommandBusFuture},
    dto::{CommandDto, CommandResponse},
};

pub struct DoCommandBus<'a> {
    pub env: &'a Env,
}

#[derive(Serialize)]
struct DoCommandRequest<'a> {
    account_id: &'a str,
    command: &'a CommandDto,
}

impl CommandBus for DoCommandBus<'_> {
    fn dispatch<'a>(
        &'a self,
        account_id: &'a str,
        cmd: &'a CommandDto,
    ) -> CommandBusFuture<'a, CommandResponse> {
        Box::pin(async move {
            let namespace = self
                .env
                .durable_object("AUTH_DO")
                .map_err(to_backend_error)?;
            let stub = namespace
                .id_from_name(account_id)
                .map_err(to_backend_error)?
                .get_stub()
                .map_err(to_backend_error)?;

            let body = serde_json::to_string(&DoCommandRequest {
                account_id,
                command: cmd,
            })
            .map_err(|e| CommandBusError::Backend(e.to_string()))?;

            let headers = worker::Headers::new();
            headers
                .set("Content-Type", "application/json")
                .map_err(to_backend_error)?;

            let mut req_init = worker::RequestInit::new();
            req_init.with_method(worker::Method::Post);
            req_init.with_body(Some(JsValue::from_str(&body)));
            req_init.with_headers(headers);

            let req = worker::Request::new_with_init("https://do/command", &req_init)
                .map_err(to_backend_error)?;
            let mut resp = stub
                .fetch_with_request(req)
                .await
                .map_err(to_backend_error)?;

            if resp.status_code() == 422 {
                let text = resp.text().await.unwrap_or_default();
                return Err(CommandBusError::DomainError(text));
            }
            if !(200..300).contains(&resp.status_code()) {
                let text = resp.text().await.unwrap_or_default();
                return Err(CommandBusError::Backend(format!(
                    "DO command failed {}: {text}",
                    resp.status_code()
                )));
            }

            resp.json::<CommandResponse>()
                .await
                .map_err(|e| CommandBusError::Backend(e.to_string()))
        })
    }
}

fn to_backend_error(e: worker::Error) -> CommandBusError {
    CommandBusError::Backend(e.to_string())
}
