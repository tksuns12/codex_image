use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use reqwest::{Client, Url};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::auth::state::{AuthState, PersistedAuth};
use crate::config::AuthConfig;

const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const AUTHORIZE_PATH: &str = "/oauth/authorize";
const TOKEN_PATH: &str = "/oauth/token";
const SCOPE: &str = "openid profile email offline_access";

#[derive(Clone, Debug)]
pub struct OAuthLoginPolicy {
    pub callback_timeout: Duration,
    pub request_timeout: Duration,
}

impl OAuthLoginPolicy {
    pub fn production() -> Self {
        Self {
            callback_timeout: Duration::from_secs(300),
            request_timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OAuthLoginError {
    #[error("failed to build authorization URL")]
    AuthorizeUrl,
    #[error("failed to bind local OAuth callback listener")]
    CallbackBind,
    #[error("OAuth callback timed out")]
    CallbackTimeout,
    #[error("OAuth callback failed")]
    Callback,
    #[error("OAuth callback state mismatch")]
    CallbackState,
    #[error("token exchange request error")]
    TokenExchangeApi,
    #[error("token exchange timeout")]
    TokenExchangeTimeout,
    #[error("token exchange response contract error")]
    TokenExchangeContract,
    #[error("failed to write login instructions")]
    InstructionWrite,
}

impl OAuthLoginError {
    pub fn redacted_message(&self) -> &'static str {
        match self {
            Self::AuthorizeUrl => "auth authorization URL error",
            Self::CallbackBind => "auth callback listener error",
            Self::CallbackTimeout => "auth callback timeout",
            Self::Callback => "auth callback error",
            Self::CallbackState => "auth callback state error",
            Self::TokenExchangeApi => "auth token exchange error",
            Self::TokenExchangeTimeout => "auth token exchange timeout",
            Self::TokenExchangeContract => "auth token response error",
            Self::InstructionWrite => "auth login instruction output error",
        }
    }
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

pub async fn login_oauth_callback<W: Write>(
    config: &AuthConfig,
    client: &Client,
    policy: &OAuthLoginPolicy,
    mut writer: W,
) -> Result<PersistedAuth, OAuthLoginError> {
    let listener =
        TcpListener::bind("127.0.0.1:1455").map_err(|_| OAuthLoginError::CallbackBind)?;
    listener
        .set_nonblocking(true)
        .map_err(|_| OAuthLoginError::CallbackBind)?;

    let verifier = random_urlsafe(32);
    let challenge = pkce_challenge(&verifier);
    let state = random_urlsafe(16);
    let authorize_url = build_authorize_url(config, &challenge, &state)?;

    writeln!(
        writer,
        "Open {authorize_url} to continue login. The browser will return to {REDIRECT_URI}."
    )
    .map_err(|_| OAuthLoginError::InstructionWrite)?;

    let authorization_code = wait_for_callback(listener, &state, policy.callback_timeout)?;

    exchange_token(config, client, policy, &authorization_code, &verifier).await
}

fn build_authorize_url(
    config: &AuthConfig,
    challenge: &str,
    state: &str,
) -> Result<Url, OAuthLoginError> {
    let mut url = config
        .auth_base_url
        .join(AUTHORIZE_PATH)
        .map_err(|_| OAuthLoginError::AuthorizeUrl)?;

    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("scope", SCOPE)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state)
        .append_pair("id_token_add_organizations", "true")
        .append_pair("codex_cli_simplified_flow", "true")
        .append_pair("originator", "codex-image");

    Ok(url)
}

fn wait_for_callback(
    listener: TcpListener,
    expected_state: &str,
    timeout: Duration,
) -> Result<String, OAuthLoginError> {
    let deadline = std::time::Instant::now() + timeout;
    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(accepted) => break accepted,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    return Err(OAuthLoginError::CallbackTimeout);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return Err(OAuthLoginError::Callback),
        }
    };

    let mut buffer = [0_u8; 8192];
    let read = stream
        .read(&mut buffer)
        .map_err(|_| OAuthLoginError::Callback)?;
    let request = std::str::from_utf8(&buffer[..read]).map_err(|_| OAuthLoginError::Callback)?;
    let request_line = request.lines().next().ok_or(OAuthLoginError::Callback)?;
    let path = request_line
        .strip_prefix("GET ")
        .and_then(|rest| rest.split_whitespace().next())
        .ok_or(OAuthLoginError::Callback)?;

