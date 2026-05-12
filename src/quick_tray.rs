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
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, sel, AllocAnyThread, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSBezelStyle, NSButton,
        NSColor, NSFloatingWindowLevel, NSImage, NSImageScaling, NSImageView, NSScreen, NSWindow,
        NSWindowStyleMask,
    };
    use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize, NSString, NSTimer, NSURL};

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

    const PANEL_W: f64 = 540.0;
    const PANEL_H: f64 = 110.0;
    const MARGIN: f64 = 24.0;
    const THUMB: f64 = 80.0;

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
            .map(|s| s.frame())
            .ok_or_else(|| "no main screen".to_string())?;
        eprintln!(
            "quick_tray: screen frame ({},{}) {}x{}",
            screen_frame.origin.x,
            screen_frame.origin.y,
            screen_frame.size.width,
            screen_frame.size.height
        );

        // Bottom-right of the main screen, above the dock.
        let origin = NSPoint {
            x: screen_frame.origin.x + screen_frame.size.width - PANEL_W - MARGIN,
            y: screen_frame.origin.y + MARGIN + 80.0, // dock clearance
        };
        let rect = NSRect {
            origin,
            size: NSSize {
                width: PANEL_W,
                height: PANEL_H,
            },
        };
        eprintln!(
            "quick_tray: window rect ({},{}) {}x{}",
            rect.origin.x, rect.origin.y, rect.size.width, rect.size.height
        );

        // Standard titled NSWindow — guaranteed visible chrome.
        let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable;
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
            window.setCollectionBehavior(
                objc2_app_kit::NSWindowCollectionBehavior::CanJoinAllSpaces
                    | objc2_app_kit::NSWindowCollectionBehavior::Transient,
            );
            let title = NSString::from_str("Screenshot Ultra");
            window.setTitle(&title);
            // Bright background so we can't miss it during diagnosis.
            let bg = NSColor::colorWithSRGBRed_green_blue_alpha(0.10, 0.12, 0.18, 1.0);
            window.setBackgroundColor(Some(&bg));
            window.setOpaque(true);
        }

        let content_view = window
            .contentView()
            .ok_or_else(|| "window has no content view".to_string())?;

        // Thumbnail
        let thumb_rect = NSRect {
            origin: NSPoint {
                x: 14.0,
                y: (PANEL_H - THUMB) / 2.0 - 12.0, /* leave room for title bar */
            },
            size: NSSize {
                width: THUMB,
                height: THUMB,
            },
        };
        let image_view = unsafe {
            let v = NSImageView::initWithFrame(NSImageView::alloc(mtm), thumb_rect);
            v.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
            v.setWantsLayer(true);
            if let Some(img) = load_image(image_path) {
                v.setImage(Some(&img));
            }
            v
        };
        unsafe { content_view.addSubview(&image_view) };

        // Handler for clicks.
        let handler: Retained<Handler> = unsafe { msg_send![Handler::alloc(), init] };

        // Button row.
        let labels = ["Copy", "Text", "Edit", "Folder", "Reveal", "Pin", "Discard"];
        let tags: [isize; 7] = [1, 7, 2, 3, 4, 5, 6];
        let btn_w = 56.0;
        let btn_h = 28.0;
        let gap = 4.0;
        let total = (btn_w * labels.len() as f64) + (gap * (labels.len() as f64 - 1.0));
        let start_x = PANEL_W - total - 14.0;
        let y = (PANEL_H - btn_h) / 2.0 - 12.0;
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
                b.setBordered(true);
                b.setTag(tags[i]);
                b.setTarget(Some(handler.as_ref()));
                b.setAction(Some(sel!(buttonClicked:)));
                b
            };
            unsafe { content_view.addSubview(&btn) };
        }

        // Show with explicit activation. `makeKeyAndOrderFront` brings it on
        // screen, then we force a draw via display() and pump one runloop
        // iteration so AppKit actually paints before we return.
        unsafe {
            window.makeKeyAndOrderFront(None);
            window.orderFrontRegardless();
            window.display();
        }

        eprintln!("quick_tray: window onScreen={}", unsafe {
            window.isVisible()
        });

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
