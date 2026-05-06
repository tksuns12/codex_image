use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

use crate::codex::generate_image_with_codex;
use crate::diagnostics::CliError;
use crate::output::write_generation_output_from_files;
use crate::skill_install_ux::{
    expand_selected_targets, interactive_target_options, select_interactive_targets,
    DialoguerTargetSelector, InstallTargetSelector, InteractiveSelectionError, SkillInstallTarget,
    TargetSelectionState,
};
use crate::skill_installer::{
    install_skill, SkillInstallOptions, SkillInstallPlan, SkillInstallStatus,
};
use crate::skills::{SkillScope, SupportedTool};
use crate::updater::{GitHubReleaseClient, UpdateOptions, UpdateResult, run_update};

const GPT_IMAGE_MODEL: &str = "gpt-image-2";
const UPDATE_REPOSITORY: &str = "tksuns12/codex_image";

#[derive(Debug, Parser)]
#[command(name = "codex-image", version, about = "Codex Image CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Generate image artifacts and a manifest for the provided prompt via installed Codex.
    Generate {
        /// Prompt text passed to Codex's built-in image generation tool.
        prompt: String,
        /// Output directory where generated image files and manifest.json are written.
        #[arg(long, value_name = "DIR")]
        out: PathBuf,
    },
    /// Update codex-image from GitHub Release archives for the current platform.
    Update {
        /// Required confirmation before replacing the current binary.
        #[arg(long)]
        yes: bool,
        /// Resolve, download, and validate archive contents without replacing the current binary.
        #[arg(long)]
        dry_run: bool,
        /// Optional GitHub Release tag (for example: v1.2.3). Defaults to latest when omitted.
        #[arg(long = "version", value_name = "TAG", value_parser = parse_release_tag)]
        version: Option<String>,
    },
    /// Manage codex-image native skill installation paths.
    Skill {
        #[command(subcommand)]
        command: SkillCommands,
    },
}

