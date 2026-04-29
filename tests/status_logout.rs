use std::fs;

use assert_cmd::Command;
use chrono::{Duration, Utc};
use codex_image::auth::state::fake_jwt;
use serde_json::Value;
use tempfile::TempDir;

fn write_auth_json(home: &std::path::Path, id_token: &str) {
    let auth_path = home.join("auth.json");
    let body = serde_json::json!({
        "version": 1,
        "auth_type": "oauth",
        "access_token": "access-secret",
        "refresh_token": "refresh-secret",
        "id_token": id_token,
        "account_id": "acct_cli_status",
        "access_token_expires_at": Utc::now().to_rfc3339(),
        "last_refresh": Utc::now().to_rfc3339(),
    });
    fs::write(auth_path, serde_json::to_vec_pretty(&body).unwrap()).unwrap();
}

#[test]
fn status_json_not_logged_in_contract() {
    let temp = TempDir::new().unwrap();

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("status")
        .arg("--json")
        .env("CODEX_IMAGE_HOME", temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "not_logged_in");
    assert!(json.get("access_token").is_none());
    assert!(json.get("refresh_token").is_none());
    assert!(json.get("id_token").is_none());
}

#[test]
fn status_json_valid_expired_and_invalid_contracts() {
    let temp = TempDir::new().unwrap();

    let valid_jwt = fake_jwt(
        "acct_cli_status",
        (Utc::now() + Duration::minutes(15)).timestamp(),
    );
    write_auth_json(temp.path(), &valid_jwt);

    let valid_output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("status")
        .arg("--json")
        .env("CODEX_IMAGE_HOME", temp.path())
        .output()
        .unwrap();
    assert!(valid_output.status.success());
    assert!(valid_output.stderr.is_empty());

    let valid_json: Value = serde_json::from_slice(&valid_output.stdout).unwrap();
    assert_eq!(valid_json["status"], "valid");
    assert_eq!(valid_json["account_id"], "acct_cli_status");

    let valid_stdout = String::from_utf8_lossy(&valid_output.stdout);
    assert!(!valid_stdout.contains("access-secret"));
    assert!(!valid_stdout.contains("refresh-secret"));
    assert!(!valid_stdout.contains("id_token"));

    let expired_jwt = fake_jwt(
        "acct_cli_status",
        (Utc::now() - Duration::minutes(15)).timestamp(),
    );
    write_auth_json(temp.path(), &expired_jwt);

    let expired_output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("status")
        .arg("--json")
        .env("CODEX_IMAGE_HOME", temp.path())
        .output()
        .unwrap();
    assert!(expired_output.status.success());
    assert!(expired_output.stderr.is_empty());

    let expired_json: Value = serde_json::from_slice(&expired_output.stdout).unwrap();
    assert_eq!(expired_json["status"], "expired_refreshable");

    fs::write(temp.path().join("auth.json"), "{not-json").unwrap();

    let invalid_output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("status")
        .arg("--json")
        .env("CODEX_IMAGE_HOME", temp.path())
        .output()
        .unwrap();
    assert!(invalid_output.status.success());
    assert!(invalid_output.stderr.is_empty());

    let invalid_json: Value = serde_json::from_slice(&invalid_output.stdout).unwrap();
    assert_eq!(invalid_json["status"], "invalid");
}

