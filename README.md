# codex-image

`codex-image` is a CLI for logging in with OpenAI OAuth, checking machine-readable auth status, generating images, and clearing local auth state.

After reading this README, you should be able to install the binary locally, verify auth status, run a generation, and find the guarded live UAT smoke runbook.

## Install

From repository root:

```bash
cargo install --path . --force
```

Verify the installed binary is available:

```bash
codex-image --help
```

## Core commands

### 1) Login

Start OAuth login:

```bash
codex-image login
```

Complete the browser login shown by the CLI. The browser redirects to the local callback path printed by the command.

### 2) Check auth status (machine-readable contract)

`status` requires the JSON flag:

```bash
codex-image status --json
```

Expected unauthenticated shape:

```json
{"status":"not_logged_in"}
```

### 3) Generate images + manifest

Run a generation with an output directory:

```bash
codex-image generate "A watercolor fox reading in a library" --out ./out
```

By default, `generate` delegates image creation to the installed Codex CLI's built-in image generation tool, then copies the generated image into `--out` and writes `manifest.json`. This uses the existing Codex login surface; `codex-image` does not read, print, or mutate Codex auth files.

If `CODEX_IMAGE_API_BASE_URL` is explicitly set, `generate` uses the direct Image API compatibility path for trusted/test endpoints and requires this tool's owned auth state.

### 4) Logout

Clear local `codex-image` auth state:

```bash
codex-image logout
```

## Environment and trust boundaries

- `CODEX_IMAGE_HOME` controls where `codex-image` stores its auth state.
- `CODEX_IMAGE_CODEX_BIN` optionally points `generate` at a specific Codex executable. If unset, `codex-image` checks `codex` on `PATH` and common VS Code/Cursor extension install locations.
- `CODEX_IMAGE_AUTH_BASE_URL` and `CODEX_IMAGE_API_BASE_URL` should only point to trusted endpoints you control. Setting `CODEX_IMAGE_API_BASE_URL` opts `generate` into the direct Image API compatibility path instead of the default Codex backend.
- Keep token material private: do not print auth files, bearer headers, raw upstream response bodies, or raw base64 image payloads.

## Verification scripts

### Local install verification (no live credentials required)

```bash
bash scripts/verify-local-install.sh
```

This validates `cargo install --path .` output, installed-binary execution, isolated `CODEX_IMAGE_HOME`, and `status --json` contract behavior.

### Live UAT smoke (opt-in, real credentials)

Use only when you intentionally want a real login + generate smoke run:

```bash
CODEX_IMAGE_RUN_LIVE=1 bash scripts/uat-live-smoke.sh
```

The live script is guarded and exits early unless `CODEX_IMAGE_RUN_LIVE=1` is set.

Read the full runbook before use: [docs/uat-live-smoke.md](docs/uat-live-smoke.md)

The runbook documents safety/billing cautions, custom endpoint trust boundaries, and checks that `$HOME/.codex/auth.json` is preserved.
