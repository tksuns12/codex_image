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

fn parse_json_lines(bytes: Vec<u8>) -> Vec<Value> {
    let text = String::from_utf8(bytes).expect("output should be utf-8");
    let trimmed = text.trim_end();
    assert!(!trimmed.is_empty(), "output must not be empty");

    trimmed
        .lines()
        .map(|line| serde_json::from_str(line).expect("each output line should be valid json"))
        .collect()
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
fn skill_install_cli_multi_target_repeated_flags_emit_deterministic_json_lines() {
    let project = tempdir().expect("project tempdir");
    let home = tempdir().expect("home tempdir");

    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    let output = cmd
        .current_dir(project.path())
        .arg("skill")
        .arg("install")
        .arg("--tool")
        .arg("pi")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("global")
        .arg("--scope")
        .arg("project")
        .arg("--scope")
        .arg("global")
        .arg("--yes")
        .env("HOME", home.path())
        .output()
        .expect("multi-target install runs");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());

    let lines = parse_json_lines(output.stdout);
    assert_eq!(lines.len(), 2, "deduped tool/scope should produce 2 targets");

    assert_eq!(lines[0]["tool"], "pi");
    assert_eq!(lines[0]["scope"], "global");
    assert_eq!(lines[0]["status"], "created");

    let expected_global = home
        .path()
        .join(".agents")
        .join("skills")
        .join("codex-image")
        .join("SKILL.md");
    assert_eq!(
        lines[0]["target_path"].as_str(),
        Some(expected_global.to_string_lossy().as_ref())
    );

    assert_eq!(lines[1]["tool"], "pi");
    assert_eq!(lines[1]["scope"], "project");
    assert_eq!(lines[1]["status"], "created");

    let expected_project = project
        .path()
        .join(".agents")
        .join("skills")
        .join("codex-image")
        .join("SKILL.md");
    assert_eq!(
        lines[1]["target_path"].as_str(),
        Some(expected_project.to_string_lossy().as_ref())
    );

    assert!(expected_global.is_file());
    assert!(expected_project.is_file());
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
fn skill_install_cli_partial_flags_and_no_target_non_tty_fail_without_writes() {
    let project = tempdir().expect("project tempdir");
    let home = tempdir().expect("home tempdir");

    let target = project
        .path()
        .join(".agents")
        .join("skills")
        .join("codex-image")
        .join("SKILL.md");

    let mut partial = Command::cargo_bin("codex-image").expect("binary exists");
    let partial_output = partial
        .current_dir(project.path())
        .arg("skill")
        .arg("install")
        .arg("--tool")
        .arg("pi")
        .arg("--yes")
        .env("HOME", home.path())
        .output()
        .expect("partial command runs");

    assert_eq!(partial_output.status.code(), Some(2));
    assert!(partial_output.stdout.is_empty());

    let partial_json = parse_json_line(partial_output.stderr);
    assert_eq!(
        partial_json["error"]["code"],
        "usage.install_partial_target_selection"
    );
    assert!(
        !target.exists(),
        "filesystem writes must not happen on partial target selection"
    );

    let mut none_selected = Command::cargo_bin("codex-image").expect("binary exists");
    let none_selected_output = none_selected
        .current_dir(project.path())
        .arg("skill")
        .arg("install")
        .env("HOME", home.path())
        .output()
        .expect("no target selection command runs");

    assert_eq!(none_selected_output.status.code(), Some(2));
    assert!(none_selected_output.stdout.is_empty());

    let none_selected_json = parse_json_line(none_selected_output.stderr);
    assert_eq!(
        none_selected_json["error"]["code"],
        "usage.install_no_targets_non_interactive"
    );
    assert!(
        !target.exists(),
        "filesystem writes must not happen when no non-interactive target flags are selected"
    );
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
