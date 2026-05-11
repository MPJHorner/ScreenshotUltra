# Configuration

All settings live in `~/Library/Application Support/ScreenshotUltra/settings.toml`.
The file is auto-created with defaults on first run. Edit it freely — changes
take effect within a second (no restart).

Print the path:

```sh
screenshot-ultra --settings-path
```

Print the full default file (handy for regenerating):

```sh
screenshot-ultra --print-defaults
```

## Schema

```toml
[general]
save_folder            = "~/Pictures/ScreenshotUltra"
filename_template      = "{date}_{time}_{mode}_{seq}"
default_image_format   = "png"          # png | jpg | tiff | pdf | bmp
copy_on_capture        = true
play_shutter_sound     = true
show_in_dock           = false
quick_tray_timeout_ms  = 6000

[capture]
include_cursor         = false
fullscreen_scope       = "main"         # main | all

[hotkeys]
region                 = "ctrl+alt+cmd+1"
window                 = "ctrl+alt+cmd+2"
fullscreen             = "ctrl+alt+cmd+3"
open_clipboard_image   = "ctrl+alt+cmd+e"
pin_last               = "ctrl+alt+cmd+period"
repeat_last            = "ctrl+alt+cmd+r"
silent_region          = ""             # unset by default
silent_window          = ""             # unset by default
silent_fullscreen      = ""             # unset by default

[sinks]
clipboard              = true
disk                   = true
shell                  = ""             # e.g. "scp $1 user@host:/var/www/img/"
```

## Filename template tokens

| Token   | Example       |
|---------|---------------|
| `{date}`| `20260511`    |
| `{time}`| `144527`      |
| `{mode}`| `region` / `window` / `fullscreen` |
| `{seq}` | `001` (zero-padded, increments per app run) |

Unknown tokens are passed through literally — so `_{nope}_` will appear
in the filename as `_{nope}_`. Useful while you're playing with templates.
