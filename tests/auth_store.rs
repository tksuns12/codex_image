use std::fs;
use std::sync::{Mutex, OnceLock};

use chrono::{TimeDelta, Utc};
use codex_image::auth::state::{fake_jwt, AuthState, PersistedAuth};
use codex_image::auth::store::{resolve_auth_path, AuthStore};
use codex_image::config::{AuthConfig, ENV_AUTH_FILE, ENV_HOME};
use reqwest::Url;
use tempfile::TempDir;

fn base_config() -> AuthConfig {
    AuthConfig {
        auth_file: None,
        home_dir: None,
        auth_base_url: Url::parse("https://api.openai.com").unwrap(),
        client_id: "codex-image".to_string(),
    }
}

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn auth_store_prefers_explicit_auth_file_path() {
    let _guard = env_lock();
    let temp = TempDir::new().unwrap();
    let explicit = temp.path().join("custom-auth.json");

    std::env::set_var(ENV_AUTH_FILE, &explicit);
    std::env::remove_var(ENV_HOME);

    let config = AuthConfig::from_env().unwrap();
    std::env::remove_var(ENV_AUTH_FILE);

    let resolved = resolve_auth_path(&config).unwrap();
    assert_eq!(resolved, explicit);
}

#[test]
fn auth_store_uses_home_fallback_path() {
    let _guard = env_lock();
    let temp = TempDir::new().unwrap();

    std::env::remove_var(ENV_AUTH_FILE);
    std::env::set_var(ENV_HOME, temp.path());

    let config = AuthConfig::from_env().unwrap();
    std::env::remove_var(ENV_HOME);

    let resolved = resolve_auth_path(&config).unwrap();
    assert_eq!(resolved, temp.path().join("auth.json"));
}

#[test]
fn auth_store_uses_xdg_style_path_and_not_codex_default() {
    let _guard = env_lock();
    let temp = TempDir::new().unwrap();

    std::env::remove_var(ENV_AUTH_FILE);
    std::env::remove_var(ENV_HOME);
    std::env::set_var("XDG_DATA_HOME", temp.path());

    let config = AuthConfig::from_env().unwrap();
    let resolved = resolve_auth_path(&config).unwrap();

    std::env::remove_var("XDG_DATA_HOME");

    assert_eq!(resolved, temp.path().join("codex-image").join("auth.json"));
    assert_ne!(
        resolved,
        temp.path().join(".codex").join("auth.json"),
        "must never default to Codex CLI auth path"
    );
}

#[test]
fn auth_store_classify_valid_and_expired_and_invalid_states() {
    let now = Utc::now();

    let mut valid = PersistedAuth::new(
        "access-token".to_string(),
        "refresh-token".to_string(),
        fake_jwt("acct_123", (now + TimeDelta::minutes(5)).timestamp()),
    );
    assert_eq!(valid.populate_claim_metadata(), AuthState::Valid);
    assert_eq!(valid.classify(now), AuthState::Valid);

    let mut expired = PersistedAuth::new(
        "access-token".to_string(),
        "refresh-token".to_string(),
        fake_jwt("acct_123", (now - TimeDelta::minutes(5)).timestamp()),
    );
    assert_eq!(
        expired.populate_claim_metadata(),
        AuthState::ExpiredRefreshable
    );
    assert_eq!(expired.classify(now), AuthState::ExpiredRefreshable);

    let mut malformed = PersistedAuth::new(
        "access-token".to_string(),
        "refresh-token".to_string(),
        "not-a-jwt".to_string(),
    );
    assert_eq!(malformed.populate_claim_metadata(), AuthState::Invalid);
    assert_eq!(malformed.classify(now), AuthState::Invalid);

    let mut missing_payload = PersistedAuth::new(
        "access-token".to_string(),
        "refresh-token".to_string(),
        "header..signature".to_string(),
    );
    assert_eq!(
        missing_payload.populate_claim_metadata(),
        AuthState::Invalid
    );
    assert_eq!(missing_payload.classify(now), AuthState::Invalid);
}

#[test]
fn auth_store_debug_output_redacts_tokens() {
    let auth = PersistedAuth::new(
        "secret-access".to_string(),
        "secret-refresh".to_string(),
        "secret-id".to_string(),
    );

    let debug = format!("{auth:?}");

    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("secret-access"));
    assert!(!debug.contains("secret-refresh"));
    assert!(!debug.contains("secret-id"));
}

#[test]
fn auth_store_save_writes_atomically_and_preserves_existing_on_failure() {
    let temp = TempDir::new().unwrap();
    let auth_path = temp.path().join("codex-image").join("auth.json");

    let store = AuthStore::new(auth_path.clone());
    let auth = PersistedAuth::sample_valid_for_tests("acct_123");
    store.save(&auth).unwrap();

    let written = fs::read_to_string(&auth_path).unwrap();
    assert!(written.contains("\"auth_type\""));
    assert!(written.contains("\"account_id\": \"acct_123\""));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file_mode = fs::metadata(&auth_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(file_mode, 0o600);

        let parent_mode = fs::metadata(auth_path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(parent_mode, 0o700);
    }

    let previous = fs::read(&auth_path).unwrap();

    let fail_store = AuthStore::new(auth_path.join("child.json"));
    let second_auth = PersistedAuth::sample_valid_for_tests("acct_987");
    let save_result = fail_store.save(&second_auth);

    assert!(save_result.is_err());
    let after = fs::read(&auth_path).unwrap();
    assert_eq!(after, previous, "existing auth file must remain unchanged");
}

#[test]
fn auth_store_load_missing_file_returns_none() {
    let temp = TempDir::new().unwrap();
    let store = AuthStore::new(temp.path().join("missing.json"));
    let loaded = store.load().unwrap();
    assert!(loaded.is_none());
}

#[test]
fn auth_store_resolve_from_config_constructor() {
    let temp = TempDir::new().unwrap();
    let config = AuthConfig {
        auth_file: Some(temp.path().join("owned.json")),
        ..base_config()
    };

    let store = AuthStore::from_config(&config).unwrap();
    assert_eq!(store.path(), temp.path().join("owned.json"));
}

#[test]
fn auth_store_clear_is_idempotent_for_missing_file() {
    let temp = TempDir::new().unwrap();
    let store = AuthStore::new(temp.path().join("missing.json"));

    store.clear().unwrap();
    store.clear().unwrap();

    assert!(!store.path().exists());
}

#[test]
fn auth_store_clear_removes_only_owned_auth_file() {
    let temp = TempDir::new().unwrap();
    let owned_auth_path = temp.path().join("codex-image").join("auth.json");
    let codex_sentinel_path = temp.path().join(".codex").join("auth.json");

    fs::create_dir_all(codex_sentinel_path.parent().unwrap()).unwrap();
    fs::write(
        &codex_sentinel_path,
        br#"{"access_token":"codex-sentinel"}"#,
    )
    .unwrap();

    let store = AuthStore::new(owned_auth_path.clone());
    let auth = PersistedAuth::sample_valid_for_tests("acct_123");
    store.save(&auth).unwrap();
    assert!(owned_auth_path.exists());

    store.clear().unwrap();

    assert!(!owned_auth_path.exists());
    assert_eq!(
        fs::read_to_string(&codex_sentinel_path).unwrap(),
        r#"{"access_token":"codex-sentinel"}"#
    );
}
