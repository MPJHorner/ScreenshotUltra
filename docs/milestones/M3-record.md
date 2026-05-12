# M3 — Record

**Status:** 🚧 in progress (v0.6.0 ships v0)

**Goal:** Screen recording (video + GIF) with the same hotkey-first ergonomics
as still capture.

## Scope (from plan.md §13)

- [x] **Video recording — native ScreenCaptureKit backend** (v0.10.0).
      `mac/STURecorder.swift` uses `SCStream` + `AVAssetWriter` to write
      H.264 .mov at full Retina pixel dimensions. Bundled into the .app's
      `Contents/Resources/` at build time via `scripts/build-recorder.sh`.
      Auto-falls back to `screencapture -v` when the bundled binary
      isn't present.
- [x] **60 fps**. Default; user can override via the SCK recorder's
      `--fps N` arg (not yet plumbed into a setting — easy follow-up).
- [ ] System audio capture. *(Adding SCStream audio output is straightforward
      in the Swift recorder; the main blocker is that system-audio capture
      on unsigned binaries is unreliable on macOS 26. Will land when M6
      signing unlocks.)*
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
