use std::path::Path;

use codex_image::skills::{resolve_skill_path, SkillScope, SupportedTool, CANONICAL_SKILL_NAME};

#[test]
fn skill_path_contracts_canonical_skill_name_is_fixed() {
    assert_eq!(CANONICAL_SKILL_NAME, "codex-image");
}

#[test]
fn skill_path_contracts_supported_tool_labels_and_slugs_are_stable() {
    let matrix = [
        (SupportedTool::Claude, "claude", "Claude"),
        (SupportedTool::ClaudeCode, "claude-code", "Claude Code"),
        (SupportedTool::Codex, "codex", "Codex"),
        (SupportedTool::Pi, "pi", "pi"),
        (SupportedTool::OpenCode, "opencode", "OpenCode"),
    ];

    for (tool, slug, display) in matrix {
        assert_eq!(tool.slug(), slug);
        assert_eq!(tool.display_name(), display);
        assert_eq!(SupportedTool::from_slug(slug), Some(tool));
    }
}

#[test]
fn skill_path_contracts_rejects_unsupported_tool_slugs() {
    for unsupported in ["", "claudecode", "open-code", "copilot", "cursor"] {
        assert_eq!(SupportedTool::from_slug(unsupported), None);
    }
}

#[test]
fn skill_path_contracts_resolves_every_supported_tool_scope_pair() {
    let home = Path::new("/mock/home");
    let project = Path::new("/mock/project");

    let matrix = [
        (
            SupportedTool::Claude,
            SkillScope::Global,
            "/mock/home/.claude/skills/codex-image/SKILL.md",
        ),
        (
            SupportedTool::Claude,
            SkillScope::ProjectLocal,
            "/mock/project/.claude/skills/codex-image/SKILL.md",
        ),
        (
            SupportedTool::ClaudeCode,
            SkillScope::Global,
            "/mock/home/.claude/skills/codex-image/SKILL.md",
        ),
        (
            SupportedTool::ClaudeCode,
            SkillScope::ProjectLocal,
            "/mock/project/.claude/skills/codex-image/SKILL.md",
        ),
        (
            SupportedTool::Codex,
            SkillScope::Global,
            "/mock/home/.agents/skills/codex-image/SKILL.md",
        ),
        (
            SupportedTool::Codex,
            SkillScope::ProjectLocal,
            "/mock/project/.agents/skills/codex-image/SKILL.md",
        ),
        (
            SupportedTool::Pi,
            SkillScope::Global,
            "/mock/home/.agents/skills/codex-image/SKILL.md",
        ),
        (
            SupportedTool::Pi,
            SkillScope::ProjectLocal,
            "/mock/project/.agents/skills/codex-image/SKILL.md",
        ),
        (
            SupportedTool::OpenCode,
            SkillScope::Global,
            "/mock/home/.config/opencode/skills/codex-image/SKILL.md",
        ),
        (
            SupportedTool::OpenCode,
            SkillScope::ProjectLocal,
            "/mock/project/.opencode/skills/codex-image/SKILL.md",
        ),
    ];

    for (tool, scope, expected) in matrix {
        assert_eq!(
            resolve_skill_path(tool, scope, home, project),
            Path::new(expected)
        );
    }
}

#[test]
fn skill_path_contracts_codex_never_uses_legacy_codex_directory() {
    let home = Path::new("/mock/home");
    let project = Path::new("/mock/project");

    for scope in [SkillScope::Global, SkillScope::ProjectLocal] {
        let resolved = resolve_skill_path(SupportedTool::Codex, scope, home, project);
        assert!(
            !resolved.to_string_lossy().contains(".codex/skills"),
            "codex path unexpectedly used .codex/skills: {resolved:?}"
        );
    }
}

#[test]
fn skill_path_contracts_deterministic_resolution_under_repeated_calls() {
    let home = Path::new("/mock/home");
    let project = Path::new("/mock/project");

    for tool in SupportedTool::all() {
        for scope in [SkillScope::Global, SkillScope::ProjectLocal] {
            let baseline = resolve_skill_path(tool, scope, home, project);
            for _ in 0..10 {
                assert_eq!(resolve_skill_path(tool, scope, home, project), baseline);
            }
        }
    }
}
