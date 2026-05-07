#!/usr/bin/env sh
set -eu

REPO="${CODEX_IMAGE_REPO:-tksuns12/codex-image}"
INSTALL_DIR="${CODEX_IMAGE_INSTALL_DIR:-$HOME/.local/bin}"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"

need_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "codex-image installer requires '$1' on PATH" >&2
    exit 1
  fi
}

need_command curl
need_command sed
need_command tar
need_command uname
need_command mktemp
need_command install

VERSION="$(
  curl -fsSL "$API_URL" |
    sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' |
    sed -n '1p'
)"

if [ -z "$VERSION" ]; then
  echo "could not resolve latest codex-image release from ${API_URL}" >&2
  exit 1
fi

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
  Darwin-x86_64) TARGET="x86_64-apple-darwin" ;;
  Darwin-arm64|Darwin-aarch64) TARGET="aarch64-apple-darwin" ;;
  *)
    echo "unsupported platform: $(uname -s)-$(uname -m)" >&2
    exit 1
    ;;
esac

ASSET="codex-image-${VERSION}-${TARGET}.tar.gz"
ARCHIVE_ROOT="codex-image-${VERSION}-${TARGET}"
TMPDIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT HUP INT TERM

curl -fL "https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}" -o "${TMPDIR}/${ASSET}"
tar -xzf "${TMPDIR}/${ASSET}" -C "$TMPDIR"
mkdir -p "$INSTALL_DIR"
install -m 0755 "${TMPDIR}/${ARCHIVE_ROOT}/codex-image" "${INSTALL_DIR}/codex-image"

echo "installed codex-image ${VERSION} to ${INSTALL_DIR}/codex-image"
echo "make sure ${INSTALL_DIR} is on your PATH"
"${INSTALL_DIR}/codex-image" --help >/dev/null
