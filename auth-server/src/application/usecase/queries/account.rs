use serde::Deserialize;
use worker::{wasm_bindgen::JsValue, Env, Result};

#[derive(Debug, Deserialize)]
pub struct AccountRow {
    pub id: String,
    pub status: String,
    pub primary_email: String,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct IdentityRow {
    pub provider: String,
    pub subject: String,
    pub email: String,
    pub email_verified: i32,
    pub linked_at: i64,
}

pub async fn find_account_id_by_google_subject(env: &Env, subject: &str) -> Result<Option<String>> {
    #[derive(Deserialize)]
    struct Row {
        account_id: String,
    }
    let db = env.d1("DB")?;
    let row = db
        .prepare("SELECT account_id FROM identities WHERE provider = ?1 AND subject = ?2")
        .bind(&[JsValue::from_str("Google"), JsValue::from_str(subject)])?
        .first::<Row>(None)
        .await?;
    Ok(row.map(|r| r.account_id))
}

pub async fn get_account(env: &Env, account_id: &str) -> Result<Option<AccountRow>> {
    let db = env.d1("DB")?;
    db.prepare("SELECT id, status, primary_email, version FROM accounts WHERE id = ?1")
        .bind(&[JsValue::from_str(account_id)])?
        .first::<AccountRow>(None)
        .await
}

pub async fn get_identities(env: &Env, account_id: &str) -> Result<Vec<IdentityRow>> {
    let db = env.d1("DB")?;
    db.prepare(
        "SELECT provider, subject, email, email_verified, linked_at FROM identities WHERE account_id = ?1",
    )
    .bind(&[JsValue::from_str(account_id)])?
    .all()
    .await?
    .results::<IdentityRow>()
}
