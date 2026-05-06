use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_help_binary_exists_as_codex_image() {
    Command::cargo_bin("codex-image").expect("codex-image binary should compile");
}

#[test]
fn cli_help_generate_help_documents_required_out_and_prompt_contract() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("generate").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("prompt"))
        .stdout(predicate::str::contains("Codex"));
}

#[test]
fn cli_help_skill_install_help_documents_non_interactive_and_interactive_modes() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("skill").arg("install").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--tool <TOOL>").or(predicate::str::contains("--tool <tool>")))
        .stdout(predicate::str::contains("--scope <SCOPE>").or(predicate::str::contains("--scope <scope>")))
        .stdout(predicate::str::contains("May be repeated"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("non-interactive"))
        .stdout(predicate::str::contains("interactive"))
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("claude-code"))
        .stdout(predicate::str::contains("opencode"));
}

#[test]
fn cli_help_removed_auth_lifecycle_commands() {
    for command in ["login", "status", "logout"] {
        let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
        cmd.arg(command).arg("--help");

        cmd.assert()
            .code(2)
            .stderr(predicate::str::contains("unrecognized subcommand"))
            .stderr(predicate::str::contains("panic").not());
    }
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
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("generate")
        .arg("prompt")
        .arg("--out")
        .arg("./images")
        .env("CODEX_IMAGE_CODEX_BIN", "   ");

    let output = cmd.output().expect("generate command should run");

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

    assert!(!stderr.contains("CODEX_IMAGE_CODEX_BIN"));
}
