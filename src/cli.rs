use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

use crate::codex::generate_image_with_codex;
use crate::diagnostics::CliError;
use crate::output::write_generation_output_from_files;
use crate::skill_install_ux::{expand_selected_targets, TargetSelectionState};
use crate::skill_installer::{
    install_skill, SkillInstallOptions, SkillInstallPlan, SkillInstallStatus,
};
use crate::skills::{SkillScope, SupportedTool};

const GPT_IMAGE_MODEL: &str = "gpt-image-2";

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

pub async fn run() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let _ = err.print();
            return err.exit_code();
        }
    };

    match dispatch(cli).await {
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

async fn dispatch(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Generate { prompt, out } => generate(prompt, out).await,
        Commands::Skill { command } => dispatch_skill(command),
    }
}

async fn generate(prompt: String, out: PathBuf) -> Result<(), CliError> {
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

fn dispatch_skill(command: SkillCommands) -> Result<(), CliError> {
    match command {
        SkillCommands::Install {
            tool,
            scope,
            yes,
            force,
        } => install_skill_command(&tool, &scope, yes, force),
    }
}

fn install_skill_command(
    tools: &[ToolArg],
    scopes: &[ScopeArg],
    yes: bool,
    force: bool,
) -> Result<(), CliError> {
    let selected_tools: Vec<SupportedTool> = tools.iter().copied().map(Into::into).collect();
    let selected_scopes: Vec<SkillScope> = scopes.iter().copied().map(Into::into).collect();

    let selection = expand_selected_targets(&selected_tools, &selected_scopes);
    match selection.state {
        TargetSelectionState::PartialTargets => {
            return Err(CliError::PartialInstallTargetSelection);
        }
        TargetSelectionState::NoTargets => {
            if !std::io::stdin().is_terminal() {
                return Err(CliError::NoInstallTargetsInNonInteractiveMode);
            }
            return Err(CliError::NoInstallTargetsInNonInteractiveMode);
        }
        TargetSelectionState::Complete => {}
    }

    if !yes {
        return Err(CliError::MissingInstallConfirmation);
    }

    let project_root = std::env::current_dir().map_err(|_| CliError::ProjectRootUnavailable)?;
    let home_dir = resolve_home_dir(&selection.selected_scopes, &project_root)?;

    let mut outputs = Vec::with_capacity(selection.targets.len());
    for target in selection.targets {
        let plan = SkillInstallPlan::build(target.tool, target.scope, &home_dir, &project_root);
        let result = install_skill(&plan, SkillInstallOptions { force })
            .map_err(|_| CliError::SkillInstallWriteFailed)?;

        if result.status == SkillInstallStatus::BlockedManualEdit {
            return Err(CliError::SkillInstallBlockedManualEdit);
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

fn resolve_home_dir(scopes: &[SkillScope], project_root: &Path) -> Result<PathBuf, CliError> {
    if scopes.contains(&SkillScope::Global) {
        return read_home_dir().ok_or(CliError::HomeUnavailable);
    }

    Ok(read_home_dir().unwrap_or_else(|| project_root.to_path_buf()))
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
