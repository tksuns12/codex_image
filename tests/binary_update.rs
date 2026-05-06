use std::io::{Cursor, Write};
use std::path::Path;

use codex_image::updater::{
    parse_release_metadata, release_asset_name_for_tag, resolve_artifact, validate_archive_bytes,
    ArchiveKind, Platform, UpdateError,
};
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;
use zip::write::FileOptions;

#[test]
fn release_workflow_targets_map_to_expected_artifact_names() {
    let cases = [
        (
            Platform::new("linux", "x86_64").expect("linux platform"),
            "codex-image-v1.2.3-x86_64-unknown-linux-gnu.tar.gz",
        ),
        (
            Platform::new("macos", "x86_64").expect("macos x86_64 platform"),
            "codex-image-v1.2.3-x86_64-apple-darwin.tar.gz",
        ),
        (
            Platform::new("macos", "aarch64").expect("macos arm platform"),
            "codex-image-v1.2.3-aarch64-apple-darwin.tar.gz",
        ),
        (
            Platform::new("windows", "x86_64").expect("windows platform"),
            "codex-image-v1.2.3-x86_64-pc-windows-msvc.zip",
        ),
    ];

    for (platform, expected_asset_name) in cases {
        let actual = release_asset_name_for_tag("v1.2.3", &platform);
        assert_eq!(actual, expected_asset_name);
    }
}

#[test]
fn unsupported_os_arch_maps_to_typed_error() {
    let err = Platform::new("freebsd", "x86_64").expect_err("unsupported platform must fail");
    assert!(matches!(err, UpdateError::UnsupportedPlatform));
}

#[test]
fn resolves_exactly_one_asset_for_platform() {
    let platform = Platform::new("linux", "x86_64").expect("linux platform");
    let metadata = parse_release_metadata(release_fixture()).expect("fixture parses");

    let resolved = resolve_artifact(&metadata, &platform).expect("linux asset resolves");

    assert_eq!(resolved.version, "v1.2.3");
    assert_eq!(
        resolved.asset_name,
        "codex-image-v1.2.3-x86_64-unknown-linux-gnu.tar.gz"
    );
    assert_eq!(resolved.archive_kind, ArchiveKind::TarGz);
}

#[test]
fn missing_target_asset_returns_typed_error() {
    let platform = Platform::new("macos", "aarch64").expect("macos arm platform");
    let metadata = parse_release_metadata(
        r#"{
            "tag_name": "v1.2.3",
            "assets": [
                {
                    "name": "codex-image-v1.2.3-x86_64-unknown-linux-gnu.tar.gz",
                    "browser_download_url": "https://example.invalid/linux"
                }
            ]
        }"#,
    )
    .expect("fixture parses");

    let err = resolve_artifact(&metadata, &platform).expect_err("missing target must fail");
    assert!(matches!(err, UpdateError::MissingReleaseAsset));
}

#[test]
fn duplicate_target_asset_returns_typed_error() {
    let platform = Platform::new("linux", "x86_64").expect("linux platform");
    let metadata = parse_release_metadata(
        r#"{
            "tag_name": "v1.2.3",
            "assets": [
                {
                    "name": "codex-image-v1.2.3-x86_64-unknown-linux-gnu.tar.gz",
                    "browser_download_url": "https://example.invalid/linux-a"
                },
                {
                    "name": "codex-image-v1.2.3-x86_64-unknown-linux-gnu.tar.gz",
                    "browser_download_url": "https://example.invalid/linux-b"
                }
            ]
        }"#,
    )
    .expect("fixture parses");

    let err = resolve_artifact(&metadata, &platform).expect_err("duplicate target must fail");
    assert!(matches!(err, UpdateError::DuplicateReleaseAsset));
}

#[test]
fn malformed_release_json_returns_typed_error() {
    let err = parse_release_metadata("{not valid json").expect_err("must fail");
    assert!(matches!(err, UpdateError::ReleaseMetadataInvalid));
}

#[test]
fn validates_tar_gz_archive_successfully() {
    let bytes = tar_gz_fixture(
        "codex-image-v1.2.3-x86_64-unknown-linux-gnu",
        "codex-image",
        false,
        true,
        false,
    );

    let validated = validate_archive_bytes(
        ArchiveKind::TarGz,
        &bytes,
        "codex-image-v1.2.3-x86_64-unknown-linux-gnu",
        "codex-image",
    )
    .expect("archive should validate");

    assert_eq!(
        validated.binary_path,
        Path::new("codex-image-v1.2.3-x86_64-unknown-linux-gnu/codex-image")
    );
}

#[test]
fn rejects_tar_gz_path_traversal() {
    let bytes = tar_gz_fixture_with_traversal();

    let err = validate_archive_bytes(
        ArchiveKind::TarGz,
        &bytes,
        "codex-image-v1.2.3-x86_64-unknown-linux-gnu",
        "codex-image",
    )
    .expect_err("path traversal must fail");

    assert!(matches!(err, UpdateError::ArchivePathTraversal));
}

