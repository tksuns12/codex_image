use std::fmt;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

const AUTH_VERSION: u32 = 1;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedAuth {
    pub version: u32,
    pub auth_type: String,
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub account_id: Option<String>,
    pub access_token_expires_at: Option<DateTime<Utc>>,
    pub last_refresh: Option<DateTime<Utc>>,
}

impl PersistedAuth {
    pub fn new(access_token: String, refresh_token: String, id_token: String) -> Self {
        Self {
            version: AUTH_VERSION,
            auth_type: "oauth".to_string(),
            access_token,
            refresh_token,
            id_token,
            account_id: None,
            access_token_expires_at: None,
            last_refresh: None,
        }
    }

    pub fn classify(&self, now: DateTime<Utc>) -> AuthState {
        if self.access_token.trim().is_empty() || self.refresh_token.trim().is_empty() {
            return AuthState::Invalid;
        }

        let id_claims = match decode_jwt_claims(&self.id_token) {
            Ok(claims) => claims,
            Err(_) => return AuthState::Invalid,
        };

        let exp = match id_claims.exp {
            Some(exp) => exp,
            None => return AuthState::Invalid,
        };

        let expires_at = match DateTime::<Utc>::from_timestamp(exp, 0) {
            Some(expires_at) => expires_at,
            None => return AuthState::Invalid,
        };

        if self.account_id.is_none() {
            if let Some(account_id) = id_claims.account_id.or(id_claims.sub) {
                if account_id.trim().is_empty() {
                    return AuthState::Invalid;
                }
            } else {
                return AuthState::Invalid;
            }
        }

        if expires_at > now {
            AuthState::Valid
        } else {
            AuthState::ExpiredRefreshable
        }
    }

    pub fn populate_claim_metadata(&mut self) -> AuthState {
        let now = Utc::now();

        let id_claims = match decode_jwt_claims(&self.id_token) {
            Ok(claims) => claims,
            Err(_) => return AuthState::Invalid,
        };

        let exp = match id_claims.exp {
            Some(exp) => exp,
            None => return AuthState::Invalid,
        };

        let expires_at = match DateTime::<Utc>::from_timestamp(exp, 0) {
            Some(expires_at) => expires_at,
            None => return AuthState::Invalid,
        };

        let account_id = id_claims.account_id.or(id_claims.sub);
        let account_id = match account_id {
            Some(id) if !id.trim().is_empty() => id,
            _ => return AuthState::Invalid,
        };

        self.account_id = Some(account_id);
        self.access_token_expires_at = Some(expires_at);
        self.last_refresh = Some(now);

        if expires_at > now {
            AuthState::Valid
        } else {
            AuthState::ExpiredRefreshable
        }
    }

    pub fn sample_valid_for_tests(account_id: &str) -> Self {
        let exp = Utc::now() + TimeDelta::minutes(10);
        let id_token = fake_jwt(account_id, exp.timestamp());
        let mut auth = Self::new(
            "access-token".to_string(),
            "refresh-token".to_string(),
            id_token,
        );
        let _ = auth.populate_claim_metadata();
        auth
    }
}

impl fmt::Debug for PersistedAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PersistedAuth")
            .field("version", &self.version)
            .field("auth_type", &self.auth_type)
            .field("access_token", &"[REDACTED]")
            .field("refresh_token", &"[REDACTED]")
            .field("id_token", &"[REDACTED]")
            .field("account_id", &self.account_id)
            .field("access_token_expires_at", &self.access_token_expires_at)
            .field("last_refresh", &self.last_refresh)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthState {
    NotLoggedIn,
    Valid,
    ExpiredRefreshable,
    Invalid,
}

#[derive(Debug, Deserialize)]
struct JwtClaims {
    #[serde(default)]
    sub: Option<String>,
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    exp: Option<i64>,
}

fn decode_jwt_claims(token: &str) -> Result<JwtClaims, ()> {
    let mut parts = token.split('.');
    let _header = parts.next().ok_or(())?;
    let payload = parts.next().ok_or(())?;
    let _sig = parts.next().ok_or(())?;

    if payload.is_empty() {
        return Err(());
    }

    let decoded = URL_SAFE_NO_PAD.decode(payload).map_err(|_| ())?;
    serde_json::from_slice::<JwtClaims>(&decoded).map_err(|_| ())
}

pub fn fake_jwt(account_id: &str, exp: i64) -> String {
    #[derive(Serialize)]
    struct Header<'a> {
        alg: &'a str,
        typ: &'a str,
    }

    #[derive(Serialize)]
    struct Claims<'a> {
        sub: &'a str,
        account_id: &'a str,
        exp: i64,
    }

    let header = serde_json::to_vec(&Header {
        alg: "none",
        typ: "JWT",
    })
    .expect("header json");
    let claims = serde_json::to_vec(&Claims {
        sub: account_id,
        account_id,
        exp,
    })
    .expect("claims json");

    format!(
        "{}.{}.",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(claims)
    )
}
