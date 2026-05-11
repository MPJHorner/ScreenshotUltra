# M3 — Record

**Status:** ⏳ planned

**Goal:** Screen recording (video + GIF) with the same hotkey-first ergonomics
as still capture.

## Scope (from plan.md §13)

- [ ] Video recording via ScreenCaptureKit → AVAssetWriter (H.264 / HEVC, mp4/mov).
- [ ] 30 / 60 fps configurable.
- [ ] System audio capture (ScreenCaptureKit, no extension on macOS 13+).
- [ ] Microphone capture with input device picker + live level meter.
- [ ] Mouse highlight ring + click flash, toggleable.
- [ ] Keystroke overlay, toggleable, with allow/deny list per app.
- [ ] GIF recording via `gifski` (same UX as video, output is `.gif`).
- [ ] Post-stop trim UI: drag handles on a timeline, no re-encode on keyframe cuts.
- [ ] Native ScreenCaptureKit backend replaces the `screencapture` shell-out
      for still capture too — single pipeline for stills + video.

## Risks

- `screencapturekit-rs` may be missing pieces; budget a week for a thin Swift
  bridge (`mac/` + `swift-bridge` or pure Obj-C FFI). See plan.md §15.
- AVAssetWriter pipeline must not drop frames at 60 fps on Retina displays.
