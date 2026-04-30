use std::env;
use std::path::PathBuf;

use reqwest::Url;

pub const ENV_AUTH_FILE: &str = "CODEX_IMAGE_AUTH_FILE";
pub const ENV_HOME: &str = "CODEX_IMAGE_HOME";
pub const ENV_AUTH_BASE_URL: &str = "CODEX_IMAGE_AUTH_BASE_URL";
pub const ENV_API_BASE_URL: &str = "CODEX_IMAGE_API_BASE_URL";
pub const ENV_CLIENT_ID: &str = "CODEX_IMAGE_CLIENT_ID";

const DEFAULT_AUTH_BASE_URL: &str = "https://auth.openai.com";
const DEFAULT_API_BASE_URL: &str = "https://api.openai.com";
const DEFAULT_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub auth_file: Option<PathBuf>,
    pub home_dir: Option<PathBuf>,
    pub auth_base_url: Url,
    pub client_id: String,
}

#[derive(Debug, Clone)]
pub struct GenerateConfig {
    pub api_base_url: Url,
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

    pub fn from_env_for_store() -> Result<Self, ConfigError> {
        let auth_file = read_non_empty_path(ENV_AUTH_FILE)?;
        let home_dir = read_non_empty_path(ENV_HOME)?;

        Ok(Self {
            auth_file,
            home_dir,
            auth_base_url: Url::parse(DEFAULT_AUTH_BASE_URL)
                .expect("default auth base URL must be valid"),
            client_id: DEFAULT_CLIENT_ID.to_string(),
        })
    }
}

impl GenerateConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let base_url =
            env::var(ENV_API_BASE_URL).unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string());
        let api_base_url = Url::parse(&base_url).map_err(|_| ConfigError::InvalidValue {
            key: ENV_API_BASE_URL,
        })?;

        Ok(Self { api_base_url })
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
    use std::sync::{Mutex, OnceLock};

    use super::*;

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn rejects_invalid_auth_base_url() {
        let _guard = env_lock();
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
        let _guard = env_lock();
        std::env::set_var(ENV_AUTH_FILE, "   ");
        let result = AuthConfig::from_env();
        std::env::remove_var(ENV_AUTH_FILE);

        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue { key: ENV_AUTH_FILE })
        ));
    }

    #[test]
    fn rejects_invalid_generate_api_base_url() {
        let _guard = env_lock();
        std::env::set_var(ENV_API_BASE_URL, "::not-a-url::");
        let result = GenerateConfig::from_env();
        std::env::remove_var(ENV_API_BASE_URL);

        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue {
                key: ENV_API_BASE_URL
            })
        ));
    }

    #[test]
    fn auth_base_url_defaults_to_openai_auth() {
        let _guard = env_lock();
        std::env::remove_var(ENV_AUTH_BASE_URL);
        std::env::remove_var(ENV_CLIENT_ID);

        let result = AuthConfig::from_env().expect("default auth config should parse");

        assert_eq!(result.auth_base_url.as_str(), "https://auth.openai.com/");
    }

    #[test]
    fn generate_api_base_url_defaults_to_openai_api() {
        let _guard = env_lock();
        std::env::remove_var(ENV_API_BASE_URL);

        let result = GenerateConfig::from_env().expect("default generate config should parse");

        assert_eq!(result.api_base_url.as_str(), "https://api.openai.com/");
    }

    #[test]
    fn store_config_ignores_login_only_env_values() {
        let _guard = env_lock();
        std::env::set_var(ENV_AUTH_BASE_URL, "::not-a-url::");
        std::env::set_var(ENV_API_BASE_URL, "::also-not-a-url::");
        std::env::set_var(ENV_CLIENT_ID, "   ");

        let result = AuthConfig::from_env_for_store();

        std::env::remove_var(ENV_AUTH_BASE_URL);
        std::env::remove_var(ENV_API_BASE_URL);
        std::env::remove_var(ENV_CLIENT_ID);

        assert!(result.is_ok());
    }
}
