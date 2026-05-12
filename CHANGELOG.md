# Changelog

All notable changes to Screenshot Ultra are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.10.0] — 2026-05-12

The **native recorder** release. `screencapture -v` was a great
bootstrap, but it tops out at ~30 fps and gives us no quality knobs.
v0.10 ships a real ScreenCaptureKit + AVAssetWriter pipeline.

### Added — ScreenCaptureKit recording backend
- **`mac/STURecorder.swift`** — a ~190-line Swift CLI built on
  `SCStream` + `AVAssetWriter`. Captures the main display at H.264
  1080p+ (full Retina pixel dimensions) at **60 fps** by default,
  with a sensible bitrate (≈ `width × height × fps / 8` bps), 2-second
  GOP, High Profile. Cleanly handles SIGTERM/SIGINT to finalise the
  `.mov` (writes the `moov` atom before exit).
- **`scripts/build-recorder.sh`** compiles a **universal binary**
  (arm64 + x86_64 on supported SDKs, arm64-only otherwise) via
  `swiftc` and drops it at `target/recorder/STURecorder`. Graceful
  no-op when `swiftc` isn't on PATH.
- **`make app`** bundles the binary into
  `Contents/Resources/STURecorder` so the `.app` is fully self-
  contained.
- `src/recording.rs` now tries the bundled `STURecorder` first and
  **falls back to `screencapture -v` automatically** if (a) the
  bundle doesn't have one (dev build without `swiftc`), (b) spawning
  fails, or (c) the user sets `[recording].use_screen_capture_kit = false`.
- Per-backend signal handling: SIGTERM for STURecorder (matches its
  graceful shutdown), SIGINT for `screencapture -v` (what it expects).
- `recording_start` NDJSON event now records the `backend` field
  (`screencapturekit` or `screencapture`).
- New setting `[recording].use_screen_capture_kit` (default `true`).

### Why a Swift bridge (not pure-Rust objc2)?
- `SCStreamOutput` is a Cocoa delegate with async sample-buffer
  callbacks. `AVAssetWriter` has its own threading model around
  `expectsMediaDataInRealTime` and `startSession(atSourceTime:)`.
- Swift expresses this in ~190 lines with idiomatic `async` /
  `await`. The same in `objc2` would be 600+ lines of
  `RcBlock`-wrapped delegate methods + manual `CMSampleBuffer`
  juggling. The Swift binary is 250 KB and lives inside the .app
  bundle — no runtime hit, no extra deps.

## [0.9.1] — 2026-05-12

### Added — per-mode shell sink overrides
- New `[sinks].shell_region`, `shell_window`, `shell_fullscreen`,
  `shell_video`, `shell_gif` fields. If set, override the global
  `[sinks].shell` for that capture mode — so region captures can land
  on S3 while videos go to your team's Slack while GIFs go to a
  personal web server. Each override is independently optional;
  whatever's empty falls back to the global `shell`.
- `Sinks::shell_for(mode)` is the single lookup used by every
  capture path (still capture, timed fullscreen, and recording).
  Timed-fullscreen captures use the `fullscreen` override.
- 5 new tests for the lookup logic.

Example:

```toml
[sinks]
shell        = "rclone copy $1 cloud:misc/"
shell_video  = "scp $1 home.example.com:/var/www/recordings/"
shell_gif    = "/usr/local/bin/upload-shot $1"
```

## [0.9.0] — 2026-05-12

### Changed — M6 (sign/notarise) parked
- M6 (signed `.dmg` + notarisation + Homebrew cask) is parked until a
  Developer ID Application certificate is available to test against.
  Until then the existing tag-driven pipeline continues to ship
  unsigned universal `.zip`s. plan.md, milestone README, and
  `docs/milestones/M6-ship.md` all flagged 🔒 parked.

### Added — opt-in update check
- New setting **`[general].check_for_updates = false`** (off by default
  per plan §11 "no telemetry / no phone-home unless the user opts in").
  When on: at startup (after a 30 s settling delay) and every 24 hours
  we poll the GitHub Releases API for a newer `tag_name`. If one
  exists, fire a notification banner and emit an `update_available`
  NDJSON event.
