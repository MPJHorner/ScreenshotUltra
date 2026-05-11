# Quick start

After [installing](install.md) and granting Screen Recording permission,
Screenshot Ultra runs as a tiny menu-bar icon (no dock icon).

## First capture

Press **`⌃⌥⌘1`** anywhere. A crosshair appears.

Drag a rectangle. Let go.

- The image is **on your clipboard**.
- A copy is **saved to `~/Pictures/ScreenshotUltra/`**.
- The **Quick Tray** floats in the bottom-right corner: thumbnail plus
  Copy / Edit / Folder / Reveal / Pin / Discard buttons. After 6 seconds
  it auto-dismisses.

Press `⌘V` in any app to paste.

## The full keymap (defaults)

| Action               | Default | Description                                |
|----------------------|---------|--------------------------------------------|
| Region capture       | `⌃⌥⌘1`  | Drag a rectangle; Esc cancels              |
| Window capture       | `⌃⌥⌘2`  | Hover-highlight, click to capture          |
| Fullscreen capture   | `⌃⌥⌘3`  | Main display (or all — see config)         |
| Open clipboard image | `⌃⌥⌘E`  | Pastes a clipboard image into the same flow|
| Repeat last capture  | `⌃⌥⌘R`  | Re-runs the previous mode                  |
| Pin last to screen   | `⌃⌥⌘.`  | Floating always-on-top window              |

All hotkeys are configurable; see [Hotkeys](hotkeys.md).

## The silent variants

Want a no-tray capture flow alongside the standard one? Set the silent
slots in `settings.toml`:

```toml
[hotkeys]
silent_region     = "ctrl+alt+cmd+4"
silent_fullscreen = "ctrl+alt+cmd+5"
silent_window     = "ctrl+alt+cmd+6"
```

Save the file. Within a second your bindings update — no restart.

## Where things live

| File                                                                          | What                  |
|-------------------------------------------------------------------------------|-----------------------|
| `~/Library/Application Support/ScreenshotUltra/settings.toml`                 | All settings          |
| `~/Library/Logs/ScreenshotUltra/log.ndjson`                                   | One JSON line / event |
| `~/Pictures/ScreenshotUltra/`                                                 | Default save folder   |
| `~/Pictures/ScreenshotUltra/.screenshot-ultra/index.ndjson`                   | Per-folder history    |
