#!/usr/bin/env bash
# Render icon/icon.svg into a full macOS AppIcon.icns.
#
# Pipeline:
#   icon.svg → NSImage → NSBitmapImageRep at 7 sizes (Swift one-liner)
#       → Apple iconset naming convention → iconutil -c icns
#
# Self-contained: only depends on `swift`, `iconutil` and `cp`, all bundled
# with macOS, so it works on any developer Mac without `brew install`.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ICON_SVG="${ROOT}/icon/icon.svg"
ICONSET="${ROOT}/icon/AppIcon.iconset"
ICNS="${ROOT}/icon/AppIcon.icns"

if [[ ! -f "${ICON_SVG}" ]]; then
  echo "render-icon: ${ICON_SVG} not found" >&2
  exit 1
fi

TMP_SCRIPT="$(mktemp -t svg2png-XXXXXX.swift)"
trap 'rm -f "$TMP_SCRIPT"' EXIT

cat > "$TMP_SCRIPT" <<'SWIFT'
import AppKit
let args = CommandLine.arguments
guard args.count == 4 else { fputs("usage: svg2png <in.svg> <size> <out.png>\n", stderr); exit(2) }
let inUrl = URL(fileURLWithPath: args[1])
let size = Double(args[2])!
let outUrl = URL(fileURLWithPath: args[3])
guard let img = NSImage(contentsOf: inUrl) else { fputs("failed to load svg\n", stderr); exit(1) }
let target = NSSize(width: size, height: size)
let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(size), pixelsHigh: Int(size), bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bitmapFormat: [], bytesPerRow: 0, bitsPerPixel: 0)!
rep.size = target
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
img.draw(in: NSRect(origin: .zero, size: target), from: .zero, operation: .copy, fraction: 1.0)
NSGraphicsContext.restoreGraphicsState()
guard let data = rep.representation(using: .png, properties: [:]) else { exit(1) }
try data.write(to: outUrl)
SWIFT

rm -rf "${ICONSET}"
mkdir -p "${ICONSET}"

render() {
  local size="$1" out="$2"
  swift "$TMP_SCRIPT" "${ICON_SVG}" "${size}" "${out}" >/dev/null
}

render 16   "${ICONSET}/icon_16x16.png"
render 32   "${ICONSET}/icon_16x16@2x.png"
render 32   "${ICONSET}/icon_32x32.png"
render 64   "${ICONSET}/icon_32x32@2x.png"
render 128  "${ICONSET}/icon_128x128.png"
render 256  "${ICONSET}/icon_128x128@2x.png"
render 256  "${ICONSET}/icon_256x256.png"
render 512  "${ICONSET}/icon_256x256@2x.png"
render 512  "${ICONSET}/icon_512x512.png"
render 1024 "${ICONSET}/icon_512x512@2x.png"

iconutil -c icns "${ICONSET}" -o "${ICNS}"
echo "wrote ${ICNS}"
