// First-run welcome window. The first time the app launches we open a
// styled NSWindow walking the user through the three things they need
// to know: (1) the Screen Recording permission, (2) the default
// hotkeys, (3) the menu bar icon. Once dismissed we touch a marker
// file so we never show it again.
//
// We deliberately gate on a marker file rather than a settings.toml
// flag so users upgrading from an older release don't suddenly see
// the welcome on their next launch.

use std::path::PathBuf;

#[cfg(not(target_os = "macos"))]
pub fn show_if_first_run(_settings: &crate::settings::Settings) {}

#[cfg(target_os = "macos")]
pub use mac::show_if_first_run;

/// Returns the path of the "we've already welcomed you" marker file.
/// Versioned so future revisions of the welcome flow can re-trigger.
pub fn marker_path() -> Option<PathBuf> {
    let base = dirs::config_dir().or_else(dirs::home_dir)?;
    Some(base.join("ScreenshotUltra").join(".welcomed-v1"))
}

#[cfg(target_os = "macos")]
mod mac {
    #![allow(unused_unsafe)]

    use std::cell::RefCell;

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, sel, AllocAnyThread, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSBezelStyle, NSButton,
        NSColor, NSFloatingWindowLevel, NSTextField, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

    use crate::settings::Settings;
    use crate::welcome::marker_path;

    thread_local! {
        static WELCOME_WINDOW: RefCell<Option<Retained<NSWindow>>> = const { RefCell::new(None) };
    }

    pub fn show_if_first_run(settings: &Settings) {
        let Some(marker) = marker_path() else { return };
        if marker.exists() {
            return;
        }
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        match build(mtm, settings) {
            Ok(w) => WELCOME_WINDOW.with(|c| *c.borrow_mut() = Some(w)),
            Err(err) => eprintln!("welcome: build failed: {err}"),
        }
    }

    define_class!(
        // Receives clicks from the welcome window's buttons.
        #[unsafe(super(objc2_foundation::NSObject))]
        #[name = "STUWelcomeHandler"]
        #[derive(Debug)]
        struct Handler;

        impl Handler {
            #[unsafe(method(buttonClicked:))]
            fn button_clicked(&self, sender: *mut AnyObject) {
                let tag: isize = unsafe { msg_send![sender, tag] };
                match tag {
                    1 => open_screen_recording_settings(),
                    2 => open_accessibility_settings(),
                    9 => close_window(true),
                    _ => {}
                }
            }
        }
    );

    fn open_screen_recording_settings() {
        // Deep-link straight to System Settings → Privacy & Security →
        // Screen & System Audio Recording.
        let url = "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";
        let _ = std::process::Command::new("/usr/bin/open")
            .arg(url)
            .status();
    }

    fn open_accessibility_settings() {
        let url = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
        let _ = std::process::Command::new("/usr/bin/open")
            .arg(url)
            .status();
    }

