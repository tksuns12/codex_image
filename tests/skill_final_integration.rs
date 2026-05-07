use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use codex_image::skill_installer::render_managed_skill_content;
use codex_image::skills::{resolve_skill_path, SkillScope, SupportedTool};
use serde_json::Value;
use tempfile::tempdir;

#[derive(Debug)]
struct ExpectedTarget {
    tool: SupportedTool,
    scope: SkillScope,
    path: PathBuf,
}

fn parse_json_lines(bytes: &[u8]) -> Vec<Value> {
    let text = String::from_utf8(bytes.to_vec()).expect("stdout should be utf-8");
    let trimmed = text.trim_end();
    assert!(!trimmed.is_empty(), "stdout must not be empty");

    trimmed
        .lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str::<Value>(line).unwrap_or_else(|error| {
                panic!(
                    "line {} should be valid JSON: {line:?} ({error})",
                    index + 1
                )
            })
        })
        .collect()
}

fn expected_targets(home_dir: &Path, project_root: &Path) -> Vec<ExpectedTarget> {
    let mut targets = Vec::new();
    for tool in SupportedTool::all() {
        for scope in SkillScope::all() {
            targets.push(ExpectedTarget {
                tool,
                scope,
                path: resolve_skill_path(tool, scope, home_dir, project_root),
            });
        }
    }

    targets
}

fn expected_install_statuses(expected: &[ExpectedTarget]) -> Vec<String> {
    let mut seen_paths = HashSet::<PathBuf>::new();
    expected
        .iter()
        .map(|target| {
            if seen_paths.insert(target.path.clone()) {
                "created".to_string()
            } else {
                "unchanged".to_string()
            }
        })
        .collect()
}

fn expected_unchanged_statuses(expected: &[ExpectedTarget]) -> Vec<String> {
    vec!["unchanged".to_string(); expected.len()]
}

fn run_skill_command(
    project_root: &Path,
    home_dir: &Path,
    operation: &str,
) -> std::process::Output {
    let mut cmd = Command::cargo_bin("codex-image").expect("binary exists");
    cmd.current_dir(project_root)
        .arg("skill")
        .arg(operation)
        .arg("--tool")
        .arg("claude")
        .arg("--tool")
        .arg("claude-code")
        .arg("--tool")
        .arg("codex")
        .arg("--tool")
        .arg("pi")
        .arg("--tool")
        .arg("opencode")
        .arg("--scope")
        .arg("global")
        .arg("--scope")
        .arg("project")
        .arg("--yes")
        .env("HOME", home_dir);

    cmd.output().expect("command should run")
}

fn assert_skill_outputs(
    outputs: &[Value],
    expected: &[ExpectedTarget],
    expected_statuses: &[String],
    expected_content: &str,
) {
    assert_eq!(
        outputs.len(),
        expected.len(),
        "expected {} JSON lines, got {}",
        expected.len(),
        outputs.len()
    );

    assert_eq!(
        expected_statuses.len(),
        expected.len(),
        "expected statuses must align with expected targets"
    );

    for ((json, expected_target), expected_status) in outputs
        .iter()
        .zip(expected.iter())
        .zip(expected_statuses.iter())
    {
        assert_eq!(json["tool"], expected_target.tool.slug());
        assert_eq!(json["scope"], expected_target.scope.slug());
        assert_eq!(json["status"], expected_status.as_str());
        assert_eq!(
            json["target_path"].as_str(),
            Some(expected_target.path.to_string_lossy().as_ref())
        );

        assert!(
            expected_target.path.is_file(),
            "expected target path to exist: {}",
            expected_target.path.display()
        );

        let written = fs::read_to_string(&expected_target.path).unwrap_or_else(|error| {
            panic!("failed to read {}: {error}", expected_target.path.display())
        });
        assert_eq!(
            written,
            expected_content,
            "managed bytes should match for {}",
            expected_target.path.display()
        );
    }
}

#[test]
fn skill_final_integration_cli_installs_openai_aligned_managed_skill_for_all_supported_targets() {
    let project = tempdir().expect("project tempdir");
    let home = tempdir().expect("home tempdir");

    let expected = expected_targets(home.path(), project.path());
    assert_eq!(
        expected.len(),
        10,
        "5 tools x 2 scopes must produce 10 targets"
    );

    let managed_content = render_managed_skill_content();
    assert!(
        managed_content.contains("https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide"),
        "managed content must include official OpenAI prompting guide URL"
    );
    assert!(
        managed_content.contains("subject, composition, framing, viewpoint, lighting, and style."),
        "managed content should include guide-aligned prompt-structure language"
    );

    let install_output = run_skill_command(project.path(), home.path(), "install");
    assert!(
        install_output.status.success(),
        "install stderr: {}",
        String::from_utf8_lossy(&install_output.stderr)
    );
    assert!(
        install_output.stderr.is_empty(),
        "install stderr should be empty, got: {}",
        String::from_utf8_lossy(&install_output.stderr)
    );

    let install_lines = parse_json_lines(&install_output.stdout);
    let install_expected_statuses = expected_install_statuses(&expected);
    assert_skill_outputs(
        &install_lines,
        &expected,
        &install_expected_statuses,
        &managed_content,
    );

    let before_update_bytes: HashMap<PathBuf, String> = expected
        .iter()
        .map(|target| {
            (
                target.path.clone(),
                fs::read_to_string(&target.path).unwrap_or_else(|error| {
                    panic!("failed to snapshot {}: {error}", target.path.display())
                }),
            )
        })
        .collect();

    let update_output = run_skill_command(project.path(), home.path(), "update");
    assert!(
        update_output.status.success(),
        "update stderr: {}",
        String::from_utf8_lossy(&update_output.stderr)
    );
    assert!(
        update_output.stderr.is_empty(),
        "update stderr should be empty, got: {}",
        String::from_utf8_lossy(&update_output.stderr)
    );

    let update_lines = parse_json_lines(&update_output.stdout);
    let update_expected_statuses = expected_unchanged_statuses(&expected);
    assert_skill_outputs(
        &update_lines,
        &expected,
        &update_expected_statuses,
        &managed_content,
    );

    for target in &expected {
        let after_update = fs::read_to_string(&target.path)
            .unwrap_or_else(|error| panic!("failed to re-read {}: {error}", target.path.display()));
        let before_update = before_update_bytes
            .get(&target.path)
            .expect("snapshot should contain every target");
        assert_eq!(
            after_update,
            *before_update,
            "update should preserve exact bytes for {}",
            target.path.display()
        );
    }
}