    let parsed =
        Url::parse(&format!("http://localhost{path}")).map_err(|_| OAuthLoginError::Callback)?;

    if parsed.path() != "/auth/callback" {
        write_callback_response(&mut stream, 404, "Callback route not found.");
        return Err(OAuthLoginError::Callback);
    }

    let state = parsed
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.into_owned())
        .ok_or(OAuthLoginError::CallbackState)?;
    if state != expected_state {
        write_callback_response(&mut stream, 400, "State mismatch.");
        return Err(OAuthLoginError::CallbackState);
    }

    let code = parsed
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .filter(|value| !value.trim().is_empty())
        .ok_or(OAuthLoginError::Callback)?;

    write_callback_response(
        &mut stream,
        200,
        "OpenAI authentication completed. You can close this window.",
    );

    Ok(code)
}

fn write_callback_response(stream: &mut TcpStream, status: u16, message: &str) {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    let body = format!("<html><body>{message}</body></html>");
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

async fn exchange_token(
    config: &AuthConfig,
    client: &Client,
    policy: &OAuthLoginPolicy,
    authorization_code: &str,
    code_verifier: &str,
) -> Result<PersistedAuth, OAuthLoginError> {
    let token_url = config
        .auth_base_url
        .join(TOKEN_PATH)
        .map_err(|_| OAuthLoginError::TokenExchangeApi)?;

    let response = send_with_timeout(
        client.post(token_url).form(&[
            ("grant_type", "authorization_code"),
            ("client_id", config.client_id.as_str()),
            ("code", authorization_code),
            ("code_verifier", code_verifier),
            ("redirect_uri", REDIRECT_URI),
        ]),
        policy.request_timeout,
    )
    .await
    .map_err(|err| match err {
        RequestFailure::Timeout => OAuthLoginError::TokenExchangeTimeout,
        RequestFailure::Transport => OAuthLoginError::TokenExchangeApi,
    })?;

    if !response.status().is_success() {
        return Err(OAuthLoginError::TokenExchangeApi);
    }

    let payload =
        parse_json_with_timeout::<TokenExchangeResponse>(response, policy.request_timeout)
            .await
            .map_err(|err| match err {
                ParseFailure::Timeout => OAuthLoginError::TokenExchangeTimeout,
                ParseFailure::Invalid => OAuthLoginError::TokenExchangeContract,
            })?;

    let access_token = payload
        .access_token
        .filter(|token| !token.trim().is_empty())
        .ok_or(OAuthLoginError::TokenExchangeContract)?;
    let refresh_token = payload
        .refresh_token
        .filter(|token| !token.trim().is_empty())
        .ok_or(OAuthLoginError::TokenExchangeContract)?;
    let id_token = payload
        .id_token
        .filter(|token| !token.trim().is_empty())
        .unwrap_or_else(|| access_token.clone());

    let mut auth = PersistedAuth::new(access_token, refresh_token, id_token);
    match auth.populate_claim_metadata() {
        AuthState::Invalid => Err(OAuthLoginError::TokenExchangeContract),
        _ => Ok(auth),
    }
}

fn random_urlsafe(byte_count: usize) -> String {
    let bytes: Vec<u8> = (0..byte_count).map(|_| rand::random::<u8>()).collect();
    URL_SAFE_NO_PAD.encode(bytes)
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_uses_registered_callback_and_codex_params() {
        let config = AuthConfig {
            auth_file: None,
            home_dir: None,
            auth_base_url: Url::parse("https://auth.openai.com").unwrap(),
            client_id: "client-test".to_string(),
        };

        let url = build_authorize_url(&config, "challenge", "state-test").unwrap();
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();

        assert_eq!(
            url.as_str().split('?').next().unwrap(),
            "https://auth.openai.com/oauth/authorize"
        );
        assert_eq!(pairs.get("response_type").unwrap(), "code");
        assert_eq!(pairs.get("client_id").unwrap(), "client-test");
        assert_eq!(pairs.get("redirect_uri").unwrap(), REDIRECT_URI);
        assert_eq!(pairs.get("scope").unwrap(), SCOPE);
        assert_eq!(pairs.get("code_challenge").unwrap(), "challenge");
        assert_eq!(pairs.get("code_challenge_method").unwrap(), "S256");
        assert_eq!(pairs.get("state").unwrap(), "state-test");
        assert_eq!(pairs.get("codex_cli_simplified_flow").unwrap(), "true");
    }
}
