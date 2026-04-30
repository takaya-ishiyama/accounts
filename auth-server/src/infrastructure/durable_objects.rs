use serde::Deserialize;
use std::time::Duration;
use worker::*;

use crate::application::commands::dto::{CommandDto, CommandResponse};
use crate::domain::account::account::Account;
use crate::domain::events::EventEnvelope;
use crate::domain::logic::{Command, decide};
use crate::infrastructure::ids::new_uuid_v7;
use crate::infrastructure::time::now_ms;

#[derive(Deserialize)]
struct DoCommandRequest {
    account_id: String,
    command: CommandDto,
}

#[durable_object]
pub struct AuthDurableObject {
    state: State,
    env: Env,
}

impl DurableObject for AuthDurableObject {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        match (req.method(), req.path().as_str()) {
            (Method::Post, "/command") => self.handle_command(&mut req).await,
            _ => Response::error("Not Found", 404),
        }
    }

    async fn alarm(&self) -> Result<Response> {
        self.sweep_outbox().await?;
        Response::ok("ok")
    }
}

impl AuthDurableObject {
    async fn handle_command(&self, req: &mut Request) -> Result<Response> {
        let body: DoCommandRequest = req.json().await?;
        let account_id = body.account_id;
        let cmd_dto = body.command;
        let idempotency_key = cmd_dto.idempotency_key().to_owned();
        let cache_key = format!("cmd:{idempotency_key}");

        let storage = self.state.storage();

        // Return cached result for duplicate requests
        if let Some(cached_json) = storage.get::<String>(&cache_key).await? {
            let cached: CommandResponse = serde_json::from_str(&cached_json)
                .map_err(|e| Error::RustError(e.to_string()))?;
            return Response::from_json(&cached);
        }

        // Replay events to reconstruct current state
        let version: u64 = storage.get::<u64>("meta:version").await?.unwrap_or(0);
        let mut account = Account::empty();
        for seq in 1..=version {
            let key = format!("event:{seq:020}");
            if let Some(ev_json) = storage.get::<String>(&key).await? {
                let ev: EventEnvelope = serde_json::from_str(&ev_json)
                    .map_err(|e| Error::RustError(e.to_string()))?;
                account = account.apply(&ev);
            }
        }

        // Convert DTO → domain Command
        let domain_cmd = dto_to_command(&cmd_dto);

        // Run business logic
        let new_payloads = match decide(&account, &domain_cmd) {
            Ok(payloads) => payloads,
            Err(domain_err) => {
                let body = serde_json::to_string(
                    &serde_json::json!({ "error": domain_err.to_string() }),
                )
                .unwrap_or_default();
                let mut resp = Response::ok(body)?;
                resp = resp.with_status(422);
                let headers = Headers::new();
                headers.set("Content-Type", "application/json")?;
                *resp.headers_mut() = headers;
                return Ok(resp);
            }
        };

        // Wrap payloads in EventEnvelopes
        let mut new_envelopes: Vec<EventEnvelope> = Vec::with_capacity(new_payloads.len());
        for (i, payload) in new_payloads.into_iter().enumerate() {
            let ev_version = version + 1 + i as u64;
            new_envelopes.push(EventEnvelope {
                event_id: new_uuid_v7(),
                aggregate_id: account_id.clone(),
                aggregate_type: "Account".to_string(),
                schema_version: 1,
                version: ev_version,
                occurred_at: now_ms(),
                command_id: Some(idempotency_key.clone()),
                payload,
            });
        }

        // Persist events + outbox flags
        for ev in &new_envelopes {
            let event_key = format!("event:{:020}", ev.version);
            let outbox_key = format!("outbox:{:020}", ev.version);
            let ev_json = serde_json::to_string(ev)
                .map_err(|e| Error::RustError(e.to_string()))?;
            storage.put(&event_key, ev_json).await?;
            storage.put(&outbox_key, "pending").await?;
        }
        let new_version = version + new_envelopes.len() as u64;
        storage.put("meta:version", new_version).await?;

        // Attempt Queue flush; outbox flags remain on failure for alarm sweep
        let _ = self.flush_to_queue(&new_envelopes).await;

        // Cache idempotency result
        let resp = CommandResponse {
            account_id: account_id.clone(),
            events_emitted: new_envelopes.len(),
        };
        let resp_json = serde_json::to_string(&resp)
            .map_err(|e| Error::RustError(e.to_string()))?;
        storage.put(&cache_key, resp_json).await?;

        // Schedule outbox alarm as safety net (30 s after command)
        if !new_envelopes.is_empty() {
            let _ = storage.set_alarm(Duration::from_secs(30)).await;
        }

        Response::from_json(&resp)
    }

    /// Best-effort Queue send + outbox flag removal.
    async fn flush_to_queue(&self, envelopes: &[EventEnvelope]) -> Result<()> {
        if envelopes.is_empty() {
            return Ok(());
        }
        let queue = self.env.queue("QUEUE")?;
        for ev in envelopes {
            queue.send(ev).await?;
        }
        let storage = self.state.storage();
        for ev in envelopes {
            let outbox_key = format!("outbox:{:020}", ev.version);
            let _ = storage.delete(&outbox_key).await;
        }
        Ok(())
    }

    /// Alarm handler: scan all outbox flags and retry Queue send.
    async fn sweep_outbox(&self) -> Result<()> {
        let storage = self.state.storage();
        let version: u64 = storage.get::<u64>("meta:version").await?.unwrap_or(0);

        let mut pending: Vec<EventEnvelope> = Vec::new();
        for seq in 1..=version {
            let outbox_key = format!("outbox:{seq:020}");
            if storage.get::<String>(&outbox_key).await?.is_some() {
                let event_key = format!("event:{seq:020}");
                if let Some(ev_json) = storage.get::<String>(&event_key).await? {
                    if let Ok(ev) = serde_json::from_str::<EventEnvelope>(&ev_json) {
                        pending.push(ev);
                    }
                }
            }
        }

        if !pending.is_empty() {
            self.flush_to_queue(&pending).await?;
        }
        Ok(())
    }
}

fn dto_to_command(dto: &CommandDto) -> Command {
    match dto {
        CommandDto::SignInWithGoogle {
            subject,
            email,
            email_verified,
            occurred_at,
            ..
        } => Command::SignInWithGoogle {
            subject: subject.clone(),
            email: email.clone(),
            email_verified: *email_verified,
            occurred_at: *occurred_at,
        },
        CommandDto::LinkGoogleIdentity {
            subject,
            email,
            email_verified,
            occurred_at,
            ..
        } => Command::LinkGoogleIdentity {
            subject: subject.clone(),
            email: email.clone(),
            email_verified: *email_verified,
            occurred_at: *occurred_at,
        },
        CommandDto::UnlinkIdentity {
            provider,
            subject,
            occurred_at,
            ..
        } => Command::UnlinkIdentity {
            provider: provider.clone(),
            subject: subject.clone(),
            occurred_at: *occurred_at,
        },
        CommandDto::SuspendAccount { occurred_at, .. } => {
            Command::SuspendAccount { occurred_at: *occurred_at }
        }
        CommandDto::ReactivateAccount { occurred_at, .. } => {
            Command::ReactivateAccount { occurred_at: *occurred_at }
        }
    }
}
