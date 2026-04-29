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
fn cli_help_unknown_subcommand_returns_clap_error_not_panic() {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.arg("not-a-command");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage").or(predicate::str::contains("USAGE")))
        .stderr(predicate::str::contains("unrecognized subcommand"))
        .stderr(predicate::str::contains("panic").not());
}
