use std::env;
use std::path::PathBuf;

pub const ENV_CODEX_BIN: &str = "CODEX_IMAGE_CODEX_BIN";

pub fn read_non_empty_env_path(key: &'static str) -> Result<Option<PathBuf>, ConfigError> {
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
    fn codex_bin_env_is_optional_but_rejects_empty_value() {
        std::env::remove_var(ENV_CODEX_BIN);
        let result = read_non_empty_env_path(ENV_CODEX_BIN).expect("missing env is ok");
        assert_eq!(result, None);

        std::env::set_var(ENV_CODEX_BIN, "   ");
        let result = read_non_empty_env_path(ENV_CODEX_BIN);
        std::env::remove_var(ENV_CODEX_BIN);

        assert!(matches!(
            result,
            Err(ConfigError::InvalidValue { key: ENV_CODEX_BIN })
        ));
    }
}
