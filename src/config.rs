use std::env;
use std::path::PathBuf;

use reqwest::Url;

pub const ENV_AUTH_FILE: &str = "CODEX_IMAGE_AUTH_FILE";
pub const ENV_HOME: &str = "CODEX_IMAGE_HOME";
pub const ENV_AUTH_BASE_URL: &str = "CODEX_IMAGE_AUTH_BASE_URL";
pub const ENV_CLIENT_ID: &str = "CODEX_IMAGE_CLIENT_ID";

const DEFAULT_AUTH_BASE_URL: &str = "https://api.openai.com";
const DEFAULT_CLIENT_ID: &str = "codex-image";

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub auth_file: Option<PathBuf>,
    pub home_dir: Option<PathBuf>,
    pub auth_base_url: Url,
    pub client_id: String,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let auth_file = read_non_empty_path(ENV_AUTH_FILE)?;
        let home_dir = read_non_empty_path(ENV_HOME)?;

        let base_url =
            env::var(ENV_AUTH_BASE_URL).unwrap_or_else(|_| DEFAULT_AUTH_BASE_URL.to_string());
        let auth_base_url = Url::parse(&base_url).map_err(|_| ConfigError::InvalidValue {
            key: ENV_AUTH_BASE_URL,
        })?;

        let client_id = env::var(ENV_CLIENT_ID).unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string());
        if client_id.trim().is_empty() {
            return Err(ConfigError::InvalidValue { key: ENV_CLIENT_ID });
        }

        Ok(Self {
            auth_file,
            home_dir,
            auth_base_url,
            client_id,
        })
    }
}

fn read_non_empty_path(key: &'static str) -> Result<Option<PathBuf>, ConfigError> {
    match env::var(key) {
        Ok(raw) if raw.trim().is_empty() => Err(ConfigError::InvalidValue { key }),
        Ok(raw) => Ok(Some(PathBuf::from(raw))),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(ConfigError::InvalidValue { key }),
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("invalid configuration")]
    InvalidValue { key: &'static str },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_auth_base_url() {
        std::env::set_var(ENV_AUTH_BASE_URL, "::not-a-url::");
        let result = AuthConfig::from_env();
        std::env::remove_var(ENV_AUTH_BASE_URL);

        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue {
                key: ENV_AUTH_BASE_URL
            })
        ));
    }

    #[test]
    fn rejects_empty_auth_path_env() {
        std::env::set_var(ENV_AUTH_FILE, "   ");
        let result = AuthConfig::from_env();
        std::env::remove_var(ENV_AUTH_FILE);

        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue { key: ENV_AUTH_FILE })
        ));
    }
}
