use worker::{wasm_bindgen::JsValue, Env};

use crate::application::account::read_repository::{
    AccountReadError, AccountReadFuture, AccountReadRepository,
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
                .prepare(
                    "SELECT account_id FROM identities WHERE provider = ?1 AND subject = ?2",
                )
                .bind(&[
                    JsValue::from_str(provider),
                    JsValue::from_str(subject),
                ])
                .map_err(to_backend_error)?
                .first::<Row>(None)
                .await
                .map_err(to_backend_error)?;
            Ok(row.map(|r| r.account_id))
        })
    }
}

fn to_backend_error(e: worker::Error) -> AccountReadError {
    AccountReadError::Backend(e.to_string())
}
