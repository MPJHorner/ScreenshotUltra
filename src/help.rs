// "Cheat sheet" window — a single NSWindow showing every global hotkey
// and every editor shortcut, rendered as a read-only NSTextView so users
// can scroll, search (⌘F), and copy lines if they want.
//
// The content is generated from the *live* Settings struct, so a user's
// rebound hotkeys show up here verbatim — this isn't a hard-coded docs
// dump. Editor shortcuts are static because they aren't currently
// rebindable.

#[cfg(not(target_os = "macos"))]
pub fn show(_s: &crate::settings::Settings) {}

#[cfg(target_os = "macos")]
pub use mac::show;

#[cfg(target_os = "macos")]
mod mac {
    #![allow(unused_unsafe)]

    use std::cell::RefCell;

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{msg_send, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSScrollView, NSTextView,
        NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

    use crate::settings::Settings;

    thread_local! {
        static HELP_WINDOW: RefCell<Option<Retained<NSWindow>>> = const { RefCell::new(None) };
    }

    pub fn show(s: &Settings) {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };

        // Re-open the existing window if there is one.
        let already = HELP_WINDOW.with(|c| c.borrow().is_some());
        if already {
            HELP_WINDOW.with(|c| {
                if let Some(w) = c.borrow().as_ref() {
                    unsafe { w.makeKeyAndOrderFront(None) };
                }
            });
            return;
        }

        match build(mtm, s) {
            Ok(w) => HELP_WINDOW.with(|c| *c.borrow_mut() = Some(w)),
            Err(err) => eprintln!("help: build failed: {err}"),
        }
    }

