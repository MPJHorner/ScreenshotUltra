# Screenshot Ultra — Implementation Plan

> A native macOS screenshot + screen recording app, driven entirely by configurable global hotkeys. Local-first, offline, no telemetry, no cloud account. Sister project to **MailBox Ultra** and **Postbin Ultra**.

## Delivery status

This document is the design. Delivery is tracked per milestone in
[`docs/milestones/`](docs/milestones/):

| Milestone | Title              | Status                |
|-----------|--------------------|-----------------------|
| [M1](docs/milestones/M1-press-the-key.md)        | Press the key       | ✅ shipped (v0.1.0)   |
| [M2](docs/milestones/M2-annotate.md)             | Annotate            | ⏳ planned            |
| [M3](docs/milestones/M3-record.md)               | Record              | ⏳ planned            |
| [M4](docs/milestones/M4-scroll-and-beautify.md)  | Scroll & Beautify   | ⏳ planned            |
| [M5](docs/milestones/M5-ocr-and-polish.md)       | OCR & polish        | ⏳ planned            |
| [M6](docs/milestones/M6-ship.md)                 | Ship v1.0           | ⏳ planned            |

§13 below holds the original milestone breakdown verbatim; the
[`docs/milestones/`](docs/milestones/) files are the working surface where
scope is checked off as work lands.

---

## 0. North Star

**One sentence:** Press a key, capture anything on screen — region, window, scrolling page, video, or GIF — annotate it inline, and have it on your clipboard before the shutter sound finishes.

**Three promises:**

1. **Snappy.** Hotkey → capture is sub‑100 ms cold, sub‑16 ms warm. No splash screens, no spinner. The overlay must feel like an OS primitive, not an app.
2. **Hotkey-first.** Every action is keyboardable. Mouse use is optional. Hotkeys are fully rebindable from Preferences.
3. **Local-first.** No accounts, no analytics, no auto-upload. Captures live on your disk, in a folder you choose. Optional "share to URL" is opt-in and ships disabled.

**Non-goals (explicit):**

- No cloud-hosted gallery / no "Screenshot Ultra Cloud".
- No team features, no comments, no SaaS.
- No mobile companion app.
- No AI-anything in v1. Maybe local OCR via Apple Vision later. No remote LLM calls, ever.
- No installer pkg with kernel extensions. App bundle only.

---

## 1. Reference apps & what to steal

| Source | What we copy | What we skip |
|---|---|---|
| **CleanShot X** | Region / window / fullscreen / scrolling capture; pin-to-screen overlay; quick-access tray; video + GIF; annotation toolbar; "all-in-one" feel | Cloud sync, paid tiers, opinionated branding |
| **Shottr** | Pixel-perfect crosshair + magnifier; OCR; measurement ruler; tiny binary; speed obsession | Solo-dev quirks |
| **Xnapper** | Beautify mode (gradient backdrop, device frames, padding) for social-ready exports | Auto-redact-by-default |
| **macOS built-in (⌘⇧5)** | Familiar default keymap; control bar UX patterns | Single-format output, no annotation, no scrolling |
| **MailBox Ultra / Postbin Ultra** | Project layout, Rust + egui stack, docs site shape, install.sh one-liner, MIT, no-telemetry posture, NDJSON logging convention | — |

---

## 2. Feature list (v1.0 scope)

### 2.1 Capture modes

- **Region capture** — drag a marquee; arrow keys nudge; shift-arrow resizes; space to grab a window under cursor; esc cancels.
- **Window capture** — hover to highlight, click to capture. Captures pixel-perfect with transparent corners and a synthesized drop shadow (toggleable).
- **Fullscreen capture** — active display.
- **All-displays capture** — every connected display, stitched or as separate files (user choice).
- **Scrolling capture** — start at a region, scroll with trackpad/wheel, app stitches frames in real-time. Live preview panel during capture so you see the stitched result grow.
- **Timed capture** — 3 / 5 / 10 s countdown with on-screen ring.
- **Repeat last capture** — re-runs the previous capture mode at the same coordinates.
- **Clipboard image → editor** — hotkey opens any clipboard image in the annotator (useful for received screenshots).

### 2.2 Recording modes

