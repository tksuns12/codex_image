#[test]
fn skill_path_docs_readme_links_command_usage_and_canonical_matrix_doc() {
    let readme = include_str!("../README.md");

    assert!(
        readme.contains("docs/skill-paths.md"),
        "README must link to docs/skill-paths.md"
    );
    assert!(
        readme.contains("codex-image skill install --tool")
            && readme.contains("codex-image skill update --tool"),
        "README must link readers to concrete skill install/update command usage"
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
fn skill_path_docs_states_contract_consumers_and_prompt_guide_requirement() {
    let doc = include_str!("../docs/skill-paths.md");
    let advanced_reference = include_str!("../docs/advanced-reference.md");
    let prompt_guide =
        "https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide";

    assert!(
        doc.contains("consumed by `codex-image skill install` and `codex-image skill update`"),
        "skill path doc must state install/update commands consume this path contract"
    );
    assert!(
        !doc.contains("File-writing commands and installer UX are delivered in later slices."),
        "skill path doc must reject stale future-tense wording about install/write commands"
    );
    assert!(
        doc.contains(prompt_guide),
        "skill path doc must include the required prompting guide link"
    );
    assert!(
        advanced_reference.contains(prompt_guide),
        "advanced reference must include the required prompting guide link"
    );
}