#[test]
fn rejects_wrong_top_level_directory() {
    let bytes = tar_gz_fixture("wrong-root", "codex-image", false, true, false);

    let err = validate_archive_bytes(
        ArchiveKind::TarGz,
        &bytes,
        "codex-image-v1.2.3-x86_64-unknown-linux-gnu",
        "codex-image",
    )
    .expect_err("wrong root must fail");

    assert!(matches!(err, UpdateError::ArchiveTopLevelDirectoryMismatch));
}

#[test]
fn rejects_archive_missing_readme_files() {
    let bytes = tar_gz_fixture(
        "codex-image-v1.2.3-x86_64-unknown-linux-gnu",
        "codex-image",
        true,
        false,
        false,
    );

    let err = validate_archive_bytes(
        ArchiveKind::TarGz,
        &bytes,
        "codex-image-v1.2.3-x86_64-unknown-linux-gnu",
        "codex-image",
    )
    .expect_err("missing readme must fail");

    assert!(matches!(err, UpdateError::ArchiveMissingRequiredFile));
}

#[test]
fn rejects_archive_with_duplicate_binary() {
    let bytes = tar_gz_fixture(
        "codex-image-v1.2.3-x86_64-unknown-linux-gnu",
        "codex-image",
        false,
        true,
        true,
    );

    let err = validate_archive_bytes(
        ArchiveKind::TarGz,
        &bytes,
        "codex-image-v1.2.3-x86_64-unknown-linux-gnu",
        "codex-image",
    )
    .expect_err("duplicate binary must fail");

    assert!(matches!(err, UpdateError::ArchiveDuplicateBinary));
}

#[test]
fn validates_zip_archive_successfully() {
    let bytes = zip_fixture(
        "codex-image-v1.2.3-x86_64-pc-windows-msvc",
        "codex-image.exe",
        false,
        true,
        false,
    );

    let validated = validate_archive_bytes(
        ArchiveKind::Zip,
        &bytes,
        "codex-image-v1.2.3-x86_64-pc-windows-msvc",
        "codex-image.exe",
    )
    .expect("zip should validate");

    assert_eq!(
        validated.binary_path,
        Path::new("codex-image-v1.2.3-x86_64-pc-windows-msvc/codex-image.exe")
    );
}

#[test]
fn rejects_zip_path_traversal() {
    let bytes = zip_fixture_with_traversal();

    let err = validate_archive_bytes(
        ArchiveKind::Zip,
        &bytes,
        "codex-image-v1.2.3-x86_64-pc-windows-msvc",
        "codex-image.exe",
    )
    .expect_err("zip traversal must fail");

    assert!(matches!(err, UpdateError::ArchivePathTraversal));
}

#[test]
fn rejects_archive_missing_binary() {
    let bytes = zip_fixture(
        "codex-image-v1.2.3-x86_64-pc-windows-msvc",
        "other.exe",
        false,
        true,
        false,
    );

    let err = validate_archive_bytes(
        ArchiveKind::Zip,
        &bytes,
        "codex-image-v1.2.3-x86_64-pc-windows-msvc",
        "codex-image.exe",
    )
    .expect_err("missing binary must fail");

    assert!(matches!(err, UpdateError::ArchiveMissingRequiredFile));
}

fn release_fixture() -> &'static str {
    r#"{
        "tag_name": "v1.2.3",
        "assets": [
            {
                "name": "codex-image-v1.2.3-x86_64-unknown-linux-gnu.tar.gz",
                "browser_download_url": "https://example.invalid/linux"
            },
            {
                "name": "codex-image-v1.2.3-x86_64-apple-darwin.tar.gz",
                "browser_download_url": "https://example.invalid/macos-intel"
            },
            {
                "name": "codex-image-v1.2.3-aarch64-apple-darwin.tar.gz",
                "browser_download_url": "https://example.invalid/macos-arm"
            },
            {
                "name": "codex-image-v1.2.3-x86_64-pc-windows-msvc.zip",
                "browser_download_url": "https://example.invalid/windows"
            }
        ]
    }"#
}

fn tar_gz_fixture(
    top_level_dir: &str,
    binary_name: &str,
    omit_readme: bool,
    include_korean_readme: bool,
    duplicate_binary: bool,
) -> Vec<u8> {
    let mut compressed = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut tar = Builder::new(&mut compressed);

        append_tar_file(
            &mut tar,
            &format!("{top_level_dir}/{binary_name}"),
            b"#!/bin/sh\necho codex-image\n",
            0o755,
        );

        if duplicate_binary {
            append_tar_file(
                &mut tar,
                &format!("{top_level_dir}/nested/{binary_name}"),
                b"#!/bin/sh\n",
                0o755,
            );
        }

        if !omit_readme {
            append_tar_file(
                &mut tar,
                &format!("{top_level_dir}/README.md"),
                b"README",
                0o644,
            );
        }

        if include_korean_readme {
            append_tar_file(
                &mut tar,
                &format!("{top_level_dir}/README.ko.md"),
                b"README KO",
                0o644,
            );
        }

        tar.finish().expect("finish tar");
    }

    compressed.finish().expect("finish gzip")
}

