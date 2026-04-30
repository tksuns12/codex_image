use std::fs;

use codex_image::diagnostics::CliError;
use codex_image::output::write_generation_output_from_files;
use tempfile::{tempdir, NamedTempFile};

#[test]
fn output_writes_single_image_and_manifest_with_expected_contract() {
    let temp = tempdir().expect("tempdir should create");
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("generated.png");
    fs::write(&source, b"png-bytes").unwrap();
    let out_dir = temp.path().join("images");

    let manifest =
        write_generation_output_from_files("sunrise", "gpt-image-2", &out_dir, &[source])
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
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let sources = [
        source_dir.join("first.png"),
        source_dir.join("second.webp"),
        source_dir.join("third.jpeg"),
    ];
    fs::write(&sources[0], b"first").unwrap();
    fs::write(&sources[1], b"second").unwrap();
    fs::write(&sources[2], b"third").unwrap();
    let out_dir = temp.path().join("images");

    let manifest = write_generation_output_from_files("multi", "gpt-image-2", &out_dir, &sources)
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
fn output_manifest_redacts_source_path_and_token_sentinels() {
    let temp = tempdir().expect("tempdir should create");
    let source_dir = temp.path().join("source-access-token-Bearer");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("generated-b64_json.png");
    fs::write(
        &source,
        b"binary access-token refresh-token id-token Bearer b64_json",
    )
    .unwrap();
    let out_dir = temp.path().join("images");

    let manifest =
        write_generation_output_from_files("safe prompt", "gpt-image-2", &out_dir, &[source])
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
fn output_empty_image_list_maps_to_response_contract_error() {
    let temp = tempdir().expect("tempdir should create");
    let out_dir = temp.path().join("images");

    let err = write_generation_output_from_files("empty", "gpt-image-2", &out_dir, &[])
        .expect_err("empty image list must fail");

    assert!(matches!(
        err,
        CliError::ImageGenerationResponseContract { .. }
    ));
}

#[test]
fn output_missing_source_maps_to_response_contract_error() {
    let temp = tempdir().expect("tempdir should create");
    let out_dir = temp.path().join("images");
    let missing = temp.path().join("missing.png");

    let err = write_generation_output_from_files("missing", "gpt-image-2", &out_dir, &[missing])
        .expect_err("missing source must fail");

    assert!(matches!(
        err,
        CliError::ImageGenerationResponseContract { .. }
    ));
}

#[test]
fn output_existing_file_target_maps_to_filesystem_error() {
    let temp = tempdir().expect("tempdir should create");
    let source = temp.path().join("source.png");
    fs::write(&source, b"bytes").unwrap();
    let file = NamedTempFile::new().expect("file should create");
    let out_path = file.path();

    let err = write_generation_output_from_files("prompt", "gpt-image-2", out_path, &[source])
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
