use std::env::consts;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    TarGz,
    Zip,
}

impl ArchiveKind {
    fn extension(self) -> &'static str {
        match self {
            Self::TarGz => "tar.gz",
            Self::Zip => "zip",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Platform {
    os: &'static str,
    arch: &'static str,
    rust_target: &'static str,
    binary_name: &'static str,
    archive_kind: ArchiveKind,
}

impl Platform {
    pub fn new(os: &str, arch: &str) -> Result<Self, UpdateError> {
        platform_from_parts(os, arch)
    }

    pub fn os(&self) -> &'static str {
        self.os
    }

    pub fn arch(&self) -> &'static str {
        self.arch
    }

    pub fn rust_target(&self) -> &'static str {
        self.rust_target
    }

    pub fn binary_name(&self) -> &'static str {
        self.binary_name
    }

    pub fn archive_kind(&self) -> ArchiveKind {
        self.archive_kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseMetadata {
    pub tag_name: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedArtifact {
    pub version: String,
    pub platform_target: String,
    pub asset_name: String,
    pub download_url: String,
    pub archive_kind: ArchiveKind,
    pub binary_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedArchive {
    pub binary_path: PathBuf,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UpdateError {
    #[error("unsupported platform")]
    UnsupportedPlatform,
    #[error("release metadata is invalid")]
    ReleaseMetadataInvalid,
    #[error("missing required release asset")]
    MissingReleaseAsset,
    #[error("duplicate release asset for platform")]
    DuplicateReleaseAsset,
    #[error("archive payload is invalid")]
    ArchiveInvalid,
    #[error("archive contains path traversal")]
    ArchivePathTraversal,
    #[error("archive top-level directory mismatch")]
    ArchiveTopLevelDirectoryMismatch,
    #[error("archive missing required file")]
    ArchiveMissingRequiredFile,
    #[error("archive contains duplicate binary")]
    ArchiveDuplicateBinary,
}

pub fn current_platform() -> Result<Platform, UpdateError> {
    platform_from_parts(consts::OS, consts::ARCH)
}

pub fn platform_from_parts(os: &str, arch: &str) -> Result<Platform, UpdateError> {
    match (os, arch) {
        ("linux", "x86_64") => Ok(Platform {
            os: "linux",
            arch: "x86_64",
            rust_target: "x86_64-unknown-linux-gnu",
            binary_name: "codex-image",
            archive_kind: ArchiveKind::TarGz,
        }),
        ("macos", "x86_64") => Ok(Platform {
            os: "macos",
            arch: "x86_64",
            rust_target: "x86_64-apple-darwin",
            binary_name: "codex-image",
            archive_kind: ArchiveKind::TarGz,
        }),
        ("macos", "aarch64") => Ok(Platform {
            os: "macos",
            arch: "aarch64",
            rust_target: "aarch64-apple-darwin",
            binary_name: "codex-image",
            archive_kind: ArchiveKind::TarGz,
        }),
        ("windows", "x86_64") => Ok(Platform {
            os: "windows",
            arch: "x86_64",
            rust_target: "x86_64-pc-windows-msvc",
            binary_name: "codex-image.exe",
            archive_kind: ArchiveKind::Zip,
        }),
        _ => Err(UpdateError::UnsupportedPlatform),
    }
}

pub fn release_asset_name_for_tag(tag_name: &str, platform: &Platform) -> String {
    format!(
        "codex-image-{tag_name}-{}.{}",
        platform.rust_target,
        platform.archive_kind.extension()
    )
}

pub fn parse_release_metadata(input: &str) -> Result<ReleaseMetadata, UpdateError> {
    let parsed: GithubRelease =
        serde_json::from_str(input).map_err(|_| UpdateError::ReleaseMetadataInvalid)?;

    if parsed.tag_name.trim().is_empty() {
        return Err(UpdateError::ReleaseMetadataInvalid);
    }

    let assets = parsed
        .assets
        .into_iter()
        .map(|asset| ReleaseAsset {
            name: asset.name,
            download_url: asset.browser_download_url,
        })
        .collect();

    Ok(ReleaseMetadata {
        tag_name: parsed.tag_name,
        assets,
    })
}

pub fn resolve_artifact(
    metadata: &ReleaseMetadata,
    platform: &Platform,
) -> Result<ResolvedArtifact, UpdateError> {
    let expected = release_asset_name_for_tag(&metadata.tag_name, platform);

    let mut matches = metadata
        .assets
        .iter()
        .filter(|asset| asset.name == expected);
    let first = matches.next().ok_or(UpdateError::MissingReleaseAsset)?;

    if matches.next().is_some() {
        return Err(UpdateError::DuplicateReleaseAsset);
    }

    Ok(ResolvedArtifact {
        version: metadata.tag_name.clone(),
        platform_target: platform.rust_target.to_string(),
        asset_name: first.name.clone(),
        download_url: first.download_url.clone(),
        archive_kind: platform.archive_kind,
        binary_name: platform.binary_name.to_string(),
    })
}

pub fn validate_archive_bytes(
    archive_kind: ArchiveKind,
    bytes: &[u8],
    expected_top_level_dir: &str,
    expected_binary_name: &str,
) -> Result<ValidatedArchive, UpdateError> {
    match archive_kind {
        ArchiveKind::TarGz => validate_tar_gz_archive(
            bytes,
            expected_top_level_dir,
            expected_binary_name,
            archive_kind,
        ),
        ArchiveKind::Zip => validate_zip_archive(
            bytes,
            expected_top_level_dir,
            expected_binary_name,
            archive_kind,
        ),
    }
}

fn validate_tar_gz_archive(
    bytes: &[u8],
    expected_top_level_dir: &str,
    expected_binary_name: &str,
    archive_kind: ArchiveKind,
) -> Result<ValidatedArchive, UpdateError> {
    let decoder = flate2::read::GzDecoder::new(Cursor::new(bytes));
    let mut archive = tar::Archive::new(decoder);

    let mut entries = archive.entries().map_err(|_| UpdateError::ArchiveInvalid)?;
    let mut state = ValidationState::new(archive_kind, expected_binary_name);

    while let Some(next) = entries.next() {
        let mut entry = next.map_err(|_| UpdateError::ArchiveInvalid)?;
        if !entry.header().entry_type().is_file() {
            continue;
        }

        let path = entry.path().map_err(|_| UpdateError::ArchiveInvalid)?;
        state.observe_path(path.as_ref(), expected_top_level_dir)?;

        let mut sink = [0_u8; 1];
        let _ = entry
            .read(&mut sink)
            .map_err(|_| UpdateError::ArchiveInvalid)?;
    }

    state.finish()
}

fn validate_zip_archive(
    bytes: &[u8],
    expected_top_level_dir: &str,
    expected_binary_name: &str,
    archive_kind: ArchiveKind,
) -> Result<ValidatedArchive, UpdateError> {
    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|_| UpdateError::ArchiveInvalid)?;

    let mut state = ValidationState::new(archive_kind, expected_binary_name);

    for idx in 0..archive.len() {
        let file = archive
            .by_index(idx)
            .map_err(|_| UpdateError::ArchiveInvalid)?;
        if file.is_dir() {
            continue;
        }

        let path = Path::new(file.name());
        state.observe_path(path, expected_top_level_dir)?;
    }

    state.finish()
}

struct ValidationState {
    archive_kind: ArchiveKind,
    expected_binary_name: String,
    binary_path: Option<PathBuf>,
    has_readme: bool,
    has_readme_ko: bool,
}

impl ValidationState {
    fn new(archive_kind: ArchiveKind, expected_binary_name: &str) -> Self {
        Self {
            archive_kind,
            expected_binary_name: expected_binary_name.to_string(),
            binary_path: None,
            has_readme: false,
            has_readme_ko: false,
        }
    }

    fn observe_path(
        &mut self,
        path: &Path,
        expected_top_level_dir: &str,
    ) -> Result<(), UpdateError> {
        let relative = normalize_archive_path(path, expected_top_level_dir)?;

        if relative == Path::new("README.md") {
            self.has_readme = true;
        }

        if relative == Path::new("README.ko.md") {
            self.has_readme_ko = true;
        }

        let file_name = relative.file_name().and_then(|value| value.to_str());
        if file_name == Some(self.expected_binary_name.as_str()) {
            let normalized = Path::new(expected_top_level_dir).join(relative);
            if self.binary_path.is_some() {
                return Err(UpdateError::ArchiveDuplicateBinary);
            }
            self.binary_path = Some(normalized);
        }

        Ok(())
    }

    fn finish(self) -> Result<ValidatedArchive, UpdateError> {
        if !self.has_readme || !self.has_readme_ko {
            return Err(UpdateError::ArchiveMissingRequiredFile);
        }

        let Some(binary_path) = self.binary_path else {
            return Err(UpdateError::ArchiveMissingRequiredFile);
        };

        let _ = self.archive_kind;

        Ok(ValidatedArchive { binary_path })
    }
}

fn normalize_archive_path(
    path: &Path,
    expected_top_level_dir: &str,
) -> Result<PathBuf, UpdateError> {
    let mut components = path.components();

    let Some(first) = components.next() else {
        return Err(UpdateError::ArchiveInvalid);
    };

    let top_level = match first {
        Component::Normal(value) => value,
        Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
            return Err(UpdateError::ArchivePathTraversal)
        }
    };

    if top_level != expected_top_level_dir {
        return Err(UpdateError::ArchiveTopLevelDirectoryMismatch);
    }

    let mut relative = PathBuf::new();
    for component in components {
        match component {
            Component::Normal(value) => relative.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(UpdateError::ArchivePathTraversal)
            }
        }
    }

    if relative.as_os_str().is_empty() {
        return Err(UpdateError::ArchiveInvalid);
    }

    Ok(relative)
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}
