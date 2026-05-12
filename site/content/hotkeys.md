---
title: "Hotkeys"
description: "Every default binding, every modifier, the rebinding syntax — and how to wire silent flows alongside the tray ones."
slug: "hotkeys"
---

# Hotkeys

Every action is bound from `settings.toml`. Live-reloaded — save the
file and your new bindings take effect within a second.

## Defaults

```toml
[hotkeys]
region                 = "ctrl+alt+cmd+1"   # Region (Quick Tray)
window                 = "ctrl+alt+cmd+2"   # Window (Quick Tray)
fullscreen             = "ctrl+alt+cmd+3"   # Fullscreen (Quick Tray)
open_clipboard_image   = "ctrl+alt+cmd+e"   # Paste clipboard image
repeat_last            = "ctrl+alt+cmd+r"   # Repeat previous mode
pin_last               = "ctrl+alt+cmd+."   # Pin most recent capture
color_picker           = "ctrl+alt+cmd+p"   # Eyedropper → hex on clipboard
record_video           = "ctrl+alt+cmd+v"   # Start/stop video recording
record_gif             = "ctrl+alt+cmd+g"   # Start/stop GIF recording
preferences            = "ctrl+alt+cmd+,"   # In-app settings editor
help                   = "ctrl+alt+cmd+/"   # Cheat-sheet window
silent_region          = ""                 # unbound by default
silent_window          = ""                 # unbound by default
silent_fullscreen      = ""                 # unbound by default
```

## Syntax

A binding is `mod+mod+key`. Modifiers, in any order:

- `ctrl` / `control`
- `alt` / `option` / `opt`
- `cmd` / `command` / `super` / `meta`
- `shift`

Keys:

- Digits `0`-`9`
- Letters `a`-`z` (case-insensitive)
- `.` / `period`, `,` / `comma`, `/` / `slash`, `\\` / `backslash`
- `space`, `tab`, `enter` / `return`, `esc` / `escape`

Whitespace around `+` is fine. `ctrl+alt+cmd+1` and `Ctrl + Alt + Cmd + 1`
parse identically.

## Empty = unbound

An empty string in any slot disables that hotkey. Useful when you want
to remove the default for an action without inventing a new one.

## Invalid bindings

If you save an invalid hotkey (typo, conflict, modifier-only), the log
will say:

```
settings reload: invalid hotkeys, keeping previous: parsing hotkey for window: wat+1
```

…and the *previous* set stays active. You can't lock yourself out by typo.

## macOS built-in conflicts

Avoid `⌘⇧3` / `⌘⇧4` / `⌘⇧5` — those are owned by macOS's own screenshot
tool. The default `⌃⌥⌘N` scheme was chosen to never clash with system
shortcuts or common app bindings.

## All action IDs

These are the keys you set under `[hotkeys]` in `settings.toml`:

| ID                      | What it does                                                  |
|-------------------------|---------------------------------------------------------------|
| `region`                | Drag a rectangle; show Quick Tray afterwards                  |
| `window`                | Pick a window; show Quick Tray                                |
| `fullscreen`            | Capture the whole display; show Quick Tray                    |
| `silent_region`         | Region, no tray (saves + clipboard only)                      |
| `silent_window`         | Window, no tray                                               |
| `silent_fullscreen`     | Fullscreen, no tray                                           |
| `pin_last`              | Pin the most recent capture as a floating window              |
| `repeat_last`           | Re-run the previous mode with the same tray-or-silent flag    |
| `open_clipboard_image`  | Paste a clipboard image into the Quick Tray flow              |
| `color_picker`          | Show macOS's colour sampler; copies the picked hex            |
| `record_video`          | Toggle screen recording (`.mov`)                              |
| `record_gif`            | Toggle screen recording, post-process to `.gif`               |
| `preferences`           | Open the in-app `settings.toml` editor                        |
| `help`                  | Open the keyboard-shortcut cheat sheet                        |
