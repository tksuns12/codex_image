#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PHASE="guard"
install_root=""
auth_home=""
out_dir=""
cli_home=""
status_stdout=""
status_stderr=""
login_stderr=""
generate_stdout=""
generate_stderr=""

log_phase() {
  printf '[uat-live-smoke] phase=%s %s\n' "$PHASE" "$1"
}

redact_cli_stderr() {
  local stderr_path="$1"
  if [[ -f "$stderr_path" ]]; then
    while IFS= read -r line; do
      printf '[uat-live-smoke] cli-stderr-redacted %s\n' "$line" >&2
    done < "$stderr_path"
  fi
}

cleanup() {
  local rc=$?

  for file_path in "$status_stdout" "$status_stderr" "$login_stderr" "$generate_stdout" "$generate_stderr"; do
    if [[ -n "$file_path" && -f "$file_path" ]]; then
      rm -f "$file_path"
    fi
  done

  for dir_path in "$install_root" "$auth_home" "$out_dir"; do
    if [[ -n "$dir_path" && -d "$dir_path" ]]; then
      rm -rf "$dir_path"
    fi
  done

  if [[ $rc -eq 0 ]]; then
    printf '[uat-live-smoke] phase=cleanup removed temp roots\n'
  else
    printf '[uat-live-smoke] ERROR phase=%s exit_code=%d\n' "$PHASE" "$rc" >&2
  fi

  exit "$rc"
}
trap cleanup EXIT

if [[ "${CODEX_IMAGE_RUN_LIVE:-}" != "1" ]]; then
  echo '[uat-live-smoke] ERROR phase=guard refusing live UAT; set CODEX_IMAGE_RUN_LIVE=1 to opt in' >&2
  exit 1
fi

PHASE="temp-filesystem"
install_root="$(mktemp -d -t codex-image-uat-install.XXXXXX)"
auth_home="$(mktemp -d -t codex-image-uat-auth.XXXXXX)"
out_dir="$(mktemp -d -t codex-image-uat-output.XXXXXX)"
log_phase "created isolated install/auth/output roots"

PHASE="codex-auth-snapshot"
cli_home="${HOME:-}"
if [[ -z "$cli_home" ]]; then
  echo '[uat-live-smoke] ERROR phase=codex-auth-snapshot HOME is not set' >&2
  exit 1
fi

codex_auth_path="$cli_home/.codex/auth.json"
if [[ -f "$codex_auth_path" ]]; then
  codex_auth_before="$(shasum -a 256 "$codex_auth_path" | awk '{print $1}')"
  codex_auth_mode="present"
  log_phase "captured checksum for existing ~/.codex/auth.json"
else
  codex_auth_before=""
  codex_auth_mode="absent"
  log_phase "~/.codex/auth.json not present before run"
fi

PHASE="cargo-install"
log_phase "running cargo install --path . --root <temp> --force"
cargo install --path "$REPO_ROOT" --root "$install_root" --force >/dev/null

binary="$install_root/bin/codex-image"
if [[ ! -x "$binary" ]]; then
  echo "[uat-live-smoke] ERROR phase=cargo-install installed binary missing at $binary" >&2
  exit 1
fi

PHASE="login"
log_phase "running interactive login (device-code flow)"
login_stderr="$(mktemp -t codex-image-uat-login-err.XXXXXX)"
if ! HOME="$cli_home" CODEX_IMAGE_HOME="$auth_home" "$binary" login 2>"$login_stderr"; then
  redact_cli_stderr "$login_stderr"
  echo '[uat-live-smoke] ERROR phase=login login command failed' >&2
  exit 1
fi

PHASE="status-json"
log_phase "running isolated status --json"
status_stdout="$(mktemp -t codex-image-uat-status-out.XXXXXX)"
status_stderr="$(mktemp -t codex-image-uat-status-err.XXXXXX)"
if ! HOME="$cli_home" CODEX_IMAGE_HOME="$auth_home" "$binary" status --json >"$status_stdout" 2>"$status_stderr"; then
  redact_cli_stderr "$status_stderr"
  echo '[uat-live-smoke] ERROR phase=status-json status --json failed' >&2
  exit 1
