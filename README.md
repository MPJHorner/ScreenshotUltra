# Screenshot Ultra

> **Capture every pixel. Annotate like a native.**
> A snappy, hotkey-first macOS screenshot and screen recorder.
> Local-first. No cloud. No telemetry. No account.

Sister project to **MailBox Ultra** and **Postbin Ultra** — same stack, same posture, same one-liner install.

## Status

This repository is under active development. The current shipped milestone is
**M1 — "Press the key"**:

- Menu-bar agent (no dock icon) with a small status-bar menu.
- Global hotkeys (rebindable in `settings.toml`).
- Region and fullscreen capture via the macOS `screencapture` backend.
- Disk sink (`~/Pictures/ScreenshotUltra/` by default) with templated filenames.
- Clipboard sink (image lands on your clipboard on every capture).
- NDJSON event log at `~/Library/Logs/ScreenshotUltra/log.ndjson`.

**M2 progress:** the Quick Tray (post-capture floating toolbar) and the
silent-vs-standard capture split have landed; the annotation editor, window
capture, Pin-to-screen, and Preferences UI are still ahead.

Future milestones (M2–M6) are tracked in [`docs/milestones/`](docs/milestones/)
and the full design lives in [`plan.md`](plan.md).

## Install

Once `.dmg` releases are published the one-liner will be:

```sh
curl -sSL https://raw.githubusercontent.com/MPJHorner/ScreenshotUltra/main/scripts/install.sh | bash
```

For now, build from source (Rust stable, macOS 13+):

```sh
git clone https://github.com/MPJHorner/ScreenshotUltra.git
cd ScreenshotUltra
make app                     # builds dist/Screenshot Ultra.app
open "dist/Screenshot Ultra.app"
```

On first launch macOS will prompt for **Screen Recording** permission.
Grant it in System Settings → Privacy & Security → Screen & System Audio
Recording, then launch the app again.

## Default hotkeys

| Action                       | Default | Notes                                |
|------------------------------|---------|--------------------------------------|
| Region capture (Quick Tray)  | `⌃⌥⌘1`  | Standard flow: shows the floating tray |
| Fullscreen capture (Quick Tray)| `⌃⌥⌘3`| Standard flow: shows the floating tray |
| Region capture (silent)      | _unset_ | Bind `silent_region` in settings to enable |
| Fullscreen capture (silent)  | _unset_ | Bind `silent_fullscreen` in settings to enable |

After every "standard" capture, the **Quick Tray** appears bottom-right with
Copy / Folder / Reveal / Discard buttons. It auto-dismisses after 6 s
(`quick_tray_timeout_ms`). The silent variants skip the tray and just save +
copy to clipboard — useful when you want zero-latency capture for piping
into other tools.

All hotkeys are rebindable in
`~/Library/Application Support/ScreenshotUltra/settings.toml`.

The full roadmap of hotkeys (window, scrolling, video, GIF, etc.) is in
[`plan.md` §3](plan.md#3-hotkeys--the-centerpiece).

## Settings

`~/Library/Application Support/ScreenshotUltra/settings.toml` is auto-created on
first run with sensible defaults. Edit it freely and restart the app.

```toml
[general]
save_folder            = "~/Pictures/ScreenshotUltra"
filename_template      = "{date}_{time}_{mode}_{seq}"
default_image_format   = "png"
copy_on_capture        = true

[hotkeys]
region                 = "ctrl+alt+cmd+1"   # standard flow (Quick Tray)
fullscreen             = "ctrl+alt+cmd+3"   # standard flow (Quick Tray)
silent_region          = ""                 # set e.g. "ctrl+alt+cmd+2" to enable
silent_fullscreen      = ""                 # set e.g. "ctrl+alt+cmd+4" to enable

[sinks]
clipboard              = true
disk                   = true
```

## Logging

Every event is one JSON line in `~/Library/Logs/ScreenshotUltra/log.ndjson`:

```json
{"ts":"2026-05-11T14:25:01.327Z","evt":"capture","mode":"region","bytes":284113,"fmt":"png","saved_to":"…","sinks":["disk","clipboard"],"duration_ms":47}
```

Grep, pipe to `jq`, or just `tail -f` while debugging.

## Roadmap

- **M1** — Press the key (shipped). ✅
- **M2** — Annotate: native annotation editor with arrows, blur, crop, undo/redo.
- **M3** — Record: video + GIF, system audio, mouse highlight.
- **M4** — Scroll & Beautify: scrolling capture, padding/gradient/frames.
- **M5** — OCR & polish: Apple Vision OCR, ruler, color picker.
- **M6** — Ship: signed + notarised DMG, Homebrew cask, v1.0.

See [`docs/milestones/`](docs/milestones/) for per-milestone breakdowns.

## License

MIT — see [LICENSE](LICENSE).