fn tar_gz_fixture_with_traversal() -> Vec<u8> {
    let mut compressed = GzEncoder::new(Vec::new(), Compression::default());
    compressed
        .write_all(&raw_tar_with_single_file("../escape", b"nope", 0o644))
        .expect("write raw tar to gzip stream");
    compressed.finish().expect("finish gzip")
}

fn raw_tar_with_single_file(path: &str, content: &[u8], mode: u32) -> Vec<u8> {
    let mut tar_bytes = Vec::new();
    let header = build_tar_header(path, content.len() as u64, mode);
    tar_bytes.extend_from_slice(&header);
    tar_bytes.extend_from_slice(content);

    let file_padding = (512 - (content.len() % 512)) % 512;
    tar_bytes.extend(std::iter::repeat(0).take(file_padding));

    // End-of-archive marker: two all-zero blocks.
    tar_bytes.extend(std::iter::repeat(0).take(1024));
    tar_bytes
}

fn build_tar_header(path: &str, size: u64, mode: u32) -> [u8; 512] {
    let mut header = [0_u8; 512];

    write_tar_bytes(&mut header, 0, 100, path.as_bytes());
    write_tar_octal(&mut header, 100, 8, mode as u64);
    write_tar_octal(&mut header, 108, 8, 0);
    write_tar_octal(&mut header, 116, 8, 0);
    write_tar_octal(&mut header, 124, 12, size);
    write_tar_octal(&mut header, 136, 12, 0);
    for byte in &mut header[148..156] {
        *byte = b' ';
    }
    header[156] = b'0';
    write_tar_bytes(&mut header, 257, 6, b"ustar");
    header[262] = 0;
    write_tar_bytes(&mut header, 263, 2, b"00");

    let checksum: u32 = header.iter().map(|byte| u32::from(*byte)).sum();
    let checksum_text = format!("{:06o}\0 ", checksum);
    write_tar_bytes(&mut header, 148, 8, checksum_text.as_bytes());

    header
}

fn write_tar_bytes(header: &mut [u8; 512], start: usize, width: usize, value: &[u8]) {
    let count = value.len().min(width);
    header[start..start + count].copy_from_slice(&value[..count]);
}

fn write_tar_octal(header: &mut [u8; 512], start: usize, width: usize, value: u64) {
    let mut text = format!("{:o}", value);
    if text.len() + 1 > width {
        text = "0".repeat(width.saturating_sub(1));
    }

    let padding = width - text.len() - 1;
    for idx in 0..padding {
        header[start + idx] = b'0';
    }
    let text_start = start + padding;
    header[text_start..text_start + text.len()].copy_from_slice(text.as_bytes());
    header[start + width - 1] = 0;
}

fn append_tar_file(
    tar: &mut Builder<&mut GzEncoder<Vec<u8>>>,
    path: &str,
    content: &[u8],
    mode: u32,
) {
    let mut header = tar::Header::new_gnu();
    header.set_mode(mode);
    header.set_size(content.len() as u64);
    header.set_cksum();
    tar.append_data(&mut header, path, Cursor::new(content))
        .expect("append tar entry");
}

fn zip_fixture(
    top_level_dir: &str,
    binary_name: &str,
    omit_readme: bool,
    include_korean_readme: bool,
    duplicate_binary: bool,
) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = FileOptions::default();

        zip.start_file(format!("{top_level_dir}/{binary_name}"), options)
            .expect("start binary");
        zip.write_all(b"MZ").expect("write binary");

        if duplicate_binary {
            zip.start_file(format!("{top_level_dir}/nested/{binary_name}"), options)
                .expect("start duplicate binary");
            zip.write_all(b"MZ").expect("write duplicate binary");
        }

        if !omit_readme {
            zip.start_file(format!("{top_level_dir}/README.md"), options)
                .expect("start readme");
            zip.write_all(b"README").expect("write readme");
        }

        if include_korean_readme {
            zip.start_file(format!("{top_level_dir}/README.ko.md"), options)
                .expect("start ko readme");
            zip.write_all(b"README KO").expect("write ko readme");
        }

        zip.finish().expect("finish zip");
    }

    cursor.into_inner()
}

fn zip_fixture_with_traversal() -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = FileOptions::default();
        zip.start_file("../escape", options)
            .expect("start traversal entry");
        zip.write_all(b"nope").expect("write traversal entry");
        zip.finish().expect("finish zip");
    }
    cursor.into_inner()
}
