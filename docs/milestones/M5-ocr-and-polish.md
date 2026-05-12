# M5 — OCR & polish

**Status:** 🚧 in progress (v0.7.0 ships v0 OCR)

**Goal:** Local OCR via Apple Vision, measurement and color tools, and the
per-app hotkey exclusions UI.

## Scope (from plan.md §13)

- [x] **Apple Vision OCR** (`VNRecognizeTextRequest` via `objc2-vision`)
      — entirely on-device, no network calls.
- [x] **"Text" button on the Quick Tray** — every recognised string
      lands on the clipboard, notification banner shows the character
      count. Wired through `src/ocr.rs::extract_text(path)`.
- [ ] Editor "Copy text" button — same `extract_text` but invoked from
      inside the annotation editor.
- [ ] Right-click history thumbnail → "Copy text".
- [ ] Searchable OCR results in the History window.
- [ ] Crosshair pixel ruler: shows X/Y, distance, RGB/HEX at cursor.
- [ ] Color picker eyedropper (`⌃⌥⌘P`) — copies hex to clipboard.
- [ ] Spacing guide: alt-hover a rectangle to measure distance to nearby
      edges (Shottr-style).
- [ ] Per-app hotkey exclusions: list in Preferences; hotkeys pass through to
      excluded apps' focused windows.
- [ ] Modifier-only / chord hotkeys via custom CGEventTap (Accessibility
      permission with one-click deeplink to System Settings if missing).
