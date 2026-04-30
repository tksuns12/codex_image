#[test]
fn skill_path_docs_readme_links_canonical_matrix_doc() {
    let readme = include_str!("../README.md");

    assert!(
        readme.contains("docs/skill-paths.md"),
        "README must link to docs/skill-paths.md"
    );
    assert!(
        readme.contains("M002/S01 contract"),
        "README must label the S01 support-matrix contract"
    );
}

#[test]
fn skill_path_docs_contains_supported_tools_and_paths() {
    let doc = include_str!("../docs/skill-paths.md");

    for tool in ["Claude", "Claude Code", "Codex", "pi / GSD", "OpenCode"] {
        assert!(
            doc.contains(tool),
            "skill path doc must include supported tool row: {tool}"
        );
    }

    for required_path in [
        "~/.claude/skills/codex-image/SKILL.md",
        ".claude/skills/codex-image/SKILL.md",
        "~/.agents/skills/codex-image/SKILL.md",
        ".agents/skills/codex-image/SKILL.md",
        "~/.config/opencode/skills/codex-image/SKILL.md",
        ".opencode/skills/codex-image/SKILL.md",
    ] {
        assert!(
            doc.contains(required_path),
            "skill path doc must include required path: {required_path}"
        );
    }
}

#[test]
fn skill_path_docs_contains_required_source_links() {
    let doc = include_str!("../docs/skill-paths.md");

    for url in [
        "https://code.claude.com/docs/en/skills#where-skills-live",
        "https://developers.openai.com/codex/skills#where-to-save-skills",
        "https://opencode.ai/docs/skills/#place-files",
    ] {
        assert!(
            doc.contains(url),
            "skill path doc must include source evidence URL: {url}"
        );
    }
}

#[test]
fn skill_path_docs_explicitly_documents_codex_agents_choice() {
    let doc = include_str!("../docs/skill-paths.md");

    assert!(
        doc.contains("Codex path contract is explicitly `.agents/skills`, not `~/.codex/skills`."),
        "skill path doc must lock the Codex .agents/skills decision"
    );
}

#[test]
fn skill_path_docs_states_scope_and_prompt_guide_requirement() {
    let doc = include_str!("../docs/skill-paths.md");
    let readme = include_str!("../README.md");
    let prompt_guide =
        "https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide";

    assert!(
        doc.contains("S01 establishes and tests this path contract only."),
        "skill path doc must state S01 is contract-only"
    );
    assert!(
        doc.contains("File-writing commands and installer UX are delivered in later slices."),
        "skill path doc must state install/write commands land later"
    );
    assert!(
        doc.contains(prompt_guide),
        "skill path doc must include the required prompting guide link"
    );
    assert!(
        readme.contains(prompt_guide),
        "README must include the required prompting guide link"
    );
}
