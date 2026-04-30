use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use assert_cmd::Command;
use chrono::{Duration, Utc};
use codex_image::auth::state::fake_jwt;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::{NamedTempFile, TempDir};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn write_auth_json(home: &std::path::Path, access_token: &str) {
    let auth_path = home.join("auth.json");
    let id_token = fake_jwt(
        "acct_generate",
        (Utc::now() + Duration::minutes(15)).timestamp(),
    );

    let body = serde_json::json!({
        "version": 1,
        "auth_type": "oauth",
        "access_token": access_token,
        "refresh_token": "refresh-secret-generate",
        "id_token": id_token,
        "account_id": "acct_generate",
        "access_token_expires_at": Utc::now().to_rfc3339(),
        "last_refresh": Utc::now().to_rfc3339(),
    });

    fs::create_dir_all(home).unwrap();
    fs::write(auth_path, serde_json::to_vec_pretty(&body).unwrap()).unwrap();
}

fn codex_sentinel_setup(home: &std::path::Path) -> std::path::PathBuf {
    let codex_auth_path = home.join(".codex").join("auth.json");
    fs::create_dir_all(codex_auth_path.parent().unwrap()).unwrap();
    fs::write(&codex_auth_path, br#"{"access_token":"codex-sentinel"}"#).unwrap();
    codex_auth_path
}

fn write_fake_codex(temp: &TempDir, source_image: &std::path::Path) -> std::path::PathBuf {
    let script_path = temp.path().join("fake-codex");
    let script = format!(
        r#"#!/usr/bin/env bash
set -eu
last_message=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-last-message)
      shift
      last_message="$1"
      ;;
  esac
  shift || true
done
if [ -z "$last_message" ]; then
  exit 41
fi
printf '{{"image_path":"{}","note":"fake codex generated image"}}' > "$last_message"
"#,
        source_image.display()
    );
    fs::write(&script_path, script).unwrap();
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&script_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).unwrap();
    }
    script_path
}

#[tokio::test]
async fn generate_default_codex_backend_copies_image_and_writes_manifest() {
    let temp = TempDir::new().unwrap();
    let source_image = temp.path().join("codex-source.png");
    fs::write(&source_image, b"codex-image-bytes").unwrap();
    let fake_codex = write_fake_codex(&temp, &source_image);
    let out_dir = temp.path().join("images");

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("generate")
        .arg("red circle")
        .arg("--out")
        .arg(&out_dir)
        .env("CODEX_IMAGE_CODEX_BIN", &fake_codex)
        .env_remove("CODEX_IMAGE_API_BASE_URL")
        .env_remove("CODEX_IMAGE_HOME")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let manifest: Value = serde_json::from_str(stdout.trim_end()).unwrap();
    assert_eq!(manifest["prompt"], "red circle");
    assert_eq!(manifest["model"], "gpt-image-2");

    let image_path = std::path::PathBuf::from(manifest["images"][0]["path"].as_str().unwrap());
    assert_eq!(fs::read(&image_path).unwrap(), b"codex-image-bytes");
    assert_eq!(image_path, out_dir.join("image-0001.png"));

    let manifest_path = out_dir.join("manifest.json");
    assert!(manifest_path.is_file());
}

