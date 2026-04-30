use crate::domain::account::account::{Account, AccountStatus};
use crate::domain::account::identity::{Identity, IdentityProvider};
use crate::domain::error::error::DomainError;
use crate::domain::events::EventPayload;

pub enum Command {
    SignInWithGoogle {
        subject: String,
        email: String,
        email_verified: bool,
        occurred_at: i64,
    },
    LinkGoogleIdentity {
        subject: String,
        email: String,
        email_verified: bool,
        occurred_at: i64,
    },
    UnlinkIdentity {
        provider: IdentityProvider,
        subject: String,
        occurred_at: i64,
    },
    SuspendAccount {
        occurred_at: i64,
    },
    ReactivateAccount {
        occurred_at: i64,
    },
}

pub fn decide(state: &Account, cmd: &Command) -> Result<Vec<EventPayload>, DomainError> {
    match cmd {
        Command::SignInWithGoogle { subject, email, email_verified, occurred_at } => {
            if !email_verified {
                return Err(DomainError::EmailNotVerified);
            }
            if state.version() == 0 {
                let identity = Identity::new_google(subject.clone(), email.clone(), *email_verified, *occurred_at);
                Ok(vec![EventPayload::AccountRegistered {
                    primary_email: email.clone(),
                    identity,
                }])
            } else if state.find_identity(&IdentityProvider::Google, subject).is_some() {
                Ok(vec![])
            } else {
                let identity = Identity::new_google(subject.clone(), email.clone(), *email_verified, *occurred_at);
                Ok(vec![EventPayload::GoogleIdentityLinked { identity }])
            }
        }
        Command::LinkGoogleIdentity { subject, email, email_verified, occurred_at } => {
            if !email_verified {
                return Err(DomainError::EmailNotVerified);
            }
            if state.find_identity(&IdentityProvider::Google, subject).is_some() {
                return Err(DomainError::IdentityAlreadyLinked {
                    provider: IdentityProvider::Google,
                    subject: subject.clone(),
                });
            }
            let identity = Identity::new_google(subject.clone(), email.clone(), *email_verified, *occurred_at);
            Ok(vec![EventPayload::GoogleIdentityLinked { identity }])
        }
        Command::UnlinkIdentity { provider, subject, .. } => {
            if state.find_identity(provider, subject).is_none() {
                return Err(DomainError::IdentityNotFound {
                    provider: provider.clone(),
                    subject: subject.clone(),
                });
            }
            if state.identities().len() <= 1 && *state.status() == AccountStatus::Active {
                return Err(DomainError::CannotUnlinkLastIdentity);
            }
            Ok(vec![EventPayload::IdentityUnlinked {
                provider: provider.clone(),
                subject: subject.clone(),
            }])
        }
        Command::SuspendAccount { .. } => {
            if *state.status() == AccountStatus::Suspended {
                return Err(DomainError::InvalidStatusTransition {
                    from: AccountStatus::Suspended,
                    to: AccountStatus::Suspended,
                });
            }
            Ok(vec![EventPayload::AccountSuspended])
        }
        Command::ReactivateAccount { .. } => {
            if *state.status() != AccountStatus::Suspended {
                return Err(DomainError::InvalidStatusTransition {
                    from: state.status().clone(),
                    to: AccountStatus::Active,
                });
            }
            Ok(vec![EventPayload::AccountReactivated])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::{EventEnvelope, EventPayload};

    fn make_envelope(version: u64, payload: EventPayload) -> EventEnvelope {
        EventEnvelope {
            event_id: format!("evt-{version}"),
            aggregate_id: "acct-1".to_string(),
            aggregate_type: "Account".to_string(),
            schema_version: 1,
            version,
            occurred_at: 1000,
            command_id: None,
            payload,
        }
    }

    fn apply_events(mut state: Account, events: &[EventPayload]) -> Account {
        for ev in events {
            let next_version = state.version() + 1;
            let envelope = make_envelope(next_version, ev.clone());
            state = state.apply(&envelope);
        }
        state
    }

    fn google_account() -> Account {
        let state = Account::empty();
        let events = decide(&state, &Command::SignInWithGoogle {
            subject: "google-123".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            occurred_at: 1000,
        })
        .unwrap();
        apply_events(state, &events)
    }

    #[test]
    fn sign_in_with_google_new_account_emits_registered() {
        let state = Account::empty();
        let events = decide(&state, &Command::SignInWithGoogle {
            subject: "google-123".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            occurred_at: 1000,
        })
        .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], EventPayload::AccountRegistered { .. }));
    }

    #[test]
    fn sign_in_with_google_unverified_email_is_rejected() {
        let state = Account::empty();
        let result = decide(&state, &Command::SignInWithGoogle {
            subject: "google-123".to_string(),
            email: "test@example.com".to_string(),
            email_verified: false,
            occurred_at: 1000,
        });
        assert_eq!(result, Err(DomainError::EmailNotVerified));
    }

    #[test]
    fn sign_in_with_google_existing_same_subject_is_noop() {
        let state = google_account();
        let events = decide(&state, &Command::SignInWithGoogle {
            subject: "google-123".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            occurred_at: 2000,
        })
        .unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn link_google_identity_new_subject_succeeds() {
        let state = google_account();
        let events = decide(&state, &Command::LinkGoogleIdentity {
            subject: "google-456".to_string(),
            email: "other@example.com".to_string(),
            email_verified: true,
            occurred_at: 2000,
        })
        .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], EventPayload::GoogleIdentityLinked { .. }));
    }

    #[test]
    fn link_google_identity_duplicate_subject_is_rejected() {
        let state = google_account();
        let result = decide(&state, &Command::LinkGoogleIdentity {
            subject: "google-123".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            occurred_at: 2000,
        });
        assert_eq!(
            result,
            Err(DomainError::IdentityAlreadyLinked {
                provider: IdentityProvider::Google,
                subject: "google-123".to_string(),
            })
        );
    }

    #[test]
    fn unlink_last_identity_from_active_account_is_rejected() {
        let state = google_account();
        assert_eq!(*state.status(), AccountStatus::Active);
        assert_eq!(state.identities().len(), 1);
        let result = decide(&state, &Command::UnlinkIdentity {
            provider: IdentityProvider::Google,
            subject: "google-123".to_string(),
            occurred_at: 2000,
        });
        assert_eq!(result, Err(DomainError::CannotUnlinkLastIdentity));
    }

    #[test]
    fn unlink_one_of_two_identities_succeeds() {
        let state = google_account();
        let link_events = decide(&state, &Command::LinkGoogleIdentity {
            subject: "google-456".to_string(),
            email: "other@example.com".to_string(),
            email_verified: true,
            occurred_at: 2000,
        })
        .unwrap();
        let state = apply_events(state, &link_events);
        assert_eq!(state.identities().len(), 2);

        let events = decide(&state, &Command::UnlinkIdentity {
            provider: IdentityProvider::Google,
            subject: "google-123".to_string(),
            occurred_at: 3000,
        })
        .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], EventPayload::IdentityUnlinked { .. }));
    }

    #[test]
    fn suspend_active_account_succeeds() {
        let state = google_account();
        let events = decide(&state, &Command::SuspendAccount { occurred_at: 2000 }).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], EventPayload::AccountSuspended));
    }

    #[test]
    fn suspend_already_suspended_account_is_rejected() {
        let state = google_account();
        let suspend_events = decide(&state, &Command::SuspendAccount { occurred_at: 2000 }).unwrap();
        let state = apply_events(state, &suspend_events);

        let result = decide(&state, &Command::SuspendAccount { occurred_at: 3000 });
        assert_eq!(
            result,
            Err(DomainError::InvalidStatusTransition {
                from: AccountStatus::Suspended,
                to: AccountStatus::Suspended,
            })
        );
    }

    #[test]
    fn reactivate_suspended_account_succeeds() {
        let state = google_account();
        let suspend_events = decide(&state, &Command::SuspendAccount { occurred_at: 2000 }).unwrap();
        let state = apply_events(state, &suspend_events);

        let events = decide(&state, &Command::ReactivateAccount { occurred_at: 3000 }).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], EventPayload::AccountReactivated));
    }

    #[test]
    fn roundtrip_sign_in_with_google() {
        let state = Account::empty();
        let events = decide(&state, &Command::SignInWithGoogle {
            subject: "google-123".to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            occurred_at: 1000,
        })
        .unwrap();

        let state = apply_events(state, &events);

        assert_eq!(state.version(), 1);
        assert_eq!(state.id(), "acct-1");
        assert_eq!(state.identities().len(), 1);
        assert_eq!(*state.status(), AccountStatus::Active);
    }
}
