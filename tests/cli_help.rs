use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_help_binary_exists_as_codex_image() {
    Command::cargo_bin("codex-image").expect("codex-image binary should compile");
}

#[test]
fn cli_help_login_help_parses() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("login").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Login"));
}

#[test]
fn cli_help_status_help_documents_json_flag() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("status").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--json"));
}

#[test]
fn cli_help_status_without_json_flag_is_usage_error() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("status");

    cmd.assert()
        .code(2)
        .stderr(predicate::str::contains("--json"))
        .stderr(predicate::str::contains("\"error\":").not());
}

#[test]
fn cli_help_generate_help_documents_required_out_and_prompt_contract() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("generate").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("prompt"));
}

#[test]
fn cli_help_generate_without_out_is_usage_error_without_json_envelope() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("generate").arg("prompt only");

    cmd.assert()
        .code(2)
        .stderr(predicate::str::contains("--out"))
        .stderr(predicate::str::contains("\"error\":").not());
}

#[test]
fn cli_help_generate_without_prompt_is_usage_error_without_json_envelope() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("generate").arg("--out").arg("./images");

    cmd.assert()
        .code(2)
        .stderr(predicate::str::contains("<PROMPT>").or(predicate::str::contains("<prompt>")))
        .stderr(predicate::str::contains("\"error\":").not());
}

#[test]
fn cli_help_logout_command_is_available() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("logout").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("logout").or(predicate::str::contains("Logout")));
}

#[test]
fn cli_help_unknown_subcommand_returns_clap_error_not_panic() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("not-a-command");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage").or(predicate::str::contains("USAGE")))
        .stderr(predicate::str::contains("unrecognized subcommand"))
        .stderr(predicate::str::contains("panic").not());
}

#[test]
fn cli_help_non_clap_dispatch_failures_emit_single_json_envelope_with_mapped_exit_code() {
    let token_like_secret = "access-token-should-never-leak";
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("login")
        .env(
            "CODEX_IMAGE_AUTH_BASE_URL",
            format!("::invalid-{token_like_secret}::"),
        )
        .env("CODEX_IMAGE_CLIENT_ID", "codex-image");

    let output = cmd.output().expect("login command should run");

    assert_eq!(output.status.code(), Some(2));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8 JSON");
    let stderr_trimmed = stderr.trim_end();
    assert_eq!(
        stderr_trimmed.lines().count(),
        1,
        "dispatch failures should emit exactly one JSON envelope line"
    );

    let envelope: serde_json::Value =
        serde_json::from_str(stderr_trimmed).expect("stderr should be json envelope");
    assert_eq!(envelope["error"]["code"], "config.invalid");
    assert_eq!(envelope["error"]["message"], "configuration error");
    assert_eq!(envelope["error"]["recoverable"], true);
    assert_eq!(
        envelope["error"]["hint"],
        "Check CODEX_IMAGE_* configuration values."
    );

    assert!(!stderr.contains(token_like_secret));
    assert!(!stderr.contains("CODEX_IMAGE_AUTH_BASE_URL"));
}