- **Tray menu** gains a `Check for Updates…` entry that runs the
  check on demand regardless of the setting and notifies whether
  you're up to date. Useful even with the auto-poll off.
- Uses `/usr/bin/curl` rather than pulling in `reqwest` for a single
  GET — keeps the binary small.
- 7 new unit tests for the SemVer-ish comparator.
- We never auto-install (signing pipeline isn't in place; see M6
  parked status).

### Added — drag-out from Pin windows
- The Pin window's image is now a drag source too. Drag a pinned
  screenshot into Slack / Mail / Finder / any drop target that
  accepts a file URL and the file lands there.
- The `STUPinControl` overlay view was folded back into the image
  view itself (new `STUDraggablePin`, NSImageView subclass) so one
  class handles drawing, scroll-dim, keyboard zoom + close, *and*
  drag-out. Multi-pin support via a `PIN_PATHS: HashMap<view_ptr,
  PathBuf>` keyed by stable view pointer.
- New NDJSON event: `pin_action.action = "drag_out"`.

## [0.8.3] — 2026-05-12

### Added — first-run welcome window
- On first launch the app opens a styled `NSWindow` walking the user
  through the three things they need to know: (1) the Screen Recording
  permission, (2) the default hotkeys, (3) the menu bar icon.
- Two deep-link buttons jump straight to the relevant System Settings
  panes:
  - **Grant Screen Recording…** →
    `x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture`
  - **Grant Accessibility (optional)…** →
    `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`
  (The Accessibility one is only needed for the keystroke overlay.)
- Hotkey summary inside the welcome window is pulled from the **live**
  `Settings`, so rebound bindings show up if the user has imported a
  config from another machine.
- Gated by a versioned marker file at
  `~/Library/Application Support/ScreenshotUltra/.welcomed-v1` — users
  upgrading from an earlier version don't get re-welcomed.
- New NDJSON event: `welcomed`.

## [0.8.2] — 2026-05-12

### Added — drag-out from the Quick Tray
- The Quick Tray's thumbnail is now a real drag source. Hold the mouse
  button on the thumb, drag, drop onto Slack / Mail / a Finder window /
  iMessage / your team's CRM — anywhere that accepts a file URL — and
  the screenshot lands there.
- Mechanically: `STUDraggableThumb` is an `NSImageView` subclass with
  `mouseDown:` / `mouseDragged:` handlers + the required
  `draggingSession:sourceOperationMaskForDraggingContext:` method
  (returning `Copy`). On drag, we build an `NSPasteboardItem` with
  `NSPasteboardTypeFileURL` pointing at the captured file, wrap it in
  an `NSDraggingItem` whose drag-image is the live thumbnail, and call
  `beginDraggingSessionWithItems:event:source:`.
- New NDJSON event: `tray_action.action = "drag_out"`.

## [0.8.1] — 2026-05-12

### Added — Keystroke overlay for screencasts
- Setting **`[recording].keystroke_overlay = true`** (default off) shows
  a floating dark-glass pill at the bottom-centre of the main screen
  whenever a recording is active. Every key (and modifier combo) you
  type briefly flashes into the pill — perfect for screencasts where
  you want viewers to see which shortcut you just hit without
  narrating.
- Translates raw NSEvent characters into the macOS glyphs you'd expect:
  `⌘C`, `↩`, `⌫`, `Tab`, `Space`, arrow keys, `F1`–`F12`, etc.
- New module `src/keystroke_overlay.rs` (~290 LOC). NSEvent global
  monitor + transparent borderless click-through NSWindow + custom
  `STUKeystrokePill` NSView that paints the rounded pill. Keys fade
  after 2 s; up to 8 chunks shown at once. Requires Accessibility
  permission (macOS prompts on first use).
- `recording::start` enables the overlay if the setting's on;
  `recording::stop` always tears it down.

## [0.8.0] — 2026-05-12

