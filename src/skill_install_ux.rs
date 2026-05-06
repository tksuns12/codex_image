use std::path::{Path, PathBuf};

use dialoguer::{theme::ColorfulTheme, MultiSelect};

use crate::skill_installer::{classify_skill_path, SkillContentClassification};
use crate::skills::{resolve_skill_path, SkillScope, SupportedTool};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractiveInstallState {
    NotInstalled,
    InstalledCurrent,
    InstalledOutdated,
    InstalledProtected,
}

impl InteractiveInstallState {
    pub const fn is_installed(self) -> bool {
        !matches!(self, Self::NotInstalled)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::NotInstalled => "not-installed",
            Self::InstalledCurrent => "installed",
            Self::InstalledOutdated => "installed:outdated",
            Self::InstalledProtected => "installed:protected",
        }
    }

    pub const fn from_classification(classification: SkillContentClassification) -> Self {
        match classification {
            SkillContentClassification::Missing => Self::NotInstalled,
            SkillContentClassification::ManagedCurrent => Self::InstalledCurrent,
            SkillContentClassification::ManagedOutdated => Self::InstalledOutdated,
            SkillContentClassification::ManualUnmanaged
            | SkillContentClassification::ManagedTampered => Self::InstalledProtected,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractiveTargetOption {
    pub target: SkillInstallTarget,
    pub target_path: PathBuf,
    pub install_state: InteractiveInstallState,
    pub default_selected: bool,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractiveSelectionError {
    Cancelled,
    PromptFailed,
    EmptySelection,
}

pub trait InstallTargetSelector {
    fn select(
        &self,
        options: &[InteractiveTargetOption],
    ) -> Result<Vec<SkillInstallTarget>, InteractiveSelectionError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DialoguerTargetSelector;

impl InstallTargetSelector for DialoguerTargetSelector {
    fn select(
        &self,
        options: &[InteractiveTargetOption],
    ) -> Result<Vec<SkillInstallTarget>, InteractiveSelectionError> {
        let labels: Vec<&str> = options.iter().map(|option| option.label.as_str()).collect();
        let defaults: Vec<bool> = options
            .iter()
            .map(|option| option.default_selected)
            .collect();
        let selected = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select install targets (Space to toggle, Enter to confirm)")
            .items(&labels)
            .defaults(&defaults)
            .interact_opt()
            .map_err(|_| InteractiveSelectionError::PromptFailed)?;

        let Some(selected) = selected else {
            return Err(InteractiveSelectionError::Cancelled);
        };

        if selected.is_empty() {
            return Err(InteractiveSelectionError::EmptySelection);
        }

        selected
            .into_iter()
            .map(|index| {
                options
                    .get(index)
                    .map(|option| option.target)
                    .ok_or(InteractiveSelectionError::PromptFailed)
            })
            .collect()
    }
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

pub fn interactive_target_options(
    home_dir: &Path,
    project_root: &Path,
) -> Vec<InteractiveTargetOption> {
    all_selectable_targets()
        .into_iter()
        .map(|target| {
            let target_path = resolve_skill_path(target.tool, target.scope, home_dir, project_root);
            let install_state = classify_skill_path(&target_path)
                .map(InteractiveInstallState::from_classification)
                .unwrap_or(InteractiveInstallState::InstalledProtected);

            InteractiveTargetOption {
                label: format!(
                    "{} ({}) [{}] [{}] -> {}",
                    target.tool.display_name(),
                    target.tool.slug(),
                    target.scope.slug(),
                    install_state.label(),
                    target_path.display(),
                ),
                target,
                target_path,
                default_selected: install_state.is_installed(),
                install_state,
            }
        })
        .collect()
}

pub fn select_interactive_targets(
    selector: &dyn InstallTargetSelector,
    options: &[InteractiveTargetOption],
) -> Result<Vec<SkillInstallTarget>, InteractiveSelectionError> {
    selector.select(options)
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
