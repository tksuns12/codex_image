use std::fs;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

fn parse_json_line(bytes: Vec<u8>) -> Value {
    let text = String::from_utf8(bytes).expect("output should be utf-8");
    let trimmed = text.trim_end();
    assert_eq!(trimmed.lines().count(), 1, "output must be one JSON line");
    serde_json::from_str(trimmed).expect("output should be valid json")
}

#[test]
fn skill_install_cli_project_install_then_repeat_is_created_then_unchanged() {
    let project = tempdir().expect("project tempdir");
    let home = tempdir().expect("home tempdir");

    let mut first = Command::cargo_bin("codex-image").expect("binary exists");
    let first_output = first
        .current_dir(project.path())
        .arg("skill")
        .arg("install")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("project")
        .arg("--yes")
        .env("HOME", home.path())
        .output()
        .expect("first install runs");

    assert!(
        first_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first_output.stderr)
    );
    assert!(first_output.stderr.is_empty());

    let first_json = parse_json_line(first_output.stdout);
    assert_eq!(first_json["tool"], "pi");
    assert_eq!(first_json["scope"], "project");
    assert_eq!(first_json["status"], "created");

    let expected_path = project
        .path()
        .join(".agents")
        .join("skills")
        .join("codex-image")
        .join("SKILL.md");
    assert_eq!(
        first_json["target_path"].as_str(),
        Some(expected_path.to_string_lossy().as_ref())
    );

    let written = fs::read_to_string(&expected_path).expect("skill file should be written");
    assert!(written.starts_with("<!-- codex-image:managed checksum="));

    let mut second = Command::cargo_bin("codex-image").expect("binary exists");
    let second_output = second
        .current_dir(project.path())
        .arg("skill")
        .arg("install")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("project")
        .arg("--yes")
        .env("HOME", home.path())
        .output()
        .expect("second install runs");

    assert!(second_output.status.success());
    let second_json = parse_json_line(second_output.stdout);
    assert_eq!(second_json["status"], "unchanged");
}

#[test]
fn skill_install_cli_blocks_manual_edit_by_default_and_force_overwrites() {
    let project = tempdir().expect("project tempdir");
    let home = tempdir().expect("home tempdir");
    let target = project
        .path()
        .join(".agents")
        .join("skills")
        .join("codex-image")
        .join("SKILL.md");
    fs::create_dir_all(target.parent().expect("target parent")).expect("create target parent");

    let manual_content = "# custom skill\nBearer not-for-output\n";
    fs::write(&target, manual_content).expect("seed manual skill");

    let mut blocked = Command::cargo_bin("codex-image").expect("binary exists");
    let blocked_output = blocked
        .current_dir(project.path())
        .arg("skill")
        .arg("install")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("project")
        .arg("--yes")
        .env("HOME", home.path())
        .output()
        .expect("blocked install runs");

    assert_eq!(blocked_output.status.code(), Some(5));
    assert!(blocked_output.stdout.is_empty());

    let blocked_json = parse_json_line(blocked_output.stderr);
    assert_eq!(
        blocked_json["error"]["code"],
        "filesystem.skill_install_blocked_manual_edit"
    );

    let blocked_stderr = serde_json::to_string(&blocked_json).expect("json serializes");
    assert!(!blocked_stderr.contains("Bearer"));

    let preserved = fs::read_to_string(&target).expect("manual file should be preserved");
    assert_eq!(preserved, manual_content);

    let mut forced = Command::cargo_bin("codex-image").expect("binary exists");
    let forced_output = forced
        .current_dir(project.path())
        .arg("skill")
        .arg("install")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("project")
        .arg("--yes")
        .arg("--force")
        .env("HOME", home.path())
        .output()
        .expect("forced install runs");

    assert!(forced_output.status.success());
    let forced_json = parse_json_line(forced_output.stdout);
    assert_eq!(forced_json["status"], "forced_overwrite");

    let overwritten = fs::read_to_string(&target).expect("manual file should be overwritten");
    assert!(overwritten.starts_with("<!-- codex-image:managed checksum="));
    assert!(!overwritten.contains("Bearer not-for-output"));
}

#[test]
fn skill_install_cli_missing_yes_and_missing_home_emit_redacted_errors() {
    let project = tempdir().expect("project tempdir");

    let target = project
        .path()
        .join(".agents")
        .join("skills")
        .join("codex-image")
        .join("SKILL.md");

    let mut missing_yes = Command::cargo_bin("codex-image").expect("binary exists");
    let missing_yes_output = missing_yes
        .current_dir(project.path())
        .arg("skill")
        .arg("install")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("project")
        .output()
        .expect("missing yes command runs");

    assert_eq!(missing_yes_output.status.code(), Some(2));
    assert!(missing_yes_output.stdout.is_empty());
    let missing_yes_json = parse_json_line(missing_yes_output.stderr);
    assert_eq!(
        missing_yes_json["error"]["code"],
        "usage.install_confirmation_required"
    );
    assert!(
        !target.exists(),
        "filesystem writes must not happen without --yes"
    );

    let mut missing_home = Command::cargo_bin("codex-image").expect("binary exists");
    let missing_home_output = missing_home
        .current_dir(project.path())
        .arg("skill")
        .arg("install")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("global")
        .arg("--yes")
        .env_remove("HOME")
        .output()
        .expect("missing home command runs");

    assert_eq!(missing_home_output.status.code(), Some(2));
    assert!(missing_home_output.stdout.is_empty());

    let missing_home_json = parse_json_line(missing_home_output.stderr);
    assert_eq!(
        missing_home_json["error"]["code"],
        "config.home_unavailable"
    );

    let rendered = serde_json::to_string(&missing_home_json).expect("json serializes");
    assert!(!rendered.contains(project.path().to_string_lossy().as_ref()));
    assert!(!rendered.contains("HOME="));
}

#[test]
fn skill_install_cli_global_scope_writes_under_home_directory() {
    let project = tempdir().expect("project tempdir");
    let home = tempdir().expect("home tempdir");

    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    let output = cmd
        .current_dir(project.path())
        .arg("skill")
        .arg("install")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("global")
        .arg("--yes")
        .env("HOME", home.path())
        .output()
        .expect("global install runs");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let json = parse_json_line(output.stdout);
    assert_eq!(json["scope"], "global");
    assert_eq!(json["status"], "created");

    let expected_path = home
        .path()
        .join(".agents")
        .join("skills")
        .join("codex-image")
        .join("SKILL.md");
    assert_eq!(
        json["target_path"].as_str(),
        Some(expected_path.to_string_lossy().as_ref())
    );
    assert!(expected_path.is_file());
}
