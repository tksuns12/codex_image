use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use crate::auth::state::PersistedAuth;
use crate::config::AuthConfig;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("failed to resolve auth path")]
    ResolvePath,
    #[error("failed to read auth file")]
    Read,
    #[error("failed to parse auth file")]
    Parse,
    #[error("failed to persist auth file")]
    Persist,
    #[error("failed to serialize auth state")]
    Serialize,
}

#[derive(Debug, Clone)]
pub struct AuthStore {
    path: PathBuf,
}

impl AuthStore {
    pub fn from_config(config: &AuthConfig) -> Result<Self, StoreError> {
        let path = resolve_auth_path(config)?;
        Ok(Self { path })
    }

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Option<PersistedAuth>, StoreError> {
        if !self.path.exists() {
            return Ok(None);
        }

        let file = File::open(&self.path).map_err(|_| StoreError::Read)?;
        let reader = BufReader::new(file);
        let auth = serde_json::from_reader(reader).map_err(|_| StoreError::Parse)?;
        Ok(Some(auth))
    }

    pub fn save(&self, auth: &PersistedAuth) -> Result<(), StoreError> {
        let serialized = serde_json::to_vec_pretty(auth).map_err(|_| StoreError::Serialize)?;
        atomic_write(&self.path, &serialized)
    }

    pub fn clear(&self) -> Result<(), StoreError> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(StoreError::Persist),
        }
    }
}

pub fn resolve_auth_path(config: &AuthConfig) -> Result<PathBuf, StoreError> {
    if let Some(path) = &config.auth_file {
        return Ok(path.clone());
    }

    if let Some(home) = &config.home_dir {
        return Ok(home.join("auth.json"));
    }

    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or(StoreError::ResolvePath)?;
    Ok(data_home.join("codex-image").join("auth.json"))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let parent = path.parent().ok_or(StoreError::Persist)?;

    fs::create_dir_all(parent).map_err(|_| StoreError::Persist)?;
    set_dir_permissions(parent)?;

    let mut temp_path = path.to_path_buf();
    temp_path.set_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|_| StoreError::Persist)?;

    set_file_permissions(&file)?;

    file.write_all(bytes).map_err(|_| StoreError::Persist)?;
    file.flush().map_err(|_| StoreError::Persist)?;
    file.sync_all().map_err(|_| StoreError::Persist)?;
    drop(file);

    fs::rename(&temp_path, path).map_err(|_| {
        let _ = fs::remove_file(&temp_path);
        StoreError::Persist
    })?;

    Ok(())
}

fn set_file_permissions(file: &File) -> Result<(), StoreError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| StoreError::Persist)?;
    }

    #[cfg(not(unix))]
    {
        let _ = file;
    }

    Ok(())
}

fn set_dir_permissions(dir: &Path) -> Result<(), StoreError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
            .map_err(|_| StoreError::Persist)?;
    }

    #[cfg(not(unix))]
    {
        let _ = dir;
    }

    Ok(())
}