#[tokio::test]
async fn generate_success_writes_images_and_manifest_with_json_stdout_contract() {
    let temp = TempDir::new().unwrap();
    let home_dir = temp.path().join("home");
    let codex_auth_path = codex_sentinel_setup(&home_dir);

    let owned_home = temp.path().join("owned");
    let access_token = "owned-access-token";
    write_auth_json(&owned_home, access_token);

    let out_dir = temp.path().join("images");

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .and(header("authorization", format!("Bearer {access_token}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "created": 1_746_111_222,
            "data": [{
                "b64_json": "aW1hZ2UtYnl0ZXM=",
                "size": "1024x1024",
                "quality": "high",
                "background": "transparent",
                "output_format": "png"
            }],
            "usage": {
                "total_tokens": 10,
                "input_tokens": 8,
                "output_tokens": 2
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("generate")
        .arg("sunrise over mountains")
        .arg("--out")
        .arg(&out_dir)
        .env("HOME", &home_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .env("CODEX_IMAGE_API_BASE_URL", server.uri())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let trimmed = stdout.trim_end();
    assert_eq!(trimmed.lines().count(), 1, "stdout must be one JSON object");

    let success_json: Value = serde_json::from_str(trimmed).unwrap();
    assert_eq!(success_json["prompt"], "sunrise over mountains");
    assert_eq!(success_json["model"], "gpt-image-2");

    let manifest_path = std::path::PathBuf::from(success_json["manifest_path"].as_str().unwrap());
    assert!(manifest_path.is_file(), "manifest path should exist");

    let image_path = std::path::PathBuf::from(success_json["images"][0]["path"].as_str().unwrap());
    assert!(image_path.is_file(), "generated image path should exist");
    assert_eq!(fs::read(&image_path).unwrap(), b"image-bytes");

    let manifest_text = fs::read_to_string(&manifest_path).unwrap();
    let manifest_json: Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(manifest_json["prompt"], "sunrise over mountains");
    assert_eq!(manifest_json["model"], "gpt-image-2");
    assert_eq!(
        manifest_json["images"][0]["path"],
        image_path.to_string_lossy().as_ref()
    );

    for forbidden in [
        "owned-access-token",
        "refresh-secret-generate",
        "Bearer",
        "b64_json",
        "codex-sentinel",
    ] {
        assert!(!trimmed.contains(forbidden), "stdout leaked {forbidden}");
        assert!(
            !manifest_text.contains(forbidden),
            "manifest leaked {forbidden}"
        );
    }

    assert_eq!(
        fs::read(&codex_auth_path).unwrap(),
        br#"{"access_token":"codex-sentinel"}"#
    );
}

#[tokio::test]
async fn generate_not_logged_in_exits_auth_without_calling_image_api() {
    let temp = TempDir::new().unwrap();
    let out_dir = temp.path().join("images");

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "created": 1,
            "data": [{"b64_json": "Zm9v"}]
        })))
        .expect(0)
        .mount(&server)
        .await;

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("generate")
        .arg("no auth")
        .arg("--out")
        .arg(&out_dir)
        .env("CODEX_IMAGE_HOME", temp.path().join("missing-auth-home"))
        .env("CODEX_IMAGE_API_BASE_URL", server.uri())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8(output.stderr).unwrap();
    let envelope: Value = serde_json::from_str(stderr.trim_end()).unwrap();
    assert_eq!(envelope["error"]["code"], "auth.not_logged_in");

    let requests = server.received_requests().await.unwrap();
    assert!(
        requests.is_empty(),
        "generate must not call API when unauthenticated"
    );
}

#[tokio::test]
async fn generate_api_failure_maps_to_exit_4_and_redacted_stderr() {
    let temp = TempDir::new().unwrap();
    let owned_home = temp.path().join("owned");
    write_auth_json(&owned_home, "access-token-api-fail");

    let out_dir = temp.path().join("images");
    let server = MockServer::start().await;
    let sentinel = "Bearer access-token-api-fail b64_json sk-live-secret";

    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(500).set_body_string(sentinel))
        .expect(1)
        .mount(&server)
        .await;

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("generate")
        .arg("api fail")
        .arg("--out")
        .arg(&out_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .env("CODEX_IMAGE_API_BASE_URL", server.uri())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8(output.stderr).unwrap();
    let envelope: Value = serde_json::from_str(stderr.trim_end()).unwrap();
    assert_eq!(envelope["error"]["code"], "api.image_generation_failed");

    for forbidden in [
        "Bearer",
        "access-token-api-fail",
        "b64_json",
        "sk-live-secret",
    ] {
        assert!(!stderr.contains(forbidden), "stderr leaked {forbidden}");
    }
}

#[tokio::test]
async fn generate_missing_image_scope_maps_to_auth_exit_with_relogin_hint() {
    let temp = TempDir::new().unwrap();
    let owned_home = temp.path().join("owned");
    write_auth_json(&owned_home, "access-token-missing-scope");

    let out_dir = temp.path().join("images");
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "You have insufficient permissions for this operation. Missing scopes: api.model.images.request. Bearer access-token-missing-scope b64_json sk-live-secret"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("generate")
        .arg("scope fail")
        .arg("--out")
        .arg(&out_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .env("CODEX_IMAGE_API_BASE_URL", server.uri())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8(output.stderr).unwrap();
    let envelope: Value = serde_json::from_str(stderr.trim_end()).unwrap();
    assert_eq!(envelope["error"]["code"], "auth.insufficient_scope");
    assert_eq!(
        envelope["error"]["hint"],
        "Run `codex-image login` again to grant image generation access."
    );

    for forbidden in [
        "Bearer",
        "access-token-missing-scope",
        "b64_json",
        "sk-live-secret",
    ] {
        assert!(!stderr.contains(forbidden), "stderr leaked {forbidden}");
    }
}

#[tokio::test]
async fn generate_malformed_or_missing_response_data_maps_to_exit_6() {
    let temp = TempDir::new().unwrap();
    let owned_home = temp.path().join("owned");
    write_auth_json(&owned_home, "access-token-response-contract");
    let out_dir = temp.path().join("images");

    let malformed_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{not-json"))
        .expect(1)
        .mount(&malformed_server)
        .await;

    let malformed_output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("generate")
        .arg("malformed")
        .arg("--out")
        .arg(&out_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .env("CODEX_IMAGE_API_BASE_URL", malformed_server.uri())
        .output()
        .unwrap();

    assert_eq!(malformed_output.status.code(), Some(6));
    let malformed_stderr = String::from_utf8(malformed_output.stderr).unwrap();
    let malformed_envelope: Value = serde_json::from_str(malformed_stderr.trim_end()).unwrap();
    assert_eq!(
        malformed_envelope["error"]["code"],
        "response_contract.image_generation"
    );

    let missing_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "created": 1,
            "data": [{}]
        })))
        .expect(1)
        .mount(&missing_server)
        .await;

    let missing_output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("generate")
        .arg("missing b64")
        .arg("--out")
        .arg(&out_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .env("CODEX_IMAGE_API_BASE_URL", missing_server.uri())
        .output()
        .unwrap();

    assert_eq!(missing_output.status.code(), Some(6));
    let missing_stderr = String::from_utf8(missing_output.stderr).unwrap();
    let missing_envelope: Value = serde_json::from_str(missing_stderr.trim_end()).unwrap();
    assert_eq!(
        missing_envelope["error"]["code"],
        "response_contract.image_generation"
    );
}

#[tokio::test]
async fn generate_invalid_base64_maps_to_exit_6() {
    let temp = TempDir::new().unwrap();
    let owned_home = temp.path().join("owned");
    write_auth_json(&owned_home, "access-token-invalid-base64");
    let out_dir = temp.path().join("images");

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "created": 1,
            "data": [{"b64_json": "%%% not-base64 %%%"}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("generate")
        .arg("bad b64")
        .arg("--out")
        .arg(&out_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .env("CODEX_IMAGE_API_BASE_URL", server.uri())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(6));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8(output.stderr).unwrap();
    let envelope: Value = serde_json::from_str(stderr.trim_end()).unwrap();
    assert_eq!(
        envelope["error"]["code"],
        "response_contract.image_generation"
    );
}

#[test]
fn generate_invalid_api_base_config_maps_to_usage_or_config_exit_2_with_json_envelope() {
    let temp = TempDir::new().unwrap();
    let owned_home = temp.path().join("owned");
    write_auth_json(&owned_home, "access-token-config");
    let out_dir = temp.path().join("images");

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("generate")
        .arg("invalid config")
        .arg("--out")
        .arg(&out_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .env("CODEX_IMAGE_API_BASE_URL", "::not-a-url::")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8(output.stderr).unwrap();
    let envelope: Value = serde_json::from_str(stderr.trim_end()).unwrap();
    assert_eq!(envelope["error"]["code"], "config.invalid");
    assert!(!stderr.contains("CODEX_IMAGE_API_BASE_URL"));
}

#[tokio::test]
async fn generate_filesystem_failure_maps_to_exit_5_when_out_is_existing_file() {
    let temp = TempDir::new().unwrap();
    let owned_home = temp.path().join("owned");
    write_auth_json(&owned_home, "access-token-filesystem");

    let existing_file = NamedTempFile::new().unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "created": 1,
            "data": [{"b64_json": "ZmFrZS1ieXRlcw=="}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("generate")
        .arg("filesystem fail")
        .arg("--out")
        .arg(existing_file.path())
        .env("CODEX_IMAGE_HOME", &owned_home)
        .env("CODEX_IMAGE_API_BASE_URL", server.uri())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8(output.stderr).unwrap();
    let envelope: Value = serde_json::from_str(stderr.trim_end()).unwrap();
    assert_eq!(envelope["error"]["code"], "filesystem.output_write_failed");
}

#[test]
fn generate_clap_usage_errors_emit_no_json_envelope() {
    let mut missing_prompt = Command::cargo_bin("codex-image").unwrap();
    missing_prompt.arg("generate").arg("--out").arg("./images");

    missing_prompt
        .assert()
        .code(2)
        .stderr(predicate::str::contains("<PROMPT>").or(predicate::str::contains("<prompt>")))
        .stderr(predicate::str::contains("\"error\":").not());

    let mut missing_out = Command::cargo_bin("codex-image").unwrap();
    missing_out.arg("generate").arg("prompt only");

    missing_out
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--out"))
        .stderr(predicate::str::contains("\"error\":").not());
}