The **history-and-OCR** release. Two new in-app windows plus a custom
hand-rolled docs site that matches the rest of the Ultra family
exactly.

### Added — History window (`⌃⌥⌘H`)
- Native `NSWindow` browser of the per-folder NDJSON history index
  (`<save_folder>/.screenshot-ultra/index.ndjson`). One row per
  capture, most recent first, fixed-width columns for timestamp /
  mode / size / filename. ⌘F searches the view.
- Friendly "No history yet" empty state for first-run.
- New `Action::History`, tray menu "History…" entry.

### Added — Editor "OCR" button
- New action button in the annotation editor's bottom row that runs
  Apple Vision against the source image and copies the recognised
  text onto the clipboard via `pbcopy`. Same notification flow as the
  Quick Tray's Text button.
- 12 default hotkeys now register at startup (added `history`).

### Changed — custom docs site replacing mkdocs-material
- The whole `site/` directory is now a hand-rolled static site
  matching MailBox Ultra / Postbin Ultra exactly:
  - Node 20 builder in `site/build.mjs` (~370 LOC, deps: gray-matter +
    marked).
  - `site/templates/{layout,home}.html` + `partials/{nav,footer}.html`
    with the aperture-iris SVG logo inline.
  - `site/static/style.css` carried over from MailboxUltra with the
    palette swapped to brand red (`#FF3D54` / `#B91D36`), light-mode
    accent `#DC2542`.
  - `site/content/*.md` — 10 frontmatter-equipped pages including a
    full home hero (badge, gradient headline, hero meta strip, feature
    grid, four-step tour, "Part of the Ultra family" cross-link grid).
  - `.github/workflows/pages.yml` swapped from `pip install mkdocs-material`
    to `setup-node@v4` + `npm ci` + `node build.mjs`.
  - The build pulls version from `Cargo.toml` and parses
    `CHANGELOG.md` release sections at build time so the docs site
    can never drift from shipping code.
- Site lives at the same URL: <https://mpjhorner.github.io/ScreenshotUltra/>.

## [0.7.0] — 2026-05-12

The **M5 v0** release. On-device OCR via Apple Vision lands as a Quick
Tray button.

### Added — OCR (Apple Vision, on-device)
- **"Text" button on the Quick Tray** — pulls every recognised string
  out of the capture via `VNRecognizeTextRequest` and dumps it onto the
  clipboard. Result count + notification banner so you know how much
  text came out.
- Recognition level pinned to `Accurate`, language correction on. Pure
  on-device — no network calls, no API keys, no per-character pricing.
- New `src/ocr.rs` (`extract_text(path) -> Option<String>`) — same
  boundary as the rest of our native helpers.
- New NDJSON event: `ocr` with `path` and `chars` fields.

### Added — better video Quick Tray (poster frames)
- `.mov` / `.mp4` / `.m4v` captures now show a **poster-frame
  thumbnail** in the Quick Tray, generated by macOS's own `qlmanage -t`
  Quick Look engine. Cached under `$TMPDIR/screenshot-ultra-posters/`
  so repeated shows of the same recording skip the spawn.

## [0.6.2] — 2026-05-12

### Added — Cheat Sheet window (`⌃⌥⌘/`)
- New in-app reference card listing every global hotkey + every editor
  tool / colour / width / action / pin gesture / Quick-Tray button.
- Rendered into a read-only monospaced `NSTextView` so users can scroll,
  `⌘F` search, and copy lines. Window is resizable + searchable.
- Hotkey bindings come from the **live** `Settings` — if you've
  rebound `region` to `⌃⌥⌘5`, the cheat sheet shows `⌃⌥⌘5`. Unbound
  slots render as `—`.
- New tray menu item "Cheat Sheet" with the `⌃⌥⌘/` accelerator shown
  inline.

## [0.6.1] — 2026-05-12

Recording UX polish on top of v0.6.0's screen-recording v0.

### Added — recording feedback
- **Menu-bar indicator** — the aperture-iris glyph swaps to a filled
  solid disc whenever a recording is in progress, and the tooltip
  reads "Screenshot Ultra — recording". Removes the uncertainty about
  "did I start it / stop it" that v0.6.0 had.
