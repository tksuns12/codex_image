use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};

use chrono::Utc;
use codex_image::auth::state::fake_jwt;
use tempfile::TempDir;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn token_response() -> String {
    format!(
        "{{\"access_token\":\"{}\",\"refresh_token\":\"refresh-secret\"}}",
        fake_jwt("acct_cli_123", Utc::now().timestamp() + 3600)
    )
}

#[tokio::test]
async fn login_command_uses_oauth_callback_and_persists_owned_auth() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=auth-code-cli"))
        .and(body_string_contains("client_id=client-from-test"))
        .and(body_string_contains(
            "redirect_uri=http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback",
        ))
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

    let mut child = Command::new(env!("CARGO_BIN_EXE_codex-image"))
        .arg("login")
        .env("HOME", &home_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .env("CODEX_IMAGE_AUTH_BASE_URL", server.uri())
        .env("CODEX_IMAGE_CLIENT_ID", "client-from-test")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("login command should spawn");

    let stdout = child.stdout.take().expect("stdout should be piped");
    let mut stdout_reader = BufReader::new(stdout);
    let mut first_line = String::new();
    stdout_reader
        .read_line(&mut first_line)
        .expect("login should print authorization URL");

    let authorize_url = first_line
        .strip_prefix("Open ")
        .and_then(|line| line.split(" to continue login.").next())
        .expect("login output should include authorization URL");
    let authorize_url = reqwest::Url::parse(authorize_url).unwrap();
    assert_eq!(authorize_url.path(), "/oauth/authorize");

    let query: std::collections::HashMap<_, _> = authorize_url.query_pairs().into_owned().collect();
    assert_eq!(query.get("response_type").unwrap(), "code");
    assert_eq!(query.get("client_id").unwrap(), "client-from-test");
    assert_eq!(
        query.get("redirect_uri").unwrap(),
        "http://localhost:1455/auth/callback"
    );
    assert_eq!(query.get("codex_cli_simplified_flow").unwrap(), "true");
    let scope = query
        .get("scope")
        .expect("authorize URL should request scopes");
    for required_scope in [
        "openid",
        "profile",
        "email",
        "offline_access",
        "api.model.images.request",
    ] {
        assert!(
            scope
                .split_whitespace()
                .any(|scope| scope == required_scope),
            "authorize URL should request {required_scope}, got {scope}"
        );
    }
    let state = query.get("state").unwrap();

    complete_callback("auth-code-cli", state);

    let status = child.wait().expect("login command should exit");
    let mut stdout_rest = String::new();
    stdout_reader.read_to_string(&mut stdout_rest).unwrap();
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();

    assert!(status.success(), "stderr: {stderr}");
    assert!(stdout_rest.contains("Login successful."));
    assert!(!first_line.contains("refresh-secret"));
    assert!(!stdout_rest.contains("refresh-secret"));
    assert!(!stderr.contains("refresh-secret"));

    let owned_auth_path = owned_home.join("auth.json");
    let saved_auth = fs::read_to_string(&owned_auth_path).unwrap();
    assert!(saved_auth.contains("\"auth_type\": \"oauth\""));
    assert!(saved_auth.contains("\"account_id\": \"acct_cli_123\""));
    assert!(saved_auth.contains("\"refresh_token\": \"refresh-secret\""));

    let codex_after = fs::read(&codex_auth_path).unwrap();
    assert_eq!(codex_after, codex_sentinel);
}

fn complete_callback(code: &str, state: &str) {
    let mut stream = TcpStream::connect("127.0.0.1:1455").expect("callback listener should bind");
    let request = format!(
        "GET /auth/callback?code={code}&state={state} HTTP/1.1\r\nhost: localhost\r\nconnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
}
