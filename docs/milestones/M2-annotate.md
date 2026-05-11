# M2 — Annotate

**Status:** ✅ nearly complete (only Preferences GUI deferred)

**Goal:** A native annotation editor that opens immediately after every
capture (configurable), plus the Quick Tray, Pin-to-screen, and the Window
capture mode.

## Scope (from plan.md §13)

- [x] **Editor window** — native `NSWindow` + custom `CanvasView` via
      `objc2`. Eleven tools (`P` Pen, `L` Line, `A` Arrow, `R` Rect,
      `E` Ellipse, `H` Highlighter, `X` Redact, `N` Counter, `T` Text,
      `B` Blur, `C` Crop). Five-colour palette (Red / Yellow / Green /
      Blue / Black) and three-step width picker (Thin / Med / Thick;
      shortcuts `1` / `2` / `3`). Visual active-button indicators.
      Tooltips on every button.
      `⌘S` save, `⌘C` copy, `⌘Z` undo, `⌘⇧Z` redo, `⌘W` close,
      `Clear` button.
- [x] **Window capture mode** — `screencapture -W -o`. Default
      hotkey `⌃⌥⌘2`. *(Native `CGWindowListCopyWindowInfo`-driven hover
      highlight will replace the shell-out in M3 alongside
      ScreenCaptureKit.)*
- [x] **Pin-to-screen** — floating always-on-top `NSWindow` with
      title bar, sized to capture aspect ratio. Hotkey `⌃⌥⌘.` and
      Quick Tray "Pin" button. Multiple pins cascade.
      **Polish landed:** scroll-wheel opacity (0.3–1.0), `⌘+`/`⌘-`
      zoom in place, `⌫` / `Esc` to dismiss, `⌘0` resets alpha.
- [x] **Quick Tray** (post-capture floating panel, bottom-right):
      Copy / Edit / Folder / Reveal / Pin / Discard buttons; auto-dismiss
      after `quick_tray_timeout_ms`. Native `NSWindow` via `objc2`.
- [x] **Silent capture flow** — separate `silent_region` /
      `silent_window` / `silent_fullscreen` hotkey slots so users can
      pick tray-or-no-tray per hotkey.
- [ ] **Preferences GUI window** (general + hotkeys tabs). Currently
      `settings.toml` covers everything; the file is hot-reloaded within
      ~1 s of any save. A GUI on top is the only material M2 item still
      pending — deferred since the TOML path has proved adequate during
      M2 development.

## Beyond plan.md §13 — bonus M2 features

- **Repeat last capture** (`⌃⌥⌘R`) + **Open clipboard image** (`⌃⌥⌘E`)
  + **Eyedropper colour picker** (`⌃⌥⌘P`) — the latter shows macOS's
  `NSColorSampler` magnifier and copies the picked sRGB hex to the
  clipboard.
- **Timed fullscreen captures** (3 / 5 / 10 s) via `screencapture -T`.
- **Settings hot-reload** — `settings.toml` is watched on a background
  thread; changes apply within ~1 s without restarting the app.
  Invalid hotkeys keep the previous binding (with a log line) so you
  can't lock yourself out.
- **Shell-sink** — `[sinks].shell = "scp $1 ..."` runs an arbitrary
  command after every capture (detached).
- **Crash handler** — panics write a report to
  `~/Library/Logs/ScreenshotUltra/crashes/`.
- **CLI flags** — `--version`, `--help`, `--settings-path`,
  `--print-defaults`.
- **mkdocs-material docs site** at
  [mpjhorner.github.io/ScreenshotUltra](https://mpjhorner.github.io/ScreenshotUltra/).
- **Universal release pipeline** — tag `v*.*.*` → universal `.zip` + SHA-256.
- **App icon** — aperture-iris SVG (`icon/icon.svg`) rendered into a
  full `AppIcon.icns` via `scripts/render-icon.sh` + the matching
  aperture-shaped menu-bar template image in `src/tray.rs`.

## Open questions resolved

- "Custom colour picker" was rolled into the editor via the global
  eyedropper hotkey rather than an editor sub-tool — better UX and the
  picked hex goes onto the clipboard for use anywhere.
- "Edit in Preview" tray-button placeholder removed: **Edit** now opens
  the native editor.
- Quick Tray ships as a native `NSWindow` (was originally a borderless
  `NSPanel`; the panel variant didn't actually display under macOS 26
  + `LSUIElement`).
