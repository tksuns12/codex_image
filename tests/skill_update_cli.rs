use std::fs;

use assert_cmd::Command;
use codex_image::skill_installer::{
    managed_marker_line, render_managed_skill_content, render_skill_body,
};
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
fn skill_update_cli_project_update_missing_then_repeat_is_created_then_unchanged() {
    let project = tempdir().expect("project tempdir");
    let home = tempdir().expect("home tempdir");

    let mut first = Command::cargo_bin("codex-image").expect("binary exists");
    let first_output = first
        .current_dir(project.path())
        .arg("skill")
        .arg("update")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("project")
        .arg("--yes")
        .env("HOME", home.path())
        .output()
        .expect("first update runs");

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

    let mut second = Command::cargo_bin("codex-image").expect("binary exists");
    let second_output = second
        .current_dir(project.path())
        .arg("skill")
        .arg("update")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("project")
        .arg("--yes")
        .env("HOME", home.path())
        .output()
        .expect("second update runs");

    assert!(second_output.status.success());
    assert!(second_output.stderr.is_empty());

    let second_json = parse_json_line(second_output.stdout);
    assert_eq!(second_json["status"], "unchanged");
}

#[test]
fn skill_update_cli_updates_valid_outdated_managed_file_to_current_content() {
    let project = tempdir().expect("project tempdir");
    let home = tempdir().expect("home tempdir");
    let target = project
        .path()
        .join(".agents")
        .join("skills")
        .join("codex-image")
        .join("SKILL.md");

    fs::create_dir_all(target.parent().expect("target parent")).expect("create target parent");

    let outdated_body = render_skill_body().replace("## Guardrails", "## Guardrails (old)");
    let outdated_content = format!("{}\n{}", managed_marker_line(&outdated_body), outdated_body);
    fs::write(&target, outdated_content).expect("seed outdated managed content");

    let mut update = Command::cargo_bin("codex-image").expect("binary exists");
    let update_output = update
        .current_dir(project.path())
        .arg("skill")
        .arg("update")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("project")
        .arg("--yes")
        .env("HOME", home.path())
        .output()
        .expect("update runs");

    assert!(
        update_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&update_output.stderr)
    );
    assert!(update_output.stderr.is_empty());

    let update_json = parse_json_line(update_output.stdout);
    assert_eq!(update_json["status"], "updated");

    let current = fs::read_to_string(&target).expect("updated target should be readable");
    assert_eq!(current, render_managed_skill_content());
}

#[test]
fn skill_update_cli_blocks_manual_edit_without_force_and_redacts_stderr() {
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
        .arg("update")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("project")
        .arg("--yes")
        .env("HOME", home.path())
        .output()
        .expect("blocked update runs");

    assert_eq!(blocked_output.status.code(), Some(5));
    assert!(blocked_output.stdout.is_empty());

    let blocked_json = parse_json_line(blocked_output.stderr);
    assert_eq!(
        blocked_json["error"]["code"],
        "filesystem.skill_update_blocked_manual_edit"
    );

    let blocked_stderr = serde_json::to_string(&blocked_json).expect("json serializes");
    assert!(!blocked_stderr.contains("Bearer not-for-output"));

    let preserved = fs::read_to_string(&target).expect("manual file should be preserved");
    assert_eq!(preserved, manual_content);
}

#[test]
fn skill_update_cli_force_overwrites_manual_or_tampered_content() {
    let project = tempdir().expect("project tempdir");
    let home = tempdir().expect("home tempdir");
    let target = project
        .path()
        .join(".agents")
        .join("skills")
        .join("codex-image")
        .join("SKILL.md");

    fs::create_dir_all(target.parent().expect("target parent")).expect("create target parent");
    let tampered =
        "<!-- codex-image:managed checksum=deadbeefdeadbeef -->\n# manual\nBearer not-for-output\n";
    fs::write(&target, tampered).expect("seed tampered skill");

    let mut forced = Command::cargo_bin("codex-image").expect("binary exists");
    let forced_output = forced
        .current_dir(project.path())
        .arg("skill")
        .arg("update")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("project")
        .arg("--yes")
        .arg("--force")
        .env("HOME", home.path())
        .output()
        .expect("forced update runs");

    assert!(
        forced_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&forced_output.stderr)
    );
    assert!(forced_output.stderr.is_empty());

    let forced_json = parse_json_line(forced_output.stdout);
    assert_eq!(forced_json["status"], "forced_overwrite");

    let overwritten = fs::read_to_string(&target).expect("manual file should be overwritten");
    assert_eq!(overwritten, render_managed_skill_content());
    assert!(!overwritten.contains("Bearer not-for-output"));
}

#[test]
fn skill_update_cli_multi_target_repeated_flags_dedupes_and_orders_global_then_project() {
    let project = tempdir().expect("project tempdir");
    let home = tempdir().expect("home tempdir");

    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    let output = cmd
        .current_dir(project.path())
        .arg("skill")
        .arg("update")
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
        .expect("multi-target update runs");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());

    let lines = parse_json_lines(output.stdout);
    assert_eq!(
        lines.len(),
        2,
        "deduped tool/scope should produce 2 targets"
    );

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
fn skill_update_cli_missing_home_for_global_scope_emits_redacted_config_error() {
    let project = tempdir().expect("project tempdir");

    let mut missing_home = Command::cargo_bin("codex-image").expect("binary exists");
    let missing_home_output = missing_home
        .current_dir(project.path())
        .arg("skill")
        .arg("update")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("global")
        .arg("--yes")
        .env_remove("HOME")
        .output()
        .expect("missing home update runs");

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
fn skill_update_cli_failure_does_not_emit_partial_success_lines() {
    let project = tempdir().expect("project tempdir");
    let home = tempdir().expect("home tempdir");

    let project_target = project
        .path()
        .join(".agents")
        .join("skills")
        .join("codex-image")
        .join("SKILL.md");
    fs::create_dir_all(project_target.parent().expect("target parent"))
        .expect("create project target parent");
    fs::write(&project_target, "# manual\nBearer not-for-output\n")
        .expect("seed manual project skill");

    let global_target = home
        .path()
        .join(".agents")
        .join("skills")
        .join("codex-image")
        .join("SKILL.md");

    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    let output = cmd
        .current_dir(project.path())
        .arg("skill")
        .arg("update")
        .arg("--tool")
        .arg("pi")
        .arg("--scope")
        .arg("global")
        .arg("--scope")
        .arg("project")
        .arg("--yes")
        .env("HOME", home.path())
        .output()
        .expect("multi-target blocked update runs");

    assert_eq!(output.status.code(), Some(5));
    assert!(
        output.stdout.is_empty(),
        "stdout should stay empty when any target blocks"
    );

    let stderr_json = parse_json_line(output.stderr);
    assert_eq!(
        stderr_json["error"]["code"],
        "filesystem.skill_update_blocked_manual_edit"
    );

    assert!(
        global_target.is_file(),
        "earlier global target can be written before later blocked target"
    );
}
