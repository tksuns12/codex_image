use std::future::Future;
use std::io::Write;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::json;

use crate::auth::state::{AuthState, PersistedAuth};
use crate::config::AuthConfig;

pub type SleepFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

#[derive(Clone)]
pub struct DeviceLoginPollPolicy {
    pub max_attempts: usize,
    pub default_interval: Duration,
    pub request_timeout: Duration,
    sleep_fn: Arc<dyn Fn(Duration) -> SleepFuture + Send + Sync>,
}

impl DeviceLoginPollPolicy {
    pub fn production() -> Self {
        Self::new(120, Duration::from_secs(5), |duration| {
            Box::pin(async move {
                tokio::time::sleep(duration).await;
            })
        })
    }

    pub fn new<F>(max_attempts: usize, default_interval: Duration, sleep_fn: F) -> Self
    where
        F: Fn(Duration) -> SleepFuture + Send + Sync + 'static,
    {
        Self {
            max_attempts,
            default_interval,
            request_timeout: Duration::from_secs(30),
            sleep_fn: Arc::new(sleep_fn),
        }
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    async fn sleep(&self, duration: Duration) {
        (self.sleep_fn)(duration).await;
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DeviceLoginError {
    #[error("user-code request error")]
    UserCodeApi,
    #[error("user-code request timeout")]
    UserCodeTimeout,
    #[error("user-code response contract error")]
    UserCodeContract,
    #[error("poll request error")]
    PollApi,
    #[error("poll timeout")]
    PollTimeout,
    #[error("poll response contract error")]
    PollContract,
    #[error("token exchange request error")]
    TokenExchangeApi,
    #[error("token exchange timeout")]
    TokenExchangeTimeout,
    #[error("token exchange response contract error")]
    TokenExchangeContract,
    #[error("failed to write login instructions")]
    InstructionWrite,
}

impl DeviceLoginError {
    pub fn redacted_message(&self) -> &'static str {
        match self {
            Self::UserCodeApi => "auth user-code request error",
            Self::UserCodeTimeout => "auth user-code timeout",
            Self::UserCodeContract => "auth user-code response error",
            Self::PollApi => "auth poll request error",
            Self::PollTimeout => "auth poll timeout",
            Self::PollContract => "auth poll response error",
            Self::TokenExchangeApi => "auth token exchange error",
            Self::TokenExchangeTimeout => "auth token exchange timeout",
            Self::TokenExchangeContract => "auth token response error",
            Self::InstructionWrite => "auth login instruction output error",
        }
    }
}

#[derive(Debug, Deserialize)]
struct UserCodeResponse {
    #[serde(default)]
    device_auth_id: Option<String>,
    #[serde(default)]
    user_code: Option<String>,
    #[serde(default)]
    usercode: Option<String>,
    #[serde(default)]
    interval: Option<serde_json::Value>,
    #[serde(default)]
    verification_uri: Option<String>,
    #[serde(default)]
    verification_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PollTokenResponse {
    #[serde(default)]
    authorization_code: Option<String>,
    #[serde(default)]
    code_verifier: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenExchangeResponse {
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
}

pub async fn login_device_code<W: Write>(
    config: &AuthConfig,
    client: &Client,
    poll_policy: &DeviceLoginPollPolicy,
    mut writer: W,
) -> Result<PersistedAuth, DeviceLoginError> {
    let user_code_url = join_url(&config.auth_base_url, "/api/accounts/deviceauth/usercode")
        .map_err(|_| DeviceLoginError::UserCodeApi)?;

    let user_code_resp = send_with_timeout(
        client
            .post(user_code_url)
            .json(&json!({ "client_id": config.client_id })),
        poll_policy.request_timeout,
    )
    .await
    .map_err(|err| match err {
        RequestFailure::Timeout => DeviceLoginError::UserCodeTimeout,
        RequestFailure::Transport => DeviceLoginError::UserCodeApi,
    })?;

    if !user_code_resp.status().is_success() {
        return Err(DeviceLoginError::UserCodeApi);
    }

    let user_code_payload =
        parse_json_with_timeout::<UserCodeResponse>(user_code_resp, poll_policy.request_timeout)
            .await
            .map_err(|err| match err {
                ParseFailure::Timeout => DeviceLoginError::UserCodeTimeout,
                ParseFailure::Invalid => DeviceLoginError::UserCodeContract,
            })?;

    let user_code = user_code_payload
        .user_code
        .or(user_code_payload.usercode)
        .filter(|code| !code.trim().is_empty())
        .ok_or(DeviceLoginError::UserCodeContract)?;
    let device_auth_id = user_code_payload
        .device_auth_id
        .filter(|id| !id.trim().is_empty())
        .ok_or(DeviceLoginError::UserCodeContract)?;

    let verification_uri = user_code_payload
        .verification_uri
        .or(user_code_payload.verification_url)
        .unwrap_or_else(|| {
            config
                .auth_base_url
                .join("/activate")
                .map(|url| url.to_string())
                .unwrap_or_else(|_| "<issuer>/activate".to_string())
        });

    writeln!(
        writer,
        "Open {verification_uri} and enter code {user_code} to continue login."
    )
    .map_err(|_| DeviceLoginError::InstructionWrite)?;

    let interval = parse_interval(
        user_code_payload.interval.as_ref(),
        poll_policy.default_interval,
    );

    let (authorization_code, code_verifier) = poll_for_authorization_code(
        config,
        client,
        poll_policy,
        &device_auth_id,
        &user_code,
        interval,
    )
    .await?;

    exchange_token(
        config,
        client,
        poll_policy,
        authorization_code,
        code_verifier,
    )
    .await
}

async fn poll_for_authorization_code(
    config: &AuthConfig,
    client: &Client,
    poll_policy: &DeviceLoginPollPolicy,
    device_auth_id: &str,
    user_code: &str,
    poll_interval: Duration,
) -> Result<(String, String), DeviceLoginError> {
    if poll_policy.max_attempts == 0 {
        return Err(DeviceLoginError::PollTimeout);
    }

    let poll_url = join_url(&config.auth_base_url, "/api/accounts/deviceauth/token")
        .map_err(|_| DeviceLoginError::PollApi)?;

    for attempt in 0..poll_policy.max_attempts {
        let response = send_with_timeout(
            client.post(poll_url.clone()).json(&json!({
                "device_auth_id": device_auth_id,
                "user_code": user_code
            })),
            poll_policy.request_timeout,
        )
        .await
        .map_err(|err| match err {
            RequestFailure::Timeout => DeviceLoginError::PollTimeout,
            RequestFailure::Transport => DeviceLoginError::PollApi,
        })?;

        let status = response.status();

        if status.as_u16() == 403 || status.as_u16() == 404 {
            if attempt + 1 == poll_policy.max_attempts {
                return Err(DeviceLoginError::PollTimeout);
            }

            poll_policy.sleep(poll_interval).await;
            continue;
        }

        if !status.is_success() {
            return Err(DeviceLoginError::PollApi);
        }

        let payload =
            parse_json_with_timeout::<PollTokenResponse>(response, poll_policy.request_timeout)
                .await
                .map_err(|err| match err {
                    ParseFailure::Timeout => DeviceLoginError::PollTimeout,
                    ParseFailure::Invalid => DeviceLoginError::PollContract,
                })?;

        let authorization_code = payload
            .authorization_code
            .filter(|value| !value.trim().is_empty())
            .ok_or(DeviceLoginError::PollContract)?;
        let code_verifier = payload
            .code_verifier
            .filter(|value| !value.trim().is_empty())
            .ok_or(DeviceLoginError::PollContract)?;

        return Ok((authorization_code, code_verifier));
    }

    Err(DeviceLoginError::PollTimeout)
}

async fn exchange_token(
    config: &AuthConfig,
    client: &Client,
    poll_policy: &DeviceLoginPollPolicy,
    authorization_code: String,
    code_verifier: String,
) -> Result<PersistedAuth, DeviceLoginError> {
    let token_url = join_url(&config.auth_base_url, "/oauth/token")
        .map_err(|_| DeviceLoginError::TokenExchangeApi)?;
    let redirect_uri = join_url(&config.auth_base_url, "/deviceauth/callback")
        .map_err(|_| DeviceLoginError::TokenExchangeApi)?;

    let response = send_with_timeout(
        client.post(token_url).form(&[
            ("grant_type", "authorization_code"),
            ("code", authorization_code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("client_id", config.client_id.as_str()),
            ("code_verifier", code_verifier.as_str()),
        ]),
        poll_policy.request_timeout,
    )
    .await
    .map_err(|err| match err {
        RequestFailure::Timeout => DeviceLoginError::TokenExchangeTimeout,
        RequestFailure::Transport => DeviceLoginError::TokenExchangeApi,
    })?;

    if !response.status().is_success() {
        return Err(DeviceLoginError::TokenExchangeApi);
    }

    let payload =
        parse_json_with_timeout::<TokenExchangeResponse>(response, poll_policy.request_timeout)
            .await
            .map_err(|err| match err {
                ParseFailure::Timeout => DeviceLoginError::TokenExchangeTimeout,
                ParseFailure::Invalid => DeviceLoginError::TokenExchangeContract,
            })?;

    let access_token = payload
        .access_token
        .filter(|token| !token.trim().is_empty())
        .ok_or(DeviceLoginError::TokenExchangeContract)?;
    let refresh_token = payload
        .refresh_token
        .filter(|token| !token.trim().is_empty())
        .ok_or(DeviceLoginError::TokenExchangeContract)?;
    let id_token = payload
        .id_token
        .filter(|token| !token.trim().is_empty())
        .ok_or(DeviceLoginError::TokenExchangeContract)?;

    let mut auth = PersistedAuth::new(access_token, refresh_token, id_token);
    match auth.populate_claim_metadata() {
        AuthState::Invalid => Err(DeviceLoginError::TokenExchangeContract),
        _ => Ok(auth),
    }
}

#[derive(Debug)]
enum RequestFailure {
    Timeout,
    Transport,
}

#[derive(Debug)]
enum ParseFailure {
    Timeout,
    Invalid,
}

async fn send_with_timeout(
    request: reqwest::RequestBuilder,
    timeout: Duration,
) -> Result<reqwest::Response, RequestFailure> {
    match tokio::time::timeout(timeout, request.send()).await {
        Err(_) => Err(RequestFailure::Timeout),
        Ok(Err(_)) => Err(RequestFailure::Transport),
        Ok(Ok(response)) => Ok(response),
    }
}

async fn parse_json_with_timeout<T>(
    response: reqwest::Response,
    timeout: Duration,
) -> Result<T, ParseFailure>
where
    T: for<'de> Deserialize<'de>,
{
    match tokio::time::timeout(timeout, response.json::<T>()).await {
        Err(_) => Err(ParseFailure::Timeout),
        Ok(Err(_)) => Err(ParseFailure::Invalid),
        Ok(Ok(parsed)) => Ok(parsed),
    }
}

fn parse_interval(raw: Option<&serde_json::Value>, default: Duration) -> Duration {
    let Some(raw) = raw else {
        return default;
    };

    let parsed = match raw {
        serde_json::Value::String(value) => value.parse::<u64>().ok(),
        serde_json::Value::Number(value) => value.as_u64(),
        _ => None,
    };

    match parsed {
        Some(seconds) => Duration::from_secs(seconds),
        None => default,
    }
}

fn join_url(base: &Url, path: &str) -> Result<Url, ()> {
    base.join(path).map_err(|_| ())
}
