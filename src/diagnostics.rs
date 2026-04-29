use serde::Serialize;

use crate::auth::device::DeviceLoginError;
use crate::auth::store::StoreError;
use crate::config::ConfigError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    UsageOrConfig,
    Auth,
    Api,
    Filesystem,
    ResponseContract,
    Unknown,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        match self {
            Self::UsageOrConfig => 2,
            Self::Auth => 3,
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
    #[error("auth state error")]
    AuthStore(#[from] StoreError),
    #[error("device login error")]
    DeviceLogin(#[from] DeviceLoginError),
    #[error("login flow not implemented")]
    LoginNotImplemented,
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
            Self::AuthStore(StoreError::Parse) => ErrorClassification {
                code: "auth.invalid_state",
                message: "auth state error",
                recoverable: false,
                hint: "Run `codex-image login` to refresh local auth state.",
                exit_code: ExitCode::Auth,
            },
            Self::AuthStore(StoreError::Read) => ErrorClassification {
                code: "filesystem.auth_read_failed",
                message: "auth state error",
                recoverable: true,
                hint: "Ensure the auth file is readable and retry.",
                exit_code: ExitCode::Filesystem,
            },
            Self::AuthStore(StoreError::Persist) => ErrorClassification {
                code: "filesystem.auth_write_failed",
                message: "auth state error",
                recoverable: true,
                hint: "Ensure the auth directory is writable and retry.",
                exit_code: ExitCode::Filesystem,
            },
            Self::AuthStore(StoreError::ResolvePath) => ErrorClassification {
                code: "filesystem.auth_path_resolution_failed",
                message: "auth state error",
                recoverable: true,
                hint: "Set CODEX_IMAGE_HOME or CODEX_IMAGE_AUTH_FILE and retry.",
                exit_code: ExitCode::Filesystem,
            },
            Self::AuthStore(StoreError::Serialize) => ErrorClassification {
                code: "response_contract.auth_state",
                message: "auth state error",
                recoverable: false,
                hint:
                    "Try logging in again; if it persists, report the issue with request context.",
                exit_code: ExitCode::ResponseContract,
            },
            Self::DeviceLogin(
                DeviceLoginError::UserCodeApi
                | DeviceLoginError::PollApi
                | DeviceLoginError::TokenExchangeApi,
            ) => ErrorClassification {
                code: "api.auth_service_request_failed",
                message: "authentication service request failed",
                recoverable: true,
                hint: "Retry login in a moment.",
                exit_code: ExitCode::Api,
            },
            Self::DeviceLogin(
                DeviceLoginError::UserCodeTimeout
                | DeviceLoginError::PollTimeout
                | DeviceLoginError::TokenExchangeTimeout,
            ) => ErrorClassification {
                code: "api.auth_timeout",
                message: "authentication service request failed",
                recoverable: true,
                hint: "Retry login in a moment.",
                exit_code: ExitCode::Api,
            },
            Self::DeviceLogin(
                DeviceLoginError::UserCodeContract
                | DeviceLoginError::PollContract
                | DeviceLoginError::TokenExchangeContract,
            ) => ErrorClassification {
                code: "response_contract.oauth_token",
                message: "authentication service response did not match expected schema",
                recoverable: false,
                hint:
                    "Try logging in again; if it persists, report the issue with request context.",
                exit_code: ExitCode::ResponseContract,
            },
            Self::DeviceLogin(DeviceLoginError::InstructionWrite) => ErrorClassification {
                code: "filesystem.stderr_write_failed",
                message: "failed to write login instructions",
                recoverable: true,
                hint: "Ensure stdout/stderr are writable and retry.",
                exit_code: ExitCode::Filesystem,
            },
            Self::LoginNotImplemented => ErrorClassification {
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