- **Screen recording (video)** — region / window / fullscreen.
  - H.264 or HEVC via ScreenCaptureKit, 30/60 fps configurable.
  - Optional system audio capture (ScreenCaptureKit, no extension needed on macOS 13+).
  - Optional mic capture, with input device picker and live level meter.
  - Mouse highlight ring + click flash, toggleable.
  - Keystroke overlay, toggleable, with allow/deny list per app.
- **GIF recording** — same UX as video, output is `.gif` via `gifski` for great quality at small size.
- **Recording trim** — after stop, a lightweight trim UI: drag handles on a timeline, no re-encode if cuts land on keyframes.

### 2.3 Annotation editor

Native window opens immediately after every capture (configurable: open, copy-only, both). Toolbar is icon-first with single-key shortcuts.

Tools:

- Arrow, line, rectangle, ellipse, freehand pen.
- Highlighter (translucent rect).
- Text with font picker + bg fill.
- Numbered counter (auto-increments per click).
- Blur, pixelate, redact (solid black bar).
- Crop, rotate, flip.
- Step-back / step-forward (full history stack).
- Color palette + custom hex.
- Stroke width picker (1–24 px).
- "Magic-arrow" — auto-snaps endpoints to nearest edge/corner of a detected UI element.

Editor shortcuts (defaults):

| Key | Tool |
|---|---|
| V | Select / move |
| A | Arrow |
| R | Rectangle |
| O | Ellipse |
| L | Line |
| P | Pen |
| H | Highlighter |
| T | Text |
| N | Numbered counter |
| B | Blur |
| X | Redact |
| C | Crop |
| ⌘Z / ⌘⇧Z | Undo / Redo |
| ⌘C | Copy to clipboard |
| ⌘S | Save to default folder |
| ⌘⇧S | Save As… |
| ⌘W | Close editor |
| Space (hold) | Pan |
| ⌘ + / ⌘ − | Zoom |

### 2.4 Beautify mode

A panel in the editor that wraps the screenshot in:

- Padding (0 – 200 px).
- Background: solid color, gradient presets, image, transparent, or "auto-pick from screenshot" (samples dominant edge colors).
- Device frames: macOS window chrome, iPhone, iPad, browser address-bar.
- Rounded corners + shadow.
- Aspect-ratio presets: 1:1, 4:5, 16:9, 9:16, Twitter, LinkedIn.
- Export with one click; preset can be set as default for all future captures.

### 2.5 Post-capture overlay (the "Quick Tray")

After any capture, a small floating thumbnail tray appears bottom-right (configurable corner). Actions on hover or via keys:

- ↵ — open editor
- ⌘C — copy to clipboard (default action)
- ⌘S — save to disk
- D — drag the image out to any app (real native drag).
- P — pin to screen (always-on-top floating image; great for referencing while typing).
- ⌫ — discard.
- Auto-dismiss after configurable duration (default 6 s).

### 2.6 Pin-to-screen

Floating, frameless window with the captured image. Always on top by default, opacity adjustable with scroll-on-hover, zoom with ⌘+/−, draggable, dismissible with ⌫ or click-through-toggle. Multiple pins supported.

### 2.7 History library

- Every capture is written to `~/Pictures/ScreenshotUltra/` (configurable).
- Filename pattern is templated: `{date}_{time}_{mode}_{seq}.{ext}` with full token list documented.
- An optional in-app History window (⌘⇧H) shows a grid of thumbnails with filter chips (image / video / gif / pinned / starred), free-text search across OCR results, and tag editing.
- Bounded retention: keep last N days or last N captures (configurable; default off — keep forever).

### 2.8 OCR (local)

- Apple Vision (`VNRecognizeTextRequest`) — runs locally, no network.
- Editor button "Copy text" extracts text from the current capture.
- Right-click any history thumbnail → "Copy text".
- Searchable in history.

### 2.9 Measurement & color tools

- Crosshair-mode pixel ruler: shows X/Y, distance, RGB/HEX at cursor.
- Color picker eyedropper (hotkey ⌃⌥⌘P) — copies hex to clipboard.
- Spacing guide: alt-hover a rectangle to measure distance to nearby edges (Shottr-style).

### 2.10 Output destinations

A capture can be routed to one or more sinks in parallel. Configurable per-mode and per-hotkey:

