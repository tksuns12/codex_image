use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;

use crate::diagnostics::CliError;
use crate::openai::{ImageGenerationResponse, ImageGenerationUsage};

const DEFAULT_IMAGE_FORMAT: &str = "png";
const MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GenerationManifest {
    pub prompt: String,
    pub model: String,
    pub manifest_path: String,
    pub images: Vec<GeneratedImageArtifact>,
    pub response: GenerationResponseMetadata,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GeneratedImageArtifact {
    pub index: usize,
    pub path: String,
    pub format: String,
    pub byte_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GenerationResponseMetadata {
    pub created: i64,
    pub usage: UsageMetadata,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct UsageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
}

impl From<Option<ImageGenerationUsage>> for UsageMetadata {
    fn from(value: Option<ImageGenerationUsage>) -> Self {
        match value {
            Some(usage) => Self {
                total_tokens: usage.total_tokens,
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
            },
            None => Self::default(),
        }
    }
}

pub fn write_generation_output(
    prompt: &str,
    model: &str,
    out_dir: &Path,
    response: &ImageGenerationResponse,
) -> Result<GenerationManifest, CliError> {
    if response.data.is_empty() {
        return Err(CliError::ImageGenerationResponseContract {
            source_message: "image generation response missing data".to_string(),
        });
    }

    fs::create_dir_all(out_dir).map_err(|_| CliError::OutputWriteFailed)?;

    let mut images = Vec::with_capacity(response.data.len());

    for (idx, image) in response.data.iter().enumerate() {
        let format = image
            .output_format
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_IMAGE_FORMAT)
            .to_ascii_lowercase();

        if !is_safe_format(&format) {
            return Err(CliError::ImageGenerationResponseContract {
                source_message: "image generation response contains invalid output_format"
                    .to_string(),
            });
        }

        let image_bytes = STANDARD.decode(image.b64_json.as_bytes()).map_err(|_| {
            CliError::ImageGenerationResponseContract {
                source_message: "image generation response contains invalid b64_json".to_string(),
            }
        })?;

        let image_name = format!("image-{index:04}.{format}", index = idx + 1, format = format);
        let image_path = out_dir.join(image_name);
        atomic_write_bytes(&image_path, &image_bytes).map_err(|_| CliError::OutputWriteFailed)?;

        if !image_path.is_file() {
            return Err(CliError::OutputVerificationFailed);
        }

        images.push(GeneratedImageArtifact {
            index: idx + 1,
            path: path_to_string(&image_path),
            format,
            byte_count: image_bytes.len(),
            size: image.size.clone(),
            quality: image.quality.clone(),
            background: image.background.clone(),
        });
    }

    let manifest_path = out_dir.join(MANIFEST_FILE);
    let manifest = GenerationManifest {
        prompt: prompt.to_string(),
        model: model.to_string(),
        manifest_path: path_to_string(&manifest_path),
        images,
        response: GenerationResponseMetadata {
            created: response.created,
            usage: response.usage.clone().into(),
        },
    };

    let manifest_json = serde_json::to_vec_pretty(&manifest).map_err(|_| CliError::OutputWriteFailed)?;
    atomic_write_bytes(&manifest_path, &manifest_json).map_err(|_| CliError::OutputWriteFailed)?;

    if !manifest_path.is_file() {
        return Err(CliError::OutputVerificationFailed);
    }

    Ok(manifest)
}

fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "output path does not have a parent directory",
        )
    })?;

    let mut attempt = 0_u32;
    let pid = std::process::id();

    loop {
        let tmp_path = parent.join(format!(
            ".{}.tmp-{}-{}",
            path.file_name().and_then(|n| n.to_str()).unwrap_or("output"),
            pid,
            attempt
        ));

        match fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&tmp_path)
        {
            Ok(mut file) => {
                file.write_all(bytes)?;
                file.sync_all()?;
                drop(file);

                fs::rename(&tmp_path, path)?;
                return Ok(());
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                attempt += 1;
                if attempt > 100 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "failed to allocate temporary output file",
                    ));
                }
            }
            Err(err) => return Err(err),
        }
    }
}

fn is_safe_format(format: &str) -> bool {
    !format.is_empty() && format.chars().all(|c| c.is_ascii_alphanumeric())
}

fn path_to_string(path: &PathBuf) -> String {
    path.to_string_lossy().into_owned()
}
