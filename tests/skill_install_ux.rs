use std::cell::RefCell;

use codex_image::skill_install_ux::{
    all_selectable_targets, expand_selected_targets, interactive_target_options,
    select_interactive_targets, InstallTargetSelector, InteractiveSelectionError,
    InteractiveTargetOption, SkillInstallTarget, TargetSelectionState,
};
use codex_image::skills::{SkillScope, SupportedTool};

struct FakeSelector {
    result: Result<Vec<SkillInstallTarget>, InteractiveSelectionError>,
    calls: RefCell<usize>,
}

impl FakeSelector {
    fn from_result(result: Result<Vec<SkillInstallTarget>, InteractiveSelectionError>) -> Self {
        Self {
            result,
            calls: RefCell::new(0),
        }
    }

    fn call_count(&self) -> usize {
        *self.calls.borrow()
    }
}

impl InstallTargetSelector for FakeSelector {
    fn select(
        &self,
        _options: &[InteractiveTargetOption],
    ) -> Result<Vec<SkillInstallTarget>, InteractiveSelectionError> {
        *self.calls.borrow_mut() += 1;
        self.result.clone()
    }
}

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

#[test]
fn skill_install_ux_interactive_options_include_tool_scope_and_target_path_labels() {
    let project = tempfile::tempdir().expect("project tempdir");
    let home = tempfile::tempdir().expect("home tempdir");

    let options = interactive_target_options(home.path(), project.path());
    assert_eq!(options.len(), 10);

    let first = &options[0];
    assert_eq!(first.target.tool, SupportedTool::Claude);
    assert_eq!(first.target.scope, SkillScope::Global);
    assert!(first.label.contains("Claude"));
    assert!(first.label.contains("(claude)"));
    assert!(first.label.contains("[global]"));
    assert!(
        first.label.contains(home.path().to_string_lossy().as_ref()),
        "label should include resolved global target path"
    );

    let project_local_pi = options
        .iter()
        .find(|option| {
            option.target.tool == SupportedTool::Pi
                && option.target.scope == SkillScope::ProjectLocal
        })
        .expect("pi/project option exists");
    assert!(project_local_pi.label.contains("pi"));
    assert!(project_local_pi.label.contains("[project]"));
    assert!(
        project_local_pi
            .label
            .contains(project.path().to_string_lossy().as_ref()),
        "label should include resolved project-local target path"
    );
}

#[test]
fn skill_install_ux_prompt_boundary_allows_multiple_targets_via_fake_selector() {
    let selector = FakeSelector::from_result(Ok(vec![
        SkillInstallTarget::new(SupportedTool::Pi, SkillScope::Global),
        SkillInstallTarget::new(SupportedTool::OpenCode, SkillScope::ProjectLocal),
    ]));

    let options = vec![
        InteractiveTargetOption {
            target: SkillInstallTarget::new(SupportedTool::Pi, SkillScope::Global),
            label: "pi global".to_string(),
        },
        InteractiveTargetOption {
            target: SkillInstallTarget::new(SupportedTool::OpenCode, SkillScope::ProjectLocal),
            label: "opencode project".to_string(),
        },
    ];

    let selected = select_interactive_targets(&selector, &options).expect("selection should pass");

    assert_eq!(selector.call_count(), 1);
    assert_eq!(
        selected,
        vec![
            SkillInstallTarget::new(SupportedTool::Pi, SkillScope::Global),
            SkillInstallTarget::new(SupportedTool::OpenCode, SkillScope::ProjectLocal),
        ]
    );
}

#[test]
fn skill_install_ux_prompt_boundary_surfaces_empty_and_cancel_errors() {
    let options = vec![InteractiveTargetOption {
        target: SkillInstallTarget::new(SupportedTool::Pi, SkillScope::ProjectLocal),
        label: "pi project".to_string(),
    }];

    let empty_selector = FakeSelector::from_result(Err(InteractiveSelectionError::EmptySelection));
    let empty_result = select_interactive_targets(&empty_selector, &options);
    assert!(matches!(
        empty_result,
        Err(InteractiveSelectionError::EmptySelection)
    ));
    assert_eq!(empty_selector.call_count(), 1);

    let cancel_selector = FakeSelector::from_result(Err(InteractiveSelectionError::Cancelled));
    let cancel_result = select_interactive_targets(&cancel_selector, &options);
    assert!(matches!(
        cancel_result,
        Err(InteractiveSelectionError::Cancelled)
    ));
    assert_eq!(cancel_selector.call_count(), 1);
}
