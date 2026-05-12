// History window. Reads the per-folder NDJSON index
// (`<save_folder>/.screenshot-ultra/index.ndjson`) and renders it as a
// scrollable, searchable table in a native NSWindow.
//
// The content is a fixed-width text dump rather than a real NSTableView
// because text is searchable via ⌘F out of the box, the same set-up
// already works in Preferences / Help / About windows, and the
// rendering surface is trivially extensible (we can add OCR snippets,
// thumbnails, or per-row buttons later without rewriting the chrome).

#[cfg(not(target_os = "macos"))]
pub fn show(_s: &crate::settings::Settings) {}

#[cfg(target_os = "macos")]
pub use mac::show;

#[cfg(target_os = "macos")]
mod mac {
    #![allow(unused_unsafe)]

    use std::cell::RefCell;
    use std::path::PathBuf;

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
        static HISTORY_WINDOW: RefCell<Option<Retained<NSWindow>>> = const { RefCell::new(None) };
    }

    pub fn show(s: &Settings) {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };

        // Re-open an existing window instead of stacking duplicates.
        let already = HISTORY_WINDOW.with(|c| c.borrow().is_some());
        if already {
            HISTORY_WINDOW.with(|c| {
                if let Some(w) = c.borrow().as_ref() {
                    unsafe { w.makeKeyAndOrderFront(None) };
                }
            });
            return;
        }

        match build(mtm, s) {
            Ok(w) => HISTORY_WINDOW.with(|c| *c.borrow_mut() = Some(w)),
            Err(err) => eprintln!("history: build failed: {err}"),
        }
    }

    fn build(mtm: MainThreadMarker, s: &Settings) -> Result<Retained<NSWindow>, String> {
        let app = NSApplication::sharedApplication(mtm);
        unsafe {
            app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
            app.activate();
        }

        let frame = NSRect {
            origin: NSPoint { x: 280.0, y: 200.0 },
            size: NSSize {
                width: 760.0,
                height: 600.0,
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
            window.setTitle(&NSString::from_str("Screenshot Ultra — History"));
        }

        let content = window
            .contentView()
            .ok_or_else(|| "no content view".to_string())?;
        let bounds = NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: frame.size,
        };

        let scroll: Retained<NSScrollView> = unsafe {
            let v = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), bounds);
            v.setHasVerticalScroller(true);
            v.setHasHorizontalScroller(false);
            v.setAutoresizingMask(
                objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable
                    | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable,
            );
            v
        };

        let body = render(s);
        let text_view: Retained<NSTextView> = unsafe {
            let tv: Retained<NSTextView> =
                NSTextView::initWithFrame(NSTextView::alloc(mtm), bounds);
            tv.setEditable(false);
            tv.setRichText(false);
            tv.setFont(Some(
                &objc2_app_kit::NSFont::userFixedPitchFontOfSize(12.0).unwrap(),
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

    /// Read the per-folder history index and format it. Most recent
    /// first. Falls back to a clear "no captures yet" message when the
    /// index doesn't exist (first-run, or user deleted the folder).
    fn render(s: &Settings) -> String {
        let folder = s.general.save_folder_expanded();
        let path: PathBuf = folder.join(".screenshot-ultra/index.ndjson");
        let mut out = String::new();
        out.push_str("══════════════════════════════════════════════════════════════════════\n");
        out.push_str(&format!("  HISTORY — {}\n", folder.display()));
        out.push_str("══════════════════════════════════════════════════════════════════════\n\n");

        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => {
                out.push_str("No history yet.\n\n");
                out.push_str("Captures land in this folder. Once you've taken your first\n");
                out.push_str("screenshot, the index file at\n");
                out.push_str(&format!("  {}\n", path.display()));
                out.push_str("will start filling up. Tip: tail it with `jq` for live updates.\n");
                return out;
            }
        };

        // The index is append-only; parse each non-empty line.
        let mut rows: Vec<serde_json::Value> = raw
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        // Most recent first — newest captures land at the bottom of the file.
        rows.reverse();

        if rows.is_empty() {
            out.push_str("The history index exists but is empty.\n");
            return out;
        }

        out.push_str(&format!("  {} captures total\n", rows.len()));
        out.push_str("  Tip: ⌘F to search this view.\n\n");
        out.push_str(&format!(
            "  {:<19}  {:<10}  {:>8}  {}\n",
            "TIMESTAMP", "MODE", "BYTES", "FILE"
        ));
        out.push_str("  ──────────────────────────────────────────────────────────────────\n");

        for row in &rows {
            let ts = row.get("ts").and_then(|v| v.as_str()).unwrap_or("—");
            let mode = row
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or(row.get("kind").and_then(|v| v.as_str()).unwrap_or("—"));
            let bytes = row.get("bytes").and_then(|v| v.as_u64()).unwrap_or(0);
            let saved = row.get("saved_to").and_then(|v| v.as_str()).unwrap_or("—");
            let pretty_ts = ts.split('.').next().unwrap_or(ts);
            let file_name = saved.rsplit('/').next().unwrap_or(saved);
            out.push_str(&format!(
                "  {:<19}  {:<10}  {:>8}  {}\n",
                pretty_ts,
                mode,
                pretty_bytes(bytes),
                file_name
            ));
        }

        out.push_str("\nFull paths above are relative to the save folder header.\n");
        out
    }

    fn pretty_bytes(b: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * 1024;
        if b >= MB {
            format!("{:.1} M", b as f64 / MB as f64)
        } else if b >= KB {
            format!("{:.0} K", b as f64 / KB as f64)
        } else {
            format!("{} B", b)
        }
    }
}
