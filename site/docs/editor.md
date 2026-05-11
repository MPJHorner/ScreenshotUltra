# Annotation editor

Click **Edit** on the Quick Tray (or send a capture into the editor any
other way) to open the native annotation editor. It's a real `NSWindow`
backed by a custom canvas — not a subprocess, not a web view, not
Preview.

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

## Colours

Five-colour palette in the toolbar (Red / Yellow / Green / Blue / Black).
Each annotation captures its colour at the moment you start drawing, so
changing colour later doesn't repaint old strokes.

The Highlighter pins to yellow with alpha; Redact pins to black.

## Stroke width

Three-step picker: **Thin** (3 px), **Med** (6 px), **Thick** (12 px).
Shortcuts `1` / `2` / `3`. The Highlighter overrides to a minimum 18 px.

## Keyboard actions

| Shortcut | Action                                                  |
|----------|---------------------------------------------------------|
| `⌘S`     | Save — overwrites the original capture file with PNG    |
| `⌘C`     | Copy the annotated image to the clipboard               |
| `⌘Z`     | Undo last shape                                         |
| `⌘⇧Z`    | Redo                                                    |
| `⌘W`     | Close the editor (does not save)                        |
| Clear    | Removes every annotation (button only — no shortcut)    |

## How saving works

On `⌘S` the editor:

1. Allocates an `NSBitmapImageRep` at the source image's pixel dimensions.
2. Pushes a Quartz graphics context backed by that bitmap.
3. Draws the original `NSImage` filling the bitmap.
4. Maps every annotation from view coordinates to pixel coordinates
   (line widths scale by the larger axis) and paints it into the bitmap.
5. Encodes the bitmap as PNG and writes it over the source path.

The result is a single flat PNG that matches the editor view 1:1, at the
full original resolution — no quality loss from the bitmap re-encode.

## What's not in the editor yet

- Blur tool (Core Image gaussian blur on a selection)
- Crop tool
- Custom colour picker / hex input
- Layers panel / shape selection / move-existing
- Magic-arrow snap-to-UI-element heuristic

These are the remaining M2 items. PRs welcome.
