---
title: "Capture modes"
description: "Region, window, fullscreen, timed, repeat-last, clipboard image, video, GIF — every way to put pixels into the pipeline."
slug: "capture"
---

# Capture modes

## Region — `⌃⌥⌘1`

Drag a rectangle. `Esc` cancels (no file written). The image is saved
to disk and copied to your clipboard, then the Quick Tray appears.

## Window — `⌃⌥⌘2`

A crosshair cursor with hover-highlight appears. Click any window to
capture it tightly. The drop shadow is trimmed off (`-o`) because we
want a pixel-tight crop.

## Fullscreen — `⌃⌥⌘3`

Captures the whole display in one shot. By default this is the **main**
display (the one with the menu bar). Change behaviour in `settings.toml`:

```toml
[capture]
fullscreen_scope = "main"   # default: just the main display
# fullscreen_scope = "all"  # every connected display, one file per display
```

## Cursor

By default the cursor is **not** included in any capture. Toggle:

```toml
[capture]
include_cursor = true
```

## Timed captures

The tray menu has **Fullscreen in 3 / 5 / 10 s** entries. They use
`screencapture -T` so the OS-level countdown runs (no flash of UI
chrome from us).

## Repeat last — `⌃⌥⌘R`

Re-runs whatever mode you used last with the same tray-or-silent
behaviour. The most recent capture is kept in memory until you quit the
app.

## Pin last — `⌃⌥⌘.`

Opens the most recent capture as a floating, always-on-top window.
Useful for keeping a reference visible while you type. Multiple pins
cascade 24 px from each other.

While a pin is focused:

- **Scroll** adjusts opacity (0.3 ↔ 1.0)
- **⌘+** / **⌘-** zoom in place
- **⌘0** resets opacity to 1.0
- **⌫** / **Esc** closes the pin
- **⌘W** closes the pin

## Open clipboard image — `⌃⌥⌘E`

Pulls an image off the macOS clipboard (any other app's "copy image"
output, including macOS's own `⌘⇧4` to clipboard), saves it as a normal
capture, and runs it through the Quick Tray. Useful when a teammate
sends you an image inline and you want to annotate or pin it.

## Video — `⌃⌥⌘V`

Toggle. First press starts a recording (`screencapture -v`), second
press stops it (SIGINT, which finalises the `.mov`). The menu-bar icon
flips to a solid filled disc while recording; a notification banner
fires when the file lands.

Settings:

```toml
[recording]
show_clicks       = true   # highlight mouse clicks in the video
record_microphone = false  # capture from the default input device
```

## GIF — `⌃⌥⌘G`

Same flow as video but post-processed via `ffmpeg` (12 fps, palettegen
+ paletteuse) into a `.gif`. Install `ffmpeg` once:

```sh
brew install ffmpeg
```

Without `ffmpeg` on PATH, the `.mov` is kept and a log line tells you
why.

## Eyedropper — `⌃⌥⌘P`

Shows macOS's built-in colour sampler magnifier. Clicking any on-screen
pixel copies its sRGB `#rrggbb` onto the clipboard and emits an
`eyedropper_pick` NDJSON event.
