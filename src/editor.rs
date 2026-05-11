// Annotation editor MVP. Opens the captured image in an NSWindow with a
// custom canvas view on top; pen tool draws red strokes; ⌘S saves the
// annotated PNG over the original, ⌘C copies to the clipboard, ⌘Z undoes
// the last stroke, ⌘⌫ clears all strokes. A toolbar at the bottom mirrors
// the keyboard shortcuts.
//
// Scope: single tool (freehand pen) with one color. More tools (arrow,
// rectangle, ellipse, text, blur, crop, color picker, stroke width) land
// in follow-up commits.

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
        Arrow,
        Rect,
        Ellipse,
    }

    #[derive(Clone, Debug)]
    enum Shape {
        Pen { points: Vec<NSPoint> },
        Arrow { from: NSPoint, to: NSPoint },
        Rect { from: NSPoint, to: NSPoint },
        Ellipse { from: NSPoint, to: NSPoint },
    }

    impl Shape {
        fn from_tool(tool: Tool, start: NSPoint) -> Self {
            match tool {
                Tool::Pen => Shape::Pen {
                    points: vec![start],
                },
                Tool::Arrow => Shape::Arrow {
                    from: start,
                    to: start,
                },
                Tool::Rect => Shape::Rect {
                    from: start,
                    to: start,
                },
                Tool::Ellipse => Shape::Ellipse {
                    from: start,
                    to: start,
                },
            }
        }
        fn update(&mut self, point: NSPoint) {
            match self {
                Shape::Pen { points } => points.push(point),
                Shape::Arrow { to, .. } | Shape::Rect { to, .. } | Shape::Ellipse { to, .. } => {
                    *to = point;
                }
            }
        }
        fn is_meaningful(&self) -> bool {
            match self {
                Shape::Pen { points } => points.len() >= 2,
                Shape::Arrow { from, to }
                | Shape::Rect { from, to }
                | Shape::Ellipse { from, to } => {
                    (from.x - to.x).abs() > 2.0 || (from.y - to.y).abs() > 2.0
                }
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
        shapes: Vec<Shape>,
        redo_stack: Vec<Shape>,
        current: Option<Shape>,
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
                    paint_shapes(&shapes, current.as_ref(), 3.0);
                });
            }

            #[unsafe(method(mouseDown:))]
            fn mouse_down(&self, event: &NSEvent) {
                let location = unsafe { self.convertPoint_fromView(event.locationInWindow(), None) };
                EDITOR.with(|slot| {
                    if let Some(state) = slot.borrow_mut().as_mut() {
                        state.redo_stack.clear();
                        state.current = Some(Shape::from_tool(state.tool, location));
                    }
                });
                unsafe { self.setNeedsDisplay(true) };
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
                // Tool shortcuts (no Cmd): P/A/R/E
                if !cmd {
                    match s.as_str() {
                        "p" | "P" => set_tool(Tool::Pen),
                        "a" | "A" => set_tool(Tool::Arrow),
                        "r" | "R" => set_tool(Tool::Rect),
                        "e" | "E" => set_tool(Tool::Ellipse),
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
                    11 => set_tool(Tool::Arrow),
                    12 => set_tool(Tool::Rect),
                    13 => set_tool(Tool::Ellipse),
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

    // ---- Shape painting (called from drawRect:) ------------------------

    fn paint_shapes(shapes: &[Shape], current: Option<&Shape>, width: f64) {
        unsafe {
            let red = NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 0.24, 0.33, 1.0);
            red.setStroke();
            for shape in shapes.iter().chain(current) {
                paint_one(shape, width);
            }
        }
    }

    unsafe fn paint_one(shape: &Shape, width: f64) {
        let path = NSBezierPath::bezierPath();
        path.setLineWidth(width);
        path.setLineCapStyle(objc2_app_kit::NSLineCapStyle::Round);
        path.setLineJoinStyle(objc2_app_kit::NSLineJoinStyle::Round);
        match shape {
            Shape::Pen { points } => {
                if points.len() < 2 {
                    return;
                }
                path.moveToPoint(points[0]);
                for p in &points[1..] {
                    path.lineToPoint(*p);
                }
            }
            Shape::Arrow { from, to } => {
                path.moveToPoint(*from);
                path.lineToPoint(*to);
                // Arrowhead: two short lines at the tip.
                let dx = to.x - from.x;
                let dy = to.y - from.y;
                let len = (dx * dx + dy * dy).sqrt().max(0.0001);
                let ux = dx / len;
                let uy = dy / len;
                let head_len = (width * 5.0).max(12.0);
                let head_angle = 0.5_f64; /* ~28.6 degrees */
                let cos_a = head_angle.cos();
                let sin_a = head_angle.sin();
                // Two perpendicular rotated unit vectors pointing back from `to`.
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
        }
        path.stroke();
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

                // Map view coords → pixel coords and paint shapes.
                let sx = pixel_size.width / view_size.width.max(1.0);
                let sy = pixel_size.height / view_size.height.max(1.0);
                let scaled: Vec<Shape> = shapes
                    .iter()
                    .chain(current.as_ref())
                    .map(|s| s.scaled(sx, sy))
                    .collect();
                // Scale the line width too so the saved image matches what
                // the user saw in the editor.
                paint_shapes(&scaled, None, sx.max(sy) * 3.0);

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
    const TOOLPICKER_H: f64 = 32.0;

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

            // Top tool-picker row (above the canvas).
            let tools = [
                ("Pen (P)", 10),
                ("Arrow (A)", 11),
                ("Rect (R)", 12),
                ("Ellipse (E)", 13),
            ];
            let tool_btn_w = 88.0;
            let tool_btn_h = 24.0;
            let tool_gap = 4.0;
            let tool_total =
                (tool_btn_w * tools.len() as f64) + (tool_gap * (tools.len() as f64 - 1.0));
            let tool_start_x = (view_w - tool_total) / 2.0;
            let tool_y = TOOLBAR_H + view_h + (TOOLPICKER_H - tool_btn_h) / 2.0;
            for (i, (label, tag)) in tools.iter().enumerate() {
                let btn = make_button(
                    label,
                    *tag,
                    tool_start_x + (tool_btn_w + tool_gap) * i as f64,
                    tool_y,
                    tool_btn_w,
                    tool_btn_h,
                );
                unsafe { content_view.addSubview(&btn) };
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
