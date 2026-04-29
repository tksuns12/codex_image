use crate::config::ConfigError;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("configuration error")]
    Config(#[from] ConfigError),
    #[error("login flow not implemented")]
    LoginNotImplemented,
}

impl CliError {
    pub fn redacted_message(&self) -> &'static str {
        match self {
            Self::Config(_) => "configuration error",
            Self::LoginNotImplemented => "login flow not implemented",
        }
    }
}
