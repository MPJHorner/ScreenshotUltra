# M1 — Press the key

**Status:** ✅ shipped (v0.1.0)

**Goal:** Hotkey → marquee → image on clipboard, file on disk. The smallest
possible loop that proves the capture pipeline end-to-end.

## Scope (from plan.md §13)

- [x] Repo skeleton (Cargo workspace, Makefile, layout matching MailBox/Postbin Ultra).
- [x] CI workflow (fmt + clippy + test + release build) on macOS 14.
- [x] `install.sh` one-liner (release-download path with source-build fallback).
- [x] Menu-bar agent (`LSUIElement = true`, no dock icon) via `tray-icon`.
- [x] Global hotkey listener via `global-hotkey`.
- [x] Region capture (interactive marquee, Esc cancels).
- [x] Fullscreen capture (main display, single file).
- [x] Clipboard sink (image lands on the macOS clipboard).
- [x] Disk sink with templated filenames (`{date}_{time}_{mode}_{seq}`).
- [x] NDJSON event log at `~/Library/Logs/ScreenshotUltra/log.ndjson`.
- [x] `settings.toml` auto-created on first run, with rebindable hotkeys.
- [x] `make app` → unsigned `Screenshot Ultra.app` bundle in `dist/`.

## Deferred to later milestones

- Quick Tray overlay (open-editor / copy / save / drag-out / pin) → **M2**
  ships it alongside the editor since the tray's primary purpose is opening
  the editor.
- Preferences window UI (currently `settings.toml` only) → **M2**.
- Native ScreenCaptureKit backend → planned around **M3** when video
  recording forces us off the `screencapture` shell-out anyway.

## Demo

```sh
make app
open "dist/Screenshot Ultra.app"
# grant Screen Recording permission, relaunch
# press ⌃⌥⌘1 — drag a region — image is on your clipboard and saved to
# ~/Pictures/ScreenshotUltra/
```

## Architecture notes

- **Backend:** `/usr/sbin/screencapture` is invoked per capture. It's bundled
  with macOS, fast, and battle-tested. We document the swap-out plan in
  `plan.md §4.1`. The `capture::run(mode, settings)` boundary is stable; only
  the body changes when ScreenCaptureKit lands.
- **Clipboard:** `osascript`'s `set the clipboard to … as JPEG picture` does
  the right thing for PNG and JPG files. `arboard` only takes raw RGBA, which
  would require an image decode we don't need yet.
- **Event loop:** `tao` event loop carries both `GlobalHotKeyEvent` and
  `tray-icon::MenuEvent` user events via `EventLoopProxy`. One process, one
  loop, no extra threads.

## Verification

- `cargo test` — 2 passing unit tests (filename templating, tilde expansion).
- `cargo clippy -- -D warnings` — clean.
- `make app` — builds `dist/Screenshot Ultra.app` in ~15 s warm.
- Manual smoke test: launch app, ⌃⌥⌘1, drag, paste into Preview → image
  appears; file exists in `~/Pictures/ScreenshotUltra/`; NDJSON log appended.
