// Eyedropper colour picker. Pressing the hotkey shows macOS's system
// `NSColorSampler` magnifier cursor; clicking on any on-screen pixel
// returns its colour. We convert to sRGB hex (`#rrggbb`), copy it onto
// the clipboard, and log an `eyedropper_pick` event.

#[cfg(not(target_os = "macos"))]
pub fn pick() {}

#[cfg(target_os = "macos")]
pub use mac::pick;

#[cfg(target_os = "macos")]
mod mac {
    #![allow(unused_unsafe)]

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::{msg_send, AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSColor, NSColorSampler, NSPasteboard,
    };
    use objc2_foundation::NSString;

    pub fn pick() {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("eyedropper::pick: not on main thread; skipping");
            return;
        };
        // Promote to accessory so the sampler magnifier paints over our app.
        let app = NSApplication::sharedApplication(mtm);
        unsafe {
            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
            app.activate();
        }

        let sampler: Retained<NSColorSampler> = unsafe { msg_send![NSColorSampler::alloc(), init] };

        let handler = RcBlock::new(|color: *mut NSColor| {
            if color.is_null() {
                return;
            }
            unsafe {
                let color = &*color;
                let hex = ns_color_to_hex(color);
                copy_string_to_clipboard(&hex);
                crate::logging::event(serde_json::json!({
                    "evt": "eyedropper_pick",
                    "hex": hex,
                }));
                eprintln!("eyedropper: copied {hex} to clipboard");
            }
        });

        unsafe { sampler.showSamplerWithSelectionHandler(&handler) };
    }

    /// Convert an arbitrary NSColor to a `#rrggbb` string by first asking
    /// it for its sRGB representation. Returns `#000000` if the colour
    /// space conversion fails (a black pixel is a sensible fallback).
    unsafe fn ns_color_to_hex(color: &NSColor) -> String {
        let space = objc2_app_kit::NSColorSpace::sRGBColorSpace();
        let converted: Option<Retained<NSColor>> = color.colorUsingColorSpace(&space);
        let c = converted.as_deref().unwrap_or(color);
        let r = (c.redComponent() * 255.0).round().clamp(0.0, 255.0) as u8;
        let g = (c.greenComponent() * 255.0).round().clamp(0.0, 255.0) as u8;
        let b = (c.blueComponent() * 255.0).round().clamp(0.0, 255.0) as u8;
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// Push a plain-text string onto the macOS clipboard (general
    /// pasteboard). Used to deposit the picked hex value for the user.
    unsafe fn copy_string_to_clipboard(s: &str) {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        let ns = NSString::from_str(s);
        // setString:forType: is the simplest path for plain text; the
        // typed binding wants NSPasteboardType which is an NSString alias.
        let kind = NSString::from_str("public.utf8-plain-text");
        let _: bool = msg_send![&*pb, setString: &*ns, forType: &*kind];
    }
}
