# codex-image

`codex-image` is a small CLI that asks an installed Codex CLI to generate an image with Codex's built-in image tool, then copies the result into a requested output directory and writes a manifest.

It does **not** implement its own OpenAI OAuth flow, does **not** call URL-configured image API endpoints, and does **not** read or mutate Codex auth files. Codex itself owns login and image generation access.

## Install

From repository root:

```bash
cargo install --path . --force
```

Verify the installed binary is available:

```bash
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

## Environment

- `CODEX_IMAGE_CODEX_BIN` optionally points to the Codex executable.

No URL base environment variables are supported. No separate auth/API behavior exists.

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
