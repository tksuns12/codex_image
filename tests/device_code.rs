use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use codex_image::auth::device::{login_device_code, DeviceLoginError, DeviceLoginPollPolicy};
use codex_image::auth::state::fake_jwt;
use codex_image::config::AuthConfig;
use reqwest::Url;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

fn base_config(server: &MockServer) -> AuthConfig {
    AuthConfig {
        auth_file: None,
        home_dir: None,
        auth_base_url: Url::parse(&server.uri()).unwrap(),
        client_id: "client-test".to_string(),
    }
}

fn no_sleep_policy(max_attempts: usize) -> DeviceLoginPollPolicy {
    DeviceLoginPollPolicy::new(max_attempts, Duration::from_secs(0), |_| Box::pin(async {}))
        .with_request_timeout(Duration::from_secs(2))
}

fn token_response() -> String {
    format!(
        "{{\"access_token\":\"access-secret\",\"refresh_token\":\"refresh-secret\",\"id_token\":\"{}\"}}",
        fake_jwt("acct_123", chrono::Utc::now().timestamp() + 3600)
    )
}

#[derive(Clone)]
struct PendingThenSuccess {
    attempts: Arc<AtomicUsize>,
}

impl Respond for PendingThenSuccess {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        match attempt {
            0 => ResponseTemplate::new(403),
            1 => ResponseTemplate::new(404),
            _ => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "authorization_code": "auth-code",
                "code_verifier": "verifier"
            })),
        }
    }
}

#[tokio::test]
async fn device_code_success_returns_auth_and_prints_only_instructions() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_123",
            "usercode": "ABCD-EFGH",
            "interval": "1",
            "verification_uri": "https://example.test/activate"
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "auth-code",
            "code_verifier": "verifier"
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=auth-code"))
        .and(body_string_contains("code_verifier=verifier"))
        .respond_with(ResponseTemplate::new(200).set_body_string(token_response()))
        .expect(1)
        .mount(&server)
        .await;

    let config = base_config(&server);
    let client = reqwest::Client::new();
    let mut output = Vec::new();

    let auth = login_device_code(&config, &client, &no_sleep_policy(2), &mut output)
        .await
        .unwrap();

    assert_eq!(auth.account_id.as_deref(), Some("acct_123"));
    let rendered = String::from_utf8(output).unwrap();
    assert!(rendered.contains("ABCD-EFGH"));
    assert!(rendered.contains("example.test/activate"));
    assert!(!rendered.contains("access-secret"));
    assert!(!rendered.contains("refresh-secret"));
}

#[tokio::test]
async fn device_code_poll_pending_403_and_404_then_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_123",
            "user_code": "ABCD-EFGH",
            "interval": "0"
        })))
        .mount(&server)
        .await;

    let attempts = Arc::new(AtomicUsize::new(0));
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(PendingThenSuccess {
            attempts: attempts.clone(),
        })
        .expect(3)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(token_response()))
        .expect(1)
        .mount(&server)
        .await;

    let config = base_config(&server);
    let client = reqwest::Client::new();
    let mut output = Vec::new();

    let policy = DeviceLoginPollPolicy::new(4, Duration::from_millis(0), |_| Box::pin(async {}));
    let auth = login_device_code(&config, &client, &policy, &mut output)
        .await
        .unwrap();

    assert_eq!(auth.account_id.as_deref(), Some("acct_123"));
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn device_code_poll_missing_code_verifier_fails_closed() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_123",
            "user_code": "ABCD-EFGH",
            "interval": "0"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "auth-code"
        })))
        .mount(&server)
        .await;

    let config = base_config(&server);
    let client = reqwest::Client::new();
    let mut output = Vec::new();

    let err = login_device_code(&config, &client, &no_sleep_policy(2), &mut output)
        .await
        .unwrap_err();

    assert!(matches!(err, DeviceLoginError::PollContract));
    assert!(!err.redacted_message().contains("auth-code"));
}

#[tokio::test]
async fn device_code_token_endpoint_500_is_redacted_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_123",
            "usercode": "ABCD-EFGH",
            "interval": "0"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "auth-code",
            "code_verifier": "verifier"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server exploded"))
        .mount(&server)
        .await;

    let config = base_config(&server);
    let client = reqwest::Client::new();
    let mut output = Vec::new();

    let err = login_device_code(&config, &client, &no_sleep_policy(2), &mut output)
        .await
        .unwrap_err();

    assert!(matches!(err, DeviceLoginError::TokenExchangeApi));
    let redacted = err.redacted_message();
    assert!(!redacted.contains("auth-code"));
    assert!(!redacted.contains("server exploded"));
}

#[tokio::test]
async fn device_code_token_endpoint_invalid_json_returns_contract_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_123",
            "usercode": "ABCD-EFGH",
            "interval": "0"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "auth-code",
            "code_verifier": "verifier"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{not-json"))
        .mount(&server)
        .await;

    let config = base_config(&server);
    let client = reqwest::Client::new();
    let mut output = Vec::new();

    let err = login_device_code(&config, &client, &no_sleep_policy(2), &mut output)
        .await
        .unwrap_err();

    assert!(matches!(err, DeviceLoginError::TokenExchangeContract));
}

#[tokio::test]
async fn device_code_malformed_responses_fail_closed() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"usercode\":\"ABCD\"}"))
        .mount(&server)
        .await;

    let config = base_config(&server);
    let client = reqwest::Client::new();
    let mut output = Vec::new();

    let err = login_device_code(&config, &client, &no_sleep_policy(2), &mut output)
        .await
        .unwrap_err();
    assert!(matches!(err, DeviceLoginError::UserCodeContract));

    let output_text = String::from_utf8(output).unwrap();
    assert!(!output_text.contains("access-secret"));
    assert!(!output_text.contains("refresh-secret"));
}

#[tokio::test]
async fn device_code_missing_required_token_fields_fails_closed_without_leaking() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_123",
            "user_code": "ABCD-EFGH",
            "interval": "0"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "auth-code",
            "code_verifier": "verifier"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "access-secret",
            "refresh_token": "refresh-secret"
        })))
        .mount(&server)
        .await;

    let config = base_config(&server);
    let client = reqwest::Client::new();
    let mut output = Vec::new();

    let err = login_device_code(&config, &client, &no_sleep_policy(2), &mut output)
        .await
        .unwrap_err();

    assert!(matches!(err, DeviceLoginError::TokenExchangeContract));

    let message = err.redacted_message();
    assert!(!message.contains("access-secret"));
    assert!(!message.contains("refresh-secret"));
}