#[derive(Debug, Subcommand)]
enum SkillCommands {
    /// Install the codex-image SKILL.md file for selected supported tool/scope targets.
    /// Omit flags to use interactive target selection when running in a terminal.
    Install {
        /// Tool slug to install for. May be repeated for deterministic multi-target installs.
        #[arg(long, value_enum)]
        tool: Vec<ToolArg>,
        /// Installation scope. May be repeated for deterministic multi-target installs.
        #[arg(long, value_enum)]
        scope: Vec<ScopeArg>,
        /// Required confirmation for non-interactive installs that pass --tool/--scope.
        #[arg(long)]
        yes: bool,
        /// Overwrite manual or tampered existing content.
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Refresh managed codex-image SKILL.md files for selected supported tool/scope targets.
    /// No-ops current managed files and protects manual edits unless --force is passed.
    /// Omit flags to use interactive target selection when running in a terminal.
    Update {
        /// Tool slug to update for. May be repeated for deterministic multi-target updates.
        #[arg(long, value_enum)]
        tool: Vec<ToolArg>,
        /// Update scope. May be repeated for deterministic multi-target updates.
        #[arg(long, value_enum)]
        scope: Vec<ScopeArg>,
        /// Required confirmation for non-interactive updates that pass --tool/--scope.
        #[arg(long)]
        yes: bool,
        /// Overwrite manual or tampered existing content.
        #[arg(long, default_value_t = false)]
        force: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ToolArg {
    Claude,
    #[value(name = "claude-code")]
    ClaudeCode,
    Codex,
    Pi,
    #[value(name = "opencode")]
    OpenCode,
}

impl From<ToolArg> for SupportedTool {
    fn from(value: ToolArg) -> Self {
        match value {
            ToolArg::Claude => SupportedTool::Claude,
            ToolArg::ClaudeCode => SupportedTool::ClaudeCode,
            ToolArg::Codex => SupportedTool::Codex,
            ToolArg::Pi => SupportedTool::Pi,
            ToolArg::OpenCode => SupportedTool::OpenCode,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ScopeArg {
    Global,
    #[value(name = "project")]
    Project,
}

impl From<ScopeArg> for SkillScope {
    fn from(value: ScopeArg) -> Self {
        match value {
            ScopeArg::Global => SkillScope::Global,
            ScopeArg::Project => SkillScope::ProjectLocal,
        }
    }
}

#[derive(Debug, Serialize)]
struct SkillInstallOutput {
    tool: &'static str,
    scope: &'static str,
    status: &'static str,
    target_path: String,
}

#[derive(Clone, Copy, Debug)]
enum SkillCommandOperation {
    Install,
    Update,
}

impl SkillCommandOperation {
    const fn missing_confirmation_error(self) -> CliError {
        match self {
            Self::Install => CliError::MissingInstallConfirmation,
            Self::Update => CliError::MissingUpdateConfirmation,
        }
    }

    const fn partial_selection_error(self) -> CliError {
        match self {
            Self::Install => CliError::PartialInstallTargetSelection,
            Self::Update => CliError::PartialUpdateTargetSelection,
        }
    }

    const fn no_targets_non_interactive_error(self) -> CliError {
        match self {
            Self::Install => CliError::NoInstallTargetsInNonInteractiveMode,
            Self::Update => CliError::NoUpdateTargetsInNonInteractiveMode,
        }
    }

    const fn interactive_cancelled_error(self) -> CliError {
        match self {
            Self::Install => CliError::InteractiveInstallSelectionCancelled,
            Self::Update => CliError::InteractiveUpdateSelectionCancelled,
        }
    }

    const fn interactive_prompt_failed_error(self) -> CliError {
        match self {
            Self::Install => CliError::InteractiveInstallPromptFailed,
            Self::Update => CliError::InteractiveUpdatePromptFailed,
        }
    }

    const fn interactive_empty_selection_error(self) -> CliError {
        match self {
            Self::Install => CliError::InteractiveInstallSelectionEmpty,
            Self::Update => CliError::InteractiveUpdateSelectionEmpty,
        }
    }

    const fn write_failed_error(self) -> CliError {
        match self {
            Self::Install => CliError::SkillInstallWriteFailed,
            Self::Update => CliError::SkillUpdateWriteFailed,
        }
    }

    const fn blocked_manual_edit_error(self) -> CliError {
        match self {
            Self::Install => CliError::SkillInstallBlockedManualEdit,
            Self::Update => CliError::SkillUpdateBlockedManualEdit,
        }
    }
}

pub fn run() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let _ = err.print();
            return err.exit_code();
        }
    };

    match dispatch(cli) {
        Ok(()) => 0,
        Err(err) => {
            let envelope = err.error_envelope();
            let line = serde_json::to_string(&envelope).unwrap_or_else(|_| {
                "{\"error\":{\"code\":\"unknown\",\"message\":\"unexpected failure\",\"recoverable\":false,\"hint\":\"Re-run with supported commands or update the binary.\"}}".to_string()
            });
            eprintln!("{line}");
            err.exit_code().as_i32()
        }
    }
}

fn dispatch(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Generate { prompt, out } => generate(prompt, out),
        Commands::Update {
            yes,
            dry_run,
            version,
        } => update(yes, dry_run, version),
        Commands::Skill { command } => dispatch_skill(command),
    }
}

fn generate(prompt: String, out: PathBuf) -> Result<(), CliError> {
    let generated = generate_image_with_codex(&prompt, &out)?;
    let manifest = write_generation_output_from_files(
        &prompt,
        GPT_IMAGE_MODEL,
        &out,
        &[generated.source_path],
    )?;
    let line = serde_json::to_string(&manifest).map_err(|_| CliError::OutputWriteFailed)?;
    println!("{line}");

    Ok(())
}

fn update(yes: bool, dry_run: bool, version: Option<String>) -> Result<(), CliError> {
    if !dry_run && !yes {
        return Err(crate::updater::UpdateError::ConfirmationRequired.into());
    }

    let client = GitHubReleaseClient::new(UPDATE_REPOSITORY)?;
    let current_executable = std::env::current_exe().map_err(|_| CliError::ProjectRootUnavailable)?;
    let options = UpdateOptions {
        current_executable,
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        requested_version: version,
        dry_run,
        confirm: yes,
    };

    let result = run_update(&client, &options)?;
    print_update_result(&result)
}

fn print_update_result(result: &UpdateResult) -> Result<(), CliError> {
    let line = serde_json::to_string(result).map_err(|_| CliError::OutputWriteFailed)?;
    println!("{line}");
    Ok(())
}

fn parse_release_tag(value: &str) -> Result<String, String> {
    if !value.starts_with('v') {
        return Err("version tag must start with 'v' (example: v1.2.3)".to_string());
    }

    let mut components = value[1..].split('.');
    let valid = components.clone().count() == 3
        && components.all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()));

