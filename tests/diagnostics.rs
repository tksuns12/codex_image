use codex_image::auth::device::DeviceLoginError;
use codex_image::auth::store::StoreError;
use codex_image::config::ConfigError;
use codex_image::diagnostics::{CliError, ExitCode};

fn parse_envelope(err: &CliError) -> serde_json::Value {
    serde_json::to_value(err.error_envelope()).expect("error envelope serializes")
}

fn assert_error_contract_shape(json: &serde_json::Value) {
    let root = json
        .as_object()
        .expect("error envelope root should be an object");
    assert_eq!(root.len(), 1, "error envelope root must only contain `error`");

    let error = root
        .get("error")
        .and_then(serde_json::Value::as_object)
        .expect("error envelope `error` field should be an object");
    assert_eq!(
        error.len(),
        4,
        "error object must only contain code/message/recoverable/hint"
    );

    assert!(
        error
            .get("code")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "error.code must be a string"
    );
    assert!(
        error
            .get("message")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "error.message must be a string"
    );
    assert!(
        error
            .get("recoverable")
            .and_then(serde_json::Value::as_bool)
            .is_some(),
        "error.recoverable must be a bool"
    );
    assert!(
        error
            .get("hint")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "error.hint must be a string"
    );
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
    assert_eq!(
        json["error"]["hint"],
        "Check CODEX_IMAGE_* configuration values."
    );

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
    assert_eq!(
        json["error"]["hint"],
        "Run `codex-image login` to refresh local auth state."
    );
}

#[test]
fn diagnostics_store_persist_error_maps_to_filesystem() {
    let err = CliError::AuthStore(StoreError::Persist);

    assert_eq!(err.exit_code(), ExitCode::Filesystem);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "filesystem.auth_write_failed");
    assert_eq!(json["error"]["message"], "auth state error");
    assert_eq!(json["error"]["recoverable"], true);
    assert_eq!(
        json["error"]["hint"],
        "Ensure the auth directory is writable and retry."
    );
}

#[test]
fn diagnostics_device_timeout_maps_to_api_recoverable() {
    let err = CliError::DeviceLogin(DeviceLoginError::PollTimeout);

    assert_eq!(err.exit_code(), ExitCode::Api);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "api.auth_timeout");
    assert_eq!(
        json["error"]["message"],
        "authentication service request failed"
    );
    assert_eq!(json["error"]["recoverable"], true);
    assert_eq!(json["error"]["hint"], "Retry login in a moment.");
}

#[test]
fn diagnostics_device_contract_error_maps_to_response_contract_without_raw_body() {
    let err = CliError::DeviceLogin(DeviceLoginError::TokenExchangeContract);

    assert_eq!(err.exit_code(), ExitCode::ResponseContract);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "response_contract.oauth_token");
    assert_eq!(
        json["error"]["message"],
        "authentication service response did not match expected schema"
    );
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
fn diagnostics_image_api_failure_maps_to_api_exit_and_redacts_source() {
    let err = CliError::ImageGenerationApi {
        source_message: "Bearer sk-test-secret raw upstream body".to_string(),
    };

    assert_eq!(err.exit_code(), ExitCode::Api);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "api.image_generation_failed");
    assert_eq!(json["error"]["message"], "image generation request failed");
    assert_eq!(json["error"]["recoverable"], true);
    assert_eq!(json["error"]["hint"], "Retry image generation in a moment.");

    let rendered = serde_json::to_string(&json).unwrap();
    assert!(!rendered.contains("Bearer"));
    assert!(!rendered.contains("sk-test-secret"));
    assert!(!rendered.contains("raw upstream body"));
}

#[test]
fn diagnostics_image_timeout_maps_to_api_exit() {
    let err = CliError::ImageGenerationTimeout {
        source_message: "timeout waiting for api.openai.com".to_string(),
    };

    assert_eq!(err.exit_code(), ExitCode::Api);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "api.image_generation_failed");
    assert_eq!(json["error"]["message"], "image generation request failed");
    assert_eq!(json["error"]["recoverable"], true);
    assert_eq!(json["error"]["hint"], "Retry image generation in a moment.");
}

