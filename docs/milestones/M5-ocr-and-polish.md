# M5 — OCR & polish

**Status:** ⏳ planned

**Goal:** Local OCR via Apple Vision, measurement and color tools, and the
per-app hotkey exclusions UI.

## Scope (from plan.md §13)

- [ ] Apple Vision OCR (`VNRecognizeTextRequest`) via Obj-C FFI — local only.
- [ ] Editor "Copy text" button.
- [ ] Right-click any history thumbnail → "Copy text".
- [ ] Searchable OCR results in History.
- [ ] Crosshair pixel ruler: shows X/Y, distance, RGB/HEX at cursor.
- [ ] Color picker eyedropper (`⌃⌥⌘P`) — copies hex to clipboard.
- [ ] Spacing guide: alt-hover a rectangle to measure distance to nearby
      edges (Shottr-style).
- [ ] Per-app hotkey exclusions: list in Preferences; hotkeys pass through to
      excluded apps' focused windows.
- [ ] Modifier-only / chord hotkeys via custom CGEventTap (Accessibility
      permission with one-click deeplink to System Settings if missing).
