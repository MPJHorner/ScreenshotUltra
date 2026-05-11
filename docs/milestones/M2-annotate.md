# M2 — Annotate

**Status:** ⏳ planned

**Goal:** A native annotation editor that opens immediately after every
capture (configurable), plus the Quick Tray, Pin-to-screen, and the Window
capture mode.

## Scope (from plan.md §13)

- [ ] Editor window (egui) with these tools and their shortcuts:
      arrow (A), rectangle (R), ellipse (O), line (L), pen (P),
      highlighter (H), text (T), numbered counter (N), blur (B), redact (X),
      crop (C), undo/redo (⌘Z / ⌘⇧Z), copy (⌘C), save (⌘S), save as (⌘⇧S).
- [ ] Color palette + custom hex; stroke width picker (1–24 px).
- [ ] History stack (full undo/redo).
- [ ] Window capture mode — hover to highlight, click to capture, synthesised
      drop shadow toggle.
- [ ] Pin-to-screen: floating, frameless, always-on-top, opacity scroll,
      ⌘+/− zoom, ⌫ to dismiss; multiple pins supported.
- [ ] Quick Tray (post-capture floating thumbnail, bottom-right, configurable
      corner): ↵ open editor, ⌘C copy, ⌘S save, D drag-out, P pin,
      ⌫ discard, auto-dismiss after `quick_tray_timeout_ms`.
- [ ] Basic Preferences window (general + hotkeys tabs) — at this point we
      have enough surface area to justify a GUI for the bits in settings.toml.

## Open questions for M2

- Does the editor canvas need a Metal layer behind egui chrome for drag
  latency on 6K displays? Profile early (plan.md §15 risk).
- "Magic-arrow" snap-to-UI-element heuristic — leave for M5 polish?
