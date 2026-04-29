use codex_image::auth::device::DeviceLoginError;
use codex_image::auth::store::StoreError;
use codex_image::config::ConfigError;
use codex_image::diagnostics::{CliError, ExitCode};

fn parse_envelope(err: &CliError) -> serde_json::Value {
    serde_json::to_value(err.error_envelope()).expect("error envelope serializes")
}

#[test]
fn diagnostics_config_error_maps_to_usage_config_exit_and_shape() {
    let err = CliError::Config(ConfigError::InvalidValue {
        key: "CODEX_IMAGE_AUTH_BASE_URL",
    });

    assert_eq!(err.exit_code(), ExitCode::UsageOrConfig);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "config.invalid");
    assert_eq!(json["error"]["message"], "configuration error");
    assert_eq!(json["error"]["recoverable"], true);
    assert_eq!(json["error"]["hint"], "Check CODEX_IMAGE_* configuration values.");

    let rendered = serde_json::to_string(&json).unwrap();
    assert!(!rendered.contains("CODEX_IMAGE_AUTH_BASE_URL"));
}

#[test]
fn diagnostics_store_parse_error_is_redacted_auth_domain() {
    let err = CliError::AuthStore(StoreError::Parse);

    assert_eq!(err.exit_code(), ExitCode::Auth);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "auth.invalid_state");
    assert_eq!(json["error"]["message"], "auth state error");
    assert_eq!(json["error"]["recoverable"], false);
    assert_eq!(json["error"]["hint"], "Run `codex-image login` to refresh local auth state.");
}

#[test]
fn diagnostics_store_persist_error_maps_to_filesystem() {
    let err = CliError::AuthStore(StoreError::Persist);

    assert_eq!(err.exit_code(), ExitCode::Filesystem);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "filesystem.auth_write_failed");
    assert_eq!(json["error"]["message"], "auth state error");
    assert_eq!(json["error"]["recoverable"], true);
    assert_eq!(json["error"]["hint"], "Ensure the auth directory is writable and retry.");
}

#[test]
fn diagnostics_device_timeout_maps_to_api_recoverable() {
    let err = CliError::DeviceLogin(DeviceLoginError::PollTimeout);

    assert_eq!(err.exit_code(), ExitCode::Api);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "api.auth_timeout");
    assert_eq!(json["error"]["message"], "authentication service request failed");
    assert_eq!(json["error"]["recoverable"], true);
    assert_eq!(json["error"]["hint"], "Retry login in a moment.");
}

#[test]
fn diagnostics_device_contract_error_maps_to_response_contract_without_raw_body() {
    let err = CliError::DeviceLogin(DeviceLoginError::TokenExchangeContract);

    assert_eq!(err.exit_code(), ExitCode::ResponseContract);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "response_contract.oauth_token");
    assert_eq!(json["error"]["message"], "authentication service response did not match expected schema");
    assert_eq!(json["error"]["recoverable"], false);
    assert_eq!(
        json["error"]["hint"],
        "Try logging in again; if it persists, report the issue with request context."
    );

    let rendered = serde_json::to_string(&json).unwrap();
    assert!(!rendered.contains("access_token"));
    assert!(!rendered.contains("refresh_token"));
    assert!(!rendered.contains("authorization_code"));
}

#[test]
fn diagnostics_unknown_fallback_is_stable() {
    let err = CliError::LoginNotImplemented;

    assert_eq!(err.exit_code(), ExitCode::Unknown);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "unknown");
    assert_eq!(json["error"]["message"], "unexpected failure");
    assert_eq!(json["error"]["recoverable"], false);
    assert_eq!(json["error"]["hint"], "Re-run with supported commands or update the binary.");
}
