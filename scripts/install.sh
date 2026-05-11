#!/usr/bin/env bash
# Screenshot Ultra — one-liner installer.
#
#   curl -sSL https://raw.githubusercontent.com/MPJHorner/ScreenshotUltra/main/scripts/install.sh | bash
#
# Until we ship signed .dmg releases this falls back to a from-source build
# (requires cargo). The same script will fetch a pre-built .dmg once releases
# exist.

set -euo pipefail

REPO="MPJHorner/ScreenshotUltra"
APP_NAME="Screenshot Ultra"
BIN="screenshot-ultra"

err()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; }
info() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }

case "$(uname -s)" in
  Darwin) ;;
  *) err "Screenshot Ultra is macOS-only in v1."; exit 1 ;;
esac

# Try the GitHub release path first.
TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' || true)"

if [[ -n "${TAG:-}" ]]; then
  # We currently ship a universal .zip (signed/notarized .dmg lands in M6).
  ZIP="ScreenshotUltra-${TAG}-universal.zip"
  SHA="${ZIP}.sha256"
  URL="https://github.com/${REPO}/releases/download/${TAG}/${ZIP}"
  SHA_URL="https://github.com/${REPO}/releases/download/${TAG}/${SHA}"
  TMP="$(mktemp -d)"
  info "Downloading ${ZIP}"
  if curl -fsSL -o "${TMP}/${ZIP}" "${URL}"; then
    if curl -fsSL -o "${TMP}/${SHA}" "${SHA_URL}" 2>/dev/null; then
      info "Verifying SHA-256"
      ( cd "${TMP}" && /usr/bin/shasum -a 256 -c "${SHA}" >/dev/null ) \
        || { err "checksum mismatch on ${ZIP}"; exit 1; }
    else
      info "Skipping SHA-256 (no sidecar file)"
    fi
    info "Removing any previous /Applications/${APP_NAME}.app"
    rm -rf "/Applications/${APP_NAME}.app"
    info "Expanding ${ZIP} to /Applications"
    /usr/bin/ditto -xk "${TMP}/${ZIP}" /Applications/
    xattr -dr com.apple.quarantine "/Applications/${APP_NAME}.app" || true
    info "Launching for first-run permissions prompt"
    open "/Applications/${APP_NAME}.app" || true
    info "Done. Press ⌃⌥⌘1 for region, ⌃⌥⌘3 for fullscreen."
    exit 0
  fi
  info "Release download failed — falling back to source build."
fi

# Source-build fallback.
if ! command -v cargo >/dev/null 2>&1; then
  err "cargo not found. Install Rust from https://rustup.rs and re-run, or wait for a signed .dmg release."
  exit 1
fi

WORK="$(mktemp -d)"
info "Cloning ${REPO} into ${WORK}"
git clone --depth=1 "https://github.com/${REPO}.git" "${WORK}/src"
cd "${WORK}/src"
info "Building release binary (this takes a minute on first run)"
cargo build --release
info "Building .app bundle"
make app
mv "dist/${APP_NAME}.app" /Applications/ 2>/dev/null || cp -R "dist/${APP_NAME}.app" /Applications/
xattr -dr com.apple.quarantine "/Applications/${APP_NAME}.app" || true
info "Launching for first-run permissions prompt"
open "/Applications/${APP_NAME}.app" || true
info "Done. Press ⌃⌥⌘1 for region, ⌃⌥⌘3 for fullscreen."
