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

    /// A single freehand stroke.
    #[derive(Clone)]
    struct Stroke {
        points: Vec<NSPoint>,
    }

    struct EditorState {
        window: Retained<NSWindow>,
        canvas: Retained<CanvasView>,
        _handler: Retained<Handler>,
        image: Retained<NSImage>,
        image_pixel_size: NSSize, /* pixel size of the source image */
        view_size: NSSize,        /* size of the canvas view */
        strokes: Vec<Stroke>,
        current: Option<Stroke>,
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
                        s.strokes.clone(),
                        s.current.clone(),
                        s.view_size,
                    )) else { return };
                    let (image, strokes, current, view_size) = state;
                    let bounds = NSRect {
                        origin: NSPoint { x: 0.0, y: 0.0 },
                        size: view_size,
                    };
                    unsafe {
                        // Background — fill with the image, scaled to fit.
                        image.drawInRect(bounds);
                    }
                    paint_strokes(&strokes, current.as_ref());
                });
            }

            #[unsafe(method(mouseDown:))]
            fn mouse_down(&self, event: &NSEvent) {
                let location = unsafe { self.convertPoint_fromView(event.locationInWindow(), None) };
                EDITOR.with(|slot| {
                    if let Some(state) = slot.borrow_mut().as_mut() {
                        state.current = Some(Stroke {
                            points: vec![location],
                        });
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
                            s.points.push(location);
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
                            if s.points.len() >= 2 {
                                state.strokes.push(s);
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
                let chars = unsafe { event.charactersIgnoringModifiers() };
                let s = chars.map(|c| c.to_string()).unwrap_or_default();
                if !cmd {
                    return;
                }
                match s.as_str() {
                    "s" => save_to_disk(),
                    "c" => copy_to_clipboard(),
                    "z" => undo_last_stroke(),
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
                    1 => save_to_disk(),
                    2 => copy_to_clipboard(),
                    3 => undo_last_stroke(),
                    4 => clear_strokes(),
                    5 => close_editor(),
                    _ => {}
                }
            }
        }
    );

    // ---- Stroke painting (called from drawRect:) ------------------------

    fn paint_strokes(strokes: &[Stroke], current: Option<&Stroke>) {
        unsafe {
            let red = NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 0.24, 0.33, 1.0);
            red.setStroke();
            for s in strokes.iter().chain(current) {
                if s.points.len() < 2 {
                    continue;
                }
                let path = NSBezierPath::bezierPath();
                path.setLineWidth(3.0);
                path.setLineCapStyle(objc2_app_kit::NSLineCapStyle::Round);
                path.setLineJoinStyle(objc2_app_kit::NSLineJoinStyle::Round);
                path.moveToPoint(s.points[0]);
                for p in &s.points[1..] {
                    path.lineToPoint(*p);
                }
                path.stroke();
            }
        }
    }

    // ---- Actions --------------------------------------------------------

    fn redraw() {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow().as_ref() {
                unsafe { state.canvas.setNeedsDisplay(true) };
            }
        });
    }

    fn undo_last_stroke() {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                state.strokes.pop();
            }
        });
        redraw();
    }

    fn clear_strokes() {
        EDITOR.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                state.strokes.clear();
                state.current = None;
            }
        });
        redraw();
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
            let strokes = state.strokes.clone();
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

                // Map view coords → pixel coords and paint strokes.
                let sx = pixel_size.width / view_size.width.max(1.0);
                let sy = pixel_size.height / view_size.height.max(1.0);
                let scaled: Vec<Stroke> = strokes
                    .iter()
                    .chain(current.as_ref())
                    .map(|s| Stroke {
                        points: s
                            .points
                            .iter()
                            .map(|p| NSPoint {
                                x: p.x * sx,
                                y: p.y * sy,
                            })
                            .collect(),
                    })
                    .collect();
                // Scale the line width too so it looks the same on the
                // saved image as it did in the editor.
                paint_strokes_with_width(&scaled, sx.max(sy) * 3.0);

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

    fn paint_strokes_with_width(strokes: &[Stroke], width: f64) {
        unsafe {
            let red = NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 0.24, 0.33, 1.0);
            red.setStroke();
            for s in strokes {
                if s.points.len() < 2 {
                    continue;
                }
                let path = NSBezierPath::bezierPath();
                path.setLineWidth(width);
                path.setLineCapStyle(objc2_app_kit::NSLineCapStyle::Round);
                path.setLineJoinStyle(objc2_app_kit::NSLineJoinStyle::Round);
                path.moveToPoint(s.points[0]);
                for p in &s.points[1..] {
                    path.lineToPoint(*p);
                }
                path.stroke();
            }
        }
    }

    fn ns_data_to_vec(data: &NSData) -> Vec<u8> {
        // objc2-foundation's NSData provides to_vec() on stable. Use it.
        data.to_vec()
    }

    // ---- Window construction --------------------------------------------

    const TOOLBAR_H: f64 = 36.0;

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
        let max_w = screen_size.width * 0.8;
        let max_h = (screen_size.height * 0.8) - TOOLBAR_H;
        let scale = (max_w / pixel_size.width)
            .min(max_h / pixel_size.height)
            .min(1.0);
        let view_w = (pixel_size.width * scale).max(320.0);
        let view_h = (pixel_size.height * scale).max(200.0);

        let win_rect = NSRect {
            origin: NSPoint { x: 120.0, y: 120.0 },
            size: NSSize {
                width: view_w,
                height: view_h + TOOLBAR_H,
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

        // Canvas
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

        // Toolbar buttons
        let handler: Retained<Handler> = unsafe { msg_send![Handler::alloc(), init] };
        let labels = ["Save (⌘S)", "Copy (⌘C)", "Undo (⌘Z)", "Clear", "Done (⌘W)"];
        let tags: [isize; 5] = [1, 2, 3, 4, 5];
        let btn_w = 110.0;
        let btn_h = 24.0;
        let gap = 4.0;
        let total = (btn_w * labels.len() as f64) + (gap * (labels.len() as f64 - 1.0));
        let start_x = (view_w - total) / 2.0;
        let y = (TOOLBAR_H - btn_h) / 2.0;

        if let Some(content_view) = window.contentView() {
            unsafe { content_view.addSubview(&canvas) };
            for (i, label) in labels.iter().enumerate() {
                let rect = NSRect {
                    origin: NSPoint {
                        x: start_x + (btn_w + gap) * i as f64,
                        y,
                    },
                    size: NSSize {
                        width: btn_w,
                        height: btn_h,
                    },
                };
                let title = NSString::from_str(label);
                let btn: Retained<NSButton> = unsafe {
                    let b = NSButton::initWithFrame(NSButton::alloc(mtm), rect);
                    b.setTitle(&title);
                    b.setBezelStyle(NSBezelStyle::Push);
                    b.setTag(tags[i]);
                    b.setTarget(Some(handler.as_ref()));
                    b.setAction(Some(sel!(buttonClicked:)));
                    b
                };
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
            strokes: Vec::new(),
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
