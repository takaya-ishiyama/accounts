use worker::{wasm_bindgen::JsValue, Env, MessageBatch, MessageSysInner, Result};

use crate::domain::events::{EventEnvelope, EventPayload};
use crate::infrastructure::time::now_ms;

pub async fn handle_batch(batch: MessageBatch<EventEnvelope>, env: &Env) -> Result<()> {
    let messages = batch.messages()?;
    let db = env.d1("DB")?;

    for msg in messages {
        let ev = msg.body();
        let processed = db
            .prepare(
                "INSERT OR IGNORE INTO processed_events (event_id, processed_at) VALUES (?1, ?2)",
            )
            .bind(&[JsValue::from_str(&ev.event_id), JsValue::from(now_ms())])?
            .run()
            .await?;

        // Only project if this event was newly inserted (not a duplicate)
        if processed.meta()?.and_then(|m| m.rows_written).unwrap_or(0) > 0 {
            if let Err(e) = project(ev, env).await {
                worker::console_error!("projection error for event {}: {e}", ev.event_id);
                msg.retry();
                continue;
            }
        }

        msg.ack();
    }

    Ok(())
}

async fn project(ev: &EventEnvelope, env: &Env) -> Result<()> {
    let db = env.d1("DB")?;
    let now = now_ms();
    let aggregate_id = &ev.aggregate_id;

    match &ev.payload {
        EventPayload::AccountRegistered { primary_email, identity } => {
            db.prepare(
                "INSERT INTO accounts (id, status, primary_email, created_at, updated_at, version) VALUES (?1, 'Active', ?2, ?3, ?3, ?4) ON CONFLICT(id) DO NOTHING",
            )
            .bind(&[
                JsValue::from_str(aggregate_id),
                JsValue::from_str(primary_email),
                JsValue::from(now),
                JsValue::from(ev.version as f64),
            ])?
            .run()
            .await?;

            db.prepare(
                "INSERT OR IGNORE INTO identities (provider, subject, account_id, email, email_verified, linked_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(&[
                JsValue::from_str(identity.provider().as_str()),
                JsValue::from_str(identity.subject()),
                JsValue::from_str(aggregate_id),
                JsValue::from_str(identity.email()),
                JsValue::from(identity.email_verified() as i32),
                JsValue::from(identity.linked_at()),
            ])?
            .run()
            .await?;
        }
        EventPayload::GoogleIdentityLinked { identity } => {
            db.prepare(
                "INSERT OR IGNORE INTO identities (provider, subject, account_id, email, email_verified, linked_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(&[
                JsValue::from_str(identity.provider().as_str()),
                JsValue::from_str(identity.subject()),
                JsValue::from_str(aggregate_id),
                JsValue::from_str(identity.email()),
                JsValue::from(identity.email_verified() as i32),
                JsValue::from(identity.linked_at()),
            ])?
            .run()
            .await?;

            update_account_version(&db, aggregate_id, ev.version, now).await?;
        }
        EventPayload::IdentityUnlinked { provider, subject } => {
            db.prepare("DELETE FROM identities WHERE provider = ?1 AND subject = ?2")
                .bind(&[
                    JsValue::from_str(provider.as_str()),
                    JsValue::from_str(subject),
                ])?
                .run()
                .await?;

            update_account_version(&db, aggregate_id, ev.version, now).await?;
        }
        EventPayload::EmailVerified { provider, subject } => {
            db.prepare(
                "UPDATE identities SET email_verified = 1 WHERE provider = ?1 AND subject = ?2",
            )
            .bind(&[
                JsValue::from_str(provider.as_str()),
                JsValue::from_str(subject),
            ])?
            .run()
            .await?;

            update_account_version(&db, aggregate_id, ev.version, now).await?;
        }
        EventPayload::PrimaryEmailChanged { new_email } => {
            db.prepare(
                "UPDATE accounts SET primary_email = ?1, updated_at = ?2, version = ?3 WHERE id = ?4 AND version < ?3",
            )
            .bind(&[
                JsValue::from_str(new_email),
                JsValue::from(now),
                JsValue::from(ev.version as f64),
                JsValue::from_str(aggregate_id),
            ])?
            .run()
            .await?;
        }
        EventPayload::AccountSuspended => {
            db.prepare(
                "UPDATE accounts SET status = 'Suspended', updated_at = ?1, version = ?2 WHERE id = ?3 AND version < ?2",
            )
            .bind(&[
                JsValue::from(now),
                JsValue::from(ev.version as f64),
                JsValue::from_str(aggregate_id),
            ])?
            .run()
            .await?;
        }
        EventPayload::AccountReactivated => {
            db.prepare(
                "UPDATE accounts SET status = 'Active', updated_at = ?1, version = ?2 WHERE id = ?3 AND version < ?2",
            )
            .bind(&[
                JsValue::from(now),
                JsValue::from(ev.version as f64),
                JsValue::from_str(aggregate_id),
            ])?
            .run()
            .await?;
        }
    }

    Ok(())
}

async fn update_account_version(
    db: &worker::d1::Database,
    account_id: &str,
    version: u64,
    now: i64,
) -> Result<()> {
    db.prepare(
        "UPDATE accounts SET updated_at = ?1, version = ?2 WHERE id = ?3 AND version < ?2",
    )
    .bind(&[
        JsValue::from(now),
        JsValue::from(version as f64),
        JsValue::from_str(account_id),
    ])?
    .run()
    .await?;
    Ok(())
}
