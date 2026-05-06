use codex_image::skill_install_ux::{
    all_selectable_targets, expand_selected_targets, SkillInstallTarget, TargetSelectionState,
};
use codex_image::skills::{SkillScope, SupportedTool};

#[test]
fn skill_install_ux_all_selectable_targets_cover_full_matrix_in_canonical_order() {
    let actual = all_selectable_targets();

    let mut expected = Vec::new();
    for tool in SupportedTool::all() {
        for scope in SkillScope::all() {
            expected.push(SkillInstallTarget::new(tool, scope));
        }
    }

    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 10);
}

#[test]
fn skill_install_ux_expansion_dedupes_repeated_tool_and_scope_flags() {
    let selection = expand_selected_targets(
        &[
            SupportedTool::Pi,
            SupportedTool::Pi,
            SupportedTool::Codex,
            SupportedTool::Pi,
        ],
        &[
            SkillScope::ProjectLocal,
            SkillScope::ProjectLocal,
            SkillScope::Global,
        ],
    );

    assert_eq!(selection.state, TargetSelectionState::Complete);
    assert_eq!(
        selection.selected_tools,
        vec![SupportedTool::Codex, SupportedTool::Pi]
    );
    assert_eq!(
        selection.selected_scopes,
        vec![SkillScope::Global, SkillScope::ProjectLocal]
    );

    assert_eq!(
        selection.targets,
        vec![
            SkillInstallTarget::new(SupportedTool::Codex, SkillScope::Global),
            SkillInstallTarget::new(SupportedTool::Codex, SkillScope::ProjectLocal),
            SkillInstallTarget::new(SupportedTool::Pi, SkillScope::Global),
            SkillInstallTarget::new(SupportedTool::Pi, SkillScope::ProjectLocal),
        ]
    );
}

#[test]
fn skill_install_ux_expansion_builds_cartesian_targets_for_selected_dimensions() {
    let selection = expand_selected_targets(
        &[SupportedTool::OpenCode, SupportedTool::Claude],
        &[SkillScope::ProjectLocal, SkillScope::Global],
    );

    assert_eq!(selection.state, TargetSelectionState::Complete);
    assert_eq!(
        selection.targets,
        vec![
            SkillInstallTarget::new(SupportedTool::Claude, SkillScope::Global),
            SkillInstallTarget::new(SupportedTool::Claude, SkillScope::ProjectLocal),
            SkillInstallTarget::new(SupportedTool::OpenCode, SkillScope::Global),
            SkillInstallTarget::new(SupportedTool::OpenCode, SkillScope::ProjectLocal),
        ]
    );
}

#[test]
fn skill_install_ux_expansion_reports_no_target_and_partial_target_metadata() {
    let none_selected = expand_selected_targets(&[], &[]);
    assert_eq!(none_selected.state, TargetSelectionState::NoTargets);
    assert!(none_selected.missing_tools);
    assert!(none_selected.missing_scopes);
    assert!(none_selected.targets.is_empty());

    let missing_tools = expand_selected_targets(&[], &[SkillScope::Global]);
    assert_eq!(missing_tools.state, TargetSelectionState::PartialTargets);
    assert!(missing_tools.missing_tools);
    assert!(!missing_tools.missing_scopes);
    assert!(missing_tools.targets.is_empty());

    let missing_scopes = expand_selected_targets(&[SupportedTool::Claude], &[]);
    assert_eq!(missing_scopes.state, TargetSelectionState::PartialTargets);
    assert!(!missing_scopes.missing_tools);
    assert!(missing_scopes.missing_scopes);
    assert!(missing_scopes.targets.is_empty());
}
