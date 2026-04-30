use std::fs;

use base64::{engine::general_purpose::STANDARD, Engine};
use codex_image::diagnostics::CliError;
use codex_image::openai::{GeneratedImage, ImageGenerationResponse, ImageGenerationUsage};
use codex_image::output::write_generation_output;
use tempfile::{tempdir, NamedTempFile};

fn image_entry(bytes: &[u8], output_format: Option<&str>) -> GeneratedImage {
    GeneratedImage {
        b64_json: STANDARD.encode(bytes),
        revised_prompt: Some("revised prompt".to_string()),
        size: Some("1024x1024".to_string()),
        quality: Some("high".to_string()),
        background: Some("transparent".to_string()),
        output_format: output_format.map(ToString::to_string),
    }
}

fn response_with_images(images: Vec<GeneratedImage>) -> ImageGenerationResponse {
    ImageGenerationResponse {
        created: 1_746_000_123,
        data: images,
        usage: Some(ImageGenerationUsage {
            total_tokens: Some(7),
            input_tokens: Some(5),
            output_tokens: Some(2),
        }),
    }
}

#[test]
fn output_writes_single_image_and_manifest_with_expected_contract() {
    let temp = tempdir().expect("tempdir should create");
    let out_dir = temp.path().join("images");

    let response = response_with_images(vec![image_entry(b"png-bytes", None)]);

    let manifest = write_generation_output("sunrise", "gpt-image-2", &out_dir, &response)
        .expect("output write should succeed");

    let image_path = out_dir.join("image-0001.png");
    let manifest_path = out_dir.join("manifest.json");

    assert!(image_path.exists(), "image file should exist");
    assert!(manifest_path.exists(), "manifest file should exist");
    assert_eq!(fs::read(&image_path).unwrap(), b"png-bytes");

    assert_eq!(manifest.prompt, "sunrise");
    assert_eq!(manifest.model, "gpt-image-2");
    assert_eq!(manifest.images.len(), 1);
    assert_eq!(manifest.images[0].index, 1);
    assert_eq!(manifest.images[0].format, "png");
    assert_eq!(manifest.images[0].byte_count, 9);
    assert_eq!(manifest.images[0].path, image_path.to_string_lossy());
    assert_eq!(manifest.manifest_path, manifest_path.to_string_lossy());
    assert_eq!(manifest.response.created, 1_746_000_123);
    assert_eq!(manifest.response.usage.total_tokens, Some(7));

    let manifest_text = fs::read_to_string(&manifest_path).unwrap();
    let manifest_json: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(manifest_json["prompt"], "sunrise");
    assert_eq!(manifest_json["model"], "gpt-image-2");
    assert_eq!(
        manifest_json["images"][0]["path"],
        image_path.to_string_lossy().to_string()
    );
}

#[test]
fn output_writes_multiple_images_with_deterministic_filenames() {
    let temp = tempdir().expect("tempdir should create");
    let out_dir = temp.path().join("images");

    let response = response_with_images(vec![
        image_entry(b"first", Some("png")),
        image_entry(b"second", Some("webp")),
        image_entry(b"third", Some("jpeg")),
    ]);

    let manifest = write_generation_output("multi", "gpt-image-2", &out_dir, &response)
        .expect("output write should succeed");

    let expected = [
        out_dir.join("image-0001.png"),
        out_dir.join("image-0002.webp"),
        out_dir.join("image-0003.jpeg"),
    ];

    for (idx, path) in expected.iter().enumerate() {
        assert!(path.exists(), "{} should exist", path.display());
        assert_eq!(manifest.images[idx].path, path.to_string_lossy());
        assert_eq!(manifest.images[idx].index, idx + 1);
    }
}

#[test]
fn output_manifest_redacts_b64_and_token_sentinels() {
    let temp = tempdir().expect("tempdir should create");
    let out_dir = temp.path().join("images");

    let response = response_with_images(vec![GeneratedImage {
        b64_json: STANDARD.encode(b"binary access-token refresh-token id-token Bearer b64_json"),
        revised_prompt: Some("revised access-token Bearer".to_string()),
        size: None,
        quality: None,
        background: None,
        output_format: None,
    }]);

    let manifest = write_generation_output("safe prompt", "gpt-image-2", &out_dir, &response)
        .expect("output write should succeed");

    let manifest_text = fs::read_to_string(out_dir.join("manifest.json")).unwrap();
    let json_text = serde_json::to_string(&manifest).unwrap();

    for forbidden in [
        "b64_json",
        "access-token",
        "refresh-token",
        "id-token",
        "Bearer",
    ] {
        assert!(
            !manifest_text.contains(forbidden),
            "manifest should not contain {forbidden}"
        );
        assert!(
            !json_text.contains(forbidden),
            "serialized contract should not contain {forbidden}"
        );
    }
}

#[test]
fn output_invalid_base64_maps_to_response_contract_error() {
    let temp = tempdir().expect("tempdir should create");
    let out_dir = temp.path().join("images");

    let response = response_with_images(vec![GeneratedImage {
        b64_json: "%%% not-base64 %%%".to_string(),
        revised_prompt: None,
        size: None,
        quality: None,
        background: None,
        output_format: None,
    }]);

    let err = write_generation_output("bad", "gpt-image-2", &out_dir, &response)
        .expect_err("invalid base64 must fail");

    assert!(matches!(
        err,
        CliError::ImageGenerationResponseContract { .. }
    ));
    assert_eq!(
        err.error_envelope().error.code,
        "response_contract.image_generation"
    );
}

#[test]
fn output_empty_image_list_maps_to_response_contract_error() {
    let temp = tempdir().expect("tempdir should create");
    let out_dir = temp.path().join("images");

    let response = response_with_images(vec![]);

    let err = write_generation_output("empty", "gpt-image-2", &out_dir, &response)
        .expect_err("empty image list must fail");

    assert!(matches!(
        err,
        CliError::ImageGenerationResponseContract { .. }
    ));
}

#[test]
fn output_existing_file_target_maps_to_filesystem_error() {
    let file = NamedTempFile::new().expect("file should create");
    let out_path = file.path();

    let response = response_with_images(vec![image_entry(b"bytes", None)]);

    let err = write_generation_output("prompt", "gpt-image-2", out_path, &response)
        .expect_err("existing file path should fail");

    assert!(matches!(
        err,
        CliError::OutputWriteFailed | CliError::OutputVerificationFailed
    ));
    assert_eq!(
        err.error_envelope().error.code,
        "filesystem.output_write_failed"
    );
}
