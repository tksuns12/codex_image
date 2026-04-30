# Live UAT smoke runbook (`login` + `generate`)

This runbook is for maintainers who need a **real credentials** smoke test of `codex-image`.

Post-read action: you can run one guarded command that verifies interactive `login`, `status --json` validity, image generation artifacts, and Codex CLI auth preservation.

## Safety and billing warnings

- This flow is **opt-in** and touches live auth + image generation endpoints.
- It may consume paid API quota and should be run only when you need runtime confidence.
- The script is single-shot: no retry loops are added by the wrapper.
- The script must not be used against untrusted endpoint overrides.

If you set either of these variables, point them only to trusted hosts you control:

- `CODEX_IMAGE_AUTH_BASE_URL`
- `CODEX_IMAGE_API_BASE_URL`

## Prerequisites

- Rust/Cargo installed.
- You can complete OAuth login in a browser for the current account.
- Network access to the configured auth/API hosts.
- Run from repository root.

## Run the guarded smoke check

```bash
CODEX_IMAGE_RUN_LIVE=1 bash scripts/uat-live-smoke.sh
```

Guard behavior:

- If `CODEX_IMAGE_RUN_LIVE=1` is not set, the script fails immediately before login/generate.
- This lets CI and default local runs avoid accidental live auth or billed image calls.

## What the script verifies

The script prints phase logs and fails fast on any contract break:

1. Creates temporary install/auth/output roots.
2. Snapshots the checksum state of `$HOME/.codex/auth.json` (or absence).
3. Installs local binary with `cargo install --path . --root <temp> --force`.
4. Runs interactive `login` using isolated `CODEX_IMAGE_HOME`.
5. Requires `status --json` to report `status = "valid"`.
6. Runs:
   - `generate "UAT smoke image from codex-image" --out <temp-out>`
7. Parses stdout JSON and validates:
   - `manifest_path` exists and matches `<temp-out>/manifest.json`
   - `manifest.json` is valid JSON
   - every declared image path exists
8. Verifies `$HOME/.codex/auth.json` is unchanged (or still absent).

## Expected successful observations

- Phase logs progress through guard, install, login, status, generate, manifest validation, and auth preservation.
- `status --json` validation succeeds with `valid`.
- Manifest validation reports a non-zero image count.
- Final message indicates successful completion and cleanup.

## Failure interpretation

- `phase=guard`: opt-in variable missing; rerun with `CODEX_IMAGE_RUN_LIVE=1` only if live test is intended.
- `phase=login`: OAuth callback flow failed/timed out/rejected.
- `phase=status-validate`: login did not produce a valid auth state.
- `phase=generate`: live image API call failed; script prints redacted CLI stderr envelope and exits non-zero.
- `phase=manifest-validate`: stdout or manifest contract malformed/missing.
- `phase=codex-auth-preservation`: `$HOME/.codex/auth.json` changed unexpectedly and must be investigated.

## Cleanup

- Temporary install/auth/output directories are removed automatically on success or failure.
- The script does not persist generated images after completion because output uses a temp directory.

## Manual fallback (if you need step-by-step debugging)

If the wrapper fails and you need deeper diagnosis, repeat the same sequence manually with an isolated `CODEX_IMAGE_HOME`:

1. Install local binary to a temp root.
2. Run `login` and complete OAuth callback flow.
3. Run `status --json` and inspect `status`.
4. Run `generate ... --out <dir>` and inspect `manifest.json` plus image files.
5. Re-check that `$HOME/.codex/auth.json` checksum did not change.

Use the same trust and redaction discipline: do not print tokens, raw auth files, bearer headers, raw upstream bodies, or raw `b64_json` payloads.
