#[test]
fn install_uat_docs_readme_covers_install_usage_and_codex_backend() {
    let readme = include_str!("../README.md");

    assert!(
        readme.contains("cargo install --path ."),
        "README must document local source install fallback"
    );
    assert!(
        readme.contains("actions/workflows/release.yml/badge.svg?branch=release"),
        "README must include release workflow badge scoped to release branch"
    );
    assert!(
        readme.contains("releases/download/${VERSION}/${ASSET}"),
        "README must document release artifact downloads as the primary install path"
    );
    assert!(
        readme.contains("x86_64-unknown-linux-gnu")
            && readme.contains("x86_64-apple-darwin")
            && readme.contains("aarch64-apple-darwin")
            && readme.contains("x86_64-pc-windows-msvc"),
        "README must document platform release artifact targets"
    );
    assert!(
        readme.contains("codex-image"),
        "README must name the binary"
    );
    assert!(
        readme.contains("Codex CLI") || readme.contains("Codex installation"),
        "README must document the Codex dependency"
    );
    assert!(
        readme.contains("generate"),
        "README must describe generate usage"
    );
    assert!(
        readme.contains("CODEX_IMAGE_CODEX_BIN"),
        "README must document Codex binary override"
    );
    assert!(
        readme.contains("README.ko.md"),
        "README must link to the Korean translation"
    );
    assert!(
        readme.contains("release-please") && readme.contains("release` branch"),
        "README must document release branch and semver automation"
    );
    assert!(
        readme.contains("Release / Preflight") && readme.contains("Require pull requests"),
        "README must document release branch protection expectations"
    );

    for required in [
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
    ] {
        assert!(
            readme.contains(required),
            "README must include final S06 marker: {required}"
        );
    }

    assert!(
        !readme.contains("CODEX_IMAGE_API_BASE_URL")
            && !readme.contains("CODEX_IMAGE_AUTH_BASE_URL")
            && !readme.contains("status --json")
            && !readme.contains("codex-image login"),
        "README must not document removed URL/auth surfaces"
    );
}

#[test]
fn install_uat_docs_korean_readme_covers_install_usage_and_codex_backend() {
    let readme = include_str!("../README.ko.md");

    assert!(
        readme.contains("cargo install --path ."),
        "Korean README must document local source install fallback"
    );
    assert!(
        readme.contains("actions/workflows/release.yml/badge.svg?branch=release"),
        "Korean README must include release workflow badge scoped to release branch"
    );
    assert!(
        readme.contains("releases/download/${VERSION}/${ASSET}"),
        "Korean README must document release artifact downloads as the primary install path"
    );
    assert!(
        readme.contains("x86_64-unknown-linux-gnu")
            && readme.contains("x86_64-apple-darwin")
            && readme.contains("aarch64-apple-darwin")
            && readme.contains("x86_64-pc-windows-msvc"),
        "Korean README must document platform release artifact targets"
    );
    assert!(
        readme.contains("codex-image"),
        "Korean README must name the binary"
    );
    assert!(
        readme.contains("Codex CLI") || readme.contains("Codex 설치"),
        "Korean README must document the Codex dependency"
    );
    assert!(
        readme.contains("generate"),
        "Korean README must describe generate usage"
    );
    assert!(
        readme.contains("CODEX_IMAGE_CODEX_BIN"),
        "Korean README must document Codex binary override"
    );
    assert!(
        readme.contains("README.md"),
        "Korean README must link back to the English README"
    );
    assert!(
        readme.contains("release-please") && readme.contains("release` 브랜치"),
        "Korean README must document release branch and semver automation"
    );
    assert!(
        readme.contains("Release / Preflight") && readme.contains("pull request"),
        "Korean README must document release branch protection expectations"
    );

    for required in [
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
    ] {
        assert!(
            readme.contains(required),
            "Korean README must include stable command marker: {required}"
        );
    }

    assert!(
        readme.contains("no live GitHub downloads")
            || readme.contains("GitHub 다운로드를 라이브로 수행하지 않습니다"),
        "Korean README must state no-live GitHub download verification"
    );
    assert!(
        readme.contains("no live Codex generation")
            || readme.contains("Codex 생성은 라이브로 수행하지 않습니다"),
        "Korean README must state no-live Codex generation verification"
    );
    assert!(
        readme.contains("no credentials") || readme.contains("자격 증명을 요구하지 않습니다"),
        "Korean README must state no-credentials verification"
    );
    assert!(
        readme.contains("no auth mutation") || readme.contains("인증 상태를 변경하지 않습니다"),
        "Korean README must state no-auth-mutation verification"
    );

    assert!(
        !readme.contains("CODEX_IMAGE_API_BASE_URL")
            && !readme.contains("CODEX_IMAGE_AUTH_BASE_URL")
            && !readme.contains("status --json")
            && !readme.contains("codex-image login"),
        "Korean README must not document removed URL/auth surfaces"
    );
}

#[test]
fn install_uat_docs_scripts_document_codex_backend_and_live_guard() {
    let verify_local_install = include_str!("../scripts/verify-local-install.sh");
    let uat_live_smoke = include_str!("../scripts/uat-live-smoke.sh");

    assert!(
        verify_local_install.contains("generate --help"),
        "verify-local-install script must validate generate help"
    );
    assert!(
        uat_live_smoke.contains("CODEX_IMAGE_RUN_LIVE=1"),
        "live UAT script must require explicit CODEX_IMAGE_RUN_LIVE=1 opt-in"
    );
    assert!(
        uat_live_smoke.contains("CODEX_IMAGE_CODEX_BIN"),
        "live UAT script must support Codex binary override"
    );
    assert!(
        !uat_live_smoke.contains("CODEX_IMAGE_API_BASE_URL")
            && !uat_live_smoke.contains("CODEX_IMAGE_AUTH_BASE_URL")
            && !uat_live_smoke.contains("status --json")
            && !uat_live_smoke.contains(" login"),
        "live UAT script must not depend on removed URL/auth surfaces"
    );
}

#[test]
fn install_uat_docs_uat_doc_describes_codex_only_backend() {
    let runbook = include_str!("../docs/uat-live-smoke.md");

    assert!(
        runbook.contains("Codex") && runbook.contains("generate"),
        "runbook must document Codex-backed generation"
    );
    assert!(
        runbook.contains("CODEX_IMAGE_CODEX_BIN"),
        "runbook must document Codex binary override"
    );
    assert!(
        !runbook.contains("CODEX_IMAGE_AUTH_BASE_URL")
            && !runbook.contains("CODEX_IMAGE_API_BASE_URL")
            && !runbook.contains("status --json")
            && !runbook.contains("OAuth"),
        "runbook must not document removed URL/auth surfaces"
    );
}
