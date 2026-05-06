use std::fs;

use codex_image::skill_installer::{
    classify_skill_content, install_skill, managed_checksum, managed_marker_line,
    render_managed_skill_content, render_skill_body, SkillContentClassification,
    SkillInstallOptions, SkillInstallPlan, SkillInstallStatus,
};
use codex_image::skills::{resolve_skill_path, SkillScope, SupportedTool};
use tempfile::tempdir;

#[test]
fn skill_installer_content_includes_frontmatter_and_core_sections() {
    let body = render_skill_body();

    assert!(body.starts_with("---\nname: codex-image\n"));
    assert!(body.contains("description:"));
    assert!(body.contains("## Command guidance"));
    assert!(body.contains("## Supported tools"));
    assert!(body.contains("## Prompting guide"));
}

#[test]
fn skill_installer_content_includes_expected_command_guidance() {
    let body = render_skill_body();

    assert!(body.contains("codex-image generate \"<prompt>\" --out <dir>"));
    assert!(
        body.contains(
            "codex-image skill install --tool <claude|claude-code|codex|pi|opencode> --scope <global|project> --yes"
        ),
        "skill body must describe the non-interactive install command"
    );
}

#[test]
fn skill_installer_content_lists_all_supported_tool_slugs() {
    let body = render_skill_body();

    for tool in SupportedTool::all() {
        assert!(
            body.contains(tool.slug()),
            "skill body must mention supported tool slug: {}",
            tool.slug()
        );
    }
}

#[test]
fn skill_installer_content_contains_prompting_guide_url() {
    let body = render_skill_body();
    assert!(body.contains(
        "https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide"
    ));
}

#[test]
fn skill_installer_content_excludes_banned_auth_and_api_strings() {
    let managed = render_managed_skill_content();

    for banned in [
        "OPENAI_API_KEY",
        "CODEX_IMAGE_API_BASE_URL",
        "CODEX_IMAGE_AUTH_BASE_URL",
        "codex-image login",
        "oauth",
        "api key",
        "Bearer ",
    ] {
        assert!(
            !managed
                .to_ascii_lowercase()
                .contains(&banned.to_ascii_lowercase()),
            "managed skill content must not include banned token: {banned}"
        );
    }
}

#[test]
fn skill_installer_content_is_deterministic_bytes_and_checksum() {
    let first = render_managed_skill_content();
    let second = render_managed_skill_content();

    assert_eq!(first.as_bytes(), second.as_bytes());

    let body = render_skill_body();
    let expected_marker = managed_marker_line(body);
    let actual_marker = first.lines().next().expect("marker line exists");
    assert_eq!(actual_marker, expected_marker);

    let marker_checksum = actual_marker
        .strip_prefix("<!-- codex-image:managed checksum=")
        .and_then(|value| value.strip_suffix(" -->"))
        .expect("marker must use stable checksum shape");
    assert_eq!(marker_checksum, managed_checksum(body));
}

#[test]
fn skill_installer_content_classifies_missing_manual_outdated_current_and_tampered() {
    assert_eq!(
        classify_skill_content(None),
        SkillContentClassification::Missing
    );

    assert_eq!(
        classify_skill_content(Some("# manual skill\ncontent\n")),
        SkillContentClassification::ManualUnmanaged
    );

    let current = render_managed_skill_content();
    assert_eq!(
        classify_skill_content(Some(&current)),
        SkillContentClassification::ManagedCurrent
    );

    let outdated_body = render_skill_body().replace("## Guardrails", "## Guardrails (previous)");
    let outdated = format!("{}\n{}", managed_marker_line(&outdated_body), outdated_body);
    assert_eq!(
        classify_skill_content(Some(&outdated)),
        SkillContentClassification::ManagedOutdated
    );

    let tampered = current.replacen(
        "Keep outputs in project-controlled directories.",
        "Keep outputs anywhere.",
        1,
    );
    assert_eq!(
        classify_skill_content(Some(&tampered)),
        SkillContentClassification::ManagedTampered
    );
}

#[test]
fn skill_installer_content_classifies_malformed_managed_metadata_as_tampered() {
    let body = render_skill_body();
    let malformed_marker = "<!-- codex-image:managed checksum=not-hex -->";
    let malformed = format!("{malformed_marker}\n{body}");
    assert_eq!(
        classify_skill_content(Some(&malformed)),
        SkillContentClassification::ManagedTampered
    );
}

#[test]
fn skill_installer_filesystem_plan_build_matches_resolved_path() {
    let home = tempdir().expect("home tempdir");
    let project = tempdir().expect("project tempdir");

    let plan = SkillInstallPlan::build(
        SupportedTool::OpenCode,
        SkillScope::ProjectLocal,
        home.path(),
        project.path(),
    );

    assert_eq!(
        plan.target_path(),
        resolve_skill_path(
            SupportedTool::OpenCode,
            SkillScope::ProjectLocal,
            home.path(),
            project.path()
        )
    );
}

#[test]
fn skill_installer_filesystem_creates_missing_skill_file() {
    let home = tempdir().expect("home tempdir");
    let project = tempdir().expect("project tempdir");

    let plan = SkillInstallPlan::build(
        SupportedTool::Pi,
        SkillScope::ProjectLocal,
        home.path(),
        project.path(),
    );

    assert!(!plan.target_path().exists());

    let result = install_skill(&plan, SkillInstallOptions::default()).expect("install succeeds");
    assert_eq!(result.status, SkillInstallStatus::Created);
    assert_eq!(result.path, plan.target_path());

    let written = fs::read_to_string(plan.target_path()).expect("managed file written");
    assert_eq!(written, render_managed_skill_content());
}

