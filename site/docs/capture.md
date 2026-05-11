# Capture modes

## Region — `⌃⌥⌘1`

Drag a rectangle. `Esc` cancels (no file written). The image is saved
to disk and copied to your clipboard, then the Quick Tray appears.

## Window — `⌃⌥⌘2`

A crosshair cursor with hover-highlight appears. Click any window to
capture it tightly. The window's drop shadow is trimmed off (`-o`)
because we want a pixel-tight crop. To capture with shadow, set
`include_cursor = true` and the future shadow option (planned).

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

## Repeat last — `⌃⌥⌘R`

Re-runs whatever mode you used last with the same tray-or-silent
behaviour. The most recent capture is kept in memory until you quit the
app.

## Pin last — `⌃⌥⌘.`

Opens the most recent capture as a floating, always-on-top window.
Useful for keeping a reference visible while you type or work on
something else. Multiple pins are supported and cascade 24 px from each
other.

## Open clipboard image — `⌃⌥⌘E`

Pulls an image off the macOS clipboard (any other app's "copy image"
output, including macOS's own `⌘⇧4` to clipboard), saves it as a normal
capture, and runs it through the Quick Tray. Useful when a teammate
sends you an image inline and you want to annotate or pin it.
