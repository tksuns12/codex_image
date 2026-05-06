use codex_image::config::ConfigError;
use codex_image::diagnostics::{CliError, ExitCode};

fn parse_envelope(err: &CliError) -> serde_json::Value {
    serde_json::to_value(err.error_envelope()).expect("error envelope serializes")
}

fn assert_error_contract_shape(json: &serde_json::Value) {
    let root = json
        .as_object()
        .expect("error envelope root should be an object");
    assert_eq!(
        root.len(),
        1,
        "error envelope root must only contain `error`"
    );

    let error = root
        .get("error")
        .and_then(serde_json::Value::as_object)
        .expect("error envelope `error` field should be an object");
    assert_eq!(
        error.len(),
        4,
        "error object must only contain code/message/recoverable/hint"
    );

    assert!(error
        .get("code")
        .and_then(serde_json::Value::as_str)
        .is_some());
    assert!(error
        .get("message")
        .and_then(serde_json::Value::as_str)
        .is_some());
    assert!(error
        .get("recoverable")
        .and_then(serde_json::Value::as_bool)
        .is_some());
    assert!(error
        .get("hint")
        .and_then(serde_json::Value::as_str)
        .is_some());
}

#[test]
fn diagnostics_config_error_maps_to_usage_config_exit_and_shape() {
    let err = CliError::Config(ConfigError::InvalidValue {
        key: "CODEX_IMAGE_CODEX_BIN",
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
    assert!(!rendered.contains("CODEX_IMAGE_CODEX_BIN"));
}

#[test]
fn diagnostics_output_write_or_verify_failures_map_to_filesystem_exit() {
    let write_err = CliError::OutputWriteFailed;
    let write_json = parse_envelope(&write_err);
    assert_eq!(write_err.exit_code(), ExitCode::Filesystem);
    assert_eq!(
        write_json["error"]["code"],
        "filesystem.output_write_failed"
    );
    assert_eq!(
        write_json["error"]["message"],
        "failed to write generated image output"
    );

    let verify_err = CliError::OutputVerificationFailed;
    let verify_json = parse_envelope(&verify_err);
    assert_eq!(verify_err.exit_code(), ExitCode::Filesystem);
    assert_eq!(
        verify_json["error"]["code"],
        "filesystem.output_write_failed"
    );
}

#[test]
fn diagnostics_image_response_contract_maps_to_response_contract_exit() {
    let err = CliError::ImageGenerationResponseContract {
        source_message: "unexpected generated image path".to_string(),
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
    assert!(!rendered.contains("unexpected generated image path"));
}

#[test]
fn diagnostics_codex_backend_errors_are_redacted_and_actionable() {
    let unavailable = CliError::CodexCliUnavailable;
    assert_eq!(unavailable.exit_code(), ExitCode::UsageOrConfig);
    let unavailable_json = parse_envelope(&unavailable);
    assert_eq!(
        unavailable_json["error"]["code"],
        "config.codex_cli_unavailable"
    );
    assert_eq!(
        unavailable_json["error"]["hint"],
        "Install Codex or set CODEX_IMAGE_CODEX_BIN to the Codex executable."
    );

    let failed = CliError::CodexImageGenerationFailed {
        source_message: "Bearer sk-secret raw codex output".to_string(),
    };
    assert_eq!(failed.exit_code(), ExitCode::Api);
    let failed_json = parse_envelope(&failed);
    assert_eq!(
        failed_json["error"]["code"],
        "api.codex_image_generation_failed"
    );

    let rendered = serde_json::to_string(&failed_json).unwrap();
    assert!(!rendered.contains("Bearer"));
    assert!(!rendered.contains("sk-secret"));
    assert!(!rendered.contains("raw codex output"));
}

#[test]
fn diagnostics_skill_install_confirmation_home_and_write_failures_are_redacted() {
    let missing_yes = CliError::MissingInstallConfirmation;
    assert_eq!(missing_yes.exit_code(), ExitCode::UsageOrConfig);
    let missing_yes_json = parse_envelope(&missing_yes);
    assert_eq!(
        missing_yes_json["error"]["code"],
        "usage.install_confirmation_required"
    );

    let home_missing = CliError::HomeUnavailable;
    assert_eq!(home_missing.exit_code(), ExitCode::UsageOrConfig);
    let home_missing_json = parse_envelope(&home_missing);
    assert_eq!(
        home_missing_json["error"]["code"],
        "config.home_unavailable"
    );

    let write_failed = CliError::SkillInstallWriteFailed;
    assert_eq!(write_failed.exit_code(), ExitCode::Filesystem);
    let write_failed_json = parse_envelope(&write_failed);
    assert_eq!(
        write_failed_json["error"]["code"],
        "filesystem.skill_install_write_failed"
    );

    let blocked = CliError::SkillInstallBlockedManualEdit;
    assert_eq!(blocked.exit_code(), ExitCode::Filesystem);
    let blocked_json = parse_envelope(&blocked);
    assert_eq!(
        blocked_json["error"]["code"],
        "filesystem.skill_install_blocked_manual_edit"
    );

    let rendered = serde_json::to_string(&blocked_json).unwrap();
    assert!(!rendered.contains("/tmp/"));
    assert!(!rendered.contains("HOME"));
    assert!(!rendered.contains("Bearer"));
}

#[test]
fn diagnostics_unknown_fallback_is_stable() {
    let err = CliError::Unknown;

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
fn diagnostics_exit_code_taxonomy_is_stable() {
    assert_eq!(ExitCode::Unknown.as_i32(), 1);
    assert_eq!(ExitCode::UsageOrConfig.as_i32(), 2);
    assert_eq!(ExitCode::Api.as_i32(), 4);
    assert_eq!(ExitCode::Filesystem.as_i32(), 5);
    assert_eq!(ExitCode::ResponseContract.as_i32(), 6);
}

#[test]
fn diagnostics_all_error_envelopes_keep_exact_machine_readable_shape() {
    let cases = [
        CliError::Config(ConfigError::InvalidValue {
            key: "CODEX_IMAGE_CODEX_BIN",
        }),
        CliError::OutputWriteFailed,
        CliError::OutputVerificationFailed,
        CliError::ImageGenerationResponseContract {
            source_message: "generated path".to_string(),
        },
        CliError::CodexCliUnavailable,
        CliError::CodexImageGenerationFailed {
            source_message: "codex failed".to_string(),
        },
        CliError::MissingInstallConfirmation,
        CliError::HomeUnavailable,
        CliError::ProjectRootUnavailable,
        CliError::SkillInstallWriteFailed,
        CliError::SkillInstallBlockedManualEdit,
        CliError::Unknown,
    ];

    for err in cases {
        let json = parse_envelope(&err);
        assert_error_contract_shape(&json);
    }
}
