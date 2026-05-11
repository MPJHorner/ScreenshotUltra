# M2 — Annotate

**Status:** 🚧 in progress

**Goal:** A native annotation editor that opens immediately after every
capture (configurable), plus the Quick Tray, Pin-to-screen, and the Window
capture mode.

## Scope (from plan.md §13)

- [~] Editor window — native `NSWindow` + custom `CanvasView` via `objc2`.
      **Tools shipped: Pen / Line / Arrow / Rect / Ellipse / Highlighter /
      Redact** (P / L / A / R / E / H / X shortcuts). Five-colour palette
      (Red / Yellow / Green / Blue / Black) and three-step width picker
      (Thin / Med / Thick, shortcuts 1/2/3). ⌘S save, ⌘C copy, ⌘Z undo,
      ⌘⇧Z redo, ⌘W close, Clear button.  *Still to add: Text / Counter /
      Blur / Crop.*
- [ ] Color palette + custom hex; stroke width picker (1–24 px).
      *(Single red colour for now.)*
- [~] History stack — undo only; redo is the next obvious add.
- [x] Window capture mode — `screencapture -W -o` (interactive selection
      via the macOS window-capture cursor, drop shadow trimmed). Default
      hotkey `⌃⌥⌘2`. *(Native `CGWindowListCopyWindowInfo`-driven hover
      highlight will replace the shell-out alongside ScreenCaptureKit in M3.)*
- [x] Pin-to-screen (basic): floating, always-on-top `NSWindow` with title
      bar, sized to the capture aspect ratio. Hotkey `⌃⌥⌘.` and Quick Tray
      "Pin" button. Multiple pins cascade. *(Frameless mode, opacity
      scroll, and `⌘+/⌘-` zoom land in a follow-up.)*
- [x] Quick Tray (post-capture floating panel, bottom-right): Copy / Edit /
      Folder / Reveal / Pin / Discard buttons, auto-dismiss after
      `quick_tray_timeout_ms`. Native `NSWindow` via `objc2`.
- [x] "Edit in Preview" tray button — opens the capture in macOS Preview
      for annotation. *(A native editor lands later in M2.)*
- [x] Separate "silent" capture flow (`silent_region` / `silent_fullscreen` /
      `silent_window` hotkey slots) so users can choose tray-or-no-tray per
      hotkey.
- [ ] Basic Preferences window (general + hotkeys tabs) — at this point we
      have enough surface area to justify a GUI for the bits in settings.toml.

## Open questions for M2

- Does the editor canvas need a Metal layer behind egui chrome for drag
  latency on 6K displays? Profile early (plan.md §15 risk).
- "Magic-arrow" snap-to-UI-element heuristic — leave for M5 polish?

## Notes on the Quick Tray shipped today

- Buttons are `Copy` / `Folder` (open the save folder) / `Reveal`
  (Finder reveal) / `Discard` (delete the file). The plan's `Pin` and
  `Drag-out` actions land later in M2 once the editor and Pin-to-screen
  exist.
- Implementation lives in `src/quick_tray.rs`. The panel is borderless,
  non-activating, status-window level, positioned bottom-right of the main
  screen; corner radius via `CALayer`.
- The "silent" hotkeys (`silent_region`, `silent_fullscreen`) are unbound
  by default — set them in `~/Library/Application Support/ScreenshotUltra/settings.toml`
  to enable both flows side-by-side.
