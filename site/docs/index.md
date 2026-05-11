# Screenshot Ultra

A snappy, hotkey-first macOS screenshot and screen recorder. Press a key,
capture anything on screen — region, window, fullscreen, or clipboard
image — annotate it inline with eleven tools, and it's on your clipboard
before the shutter sound finishes. Built in Rust + native AppKit, runs
entirely on your machine, no telemetry, no cloud account, no auto-upload.

> One sentence: **press the key, drag the box, it's on your clipboard.**

## Three promises

1. **Snappy.** Hotkey → capture is sub-100 ms cold, sub-16 ms warm. No
   splash screens, no spinner. The overlay must feel like an OS
   primitive, not an app.
2. **Hotkey-first.** Every action is keyboardable. Mouse use is
   optional. Hotkeys are fully rebindable from `settings.toml` and
   changes apply within a second (no restart).
3. **Local-first.** No accounts, no analytics, no auto-upload. Captures
   live on your disk in a folder you choose. Optional "share to URL"
   is opt-in via the shell-sink — *you* wire it up.

## What's in the box

- **Capture modes** — region, window, fullscreen (main or all
  displays), timed (3 / 5 / 10 s countdown), repeat-last, clipboard
  image → editor.
- **Quick Tray** — a native floating `NSWindow` after every capture:
  thumbnail + **Copy / Edit / Folder / Reveal / Pin / Discard**.
  Auto-dismisses after 6 s; silent counterpart for zero-UI mode.
- **Native annotation editor** — eleven tools (Pen / Line / Arrow /
  Rect / Ellipse / Highlighter / Redact / Counter / Text / Blur /
  Crop), five-colour palette, three-step stroke-width picker, full
  undo/redo.
- **Pin-to-screen** — floating always-on-top window with the latest
  capture. Cascade multiple pins.
- **Sinks** — clipboard, disk, and an arbitrary shell command (`$1` =
  file path). Runs detached so a slow uploader never stalls capture.
- **Logging** — NDJSON event log + per-folder history index.

## Install

```sh
curl -sSL https://raw.githubusercontent.com/MPJHorner/ScreenshotUltra/main/scripts/install.sh | bash
```

[Full install instructions](install.md) — `.zip`, manual, build-from-source,
Gatekeeper, permissions.

## Sister projects

Part of the **Ultra** family of local-first developer tools:

- [**MailBox Ultra**](https://github.com/MPJHorner/MailboxUltra) — local SMTP fake inbox with WebKit HTML preview.
- [**Postbin Ultra**](https://github.com/MPJHorner/PostbinUltra) — local HTTP request inspector with JSON tree view and forward + replay.
- [**IDE Ultra**](https://github.com/MPJHorner/IdeUltra) — local-first native code IDE in pure Rust + egui.

Same posture across all four: native, snappy, local-first, no
telemetry, MIT.
