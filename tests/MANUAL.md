# Manual test plan

Run this checklist before tagging a release. Tests that can't be automated
(real screen capture, real clipboard, real menu bar) live here.

## Setup

1. `make app` → quit any existing instance, then `open "dist/Screenshot Ultra.app"`.
2. First launch: grant **Screen Recording** in System Settings → Privacy & Security → Screen & System Audio Recording. Relaunch the app.
3. Confirm `~/Library/Application Support/ScreenshotUltra/settings.toml` was created.
4. Confirm a 📷 icon appears in the menu bar; clicking it shows the menu.

## Capture modes

- [ ] **Region (tray)** `⌃⌥⌘1` → crosshair cursor → drag a rect → image released on clipboard, file in `~/Pictures/ScreenshotUltra/`, Quick Tray appears bottom-right.
- [ ] **Region (esc cancels)** `⌃⌥⌘1` then press `Esc` → no file written, no tray.
- [ ] **Window (tray)** `⌃⌥⌘2` → hover-highlight → click a window → captured tightly (no drop shadow), tray appears.
- [ ] **Fullscreen (tray)** `⌃⌥⌘3` → entire main display captured, tray appears.
- [ ] **Silent variants** — bind `silent_region` / `silent_window` / `silent_fullscreen` in `settings.toml`, reload (settings hot-reload should pick this up within ~1 s), confirm captures happen but no tray.
- [ ] **Repeat last** `⌃⌥⌘R` → reruns the previous mode with the same tray-or-silent behaviour.
- [ ] **Pin last** `⌃⌥⌘.` → floating window appears with the latest screenshot; can drag, resize, ⌘W closes.
- [ ] **Open clipboard image** `⌃⌥⌘E` after copying an image elsewhere → image saved to disk and tray shown. With no clipboard image: "no image on the clipboard" logged.

## Quick Tray

- [ ] **Copy** button → image lands on clipboard, paste into another app works.
- [ ] **Folder** button → Finder opens at `save_folder`.
- [ ] **Reveal** button → Finder selects the captured file.
- [ ] **Pin** button → equivalent of `⌃⌥⌘.`.
- [ ] **Discard** button → file deleted, tray closes.
- [ ] **Auto-dismiss** — leave the tray alone; it should close after ~6 s (`quick_tray_timeout_ms`).
- [ ] **Successive captures** stack/replace the tray cleanly.

## Sinks

- [ ] **Disk** — every capture lands at the templated path.
- [ ] **Clipboard** — toggle `copy_on_capture = false`, capture, confirm clipboard not updated.
- [ ] **Shell** — set `sinks.shell = "/usr/bin/touch /tmp/screenshot-ultra-test.flag"`. Capture. Confirm flag file exists. Delete flag, set `shell = ""`, capture, confirm flag is NOT created.

## Logging

- [ ] `~/Library/Logs/ScreenshotUltra/log.ndjson` — one JSON line per event (startup, hotkey, capture, tray_action, settings_reloaded).
- [ ] `<save_folder>/.screenshot-ultra/index.ndjson` — one JSON line per capture, mirrors the main log's `capture` events.

## CLI

- [ ] `screenshot-ultra --version` prints `screenshot-ultra X.Y.Z`.
- [ ] `screenshot-ultra --help` prints usage.
- [ ] `screenshot-ultra --settings-path` prints `~/Library/Application Support/ScreenshotUltra/settings.toml`.
- [ ] `screenshot-ultra --print-defaults` prints a valid TOML doc that round-trips.

## Settings hot-reload

- [ ] Edit `settings.toml`, change a hotkey (e.g. `region = "ctrl+alt+cmd+9"`), save.
- [ ] Within ~1 s, console (or log) shows `settings reloaded`.
- [ ] Old binding (`⌃⌥⌘1`) no longer fires; new binding (`⌃⌥⌘9`) does.
- [ ] Set an invalid binding (`region = "wat+1"`). Save. Confirm "invalid hotkeys, keeping previous" appears and the previous binding still works.

## Settings persistence

- [ ] Quit the app, delete `settings.toml`, relaunch — file is recreated with defaults.
- [ ] Edit `quick_tray_timeout_ms = 12000`, save, capture — tray persists ~12 s.

## Permissions

- [ ] Revoke Screen Recording in System Settings → capture should fail with a log line; no crash.

## Release-build sanity

- [ ] `make app` produces `dist/Screenshot Ultra.app` (~5 s warm).
- [ ] `file dist/Screenshot Ultra.app/Contents/MacOS/screenshot-ultra` reports a Mach-O universal binary (on CI release builds).
- [ ] First launch on a clean Mac shows the Screen Recording permission prompt.
