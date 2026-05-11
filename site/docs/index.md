# Screenshot Ultra

> **Capture every pixel. Annotate like a native.**

A snappy, hotkey-first macOS screenshot and screen recorder. Local-first.
No cloud. No telemetry. No account.

Sister project to **MailBox Ultra** and **Postbin Ultra** — same stack,
same posture, same one-liner install.

## Three promises

1. **Snappy.** Hotkey → capture is sub-100 ms cold, sub-16 ms warm. No
   splash screens, no spinner. The overlay must feel like an OS
   primitive, not an app.
2. **Hotkey-first.** Every action is keyboardable. Mouse use is optional.
   Hotkeys are fully rebindable from `settings.toml` and changes apply
   live (no restart).
3. **Local-first.** No accounts, no analytics, no auto-upload. Captures
   live on your disk, in a folder you choose. Optional "share to URL" is
   opt-in via the shell-sink — you wire it up.

## What it does today (M2, in progress)

- **Capture**: region, window, fullscreen — main display or all
  connected displays.
- **Quick Tray**: a post-capture floating window with thumbnail and one-click
  actions: Copy / Edit (in Preview) / Folder / Reveal / Pin / Discard.
- **Pin-to-screen**: floating always-on-top window. Multiple pins.
- **Repeat last capture** and **paste a clipboard image** as a capture.
- **Sinks**: clipboard, disk, and an arbitrary shell command (`scp`, `rclone`,
  whatever you want — runs detached).
- **NDJSON event log** at `~/Library/Logs/ScreenshotUltra/log.ndjson` plus a
  per-folder history index at `<save_folder>/.screenshot-ultra/index.ndjson`.

The annotation editor and a graphical Preferences window are the remaining
M2 work. M3 onwards adds screen recording, scrolling capture, OCR, and the
beautify panel.

## Install

```sh
curl -sSL https://raw.githubusercontent.com/MPJHorner/ScreenshotUltra/main/scripts/install.sh | bash
```

See [Install](install.md) for `.dmg`, Homebrew, and from-source paths.
