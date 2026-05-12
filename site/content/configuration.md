---
title: "Configuration"
description: "Every setting in settings.toml, with the schema, defaults, and hot-reload semantics."
slug: "configuration"
---

# Configuration

All settings live in `~/Library/Application Support/ScreenshotUltra/settings.toml`.
The file is auto-created with defaults on first run. Edit it freely —
changes are picked up within ~1 second of any save. **No restart.**

Print the path:

```sh
screenshot-ultra --settings-path
```

Print the full default file (handy for regenerating):

```sh
screenshot-ultra --print-defaults
```

Or use the in-app Preferences window (`⌃⌥⌘,`) which validates the
TOML before saving so a typo can't silently revert your bindings.

## Schema

```toml
[general]
save_folder            = "~/Pictures/ScreenshotUltra"
filename_template      = "{date}_{time}_{mode}_{seq}"
default_image_format   = "png"          # png | jpg | tiff | bmp | heic
copy_on_capture        = true
play_shutter_sound     = true
show_in_dock           = false
quick_tray_timeout_ms  = 6000

[capture]
include_cursor         = false
fullscreen_scope       = "main"         # main | all

[recording]
show_clicks            = true
record_microphone      = false

[hotkeys]
region                 = "ctrl+alt+cmd+1"
window                 = "ctrl+alt+cmd+2"
fullscreen             = "ctrl+alt+cmd+3"
open_clipboard_image   = "ctrl+alt+cmd+e"
pin_last               = "ctrl+alt+cmd+period"
repeat_last            = "ctrl+alt+cmd+r"
color_picker           = "ctrl+alt+cmd+p"
record_video           = "ctrl+alt+cmd+v"
record_gif             = "ctrl+alt+cmd+g"
preferences            = "ctrl+alt+cmd+comma"
help                   = "ctrl+alt+cmd+slash"
silent_region          = ""             # unset by default
silent_window          = ""
silent_fullscreen      = ""

[sinks]
clipboard              = true
disk                   = true
shell                  = ""             # e.g. "scp $1 user@host:/var/www/img/"
```

## Filename template tokens

| Token   | Example       |
|---------|---------------|
| `{date}`| `20260512`    |
| `{time}`| `144527`      |
| `{mode}`| `region` / `window` / `fullscreen` / `video` / `gif` |
| `{seq}` | `001` (zero-padded, increments per app run) |

Unknown tokens are passed through literally — so `_{nope}_` will appear
in the filename as `_{nope}_`. Useful while you're experimenting.

## Hot-reload behaviour

`settings.toml` is watched on a background thread (1 Hz mtime poll).
On a successful change:

- The new config is parsed.
- Hotkeys are re-registered.
- An NDJSON `settings_reloaded` event fires.

On an invalid change (typo, modifier-only hotkey, unsupported key
name):

- The previous config stays active.
- An `settings_reload_error` event fires with the parse error.
- The console prints `settings reload: invalid hotkeys, keeping previous`.

That last bit is on purpose — you cannot lock yourself out by typo.
