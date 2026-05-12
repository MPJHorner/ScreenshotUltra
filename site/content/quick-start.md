---
title: "Quick start"
description: "First capture, first annotation, first shell-sink — in 60 seconds."
slug: "quick-start"
---

# Quick start

After [installing]({{base}}/install/) and granting Screen Recording
permission, Screenshot Ultra runs as a tiny aperture-iris icon in your
menu bar.

## Your first capture

Press **`⌃⌥⌘1`**. A crosshair cursor appears. Drag a rectangle. Let go.

You'll see three things happen at once:

- The image is on your **clipboard** — paste anywhere with `⌘V`.
- A copy is **saved to disk** at `~/Pictures/ScreenshotUltra/`.
- The **Quick Tray** pops up bottom-right with a thumbnail + actions:
  Copy / Text / Edit / Folder / Reveal / Pin / Discard. It
  auto-dismisses after 6 seconds.

That's it. From hotkey to "image in next app" is sub-100 ms warm.

## The full default keymap

| Action                  | Default | Description                                |
|-------------------------|---------|--------------------------------------------|
| Region capture          | `⌃⌥⌘1`  | Drag a rectangle; Esc cancels              |
| Window capture          | `⌃⌥⌘2`  | Hover-highlight a window, click            |
| Fullscreen capture      | `⌃⌥⌘3`  | Main display or all displays               |
| Open clipboard image    | `⌃⌥⌘E`  | Treat a clipboard image as a fresh capture |
| Repeat last capture     | `⌃⌥⌘R`  | Re-run the previous mode                   |
| Pin last to screen      | `⌃⌥⌘.`  | Floating always-on-top window              |
| Colour picker           | `⌃⌥⌘P`  | Eyedropper → `#rrggbb` on clipboard        |
| Record video (toggle)   | `⌃⌥⌘V`  | Start/stop a screen recording (`.mov`)     |
| Record GIF (toggle)     | `⌃⌥⌘G`  | Same flow, post-processed to `.gif`        |
| Preferences             | `⌃⌥⌘,`  | In-app `settings.toml` editor              |
| Cheat sheet             | `⌃⌥⌘/`  | All hotkeys + editor shortcuts             |

Every binding is configurable in `settings.toml` — see [Hotkeys]({{base}}/hotkeys/).

## Add a silent flow

The defaults all show the Quick Tray. Want a silent flow too — just
disk + clipboard, no UI? Open Preferences (`⌃⌥⌘,`) and fill in the
silent slots:

```toml
[hotkeys]
silent_region     = "ctrl+alt+cmd+4"
silent_window     = "ctrl+alt+cmd+5"
silent_fullscreen = "ctrl+alt+cmd+6"
```

Click **Apply**. Within a second your new bindings are live; the
tray-flow defaults (`⌃⌥⌘1` etc.) are untouched.

## Wire up a shell sink

To pipe every capture somewhere — S3, your team's Slack, your own URL
shortener — add a shell command. The captured file's path is `$1`:

```toml
[sinks]
shell = "rclone copy $1 s3:my-bucket/screenshots/"
```

Save. The next capture runs the command detached so slow uploaders
never stall the capture pipeline. See [Sinks]({{base}}/sinks/) for more.

## Where things live

| File                                                                          | What                  |
|-------------------------------------------------------------------------------|-----------------------|
| `~/Library/Application Support/ScreenshotUltra/settings.toml`                 | All settings          |
| `~/Library/Logs/ScreenshotUltra/log.ndjson`                                   | One JSON line / event |
| `~/Pictures/ScreenshotUltra/`                                                 | Default save folder   |
| `~/Pictures/ScreenshotUltra/.screenshot-ultra/index.ndjson`                   | Per-folder history    |
