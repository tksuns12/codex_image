use std::fs;

use assert_cmd::Command;
use chrono::Utc;
use codex_image::auth::state::fake_jwt;
use predicates::prelude::*;
use tempfile::TempDir;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn token_response() -> String {
    format!(
        "{{\"access_token\":\"access-secret\",\"refresh_token\":\"refresh-secret\",\"id_token\":\"{}\"}}",
        fake_jwt("acct_cli_123", Utc::now().timestamp() + 3600)
    )
}

#[tokio::test]
async fn login_command_persists_owned_auth_and_keeps_codex_auth_unchanged() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .and(body_string_contains("\"client_id\":\"client-from-test\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_cli_123",
            "user_code": "WXYZ-1234",
            "interval": "0",
            "verification_uri": "https://example.test/activate"
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .and(body_string_contains("\"device_auth_id\":\"dev_cli_123\""))
        .and(body_string_contains("\"user_code\":\"WXYZ-1234\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "auth-code-cli",
            "code_verifier": "verifier-cli"
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=auth-code-cli"))
        .and(body_string_contains("client_id=client-from-test"))
        .and(body_string_contains("code_verifier=verifier-cli"))
        .respond_with(ResponseTemplate::new(200).set_body_string(token_response()))
        .expect(1)
        .mount(&server)
        .await;

    let temp = TempDir::new().unwrap();
    let home_dir = temp.path().join("home");
    let codex_auth_path = home_dir.join(".codex").join("auth.json");
    fs::create_dir_all(codex_auth_path.parent().unwrap()).unwrap();
    let codex_sentinel = b"codex-auth-sentinel-v1";
    fs::write(&codex_auth_path, codex_sentinel).unwrap();

    let owned_home = temp.path().join("owned");
    fs::create_dir_all(&owned_home).unwrap();

    let mut cmd = Command::cargo_bin("codex-image").unwrap();
    cmd.arg("login")
        .env("HOME", &home_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .env("CODEX_IMAGE_AUTH_BASE_URL", server.uri())
        .env("CODEX_IMAGE_CLIENT_ID", "client-from-test");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "Open https://example.test/activate",
        ))
        .stdout(predicate::str::contains("WXYZ-1234"))
        .stdout(predicate::str::contains("Login successful."))
        .stdout(predicate::str::contains("access-secret").not())
        .stdout(predicate::str::contains("refresh-secret").not())
        .stderr(predicate::str::is_empty());

    let owned_auth_path = owned_home.join("auth.json");
    let saved_auth = fs::read_to_string(&owned_auth_path).unwrap();
    assert!(saved_auth.contains("\"auth_type\": \"oauth\""));
    assert!(saved_auth.contains("\"account_id\": \"acct_cli_123\""));
    assert!(saved_auth.contains("\"access_token\": \"access-secret\""));
    assert!(saved_auth.contains("\"refresh_token\": \"refresh-secret\""));

    let codex_after = fs::read(&codex_auth_path).unwrap();
    assert_eq!(codex_after, codex_sentinel);
}

#[tokio::test]
async fn login_command_token_exchange_failure_keeps_owned_and_codex_auth_unchanged() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "dev_cli_456",
            "user_code": "FAIL-0001",
            "interval": "0",
            "verification_uri": "https://example.test/activate"
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "auth-code-fail",
            "code_verifier": "verifier-fail"
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_string("access-secret refresh-secret backend exploded"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let temp = TempDir::new().unwrap();
    let home_dir = temp.path().join("home");
    let codex_auth_path = home_dir.join(".codex").join("auth.json");
    fs::create_dir_all(codex_auth_path.parent().unwrap()).unwrap();
    let codex_sentinel = b"codex-auth-sentinel-v2";
    fs::write(&codex_auth_path, codex_sentinel).unwrap();

    let owned_home = temp.path().join("owned");
    fs::create_dir_all(&owned_home).unwrap();
    let owned_auth_path = owned_home.join("auth.json");
    let owned_before = b"owned-auth-before";
    fs::write(&owned_auth_path, owned_before).unwrap();

    let mut cmd = Command::cargo_bin("codex-image").unwrap();
    cmd.arg("login")
        .env("HOME", &home_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .env("CODEX_IMAGE_AUTH_BASE_URL", server.uri())
        .env("CODEX_IMAGE_CLIENT_ID", "client-from-test");

    cmd.assert()
        .code(4)
        .stdout(predicate::str::contains("FAIL-0001"))
        .stdout(predicate::str::contains("access-secret").not())
        .stdout(predicate::str::contains("refresh-secret").not())
        .stderr(predicate::str::contains("\"error\""))
        .stderr(predicate::str::contains(
            "\"code\":\"api.auth_service_request_failed\"",
        ))
        .stderr(predicate::str::contains(
            "\"message\":\"authentication service request failed\"",
        ))
        .stderr(predicate::str::contains("\"recoverable\":true"))
        .stderr(predicate::str::contains("access-secret").not())
        .stderr(predicate::str::contains("refresh-secret").not());

    let owned_after = fs::read(&owned_auth_path).unwrap();
    assert_eq!(owned_after, owned_before);

    let codex_after = fs::read(&codex_auth_path).unwrap();
    assert_eq!(codex_after, codex_sentinel);
}
