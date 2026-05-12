// Quick Tray — a small floating window that appears after a capture with
// thumbnail + action buttons. Native AppKit via objc2.

#[cfg(not(target_os = "macos"))]
pub fn show(_image_path: &std::path::Path, _timeout_ms: u64) {}

#[cfg(target_os = "macos")]
pub use mac::show;

#[cfg(target_os = "macos")]
mod mac {
    #![allow(unused_unsafe)]

    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, ProtocolObject};
    use objc2::{define_class, msg_send, sel, AllocAnyThread, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSBezelStyle, NSButton,
        NSCellImagePosition, NSColor, NSDragOperation, NSDraggingContext, NSDraggingItem,
        NSDraggingSession, NSEvent, NSFloatingWindowLevel, NSImage, NSImageScaling,
        NSImageSymbolConfiguration, NSImageView, NSPasteboardItem, NSPasteboardTypeFileURL,
        NSScreen, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
        NSVisualEffectView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
    };
    use objc2_foundation::{NSArray, NSObject, NSPoint, NSRect, NSSize, NSString, NSTimer, NSURL};

    #[derive(Debug, Clone, Copy)]
    enum Action {
        Copy,
        Edit,
        OpenFolder,
        RevealInFinder,
        Pin,
        Discard,
        Text,
    }

    struct TrayState {
        window: Retained<NSWindow>,
        image_path: PathBuf,
        _handler: Retained<Handler>,
        timer: Option<Retained<NSTimer>>,
    }

    thread_local! {
        static CURRENT: RefCell<Option<TrayState>> = const { RefCell::new(None) };
    }

    pub fn show(image_path: &Path, timeout_ms: u64) {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("quick_tray: not on main thread; skipping");
            return;
        };

        // Dismiss any previous tray so we don't stack them.
        CURRENT.with(|slot| *slot.borrow_mut() = None);

        match build_window(mtm, image_path, timeout_ms) {
            Ok(state) => CURRENT.with(|slot| *slot.borrow_mut() = Some(state)),
            Err(err) => eprintln!("quick_tray: build failed: {err}"),
        }
    }

    // The path currently being shown in the Quick Tray's thumbnail.
    // Reading this from the drag source lets us avoid wrestling with
    // objc2 ivars — at most one Quick Tray is on screen at a time, so
    // a thread-local is sufficient.
    thread_local! {
        static THUMB_DRAG_PATH: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    }

    define_class!(
        /// NSImageView subclass that becomes the drag source for the
        /// thumbnail. Once the user moves the mouse more than a few
        /// points past `mouseDown`, we initiate a drag session whose
        /// pasteboard item points at the captured file — so the
        /// user can drop the screenshot straight into Slack, an
        /// email composer, Finder, anywhere that takes a file.
        ///
        /// NSDraggingSource conformance is asserted via the runtime by
        /// providing the required method below; we bypass the Rust
        /// typed binding (`beginDraggingSessionWithItems:event:source:`)
        /// because objc2's strict generics + duplicate-objc2-version
        /// trees won't let us prove the conformance at the type level.
        #[unsafe(super(NSImageView))]
        #[name = "STUDraggableThumb"]
        #[derive(Debug)]
        struct DraggableThumb;

        impl DraggableThumb {
            #[unsafe(method(mouseDown:))]
            fn mouse_down(&self, _event: &NSEvent) {
                DRAG_START.with(|s| *s.borrow_mut() = Some(true));
            }

            #[unsafe(method(mouseDragged:))]
            fn mouse_dragged(&self, event: &NSEvent) {
                let started = DRAG_START.with(|s| s.borrow().is_some());
                if !started {
                    return;
                }
                DRAG_START.with(|s| *s.borrow_mut() = None);
                let path = THUMB_DRAG_PATH.with(|p| p.borrow().clone());
                let Some(path) = path else { return };
                begin_drag(self, event, &path);
            }

            /// Required by `NSDraggingSource`. Returning `Copy` lets the
            /// drop target make a copy of the file — exactly what we
            /// want when dragging a screenshot into Mail, Slack, etc.
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

    thread_local! {
        static DRAG_START: RefCell<Option<bool>> = const { RefCell::new(None) };
    }

    fn begin_drag(view: &DraggableThumb, event: &NSEvent, path: &Path) {
        unsafe {
            // Build an NSPasteboardItem with the file URL.
            let pb_item: Retained<NSPasteboardItem> = msg_send![NSPasteboardItem::alloc(), init];
            let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let url_str = NSString::from_str(&format!("file://{}", abs.display()));
            pb_item.setString_forType(&url_str, NSPasteboardTypeFileURL);

            // Wrap it in an NSDraggingItem with the thumbnail as the drag image.
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
            // Bypass the typed binding's strict `&ProtocolObject<dyn NSDraggingSource>`
            // and dispatch by selector — Cocoa only cares that the object
            // responds to the required selector at runtime.
            let _session: Retained<NSDraggingSession> = msg_send![
                view,
                beginDraggingSessionWithItems: &*items,
                event: event,
                source: view as &AnyObject,
            ];
            crate::logging::event(serde_json::json!({
                "evt": "tray_action",
                "action": "drag_out",
                "path": path.display().to_string(),
            }));
        }
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "STUQuickTrayHandler"]
        #[derive(Debug)]
        struct Handler;

        impl Handler {
            #[unsafe(method(buttonClicked:))]
            fn button_clicked(&self, sender: *mut AnyObject) {
                let tag: isize = unsafe { msg_send![sender, tag] };
                let action = match tag {
                    1 => Action::Copy,
                    2 => Action::Edit,
                    3 => Action::OpenFolder,
                    4 => Action::RevealInFinder,
                    5 => Action::Pin,
                    6 => Action::Discard,
                    7 => Action::Text,
                    _ => return,
                };
                perform_action(action);
                dismiss();
            }

            #[unsafe(method(timerFired:))]
            fn timer_fired(&self, _timer: *mut AnyObject) {
                dismiss();
            }
        }
    );

    fn dismiss() {
        CURRENT.with(|slot| {
            if let Some(state) = slot.borrow_mut().take() {
                unsafe {
                    if let Some(t) = &state.timer {
                        t.invalidate();
                    }
                    state.window.orderOut(None);
                    state.window.close();
                }
            }
        });
    }

    fn perform_action(action: Action) {
        let path = CURRENT.with(|slot| slot.borrow().as_ref().map(|s| s.image_path.clone()));
        let Some(path) = path else { return };
        match action {
            Action::Copy => {
                if let Err(err) = crate::sinks::clipboard_copy_image(&path) {
                    eprintln!("quick_tray: copy failed: {err:#}");
                }
                crate::logging::event(serde_json::json!({
                    "evt": "tray_action", "action": "copy",
                    "path": path.display().to_string(),
                }));
            }
            Action::Edit => {
                crate::editor::open(&path);
                crate::logging::event(serde_json::json!({
                    "evt": "tray_action", "action": "edit",
                }));
            }
            Action::OpenFolder => {
                if let Some(parent) = path.parent() {
                    let _ = std::process::Command::new("open").arg(parent).status();
                }
                crate::logging::event(serde_json::json!({
                    "evt": "tray_action", "action": "open_folder",
                }));
            }
            Action::RevealInFinder => {
                let _ = std::process::Command::new("open")
                    .arg("-R")
                    .arg(&path)
                    .status();
                crate::logging::event(serde_json::json!({
                    "evt": "tray_action", "action": "reveal",
                }));
            }
            Action::Pin => {
                crate::pin::pin(&path);
                crate::logging::event(serde_json::json!({
                    "evt": "tray_action", "action": "pin",
                }));
            }
            Action::Text => {
                let text = crate::ocr::extract_text(&path);
                match text {
                    Some(t) => {
                        // Copy OCR'd text to clipboard so it's
                        // pasteable anywhere immediately.
                        let _ = std::process::Command::new("/bin/sh")
                            .arg("-c")
                            .arg("/usr/bin/pbcopy")
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                            .and_then(|mut child| {
                                use std::io::Write;
                                if let Some(stdin) = child.stdin.as_mut() {
                                    let _ = stdin.write_all(t.as_bytes());
                                }
                                child.wait()
                            });
                        let chars = t.chars().count();
                        crate::logging::event(serde_json::json!({
                            "evt": "ocr",
                            "path": path.display().to_string(),
                            "chars": chars,
                        }));
                        crate::sinks::notify(
                            "Screenshot Ultra — text copied",
                            &format!("{chars} characters on clipboard"),
                        );
                    }
                    None => {
                        crate::sinks::notify(
                            "Screenshot Ultra — no text",
                            "Vision didn't find any text in this capture.",
                        );
                    }
                }
            }
            Action::Discard => {
                let _ = std::fs::remove_file(&path);
                crate::logging::event(serde_json::json!({
                    "evt": "tray_action", "action": "discard",
                    "path": path.display().to_string(),
                }));
            }
        }
    }

    // Layout budget. The panel sizes itself to the screenshot's aspect ratio
    // (capped so giant captures don't fill the screen, floored so a thin
    // vertical strip still looks like a card). The action bar sits directly
    // below the preview — same width — so the screenshot stays "alive" as
    // the largest visual element and the actions read as belonging to it.
    const PREVIEW_MAX_W: f64 = 380.0;
    const PREVIEW_MAX_H: f64 = 260.0;
    const PREVIEW_MIN_W: f64 = 260.0;
    const PREVIEW_MIN_H: f64 = 150.0;
    const OUTER_PAD: f64 = 10.0;
    const PREVIEW_BAR_GAP: f64 = 8.0;
    const BTN_SIZE: f64 = 30.0;
    const BTN_GAP: f64 = 2.0;
    const CORNER_RADIUS: f64 = 14.0;
    const PREVIEW_CORNER_RADIUS: f64 = 8.0;
    const SCREEN_MARGIN: f64 = 22.0;

    /// Fit `(w, h)` inside `(max_w, max_h)` preserving aspect ratio, then
    /// floor to the minimums so a sliver capture still looks like a card.
    fn fit_preview(img_w: f64, img_h: f64) -> (f64, f64) {
        if img_w <= 0.0 || img_h <= 0.0 {
            return (PREVIEW_MAX_W, PREVIEW_MAX_H * 0.66);
        }
        let aspect = img_w / img_h;
        let max_aspect = PREVIEW_MAX_W / PREVIEW_MAX_H;
        let (mut w, mut h) = if aspect >= max_aspect {
            (PREVIEW_MAX_W, PREVIEW_MAX_W / aspect)
        } else {
            (PREVIEW_MAX_H * aspect, PREVIEW_MAX_H)
        };
        if w < PREVIEW_MIN_W {
            w = PREVIEW_MIN_W;
        }
        if h < PREVIEW_MIN_H {
            h = PREVIEW_MIN_H;
        }
        (w.round(), h.round())
    }

    /// Find the NSScreen that contains `pt`, or `None` if the point is
    /// off-screen (e.g. when called from a context without a real cursor
    /// like a fullscreen capture pumped from a hotkey).
    fn find_screen_for_point(mtm: MainThreadMarker, pt: NSPoint) -> Option<NSRect> {
        let screens = unsafe { NSScreen::screens(mtm) };
        for i in 0..screens.count() {
            let s = screens.objectAtIndex(i);
            let f = s.frame();
            if pt.x >= f.origin.x
                && pt.x <= f.origin.x + f.size.width
                && pt.y >= f.origin.y
                && pt.y <= f.origin.y + f.size.height
            {
                return Some(unsafe { s.visibleFrame() });
            }
        }
        None
    }

    /// Choose a top-left origin so the panel hangs just below-and-slightly-
    /// right of the cursor. Clamped to the screen's visible frame so the
    /// panel never hides under the menu bar / Dock / off-screen.
    fn anchor_under_cursor(
        cursor: NSPoint,
        screen: NSRect,
        content_w: f64,
        content_h: f64,
    ) -> NSPoint {
        // Offset just enough to clear the cursor + the OS's screencapture
        // confirmation pip. AppKit's coord system has Y growing up, so to
        // place the panel "below" the cursor we subtract content_h + offset.
        const CURSOR_OFFSET_X: f64 = 12.0;
        const CURSOR_OFFSET_Y: f64 = 14.0;

        let mut x = cursor.x - content_w / 2.0 + CURSOR_OFFSET_X;
        let mut y = cursor.y - content_h - CURSOR_OFFSET_Y;

        let min_x = screen.origin.x + SCREEN_MARGIN;
        let max_x = screen.origin.x + screen.size.width - content_w - SCREEN_MARGIN;
        let min_y = screen.origin.y + SCREEN_MARGIN;
        let max_y = screen.origin.y + screen.size.height - content_h - SCREEN_MARGIN;

        // If anchoring below would clip off the bottom of the screen, flip
        // and hang the panel above the cursor instead.
        if y < min_y {
            y = cursor.y + CURSOR_OFFSET_Y;
        }

        if max_x > min_x {
            x = x.clamp(min_x, max_x);
        }
        if max_y > min_y {
            y = y.clamp(min_y, max_y);
        }

        NSPoint { x, y }
    }

    fn build_window(
        mtm: MainThreadMarker,
        image_path: &Path,
        timeout_ms: u64,
    ) -> Result<TrayState, String> {
        eprintln!(
            "quick_tray: build_window image={} timeout={}ms",
            image_path.display(),
            timeout_ms
        );

        // For LSUIElement background apps macOS 26+ silently suppresses
        // windows unless the app has been promoted to accessory/regular and
        // explicitly activated. Promote temporarily so our window draws.
        let app = NSApplication::sharedApplication(mtm);
        unsafe {
            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
            app.activate();
        }

        let screen_frame = NSScreen::mainScreen(mtm)
            .map(|s| unsafe { s.visibleFrame() })
            .ok_or_else(|| "no main screen".to_string())?;

        // Load image up-front so the window can be sized to the capture's
        // aspect ratio rather than a fixed slab.
        let image = load_image(image_path);
        let (img_w, img_h) = image
            .as_ref()
            .map(|i| unsafe {
                let s = i.size();
                (s.width, s.height)
            })
            .unwrap_or((PREVIEW_MAX_W, PREVIEW_MAX_H * 0.66));
        let (preview_w, preview_h) = fit_preview(img_w, img_h);

        // Action row sizing — 7 icon buttons, centered under the preview.
        let n_buttons: f64 = 7.0;
        let row_w = BTN_SIZE * n_buttons + BTN_GAP * (n_buttons - 1.0);
        let row_h = BTN_SIZE;

        // Window content rect: preview on top, gap, action row beneath,
        // outer padding all around. Width = whichever of preview/row is
        // wider, so a very narrow vertical capture still hosts the bar.
        let content_w = preview_w.max(row_w) + OUTER_PAD * 2.0;
        let content_h = preview_h + PREVIEW_BAR_GAP + row_h + OUTER_PAD * 2.0;

        // Anchor the panel just below where the user finished the cut.
        // `screencapture -i` returns control the instant the user releases
        // the drag, so the cursor is sitting near the bottom-right of the
        // captured region — close enough to read as "this panel belongs to
        // what I just snipped." If the cursor is unavailable (clipboard /
        // fullscreen paths) or anchoring would push us off-screen, we fall
        // back to the bottom-right of the active display.
        let mouse = unsafe { NSEvent::mouseLocation() };
        let target_screen = find_screen_for_point(mtm, mouse).unwrap_or(screen_frame);
        let origin = anchor_under_cursor(mouse, target_screen, content_w, content_h);
        let rect = NSRect {
            origin,
            size: NSSize {
                width: content_w,
                height: content_h,
            },
        };

        // Borderless floating panel — we paint our own chrome.
        let style = NSWindowStyleMask::Borderless;
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
            window.setHidesOnDeactivate(false);
            window.setOpaque(false);
            window.setBackgroundColor(Some(&NSColor::clearColor()));
            window.setHasShadow(true);
            window.setMovableByWindowBackground(true);
            window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::Transient,
            );
        }

        // Vibrancy / blurred dark background, rounded as the entire chrome.
        let effect_frame = NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: NSSize {
                width: content_w,
                height: content_h,
            },
        };
        let effect: Retained<NSVisualEffectView> = unsafe {
            let v = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), effect_frame);
            v.setMaterial(NSVisualEffectMaterial::HUDWindow);
            v.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
            v.setState(NSVisualEffectState::Active);
            v.setWantsLayer(true);
            if let Some(layer) = v.layer() {
                layer.setCornerRadius(CORNER_RADIUS);
                layer.setMasksToBounds(true);
            }
            v
        };
        unsafe {
            let any: &AnyObject = (*effect).as_ref();
            let _: () = msg_send![&*window, setContentView: any];
        }

        // Preview image view — large, aspect-correct, draggable, rounded.
        let preview_y = OUTER_PAD + row_h + PREVIEW_BAR_GAP;
        let preview_x = (content_w - preview_w) / 2.0;
        let preview_frame = NSRect {
            origin: NSPoint {
                x: preview_x,
                y: preview_y,
            },
            size: NSSize {
                width: preview_w,
                height: preview_h,
            },
        };
        THUMB_DRAG_PATH.with(|p| *p.borrow_mut() = Some(image_path.to_path_buf()));
        let image_view: Retained<DraggableThumb> = unsafe {
            let v: Retained<DraggableThumb> =
                msg_send![DraggableThumb::alloc(mtm), initWithFrame: preview_frame];
            v.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
            v.setWantsLayer(true);
            if let Some(layer) = v.layer() {
                layer.setCornerRadius(PREVIEW_CORNER_RADIUS);
                layer.setMasksToBounds(true);
                // Faint checkerboard-ish base so transparent PNGs still
                // read as having a frame.
                let bg = NSColor::colorWithSRGBRed_green_blue_alpha(0.0, 0.0, 0.0, 0.28);
                layer.setBackgroundColor(Some(&*bg.CGColor()));
            }
            if let Some(img) = image.as_deref() {
                v.setImage(Some(img));
            }
            v
        };
        unsafe { effect.addSubview(&image_view) };

        // Handler for clicks + timer fires.
        let handler: Retained<Handler> = unsafe { msg_send![Handler::alloc(), init] };

        // Icon button row, centered horizontally under the preview.
        let labels = ["Copy", "Text", "Edit", "Folder", "Reveal", "Pin", "Discard"];
        let symbols = [
            "doc.on.doc",
            "text.viewfinder",
            "pencil",
            "folder",
            "magnifyingglass",
            "pin",
            "trash",
        ];
        let tags: [isize; 7] = [1, 7, 2, 3, 4, 5, 6];
        let row_start_x = (content_w - row_w) / 2.0;
        for i in 0..labels.len() {
            let x = row_start_x + (BTN_SIZE + BTN_GAP) * i as f64;
            let frame = NSRect {
                origin: NSPoint { x, y: OUTER_PAD },
                size: NSSize {
                    width: BTN_SIZE,
                    height: BTN_SIZE,
                },
            };
            let btn =
                make_icon_button(mtm, frame, symbols[i], labels[i], tags[i], handler.as_ref());
            unsafe { effect.addSubview(&btn) };
        }

        unsafe {
            window.makeKeyAndOrderFront(None);
            window.orderFrontRegardless();
            window.display();
        }

        let interval = (timeout_ms as f64) / 1000.0;
        let timer: Retained<NSTimer> = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                interval,
                handler.as_ref(),
                sel!(timerFired:),
                None,
                false,
            )
        };

        Ok(TrayState {
            window,
            image_path: image_path.to_path_buf(),
            _handler: handler,
            timer: Some(timer),
        })
    }

    /// Build a compact icon-only NSButton with an SF Symbol glyph, white
    /// tint, and hover border. The Handler dispatches by `tag`.
    fn make_icon_button(
        mtm: MainThreadMarker,
        frame: NSRect,
        symbol: &str,
        accessibility: &str,
        tag: isize,
        handler: &Handler,
    ) -> Retained<NSButton> {
        unsafe {
            let btn = NSButton::initWithFrame(NSButton::alloc(mtm), frame);
            btn.setTitle(&NSString::from_str(""));
            btn.setBezelStyle(NSBezelStyle::AccessoryBar);
            btn.setBordered(true);
            btn.setShowsBorderOnlyWhileMouseInside(true);
            btn.setImagePosition(NSCellImagePosition::ImageOnly);
            btn.setImageScaling(NSImageScaling::ScaleProportionallyDown);
            btn.setTag(tag);
            btn.setTarget(Some(handler));
            btn.setAction(Some(sel!(buttonClicked:)));
            let tt = NSString::from_str(accessibility);
            let _: () = msg_send![&*btn, setToolTip: &*tt];

            let sym = NSString::from_str(symbol);
            let acc = NSString::from_str(accessibility);
            if let Some(img) =
                NSImage::imageWithSystemSymbolName_accessibilityDescription(&sym, Some(&acc))
            {
                // Slightly heavier weight reads better on dark HUD bg.
                let config =
                    NSImageSymbolConfiguration::configurationWithPointSize_weight(15.0, 0.23);
                let resolved = img
                    .imageWithSymbolConfiguration(&config)
                    .unwrap_or_else(|| img.clone());
                btn.setImage(Some(&resolved));
                btn.setContentTintColor(Some(&NSColor::whiteColor()));
            }
            btn
        }
    }

    fn load_image(path: &Path) -> Option<Retained<NSImage>> {
        // For video files, NSImage::initWithContentsOfURL: returns nil.
        // Side-load a poster frame via macOS's `qlmanage` and point
        // NSImage at the resulting PNG.
        let load_path = match path.extension().and_then(|e| e.to_str()) {
            Some("mov") | Some("mp4") | Some("m4v") => poster_frame(path)?,
            _ => path.to_path_buf(),
        };
        unsafe {
            let s = NSString::from_str(&load_path.to_string_lossy());
            let url = NSURL::fileURLWithPath(&s);
            let img: Option<Retained<NSImage>> =
                msg_send![NSImage::alloc(), initWithContentsOfURL: &*url];
            img
        }
    }

    /// Render a poster-frame PNG for a video via macOS's `qlmanage -t`
    /// command. Cached under TMPDIR so repeated tray shows of the same
    /// recording reuse it. Returns None if qlmanage didn't produce a
    /// file (corrupted recording, missing tool, etc.).
    fn poster_frame(video_path: &Path) -> Option<std::path::PathBuf> {
        let stem = video_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("video");
        let tmp = std::env::temp_dir().join("screenshot-ultra-posters");
        let _ = std::fs::create_dir_all(&tmp);
        // qlmanage writes `<input>.png` into the -o directory.
        let expected = tmp.join(format!("{stem}.png"));
        if expected.exists() {
            return Some(expected);
        }
        let status = std::process::Command::new("/usr/bin/qlmanage")
            .arg("-t")
            .arg("-s")
            .arg("480")
            .arg("-o")
            .arg(&tmp)
            .arg(video_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .ok()?;
        if !status.success() || !expected.exists() {
            return None;
        }
        Some(expected)
    }
}