- **Notification banner** on recording stop — top-right macOS banner
  reads `<filename> · <size> · <duration>` so you can see your
  recording landed even if you missed the Quick Tray (which can't
  render a `.mov` thumbnail).
- New `sinks::notify(title, body)` helper (shells out to
  `osascript display notification`, so no UNUserNotification
  permission dance).
- New `tray::register(TrayIcon)` + `tray::set_recording_indicator(bool)`
  — the tray module now owns the constructed `TrayIcon` so anywhere in
  the app can toggle the indicator without threading it through.

## [0.6.0] — 2026-05-12

The **M3 begins** release. Video + GIF recording lands, plus a third row
in the tray menu and two new global hotkeys.

### Added — M3 v0 recording
- **Toggle video recording** (default `⌃⌥⌘V`) — press to start, press
  again to stop. Saves as `<save_folder>/...mov` with the same
  `filename_template` as still captures. Mode token = `video`.
- **Toggle GIF recording** (default `⌃⌥⌘G`) — captures as `.mov`, then
  post-processes through `ffmpeg` (12 fps + generated palette for crisp
  small files) into a `.gif`. Without `ffmpeg`, falls back to keeping
  the `.mov` with a log line.
- **Mouse-click highlight** in recordings via `screencapture -k` —
  enable / disable via `[recording].show_clicks` (default on).
- **Microphone capture** via `screencapture -g` —
  `[recording].record_microphone` (default off).
- Tray menu adds **Record Video / Stop** and **Record GIF / Stop**
  items. Quick Tray fires for the resulting file once recording stops.
- New NDJSON events: `recording_start`, `recording_stop`.
- New module `src/recording.rs` (~270 LOC). The `start()` / `stop()` /
  `toggle()` boundary is stable so the planned ScreenCaptureKit +
  AVAssetWriter swap-in is contained.