#[test]
fn diagnostics_output_write_or_verify_failures_map_to_filesystem_exit() {
    let write_err = CliError::OutputWriteFailed;
    let write_json = parse_envelope(&write_err);
    assert_eq!(write_err.exit_code(), ExitCode::Filesystem);
    assert_eq!(write_json["error"]["code"], "filesystem.output_write_failed");
    assert_eq!(write_json["error"]["message"], "failed to write generated image output");

    let verify_err = CliError::OutputVerificationFailed;
    let verify_json = parse_envelope(&verify_err);
    assert_eq!(verify_err.exit_code(), ExitCode::Filesystem);
    assert_eq!(verify_json["error"]["code"], "filesystem.output_write_failed");
    assert_eq!(verify_json["error"]["message"], "failed to write generated image output");
}

#[test]
fn diagnostics_image_response_contract_maps_to_response_contract_exit() {
    let err = CliError::ImageGenerationResponseContract {
        source_message: "unexpected b64_json length mismatch".to_string(),
    };

    assert_eq!(err.exit_code(), ExitCode::ResponseContract);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "response_contract.image_generation");
    assert_eq!(
        json["error"]["message"],
        "image generation response did not match expected schema"
    );
    assert_eq!(json["error"]["recoverable"], false);
    assert_eq!(
        json["error"]["hint"],
        "Try again; if it persists, report the issue with request context."
    );

    let rendered = serde_json::to_string(&json).unwrap();
    assert!(!rendered.contains("b64_json"));
}

#[test]
fn diagnostics_unknown_fallback_is_stable() {
    let err = CliError::LoginNotImplemented;

    assert_eq!(err.exit_code(), ExitCode::Unknown);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "unknown");
    assert_eq!(json["error"]["message"], "unexpected failure");
    assert_eq!(json["error"]["recoverable"], false);
    assert_eq!(
        json["error"]["hint"],
        "Re-run with supported commands or update the binary."
    );
}

#[test]
fn diagnostics_auth_not_logged_in_is_auth_domain_recoverable() {
    let err = CliError::AuthNotLoggedIn;

    assert_eq!(err.exit_code(), ExitCode::Auth);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "auth.not_logged_in");
    assert_eq!(json["error"]["message"], "not logged in");
    assert_eq!(json["error"]["recoverable"], true);
    assert_eq!(
        json["error"]["hint"],
        "Run `codex-image login` to authenticate."
    );
}

#[test]
fn diagnostics_auth_expired_is_auth_domain_recoverable() {
    let err = CliError::AuthExpired;

    assert_eq!(err.exit_code(), ExitCode::Auth);

    let json = parse_envelope(&err);
    assert_eq!(json["error"]["code"], "auth.expired");
    assert_eq!(json["error"]["message"], "auth access token expired");
    assert_eq!(json["error"]["recoverable"], true);
    assert_eq!(
        json["error"]["hint"],
        "Run `codex-image login` to refresh local auth state."
    );
}

#[test]
fn diagnostics_exit_code_taxonomy_is_stable() {
    assert_eq!(ExitCode::Unknown.as_i32(), 1);
    assert_eq!(ExitCode::UsageOrConfig.as_i32(), 2);
    assert_eq!(ExitCode::Auth.as_i32(), 3);
    assert_eq!(ExitCode::Api.as_i32(), 4);
    assert_eq!(ExitCode::Filesystem.as_i32(), 5);
    assert_eq!(ExitCode::ResponseContract.as_i32(), 6);
}

#[test]
fn diagnostics_all_error_envelopes_keep_exact_machine_readable_shape() {
    let cases = [
        CliError::Config(ConfigError::InvalidValue {
            key: "CODEX_IMAGE_CLIENT_ID",
        }),
        CliError::AuthStore(StoreError::Read),
        CliError::AuthNotLoggedIn,
        CliError::AuthExpired,
        CliError::AuthInvalidState,
        CliError::DeviceLogin(DeviceLoginError::UserCodeApi),
        CliError::DeviceLogin(DeviceLoginError::TokenExchangeContract),
        CliError::ImageGenerationApi {
            source_message: "Bearer sk-ignored".to_string(),
        },
        CliError::ImageGenerationTimeout {
            source_message: "timeout".to_string(),
        },
        CliError::OutputWriteFailed,
        CliError::OutputVerificationFailed,
        CliError::ImageGenerationResponseContract {
            source_message: "b64_json".to_string(),
        },
        CliError::LoginNotImplemented,
    ];

    for err in cases {
        let json = parse_envelope(&err);
        assert_error_contract_shape(&json);
    }
}
