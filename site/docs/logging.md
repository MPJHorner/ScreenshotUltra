# Logging

Two NDJSON streams (one JSON object per line).

## Global log

`~/Library/Logs/ScreenshotUltra/log.ndjson` — every event the app emits.

Sample events:

```json
{"ts":"2026-05-11T14:25:01.327Z","evt":"startup","version":"0.1.0","save_folder":"~/Pictures/ScreenshotUltra"}
{"ts":"2026-05-11T14:25:08.901Z","evt":"hotkey","action":"region"}
{"ts":"2026-05-11T14:25:09.034Z","evt":"capture","mode":"region","bytes":284113,"fmt":"png","saved_to":"…","sinks":["disk","clipboard"],"duration_ms":47}
{"ts":"2026-05-11T14:25:11.022Z","evt":"tray_action","action":"copy","path":"…"}
{"ts":"2026-05-11T14:25:40.000Z","evt":"settings_reloaded"}
```

## Per-folder history index

`<save_folder>/.screenshot-ultra/index.ndjson` — only `capture` and
`clipboard_image` events, mirroring what was actually saved into this
folder. Travels with the screenshots if you move the folder.

## Event types

| `evt`                    | When                                                 |
|--------------------------|------------------------------------------------------|
| `startup`                | App boot                                             |
| `hotkey`                 | Any registered global hotkey fired                   |
| `capture`                | A screen capture succeeded                           |
| `capture_cancelled`      | User pressed `Esc` during region/window capture      |
| `clipboard_image`        | Clipboard image successfully pasted as a capture     |
| `tray_action`            | User clicked a Quick Tray button                     |
| `pin`                    | User pinned the most recent capture                  |
| `settings_reloaded`      | `settings.toml` changed and the new config applied   |
| `settings_reload_error`  | Reload failed; previous config kept                  |
| `error`                  | A pipeline step failed (mode + error string)         |

## Examples

Show the last 20 captures:

```sh
tail -200 ~/Library/Logs/ScreenshotUltra/log.ndjson \
  | jq -c 'select(.evt=="capture") | {ts, mode, bytes, ms: .duration_ms}' \
  | tail -20
```

Find captures larger than 1 MB:

```sh
jq -c 'select(.evt=="capture" and .bytes > 1000000) | .saved_to' \
  ~/Library/Logs/ScreenshotUltra/log.ndjson
```

Total bytes captured today:

```sh
DATE=$(date +%Y-%m-%d)
jq -s "
  map(select(.evt==\"capture\" and (.ts | startswith(\"$DATE\")))) | map(.bytes) | add
" ~/Library/Logs/ScreenshotUltra/log.ndjson
```
