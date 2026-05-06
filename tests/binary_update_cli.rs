use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

fn parse_json_line(bytes: Vec<u8>) -> Value {
    let text = String::from_utf8(bytes).expect("output should be utf-8");
    let trimmed = text.trim_end();
    assert_eq!(trimmed.lines().count(), 1, "output must be one JSON line");
    serde_json::from_str(trimmed).expect("output should be valid json")
}

#[test]
fn update_cli_missing_yes_fails_before_network_with_redacted_json_envelope() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    let output = cmd.arg("update").output().expect("update command runs");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "stdout should stay empty on failure"
    );

    let envelope = parse_json_line(output.stderr);
    assert_eq!(
        envelope["error"]["code"],
        "usage.update_confirmation_required"
    );
    assert_eq!(
        envelope["error"]["message"],
        "binary update requires --yes confirmation"
    );
    assert_eq!(envelope["error"]["recoverable"], true);
    assert_eq!(
        envelope["error"]["hint"],
        "Re-run with --yes, or use --dry-run to validate without replacement."
    );

    let rendered = serde_json::to_string(&envelope).expect("json serializes");
    assert!(!rendered.contains("https://"));
    assert!(!rendered.contains("Bearer"));
    assert!(!rendered.contains("HOME="));
    assert!(!rendered.contains("/tmp/"));
}

#[test]
fn update_cli_invalid_version_uses_clap_stderr_not_json_envelope() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("update")
        .arg("--dry-run")
        .arg("--version")
        .arg("1.2.3");

    cmd.assert()
        .code(2)
        .stderr(predicate::str::contains("version tag must start with 'v'"))
        .stderr(predicate::str::contains("\"error\":").not());
}
