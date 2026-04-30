use std::sync::{Mutex, OnceLock};

use assert_cmd::Command;
use codex_image::config::{ConfigError, GenerateConfig, ENV_API_BASE_URL};
use codex_image::diagnostics::{CliError, ExitCode};
use tempfile::TempDir;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn generate_config_invalid_api_base_url_maps_to_config_error_and_exit_2() {
    let _guard = env_lock();
    std::env::set_var(ENV_API_BASE_URL, "::not-a-url::");

    let err = GenerateConfig::from_env().expect_err("invalid URL should fail");

    std::env::remove_var(ENV_API_BASE_URL);

    assert!(matches!(
        err,
        ConfigError::InvalidValue {
            key: ENV_API_BASE_URL
        }
    ));

    let cli_err = CliError::Config(err);
    assert_eq!(cli_err.exit_code(), ExitCode::UsageOrConfig);
    assert_eq!(cli_err.exit_code().as_i32(), 2);

    let json = serde_json::to_value(cli_err.error_envelope()).unwrap();
    assert_eq!(json["error"]["code"], "config.invalid");

    let rendered = serde_json::to_string(&json).unwrap();
    assert!(!rendered.contains("::not-a-url::"));
}

#[test]
fn status_json_ignores_invalid_api_base_url_store_path() {
    let temp = TempDir::new().unwrap();

    let output = Command::cargo_bin("codex-image")
        .unwrap()
        .arg("status")
        .arg("--json")
        .env("CODEX_IMAGE_HOME", temp.path())
        .env("CODEX_IMAGE_API_BASE_URL", "::not-a-url::")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["status"], "not_logged_in");
}
