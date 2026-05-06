use crate::skills::{SkillScope, SupportedTool};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SkillInstallTarget {
    pub tool: SupportedTool,
    pub scope: SkillScope,
}

impl SkillInstallTarget {
    pub const fn new(tool: SupportedTool, scope: SkillScope) -> Self {
        Self { tool, scope }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetSelectionState {
    Complete,
    NoTargets,
    PartialTargets,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSelection {
    pub state: TargetSelectionState,
    pub targets: Vec<SkillInstallTarget>,
    pub selected_tools: Vec<SupportedTool>,
    pub selected_scopes: Vec<SkillScope>,
    pub missing_tools: bool,
    pub missing_scopes: bool,
}

pub fn all_selectable_targets() -> Vec<SkillInstallTarget> {
    let mut targets = Vec::new();
    for tool in SupportedTool::all() {
        for scope in SkillScope::all() {
            targets.push(SkillInstallTarget::new(tool, scope));
        }
    }

    targets
}

pub fn expand_selected_targets(
    selected_tools: &[SupportedTool],
    selected_scopes: &[SkillScope],
) -> TargetSelection {
    let selected_tools = dedupe_tools_canonical(selected_tools);
    let selected_scopes = dedupe_scopes_canonical(selected_scopes);

    let missing_tools = selected_tools.is_empty();
    let missing_scopes = selected_scopes.is_empty();

    let state = if missing_tools && missing_scopes {
        TargetSelectionState::NoTargets
    } else if missing_tools || missing_scopes {
        TargetSelectionState::PartialTargets
    } else {
        TargetSelectionState::Complete
    };

    let mut targets = Vec::new();
    if state == TargetSelectionState::Complete {
        for tool in &selected_tools {
            for scope in &selected_scopes {
                targets.push(SkillInstallTarget::new(*tool, *scope));
            }
        }
    }

    TargetSelection {
        state,
        targets,
        selected_tools,
        selected_scopes,
        missing_tools,
        missing_scopes,
    }
}

fn dedupe_tools_canonical(selected_tools: &[SupportedTool]) -> Vec<SupportedTool> {
    SupportedTool::all()
        .into_iter()
        .filter(|tool| selected_tools.contains(tool))
        .collect()
}

fn dedupe_scopes_canonical(selected_scopes: &[SkillScope]) -> Vec<SkillScope> {
    SkillScope::all()
        .into_iter()
        .filter(|scope| selected_scopes.contains(scope))
        .collect()
}
