// Annotation editor. Opens the captured image in an NSWindow with a custom
// canvas; supports Pen / Line / Arrow / Rectangle / Ellipse / Highlighter /
// Redact / Counter / Text / Blur tools, a five-colour palette, and a
// three-step stroke-width picker. ⌘S saves the annotated PNG over the
// original, ⌘C copies it to the clipboard, ⌘Z / ⌘⇧Z undo/redo, ⌘W closes.
// Toolbars mirror the keyboard shortcuts.
//
// Remaining tools planned: Crop, custom colour picker.

#[cfg(not(target_os = "macos"))]
pub fn open(_image_path: &std::path::Path) {}

#[cfg(target_os = "macos")]
pub use mac::open;

#[cfg(target_os = "macos")]
mod mac {
    #![allow(unused_unsafe)]

    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, sel, AllocAnyThread, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSBezelStyle,
        NSBezierPath, NSBitmapImageFileType, NSBitmapImageRep, NSButton, NSColor, NSEvent,
        NSGraphicsContext, NSImage, NSView, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{
        NSData, NSDictionary, NSObject, NSPoint, NSRect, NSSize, NSString, NSURL,
    };

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Tool {
        Pen,
        Line,
        Arrow,
        Rect,
        Ellipse,
        Highlight,
        Redact,
        Counter,
        Text,
        Blur,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Rgba {
        r: f64,
        g: f64,
        b: f64,
        a: f64,
    }
    impl Rgba {
        const RED: Rgba = Rgba {
            r: 1.0,
            g: 0.24,
            b: 0.33,
            a: 1.0,
        };
        const YELLOW: Rgba = Rgba {
            r: 1.0,
            g: 0.86,
            b: 0.18,
            a: 1.0,
        };
        const GREEN: Rgba = Rgba {
            r: 0.22,
            g: 0.78,
            b: 0.40,
            a: 1.0,
        };
        const BLUE: Rgba = Rgba {
            r: 0.22,
            g: 0.55,
            b: 0.95,
            a: 1.0,
        };
        const BLACK: Rgba = Rgba {
            r: 0.05,
            g: 0.05,
            b: 0.05,
            a: 1.0,
        };
        const HIGHLIGHT: Rgba = Rgba {
            r: 1.0,
            g: 0.92,
            b: 0.30,
            a: 0.35,
        };
    }

    #[derive(Clone, Debug)]
    enum Shape {
        Pen { points: Vec<NSPoint> },
        Line { from: NSPoint, to: NSPoint },
        Arrow { from: NSPoint, to: NSPoint },
        Rect { from: NSPoint, to: NSPoint },
        Ellipse { from: NSPoint, to: NSPoint },
        Counter { center: NSPoint, number: u32 },
        Text { origin: NSPoint, content: String },
        Blur { from: NSPoint, to: NSPoint },
    }

    /// A drawn shape plus its style. Captured at mouseDown so later changes
    /// to the active tool/color don't retroactively repaint prior strokes.
    #[derive(Clone, Debug)]
    struct Annotation {
        shape: Shape,
        color: Rgba,
        width: f64,
        fill: bool,
        arrowhead: bool,
    }

    impl Shape {
        fn from_tool(tool: Tool, start: NSPoint) -> Self {
            // Highlight and Redact are visually distinct (alpha + fill) but
            // geometrically map onto Pen and Rect respectively. Counter and
            // Text are point-anchored — content is set elsewhere.
            match tool {
                Tool::Pen | Tool::Highlight => Shape::Pen {
                    points: vec![start],
                },
                Tool::Line => Shape::Line {
                    from: start,
                    to: start,
                },
                Tool::Arrow => Shape::Arrow {
                    from: start,
                    to: start,
                },
                Tool::Rect | Tool::Redact => Shape::Rect {
                    from: start,
                    to: start,
                },
                Tool::Ellipse => Shape::Ellipse {
                    from: start,
                    to: start,
                },
                Tool::Counter => Shape::Counter {
                    center: start,
                    number: 0,
                },
                Tool::Text => Shape::Text {
                    origin: start,
                    content: String::new(),
                },
                Tool::Blur => Shape::Blur {
                    from: start,
                    to: start,
                },
            }
        }
        fn update(&mut self, point: NSPoint) {
            match self {
                Shape::Pen { points } => points.push(point),
                Shape::Line { to, .. }
                | Shape::Arrow { to, .. }
                | Shape::Rect { to, .. }
                | Shape::Ellipse { to, .. }
                | Shape::Blur { to, .. } => {
                    *to = point;
                }
                Shape::Counter { center, .. } => *center = point,
                Shape::Text { origin, .. } => *origin = point,
            }
        }
        fn is_meaningful(&self) -> bool {
            match self {
                Shape::Pen { points } => points.len() >= 2,
                Shape::Line { from, to }
                | Shape::Arrow { from, to }
                | Shape::Rect { from, to }
                | Shape::Ellipse { from, to }
                | Shape::Blur { from, to } => {
                    (from.x - to.x).abs() > 2.0 || (from.y - to.y).abs() > 2.0
                }
                Shape::Counter { .. } => true,
                Shape::Text { content, .. } => !content.is_empty(),
            }
        }
        fn scaled(&self, sx: f64, sy: f64) -> Self {
            let s = |p: &NSPoint| NSPoint {
                x: p.x * sx,
                y: p.y * sy,
            };
            match self {
                Shape::Pen { points } => Shape::Pen {
                    points: points.iter().map(s).collect(),
                },
                Shape::Line { from, to } => Shape::Line {
                    from: s(from),
                    to: s(to),
                },
                Shape::Arrow { from, to } => Shape::Arrow {
                    from: s(from),
                    to: s(to),
                },
                Shape::Rect { from, to } => Shape::Rect {
                    from: s(from),
                    to: s(to),
                },
                Shape::Ellipse { from, to } => Shape::Ellipse {
                    from: s(from),
                    to: s(to),
                },
                Shape::Counter { center, number } => Shape::Counter {
                    center: s(center),
                    number: *number,
                },
                Shape::Text { origin, content } => Shape::Text {
                    origin: s(origin),
                    content: content.clone(),
                },
                Shape::Blur { from, to } => Shape::Blur {
                    from: s(from),
                    to: s(to),
                },
            }
        }
    }

    impl Annotation {
        fn from_tool(tool: Tool, start: NSPoint, color: Rgba, width: f64) -> Self {
            // Highlight pins to its fixed semi-transparent yellow + thicker
            // stroke. Redact is always a filled black rect. Counter is a
            // filled coloured circle (the user's chosen colour, falling
            // back to red on Black to keep the white number readable).
            let (color, width, fill, arrowhead) = match tool {
                Tool::Highlight => (Rgba::HIGHLIGHT, width.max(18.0), false, false),
                Tool::Redact => (Rgba::BLACK, 0.0, true, false),
                Tool::Arrow => (color, width, false, true),
                Tool::Counter => (color, width, true, false),
                _ => (color, width, false, false),
            };
            Annotation {
                shape: Shape::from_tool(tool, start),
                color,
                width,
                fill,
                arrowhead,
            }
        }
        fn update(&mut self, point: NSPoint) {
            self.shape.update(point);
        }
        fn is_meaningful(&self) -> bool {
            self.shape.is_meaningful()
        }
        fn scaled(&self, sx: f64, sy: f64) -> Self {
            Annotation {
                shape: self.shape.scaled(sx, sy),
                color: self.color,
                // For filled redact rects there's no stroke; otherwise
                // scale width by the larger axis to match the on-screen look.
                width: if self.fill {
                    0.0
                } else {
                    self.width * sx.max(sy)
                },
                fill: self.fill,
                arrowhead: self.arrowhead,
            }
        }
    }

    struct EditorState {
        window: Retained<NSWindow>,
        canvas: Retained<CanvasView>,
        _handler: Retained<Handler>,
        image: Retained<NSImage>,
        image_pixel_size: NSSize,
        view_size: NSSize,
        tool: Tool,
        color: Rgba,
        width: f64,
        next_counter: u32,
        shapes: Vec<Annotation>,
        redo_stack: Vec<Annotation>,
        current: Option<Annotation>,
        source_path: PathBuf,
    }

    thread_local! {
        static EDITOR: RefCell<Option<EditorState>> = const { RefCell::new(None) };
    }

    pub fn open(image_path: &Path) {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("editor::open: not on main thread; skipping");
            return;
        };
        // Close any prior editor so we don't stack windows.
        EDITOR.with(|slot| *slot.borrow_mut() = None);

        match build(mtm, image_path) {
            Ok(state) => EDITOR.with(|slot| *slot.borrow_mut() = Some(state)),
            Err(err) => eprintln!("editor: build failed: {err}"),
        }
    }

