# M4 — Scroll & Beautify

**Status:** ⏳ planned

**Goal:** Scrolling capture with live stitching preview, the Beautify panel
for social-ready exports, and the History library.

## Scope (from plan.md §13)

- [ ] Scrolling capture: start at a region, scroll with trackpad/wheel, app
      stitches frames in real-time. Live preview panel during capture.
- [ ] Beautify panel in the editor:
  - [ ] Padding (0–200 px).
  - [ ] Background: solid color, gradient presets, image, transparent, or
        auto-pick from screenshot.
  - [ ] Device frames: macOS chrome, iPhone, iPad, browser address-bar.
  - [ ] Rounded corners + drop shadow.
  - [ ] Aspect-ratio presets: 1:1, 4:5, 16:9, 9:16, Twitter, LinkedIn.
  - [ ] One-click export; preset can be set as default for all future captures.
- [ ] History library window (⌃⌥⌘H):
  - [ ] Grid of thumbnails with filter chips (image / video / gif / pinned /
        starred).
  - [ ] Free-text search (placeholder for M5 OCR-backed search).
  - [ ] Tag editing.
  - [ ] Bounded retention: keep last N days or N captures.
- [ ] NDJSON history index at `.screenshot-ultra/index.ndjson` (one line per
      capture, parity with MailBox Ultra).

## Risks

- Scrolling stitcher on dynamic pages (Twitter, infinite scrollers): ship
  conservative defaults, document known-bad cases, per-app heuristics list
  users can edit (plan.md §15).
