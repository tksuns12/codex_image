use codex_image::skill_installer::{
    classify_skill_content, managed_checksum, managed_marker_line, render_managed_skill_content,
    render_skill_body, SkillContentClassification,
};
use codex_image::skills::SupportedTool;

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
