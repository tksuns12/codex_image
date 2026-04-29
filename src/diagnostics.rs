use crate::auth::store::StoreError;
use crate::config::ConfigError;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("configuration error")]
    Config(#[from] ConfigError),
    #[error("auth state error")]
    AuthStore(#[from] StoreError),
    #[error("login flow not implemented")]
    LoginNotImplemented,
}

impl CliError {
    pub fn redacted_message(&self) -> &'static str {
        match self {
            Self::Config(_) => "configuration error",
            Self::AuthStore(StoreError::ResolvePath) => "auth path resolution error",
            Self::AuthStore(StoreError::Read) => "auth file read error",
            Self::AuthStore(StoreError::Parse) => "auth file parse error",
            Self::AuthStore(StoreError::Persist) => "auth file write error",
            Self::AuthStore(StoreError::Serialize) => "auth state serialization error",
            Self::LoginNotImplemented => "login flow not implemented",
        }
    }
}
