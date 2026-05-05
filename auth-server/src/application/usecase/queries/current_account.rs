use crate::application::{
    account::read_repository::{AccountCurrentRepository, AccountReadRepository, AccountView},
    auth::session_verifier::SessionVerifier,
    http::error::{unauthorized, AppError},
};

pub async fn current_account(
    sessions: &dyn SessionVerifier,
    accounts: &dyn AccountReadRepository,
    current_accounts: &dyn AccountCurrentRepository,
    token: &str,
) -> Result<AccountView, AppError> {
    let session = sessions.verify(token)?;
    if let Some(account) = accounts.get_account(&session.account_id).await? {
        if account.version >= session.account_version {
            return Ok(account);
        }
    }

    current_accounts
        .get_current_account(&session.account_id)
        .await?
        .ok_or_else(|| unauthorized("account_not_found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{
        account::read_repository::{AccountReadFuture, IdentityView},
        auth::session_verifier::{SessionVerifierError, VerifiedSession},
    };
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    };

    #[derive(Clone)]
    struct FakeSessions {
        session: VerifiedSession,
    }

    impl SessionVerifier for FakeSessions {
        fn verify(&self, _token: &str) -> Result<VerifiedSession, SessionVerifierError> {
            Ok(self.session.clone())
        }
    }

    struct FakeD1 {
        account: Option<AccountView>,
    }

    impl AccountReadRepository for FakeD1 {
        fn find_account_id<'a>(
            &'a self,
            _provider: &'a str,
            _subject: &'a str,
        ) -> AccountReadFuture<'a, Option<String>> {
            Box::pin(async { Ok(None) })
        }

        fn get_account<'a>(
            &'a self,
            _account_id: &'a str,
        ) -> AccountReadFuture<'a, Option<AccountView>> {
            Box::pin(async move { Ok(self.account.clone()) })
        }
    }

    struct FakeCurrent {
        account: Option<AccountView>,
    }

    impl AccountCurrentRepository for FakeCurrent {
        fn get_current_account<'a>(
            &'a self,
            _account_id: &'a str,
        ) -> AccountReadFuture<'a, Option<AccountView>> {
            Box::pin(async move { Ok(self.account.clone()) })
        }
    }

    fn account(version: u64) -> AccountView {
        AccountView {
            id: "acct-1".to_string(),
            status: "Active".to_string(),
            primary_email: "user@example.com".to_string(),
            version,
            identities: vec![IdentityView {
                provider: "Google".to_string(),
                subject: "google-1".to_string(),
                email: "user@example.com".to_string(),
                email_verified: true,
            }],
        }
    }

    #[test]
    fn returns_d1_account_when_version_is_fresh() {
        let sessions = FakeSessions {
            session: VerifiedSession {
                account_id: "acct-1".to_string(),
                account_version: 2,
            },
        };
        let d1 = FakeD1 {
            account: Some(account(2)),
        };
        let current = FakeCurrent {
            account: Some(account(3)),
        };

        let result = unwrap_account(block_on(current_account(&sessions, &d1, &current, "token")));

        assert_eq!(result.version, 2);
    }

    #[test]
    fn falls_back_to_current_account_when_d1_is_stale() {
        let sessions = FakeSessions {
            session: VerifiedSession {
                account_id: "acct-1".to_string(),
                account_version: 2,
            },
        };
        let d1 = FakeD1 {
            account: Some(account(1)),
        };
        let current = FakeCurrent {
            account: Some(account(2)),
        };

        let result = unwrap_account(block_on(current_account(&sessions, &d1, &current, "token")));

        assert_eq!(result.version, 2);
    }

    #[test]
    fn falls_back_to_current_account_when_d1_is_missing() {
        let sessions = FakeSessions {
            session: VerifiedSession {
                account_id: "acct-1".to_string(),
                account_version: 1,
            },
        };
        let d1 = FakeD1 { account: None };
        let current = FakeCurrent {
            account: Some(account(1)),
        };

        let result = unwrap_account(block_on(current_account(&sessions, &d1, &current, "token")));

        assert_eq!(result.version, 1);
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);
        let mut future = Box::pin(future);
        loop {
            if let Poll::Ready(output) = Pin::new(&mut future).poll(&mut context) {
                return output;
            }
        }
    }

    fn unwrap_account(result: Result<AccountView, AppError>) -> AccountView {
        match result {
            Ok(account) => account,
            Err(error) => panic!("unexpected error: {}", error.1),
        }
    }

    fn noop_waker() -> Waker {
        unsafe fn clone(_: *const ()) -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }
        unsafe fn wake(_: *const ()) {}
        unsafe fn wake_by_ref(_: *const ()) {}
        unsafe fn drop(_: *const ()) {}
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

        unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
    }
}
