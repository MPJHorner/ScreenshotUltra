# Changelog

All notable changes to Screenshot Ultra are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — M2 (in progress)
- **Shell sink** — `sinks.shell = "scp $1 user@host:/path/"` runs an
  arbitrary shell command with the capture's file path as `$1`. Spawned
  detached so a slow uploader can't stall capture.
- **History NDJSON index** at `<save_folder>/.screenshot-ultra/index.ndjson`,
  one JSON line per capture. Searchable with `jq`/`fzf`/`grep`.
- **Shutter sound** plays `/System/Library/Sounds/Grab.aiff` when
  `[general].play_shutter_sound = true` (the default).
- **CLI flags**: `--version`, `--help`, `--settings-path`,
  `--print-defaults`. Useful for shell pipelines and onboarding.
- **Window capture mode** (default `⌃⌥⌘2`) — interactive window selection
  via `screencapture -W`, with the window's drop shadow trimmed off.
- **Pin-to-screen** (default `⌃⌥⌘.`) — floating always-on-top window holding
  the most recent capture. Multiple pins supported; each cascades 24 px
  from the previous. Pin button added to the Quick Tray.
- **Repeat last capture** (default `⌃⌥⌘R`) — re-runs whatever mode you used
  last with the same tray-or-silent behaviour.
- Tray menu: `Edit settings.toml…`, `Reveal Log File`.
- Tray actions for every capture mode plus pin/repeat, mirroring hotkeys.
- 22 unit tests across hotkey parsing, settings serialisation, capture
  templating, and pin-window sizing.

### Added — M2 (earlier)
- Quick Tray: a floating bottom-right window that appears after a capture
  with Copy / Folder / Reveal / Pin / Discard buttons and auto-dismisses
  after `quick_tray_timeout_ms` (default 6 s). Native `NSWindow` via
  `objc2`; promotes the app to `.Accessory` activation policy on demand so
  the window actually draws for `LSUIElement` background apps.
- Silent capture variants (`silent_region`, `silent_fullscreen`,
  `silent_window`) — separate hotkey slots that skip the tray and just
  save+copy. Unbound by default; set them in `settings.toml` to enable.

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
