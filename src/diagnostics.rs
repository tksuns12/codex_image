use serde::Serialize;

use crate::config::ConfigError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    UsageOrConfig,
    Api,
    Filesystem,
    ResponseContract,
    Unknown,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        match self {
            Self::UsageOrConfig => 2,
            Self::Api => 4,
            Self::Filesystem => 5,
            Self::ResponseContract => 6,
            Self::Unknown => 1,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ErrorEnvelope {
    pub error: ErrorDetails,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ErrorDetails {
    pub code: &'static str,
    pub message: &'static str,
    pub recoverable: bool,
    pub hint: &'static str,
}

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("configuration error")]
    Config(#[from] ConfigError),
    #[error("generated output write failed")]
    OutputWriteFailed,
    #[error("generated output verification failed")]
    OutputVerificationFailed,
    #[error("image generation response contract failure")]
    ImageGenerationResponseContract { source_message: String },
    #[error("Codex CLI is unavailable")]
    CodexCliUnavailable,
    #[error("Codex image generation failed")]
    CodexImageGenerationFailed { source_message: String },
    #[error("missing required skill install confirmation")]
    MissingInstallConfirmation,
    #[error("HOME is unavailable")]
    HomeUnavailable,
    #[error("current working directory is unavailable")]
    ProjectRootUnavailable,
    #[error("skill install write failed")]
    SkillInstallWriteFailed,
    #[error("skill install blocked by existing manual edit")]
    SkillInstallBlockedManualEdit,
    #[error("unexpected failure")]
    Unknown,
}

impl CliError {
    pub fn exit_code(&self) -> ExitCode {
        self.classification().exit_code
    }

    pub fn error_envelope(&self) -> ErrorEnvelope {
        let class = self.classification();
        ErrorEnvelope {
            error: ErrorDetails {
                code: class.code,
                message: class.message,
                recoverable: class.recoverable,
                hint: class.hint,
            },
        }
    }

    pub fn redacted_message(&self) -> &'static str {
        self.classification().message
    }

    fn classification(&self) -> ErrorClassification {
        match self {
            Self::Config(_) => ErrorClassification {
                code: "config.invalid",
                message: "configuration error",
                recoverable: true,
                hint: "Check CODEX_IMAGE_* configuration values.",
                exit_code: ExitCode::UsageOrConfig,
            },
            Self::OutputWriteFailed | Self::OutputVerificationFailed => ErrorClassification {
                code: "filesystem.output_write_failed",
                message: "failed to write generated image output",
                recoverable: true,
                hint: "Ensure output paths are writable and retry.",
                exit_code: ExitCode::Filesystem,
            },
            Self::ImageGenerationResponseContract { .. } => ErrorClassification {
                code: "response_contract.image_generation",
                message: "image generation response did not match expected schema",
                recoverable: false,
                hint: "Try again; if it persists, report the issue with request context.",
                exit_code: ExitCode::ResponseContract,
            },
            Self::CodexCliUnavailable => ErrorClassification {
                code: "config.codex_cli_unavailable",
                message: "Codex CLI executable was not found",
                recoverable: true,
                hint: "Install Codex or set CODEX_IMAGE_CODEX_BIN to the Codex executable.",
                exit_code: ExitCode::UsageOrConfig,
            },
            Self::CodexImageGenerationFailed { .. } => ErrorClassification {
                code: "api.codex_image_generation_failed",
                message: "Codex image generation failed",
                recoverable: true,
                hint: "Retry generation in a moment, or verify Codex is installed and logged in.",
                exit_code: ExitCode::Api,
            },
            Self::MissingInstallConfirmation => ErrorClassification {
                code: "usage.install_confirmation_required",
                message: "skill install requires explicit confirmation",
                recoverable: true,
                hint: "Re-run with --yes to confirm non-interactive installation.",
                exit_code: ExitCode::UsageOrConfig,
            },
            Self::HomeUnavailable => ErrorClassification {
                code: "config.home_unavailable",
                message: "HOME is unavailable for global skill installation",
                recoverable: true,
                hint: "Set HOME to a writable directory and retry.",
                exit_code: ExitCode::UsageOrConfig,
            },
            Self::ProjectRootUnavailable => ErrorClassification {
                code: "config.project_root_unavailable",
                message: "current directory is unavailable",
                recoverable: true,
                hint: "Run the command from an accessible project directory and retry.",
                exit_code: ExitCode::UsageOrConfig,
            },
            Self::SkillInstallWriteFailed => ErrorClassification {
                code: "filesystem.skill_install_write_failed",
                message: "failed to write managed SKILL.md",
                recoverable: true,
                hint: "Ensure target directories are writable and retry.",
                exit_code: ExitCode::Filesystem,
            },
            Self::SkillInstallBlockedManualEdit => ErrorClassification {
                code: "filesystem.skill_install_blocked_manual_edit",
                message: "existing SKILL.md is manual or tampered",
                recoverable: true,
                hint: "Re-run with --force to overwrite the existing file.",
                exit_code: ExitCode::Filesystem,
            },
            Self::Unknown => ErrorClassification {
                code: "unknown",
                message: "unexpected failure",
                recoverable: false,
                hint: "Re-run with supported commands or update the binary.",
                exit_code: ExitCode::Unknown,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ErrorClassification {
    code: &'static str,
    message: &'static str,
    recoverable: bool,
    hint: &'static str,
    exit_code: ExitCode,
}
