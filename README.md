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

**M2 progress:** Quick Tray, silent-vs-standard split, window capture mode,
Pin-to-screen, Repeat-last, and a polished tray menu have all landed.
Annotation editor and Preferences UI are the remaining M2 pieces.

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

| Action                          | Default   | Notes                                      |
|---------------------------------|-----------|--------------------------------------------|
| Region capture (Quick Tray)     | `⌃⌥⌘1`    | Standard flow: shows the floating tray     |
| Window capture (Quick Tray)     | `⌃⌥⌘2`    | Interactive window selection               |
| Fullscreen capture (Quick Tray) | `⌃⌥⌘3`    | Standard flow: shows the floating tray     |
| Open clipboard image            | `⌃⌥⌘E`    | Pastes a clipboard image into the same flow |
| Repeat last capture             | `⌃⌥⌘R`    | Re-runs the previous mode                  |
| Pin last capture to screen      | `⌃⌥⌘.`    | Floating always-on-top window              |
| Region / Window / Fullscreen (silent) | _unset_ | Set `silent_*` in settings to enable |

After every "standard" capture, the **Quick Tray** appears bottom-right with
**Copy / Folder / Reveal / Pin / Discard** buttons. It auto-dismisses after 6 s
(`quick_tray_timeout_ms`). The silent variants skip the tray and just save +
copy to clipboard — useful when you want zero-latency capture for piping
into other tools.

All hotkeys are rebindable in
`~/Library/Application Support/ScreenshotUltra/settings.toml`. Edits take
effect within ~1 s — no app restart needed. Invalid bindings keep the
previous setting (with a log line) so you can't lock yourself out.

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
region                 = "ctrl+alt+cmd+1"      # standard flow (Quick Tray)
window                 = "ctrl+alt+cmd+2"      # standard flow (Quick Tray)
fullscreen             = "ctrl+alt+cmd+3"      # standard flow (Quick Tray)
silent_region          = ""                    # set e.g. "ctrl+alt+cmd+4" to enable
silent_window          = ""
silent_fullscreen      = ""
pin_last               = "ctrl+alt+cmd+period"
repeat_last            = "ctrl+alt+cmd+r"

[sinks]
clipboard              = true
disk                   = true
```

## Sinks

Every successful capture fans out to one or more sinks. All are configurable
in `settings.toml`:

| Sink       | What it does                                                       |
|------------|--------------------------------------------------------------------|
| `disk`     | Writes the image to the templated path under `save_folder`         |
| `clipboard`| Places the image on the macOS clipboard                            |
| `shell`    | Runs `sh -c "<your command>" -- <path>` (path is `$1`); detached   |

Shell-sink examples:

```toml
[sinks]
shell = "scp $1 user@host:/var/www/img/"
# or
shell = "rclone copy $1 remote:bucket/"
# or
shell = "/usr/local/bin/slack-upload $1"
```

The shell-sink runs detached so even a slow uploader never blocks the
capture pipeline. Output is suppressed; check stderr by running the
command manually first.

## CLI flags

```sh
screenshot-ultra --help              # print usage
screenshot-ultra --version           # print version
screenshot-ultra --settings-path     # print the absolute path to settings.toml
screenshot-ultra --print-defaults    # print the default settings.toml to stdout
```

Run without flags to launch the menu-bar agent.

## Logging

Every event is one JSON line in `~/Library/Logs/ScreenshotUltra/log.ndjson`:

```json
{"ts":"2026-05-11T14:25:01.327Z","evt":"capture","mode":"region","bytes":284113,"fmt":"png","saved_to":"…","sinks":["disk","clipboard"],"duration_ms":47}
```

Grep, pipe to `jq`, or just `tail -f` while debugging.

The same payload (minus the `ts`) is also appended to
`<save_folder>/.screenshot-ultra/index.ndjson` — a per-folder history
index that travels with your screenshots.

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
