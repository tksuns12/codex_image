fn must_contain(label: &str, doc: &str, marker: &str) -> usize {
    doc.find(marker)
        .unwrap_or_else(|| panic!("{label} must include marker: {marker}"))
}

#[allow(dead_code)]
fn assert_before(label: &str, doc: &str, first: &str, second: &str) {
    let first_pos = must_contain(label, doc, first);
    let second_pos = must_contain(label, doc, second);
    assert!(
        first_pos < second_pos,
        "{label} must place '{first}' before '{second}' (positions: {first_pos} vs {second_pos})"
    );
}

fn assert_all_present(label: &str, doc: &str, markers: &[&str]) -> Vec<usize> {
    markers
        .iter()
        .map(|marker| must_contain(label, doc, marker))
        .collect()
}

fn assert_all_absent(label: &str, doc: &str, banned: &[&str]) {
    for marker in banned {
        assert!(
            !doc.contains(marker),
            "{label} must not include removed/internal marker: {marker}"
        );
    }
}

#[test]
fn install_uat_docs_readme_covers_install_usage_and_codex_backend() {
    let label = "README";
    let readme = include_str!("../README.md");

    assert_all_present(
        label,
        readme,
        &[
            "cargo install --path .",
            "releases/download/${VERSION}/${ASSET}",
            "x86_64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
            "codex-image",
            "generate",
            "CODEX_IMAGE_CODEX_BIN",
            "README.ko.md",
            "macOS-only",
            "Claude",
            "Claude Code",
            "Codex",
            "pi",
            "OpenCode",
            "claude-code",
            "opencode",
            "codex-image skill install",
            "codex-image skill update",
            "Space",
            "Enter",
            "--tool",
            "--scope project",
            "--scope global",
            "--yes",
            "--force",
            "Agent auto-install prompt",
            "codex-image update --dry-run",
            "codex-image update --yes",
            "--version v1.2.3",
            "GitHub Release artifacts",
            "Windows same-process replacement limitation",
            "no live GitHub downloads",
            "no live Codex generation",
            "no credentials",
            "no auth mutation",
        ],
    );

    assert!(
        readme.contains("Codex CLI") || readme.contains("Codex installation"),
        "{label} must document the Codex dependency"
    );
    assert!(
        readme.contains("Codex extensions")
            || (readme.contains("VS Code") && readme.contains("Cursor")),
        "{label} must mention Codex extension install locations as a prerequisite"
    );

    assert_all_present(
        label,
        readme,
        &[
            "`codex-image` is a small CLI",
            "## Prerequisite: Codex CLI / Codex extensions",
            "macOS-only",
            "VS Code/Cursor",
            "Codex must already be logged in and able to use its built-in image generation tool.",
            "## Install",
            "codex-image generate \"A watercolor fox reading in a library\" --out ./out",
            "image-0001.<format>",
            "manifest.json",
            "## Agent skill install/update guide",
            "codex-image skill install --tool",
            "codex-image update --dry-run",
            "docs/uat-live-smoke.md",
            "## Verification scripts",
        ],
    );

    assert_before(
        label,
        readme,
        "`codex-image` is a small CLI",
        "## Prerequisite: Codex CLI / Codex extensions",
    );
    assert_before(
        label,
        readme,
        "## Prerequisite: Codex CLI / Codex extensions",
        "## Install",
    );
    assert_before(
        label,
        readme,
        "macOS-only",
        "codex-image generate \"A watercolor fox reading in a library\" --out ./out",
    );
    assert_before(
        label,
        readme,
        "VS Code/Cursor",
        "codex-image generate \"A watercolor fox reading in a library\" --out ./out",
    );
    assert_before(
        label,
        readme,
        "Codex must already be logged in and able to use its built-in image generation tool.",
        "codex-image generate \"A watercolor fox reading in a library\" --out ./out",
    );
    assert_before(
        label,
        readme,
        "CODEX_IMAGE_CODEX_BIN",
        "codex-image generate \"A watercolor fox reading in a library\" --out ./out",
    );

    assert_before(
        label,
        readme,
        "## Install",
        "codex-image generate \"A watercolor fox reading in a library\" --out ./out",
    );
    assert_before(
        label,
        readme,
        "codex-image generate \"A watercolor fox reading in a library\" --out ./out",
        "image-0001.<format>",
    );
    assert_before(
        label,
        readme,
        "codex-image generate \"A watercolor fox reading in a library\" --out ./out",
        "manifest.json",
    );

    for advanced_marker in [
        "## Agent skill install/update guide",
        "codex-image skill install --tool",
        "codex-image update --dry-run",
        "docs/uat-live-smoke.md",
        "## Verification scripts",
    ] {
        assert_before(label, readme, "image-0001.<format>", advanced_marker);
        assert_before(label, readme, "manifest.json", advanced_marker);
    }

    assert_all_absent(
        label,
        readme,
        &[
            "actions/workflows/release.yml/badge.svg?branch=release",
            "release-please",
            "Release / Preflight",
            "CODEX_IMAGE_API_BASE_URL",
            "CODEX_IMAGE_AUTH_BASE_URL",
            "status --json",
            "codex-image login",
        ],
    );
}

