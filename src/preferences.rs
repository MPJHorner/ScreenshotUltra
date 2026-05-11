// Preferences window. A native NSWindow with a scrollable NSTextView
// pre-populated with the current settings.toml. The user edits in place
// and clicks Apply to write back to disk; the file watcher in main.rs
// picks it up via the same hot-reload path that handles external edits.
//
// Why a TOML editor instead of a hand-rolled grid of controls? The
// settings file is already the source of truth, has comments, is
// diffable and AI-editable, and any new setting we add lights up here
// for free. The window is just a styled, in-app version of "Edit
// settings.toml…" — but with Apply / Reset / Cancel buttons and live
// validation.

#[cfg(not(target_os = "macos"))]
pub fn show() {}

#[cfg(target_os = "macos")]
pub use mac::show;

#[cfg(target_os = "macos")]
mod mac {
    #![allow(unused_unsafe)]

    use std::cell::RefCell;

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, sel, AllocAnyThread, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSBezelStyle, NSButton,
        NSColor, NSScrollView, NSTextField, NSTextView, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

    struct PrefsState {
        window: Retained<NSWindow>,
        text_view: Retained<NSTextView>,
        _handler: Retained<Handler>,
    }

    thread_local! {
        static PREFS: RefCell<Option<PrefsState>> = const { RefCell::new(None) };
    }

    pub fn show() {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("preferences::show: not on main thread; skipping");
            return;
        };

        // Bring an existing window to front rather than building a second one.
        let already_open = PREFS.with(|p| p.borrow().is_some());
        if already_open {
            PREFS.with(|p| {
                if let Some(state) = p.borrow().as_ref() {
                    unsafe { state.window.makeKeyAndOrderFront(None) };
                }
            });
            return;
        }

