# Manual test plan

Walk this checklist before tagging a release. Tests that can't be
automated (real screen capture, real clipboard, real menu bar, drawing
events) live here.

## Setup

1. `make app` → quit any existing instance, then `open "dist/Screenshot Ultra.app"`.
2. First launch: grant **Screen Recording** in System Settings →
   Privacy & Security → Screen & System Audio Recording. Relaunch.
3. Confirm `~/Library/Application Support/ScreenshotUltra/settings.toml`
   was created with defaults.
4. Confirm the **aperture iris** icon appears in the menu bar (not the
   old camera glyph). Light + dark mode should both tint correctly.

## Capture modes

- [ ] **Region (tray)** `⌃⌥⌘1` → crosshair → drag → file under
      `~/Pictures/ScreenshotUltra/`, image on clipboard, Quick Tray
      bottom-right.
- [ ] **Region cancel** `⌃⌥⌘1` then `Esc` → no file, no tray, log entry
      `capture_cancelled`.
- [ ] **Window (tray)** `⌃⌥⌘2` → hover-highlight → click a window →
      pixel-tight crop, no drop shadow, tray appears.
- [ ] **Fullscreen (tray)** `⌃⌥⌘3` → main display captured, tray.
- [ ] **Fullscreen all-displays** set `capture.fullscreen_scope = "all"`,
      hot-reload kicks in, `⌃⌥⌘3` captures every connected display (one
      file per display).
- [ ] **Cursor included** set `capture.include_cursor = true`, capture,
      confirm the mouse arrow is baked into the image.
- [ ] **Timed 3 s / 5 s / 10 s** tray menu items — countdown runs via
      macOS's built-in shutter; image lands in tray when the timer fires.
- [ ] **Silent variants** — bind `silent_region` /
      `silent_window` / `silent_fullscreen` in `settings.toml`, save,
      reload kicks in within ~1 s, confirm captures happen but no tray.
- [ ] **Repeat last** `⌃⌥⌘R` → reruns the previous mode with the same
      tray-or-silent behaviour.
- [ ] **Pin last** `⌃⌥⌘.` → floating always-on-top window with the
      latest screenshot; can drag, resize, `⌘W` closes. Multiple pins
      cascade 24 px apart.
- [ ] **Open clipboard image** `⌃⌥⌘E` after copying an image elsewhere
      → image saved to disk + tray shown. With no clipboard image:
      "no image on the clipboard" logged.
- [ ] **Colour picker** `⌃⌥⌘P` → macOS magnifier cursor appears; click
      any pixel → hex copied to clipboard, `eyedropper_pick` event in
      the log.

## Quick Tray

- [ ] **Copy** button → image lands on clipboard, paste into another app
      works.
- [ ] **Edit** button → annotation editor opens with the capture loaded.
- [ ] **Folder** button → Finder opens at `save_folder`.
- [ ] **Reveal** button → Finder selects the captured file.
- [ ] **Pin** button → equivalent of `⌃⌥⌘.`.
- [ ] **Discard** button → file deleted, tray closes.
- [ ] **Auto-dismiss** — leave the tray alone; it should close after
      `quick_tray_timeout_ms` (default 6 s).
- [ ] **Successive captures** stack/replace the tray cleanly.

## Annotation editor

- [ ] Open the editor via the Quick Tray's **Edit** button on a region
      capture.
- [ ] **Tool buttons** highlight correctly when clicked or shortcut pressed.
- [ ] **Pen** `P` — drag draws a freehand stroke in the current colour.
- [ ] **Line** `L` — drag draws a straight line.
- [ ] **Arrow** `A` — drag draws a line with an arrowhead at the tip.
- [ ] **Rect** `R` / **Ellipse** `E` — drag-defines bounds; stroked.
- [ ] **Highlighter** `H` — drag draws translucent yellow over the image.
- [ ] **Redact** `X` — drag-defines a filled black rect.
- [ ] **Counter** `N` — clicking drops auto-incrementing numbers
      starting at 1; numbers reset when the editor reopens.
