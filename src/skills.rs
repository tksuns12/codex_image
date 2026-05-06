use std::path::{Path, PathBuf};

pub const CANONICAL_SKILL_NAME: &str = "codex-image";
pub const SKILL_FILE_NAME: &str = "SKILL.md";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkillScope {
    Global,
    ProjectLocal,
}

impl SkillScope {
    pub const fn all() -> [Self; 2] {
        [Self::Global, Self::ProjectLocal]
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::ProjectLocal => "project",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "global" => Some(Self::Global),
            "project" => Some(Self::ProjectLocal),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportedTool {
    Claude,
    ClaudeCode,
    Codex,
    Pi,
    OpenCode,
}

impl SupportedTool {
    pub const fn all() -> [Self; 5] {
        [
            Self::Claude,
            Self::ClaudeCode,
            Self::Codex,
            Self::Pi,
            Self::OpenCode,
        ]
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::Pi => "pi",
            Self::OpenCode => "opencode",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
            Self::Pi => "pi",
            Self::OpenCode => "OpenCode",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "claude" => Some(Self::Claude),
            "claude-code" => Some(Self::ClaudeCode),
            "codex" => Some(Self::Codex),
            "pi" => Some(Self::Pi),
            "opencode" => Some(Self::OpenCode),
            _ => None,
        }
    }
}

pub fn resolve_skill_path(
    tool: SupportedTool,
    scope: SkillScope,
    home_dir: &Path,
    project_root: &Path,
) -> PathBuf {
    let base = match scope {
        SkillScope::Global => home_dir,
        SkillScope::ProjectLocal => project_root,
    };

    base.join(skill_directory_prefix(tool, scope))
        .join("skills")
        .join(CANONICAL_SKILL_NAME)
        .join(SKILL_FILE_NAME)
}

fn skill_directory_prefix(tool: SupportedTool, scope: SkillScope) -> &'static str {
    match (tool, scope) {
        (SupportedTool::Claude | SupportedTool::ClaudeCode, _) => ".claude",
        (SupportedTool::Codex | SupportedTool::Pi, _) => ".agents",
        (SupportedTool::OpenCode, SkillScope::Global) => ".config/opencode",
        (SupportedTool::OpenCode, SkillScope::ProjectLocal) => ".opencode",
    }
}
