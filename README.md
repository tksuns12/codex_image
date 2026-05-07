# codex-image

[한국어](README.ko.md)

`codex-image` is a small CLI that asks an installed Codex CLI to generate an image with Codex's built-in image tool, then copies the result into a requested output directory and writes a manifest.

If you are new, read this page in order: verify Codex prerequisites, install `codex-image`, run one generation command, then confirm the output files/stdout.

It does **not** implement its own OpenAI OAuth flow, does **not** call URL-configured image API endpoints, and does **not** read or mutate Codex auth files. Codex itself owns login and image generation access.

## Prerequisite: Codex CLI / Codex extensions

`codex-image generate` depends on a working Codex installation that can already generate images.

- The standalone Codex CLI is currently **macOS-only**.
- Codex installs provided by **VS Code**/**Cursor** extensions are also supported for `codex-image generate`.

Executable resolution order:

1. `CODEX_IMAGE_CODEX_BIN` when set.
2. `codex` on `PATH`.
3. Common VS Code/Cursor Codex extension install locations.

Codex must already be logged in and able to use its built-in image generation tool.

## Install

Recommended path: install from a release artifact for your platform.

### From a release artifact

Download the archive from the latest GitHub Release (or use snippets below). Replace `v0.1.0` with the release tag you want.

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

### From source (secondary path)

Use this only when you intentionally want to install from the current checkout (for local development/testing).

```bash
cargo install --path . --force
codex-image --help
```

## Generate images + manifest

Run one generation with an output directory:

```bash
codex-image generate "A watercolor fox reading in a library" --out ./out
```

Expected output from that single command:
- an image file named `image-0001.<format>` in `./out`
- `manifest.json` in `./out`
- the same manifest JSON printed to stdout

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

## After your first run

If your first command produced `image-0001.<format>` and `manifest.json`, you're done with the quickstart.
The sections below are optional follow-up material for agent automation, skill maintenance, and binary updates.

If you're curious about execution details: `codex-image` runs `codex exec`, asks Codex to use its built-in image tool, reads Codex's final JSON response, and copies the generated image into your output directory.

## Post-first-run references (optional)

If your first run succeeded, use these references for operations beyond quickstart:

- Advanced operations guide (skill lifecycle, automation prompt, update behavior, verification posture): [docs/advanced-reference.md](docs/advanced-reference.md)
- Canonical supported tool/path/source matrix: [docs/skill-paths.md](docs/skill-paths.md)
- Intentional live Codex-backed smoke runbook: [docs/uat-live-smoke.md](docs/uat-live-smoke.md)

Fast command reference:

```bash
codex-image skill install --tool codex --scope project --yes
codex-image skill update --tool codex --scope project --yes
codex-image update --dry-run
codex-image update --yes
codex-image update --version v1.2.3 --yes
```

Keep using explicit `--tool` and `--scope` values for automation. Use `codex-image update --dry-run` before replacement when you want a non-mutating preview.
