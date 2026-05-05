use worker::{wasm_bindgen::JsValue, Env};

use crate::application::account::read_repository::{
    AccountCurrentRepository, AccountReadError, AccountReadFuture, AccountReadRepository,
    AccountView, IdentityView,
};

pub struct D1AccountReadRepository<'a> {
    pub env: &'a Env,
}

pub struct DoAccountCurrentRepository<'a> {
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
                version: i64,
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
                .prepare("SELECT id, status, primary_email, version FROM accounts WHERE id = ?1")
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
                version: account.version.max(0) as u64,
                identities,
            }))
        })
    }
}

impl AccountCurrentRepository for DoAccountCurrentRepository<'_> {
    fn get_current_account<'a>(
        &'a self,
        account_id: &'a str,
    ) -> AccountReadFuture<'a, Option<AccountView>> {
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

            let mut req_init = worker::RequestInit::new();
            req_init.with_method(worker::Method::Get);

            let req = worker::Request::new_with_init(
                &format!("https://do/accounts/{account_id}/current"),
                &req_init,
            )
            .map_err(to_backend_error)?;
            let mut resp = stub
                .fetch_with_request(req)
                .await
                .map_err(to_backend_error)?;

            if resp.status_code() == 404 {
                return Ok(None);
            }
            if !(200..300).contains(&resp.status_code()) {
                let text = resp.text().await.unwrap_or_default();
                return Err(AccountReadError::Backend(format!(
                    "DO current account failed {}: {text}",
                    resp.status_code()
                )));
            }

            resp.json::<AccountView>()
                .await
                .map(Some)
                .map_err(to_backend_error)
        })
    }
}

fn to_backend_error(e: worker::Error) -> AccountReadError {
    AccountReadError::Backend(e.to_string())
}