- Clipboard (always available).
- Disk (default folder or per-rule folder).
- Editor (open immediately).
- Quick Tray (default).
- Run a shell command with the file path as `$1` (power-user "share to anywhere" — Slack CLI, scp, etc.).
- Optional: write to NDJSON log, mirroring MailBox Ultra's logging convention.

> **No built-in cloud uploader in v1.** The shell-command sink lets users wire up `s3 cp`, `rclone`, or their own uploader without us shipping cloud SKUs.

### 2.11 Sharing aids (no cloud)

- Native drag-out from Quick Tray and History.
- "Copy as Markdown image link" with the local file URL.
- "Copy as base64 data URL" — handy for emails / docs.

---

## 3. Hotkeys — the centerpiece

### 3.1 Defaults

Chosen to (a) not clash with macOS built-ins and (b) be muscle-memory-friendly:

| Action | Default |
|---|---|
| Region capture | ⌃⌥⌘1 |
| Window capture | ⌃⌥⌘2 |
| Fullscreen capture | ⌃⌥⌘3 |
| Scrolling capture | ⌃⌥⌘4 |
| Repeat last capture | ⌃⌥⌘R |
| Start / stop video recording | ⌃⌥⌘V |
| Start / stop GIF recording | ⌃⌥⌘G |
| Open clipboard image in editor | ⌃⌥⌘E |
| Open history | ⌃⌥⌘H |
| Color picker | ⌃⌥⌘P |
| Pin last capture | ⌃⌥⌘. |
| Quick Tray show/hide | ⌃⌥⌘\\ |