        match build(mtm) {
            Ok(state) => PREFS.with(|p| *p.borrow_mut() = Some(state)),
            Err(err) => eprintln!("preferences: build failed: {err}"),
        }
    }

    define_class!(
        /// Routes button clicks (Apply / Reset / Cancel) to the right action.
        #[unsafe(super(objc2_foundation::NSObject))]
        #[name = "STUPrefsHandler"]
        #[derive(Debug)]
        struct Handler;

        impl Handler {
            #[unsafe(method(buttonClicked:))]
            fn button_clicked(&self, sender: *mut AnyObject) {
                let tag: isize = unsafe { msg_send![sender, tag] };
                match tag {
                    1 => apply_clicked(),
                    2 => reset_clicked(),
                    3 => cancel_clicked(),
                    _ => {}
                }
            }
        }
    );

    /// Apply: collect the editor's text, write to `Settings::path()`. The
    /// background watcher in main.rs picks up the change within ~1 s; we
    /// just need to make sure the syntax is parseable first so a typo
    /// doesn't silently revert.
    fn apply_clicked() {
        let text = current_text();
        match toml::from_str::<crate::settings::Settings>(&text) {
            Ok(_) => match crate::settings::Settings::path() {
                Ok(path) => {
                    if let Err(err) = std::fs::write(&path, text) {
                        warn_alert("Couldn't save settings", &format!("{err:#}"));
                        return;
                    }
                    crate::logging::event(serde_json::json!({
                        "evt": "preferences_apply",
                        "path": path.display().to_string(),
                    }));
                    close();
                }
                Err(err) => warn_alert("Couldn't resolve settings path", &format!("{err:#}")),
            },
            Err(err) => {
                warn_alert(
                    "Settings won't parse",
                    &format!("Fix the TOML before applying.\n\n{err}"),
                );
            }
        }
    }

    /// Reset: replace the editor's text with the default-settings TOML.
    /// Does NOT touch disk until the user clicks Apply.
    fn reset_clicked() {
        let defaults = crate::settings::Settings::default();
        let Ok(raw) = toml::to_string_pretty(&defaults) else {
            return;
        };
        set_text(&raw);
    }

    fn cancel_clicked() {
        close();
    }

    fn close() {
        PREFS.with(|slot| {
            if let Some(state) = slot.borrow_mut().take() {
                unsafe {
                    state.window.orderOut(None);
                    state.window.close();
                }
            }
        });
    }

    fn current_text() -> String {
        PREFS.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|s| unsafe { s.text_view.string().to_string() })
                .unwrap_or_default()
        })
    }

    fn set_text(text: &str) {
        PREFS.with(|slot| {
            if let Some(state) = slot.borrow().as_ref() {
                let ns = NSString::from_str(text);
                unsafe {
                    let _: () = msg_send![&*state.text_view, setString: &*ns];
                }
            }
        });
    }

    fn warn_alert(title: &str, body: &str) {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let alert: Retained<objc2_app_kit::NSAlert> =
            unsafe { msg_send![objc2_app_kit::NSAlert::alloc(mtm), init] };
        unsafe {
            alert.setMessageText(&NSString::from_str(title));
            alert.setInformativeText(&NSString::from_str(body));
            alert.addButtonWithTitle(&NSString::from_str("OK"));
            alert.runModal();
        }
    }

    const WIN_W: f64 = 720.0;
    const WIN_H: f64 = 540.0;
    const BAR_H: f64 = 48.0;
    const PAD: f64 = 14.0;

    fn build(mtm: MainThreadMarker) -> Result<PrefsState, String> {
        let app = NSApplication::sharedApplication(mtm);
        unsafe {
            app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
            app.activate();
        }

        // Read current settings.toml. If it doesn't exist, use the defaults
        // (Settings::load_or_default writes them out as a side effect, which
        // is what we want).
        let settings = crate::settings::Settings::load_or_default().map_err(|e| e.to_string())?;
        let raw = toml::to_string_pretty(&settings).map_err(|e| e.to_string())?;

        let frame = NSRect {
            origin: NSPoint { x: 200.0, y: 200.0 },
            size: NSSize {
                width: WIN_W,
                height: WIN_H,
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
                initWithContentRect: frame,
                styleMask: style,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };
        unsafe {
            window.setReleasedWhenClosed(false);
            window.setTitle(&NSString::from_str("Screenshot Ultra — Preferences"));
        }

        let content = window
            .contentView()
            .ok_or_else(|| "no content view".to_string())?;

        // Header label
        let header_rect = NSRect {
            origin: NSPoint {
                x: PAD,
                y: WIN_H - 28.0,
            },
            size: NSSize {
                width: WIN_W - 2.0 * PAD,
                height: 18.0,
            },
        };
        let header = unsafe {
            NSTextField::labelWithString(&NSString::from_str(
            "Edit settings.toml below. Changes are validated, then saved to ~/Library/Application Support/ScreenshotUltra/settings.toml — the running app picks them up within ~1 s.",
        ), mtm)
        };
        unsafe {
            header.setFrame(header_rect);
            // Wrap the helper text across lines instead of clipping.
            let cell = header.cell();
            if let Some(cell) = cell {
                let _: () = msg_send![&*cell, setWraps: true];
            }
            header.setAutoresizingMask(objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable);
        }
        unsafe { content.addSubview(&header) };

        // Scrollable text editor.
        let editor_rect = NSRect {
            origin: NSPoint {
                x: PAD,
                y: BAR_H + PAD,
            },
            size: NSSize {
                width: WIN_W - 2.0 * PAD,
                height: WIN_H - BAR_H - 2.0 * PAD - 36.0,
            },
        };
        let scroll: Retained<NSScrollView> = unsafe {
            let v = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), editor_rect);
            v.setHasVerticalScroller(true);
            v.setHasHorizontalScroller(false);
            v.setAutoresizingMask(
                objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
                    | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
            );
            v
        };

        let text_view: Retained<NSTextView> = unsafe {
            let inner_rect = NSRect {
                origin: NSPoint { x: 0.0, y: 0.0 },
                size: editor_rect.size,
            };
            let tv: Retained<NSTextView> =
                NSTextView::initWithFrame(NSTextView::alloc(mtm), inner_rect);
            tv.setEditable(true);
            tv.setRichText(false);
            tv.setAllowsUndo(true);
            tv.setFont(Some(
                &objc2_app_kit::NSFont::userFixedPitchFontOfSize(13.0)
                    .expect("system fixed-pitch font should exist"),
            ));
            tv.setTextColor(Some(&NSColor::labelColor()));
            tv.setAutomaticQuoteSubstitutionEnabled(false);
            tv.setAutomaticDashSubstitutionEnabled(false);
            tv.setAutomaticSpellingCorrectionEnabled(false);
            tv.setAutomaticTextReplacementEnabled(false);
            tv.setAutomaticLinkDetectionEnabled(false);
            tv.setMinSize(NSSize {
                width: 0.0,
                height: 0.0,
            });
            tv.setMaxSize(NSSize {
                width: f64::MAX,
                height: f64::MAX,
            });
            tv.setVerticallyResizable(true);
            tv.setHorizontallyResizable(false);
            tv.setAutoresizingMask(objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable);
            let _: () = msg_send![&*tv, setString: &*NSString::from_str(&raw)];
            tv
        };
        unsafe {
            // NSScrollView's documentView slot wants a generic NSView pointer;
            // use raw msg_send to avoid wrestling with the typed binding.
            let _: () = msg_send![&*scroll, setDocumentView: &*text_view as &AnyObject];
            content.addSubview(&scroll);
        }

        // Bottom button bar.
        let handler: Retained<Handler> = unsafe { msg_send![Handler::alloc(), init] };

        let make_button =
            |label: &str, tooltip: &str, tag: isize, x: f64, w: f64| -> Retained<NSButton> {
                unsafe {
                    let title = NSString::from_str(label);
                    let tip = NSString::from_str(tooltip);
                    let rect = NSRect {
                        origin: NSPoint { x, y: 12.0 },
                        size: NSSize {
                            width: w,
                            height: 28.0,
                        },
                    };
                    let b: Retained<NSButton> = NSButton::initWithFrame(NSButton::alloc(mtm), rect);
                    b.setTitle(&title);
                    b.setBezelStyle(NSBezelStyle::Push);
                    b.setTag(tag);
                    b.setTarget(Some(handler.as_ref()));
                    b.setAction(Some(sel!(buttonClicked:)));
                    b.setToolTip(Some(&tip));
                    b
                }
            };

        let apply = make_button(
            "Apply",
            "Validate the TOML then save to settings.toml (⌘S)",
            1,
            WIN_W - PAD - 100.0,
            100.0,
        );
        let cancel = make_button(
            "Cancel",
            "Discard changes and close (⌘W)",
            3,
            WIN_W - PAD - 100.0 - 8.0 - 90.0,
            90.0,
        );
        let reset = make_button("Reset to Defaults", "Replace the editor contents with the default settings.toml. Does not save until you click Apply.", 2, PAD, 160.0);
        unsafe {
            // Apply is the default button (Return triggers it).
            apply.setKeyEquivalent(&NSString::from_str("\r"));
            cancel.setKeyEquivalent(&NSString::from_str("\u{1b}"));
            content.addSubview(&apply);
            content.addSubview(&cancel);
            content.addSubview(&reset);
        }

        unsafe {
            window.makeKeyAndOrderFront(None);
            window.makeFirstResponder(Some(&text_view));
        }

        Ok(PrefsState {
            window,
            text_view,
            _handler: handler,
        })
    }
}