    fn close_window(mark: bool) {
        WELCOME_WINDOW.with(|slot| {
            if let Some(window) = slot.borrow_mut().take() {
                unsafe {
                    window.orderOut(None);
                    window.close();
                }
            }
        });
        if mark {
            if let Some(path) = marker_path() {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&path, b"welcomed\n");
                crate::logging::event(serde_json::json!({
                    "evt": "welcomed",
                    "marker": path.display().to_string(),
                }));
            }
        }
    }

    const WIN_W: f64 = 540.0;
    const WIN_H: f64 = 480.0;
    const PAD: f64 = 24.0;

    fn build(mtm: MainThreadMarker, settings: &Settings) -> Result<Retained<NSWindow>, String> {
        let app = NSApplication::sharedApplication(mtm);
        unsafe {
            app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
            app.activate();
        }

        // Centre-ish of the main screen.
        let frame = NSRect {
            origin: NSPoint { x: 240.0, y: 240.0 },
            size: NSSize {
                width: WIN_W,
                height: WIN_H,
            },
        };
        let style =
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Resizable;
        let window: Retained<NSWindow> = unsafe {
            let alloc = NSWindow::alloc(mtm);
            msg_send![
                alloc,
                initWithContentRect: frame,
                styleMask: style,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };
        unsafe {
            window.setReleasedWhenClosed(false);
            window.setLevel(NSFloatingWindowLevel);
            window.setTitle(&NSString::from_str("Welcome to Screenshot Ultra"));
        }

        let content = window
            .contentView()
            .ok_or_else(|| "no content view".to_string())?;
        let handler: Retained<Handler> = unsafe { msg_send![Handler::alloc(), init] };

        // -- Headline ---------------------------------------------------
        let head_rect = NSRect {
            origin: NSPoint {
                x: PAD,
                y: WIN_H - 60.0,
            },
            size: NSSize {
                width: WIN_W - 2.0 * PAD,
                height: 32.0,
            },
        };
        let head = unsafe {
            NSTextField::labelWithString(&NSString::from_str("Welcome to Screenshot Ultra"), mtm)
        };
        unsafe {
            head.setFrame(head_rect);
            let font = objc2_app_kit::NSFont::boldSystemFontOfSize(22.0);
            head.setFont(Some(&font));
        }
        unsafe { content.addSubview(&head) };

        // -- Subhead body text -----------------------------------------
        let body_rect = NSRect {
            origin: NSPoint {
                x: PAD,
                y: WIN_H - 200.0,
            },
            size: NSSize {
                width: WIN_W - 2.0 * PAD,
                height: 130.0,
            },
        };
        let body =
            unsafe { NSTextField::labelWithString(&NSString::from_str(&body_copy(settings)), mtm) };
        unsafe {
            body.setFrame(body_rect);
            if let Some(cell) = body.cell() {
                let _: () = msg_send![&*cell, setWraps: true];
            }
            let font = objc2_app_kit::NSFont::systemFontOfSize(13.0);
            body.setFont(Some(&font));
            body.setTextColor(Some(&NSColor::secondaryLabelColor()));
        }
        unsafe { content.addSubview(&body) };

        // -- Hotkey list -----------------------------------------------
        let hot_rect = NSRect {
            origin: NSPoint {
                x: PAD,
                y: WIN_H - 320.0,
            },
            size: NSSize {
                width: WIN_W - 2.0 * PAD,
                height: 110.0,
            },
        };
        let hot = unsafe {
            NSTextField::labelWithString(&NSString::from_str(&hotkey_summary(settings)), mtm)
        };
        unsafe {
            hot.setFrame(hot_rect);
            if let Some(cell) = hot.cell() {
                let _: () = msg_send![&*cell, setWraps: true];
            }
            let font = objc2_app_kit::NSFont::userFixedPitchFontOfSize(12.0)
                .ok_or_else(|| "no fixed-pitch font".to_string())?;
            hot.setFont(Some(&font));
        }
        unsafe { content.addSubview(&hot) };

        // -- Permission buttons ----------------------------------------
        let make_button = |label: &str, tag: isize, x: f64, y: f64, w: f64| -> Retained<NSButton> {
            unsafe {
                let title = NSString::from_str(label);
                let rect = NSRect {
                    origin: NSPoint { x, y },
                    size: NSSize {
                        width: w,
                        height: 32.0,
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
        let screen_btn = make_button("Grant Screen Recording…", 1, PAD, 96.0, 220.0);
        let access_btn = make_button(
            "Grant Accessibility (optional)…",
            2,
            PAD + 220.0 + 8.0,
            96.0,
            260.0,
        );
        let done_btn = make_button("Got it — let's go", 9, (WIN_W - 200.0) / 2.0, 36.0, 200.0);
        unsafe {
            done_btn.setKeyEquivalent(&NSString::from_str("\r"));
            content.addSubview(&screen_btn);
            content.addSubview(&access_btn);
            content.addSubview(&done_btn);
        }

        unsafe {
            window.makeKeyAndOrderFront(None);
        }
        Ok(window)
    }

    fn body_copy(settings: &Settings) -> String {
        let save = settings.general.save_folder.clone();
        format!(
            "Screenshot Ultra runs as a tiny aperture-iris icon in your menu bar (top-right). \
             It needs Screen Recording permission to capture, and optional Accessibility \
             permission for the keystroke overlay during recordings.\n\n\
             Captures land in {save} unless you change it in Preferences (⌃⌥⌘,).\n\n\
             Press ⌃⌥⌘/ any time to see the cheat sheet."
        )
    }

    fn hotkey_summary(s: &Settings) -> String {
        let h = &s.hotkeys;
        format!(
            "  ⌃⌥⌘1   region capture     ({})\n\
             \x20 ⌃⌥⌘2   window capture     ({})\n\
             \x20 ⌃⌥⌘3   fullscreen          ({})\n\
             \x20 ⌃⌥⌘V   start/stop video    ({})\n\
             \x20 ⌃⌥⌘.   pin last capture    ({})",
            display(&h.region),
            display(&h.window),
            display(&h.fullscreen),
            display(&h.record_video),
            display(&h.pin_last),
        )
    }

    fn display(raw: &str) -> &str {
        if raw.is_empty() {
            "unbound"
        } else {
            raw
        }
    }
}