    // ---- Custom NSView with drawing + key handling -----------------------

    define_class!(
        #[unsafe(super(NSView))]
        #[name = "STUCanvasView"]
        #[derive(Debug)]
        struct CanvasView;

        impl CanvasView {
            #[unsafe(method(acceptsFirstResponder))]
            fn accepts_first_responder(&self) -> bool {
                true
            }

            #[unsafe(method(drawRect:))]
            fn draw_rect(&self, _dirty: NSRect) {
                EDITOR.with(|slot| {
                    let Some(state) = slot.borrow().as_ref().map(|s| (
                        s.image.clone(),
                        s.shapes.clone(),
                        s.current.clone(),
                        s.view_size,
                    )) else { return };
                    let (image, shapes, current, view_size) = state;
                    let bounds = NSRect {
                        origin: NSPoint { x: 0.0, y: 0.0 },
                        size: view_size,
                    };
                    unsafe {
                        image.drawInRect(bounds);
                    }
                    paint_annotations(&shapes, current.as_ref());
                });
            }

            #[unsafe(method(mouseDown:))]
            fn mouse_down(&self, event: &NSEvent) {
                let location = unsafe { self.convertPoint_fromView(event.locationInWindow(), None) };
                let tool = EDITOR.with(|s| s.borrow().as_ref().map(|s| s.tool));
                match tool {
                    Some(Tool::Counter) => {
                        place_counter(location);
                        unsafe { self.setNeedsDisplay(true) };
                    }
                    Some(Tool::Text) => {
                        place_text(location);
                        unsafe { self.setNeedsDisplay(true) };
                    }
                    Some(_) => {
                        EDITOR.with(|slot| {
                            if let Some(state) = slot.borrow_mut().as_mut() {
                                state.redo_stack.clear();
                                state.current = Some(Annotation::from_tool(
                                    state.tool,
                                    location,
                                    state.color,
                                    state.width,
                                ));
                            }
                        });
                        unsafe { self.setNeedsDisplay(true) };
                    }
                    None => {}
                }
            }

            #[unsafe(method(mouseDragged:))]
            fn mouse_dragged(&self, event: &NSEvent) {
                let location = unsafe { self.convertPoint_fromView(event.locationInWindow(), None) };
                EDITOR.with(|slot| {
                    if let Some(state) = slot.borrow_mut().as_mut() {
                        if let Some(s) = state.current.as_mut() {
                            s.update(location);
                        }
                    }
                });
                unsafe { self.setNeedsDisplay(true) };
            }

            #[unsafe(method(mouseUp:))]
            fn mouse_up(&self, _event: &NSEvent) {
                EDITOR.with(|slot| {
                    if let Some(state) = slot.borrow_mut().as_mut() {
                        if let Some(s) = state.current.take() {
                            if s.is_meaningful() {
                                state.shapes.push(s);
                            }
                        }
                    }
                });
                unsafe { self.setNeedsDisplay(true) };
            }

            #[unsafe(method(keyDown:))]
            fn key_down(&self, event: &NSEvent) {
                let mods = unsafe { event.modifierFlags() };
                let cmd = mods.contains(objc2_app_kit::NSEventModifierFlags::Command);
                let shift = mods.contains(objc2_app_kit::NSEventModifierFlags::Shift);
                let chars = unsafe { event.charactersIgnoringModifiers() };
                let s = chars.map(|c| c.to_string()).unwrap_or_default();
                // Tool / colour shortcuts (no Cmd):
                //   tools: P pen, L line, A arrow, R rect, E ellipse,
                //          H highlight, X redact
                //   width: 1/2/3 = 3/6/12 px
                if !cmd {
                    match s.as_str() {
                        "p" | "P" => set_tool(Tool::Pen),
                        "l" | "L" => set_tool(Tool::Line),
                        "a" | "A" => set_tool(Tool::Arrow),
                        "r" | "R" => set_tool(Tool::Rect),
                        "e" | "E" => set_tool(Tool::Ellipse),
                        "h" | "H" => set_tool(Tool::Highlight),
                        "x" | "X" => set_tool(Tool::Redact),
                        "n" | "N" => set_tool(Tool::Counter),
                        "t" | "T" => set_tool(Tool::Text),
                        "b" | "B" => set_tool(Tool::Blur),
                        "1" => set_width(3.0),
                        "2" => set_width(6.0),
                        "3" => set_width(12.0),
                        _ => {}
                    }
                    return;
                }
                match s.as_str() {
                    "s" => save_to_disk(),
                    "c" => copy_to_clipboard(),
                    "z" if shift => redo_last(),
                    "z" => undo_last(),
                    "w" => close_editor(),
                    _ => {}
                }
            }
        }
    );