#[test]
fn skill_installer_filesystem_noops_when_already_managed_current() {
    let home = tempdir().expect("home tempdir");
    let project = tempdir().expect("project tempdir");
    let plan = SkillInstallPlan::build(
        SupportedTool::Claude,
        SkillScope::ProjectLocal,
        home.path(),
        project.path(),
    );

    let first = install_skill(&plan, SkillInstallOptions::default()).expect("first install");
    assert_eq!(first.status, SkillInstallStatus::Created);

    let second = install_skill(&plan, SkillInstallOptions::default()).expect("second install");
    assert_eq!(second.status, SkillInstallStatus::Unchanged);
}

#[test]
fn skill_installer_filesystem_updates_valid_managed_outdated_content() {
    let home = tempdir().expect("home tempdir");
    let project = tempdir().expect("project tempdir");
    let plan = SkillInstallPlan::build(
        SupportedTool::ClaudeCode,
        SkillScope::ProjectLocal,
        home.path(),
        project.path(),
    );

    let outdated_body = render_skill_body().replace("## Guardrails", "## Guardrails (old)");
    let outdated = format!("{}\n{}", managed_marker_line(&outdated_body), outdated_body);

    let parent = plan.target_path().parent().expect("parent directory");
    fs::create_dir_all(parent).expect("create parent");
    fs::write(plan.target_path(), outdated).expect("seed outdated managed content");

    let result = install_skill(&plan, SkillInstallOptions::default()).expect("update succeeds");
    assert_eq!(result.status, SkillInstallStatus::Updated);

    let rewritten = fs::read_to_string(plan.target_path()).expect("managed content rewritten");
    assert_eq!(rewritten, render_managed_skill_content());
}

#[test]
fn skill_installer_filesystem_blocks_unmanaged_manual_edits_by_default() {
    let home = tempdir().expect("home tempdir");
    let project = tempdir().expect("project tempdir");
    let plan = SkillInstallPlan::build(
        SupportedTool::Codex,
        SkillScope::ProjectLocal,
        home.path(),
        project.path(),
    );

    let parent = plan.target_path().parent().expect("parent directory");
    fs::create_dir_all(parent).expect("create parent");
    let manual = "# custom skill\nmanual notes\n";
    fs::write(plan.target_path(), manual).expect("seed manual skill");

    let result = install_skill(&plan, SkillInstallOptions::default()).expect("install succeeds");
    assert_eq!(result.status, SkillInstallStatus::BlockedManualEdit);

    let current = fs::read_to_string(plan.target_path()).expect("manual content preserved");
    assert_eq!(current, manual);
}

#[test]
fn skill_installer_filesystem_blocks_tampered_managed_edits_by_default() {
    let home = tempdir().expect("home tempdir");
    let project = tempdir().expect("project tempdir");
    let plan = SkillInstallPlan::build(
        SupportedTool::OpenCode,
        SkillScope::ProjectLocal,
        home.path(),
        project.path(),
    );

    let tampered = render_managed_skill_content().replacen(
        "Keep outputs in project-controlled directories.",
        "Keep outputs in personal desktop directories.",
        1,
    );

    let parent = plan.target_path().parent().expect("parent directory");
    fs::create_dir_all(parent).expect("create parent");
    fs::write(plan.target_path(), &tampered).expect("seed tampered managed content");

    let result = install_skill(&plan, SkillInstallOptions::default()).expect("install succeeds");
    assert_eq!(result.status, SkillInstallStatus::BlockedManualEdit);

    let current = fs::read_to_string(plan.target_path()).expect("tampered content preserved");
    assert_eq!(current, tampered);
}

#[test]
fn skill_installer_filesystem_force_overwrites_blocked_manual_or_tampered_content() {
    let home = tempdir().expect("home tempdir");
    let project = tempdir().expect("project tempdir");
    let plan = SkillInstallPlan::build(
        SupportedTool::Pi,
        SkillScope::Global,
        home.path(),
        project.path(),
    );

    let parent = plan.target_path().parent().expect("parent directory");
    fs::create_dir_all(parent).expect("create parent");
    fs::write(plan.target_path(), "# my custom global skill\n").expect("seed manual content");

    let result = install_skill(&plan, SkillInstallOptions { force: true }).expect("forced install");
    assert_eq!(result.status, SkillInstallStatus::ForcedOverwrite);

    let current = fs::read_to_string(plan.target_path()).expect("forced rewrite");
    assert_eq!(current, render_managed_skill_content());
}

#[test]
fn skill_installer_filesystem_codex_and_pi_duplicate_path_is_idempotent() {
    let home = tempdir().expect("home tempdir");
    let project = tempdir().expect("project tempdir");

    let codex_plan = SkillInstallPlan::build(
        SupportedTool::Codex,
        SkillScope::ProjectLocal,
        home.path(),
        project.path(),
    );
    let pi_plan = SkillInstallPlan::build(
        SupportedTool::Pi,
        SkillScope::ProjectLocal,
        home.path(),
        project.path(),
    );

    assert_eq!(codex_plan.target_path(), pi_plan.target_path());

    let first = install_skill(&codex_plan, SkillInstallOptions::default()).expect("first install");
    assert_eq!(first.status, SkillInstallStatus::Created);

    let second = install_skill(&pi_plan, SkillInstallOptions::default()).expect("second install");
    assert_eq!(second.status, SkillInstallStatus::Unchanged);
}
