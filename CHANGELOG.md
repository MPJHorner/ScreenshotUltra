# Changelog

All notable changes to Screenshot Ultra are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — M2 (in progress)
- Quick Tray: a floating bottom-right panel that appears after a capture with
  Copy / Folder / Reveal / Discard buttons and auto-dismisses after
  `quick_tray_timeout_ms` (default 6 s). Native `NSPanel` via `objc2`.
- Silent capture variants (`silent_region`, `silent_fullscreen`) — separate
  hotkey slots that skip the tray and just save+copy. Unbound by default;
  set them in `settings.toml` to enable.
- Tray menu now lists both standard and silent variants.

### Added — M1 "Press the key"
- Menu-bar agent (`LSUIElement = true`) with a status-bar icon and basic menu.
- Global hotkey listener with rebindable accelerators in `settings.toml`.
- Region capture (default `⌃⌥⌘1`) — interactive marquee, Esc cancels.
- Fullscreen capture (default `⌃⌥⌘3`) — main display, single file.
- Disk sink: writes to `~/Pictures/ScreenshotUltra/` with templated filenames.
- Clipboard sink: PNG/JPG goes straight onto the macOS clipboard.
- NDJSON event log at `~/Library/Logs/ScreenshotUltra/log.ndjson`.
- `settings.toml` with first-run defaults at
  `~/Library/Application Support/ScreenshotUltra/settings.toml`.
- `make app` builds a `.app` bundle in `dist/` from `mac/Info.plist`.
- `scripts/install.sh` one-liner (release download with source-build fallback).
- GitHub Actions CI (fmt + clippy + tests + release build on macOS 14).

### Notes
- The capture backend in M1 shells out to `/usr/sbin/screencapture`. A native
  ScreenCaptureKit backend is planned in a later milestone.
- Clipboard copy uses `osascript` for reliable image transfer; this will move
  to a native path alongside the ScreenCaptureKit backend.