    // ---- Toolbar button dispatcher --------------------------------------

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "STUEditorHandler"]
        #[derive(Debug)]
        struct Handler;

        impl Handler {
            #[unsafe(method(buttonClicked:))]
            fn button_clicked(&self, sender: *mut AnyObject) {
                let tag: isize = unsafe { msg_send![sender, tag] };
                match tag {
                    // Tool picker
                    10 => set_tool(Tool::Pen),
                    11 => set_tool(Tool::Line),
                    12 => set_tool(Tool::Arrow),
                    13 => set_tool(Tool::Rect),
                    14 => set_tool(Tool::Ellipse),
                    15 => set_tool(Tool::Highlight),
                    16 => set_tool(Tool::Redact),
                    17 => set_tool(Tool::Counter),
                    18 => set_tool(Tool::Text),
                    19 => set_tool(Tool::Blur),
                    // Colour palette
                    20 => set_color(Rgba::RED),
                    21 => set_color(Rgba::YELLOW),
                    22 => set_color(Rgba::GREEN),
                    23 => set_color(Rgba::BLUE),
                    24 => set_color(Rgba::BLACK),
                    // Stroke width
                    30 => set_width(3.0),
                    31 => set_width(6.0),
                    32 => set_width(12.0),
                    // Actions
                    1 => save_to_disk(),
                    2 => copy_to_clipboard(),
                    3 => undo_last(),
                    4 => redo_last(),
                    5 => clear_shapes(),
                    6 => close_editor(),
                    _ => {}
                }
            }
        }
    );

    // ---- Annotation painting (called from drawRect:) -------------------

    fn paint_annotations(shapes: &[Annotation], current: Option<&Annotation>) {
        for a in shapes.iter().chain(current) {
            unsafe { paint_one(a) };
        }
    }

    unsafe fn paint_one(a: &Annotation) {
        let color =
            NSColor::colorWithSRGBRed_green_blue_alpha(a.color.r, a.color.g, a.color.b, a.color.a);
        if a.fill {
            color.setFill();
        } else {
            color.setStroke();
        }
        let path = NSBezierPath::bezierPath();
        path.setLineWidth(a.width);
        path.setLineCapStyle(objc2_app_kit::NSLineCapStyle::Round);
        path.setLineJoinStyle(objc2_app_kit::NSLineJoinStyle::Round);
        match &a.shape {
            Shape::Pen { points } => {
                if points.len() < 2 {
                    return;
                }
                path.moveToPoint(points[0]);
                for p in &points[1..] {
                    path.lineToPoint(*p);
                }
            }
            Shape::Line { from, to } => {
                path.moveToPoint(*from);
                path.lineToPoint(*to);
            }
            Shape::Arrow { from, to } => {
                path.moveToPoint(*from);
                path.lineToPoint(*to);
                if a.arrowhead {
                    // Arrowhead: two short lines at the tip.
                    let dx = to.x - from.x;
                    let dy = to.y - from.y;
                    let len = (dx * dx + dy * dy).sqrt().max(0.0001);
                    let ux = dx / len;
                    let uy = dy / len;
                    let head_len = (a.width * 5.0).max(12.0);
                    let cos_a = 0.5_f64.cos();
                    let sin_a = 0.5_f64.sin();
                    let left = NSPoint {
                        x: to.x - head_len * (ux * cos_a + uy * sin_a),
                        y: to.y - head_len * (uy * cos_a - ux * sin_a),
                    };
                    let right = NSPoint {
                        x: to.x - head_len * (ux * cos_a - uy * sin_a),
                        y: to.y - head_len * (uy * cos_a + ux * sin_a),
                    };
                    path.moveToPoint(*to);
                    path.lineToPoint(left);
                    path.moveToPoint(*to);
                    path.lineToPoint(right);
                }
            }
            Shape::Rect { from, to } => {
                let r = NSRect {
                    origin: NSPoint {
                        x: from.x.min(to.x),
                        y: from.y.min(to.y),
                    },
                    size: NSSize {
                        width: (to.x - from.x).abs(),
                        height: (to.y - from.y).abs(),
                    },
                };
                path.appendBezierPathWithRect(r);
            }
            Shape::Ellipse { from, to } => {
                let r = NSRect {
                    origin: NSPoint {
                        x: from.x.min(to.x),
                        y: from.y.min(to.y),
                    },
                    size: NSSize {
                        width: (to.x - from.x).abs(),
                        height: (to.y - from.y).abs(),
                    },
                };
                path.appendBezierPathWithOvalInRect(r);
            }
            Shape::Counter { center, number } => {
                let r = (a.width * 4.0).max(18.0);
                let circle_rect = NSRect {
                    origin: NSPoint {
                        x: center.x - r,
                        y: center.y - r,
                    },
                    size: NSSize {
                        width: r * 2.0,
                        height: r * 2.0,
                    },
                };
                // Filled coloured circle with a thin white border.
                let circle = NSBezierPath::bezierPath();
                circle.appendBezierPathWithOvalInRect(circle_rect);
                color.setFill();
                circle.fill();
                let white = NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 1.0, 1.0, 1.0);
                white.setStroke();
                circle.setLineWidth((a.width * 0.5).max(1.5));
                circle.stroke();
                // Number, white, centered on the circle.
                draw_text_centered(
                    &format!("{number}"),
                    *center,
                    (r * 1.1).max(12.0),
                    Rgba {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                );
                return; /* skip the trailing stroke/fill below */
            }
            Shape::Text { origin, content } => {
                let size = (a.width * 4.0).max(16.0);
                draw_text_at(content, *origin, size, a.color);
                return; /* text draws itself, no path stroke */
            }
            Shape::Blur { from, to } => {
                let dst = NSRect {
                    origin: NSPoint {
                        x: from.x.min(to.x),
                        y: from.y.min(to.y),
                    },
                    size: NSSize {
                        width: (to.x - from.x).abs(),
                        height: (to.y - from.y).abs(),
                    },
                };
                // Pixelate amount tied to the chosen width so the user can
                // make it chunkier with the Thick width button.
                let pixel = (a.width * 3.0).max(8.0);
                pixelate_blur(dst, pixel);
                return;
            }
        }
        if a.fill {
            path.fill();
        } else {
            path.stroke();
        }
    }

    /// Pixelate the canvas/output bitmap inside `dst`. Reads the source
    /// image from EDITOR state, computes the equivalent source rect via
    /// the current view-to-image scale, downsamples to a small bitmap,
    /// then redraws with nearest-neighbour interpolation. Works for both
    /// canvas painting (view coords) and save-time painting (pixel coords)
    /// because we read the source-to-view scale freshly each call.
    unsafe fn pixelate_blur(dst: NSRect, pixel: f64) {
        let (image, src_rect) = match EDITOR.with(|slot| {
            let s = slot.borrow();
            let state = s.as_ref()?;
            let sx = state.image_pixel_size.width / state.view_size.width.max(1.0);
            let sy = state.image_pixel_size.height / state.view_size.height.max(1.0);
            // `dst` may already be in pixel coords (during save) or view coords.
            // We discriminate by checking the active graphics context bounds — when
            // saving, the context's underlying NSBitmapImageRep has dimensions
            // matching the pixel size, so the destination rect we receive is in
            // pixel space.  At runtime the canvas is at view_size.  In both cases
            // we want source_rect in *image* pixel coords. So:
            let dst_in_image_coords = dst.size.width > state.view_size.width + 8.0
                || dst.size.height > state.view_size.height + 8.0;
            let src_rect = if dst_in_image_coords {
                // dst is already pixel coords
                dst
            } else {
                NSRect {
                    origin: NSPoint {
                        x: dst.origin.x * sx,
                        y: dst.origin.y * sy,
                    },
                    size: NSSize {
                        width: dst.size.width * sx,
                        height: dst.size.height * sy,
                    },
                }
            };
            Some((state.image.clone(), src_rect))
        }) {
            Some(pair) => pair,
            None => return,
        };

        // Build a small NSBitmapImageRep at dst_size / pixel.
        let small_w = (dst.size.width / pixel).max(2.0) as isize;
        let small_h = (dst.size.height / pixel).max(2.0) as isize;
        let small: Retained<NSBitmapImageRep> = match NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            small_w,
            small_h,
            8,
            4,
            true,
            false,
            objc2_app_kit::NSDeviceRGBColorSpace,
            objc2_app_kit::NSBitmapFormat::empty(),
            0,
            0,
        ) {
            Some(r) => r,
            None => return,
        };
        let small_ctx = match NSGraphicsContext::graphicsContextWithBitmapImageRep(&small) {
            Some(c) => c,
            None => return,
        };

        // Draw the source image's src_rect into the small bitmap.
        NSGraphicsContext::saveGraphicsState_class();
        NSGraphicsContext::setCurrentContext(Some(&small_ctx));
        let dest_in_small = NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: NSSize {
                width: small_w as f64,
                height: small_h as f64,
            },
        };
        image.drawInRect_fromRect_operation_fraction(
            dest_in_small,
            src_rect,
            objc2_app_kit::NSCompositingOperation::SourceOver,
            1.0,
        );
        NSGraphicsContext::restoreGraphicsState_class();

        // Now draw the small bitmap into dst with no interpolation.
        if let Some(cur) = NSGraphicsContext::currentContext() {
            cur.saveGraphicsState();
            cur.setImageInterpolation(objc2_app_kit::NSImageInterpolation::None);
            small.drawInRect(dst);
            cur.restoreGraphicsState();
        }
    }

    /// Build an NSDictionary holding NSFont + NSColor attributes for
    /// NSString's drawing methods. Returned as a raw `*const AnyObject` so
    /// it can be handed directly to `msg_send!` without wrestling with
    /// the strict generic parameters on the typed binding.
    unsafe fn make_text_attributes(font_size: f64, color: Rgba, bold: bool) -> Retained<NSObject> {
        let font = if bold {
            objc2_app_kit::NSFont::boldSystemFontOfSize(font_size)
        } else {
            objc2_app_kit::NSFont::systemFontOfSize(font_size)
        };
        let color_obj =
            NSColor::colorWithSRGBRed_green_blue_alpha(color.r, color.g, color.b, color.a);
        // Build NSDictionary via the class-level constructor that takes
        // parallel keys/values arrays. Skipping the typed binding lets us
        // mix NSFont + NSColor as values.
        let keys: [&NSObject; 2] = [
            &*NSString::from_str("NSFont") as &NSObject,
            &*NSString::from_str("NSColor") as &NSObject,
        ];
        let vals: [&NSObject; 2] = [&*font as &NSObject, &*color_obj as &NSObject];
        let dict: Retained<NSObject> = msg_send![
            objc2::class!(NSDictionary),
            dictionaryWithObjects: vals.as_ptr(),
            forKeys: keys.as_ptr(),
            count: 2usize,
        ];
        dict
    }

    /// Draw a single line of text at `origin`. Used by the Text tool.
    unsafe fn draw_text_at(s: &str, origin: NSPoint, font_size: f64, color: Rgba) {
        if MainThreadMarker::new().is_none() {
            return;
        }
        let ns = NSString::from_str(s);
        let attrs = make_text_attributes(font_size, color, false);
        let _: () = msg_send![&*ns, drawAtPoint: origin, withAttributes: &*attrs];
    }

    /// Draw `s` centred (horizontally + vertically) on `center`.
    unsafe fn draw_text_centered(s: &str, center: NSPoint, font_size: f64, color: Rgba) {
        if MainThreadMarker::new().is_none() {
            return;
        }
        let ns = NSString::from_str(s);
        let attrs = make_text_attributes(font_size, color, true);
        let size: NSSize = msg_send![&*ns, sizeWithAttributes: &*attrs];
        let origin = NSPoint {
            x: center.x - size.width / 2.0,
            y: center.y - size.height / 2.0,
        };
        let _: () = msg_send![&*ns, drawAtPoint: origin, withAttributes: &*attrs];
    }

    // ---- Actions --------------------------------------------------------

    fn redraw() {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow().as_ref() {
                unsafe { state.canvas.setNeedsDisplay(true) };
            }
        });
    }

    fn undo_last() {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                if let Some(shape) = state.shapes.pop() {
                    state.redo_stack.push(shape);
                }
            }
        });
        redraw();
    }

    fn redo_last() {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                if let Some(shape) = state.redo_stack.pop() {
                    state.shapes.push(shape);
                }
            }
        });
        redraw();
    }

    fn clear_shapes() {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                state.shapes.clear();
                state.redo_stack.clear();
                state.current = None;
            }
        });
        redraw();
    }

    fn set_tool(tool: Tool) {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                state.tool = tool;
            }
        });
    }

    fn set_color(color: Rgba) {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                state.color = color;
            }
        });
    }

    fn set_width(width: f64) {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                state.width = width;
            }
        });
    }

    /// Place a numbered counter circle at `point` and commit immediately
    /// (counters are click-to-place, not drag).
    fn place_counter(point: NSPoint) {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                state.redo_stack.clear();
                let n = state.next_counter;
                state.next_counter += 1;
                let mut a = Annotation::from_tool(Tool::Counter, point, state.color, state.width);
                if let Shape::Counter { number, .. } = &mut a.shape {
                    *number = n;
                }
                state.shapes.push(a);
            }
        });
    }

    /// Prompt the user for a string, then place a text annotation at `point`.
    /// Skipped if the user cancels or enters an empty string.
    fn place_text(point: NSPoint) {
        let Some(content) = prompt_for_text() else {
            return;
        };
        if content.is_empty() {
            return;
        }
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                state.redo_stack.clear();
                let mut a = Annotation::from_tool(Tool::Text, point, state.color, state.width);
                if let Shape::Text { content: c, .. } = &mut a.shape {
                    *c = content;
                }
                state.shapes.push(a);
            }
        });
    }

    /// Modal text-input prompt via NSAlert + NSTextField. Returns Some(string)
    /// on OK, None on cancel.
    fn prompt_for_text() -> Option<String> {
        let mtm = MainThreadMarker::new()?;
        unsafe {
            let alert: Retained<objc2_app_kit::NSAlert> =
                msg_send![objc2_app_kit::NSAlert::alloc(mtm), init];
            alert.setMessageText(&NSString::from_str("Add text annotation"));
            alert.setInformativeText(&NSString::from_str("Enter the text to draw on the image."));
            alert.addButtonWithTitle(&NSString::from_str("Add"));
            alert.addButtonWithTitle(&NSString::from_str("Cancel"));
            let field: Retained<objc2_app_kit::NSTextField> =
                objc2_app_kit::NSTextField::initWithFrame(
                    objc2_app_kit::NSTextField::alloc(mtm),
                    NSRect {
                        origin: NSPoint { x: 0.0, y: 0.0 },
                        size: NSSize {
                            width: 280.0,
                            height: 24.0,
                        },
                    },
                );
            field.setPlaceholderString(Some(&NSString::from_str("Your text")));
            alert.setAccessoryView(Some(&field));
            let resp = alert.runModal();
            if resp == 1000 {
                // First button (Add) = NSAlertFirstButtonReturn = 1000
                let s = field.stringValue();
                Some(s.to_string())
            } else {
                None
            }
        }
    }

    fn close_editor() {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow_mut().take() {
                unsafe {
                    state.window.orderOut(None);
                    state.window.close();
                }
            }
        });
    }

    /// Render the original image + all strokes into a PNG at the source
    /// path. Strokes drawn in view coordinates are mapped to image pixel
    /// coordinates so the saved PNG matches what the user sees.
    fn save_to_disk() {
        let data = render_annotated_png();
        let path = EDITOR.with(|slot| slot.borrow().as_ref().map(|s| s.source_path.clone()));
        let (Some(data), Some(path)) = (data, path) else {
            return;
        };
        if let Err(err) = std::fs::write(&path, &data) {
            eprintln!("editor: save failed: {err}");
            return;
        }
        crate::logging::event(serde_json::json!({
            "evt": "editor_save",
            "path": path.display().to_string(),
            "bytes": data.len(),
        }));
        eprintln!("editor: saved {} ({} bytes)", path.display(), data.len());
    }

    fn copy_to_clipboard() {
        let data = render_annotated_png();
        let path = EDITOR.with(|slot| slot.borrow().as_ref().map(|s| s.source_path.clone()));
        let (Some(data), Some(path)) = (data, path) else {
            return;
        };
        // Write to a temp file beside the original, then reuse the existing
        // clipboard helper (which handles every macOS clipboard quirk).
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();
        let tmp = std::env::temp_dir().join(format!("screenshot-ultra-edit-{stamp}.png"));
        if std::fs::write(&tmp, &data).is_err() {
            return;
        }
        if let Err(err) = crate::sinks::clipboard_copy_image(&tmp) {
            eprintln!("editor: copy failed: {err:#}");
            return;
        }
        crate::logging::event(serde_json::json!({
            "evt": "editor_copy",
            "from": path.display().to_string(),
        }));
    }

    /// Returns the annotated image as PNG bytes, or None if there's no
    /// active editor or rendering failed.
    fn render_annotated_png() -> Option<Vec<u8>> {
        let mtm = MainThreadMarker::new()?;
        EDITOR.with(|slot| {
            let state_ref = slot.borrow();
            let state = state_ref.as_ref()?;
            let pixel_size = state.image_pixel_size;
            let view_size = state.view_size;
            let shapes = state.shapes.clone();
            let current = state.current.clone();
            let image = state.image.clone();
            drop(state_ref);

            unsafe {
                // Create a bitmap rep at the original pixel size.
                let bitmap = NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
                    NSBitmapImageRep::alloc(),
                    std::ptr::null_mut(),
                    pixel_size.width as isize,
                    pixel_size.height as isize,
                    8,
                    4,
                    true,
                    false,
                    objc2_app_kit::NSDeviceRGBColorSpace,
                    objc2_app_kit::NSBitmapFormat::empty(),
                    0,
                    0,
                )?;

                // Make a graphics context backed by the bitmap and push it.
                let ctx = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap)?;
                NSGraphicsContext::saveGraphicsState_class();
                NSGraphicsContext::setCurrentContext(Some(&ctx));

                // Draw the original image filling the bitmap.
                let dest = NSRect {
                    origin: NSPoint { x: 0.0, y: 0.0 },
                    size: pixel_size,
                };
                image.drawInRect(dest);

                // Map view coords → pixel coords and paint annotations.
                let sx = pixel_size.width / view_size.width.max(1.0);
                let sy = pixel_size.height / view_size.height.max(1.0);
                let scaled: Vec<Annotation> = shapes
                    .iter()
                    .chain(current.as_ref())
                    .map(|a| a.scaled(sx, sy))
                    .collect();
                paint_annotations(&scaled, None);

                NSGraphicsContext::restoreGraphicsState_class();
                let _ = mtm; // suppress unused-binding warning under cfg variations

                // Encode as PNG. The properties dictionary is empty for PNG;
                // the API takes &NSDictionary<NSString, AnyObject>.
                let props = NSDictionary::<NSString, AnyObject>::new();
                let png =
                    bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &props)?;
                Some(ns_data_to_vec(&png))
            }
        })
    }

    fn ns_data_to_vec(data: &NSData) -> Vec<u8> {
        // objc2-foundation's NSData provides to_vec() on stable. Use it.
        data.to_vec()
    }

    // ---- Window construction --------------------------------------------

    const TOOLBAR_H: f64 = 40.0;
    const TOOLPICKER_H: f64 = 72.0; /* two rows: tools + (colors + width) */

    fn build(mtm: MainThreadMarker, image_path: &Path) -> Result<EditorState, String> {
        let app = NSApplication::sharedApplication(mtm);
        unsafe {
            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
            app.activate();
        }

        let image = load_image(image_path).ok_or_else(|| "failed to load image".to_string())?;
        let pixel_size = unsafe { image.size() };

        // Fit a window into ≤80% of the main screen, preserving aspect ratio.
        let screen_size = unsafe {
            objc2_app_kit::NSScreen::mainScreen(mtm)
                .map(|s| s.frame().size)
                .unwrap_or(NSSize {
                    width: 1280.0,
                    height: 800.0,
                })
        };
        let chrome_h = TOOLBAR_H + TOOLPICKER_H;
        let max_w = screen_size.width * 0.8;
        let max_h = (screen_size.height * 0.8) - chrome_h;
        let scale = (max_w / pixel_size.width)
            .min(max_h / pixel_size.height)
            .min(1.0);
        let view_w = (pixel_size.width * scale).max(420.0);
        let view_h = (pixel_size.height * scale).max(200.0);

        let win_rect = NSRect {
            origin: NSPoint { x: 120.0, y: 120.0 },
            size: NSSize {
                width: view_w,
                height: view_h + chrome_h,
            },
        };

        let style = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Resizable
            | NSWindowStyleMask::Miniaturizable;
        let window: Retained<NSWindow> = unsafe {
            let alloc = NSWindow::alloc(mtm);
            msg_send![
                alloc,
                initWithContentRect: win_rect,
                styleMask: style,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };
        unsafe {
            window.setReleasedWhenClosed(false);
            let title = NSString::from_str(&format!(
                "Annotate — {}",
                image_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled")
            ));
            window.setTitle(&title);
        }

        // Canvas sits above the bottom action bar and below the top tool
        // picker.
        let canvas_rect = NSRect {
            origin: NSPoint {
                x: 0.0,
                y: TOOLBAR_H,
            },
            size: NSSize {
                width: view_w,
                height: view_h,
            },
        };
        let canvas: Retained<CanvasView> = unsafe {
            let alloc = CanvasView::alloc(mtm);
            msg_send![alloc, initWithFrame: canvas_rect]
        };
        unsafe {
            canvas.setAutoresizingMask(
                objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
                    | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
            );
        }

        let handler: Retained<Handler> = unsafe { msg_send![Handler::alloc(), init] };

        let make_button = |label: &str, tag: isize, x: f64, y: f64, w: f64, h: f64| {
            let title = NSString::from_str(label);
            unsafe {
                let rect = NSRect {
                    origin: NSPoint { x, y },
                    size: NSSize {
                        width: w,
                        height: h,
                    },
                };
                let b: Retained<NSButton> = NSButton::initWithFrame(NSButton::alloc(mtm), rect);
                b.setTitle(&title);
                b.setBezelStyle(NSBezelStyle::Push);
                b.setTag(tag);
                b.setTarget(Some(handler.as_ref()));
                b.setAction(Some(sel!(buttonClicked:)));
                b
            }
        };

        if let Some(content_view) = window.contentView() {
            unsafe { content_view.addSubview(&canvas) };

            // Top: tool picker (row 1) + colour palette & width (row 2).
            let row_y_top = TOOLBAR_H + view_h + 40.0;
            let row_y_bot = TOOLBAR_H + view_h + 8.0;

            let tools = [
                ("Pen P", 10),
                ("Line L", 11),
                ("Arrow A", 12),
                ("Rect R", 13),
                ("Ellipse E", 14),
                ("Hilite H", 15),
                ("Redact X", 16),
                ("# N", 17),
                ("Text T", 18),
                ("Blur B", 19),
            ];
            let tool_btn_w = 62.0;
            let tool_btn_h = 24.0;
            let tool_gap = 4.0;
            let tool_total =
                (tool_btn_w * tools.len() as f64) + (tool_gap * (tools.len() as f64 - 1.0));
            let tool_start_x = (view_w - tool_total).max(8.0) / 2.0;
            for (i, (label, tag)) in tools.iter().enumerate() {
                let btn = make_button(
                    label,
                    *tag,
                    tool_start_x + (tool_btn_w + tool_gap) * i as f64,
                    row_y_top,
                    tool_btn_w,
                    tool_btn_h,
                );
                unsafe { content_view.addSubview(&btn) };
            }

            // Colour swatches + width on row 2.
            let palette = [
                ("● Red", 20),
                ("● Yellow", 21),
                ("● Green", 22),
                ("● Blue", 23),
                ("● Black", 24),
            ];
            let widths = [("Thin 1", 30), ("Med 2", 31), ("Thick 3", 32)];
            let p_w = 70.0;
            let w_w = 60.0;
            let gap2 = 4.0;
            let row2_total = (p_w * palette.len() as f64)
                + (gap2 * (palette.len() as f64 - 1.0))
                + 16.0
                + (w_w * widths.len() as f64)
                + (gap2 * (widths.len() as f64 - 1.0));
            let mut x = (view_w - row2_total).max(8.0) / 2.0;
            for (label, tag) in palette.iter() {
                let btn = make_button(label, *tag, x, row_y_bot, p_w, tool_btn_h);
                unsafe { content_view.addSubview(&btn) };
                x += p_w + gap2;
            }
            x += 16.0;
            for (label, tag) in widths.iter() {
                let btn = make_button(label, *tag, x, row_y_bot, w_w, tool_btn_h);
                unsafe { content_view.addSubview(&btn) };
                x += w_w + gap2;
            }

            // Bottom action row.
            let actions = [
                ("Save ⌘S", 1),
                ("Copy ⌘C", 2),
                ("Undo ⌘Z", 3),
                ("Redo ⌘⇧Z", 4),
                ("Clear", 5),
                ("Done ⌘W", 6),
            ];
            let act_btn_w = 88.0;
            let act_btn_h = 24.0;
            let act_gap = 4.0;
            let act_total =
                (act_btn_w * actions.len() as f64) + (act_gap * (actions.len() as f64 - 1.0));
            let act_start_x = (view_w - act_total) / 2.0;
            let act_y = (TOOLBAR_H - act_btn_h) / 2.0;
            for (i, (label, tag)) in actions.iter().enumerate() {
                let btn = make_button(
                    label,
                    *tag,
                    act_start_x + (act_btn_w + act_gap) * i as f64,
                    act_y,
                    act_btn_w,
                    act_btn_h,
                );
                unsafe { content_view.addSubview(&btn) };
            }
        }

        unsafe {
            window.makeFirstResponder(Some(&canvas));
            window.makeKeyAndOrderFront(None);
            window.display();
        }

        Ok(EditorState {
            window,
            canvas,
            _handler: handler,
            image,
            image_pixel_size: pixel_size,
            view_size: NSSize {
                width: view_w,
                height: view_h,
            },
            tool: Tool::Pen,
            color: Rgba::RED,
            width: 3.0,
            next_counter: 1,
            shapes: Vec::new(),
            redo_stack: Vec::new(),
            current: None,
            source_path: image_path.to_path_buf(),
        })
    }

    fn load_image(path: &Path) -> Option<Retained<NSImage>> {
        unsafe {
            let s = NSString::from_str(&path.to_string_lossy());
            let url = NSURL::fileURLWithPath(&s);
            let img: Option<Retained<NSImage>> =
                msg_send![NSImage::alloc(), initWithContentsOfURL: &*url];
            img
        }
    }
}
