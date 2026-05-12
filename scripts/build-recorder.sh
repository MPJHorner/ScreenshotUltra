#!/usr/bin/env bash
# Compile mac/STURecorder.swift into a universal binary at
# target/recorder/STURecorder. The .app bundle copies it into
# Contents/Resources/ so src/recording.rs can spawn it.
#
# Graceful no-op: if `swiftc` isn't on PATH this script prints a notice
# and exits 0 — Rust falls back to the screencapture -v pipeline so the
# build still produces a working bundle on machines without Xcode CLT.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${ROOT}/mac/STURecorder.swift"
OUT_DIR="${ROOT}/target/recorder"
OUT="${OUT_DIR}/STURecorder"

if ! command -v swiftc >/dev/null 2>&1; then
  echo "build-recorder: swiftc not on PATH; skipping SCK recorder build (screencapture -v fallback will be used)"
  exit 0
fi

mkdir -p "${OUT_DIR}"

# Build per-arch then lipo. macOS 13 is the floor (SCK availability).
build_arch() {
  local triple="$1" out="$2"
  swiftc \
    -O \
    -target "${triple}" \
    -framework ScreenCaptureKit \
    -framework AVFoundation \
    -framework CoreMedia \
    -framework CoreVideo \
    -framework CoreGraphics \
    -framework AppKit \
    -framework Foundation \
    "${SRC}" \
    -o "${out}"
}

build_arch "arm64-apple-macos13" "${OUT_DIR}/STURecorder.arm64" 2>&1 | sed 's|^|  arm64: |' || {
  echo "build-recorder: arm64 build failed" >&2
  exit 1
}
# Skip x86_64 on Apple Silicon if the SDK doesn't have it (common on
# newer macOS releases). The arm64 build alone is fine for a local run.
if build_arch "x86_64-apple-macos13" "${OUT_DIR}/STURecorder.x86_64" 2>&1 | sed 's|^|  x86_64: |'; then
  lipo -create \
    "${OUT_DIR}/STURecorder.arm64" \
    "${OUT_DIR}/STURecorder.x86_64" \
    -output "${OUT}"
  rm -f "${OUT_DIR}/STURecorder.arm64" "${OUT_DIR}/STURecorder.x86_64"
  echo "build-recorder: wrote universal ${OUT}"
else
  mv "${OUT_DIR}/STURecorder.arm64" "${OUT}"
  echo "build-recorder: wrote arm64-only ${OUT} (x86_64 SDK not available)"
fi
