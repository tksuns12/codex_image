use reqwest::{Client, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};

use crate::diagnostics::CliError;

pub const GPT_IMAGE_MODEL: &str = "gpt-image-2";

const DEFAULT_OUTPUT_FORMAT: &str = "png";
const IMAGE_GENERATION_SCOPE: &str = "api.model.images.request";
const ERROR_BODY_CLASSIFICATION_LIMIT_BYTES: u64 = 8 * 1024;

#[derive(Debug, Clone)]
pub struct ImageGenerationRequest {
    pub prompt: String,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub background: Option<String>,
    pub output_format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageGenerationResponse {
    pub created: i64,
    pub data: Vec<GeneratedImage>,
    pub usage: Option<ImageGenerationUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedImage {
    pub b64_json: String,
    pub revised_prompt: Option<String>,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub background: Option<String>,
    pub output_format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ImageGenerationUsage {
    pub total_tokens: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Debug, Serialize)]
struct WireImageGenerationRequest<'a> {
    model: &'static str,
    prompt: &'a str,
    output_format: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct WireImageGenerationResponse {
    created: i64,
    data: Vec<WireGeneratedImage>,
    #[serde(default)]
    usage: Option<ImageGenerationUsage>,
}

#[derive(Debug, Deserialize)]
struct WireGeneratedImage {
    b64_json: Option<String>,
    #[serde(default)]
    revised_prompt: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    quality: Option<String>,
    #[serde(default)]
    background: Option<String>,
    #[serde(default)]
    output_format: Option<String>,
}

async fn read_error_body_prefix(response: Response) -> Option<String> {
    if response
        .content_length()
        .is_some_and(|length| length > ERROR_BODY_CLASSIFICATION_LIMIT_BYTES)
    {
        return None;
    }

    let bytes = response.bytes().await.ok()?;
    let limit = usize::try_from(ERROR_BODY_CLASSIFICATION_LIMIT_BYTES).ok()?;
    let prefix_len = bytes.len().min(limit);

    Some(String::from_utf8_lossy(&bytes[..prefix_len]).into_owned())
}

fn is_missing_image_scope_error(status: StatusCode, body: &str) -> bool {
    matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN)
        && body.contains("Missing scopes:")
        && body.contains(IMAGE_GENERATION_SCOPE)
}

pub async fn generate_image(
    client: &Client,
    api_base_url: &Url,
    bearer_token: &str,
    request: &ImageGenerationRequest,
) -> Result<ImageGenerationResponse, CliError> {
    let endpoint =
        api_base_url
            .join("v1/images/generations")
            .map_err(|_| CliError::ImageGenerationApi {
                source_message: "invalid image API base URL".to_string(),
            })?;

    let payload = WireImageGenerationRequest {
        model: GPT_IMAGE_MODEL,
        prompt: request.prompt.as_str(),
        output_format: request
            .output_format
            .as_deref()
            .unwrap_or(DEFAULT_OUTPUT_FORMAT),
        size: request.size.as_deref(),
        quality: request.quality.as_deref(),
        background: request.background.as_deref(),
    };

    let response = client
        .post(endpoint)
        .bearer_auth(bearer_token)
        .json(&payload)
        .send()
        .await
        .map_err(|err| {
            if err.is_timeout() {
                CliError::ImageGenerationTimeout {
                    source_message: "request timed out".to_string(),
                }
            } else {
                CliError::ImageGenerationApi {
                    source_message: "request transport failure".to_string(),
                }
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body_prefix = read_error_body_prefix(response).await;
        if body_prefix
            .as_deref()
            .is_some_and(|body| is_missing_image_scope_error(status, body))
        {
            return Err(CliError::AuthInsufficientScope);
        }

        return Err(CliError::ImageGenerationApi {
            source_message: format!("upstream status {status}"),
        });
    }

    let parsed: WireImageGenerationResponse = response.json().await.map_err(|err| {
        if err.is_timeout() {
            CliError::ImageGenerationTimeout {
                source_message: "response parse timed out".to_string(),
            }
        } else {
            CliError::ImageGenerationResponseContract {
                source_message: "invalid image generation response JSON".to_string(),
            }
        }
    })?;

    if parsed.data.is_empty() {
        return Err(CliError::ImageGenerationResponseContract {
            source_message: "image generation response missing data".to_string(),
        });
    }

    let data = parsed
        .data
        .into_iter()
        .map(|entry| {
            let b64_json =
                entry
                    .b64_json
                    .ok_or_else(|| CliError::ImageGenerationResponseContract {
                        source_message: "image generation response missing b64_json".to_string(),
                    })?;

            if b64_json.trim().is_empty() {
                return Err(CliError::ImageGenerationResponseContract {
                    source_message: "image generation response contains empty b64_json".to_string(),
                });
            }

            Ok(GeneratedImage {
                b64_json,
                revised_prompt: entry.revised_prompt,
                size: entry.size,
                quality: entry.quality,
                background: entry.background,
                output_format: entry.output_format,
            })
        })
        .collect::<Result<Vec<_>, CliError>>()?;

    Ok(ImageGenerationResponse {
        created: parsed.created,
        data,
        usage: parsed.usage,
    })
}