#[test]
fn install_uat_docs_korean_readme_covers_install_usage_and_codex_backend() {
    let label = "Korean README";
    let readme = include_str!("../README.ko.md");

    assert_all_present(
        label,
        readme,
        &[
            "cargo install --path .",
            "releases/download/${VERSION}/${ASSET}",
            "x86_64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc",
            "codex-image",
            "generate",
            "CODEX_IMAGE_CODEX_BIN",
            "README.md",
            "codex-image skill install",
            "codex-image skill update",
            "--tool",
            "--scope project",
            "--scope global",
            "--yes",
            "--force",
            "codex-image update --dry-run",
            "codex-image update --yes",
            "--version v1.2.3",
        ],
    );

    assert!(
        readme.contains("Codex CLI") || readme.contains("Codex 설치"),
        "{label} must document the Codex dependency"
    );
    assert!(
        readme.contains("Codex 확장")
            || (readme.contains("VS Code") && readme.contains("Cursor")),
        "{label} must mention Codex extension install locations as a prerequisite"
    );
    assert!(
        readme.contains("macOS 전용") || readme.contains("macOS"),
        "{label} must state that standalone Codex CLI support is macOS-only"
    );

    assert!(
        (readme.contains("no live GitHub downloads")
            || readme.contains("GitHub 다운로드를 라이브로 수행하지 않습니다"))
            && (readme.contains("no live Codex generation")
                || readme.contains("Codex 생성은 라이브로 수행하지 않습니다"))
            && (readme.contains("no credentials") || readme.contains("자격 증명을 요구하지 않습니다"))
            && (readme.contains("no auth mutation") || readme.contains("인증 상태를 변경하지 않습니다")),
        "{label} must keep no-live verification statements"
    );

    assert_all_absent(
        label,
        readme,
        &[
            "actions/workflows/release.yml/badge.svg?branch=release",
            "release-please",
            "Release / Preflight",
            "CODEX_IMAGE_API_BASE_URL",
            "CODEX_IMAGE_AUTH_BASE_URL",
            "status --json",
            "codex-image login",
        ],
    );
}

#[test]
fn install_uat_docs_scripts_document_codex_backend_and_live_guard() {
    let verify_label = "verify-local-install script";
    let live_label = "live UAT script";

    let verify_local_install = include_str!("../scripts/verify-local-install.sh");
    let uat_live_smoke = include_str!("../scripts/uat-live-smoke.sh");

    must_contain(verify_label, verify_local_install, "generate --help");
    must_contain(live_label, uat_live_smoke, "CODEX_IMAGE_RUN_LIVE=1");
    must_contain(live_label, uat_live_smoke, "CODEX_IMAGE_CODEX_BIN");

    assert_all_absent(
        live_label,
        uat_live_smoke,
        &[
            "CODEX_IMAGE_API_BASE_URL",
            "CODEX_IMAGE_AUTH_BASE_URL",
            "status --json",
            " login",
        ],
    );
}

#[test]
fn install_uat_docs_uat_doc_describes_codex_only_backend() {
    let label = "UAT runbook";
    let runbook = include_str!("../docs/uat-live-smoke.md");

    assert_all_present(label, runbook, &["Codex", "generate", "CODEX_IMAGE_CODEX_BIN"]);

    assert_all_absent(
        label,
        runbook,
        &[
            "CODEX_IMAGE_AUTH_BASE_URL",
            "CODEX_IMAGE_API_BASE_URL",
            "status --json",
            "OAuth",
        ],
    );
}
