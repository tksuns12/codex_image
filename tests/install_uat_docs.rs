#[test]
fn readme_covers_install_usage_and_codex_backend() {
    let readme = include_str!("../README.md");

    assert!(
        readme.contains("cargo install --path ."),
        "README must document local install command"
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
        !readme.contains("CODEX_IMAGE_API_BASE_URL")
            && !readme.contains("CODEX_IMAGE_AUTH_BASE_URL")
            && !readme.contains("status --json")
            && !readme.contains("codex-image login"),
        "README must not document removed URL/auth surfaces"
    );
}

#[test]
fn scripts_document_codex_backend_and_live_guard() {
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
fn uat_doc_describes_codex_only_backend() {
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
