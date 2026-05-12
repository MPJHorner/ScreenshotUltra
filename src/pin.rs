// Pin-to-screen: a floating, always-on-top NSWindow showing a screenshot.
// Multiple pins are supported; each owns its own window. ⌫ or ⌘W dismisses,
// ⌘+ / ⌘- zooms, click-and-drag moves the window.
//
// Implementation strategy: minimal — title bar + closable + resizable so
// the user gets familiar window chrome. Polished borderless mode lands
// later in M2.

#[cfg(not(target_os = "macos"))]
pub fn pin(_image_path: &std::path::Path) {}

#[cfg(target_os = "macos")]
pub use mac::pin;

#[cfg(target_os = "macos")]
mod mac {
    #![allow(unused_unsafe)]

    use std::cell::RefCell;
    use std::path::Path;

    use std::collections::HashMap;
    use std::path::PathBuf;

    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, ProtocolObject};
    use objc2::{define_class, msg_send, AllocAnyThread, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSDragOperation,
        NSDraggingContext, NSDraggingItem, NSDraggingSession, NSEvent, NSFloatingWindowLevel,
        NSImage, NSImageScaling, NSImageView, NSPasteboardItem, NSPasteboardTypeFileURL, NSWindow,
        NSWindowStyleMask,
    };
    use objc2_foundation::{NSArray, NSPoint, NSRect, NSSize, NSString, NSURL};

    // Keep pin windows alive so they don't drop and close immediately.
    thread_local! {
        static PINS: RefCell<Vec<Retained<NSWindow>>> = const { RefCell::new(Vec::new()) };

        // Maps a pin view's stable pointer to the file path it shows. Lets a
        // single class definition serve multiple simultaneous pins, each
        // dragging out its own file. Entries are removed when the pin is
        // closed.
        static PIN_PATHS: RefCell<HashMap<usize, PathBuf>> =
            RefCell::new(HashMap::new());

        static PIN_DRAG_STARTED: RefCell<bool> = const { RefCell::new(false) };
    }

    define_class!(
        /// All-in-one pin view: shows the screenshot, accepts scroll-to-dim,
        /// keyboard zoom + close, *and* serves as the drag source for the
        /// file. Subclassing NSImageView means the user sees the image
        /// underneath their cursor as they drag.
        #[unsafe(super(NSImageView))]
        #[name = "STUDraggablePin"]
        #[derive(Debug)]
        struct DraggablePin;

        impl DraggablePin {
            #[unsafe(method(acceptsFirstResponder))]
            fn accepts_first_responder(&self) -> bool {
                true
            }

            #[unsafe(method(scrollWheel:))]
            fn scroll_wheel(&self, event: &NSEvent) {
                let dy = unsafe { event.deltaY() };
                if let Some(win) = unsafe { self.window() } {
                    let cur = unsafe { win.alphaValue() };
                    let next = (cur + (dy * 0.04)).clamp(0.3, 1.0);
                    unsafe { win.setAlphaValue(next) };
                }
            }

            #[unsafe(method(keyDown:))]
            fn key_down(&self, event: &NSEvent) {
                let chars = unsafe { event.charactersIgnoringModifiers() };
                let s = chars.map(|c| c.to_string()).unwrap_or_default();
                let mods = unsafe { event.modifierFlags() };
                let cmd = mods.contains(objc2_app_kit::NSEventModifierFlags::Command);
                let Some(win) = (unsafe { self.window() }) else { return };
                match s.as_str() {
                    "\u{8}" | "\u{7f}" | "\u{1b}" => {
                        close_pin(self, &win);
                    }
                    "=" | "+" if cmd => zoom(&win, 1.10),
                    "-" if cmd => zoom(&win, 1.0 / 1.10),
                    "0" if cmd => reset_alpha(&win),
                    _ => {}
                }
            }

            #[unsafe(method(mouseDown:))]
            fn mouse_down(&self, _event: &NSEvent) {
                PIN_DRAG_STARTED.with(|s| *s.borrow_mut() = true);
            }

            #[unsafe(method(mouseDragged:))]
            fn mouse_dragged(&self, event: &NSEvent) {
                let started = PIN_DRAG_STARTED.with(|s| *s.borrow());
                if !started {
                    return;
                }
                PIN_DRAG_STARTED.with(|s| *s.borrow_mut() = false);
                let key = self as *const Self as usize;
                let path = PIN_PATHS.with(|m| m.borrow().get(&key).cloned());
                let Some(path) = path else { return };
                begin_drag(self, event, &path);
            }

            /// Required NSDraggingSource method.
            #[unsafe(method(draggingSession:sourceOperationMaskForDraggingContext:))]
            fn dragging_session_source_operation_mask_for_dragging_context(
                &self,
                _session: &NSDraggingSession,
                _context: NSDraggingContext,
            ) -> NSDragOperation {
                NSDragOperation::Copy
            }
        }
    );

    fn begin_drag(view: &DraggablePin, event: &NSEvent, path: &std::path::Path) {
        unsafe {
            let pb_item: Retained<NSPasteboardItem> = msg_send![NSPasteboardItem::alloc(), init];
            let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let url_str = NSString::from_str(&format!("file://{}", abs.display()));
            pb_item.setString_forType(&url_str, NSPasteboardTypeFileURL);

            let writer: &ProtocolObject<dyn objc2_app_kit::NSPasteboardWriting> =
                ProtocolObject::from_ref(&*pb_item);
            let drag_item: Retained<NSDraggingItem> =
                NSDraggingItem::initWithPasteboardWriter(NSDraggingItem::alloc(), writer);
            let bounds = view.bounds();
            let contents: Option<Retained<NSImage>> = msg_send![view, image];
            drag_item
                .setDraggingFrame_contents(bounds, contents.as_deref().map(|i| i as &AnyObject));

            let items: Retained<NSArray<NSDraggingItem>> =
                NSArray::from_retained_slice(&[drag_item]);
            let _session: Retained<NSDraggingSession> = msg_send![
                view,
                beginDraggingSessionWithItems: &*items,
                event: event,
                source: view as &AnyObject,
            ];
            crate::logging::event(serde_json::json!({
                "evt": "pin_action",
                "action": "drag_out",
                "path": path.display().to_string(),
            }));
        }
    }

    fn close_pin(view: &DraggablePin, win: &NSWindow) {
        unsafe {
            win.orderOut(None);
            win.close();
        }
        let view_key = view as *const DraggablePin as usize;
        PIN_PATHS.with(|m| {
            m.borrow_mut().remove(&view_key);
        });
        PINS.with(|v| {
            v.borrow_mut()
                .retain(|w| !std::ptr::eq(&**w as *const NSWindow, win as *const NSWindow))
        });
    }

    fn zoom(win: &NSWindow, factor: f64) {
        let frame = unsafe { win.frame() };
        // Anchor zoom on the window's centre so it scales in place.
        let cx = frame.origin.x + frame.size.width / 2.0;
        let cy = frame.origin.y + frame.size.height / 2.0;
        let new_w = (frame.size.width * factor).clamp(120.0, 4000.0);
        let new_h = (frame.size.height * factor).clamp(80.0, 3000.0);
        let new_frame = NSRect {
            origin: NSPoint {
                x: cx - new_w / 2.0,
                y: cy - new_h / 2.0,
            },
            size: NSSize {
                width: new_w,
                height: new_h,
            },
        };
        unsafe { win.setFrame_display_animate(new_frame, true, true) };
    }

    fn reset_alpha(win: &NSWindow) {
        unsafe { win.setAlphaValue(1.0) };
    }

    /// Default pin window size. We'll later scale to image aspect ratio.
    const DEFAULT_W: f64 = 320.0;
    const DEFAULT_H: f64 = 220.0;

    pub fn pin(image_path: &Path) {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("pin: not on main thread; skipping");
            return;
        };

        let app = NSApplication::sharedApplication(mtm);
        unsafe {
            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
            app.activate();
        }

        // Compute target window size from the image's pixel size, capped so
        // huge screenshots don't fill the screen.
        let img = match load_image(image_path) {
            Some(img) => img,
            None => {
                eprintln!("pin: failed to load image {}", image_path.display());
                return;
            }
        };
        let size = unsafe { img.size() };
        let (w, h) = fit(size.width, size.height, 600.0, 600.0, DEFAULT_W, DEFAULT_H);

        // Cascade subsequent pins so they don't all stack at the same point.
        let offset = PINS.with(|v| v.borrow().len() as f64) * 24.0;
        let rect = NSRect {
            origin: NSPoint {
                x: 100.0 + offset,
                y: 100.0 + offset,
            },
            size: NSSize {
                width: w,
                height: h + 22.0, // title bar
            },
        };

        let style =
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Resizable;
        let window: Retained<NSWindow> = unsafe {
            let alloc = NSWindow::alloc(mtm);
            msg_send![
                alloc,
                initWithContentRect: rect,
                styleMask: style,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };

        unsafe {
            window.setLevel(NSFloatingWindowLevel);
            window.setReleasedWhenClosed(false);
            let title = NSString::from_str(
                image_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Pinned"),
            );
            window.setTitle(&title);
        }

        if let Some(content_view) = window.contentView() {
            let view_rect = NSRect {
                origin: NSPoint { x: 0.0, y: 0.0 },
                size: NSSize {
                    width: w,
                    height: h,
                },
            };
            // One view handles image display, drag-out, scroll dim, and
            // keyboard zoom/close — simpler than a stack of overlays.
            let pin_view: Retained<DraggablePin> = unsafe {
                let v: Retained<DraggablePin> =
                    msg_send![DraggablePin::alloc(mtm), initWithFrame: view_rect];
                v.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
                v.setImage(Some(&img));
                v.setAutoresizingMask(
                    objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
                        | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
                );
                v
            };
            let view_key = &*pin_view as *const DraggablePin as usize;
            PIN_PATHS.with(|m| {
                m.borrow_mut().insert(view_key, image_path.to_path_buf());
            });
            unsafe {
                content_view.addSubview(&pin_view);
                window.makeFirstResponder(Some(&pin_view));
            }
        }

        unsafe {
            window.makeKeyAndOrderFront(None);
            window.display();
        }

        PINS.with(|v| v.borrow_mut().push(window));
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

    /// Fit (img_w, img_h) into max_w × max_h, falling back to (default_w, default_h)
    /// if the image reports zero size. Returns the chosen content size.
    pub fn fit(
        img_w: f64,
        img_h: f64,
        max_w: f64,
        max_h: f64,
        default_w: f64,
        default_h: f64,
    ) -> (f64, f64) {
        if img_w <= 0.0 || img_h <= 0.0 {
            return (default_w, default_h);
        }
        let scale = (max_w / img_w).min(max_h / img_h).min(1.0);
        (img_w * scale, img_h * scale)
    }

    #[cfg(test)]
    mod tests {
        use super::fit;

        #[test]
        fn fit_downscales_large_images() {
            // 2000x1000 should fit into 600x600 keeping aspect ratio
            let (w, h) = fit(2000.0, 1000.0, 600.0, 600.0, 100.0, 100.0);
            assert!((w - 600.0).abs() < 0.01);
            assert!((h - 300.0).abs() < 0.01);
        }

        #[test]
        fn fit_preserves_small_images() {
            // 200x100 within 600x600 should stay as-is (scale capped at 1.0)
            let (w, h) = fit(200.0, 100.0, 600.0, 600.0, 100.0, 100.0);
            assert_eq!(w, 200.0);
            assert_eq!(h, 100.0);
        }

        #[test]
        fn fit_falls_back_to_default_for_zero_size() {
            let (w, h) = fit(0.0, 0.0, 600.0, 600.0, 320.0, 220.0);
            assert_eq!(w, 320.0);
            assert_eq!(h, 220.0);
        }
    }
}
