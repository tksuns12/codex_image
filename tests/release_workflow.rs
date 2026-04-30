#[test]
fn release_workflow_is_scoped_to_release_branch_and_release_please() {
    let workflow = include_str!("../.github/workflows/release.yml");

    assert!(
        workflow.contains("pull_request:\n    branches:\n      - release"),
        "release workflow must run preflight on pull requests into release"
    );
    assert!(
        workflow.contains("push:\n    branches:\n      - release"),
        "release workflow must publish only from pushes to release"
    );
    assert!(
        workflow.contains("googleapis/release-please-action@v4"),
        "release workflow must delegate semver/release PRs to release-please"
    );
    assert!(
        workflow.contains("if: ${{ github.event_name == 'push' }}"),
        "release-please must not run on pull request preflight checks"
    );
    assert!(
        workflow.contains("contents: read")
            && workflow.contains("contents: write")
            && workflow.contains("pull-requests: write"),
        "release workflow permissions must default to read and elevate only release jobs"
    );
    assert!(
        workflow.contains("release-type: rust"),
        "release-please must use Rust/Cargo versioning"
    );
    assert!(
        workflow.contains("target-branch: release"),
        "release-please must target the release branch"
    );
    assert!(
        include_str!("../CHANGELOG.md").contains("release-please"),
        "release-please Rust releases need a changelog seed file"
    );
    assert!(
        workflow.contains("dtolnay/rust-toolchain@stable"),
        "release workflow must install Rust explicitly for GitHub and act runners"
    );
    assert!(
        workflow.contains("cargo test --locked"),
        "release workflow must run tests before release-please"
    );
    assert!(
        workflow.contains("cargo clippy --locked --all-targets --all-features -- -D warnings"),
        "release workflow must run clippy before release-please"
    );
}

#[test]
fn release_workflow_builds_expected_platform_artifacts() {
    let workflow = include_str!("../.github/workflows/release.yml");

    for target in [
        "x86_64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ] {
        assert!(
            workflow.contains(target),
            "release workflow must build target {target}"
        );
    }

    assert!(
        workflow.contains("cargo build --locked --release --target"),
        "release workflow must build locked release binaries"
    );
    assert!(
        workflow.contains("gh release upload"),
        "release workflow must upload archives to the GitHub Release"
    );
    assert!(
        workflow.contains("--clobber"),
        "release workflow reruns should replace existing failed-run artifacts"
    );
}
