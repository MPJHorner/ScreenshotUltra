// Quick Tray — a small floating panel that appears bottom-right after a
// capture, showing the thumbnail and a row of action buttons.
//
// Built on AppKit's NSPanel via objc2 so we don't depend on a GUI toolkit.

#[cfg(not(target_os = "macos"))]
pub fn show(_image_path: &std::path::Path, _timeout_ms: u64) {}

#[cfg(target_os = "macos")]
pub use mac::show;

#[cfg(target_os = "macos")]
mod mac {
    // AppKit bindings mix `unsafe` and safe methods; grouping calls under
    // unsafe blocks is intentional for readability. Suppress the lint.
    #![allow(unused_unsafe)]

    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, sel, AllocAnyThread, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSBackingStoreType, NSBezelStyle, NSButton, NSColor, NSImage, NSImageScaling, NSImageView,
        NSPanel, NSScreen, NSStatusWindowLevel, NSWindowStyleMask,
    };
    use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize, NSString, NSTimer, NSURL};

    /// What the user clicked on the tray.
    #[derive(Debug, Clone, Copy)]
    enum Action {
        Copy,
        OpenFolder,
        RevealInFinder,
        Discard,
    }

    /// Per-show state, owned on the main thread only.
    struct TrayState {
        panel: Retained<NSPanel>,
        image_path: PathBuf,
        _handler: Retained<Handler>,
        timer: Option<Retained<NSTimer>>,
    }

    thread_local! {
        static CURRENT: RefCell<Option<TrayState>> = const { RefCell::new(None) };
    }

    /// Show the Quick Tray for the most recently captured file.
    /// Must be called on the main thread (where AppKit lives).
    pub fn show(image_path: &Path, timeout_ms: u64) {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("quick_tray::show called off the main thread; skipping");
            return;
        };

        // Dismiss any previous tray so we don't stack them.
        CURRENT.with(|slot| *slot.borrow_mut() = None);

        let path = image_path.to_path_buf();
        match build_panel(mtm, &path, timeout_ms) {
            Ok(state) => CURRENT.with(|slot| *slot.borrow_mut() = Some(state)),
            Err(err) => eprintln!("quick_tray: failed to build panel: {err}"),
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
                    2 => Action::OpenFolder,
                    3 => Action::RevealInFinder,
                    4 => Action::Discard,
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
                    state.panel.orderOut(None);
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
            Action::Discard => {
                let _ = std::fs::remove_file(&path);
                crate::logging::event(serde_json::json!({
                    "evt": "tray_action", "action": "discard",
                    "path": path.display().to_string(),
                }));
            }
        }
    }

    const PANEL_W: f64 = 340.0;
    const PANEL_H: f64 = 92.0;
    const MARGIN: f64 = 18.0;
    const THUMB: f64 = 64.0;

    fn build_panel(
        mtm: MainThreadMarker,
        image_path: &Path,
        timeout_ms: u64,
    ) -> Result<TrayState, String> {
        let screen_frame = NSScreen::mainScreen(mtm)
            .map(|s| s.frame())
            .ok_or_else(|| "no main screen".to_string())?;

        let origin = NSPoint {
            x: screen_frame.origin.x + screen_frame.size.width - PANEL_W - MARGIN,
            y: screen_frame.origin.y + MARGIN,
        };
        let rect = NSRect {
            origin,
            size: NSSize {
                width: PANEL_W,
                height: PANEL_H,
            },
        };

        let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
        let panel: Retained<NSPanel> = unsafe {
            let alloc = NSPanel::alloc(mtm);
            msg_send![
                alloc,
                initWithContentRect: rect,
                styleMask: style,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };

        unsafe {
            panel.setLevel(NSStatusWindowLevel);
            panel.setOpaque(false);
            panel.setHasShadow(true);
            panel.setReleasedWhenClosed(false);
            panel.setIgnoresMouseEvents(false);
            panel.setHidesOnDeactivate(false);
            panel.setBackgroundColor(Some(&NSColor::clearColor()));
        }

        let content_view = panel
            .contentView()
            .ok_or_else(|| "panel has no content view".to_string())?;

        unsafe {
            content_view.setWantsLayer(true);
            if let Some(layer) = content_view.layer() {
                layer.setCornerRadius(12.0);
                let color = NSColor::colorWithSRGBRed_green_blue_alpha(0.07, 0.07, 0.08, 0.92);
                let cg = color.CGColor();
                layer.setBackgroundColor(Some(&cg));
            }
        }

        // Thumbnail
        let thumb_rect = NSRect {
            origin: NSPoint {
                x: 14.0,
                y: (PANEL_H - THUMB) / 2.0,
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
            if let Some(layer) = v.layer() {
                layer.setCornerRadius(6.0);
                layer.setMasksToBounds(true);
            }
            if let Some(img) = load_image(image_path) {
                v.setImage(Some(&img));
            }
            v
        };
        unsafe { content_view.addSubview(&image_view) };

        // Button row
        let handler: Retained<Handler> = unsafe { msg_send![Handler::alloc(), init] };
        let labels = ["Copy", "Folder", "Reveal", "Discard"];
        let tags: [isize; 4] = [1, 2, 3, 4];
        let btn_w = 64.0;
        let btn_h = 26.0;
        let gap = 6.0;
        let total = (btn_w * labels.len() as f64) + (gap * (labels.len() as f64 - 1.0));
        let start_x = PANEL_W - total - 14.0;
        let y = (PANEL_H - btn_h) / 2.0;
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

        unsafe { panel.orderFrontRegardless() };

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
            panel,
            image_path: image_path.to_path_buf(),
            _handler: handler,
            timer: Some(timer),
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