fi

PHASE="status-validate"
status_value="$(python3 - "$status_stdout" <<'PY'
import json
import sys

status_path = sys.argv[1]
with open(status_path, 'r', encoding='utf-8') as fh:
    payload = json.load(fh)

status = payload.get('status')
if status is None:
    raise SystemExit('missing status field')

print(status)
PY
)"

if [[ "$status_value" != "valid" ]]; then
  echo "[uat-live-smoke] ERROR phase=status-validate expected status=valid got status=$status_value" >&2
  exit 1
fi
log_phase "validated status JSON contract (status=valid)"

PHASE="generate"
log_phase "running generate prompt and capturing manifest JSON"
generate_stdout="$(mktemp -t codex-image-uat-generate-out.XXXXXX)"
generate_stderr="$(mktemp -t codex-image-uat-generate-err.XXXXXX)"
if ! HOME="$cli_home" CODEX_IMAGE_HOME="$auth_home" "$binary" generate "UAT smoke image from codex-image" --out "$out_dir" >"$generate_stdout" 2>"$generate_stderr"; then
  redact_cli_stderr "$generate_stderr"
  echo '[uat-live-smoke] ERROR phase=generate generate command failed' >&2
  exit 1
fi

PHASE="manifest-validate"
image_count="$(python3 - "$generate_stdout" "$out_dir" <<'PY'
import json
import os
import sys

stdout_path, out_dir = sys.argv[1], sys.argv[2]
with open(stdout_path, 'r', encoding='utf-8') as fh:
    payload = json.load(fh)

manifest_path = payload.get('manifest_path')
if not isinstance(manifest_path, str) or not manifest_path.strip():
    raise SystemExit('manifest_path missing from generate stdout JSON')

if not os.path.isfile(manifest_path):
    raise SystemExit(f'manifest file missing at {manifest_path}')

expected_manifest_path = os.path.join(out_dir, 'manifest.json')
if os.path.realpath(manifest_path) != os.path.realpath(expected_manifest_path):
    raise SystemExit(
        f'manifest_path {manifest_path} does not match expected {expected_manifest_path}'
    )

with open(manifest_path, 'r', encoding='utf-8') as fh:
    manifest = json.load(fh)

images = manifest.get('images')
if not isinstance(images, list) or not images:
    raise SystemExit('manifest images must be a non-empty array')

for index, image in enumerate(images, start=1):
    image_path = image.get('path')
    if not isinstance(image_path, str) or not image_path.strip():
        raise SystemExit(f'manifest image[{index}] missing path')
    if not os.path.isfile(image_path):
        raise SystemExit(f'manifest image[{index}] file missing at {image_path}')

print(len(images))
PY
)"
log_phase "validated manifest and image artifacts (count=$image_count)"

PHASE="codex-auth-preservation"
if [[ "$codex_auth_mode" == "present" ]]; then
  if [[ ! -f "$codex_auth_path" ]]; then
    echo '[uat-live-smoke] ERROR phase=codex-auth-preservation ~/.codex/auth.json removed' >&2
    exit 1
  fi

  codex_auth_after="$(shasum -a 256 "$codex_auth_path" | awk '{print $1}')"
  if [[ "$codex_auth_before" != "$codex_auth_after" ]]; then
    echo '[uat-live-smoke] ERROR phase=codex-auth-preservation ~/.codex/auth.json changed' >&2
    exit 1
  fi

  log_phase "confirmed ~/.codex/auth.json checksum unchanged"
else
  if [[ -f "$codex_auth_path" ]]; then
    echo '[uat-live-smoke] ERROR phase=codex-auth-preservation ~/.codex/auth.json was created' >&2
    exit 1
  fi

  log_phase "confirmed ~/.codex/auth.json remains absent"
fi

PHASE="complete"
log_phase "live UAT smoke completed successfully"
