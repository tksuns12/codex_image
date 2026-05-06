# codex-image

[![Release](https://github.com/tksuns12/codex_image/actions/workflows/release.yml/badge.svg?branch=release)](https://github.com/tksuns12/codex_image/actions/workflows/release.yml)

[한국어](README.ko.md)

`codex-image` is a small CLI that asks an installed Codex CLI to generate an image with Codex's built-in image tool, then copies the result into a requested output directory and writes a manifest.

It does **not** implement its own OpenAI OAuth flow, does **not** call URL-configured image API endpoints, and does **not** read or mutate Codex auth files. Codex itself owns login and image generation access.

## Install

### From a release artifact

Download the archive for your platform from the latest GitHub Release, or use one of the snippets below. Replace `v0.1.0` with the release tag you want to install.

#### Linux x86_64 / macOS x86_64 / macOS arm64

```bash
REPO="tksuns12/codex_image"
VERSION="v0.1.0"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
  Darwin-x86_64) TARGET="x86_64-apple-darwin" ;;
  Darwin-arm64|Darwin-aarch64) TARGET="aarch64-apple-darwin" ;;
  *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

ASSET="codex-image-${VERSION}-${TARGET}.tar.gz"
TMPDIR="$(mktemp -d)"
curl -L "https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}" -o "${TMPDIR}/${ASSET}"
tar -xzf "${TMPDIR}/${ASSET}" -C "${TMPDIR}"
mkdir -p "${HOME}/.local/bin"
install -m 0755 "${TMPDIR}/codex-image-${VERSION}-${TARGET}/codex-image" "${HOME}/.local/bin/codex-image"

codex-image --help
```

Make sure `${HOME}/.local/bin` is on your `PATH`.

#### Windows x86_64 PowerShell

```powershell
$Repo = "tksuns12/codex_image"
$Version = "v0.1.0"
$Target = "x86_64-pc-windows-msvc"
$Asset = "codex-image-$Version-$Target.zip"
$TempDir = New-Item -ItemType Directory -Force -Path (Join-Path $env:TEMP "codex-image-install")
$ZipPath = Join-Path $TempDir $Asset

Invoke-WebRequest "https://github.com/$Repo/releases/download/$Version/$Asset" -OutFile $ZipPath
Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force
New-Item -ItemType Directory -Force -Path "$HOME\bin" | Out-Null
Copy-Item "$TempDir\codex-image-$Version-$Target\codex-image.exe" "$HOME\bin\codex-image.exe" -Force

codex-image --help
```

Make sure `$HOME\bin` is on your `PATH`.

### From source

Use this for local development or when you intentionally want to install the current checkout instead of a published release.

```bash
cargo install --path . --force
codex-image --help
```

## Prerequisite: Codex CLI

`codex-image generate` depends on a working Codex installation. Resolution order:

1. `CODEX_IMAGE_CODEX_BIN` when set.
2. `codex` on `PATH`.
3. Common VS Code/Cursor Codex extension install locations.

Codex must already be logged in and able to use its built-in image generation tool.

## Generate images + manifest

Run a generation with an output directory:

```bash
codex-image generate "A watercolor fox reading in a library" --out ./out
```

The command:

1. Spawns `codex exec`.
2. Instructs Codex to use its built-in image generation tool.
3. Reads Codex's final JSON response containing the generated image path.
4. Copies the generated file into `--out` as `image-0001.<format>`.
5. Writes `manifest.json` under `--out`.
6. Prints the manifest JSON to stdout.

Example stdout shape:

```json
{
  "prompt": "A watercolor fox reading in a library",
  "model": "gpt-image-2",
  "manifest_path": "./out/manifest.json",
  "images": [
    {
      "index": 1,
      "path": "./out/image-0001.png",
      "format": "png",
      "byte_count": 12345
    }
  ],
  "response": {
    "created": 1777523488,
    "usage": {}
  }
}
```

## Agent skill install/update guide

Reader: a fresh human or AI agent landing in this repository for the first time.
Post-read action: pick the right tool + scope, run `codex-image skill install` / `codex-image skill update` non-interactively when needed, and validate binary update behavior safely.

Section order:
1. Supported tools and canonical path matrix link.
2. Interactive and deterministic install commands.
3. Skill update behavior and tamper protection.
4. Agent auto-install prompt.
5. Binary update behavior and no-live verification posture.

### Supported tools (install targets)

Canonical source-evidence matrix: [docs/skill-paths.md](docs/skill-paths.md)

| Tool | CLI `--tool` slug | Global scope path | Project scope path |
| --- | --- | --- | --- |
| Claude | `claude` | `~/.claude/skills/codex-image/SKILL.md` | `.claude/skills/codex-image/SKILL.md` |
| Claude Code | `claude-code` | `~/.claude/skills/codex-image/SKILL.md` | `.claude/skills/codex-image/SKILL.md` |
| Codex | `codex` | `~/.agents/skills/codex-image/SKILL.md` | `.agents/skills/codex-image/SKILL.md` |
| pi | `pi` | `~/.agents/skills/codex-image/SKILL.md` | `.agents/skills/codex-image/SKILL.md` |
| OpenCode | `opencode` | `~/.config/opencode/skills/codex-image/SKILL.md` | `.opencode/skills/codex-image/SKILL.md` |

When authoring `SKILL.md` content for these tools, follow:
- https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide

### Install `codex-image` skills

Interactive (TTY) flow:

```bash
codex-image skill install
```

Use `Space` to toggle tool/scope selections and `Enter` to confirm.

Interactive selection behavior:
- Already-installed targets are preselected.
- Outdated managed installs are preselected and labeled `installed:outdated`.
- Manual/tampered installs are preselected and labeled `installed:protected`.
- If you uncheck an installed managed target, `codex-image skill install` removes that `SKILL.md`.
- Unchecking a manual/tampered target is blocked by default; use `--force` to allow removal.

Deterministic agent/CI commands:

```bash
codex-image skill install --tool codex --tool pi --scope project --yes
codex-image skill install --tool claude-code --scope global --yes
```

You can also pin explicit slugs in scripts, for example `codex-image skill install --tool opencode --scope project --yes`.

### Update managed skills

Run:

```bash
codex-image skill update
```

Or scope updates explicitly in automation:

```bash
codex-image skill update --tool codex --scope project --yes
```

Managed update behavior:
- Creates missing managed files.
- Leaves already up-to-date managed files as no-op.
- Refreshes outdated managed files to current bundled content.
- Emits line-delimited JSON rows with stable high-level fields: `tool`, `scope`, `status`, `target_path`.
- Blocks manual/tampered files by default.
- Requires `--force` as the explicit overwrite escape hatch for blocked/tampered targets.

### Agent auto-install prompt

Copy/paste this prompt for autonomous setup:

```text
Inspect the current project and choose supported tools/scopes for codex-image skills.
Run only non-interactive commands with explicit confirmation:
- codex-image skill install --tool <slug> --scope <project|global> --yes
- codex-image skill update --tool <slug> --scope <project|global> --yes
Do not mutate authentication state, do not run login flows, and do not change credentials.
Optionally run codex-image update --dry-run before any binary replacement.
```

### Binary update behavior

`codex-image update` pulls from GitHub Release artifacts and supports dry-run, non-interactive apply, and optional version pinning:

```bash
codex-image update --dry-run
codex-image update --yes
codex-image update --version v1.2.3 --yes
```

High-level JSON status output is stable (release selection/result metadata plus update status fields).

Windows caveat: Windows same-process replacement limitation applies; use `codex-image update --dry-run` plus manual replacement guidance instead of assuming in-process overwrite.

No-live verification posture for this repository:
- no live GitHub downloads
- no live Codex generation
- no credentials
- no auth mutation

## Environment

- `CODEX_IMAGE_CODEX_BIN` optionally points to the Codex executable.

No URL base environment variables are supported. No separate auth/API behavior exists.

## Release workflow

Releases are cut from the `release` branch only.

The release workflow uses release-please to decide SemVer from Conventional Commit messages:

- `fix:` creates a patch release.
- `feat:` creates a minor release.
- `feat!:`, `fix!:`, or another `!` breaking-change commit creates a major release.

Expected branch protection for `release`:

- Require pull requests before merging.
- Require the `Release / Preflight` status check.
- Require branches to be up to date before merging.
- Restrict direct pushes if the repository policy allows it.

On pull requests into `release`, the workflow runs tests and clippy. On push to `release`, release-please either opens/updates a release PR or creates the GitHub Release when that release PR is merged. When a release is created, the workflow builds and uploads archives for Linux, macOS, and Windows.

## Verification scripts

### Local install verification

```bash
bash scripts/verify-local-install.sh
```

This validates `cargo install --path .`, installed-binary execution, and help/usage behavior without requiring live image generation.

### Live UAT smoke

Use only when you intentionally want a real Codex-backed image generation smoke run:

```bash
CODEX_IMAGE_RUN_LIVE=1 bash scripts/uat-live-smoke.sh
```

The live script is guarded and exits early unless `CODEX_IMAGE_RUN_LIVE=1` is set.

Read the full runbook before use: [docs/uat-live-smoke.md](docs/uat-live-smoke.md)
