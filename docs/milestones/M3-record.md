# M3 — Record

**Status:** 🚧 in progress (v0.6.0 ships v0)

**Goal:** Screen recording (video + GIF) with the same hotkey-first ergonomics
as still capture.

## Scope (from plan.md §13)

- [~] Video recording — **v0 shipped via `screencapture -v` + SIGINT**
      (writes `.mov`). Native ScreenCaptureKit → AVAssetWriter pipeline
      replaces it in a follow-up.
- [ ] 30 / 60 fps configurable. *(screencapture -v doesn't expose this;
      ScreenCaptureKit backend will.)*
- [ ] System audio capture. *(macOS-26 ScreenCaptureKit supports it
      natively; will land with the SCK backend.)*
- [~] Microphone capture — `[recording].record_microphone = true`
      passes `-g` to `screencapture`. Device-picker UI deferred.
- [x] **Mouse-click highlight** — `[recording].show_clicks = true`
      (default) passes `-k` to `screencapture`.
- [x] **Keystroke overlay**, toggleable via
      `[recording].keystroke_overlay = true`. Shows recently-typed
      keys in a rounded pill at the bottom-centre of the screen during
      recording. NSEvent global monitor + custom NSView. Requires
      Accessibility permission. *(Per-app allow/deny list deferred —
      v0 shows every key.)*
- [x] **GIF recording** — capture as `.mov` then post-process via
      `ffmpeg` (palette + paletteuse for high-quality small GIFs). If
      `ffmpeg` isn't on PATH we keep the `.mov` and log it. A future
      pass will swap in `gifski` for bundled, no-dependency GIFs.
- [ ] Post-stop trim UI: drag handles on a timeline.
- [ ] Native ScreenCaptureKit backend replaces the `screencapture`
      shell-out for still capture too — single pipeline for stills +
      video.

## Risks

- `screencapturekit-rs` may be missing pieces; budget a week for a thin Swift
  bridge (`mac/` + `swift-bridge` or pure Obj-C FFI). See plan.md §15.
- AVAssetWriter pipeline must not drop frames at 60 fps on Retina displays.