- [ ] **Text** `T` — clicking shows an `NSAlert` prompt; the typed string
      is rendered at the click point in the current colour.
- [ ] **Blur** `B` — drag-defines a rect; the underlying image pixels
      inside are pixelated. Resizes correctly when saved.
- [ ] **Crop** `C` — drag-defines a yellow guide; mouseUp replaces the
      image with the cropped region, resizes the window, clears
      annotations.
- [ ] **Colour palette** — Red / Yellow / Green / Blue / Black; selected
      swatch highlights.
- [ ] **Stroke width** `1` / `2` / `3` — Thin / Med / Thick; selected
      width highlights.
- [ ] **Undo** `⌘Z` removes the last shape.
- [ ] **Redo** `⌘⇧Z` re-applies an undone shape.
- [ ] **Clear** button removes every annotation.
- [ ] **Save** `⌘S` overwrites the original capture; verify the file on
      disk has the annotations.
- [ ] **Copy** `⌘C` puts the annotated image on the clipboard; paste
      into another app shows the annotated version.
- [ ] **Close** `⌘W` — annotations made after the last Save are lost
      (no autosave by design).
- [ ] **Tooltips** — hover any toolbar button; a one-line hint with the
      shortcut appears after ~½ s.

## Sinks

- [ ] **Disk** — every capture lands at the templated path.
- [ ] **Clipboard** — toggle `copy_on_capture = false`, capture, confirm
      clipboard is not updated.
- [ ] **Shell** — set
      `sinks.shell = "/usr/bin/touch /tmp/screenshot-ultra-test.flag"`,
      capture, confirm `/tmp/screenshot-ultra-test.flag` exists. Delete
      flag, set `shell = ""`, capture, confirm flag is NOT created.

## Logging

- [ ] `~/Library/Logs/ScreenshotUltra/log.ndjson` — one JSON line per
      event (`startup`, `hotkey`, `capture`, `tray_action`, `pin`,
      `editor_save`, `editor_copy`, `editor_crop`, `eyedropper_pick`,
      `settings_reloaded`, `panic`).
- [ ] `<save_folder>/.screenshot-ultra/index.ndjson` — one JSON line per
      capture, mirrors the main log's `capture` events.

## CLI

- [ ] `screenshot-ultra --version` prints `screenshot-ultra X.Y.Z`.
- [ ] `screenshot-ultra --help` prints usage.
- [ ] `screenshot-ultra --settings-path` prints the absolute path.
- [ ] `screenshot-ultra --print-defaults` prints valid TOML that round-trips.

## Settings hot-reload

- [ ] Edit `settings.toml`, change a hotkey (e.g. `region = "ctrl+alt+cmd+9"`).
- [ ] Within ~1 s the console / log shows `settings_reloaded`.
- [ ] Old binding (`⌃⌥⌘1`) no longer fires; new binding (`⌃⌥⌘9`) does.
- [ ] Set an invalid binding (`region = "wat+1"`). Save. Confirm
      "invalid hotkeys, keeping previous" appears and the previous
      binding still works.

## App bundle

- [ ] `make app` produces `dist/Screenshot Ultra.app` (~5 s warm).
- [ ] `dist/Screenshot Ultra.app/Contents/Resources/AppIcon.icns` exists.
- [ ] The app's Finder icon in `/Applications` is the aperture-iris
      brand icon.
- [ ] Spotlight finds **Screenshot Ultra** with the brand icon as the
      preview.

## Release-build sanity

- [ ] Push a `vX.Y.Z` tag → Release workflow builds the universal `.zip`
      and a `.sha256` sidecar, both attached to a fresh GitHub release.
- [ ] `install.sh` against the latest release verifies the checksum and
      drops the `.app` into `/Applications`.
- [ ] First launch on a clean Mac shows the Screen Recording permission
      prompt with the brand icon.