    fn build(mtm: MainThreadMarker, s: &Settings) -> Result<Retained<NSWindow>, String> {
        let app = NSApplication::sharedApplication(mtm);
        unsafe {
            app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
            app.activate();
        }

        let frame = NSRect {
            origin: NSPoint { x: 240.0, y: 240.0 },
            size: NSSize {
                width: 560.0,
                height: 640.0,
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
            window.setTitle(&NSString::from_str("Screenshot Ultra — Cheat Sheet"));
        }

        let content = window
            .contentView()
            .ok_or_else(|| "no content view".to_string())?;
        let bounds = NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: NSSize {
                width: 560.0,
                height: 640.0,
            },
        };

        let scroll: Retained<NSScrollView> = unsafe {
            let v = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), bounds);
            v.setHasVerticalScroller(true);
            v.setAutoresizingMask(
                objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
                    | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
            );
            v
        };

        let body = render_cheat_sheet(s);
        let text_view: Retained<NSTextView> = unsafe {
            let tv: Retained<NSTextView> =
                NSTextView::initWithFrame(NSTextView::alloc(mtm), bounds);
            tv.setEditable(false);
            tv.setRichText(false);
            tv.setFont(Some(
                &objc2_app_kit::NSFont::userFixedPitchFontOfSize(13.0).unwrap(),
            ));
            tv.setVerticallyResizable(true);
            tv.setHorizontallyResizable(false);
            tv.setAutoresizingMask(objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable);
            let _: () = msg_send![&*tv, setString: &*NSString::from_str(&body)];
            tv
        };

        unsafe {
            let _: () = msg_send![&*scroll, setDocumentView: &*text_view as &AnyObject];
            content.addSubview(&scroll);
            window.makeKeyAndOrderFront(None);
        }

        Ok(window)
    }

    /// Build a fixed-width "two-column" cheat sheet from the live settings.
    /// We render with whitespace alignment so it reads well in the
    /// monospaced NSTextView. Headings are surrounded by separator lines.
    fn render_cheat_sheet(s: &Settings) -> String {
        let mut out = String::new();
        let h = &s.hotkeys;

        // Global hotkeys section — populated from `Settings`, so a user's
        // rebound bindings appear verbatim. Empty (unbound) slots are
        // displayed as "—" for clarity.
        out.push_str("══════════════════════════════════════════════════════════\n");
        out.push_str("  GLOBAL HOTKEYS\n");
        out.push_str("══════════════════════════════════════════════════════════\n\n");
        let rows: [(&str, &str); 13] = [
            ("Region capture (Quick Tray)", &h.region),
            ("Window capture (Quick Tray)", &h.window),
            ("Fullscreen capture (Quick Tray)", &h.fullscreen),
            ("Region capture (silent)", &h.silent_region),
            ("Window capture (silent)", &h.silent_window),
            ("Fullscreen capture (silent)", &h.silent_fullscreen),
            ("Open clipboard image", &h.open_clipboard_image),
            ("Repeat last capture", &h.repeat_last),
            ("Pin last to screen", &h.pin_last),
            ("Colour picker (eyedropper)", &h.color_picker),
            ("Record video (toggle)", &h.record_video),
            ("Record GIF (toggle)", &h.record_gif),
            ("Preferences", &h.preferences),
        ];
        for (action, binding) in rows {
            let display = if binding.is_empty() {
                "—".to_string()
            } else {
                format_accel(binding)
            };
            out.push_str(&format!("  {:<33}  {}\n", action, display));
        }

        out.push_str("\n══════════════════════════════════════════════════════════\n");
        out.push_str("  EDITOR — TOOLS\n");
        out.push_str("══════════════════════════════════════════════════════════\n\n");
        for (key, tool) in [
            ("P", "Pen — freehand stroke"),
            ("L", "Line"),
            ("A", "Arrow (with calculated arrowhead)"),
            ("R", "Rectangle (stroked)"),
            ("E", "Ellipse (stroked)"),
            ("H", "Highlighter (translucent yellow)"),
            ("X", "Redact (filled black)"),
            ("N", "Counter (click to drop numbered circle)"),
            ("T", "Text (click prompts for string)"),
            ("B", "Blur (pixelate region)"),
            ("C", "Crop — drag, release to apply. Destructive."),
        ] {
            out.push_str(&format!("  {:<5}  {}\n", key, tool));
        }

        out.push_str("\n══════════════════════════════════════════════════════════\n");
        out.push_str("  EDITOR — STYLE\n");
        out.push_str("══════════════════════════════════════════════════════════\n\n");
        for (key, what) in [
            (
                "Colours",
                "Red / Yellow / Green / Blue / Black (click swatch)",
            ),
            (
                "1 / 2 / 3",
                "Stroke width — Thin (3 px) / Med (6 px) / Thick (12 px)",
            ),
        ] {
            out.push_str(&format!("  {:<11}  {}\n", key, what));
        }

        out.push_str("\n══════════════════════════════════════════════════════════\n");
        out.push_str("  EDITOR — ACTIONS\n");
        out.push_str("══════════════════════════════════════════════════════════\n\n");
        for (key, what) in [
            ("⌘S", "Save annotated PNG over original capture"),
            ("⌘C", "Copy annotated image to clipboard"),
            ("⌘Z", "Undo last shape"),
            ("⌘⇧Z", "Redo"),
            ("⌘W", "Close editor"),
        ] {
            out.push_str(&format!("  {:<5}  {}\n", key, what));
        }

        out.push_str("\n══════════════════════════════════════════════════════════\n");
        out.push_str("  PIN-TO-SCREEN\n");
        out.push_str("══════════════════════════════════════════════════════════\n\n");
        for (key, what) in [
            ("scroll", "Adjust opacity (0.3 ↔ 1.0)"),
            ("⌘+", "Zoom in"),
            ("⌘-", "Zoom out"),
            ("⌘0", "Reset opacity"),
            ("⌫ / Esc", "Close pin"),
        ] {
            out.push_str(&format!("  {:<8}  {}\n", key, what));
        }

        out.push_str("\n══════════════════════════════════════════════════════════\n");
        out.push_str("  QUICK TRAY (after each capture)\n");
        out.push_str("══════════════════════════════════════════════════════════\n\n");
        for (label, what) in [
            ("Copy", "Place the image on the clipboard"),
            ("Edit", "Open the native annotation editor"),
            ("Folder", "Open the save folder in Finder"),
            ("Reveal", "Select the file in Finder"),
            ("Pin", "Pin to screen"),
            ("Discard", "Delete the file"),
        ] {
            out.push_str(&format!("  {:<8}  {}\n", label, what));
        }

        out.push('\n');
        out.push_str("Edit ~/Library/Application Support/ScreenshotUltra/settings.toml\n");
        out.push_str("to rebind any hotkey. Changes apply within ~1 second.\n");
        out
    }

    /// Translate an internal binding like "ctrl+alt+cmd+1" into the
    /// glyph-y form macOS users recognise: "⌃⌥⌘1".
    fn format_accel(raw: &str) -> String {
        let mut out = String::new();
        for part in raw.split('+') {
            let token = part.trim();
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => out.push('⌃'),
                "alt" | "option" | "opt" => out.push('⌥'),
                "shift" => out.push('⇧'),
                "cmd" | "command" | "super" | "meta" => out.push('⌘'),
                "period" => out.push('.'),
                "comma" => out.push(','),
                "slash" => out.push('/'),
                "backslash" => out.push('\\'),
                "space" => out.push_str("space"),
                "tab" => out.push_str("tab"),
                "enter" | "return" => out.push('⏎'),
                "esc" | "escape" => out.push_str("esc"),
                other => out.push_str(&other.to_uppercase()),
            }
        }
        out
    }
}