### Notes
- M3 v0 piggybacks on macOS's `screencapture -v` binary so we can ship
  start/stop video recording today. A native ScreenCaptureKit pipeline
  lands in v0.7 alongside system-audio capture and the keystroke
  overlay (which `screencapture` can't do).
- The full-screen Quick Tray "Edit" button still routes through the
  annotation editor for stills only; videos open in the user's default
  player.

## [0.5.0] — 2026-05-11

The **M2 complete** release. The Preferences GUI lands, finishing the
last material item on the M2 milestone.

### Added — Preferences window
- **In-app Preferences** (default `⌃⌥⌘,`, plus tray-menu "Preferences…"):
  a real `NSWindow` with a scrollable `NSTextView` showing the current
  `settings.toml`. Edit in place; the **Apply** button validates the TOML
  by parsing it through `Settings`'s serde derives, then writes back to
  disk so the hot-reload watcher picks it up — same path as external
  editors.
- **Reset to Defaults** button reloads the canonical defaults into the
  editor (doesn't save until you click Apply).
- **Cancel** (`Esc`) or `⌘W` closes without saving.
- Apply errors land in a native `NSAlert` — typos won't silently revert
  your bindings.
- Logs `preferences_apply` on successful save.

### Fixed — upgrade-friendly settings parsing
- Every hotkey slot now has an explicit per-field serde default function
  (`default_window`, `default_pin_last`, …). Previously
  `#[serde(default)]` fell back to `String::default()` → empty, so
  anyone upgrading from an older settings.toml silently lost the new
  bindings (window, pin_last, repeat_last, open_clipboard_image,
  color_picker, preferences).
- `Settings::load_or_default` now rewrites the file when round-tripping
  reveals missing fields, so the next `Edit settings.toml…` shows every
  current option.

## [0.4.1] — 2026-05-11

### Added — pin-to-screen interaction polish
- **Scroll-wheel opacity** — hover any pinned window and scroll to dim
  it (alpha 1.0 → 0.3). Lets you keep a pin visible while reading
  through it.
- **⌘+ / ⌘- zoom** — scales the pin in place around its centre.
- **⌫ / ⌫-forward / Esc closes the pin** — quick dismiss without
  reaching for the title-bar close button.
- **⌘0 resets alpha to 1.0** for when you've dimmed too far.
- Implemented as a transparent `PinControl` `NSView` overlay on top of
  the image view, made first responder so it gets scroll + key events.

### Added — eyedropper, tooltips, aperture menu-bar icon
- **Eyedropper colour picker** (default `⌃⌥⌘P`) — shows macOS's
  `NSColorSampler` magnifier; clicking any on-screen pixel copies its
  sRGB hex (`#rrggbb`) onto the clipboard and emits an `eyedropper_pick`
  NDJSON event. New `src/eyedropper.rs`. Uses `block2::RcBlock` to bridge
  the Cocoa selection-handler block.
- **Tooltips on every editor button** — hover any tool / colour / width
  / action button to see a one-line hint with the shortcut.
- **Aperture menu-bar icon** — replaced the programmatic camera glyph
  with a six-blade aperture iris that matches the .app icon (which is
  itself an aperture). Still procedurally drawn at 22×22; still a
  template image so macOS tints it for the menu bar's light/dark mode.

## [0.4.0] — 2026-05-11

The "annotate everything, then some" release. The annotation editor now
ships with **eleven tools** (Pen / Line / Arrow / Rect / Ellipse /
Highlighter / Redact / Counter / Text / Blur / Crop), visual indicators
for the active tool / colour / width, and a real app icon. README and
docs site restyled to match the **Ultra** family.

### Added — app icon
- Hand-tuned SVG aperture icon (`icon/icon.svg`) generated to match
  the macOS squircle + sheen treatment of the sister projects, with
  the brand camera-shutter red (`#FF3D54`) gradient and a six-bladed
  iris around a glossy lens.
- `scripts/render-icon.sh` builds the full `.icns` from the SVG using
  Swift's `NSImage` + `iconutil`. Pure macOS tooling — no `brew install`
  required. Hooked into `make app` as a dependency of the bundle.
- `Info.plist` declares `CFBundleIconFile = AppIcon` so the app shows
  the icon in `/Applications`, Spotlight, the App Switcher, and the
  Quick Tray's Reveal-in-Finder result.

### Changed — Ultra-family styling
- README rewritten to match the **MailBox Ultra** / **Postbin Ultra**
  layout: badges row, blockquote of doc links, **Why**, install one-liner
  in a 🚀 blockquote, Quick start, What it does, Hotkeys + Editor
  shortcut tables, Configuration, Sinks & shell, Development, Documentation,
  Sister projects, Contributing, License.
- Docs site `index.md` follows the same three-promises structure as the
  sister sites.
- mkdocs accent colour set to `pink` (closest Material accent to the
  brand `#FF3D54`).

### Added — active-selection indicators
- Tool, colour, and stroke-width buttons now visually reflect the
  current selection via `NSButton.state` (Cocoa renders the "on" state
  with a pressed-in highlight). No more guessing which tool is active.
- `set_tool` / `set_color` / `set_width` refresh button state in lockstep.
- Initial state seeded from `open()` so the default `Pen / Red / Thin`
  shows highlighted on first paint.

### Added — Crop tool
- **Crop tool** (`C`) — drag a yellow guide rectangle, release to crop
  the image to that region. The window resizes to the new aspect ratio,
  annotations are cleared, and a `editor_crop` NDJSON event is emitted.
  Crop is destructive (not undoable via ⌘Z) by design; ⌘Z still affects
  annotations.

### Added — Blur tool
- **Blur tool** (`B`) — pixelate-style privacy filter. Drag to define a
  rectangle; the tool reads the corresponding sub-rect from the source
  `NSImage`, downsamples it to `dst_size / (width × 3)` via
  `NSBitmapImageRep`, then redraws it across the destination rect with
  `NSImageInterpolation::None` to produce a chunky mosaic. Works in both
  on-screen painting (view coords) and save rendering (pixel coords)
  via a single `pixelate_blur` helper that picks coordinate space from
  context bounds.

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
