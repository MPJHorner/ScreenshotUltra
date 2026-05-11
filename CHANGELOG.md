# Changelog

All notable changes to Screenshot Ultra are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] — 2026-05-11

The "annotate everything" release. Nine editor tools, full-colour palette,
stroke-width picker, undo/redo, Apple Preview-quality save back to PNG.

### Added — editor: Counter + Text + docs page
- **Counter tool** (`N`) — click drops an auto-incrementing numbered
  circle in the current colour with a white outline + bold white digit.
  Numbers reset to 1 each time you open the editor.
- **Text tool** (`T`) — click prompts for a string (native `NSAlert` +
  `NSTextField`) and places it at the click point. Uses the current
  colour; size derives from the stroke-width picker.
- New **Annotation editor** docs page on the site (linked from the top
  nav).
- Editor file-header comment refreshed to list the full tool roster.

### Added — editor expansion
- **Three more editor tools**: Line (no arrowhead), Highlighter
  (translucent thick yellow), Redact (filled black rectangle). Total now
  seven: Pen / Line / Arrow / Rect / Ellipse / Highlighter / Redact.
- **Five-colour palette** (Red / Yellow / Green / Blue / Black) — each
  annotation captures its colour at draw time so later palette changes
  don't repaint prior strokes.
- **Stroke-width picker** (Thin 3 px / Med 6 px / Thick 12 px) with
  number-key shortcuts `1` / `2` / `3`.
- New tool shortcuts: `P` Pen, `L` Line, `A` Arrow, `R` Rect, `E`
  Ellipse, `H` Highlighter, `X` Redact.
- Editor toolbar grew a second row above the canvas: tools on row 1,
  colours + width on row 2. Bottom row keeps the actions (Save/Copy/
  Undo/Redo/Clear/Done).

## [0.2.0] — 2026-05-11

### Added — M2 (annotation editor + polish)
- **Native annotation editor**, now with **multiple tools**: Pen / Arrow /
  Rectangle / Ellipse. Tool-picker buttons sit above the canvas;
  shortcuts `P` / `A` / `R` / `E` switch tools. ⌘S save (overwrites the
  original), ⌘C copy annotated PNG, ⌘Z undo, **⌘⇧Z redo**, ⌘W close.
  Bottom toolbar mirrors actions plus a Clear button.
- Arrow tool draws a calculated arrowhead at the tip; Rect/Ellipse use
  drag-to-define-bounds; Pen is freehand. All four scale correctly when
  saved back into the original image's pixel dimensions.
- The Quick Tray's **Edit** button now opens the native editor (was
  bouncing through Apple Preview).
- **Timed fullscreen captures** — Tray menu offers `Fullscreen in 3 / 5 / 10 s`.
  Uses `screencapture -T` so the OS-level countdown runs.
- **About window** — `NSAlert` with version, description, and an
  "Open GitHub" button.
- **Panic crash handler** writes a report to
  `~/Library/Logs/ScreenshotUltra/crashes/<timestamp>.txt` and emits a
  `panic` NDJSON event. No auto-send, ever (per plan §11).

### Fixed
- `scripts/install.sh` downloads the universal `.zip` from the release (was
  expecting a `.dmg`, which we don't publish yet) and verifies the SHA-256
  sidecar before installing.

### Added — M2 (earlier this milestone)
- **Edit button** on the Quick Tray — opens the capture in Apple
  Preview for annotation (markup tools, signatures, redaction). A
  placeholder for the native editor planned later in M2.
- **`[capture]` settings**:
  - `include_cursor` — pass `-C` to `screencapture` so the mouse
    cursor is baked into the image.
  - `fullscreen_scope = "main"` (default) or `"all"` — main display
    only vs. every connected display.
- **Documentation site** under `site/` (mkdocs-material) — Install,
  Quick start, Hotkeys, Capture modes, Sinks, Configuration, Logging,
  Changelog. Deployed via `.github/workflows/pages.yml`.
- **Settings hot-reload** — `settings.toml` is watched in a background thread
  and changes are applied within ~1 s. Invalid hotkeys keep the previous
  binding (with a log message) instead of crashing.
- **Open clipboard image** (default `⌃⌥⌘E`) — pulls a PNG off the macOS
  clipboard, saves it to the normal save folder, and runs it through the
  Quick Tray.
- **Release workflow** — `.github/workflows/release.yml` builds a universal
  (arm64 + x86_64) `.app`, zips it via `ditto`, and uploads to a tagged
  GitHub release with a SHA-256 sidecar. Unsigned for now; signing in M6.
- **Manual test plan** at `tests/MANUAL.md` for release sign-off.
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