    if !valid {
        return Err("version tag must be semantic (example: v1.2.3)".to_string());
    }

    Ok(value.to_string())
}

fn dispatch_skill(command: SkillCommands) -> Result<(), CliError> {
    match command {
        SkillCommands::Install {
            tool,
            scope,
            yes,
            force,
        } => skill_command(SkillCommandOperation::Install, &tool, &scope, yes, force),
        SkillCommands::Update {
            tool,
            scope,
            yes,
            force,
        } => skill_command(SkillCommandOperation::Update, &tool, &scope, yes, force),
    }
}

fn skill_command(
    operation: SkillCommandOperation,
    tools: &[ToolArg],
    scopes: &[ScopeArg],
    yes: bool,
    force: bool,
) -> Result<(), CliError> {
    let interactive_mode = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let selector = DialoguerTargetSelector;
    skill_command_with_selector(
        operation,
        tools,
        scopes,
        yes,
        force,
        interactive_mode,
        &selector,
    )
}

fn skill_command_with_selector(
    operation: SkillCommandOperation,
    tools: &[ToolArg],
    scopes: &[ScopeArg],
    yes: bool,
    force: bool,
    interactive_mode: bool,
    selector: &dyn InstallTargetSelector,
) -> Result<(), CliError> {
    let project_root = std::env::current_dir().map_err(|_| CliError::ProjectRootUnavailable)?;
    skill_command_with_selector_and_project_root(
        operation,
        tools,
        scopes,
        yes,
        force,
        interactive_mode,
        selector,
        &project_root,
        None,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn install_skill_command_with_selector_and_project_root(
    tools: &[ToolArg],
    scopes: &[ScopeArg],
    yes: bool,
    force: bool,
    interactive_mode: bool,
    selector: &dyn InstallTargetSelector,
    project_root: &Path,
    home_dir_override: Option<&Path>,
) -> Result<(), CliError> {
    skill_command_with_selector_and_project_root(
        SkillCommandOperation::Install,
        tools,
        scopes,
        yes,
        force,
        interactive_mode,
        selector,
        project_root,
        home_dir_override,
    )
}

#[allow(clippy::too_many_arguments)]
fn skill_command_with_selector_and_project_root(
    operation: SkillCommandOperation,
    tools: &[ToolArg],
    scopes: &[ScopeArg],
    yes: bool,
    force: bool,
    interactive_mode: bool,
    selector: &dyn InstallTargetSelector,
    project_root: &Path,
    home_dir_override: Option<&Path>,
) -> Result<(), CliError> {
    let selected_tools: Vec<SupportedTool> = tools.iter().copied().map(Into::into).collect();
    let selected_scopes: Vec<SkillScope> = scopes.iter().copied().map(Into::into).collect();

    let selection = expand_selected_targets(&selected_tools, &selected_scopes);
    if selection.state == TargetSelectionState::PartialTargets {
        return Err(operation.partial_selection_error());
    }

    let targets = match selection.state {
        TargetSelectionState::Complete => {
            if !yes {
                return Err(operation.missing_confirmation_error());
            }
            selection.targets
        }
        TargetSelectionState::NoTargets => {
            if !interactive_mode {
                return Err(operation.no_targets_non_interactive_error());
            }

            let home_for_options =
                effective_home_dir(home_dir_override).ok_or(CliError::HomeUnavailable)?;
            let options = interactive_target_options(&home_for_options, project_root);
            select_interactive_targets(selector, &options).map_err(|error| match error {
                InteractiveSelectionError::Cancelled => operation.interactive_cancelled_error(),
                InteractiveSelectionError::PromptFailed => {
                    operation.interactive_prompt_failed_error()
                }
                InteractiveSelectionError::EmptySelection => {
                    operation.interactive_empty_selection_error()
                }
            })?
        }
        TargetSelectionState::PartialTargets => unreachable!("partial targets already handled"),
    };

    run_skill_write_loop(operation, targets, force, project_root, home_dir_override)
}

fn run_skill_write_loop(
    operation: SkillCommandOperation,
    targets: Vec<SkillInstallTarget>,
    force: bool,
    project_root: &Path,
    home_dir_override: Option<&Path>,
) -> Result<(), CliError> {
    let selected_scopes: Vec<SkillScope> = targets.iter().map(|target| target.scope).collect();
    let home_dir = resolve_home_dir(&selected_scopes, project_root, home_dir_override)?;

    let mut outputs = Vec::with_capacity(targets.len());
    for target in targets {
        let plan = SkillInstallPlan::build(target.tool, target.scope, &home_dir, project_root);
        let result = install_skill(&plan, SkillInstallOptions { force })
            .map_err(|_| operation.write_failed_error())?;

        if result.status == SkillInstallStatus::BlockedManualEdit {
            return Err(operation.blocked_manual_edit_error());
        }

        outputs.push(SkillInstallOutput {
            tool: target.tool.slug(),
            scope: target.scope.slug(),
            status: result.status.slug(),
            target_path: result.path.display().to_string(),
        });
    }

    for output in outputs {
        let line = serde_json::to_string(&output).map_err(|_| CliError::Unknown)?;
        println!("{line}");
    }

    Ok(())
}

fn resolve_home_dir(
    scopes: &[SkillScope],
    project_root: &Path,
    home_dir_override: Option<&Path>,
) -> Result<PathBuf, CliError> {
    if scopes.contains(&SkillScope::Global) {
        return effective_home_dir(home_dir_override).ok_or(CliError::HomeUnavailable);
    }

    Ok(effective_home_dir(home_dir_override).unwrap_or_else(|| project_root.to_path_buf()))
}

fn effective_home_dir(home_dir_override: Option<&Path>) -> Option<PathBuf> {
    home_dir_override
        .map(|path| path.to_path_buf())
        .or_else(read_home_dir)
}

fn read_home_dir() -> Option<PathBuf> {
    let raw = std::env::var_os("HOME")?;
    if raw.is_empty() {
        return None;
    }

    let as_text = raw.to_string_lossy();
    if as_text.trim().is_empty() {
        return None;
    }

    Some(PathBuf::from(raw))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{install_skill_command_with_selector_and_project_root, ScopeArg, ToolArg};
    use crate::diagnostics::CliError;
    use crate::skill_install_ux::{
        InstallTargetSelector, InteractiveSelectionError, InteractiveTargetOption,
        SkillInstallTarget,
    };
    use crate::skills::{SkillScope, SupportedTool};

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
    fn skill_install_cli_interactive_no_flags_installs_multiple_targets_without_yes() {
        let project = tempfile::tempdir().expect("project tempdir");
        let home = tempfile::tempdir().expect("home tempdir");

        let selector = FakeSelector::from_result(Ok(vec![
            SkillInstallTarget::new(SupportedTool::Pi, SkillScope::Global),
            SkillInstallTarget::new(SupportedTool::Pi, SkillScope::ProjectLocal),
        ]));

        let result = install_skill_command_with_selector_and_project_root(
            &[],
            &[],
            false,
            false,
            true,
            &selector,
            project.path(),
            Some(home.path()),
        );

        assert!(result.is_ok());
        assert_eq!(selector.call_count(), 1);

        assert!(home
            .path()
            .join(".agents")
            .join("skills")
            .join("codex-image")
            .join("SKILL.md")
            .is_file());

        assert!(project
            .path()
            .join(".agents")
            .join("skills")
            .join("codex-image")
            .join("SKILL.md")
            .is_file());
    }

    #[test]
    fn skill_install_cli_interactive_no_flags_empty_selection_fails_without_writes() {
        let project = tempfile::tempdir().expect("project tempdir");
        let home = tempfile::tempdir().expect("home tempdir");

        let selector = FakeSelector::from_result(Err(InteractiveSelectionError::EmptySelection));

        let result = install_skill_command_with_selector_and_project_root(
            &[],
            &[],
            false,
            false,
            true,
            &selector,
            project.path(),
            Some(home.path()),
        );

        assert!(matches!(
            result,
            Err(CliError::InteractiveInstallSelectionEmpty)
        ));
        assert_eq!(selector.call_count(), 1);

        assert!(!home
            .path()
            .join(".agents")
            .join("skills")
            .join("codex-image")
            .join("SKILL.md")
            .exists());

        assert!(!project
            .path()
            .join(".agents")
            .join("skills")
            .join("codex-image")
            .join("SKILL.md")
            .exists());
    }

    #[test]
    fn skill_install_cli_interactive_no_flags_cancel_fails_without_writes() {
        let project = tempfile::tempdir().expect("project tempdir");
        let home = tempfile::tempdir().expect("home tempdir");

        let selector = FakeSelector::from_result(Err(InteractiveSelectionError::Cancelled));

        let result = install_skill_command_with_selector_and_project_root(
            &[],
            &[],
            false,
            false,
            true,
            &selector,
            project.path(),
            Some(home.path()),
        );

        assert!(matches!(
            result,
            Err(CliError::InteractiveInstallSelectionCancelled)
        ));
        assert_eq!(selector.call_count(), 1);
    }

    #[test]
    fn skill_install_cli_interactive_selection_respects_manual_edit_block() {
        let project = tempfile::tempdir().expect("project tempdir");
        let home = tempfile::tempdir().expect("home tempdir");

        let target = project
            .path()
            .join(".agents")
            .join("skills")
            .join("codex-image")
            .join("SKILL.md");
        std::fs::create_dir_all(target.parent().expect("target parent"))
            .expect("create target parent");
        let manual_content = "# custom skill\nmanual-secret\n";
        std::fs::write(&target, manual_content).expect("seed manual content");

        let selector = FakeSelector::from_result(Ok(vec![SkillInstallTarget::new(
            SupportedTool::Pi,
            SkillScope::ProjectLocal,
        )]));

        let result = install_skill_command_with_selector_and_project_root(
            &[],
            &[],
            false,
            false,
            true,
            &selector,
            project.path(),
            Some(home.path()),
        );

        assert!(matches!(
            result,
            Err(CliError::SkillInstallBlockedManualEdit)
        ));
        assert_eq!(selector.call_count(), 1);

        let preserved = std::fs::read_to_string(target).expect("manual file should stay intact");
        assert_eq!(preserved, manual_content);
    }

    #[test]
    fn skill_install_cli_no_flags_non_tty_fails_fast_without_prompt() {
        let project = tempfile::tempdir().expect("project tempdir");
        let selector = FakeSelector::from_result(Ok(vec![SkillInstallTarget::new(
            SupportedTool::Pi,
            SkillScope::ProjectLocal,
        )]));

        let result = install_skill_command_with_selector_and_project_root(
            &[],
            &[],
            false,
            false,
            false,
            &selector,
            project.path(),
            None,
        );

        assert!(matches!(
            result,
            Err(CliError::NoInstallTargetsInNonInteractiveMode)
        ));
        assert_eq!(selector.call_count(), 0);
    }

    #[test]
    fn skill_install_cli_flagged_installs_still_require_yes_and_skip_selector() {
        let project = tempfile::tempdir().expect("project tempdir");
        let selector = FakeSelector::from_result(Ok(vec![SkillInstallTarget::new(
            SupportedTool::Pi,
            SkillScope::ProjectLocal,
        )]));

        let result = install_skill_command_with_selector_and_project_root(
            &[ToolArg::Pi],
            &[ScopeArg::Project],
            false,
            false,
            true,
            &selector,
            project.path(),
            None,
        );

        assert!(matches!(result, Err(CliError::MissingInstallConfirmation)));
        assert_eq!(selector.call_count(), 0);
    }
}
