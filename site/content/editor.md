---
title: "Annotation editor"
description: "Eleven tools, five-colour palette, three-step stroke width, undo/redo — all native NSWindow, no Chromium tab in sight."
slug: "editor"
---

# Annotation editor

Click **Edit** on the Quick Tray (or send a capture into the editor any
other way) to open the native annotation editor. It's a real
`NSWindow` backed by a custom canvas — not a subprocess, not a
WebView, not Apple Preview.

## Tools

| Tool         | Shortcut | What it does                                              |
|--------------|----------|-----------------------------------------------------------|
| Pen          | `P`      | Freehand stroke                                           |
| Line         | `L`      | Straight line between two points                          |
| Arrow        | `A`      | Line with a calculated arrowhead at the end               |
| Rectangle    | `R`      | Stroked rectangle, drag-define bounds                     |
| Ellipse      | `E`      | Stroked ellipse, drag-define bounds                       |
| Highlighter  | `H`      | Translucent thick yellow stroke (over the image)          |
| Redact       | `X`      | Filled black rectangle — for hiding sensitive info        |
| Counter      | `N`      | Click drops a numbered circle (auto-increments)           |
| Text         | `T`      | Click prompts for text, places it at the click point      |
| Blur         | `B`      | Drag a rectangle; pixelates the underlying image inside   |
| Crop         | `C`      | Drag a rectangle; replaces the image with that sub-region |

## Colours

Five-colour palette in the toolbar (Red / Yellow / Green / Blue /
Black). Each annotation captures its colour at the moment you start
drawing, so changing colour later doesn't repaint old strokes.

The Highlighter pins to yellow-with-alpha; Redact pins to black.

## Stroke width

Three-step picker: **Thin** (3 px), **Med** (6 px), **Thick** (12 px).
Shortcuts `1` / `2` / `3`. The Highlighter overrides to a minimum
18 px so it reads like a real marker.

## Actions

| Shortcut | Action                                                  |
|----------|---------------------------------------------------------|
| `⌘S`     | Save — overwrites the original capture file with PNG    |
| `⌘C`     | Copy the annotated image to the clipboard               |
| `⌘Z`     | Undo last shape                                         |
| `⌘⇧Z`    | Redo                                                    |
| `⌘W`     | Close the editor (does not save)                        |
| Clear    | Removes every annotation (button only — no shortcut)    |

## Crop semantics

- The Crop rectangle is drawn as a yellow guide while you drag.
- On mouseUp, the image is replaced with the cropped sub-region, the
  window resizes to match the new aspect ratio, and existing
  annotations are cleared (their coordinates wouldn't line up with the
  new bounds anyway).
- Crop is *not* reversible via ⌘Z — undo affects annotations, not the
  underlying image. Plan accordingly, or re-capture if you mis-crop.

## How saving works

On `⌘S` the editor:

1. Allocates an `NSBitmapImageRep` at the source image's pixel dimensions.
2. Pushes a Quartz graphics context backed by that bitmap.
3. Draws the original `NSImage` filling the bitmap.
4. Maps every annotation from view coordinates to pixel coordinates
   (line widths scale by the larger axis) and paints it into the bitmap.
5. Encodes the bitmap as PNG and writes it over the source path.

The result is a single flat PNG that matches the editor view 1 : 1, at
the full original resolution — no quality loss from the bitmap re-encode.

## What's not in the editor yet

- Custom colour picker / hex input
- Layers panel / shape selection / move-existing
- Magic-arrow snap-to-UI-element heuristic

The Eyedropper hotkey (`⌃⌥⌘P`) covers most "pick a colour" needs
already — it puts the hex on the clipboard for use anywhere.