(macOS's own ⌘⇧3/4/5 are left alone unless the user explicitly opts in to "replace system shortcuts".)

### 3.2 Rebinding UX

- Preferences → Hotkeys.
- Each row shows current binding, a "Record" button, a "Clear" button, and a conflict warning if it collides with a system or another-app shortcut we can detect.
- Modifier-only bindings allowed (e.g. double-tap ⌃ for region). Detected via custom event tap.
- Per-app exclusions: a list where Screenshot Ultra hotkeys pass through to the focused app (e.g. while gaming, while in another capture tool).
- Import / export `hotkeys.toml` so a team can share a config.

### 3.3 Implementation notes

- Use `tao` or a custom Carbon `RegisterEventHotKey` wrapper for global hotkeys (the `global-hotkey` crate covers most cases).
- Fall back to a CGEventTap for modifier-only / chord-style bindings.
- Accessibility permission is required for some chord bindings; we detect missing permission and walk the user through granting it in System Settings with a one-click deeplink.

---

## 4. Architecture

### 4.1 Stack

- **Language:** Rust (stable, pinned via `rust-toolchain.toml`).
- **UI:** `egui` + `eframe` for Preferences, Editor, History windows — matches MailBox/Postbin Ultra.
- **Capture overlay & pin windows:** native `NSWindow` via `objc2` / `cocoa-foundation`, hosting either a Metal layer (for the live magnifier) or a transparent egui surface for the marquee UI. Editor stays pure egui.
- **Screen capture:** ScreenCaptureKit (macOS 13+) via `screencapturekit-rs` for both still and video.
- **Window enumeration:** `CGWindowListCopyWindowInfo`.
- **Video encoding:** ScreenCaptureKit → CMSampleBuffer → AVAssetWriter (H.264 / HEVC). Wrapped in a thin Swift bridge if needed (`swift-bridge`), or pure Obj-C FFI.
- **GIF encoding:** `gifski` via its Rust API for high-quality dithered output.
- **OCR:** Apple Vision via Obj-C FFI.
- **Hotkeys:** `global-hotkey` crate + custom event-tap for modifier-only bindings.
- **Clipboard:** `arboard` (image + text).
- **Storage:** plain files on disk; metadata sidecar in `.screenshot-ultra/index.ndjson` (one JSON line per capture). No sqlite in v1 — NDJSON keeps parity with MailBox Ultra's logging story and is dead simple to grep/replay.
- **Config:** `~/Library/Application Support/ScreenshotUltra/settings.toml`.
- **Logging:** `tracing` → NDJSON file at `~/Library/Logs/ScreenshotUltra/log.ndjson`.

### 4.2 Process model

Single process, three "surfaces":

1. **Menu-bar agent** (always running, LSUIElement = true). Hosts the hotkey listener and capture pipeline. No dock icon by default; toggleable.
2. **Overlay windows** (transient, created per capture). Borderless, transparent, joinsAllSpaces, level = .screenSaver.
3. **App windows** (Editor, History, Preferences). Standard NSWindow, dock icon appears while at least one is open then disappears again (configurable to "always show dock icon").

### 4.3 Capture pipeline (region example)

```
Hotkey ──▶ CaptureCoordinator
            │
            ├─▶ Pre-capture: pause display sleep, hide cursor, freeze screen if "freeze" enabled
            ├─▶ Spawn OverlayWindow (per display)
            │     ├─ Crosshair + magnifier (Metal)
            │     └─ Marquee + keyboard handling (egui)
            ├─▶ User commits rect ──▶ ScreenCaptureKit single-frame at rect
            ├─▶ Image buffer ──▶ post-processing (drop shadow, scaling)
            └─▶ Sink fan-out: Clipboard | Disk | Editor | QuickTray | shell
```

Every step is instrumented; total span emitted as one NDJSON line per capture.

### 4.4 Performance budgets

| Stage | Budget (warm) |
|---|---|
| Hotkey → overlay visible | ≤ 16 ms |
| Marquee commit → image in RAM | ≤ 30 ms |
| Image → clipboard | ≤ 20 ms |
| Image → disk (PNG, 4K) | ≤ 80 ms async, not on hotpath |
| Hotkey → editor window visible | ≤ 120 ms |

We measure these in `make bench` and gate CI on regressions > 20 %.

---

## 5. Cross-platform stance

- **v1.0:** macOS 13+ only. Universal binary (arm64 + x86_64). ~12 MB target.
- **v1.x:** Linux (X11 + Wayland) — `xcap` + `wgpu` overlay. Different hotkey backend.
- **v2.0:** Windows — Windows.Graphics.Capture + Win32 hotkeys.

The codebase is organized so the capture/overlay layer sits behind a `trait Capture` and a `trait HotkeyHost`, with platform impls under `src/platform/{macos,linux,windows}`. Everything else (editor, history, settings, sinks) is platform-agnostic.

---

## 6. Repository layout

Mirrors MailBox Ultra / Postbin Ultra one-for-one so anyone returning from those repos feels at home:

```
ScreenshotUltra/
├─ Cargo.toml
├─ Cargo.lock
├─ rust-toolchain.toml
├─ Makefile
├─ README.md
├─ LICENSE              # MIT
├─ CHANGELOG.md
├─ .gitignore
│
├─ .github/
│  └─ workflows/
│     ├─ ci.yml           # fmt, clippy, test, build-mac
│     ├─ release.yml      # tag → dmg + tar.gz, attach to GH release
│     └─ pages.yml        # build & deploy site/
│
├─ src/
│  ├─ main.rs
│  ├─ app.rs              # eframe App + window routing
│  ├─ menubar.rs
│  ├─ hotkeys/
│  ├─ capture/
│  │  ├─ region.rs
│  │  ├─ window.rs
│  │  ├─ fullscreen.rs
│  │  ├─ scrolling.rs
│  │  └─ video.rs
│  ├─ editor/             # egui annotation surface
│  ├─ beautify/
│  ├─ history/
│  ├─ ocr/
│  ├─ sinks/              # clipboard, disk, shell, editor, tray
│  ├─ settings/
│  ├─ logging.rs
│  └─ platform/
│     └─ macos/
│        ├─ overlay.rs
│        ├─ screencapturekit.rs
│        ├─ vision_ocr.rs
│        ├─ avassetwriter.rs
│        └─ event_tap.rs
│
├─ mac/                   # Swift / Obj-C bridge code, Info.plist, entitlements
├─ icon/                  # icon.svg + generated .icns
├─ assets/                # marketing screenshots, demo gifs
├─ scripts/
│  ├─ install.sh
│  ├─ build-dmg.sh
│  ├─ codesign.sh
│  ├─ notarize.sh
│  └─ bench.sh
├─ tools/                 # dev utilities (e.g. fake-screen generator for tests)
├─ tests/
│  ├─ unit/
│  ├─ integration/        # full pipeline tests with synthetic displays
│  └─ snapshots/          # golden PNGs for editor output
└─ site/                  # GitHub Pages source (see §8)
```

`Makefile` provides the same verb set as the sister apps:

```
make run            # cargo run
make app            # build .app bundle
make dmg            # build signed/notarized .dmg
make check          # fmt + clippy + test
make bench          # perf budget benchmarks
make site           # build static docs locally
```

---

## 7. Installation & distribution

### 7.1 One-liner install

Matches the MailBox / Postbin Ultra pattern verbatim:

```sh
curl -sSL https://raw.githubusercontent.com/MPJHorner/ScreenshotUltra/main/scripts/install.sh | bash
```

`install.sh` will:

1. Detect arm64 vs x86_64.
2. Read latest release tag from the GitHub API.
3. Download the matching `.dmg`.
4. `hdiutil attach`, copy `.app` to `/Applications`, detach.
5. `xattr -dr com.apple.quarantine`.
6. Launch the app once so it can prompt for Screen Recording + Accessibility permissions.
7. Print a "next steps" block with default hotkeys.

### 7.2 Alternative methods (documented)

- Manual `.dmg` download from Releases.
- `brew install --cask screenshot-ultra` (we publish a tap at `MPJHorner/homebrew-ultra`).
- `cargo install --git …` for source builds.
- "Build from source" with `make app`.

### 7.3 Code signing & notarization

- Developer ID signing in `scripts/codesign.sh` using a cert read from `KEYCHAIN_PROFILE`.
- Notarization via `notarytool` in `scripts/notarize.sh`.
- CI release workflow signs/notarizes only when running in the main repo (so forks can still build unsigned).

### 7.4 First-run experience

- App opens a small onboarding window walking through: grant **Screen Recording** permission, grant **Accessibility** permission (for chord hotkeys + window-detection-during-overlay), pick default save folder, pick default capture-after action (Clipboard / Editor / Tray).
- Skippable in 5 seconds.

---

## 8. Documentation site (`site/`)

Mirrors MailBox Ultra / Postbin Ultra structurally — same navigation order, same tone, same dark-on-white minimalism with the screenshot front-and-center in the hero.

### 8.1 Tech

- Static site, built with **mkdocs-material** (matches the sister sites' "searchable, mobile-friendly, always in sync with the code" promise). Source markdown in `site/docs/`.
- Deployed via `.github/workflows/pages.yml` to `gh-pages` branch.
- A 100 % static landing page at `site/index.md` with the marketing copy; everything else under `site/docs/`.

### 8.2 Information architecture

Top nav: `Install · Quick Start · Hotkeys · Capture · Record · Annotate · Logging · Changelog · GitHub`

Pages:

1. **Home** — hero, value prop, 30-second tour, feature grid, use cases, install one-liner, footer.
2. **Install** — one-liner, manual `.dmg`, Homebrew, build-from-source, permissions.
3. **Quick Start** — "press the hotkey, drag a rectangle, you're done" + GIF.
4. **Hotkeys** — full default table, rebinding walkthrough, chord & modifier-only bindings, per-app exclusions, `hotkeys.toml` reference.
5. **Capture modes** — region / window / fullscreen / scrolling / timed, with one short GIF per mode.
6. **Recording** — video + GIF, audio sources, mouse highlight, keystroke overlay, trim.
7. **Annotate** — every tool, every shortcut, beautify panel.
8. **History & OCR** — library window, retention, search, copy-text.
9. **Sinks & shell** — clipboard / disk / editor / tray / shell command, examples (Slack, scp, s3).
10. **Configuration** — `settings.toml` reference.
11. **Logging** — NDJSON format, every event type, examples.
12. **Use cases** — mirroring the sister apps' style: bug reports, design QA, async standups, code reviews, customer support, social posts, docs screenshots.
13. **Comparison** — honest table vs CleanShot X, Shottr, Xnapper, macOS built-in.
14. **Contributing** — local dev, code style, PR rules.
15. **Changelog** — generated from `CHANGELOG.md`; RSS feed.

### 8.3 Voice

Direct, technical, confident. "Press the key. Drag the box. It's on your clipboard." Same posture as "no smtp config gymnastics."

### 8.4 Hero copy (draft)

> **Capture every pixel. Annotate like a native.**
> A snappy macOS screenshot and screen recorder driven by hotkeys you control. Region, window, scrolling page, video, GIF — all local, all yours.

Buttons: `Download .dmg` · `View on GitHub`. Below: the install one-liner in a copy-on-click block. Below that: a single 8 s looping demo video.

---

## 9. Settings (`settings.toml`)

Sample (full schema documented in `site/docs/configuration.md`):

```toml
[general]
save_folder            = "~/Pictures/ScreenshotUltra"
filename_template      = "{date}_{time}_{mode}_{seq}"
default_image_format   = "png"          # png | jpg | webp | heic
default_video_format   = "mp4"          # mp4 | mov
show_in_dock           = false
launch_at_login        = true
play_shutter_sound     = true
copy_on_capture        = true
open_editor_on_capture = false
quick_tray_enabled     = true
quick_tray_timeout_ms  = 6000

[overlay]
freeze_screen          = true
show_magnifier         = true
show_pixel_grid        = true
show_color_picker      = true

[recording]
fps                    = 60
record_system_audio    = false
record_microphone      = false
show_mouse_highlight   = true
show_clicks            = true
keystroke_overlay      = false

[ocr]
enabled                = true
languages              = ["en-US"]

[hotkeys]
region                 = "ctrl+alt+cmd+1"
window                 = "ctrl+alt+cmd+2"
fullscreen             = "ctrl+alt+cmd+3"
scrolling              = "ctrl+alt+cmd+4"
repeat_last            = "ctrl+alt+cmd+r"
record_video           = "ctrl+alt+cmd+v"
record_gif             = "ctrl+alt+cmd+g"
open_clipboard_image   = "ctrl+alt+cmd+e"
open_history           = "ctrl+alt+cmd+h"
color_picker           = "ctrl+alt+cmd+p"
pin_last               = "ctrl+alt+cmd+."
toggle_tray            = "ctrl+alt+cmd+\\"

[sinks]
clipboard              = true
disk                   = true
editor                 = false
tray                   = true
shell                  = ""              # e.g. "scp $1 user@host:/var/www/img/"

[per_app_exclusions]
bundle_ids = ["com.apple.dt.Xcode", "com.steampowered.steam"]
```

---

## 10. Logging (NDJSON)

One line per event, parity with MailBox Ultra. Sampled:

```json
{"ts":"2026-05-11T14:25:01.327Z","evt":"capture","mode":"region","display":1,"rect":[100,200,1280,720],"bytes":284113,"fmt":"png","saved_to":"~/Pictures/ScreenshotUltra/20260511_142501_region_001.png","sinks":["clipboard","disk","tray"],"duration_ms":47}
{"ts":"2026-05-11T14:25:08.901Z","evt":"hotkey","binding":"ctrl+alt+cmd+v","action":"record_video.start"}
{"ts":"2026-05-11T14:25:33.014Z","evt":"recording","mode":"video","duration_s":24.1,"bytes":4218771,"fmt":"mp4","saved_to":"…"}
{"ts":"2026-05-11T14:25:40.000Z","evt":"ocr","capture_id":"…","chars":482,"lang":"en-US","duration_ms":118}
```

Used for: history index, debugging, and (optionally) piping to `jq`/`fzf` workflows.

---

## 11. Security & privacy

- App sandbox: **disabled** (sandboxing limits ScreenCaptureKit reach). We document this honestly.
- Hardened runtime: **enabled** with the minimum entitlements (`com.apple.security.device.screen-capture`, `…audio-input`, `…device.microphone` only if mic is enabled in settings).
- No network code links into the binary by default. The optional shell-sink is gated behind a Preferences toggle and a confirmation dialog the first time it runs.
- No telemetry, ever. We do not phone home for update checks unless the user enables it; default is **off**, with a manual "Check for Updates" menu item.
- Crash reports stay on disk under `~/Library/Logs/ScreenshotUltra/crashes/`. No auto-send.

---

## 12. Testing strategy

- **Unit:** pure-Rust modules (settings parser, filename templater, NDJSON serializer, hotkey conflict detector, scrolling stitcher).
- **Snapshot:** annotator output PNGs compared to goldens in `tests/snapshots/`.
- **Integration:** synthetic display server (`tools/fake-screen`) feeds a known image into the capture pipeline; we assert the round-trip bytes.
- **Manual checklist:** documented in `tests/MANUAL.md` for each release — every hotkey, every mode, every sink, on a clean macOS VM.
- **Perf gates:** `make bench` emits a CSV; CI fails if any budget regresses > 20 %.

---

## 13. Roadmap by milestone

### M1 — "Press the key" (week 1–2)  ✅ shipped (v0.1.0)

- [x] Repo skeleton, CI, install.sh.  *(signing pipeline deferred to M6)*
- [x] Menu-bar agent + global hotkey listener.
- [x] Region + fullscreen capture.  *(macOS `screencapture` backend in M1; native ScreenCaptureKit lands in M3 with video)*
- [x] Clipboard + disk sinks.
- [ ] Quick Tray (open-editor, copy, save, drag-out).  *Moved to M2 — it depends on the editor.*
- [ ] Basic Preferences window (general + hotkeys tabs).  *Moved to M2 — `settings.toml` covers M1.*

**Demo-able:** ⌃⌥⌘1 → marquee → image on clipboard, file on disk. ✅

See [`docs/milestones/M1-press-the-key.md`](docs/milestones/M1-press-the-key.md).

### M2 — "Annotate" (week 3–4)

- Editor window with arrow / rect / ellipse / pen / highlighter / text / blur / crop / undo-redo / counter.
- Editor shortcuts.
- Window-capture mode with shadow synthesis.
- Pin-to-screen.

### M3 — "Record" (week 5–6)

- Video recording (mp4) via ScreenCaptureKit + AVAssetWriter.
- GIF recording via gifski.
- System audio + mic.
- Mouse highlight + click flash.
- Post-stop trim UI.

### M4 — "Scroll & Beautify" (week 7–8)

- Scrolling capture with live stitching preview.
- Beautify panel (padding / gradient / frames / aspect presets).
- History window + NDJSON index.

### M5 — "OCR & polish" (week 9)

- Apple Vision OCR + searchable history.
- Color picker, ruler, spacing guides.
- Per-app hotkey exclusions.

### M6 — "Ship" (week 10)

- Docs site complete.
- Signed + notarized DMG.
- Homebrew cask.
- v1.0 release on GitHub.

**Post-1.0:** Linux port (M7), Windows (M8), optional self-hosted upload sink (M9).

---

## 14. Open questions — resolved for M1

1. **GitHub org/repo name:** `MPJHorner/ScreenshotUltra`. ✅
2. **macOS minimum:** macOS 13 Ventura. `Info.plist` declares `LSMinimumSystemVersion = 13.0`. ✅
3. **License:** MIT (matches sister projects). ✅
4. **Docs site engine:** mkdocs-material, decided at M6 when the site is built — will read `site/` of MailBox/Postbin Ultra to match exactly.
5. **Brand colour:** still pending; tentative `#FF3D54` (camera-shutter red). Lock in before M6.
6. **"Ultra" naming convention:** confirmed — `ScreenshotUltra` (one-word repo) / `Screenshot Ultra` (two-word product name). ✅

---

## 15. Risks

| Risk | Mitigation |
|---|---|
| ScreenCaptureKit Rust bindings are immature | Be ready to drop a thin Swift bridge in `mac/` if `screencapturekit-rs` is missing a piece. Plan budgets a week for this. |
| Modifier-only hotkeys via CGEventTap need Accessibility permission | Detect missing permission, show one-click deeplink to System Settings, document clearly. Plain `RegisterEventHotKey` bindings work without it as a fallback. |
| Notarization flake | Pin `notarytool` workflow, retry on transient errors, document manual fallback. |
| Scrolling-capture stitching artifacts on dynamic pages | Ship with conservative defaults; document known-bad cases (Twitter timeline, infinite scrollers); add per-app heuristics list users can edit. |
| egui isn't ideal for image-heavy editing | Profile early; if drag latency is bad we move the canvas to a Metal layer behind egui chrome. |

---

## 16. Success criteria for v1.0

- Hotkey → clipboard image in < 100 ms on an M-series Mac, measured.
- Every advertised capture mode works on macOS 13 / 14 / 15 + latest.
- Zero network calls observable with Little Snitch in default config.
- DMG passes notarization on a clean GH-hosted runner.
- Docs site renders, search works, every hotkey is documented.
- A new user can go from `curl | bash` to first annotated screenshot in under 60 seconds without reading docs.
