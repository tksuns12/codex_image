use std::fs;
use std::io::Write;

use chrono::{TimeDelta, Utc};
use codex_image::auth::lifecycle::{get_access_token_or_error, status_for_cli};
use codex_image::auth::state::{fake_jwt, AuthState, PersistedAuth};
use codex_image::auth::store::AuthStore;
use codex_image::diagnostics::{CliError, ExitCode};
use tempfile::{NamedTempFile, TempDir};

#[test]
fn auth_lifecycle_status_for_cli_not_logged_in_when_file_missing() {
    let temp = TempDir::new().unwrap();
    let store = AuthStore::new(temp.path().join("missing-auth.json"));

    let status = status_for_cli(&store).unwrap();

    assert_eq!(status.status, "not_logged_in");
    assert!(status.account_id.is_none());
    assert!(status.access_token_expires_at.is_none());
}

#[test]
fn auth_lifecycle_status_for_cli_maps_parse_error_to_invalid_state() {
    let mut malformed = NamedTempFile::new().unwrap();
    writeln!(malformed, "not-json").unwrap();

    let store = AuthStore::new(malformed.path().to_path_buf());

    let status = status_for_cli(&store).unwrap();
    assert_eq!(status.status, "invalid");
}

#[test]
fn auth_lifecycle_status_for_cli_redacts_tokens_in_serialized_json() {
    let temp = TempDir::new().unwrap();
    let auth_path = temp.path().join("auth.json");
    let store = AuthStore::new(auth_path.clone());

    let auth = PersistedAuth::sample_valid_for_tests("acct_123");
    store.save(&auth).unwrap();

    let status = status_for_cli(&store).unwrap();
    let json = serde_json::to_string(&status).unwrap();

    assert_eq!(status.status, "valid");
    assert_eq!(status.account_id.as_deref(), Some("acct_123"));
    assert!(!json.contains("\"access_token\":"));
    assert!(!json.contains("refresh_token"));
    assert!(!json.contains("id_token"));
    assert!(!json.contains("access-token"));
}

#[test]
fn auth_lifecycle_get_access_token_returns_token_only_for_valid_auth() {
    let temp = TempDir::new().unwrap();
    let store = AuthStore::new(temp.path().join("auth.json"));

    let mut auth = PersistedAuth::new(
        "access-token-expected".to_string(),
        "refresh-token".to_string(),
        fake_jwt("acct_123", (Utc::now() + TimeDelta::minutes(15)).timestamp()),
    );
    assert_eq!(auth.populate_claim_metadata(), AuthState::Valid);
    store.save(&auth).unwrap();

    let token = get_access_token_or_error(&store).unwrap();
    assert_eq!(token, "access-token-expected");
}

#[test]
fn auth_lifecycle_get_access_token_maps_missing_expired_invalid_and_parse_to_auth_errors() {
    let temp = TempDir::new().unwrap();

    let missing_store = AuthStore::new(temp.path().join("missing.json"));
    let missing = get_access_token_or_error(&missing_store).unwrap_err();
    assert!(matches!(missing, CliError::AuthNotLoggedIn));
    assert_eq!(missing.exit_code(), ExitCode::Auth);

    let expired_store = AuthStore::new(temp.path().join("expired.json"));
    let mut expired_auth = PersistedAuth::new(
        "access-token".to_string(),
        "refresh-token".to_string(),
        fake_jwt("acct_123", (Utc::now() - TimeDelta::minutes(15)).timestamp()),
    );
    assert_eq!(
        expired_auth.populate_claim_metadata(),
        AuthState::ExpiredRefreshable
    );
    expired_store.save(&expired_auth).unwrap();

    let expired = get_access_token_or_error(&expired_store).unwrap_err();
    assert!(matches!(expired, CliError::AuthExpired));
    assert_eq!(expired.exit_code(), ExitCode::Auth);

    let invalid_store = AuthStore::new(temp.path().join("invalid.json"));
    let mut invalid_auth = PersistedAuth::new(
        "access-token".to_string(),
        "refresh-token".to_string(),
        "bad-jwt".to_string(),
    );
    assert_eq!(invalid_auth.populate_claim_metadata(), AuthState::Invalid);
    invalid_store.save(&invalid_auth).unwrap();

    let invalid = get_access_token_or_error(&invalid_store).unwrap_err();
    assert!(matches!(invalid, CliError::AuthInvalidState));
    assert_eq!(invalid.exit_code(), ExitCode::Auth);

    let parse_path = temp.path().join("parse-invalid.json");
    fs::write(&parse_path, "not-json").unwrap();
    let parse_store = AuthStore::new(parse_path);
    let parse = get_access_token_or_error(&parse_store).unwrap_err();

    assert!(matches!(parse, CliError::AuthInvalidState));
    assert_eq!(parse.exit_code(), ExitCode::Auth);
}

#[test]
fn auth_lifecycle_auth_state_strings_are_stable() {
    assert_eq!(AuthState::NotLoggedIn.as_str(), "not_logged_in");
    assert_eq!(AuthState::Valid.as_str(), "valid");
    assert_eq!(AuthState::ExpiredRefreshable.as_str(), "expired_refreshable");
    assert_eq!(AuthState::Invalid.as_str(), "invalid");
}
