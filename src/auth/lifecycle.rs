use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::auth::state::AuthState;
use crate::auth::store::{AuthStore, StoreError};
use crate::diagnostics::CliError;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AuthStatus {
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    pub access_token_expires_at: Option<DateTime<Utc>>,
}

impl AuthStatus {
    fn from_state(state: AuthState) -> Self {
        Self {
            status: state.as_str(),
            account_id: None,
            access_token_expires_at: None,
        }
    }
}

pub fn status_for_cli(store: &AuthStore) -> Result<AuthStatus, CliError> {
    match store.load() {
        Ok(Some(auth)) => {
            let state = auth.classify(Utc::now());
            Ok(AuthStatus {
                status: state.as_str(),
                account_id: None,
                access_token_expires_at: auth.access_token_expires_at,
            })
        }
        Ok(None) => Ok(AuthStatus::from_state(AuthState::NotLoggedIn)),
        Err(StoreError::Parse) => Ok(AuthStatus::from_state(AuthState::Invalid)),
        Err(err) => Err(CliError::AuthStore(err)),
    }
}

pub fn get_access_token_or_error(store: &AuthStore) -> Result<String, CliError> {
    match store.load() {
        Ok(Some(auth)) => match auth.classify(Utc::now()) {
            AuthState::Valid => Ok(auth.access_token),
            AuthState::NotLoggedIn => Err(CliError::AuthNotLoggedIn),
            AuthState::ExpiredRefreshable => Err(CliError::AuthExpired),
            AuthState::Invalid => Err(CliError::AuthInvalidState),
        },
        Ok(None) => Err(CliError::AuthNotLoggedIn),
        Err(StoreError::Parse) => Err(CliError::AuthInvalidState),
        Err(err) => Err(CliError::AuthStore(err)),
    }
}
