use worker::{wasm_bindgen::JsValue, Env};

use crate::application::account::read_repository::{
    AccountReadError, AccountReadFuture, AccountReadRepository, AccountView, IdentityView,
};

pub struct D1AccountReadRepository<'a> {
    pub env: &'a Env,
}

impl AccountReadRepository for D1AccountReadRepository<'_> {
    fn find_account_id<'a>(
        &'a self,
        provider: &'a str,
        subject: &'a str,
    ) -> AccountReadFuture<'a, Option<String>> {
        Box::pin(async move {
            #[derive(serde::Deserialize)]
            struct Row {
                account_id: String,
            }
            let db = self.env.d1("DB").map_err(to_backend_error)?;
            let row = db
                .prepare("SELECT account_id FROM identities WHERE provider = ?1 AND subject = ?2")
                .bind(&[JsValue::from_str(provider), JsValue::from_str(subject)])
                .map_err(to_backend_error)?
                .first::<Row>(None)
                .await
                .map_err(to_backend_error)?;
            Ok(row.map(|r| r.account_id))
        })
    }

    fn get_account<'a>(
        &'a self,
        account_id: &'a str,
    ) -> AccountReadFuture<'a, Option<AccountView>> {
        Box::pin(async move {
            #[derive(serde::Deserialize)]
            struct AccountRow {
                id: String,
                status: String,
                primary_email: String,
            }

            #[derive(serde::Deserialize)]
            struct IdentityRow {
                provider: String,
                subject: String,
                email: String,
                email_verified: i32,
            }

            let db = self.env.d1("DB").map_err(to_backend_error)?;
            let account = db
                .prepare("SELECT id, status, primary_email FROM accounts WHERE id = ?1")
                .bind(&[JsValue::from_str(account_id)])
                .map_err(to_backend_error)?
                .first::<AccountRow>(None)
                .await
                .map_err(to_backend_error)?;

            let Some(account) = account else {
                return Ok(None);
            };

            let identities = db
                .prepare(
                    "SELECT provider, subject, email, email_verified FROM identities WHERE account_id = ?1",
                )
                .bind(&[JsValue::from_str(account_id)])
                .map_err(to_backend_error)?
                .all()
                .await
                .map_err(to_backend_error)?
                .results::<IdentityRow>()
                .map_err(to_backend_error)?
                .into_iter()
                .map(|row| IdentityView {
                    provider: row.provider,
                    subject: row.subject,
                    email: row.email,
                    email_verified: row.email_verified != 0,
                })
                .collect();

            Ok(Some(AccountView {
                id: account.id,
                status: account.status,
                primary_email: account.primary_email,
                identities,
            }))
        })
    }
}

fn to_backend_error(e: worker::Error) -> AccountReadError {
    AccountReadError::Backend(e.to_string())
}
