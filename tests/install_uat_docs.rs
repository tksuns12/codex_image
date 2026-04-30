#[test]
fn readme_covers_install_usage_and_uat_discoverability() {
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
        readme.contains("status --json"),
        "README must document machine-readable status contract"
    );
    assert!(
        readme.contains("generate"),
        "README must describe generate usage"
    );
    assert!(
        readme.contains("docs/uat-live-smoke.md"),
        "README must link to the live UAT runbook"
    );
}

#[test]
fn scripts_document_live_guard_and_isolated_auth_home_contract() {
    let verify_local_install = include_str!("../scripts/verify-local-install.sh");
    let uat_live_smoke = include_str!("../scripts/uat-live-smoke.sh");

    assert!(
        verify_local_install.contains("CODEX_IMAGE_HOME"),
        "verify-local-install script must use isolated CODEX_IMAGE_HOME"
    );
    assert!(
        uat_live_smoke.contains("CODEX_IMAGE_HOME"),
        "live UAT script must use isolated CODEX_IMAGE_HOME"
    );
    assert!(
        uat_live_smoke.contains("CODEX_IMAGE_RUN_LIVE=1"),
        "live UAT script must require explicit CODEX_IMAGE_RUN_LIVE=1 opt-in"
    );
}

#[test]
fn uat_doc_warns_about_codex_auth_preservation_and_trusted_bases() {
    let runbook = include_str!("../docs/uat-live-smoke.md");

    assert!(
        runbook.contains("$HOME/.codex/auth.json"),
        "runbook must mention Codex CLI auth file preservation"
    );
    assert!(
        runbook.contains("CODEX_IMAGE_AUTH_BASE_URL"),
        "runbook must document custom auth base trust boundary"
    );
    assert!(
        runbook.contains("CODEX_IMAGE_API_BASE_URL"),
        "runbook must document custom API base trust boundary"
    );
    assert!(
        runbook.contains("trusted hosts") || runbook.contains("trusted endpoints"),
        "runbook must warn that custom bases are trusted-only"
    );
}