#[test]
fn logout_is_idempotent_and_keeps_codex_sentinel() {
    let temp = TempDir::new().unwrap();
    let home_dir = temp.path().join("home");
    let codex_auth_path = home_dir.join(".codex").join("auth.json");
    fs::create_dir_all(codex_auth_path.parent().unwrap()).unwrap();
    let codex_sentinel = br#"{"access_token":"codex-sentinel"}"#;
    fs::write(&codex_auth_path, codex_sentinel).unwrap();

    let owned_home = temp.path().join("owned");
    fs::create_dir_all(&owned_home).unwrap();
    let valid_jwt = fake_jwt(
        "acct_cli_status",
        (Utc::now() + Duration::minutes(15)).timestamp(),
    );
    write_auth_json(&owned_home, &valid_jwt);

    let first_output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("logout")
        .env("HOME", &home_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .output()
        .unwrap();

    assert!(first_output.status.success());
    assert!(first_output.stderr.is_empty());
    let first_json: Value = serde_json::from_slice(&first_output.stdout).unwrap();
    assert_eq!(first_json["logged_out"], true);
    assert_eq!(first_json["status"], "not_logged_in");
    assert!(!owned_home.join("auth.json").exists());

    let second_output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("logout")
        .env("HOME", &home_dir)
        .env("CODEX_IMAGE_HOME", &owned_home)
        .output()
        .unwrap();

    assert!(second_output.status.success());
    assert!(second_output.stderr.is_empty());
    let second_json: Value = serde_json::from_slice(&second_output.stdout).unwrap();
    assert_eq!(second_json["logged_out"], true);
    assert_eq!(second_json["status"], "not_logged_in");
    assert_eq!(fs::read(&codex_auth_path).unwrap(), codex_sentinel);
}

#[test]
fn status_json_auth_file_override_takes_precedence_over_home() {
    let temp = TempDir::new().unwrap();
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    // If status accidentally uses CODEX_IMAGE_HOME, this malformed auth would force `invalid`.
    fs::write(home_dir.join("auth.json"), "{not-json").unwrap();

    let override_path = temp.path().join("override-auth.json");
    let valid_jwt = fake_jwt(
        "acct_override",
        (Utc::now() + Duration::minutes(15)).timestamp(),
    );

    let override_body = serde_json::json!({
        "version": 1,
        "auth_type": "oauth",
        "access_token": "override-access",
        "refresh_token": "override-refresh",
        "id_token": valid_jwt,
        "account_id": "acct_override",
        "access_token_expires_at": Utc::now().to_rfc3339(),
        "last_refresh": Utc::now().to_rfc3339(),
    });
    fs::write(
        &override_path,
        serde_json::to_vec_pretty(&override_body).unwrap(),
    )
    .unwrap();

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("status")
        .arg("--json")
        .env("CODEX_IMAGE_HOME", &home_dir)
        .env("CODEX_IMAGE_AUTH_FILE", &override_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "valid");
    assert_eq!(json["account_id"], "acct_override");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("override-access"));
    assert!(!stdout.contains("override-refresh"));
}

#[test]
fn logout_honors_auth_file_override_and_preserves_codex_sentinel() {
    let temp = TempDir::new().unwrap();

    let home_dir = temp.path().join("home");
    let codex_auth_path = home_dir.join(".codex").join("auth.json");
    fs::create_dir_all(codex_auth_path.parent().unwrap()).unwrap();
    let codex_sentinel = br#"{"access_token":"codex-sentinel"}"#;
    fs::write(&codex_auth_path, codex_sentinel).unwrap();

    let override_path = temp.path().join("override-auth.json");
    let valid_jwt = fake_jwt(
        "acct_override",
        (Utc::now() + Duration::minutes(15)).timestamp(),
    );

    let override_body = serde_json::json!({
        "version": 1,
        "auth_type": "oauth",
        "access_token": "override-access",
        "refresh_token": "override-refresh",
        "id_token": valid_jwt,
        "account_id": "acct_override",
        "access_token_expires_at": Utc::now().to_rfc3339(),
        "last_refresh": Utc::now().to_rfc3339(),
    });
    fs::write(
        &override_path,
        serde_json::to_vec_pretty(&override_body).unwrap(),
    )
    .unwrap();

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("logout")
        .env("HOME", &home_dir)
        .env("CODEX_IMAGE_AUTH_FILE", &override_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["logged_out"], true);
    assert_eq!(json["status"], "not_logged_in");
    assert!(!override_path.exists());
    assert_eq!(fs::read(&codex_auth_path).unwrap(), codex_sentinel);
}
