// Keystroke overlay. While a recording is active and
// `[recording].keystroke_overlay = true`, every keypress flashes briefly
// in a rounded pill at the bottom-centre of the screen. Great for
// screencasts where you want viewers to see which shortcut you just
// hit without having to narrate.
//
// Implementation:
//   - NSEvent::addGlobalMonitorForEventsMatchingMask installs a block
//     that receives keyDown + flagsChanged events globally. (Requires
//     Accessibility permission — granted automatically the first time
//     macOS prompts.)
//   - A transparent borderless NSWindow at the bottom-centre of the
//     main screen renders the most recent keystrokes via a custom
//     NSView that paints a rounded pill + the text.
//   - An NSTimer fades the pill out after `FADE_AFTER_MS` of no input.

#[cfg(not(target_os = "macos"))]
pub fn start() {}
#[cfg(not(target_os = "macos"))]
pub fn stop() {}

#[cfg(target_os = "macos")]
pub use mac::{start, stop};

#[cfg(target_os = "macos")]
mod mac {
    #![allow(unused_unsafe)]

    use std::cell::RefCell;
    use std::collections::VecDeque;

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, sel, AllocAnyThread, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSBackingStoreType, NSBezierPath, NSColor, NSEvent, NSEventMask, NSEventModifierFlags,
        NSScreen, NSStatusWindowLevel, NSView, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize, NSString, NSTimer};

    /// How long a key sticks around before fading.
    const FADE_AFTER_MS: u128 = 2000;
    /// Max number of key chunks displayed at once.
    const MAX_KEYS: usize = 8;
    const PANEL_H: f64 = 56.0;
    const PANEL_MAX_W: f64 = 720.0;
    const PANEL_MARGIN_BOTTOM: f64 = 90.0;

    struct OverlayState {
        window: Retained<NSWindow>,
        view: Retained<PillView>,
        monitor: Retained<AnyObject>,
        /// Currently-displayed key chunks + the timestamp each was added.
        keys: VecDeque<(String, std::time::Instant)>,
        /// Timer that polls every ~100 ms to fade old keys out.
        _timer: Retained<NSTimer>,
        _handler: Retained<TickHandler>,
    }

    thread_local! {
        static OVERLAY: RefCell<Option<OverlayState>> = const { RefCell::new(None) };
    }

    pub fn start() {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("keystroke_overlay::start: not on main thread; skipping");
            return;
        };
        // Already running?
        if OVERLAY.with(|c| c.borrow().is_some()) {
            return;
        }
        match build(mtm) {
            Ok(state) => OVERLAY.with(|c| *c.borrow_mut() = Some(state)),
            Err(err) => eprintln!("keystroke_overlay: build failed: {err}"),
        }
    }

    pub fn stop() {
        OVERLAY.with(|slot| {
            if let Some(state) = slot.borrow_mut().take() {
                unsafe {
                    NSEvent::removeMonitor(&state.monitor);
                    state.window.orderOut(None);
                    state.window.close();
                }
            }
        });
    }

    define_class!(
        /// Renders the current keystroke pill — a rounded dark-glass
        /// rectangle with the recent key glyphs typeset inside.
        #[unsafe(super(NSView))]
        #[name = "STUKeystrokePill"]
        #[derive(Debug)]
        struct PillView;

        impl PillView {
            #[unsafe(method(isOpaque))]
            fn is_opaque(&self) -> bool {
                false
            }

            #[unsafe(method(drawRect:))]
            fn draw_rect(&self, _dirty: NSRect) {
                draw_overlay(self);
            }
        }
    );

    define_class!(
        /// Holds the NSTimer callback that fades old keys out and asks
        /// the view to redraw.
        #[unsafe(super(NSObject))]
        #[name = "STUKeystrokeTick"]
        #[derive(Debug)]
        struct TickHandler;

        impl TickHandler {
            #[unsafe(method(tick:))]
            fn tick(&self, _timer: *mut AnyObject) {
                let needs_redraw = OVERLAY.with(|slot| {
                    let mut g = slot.borrow_mut();
                    let Some(state) = g.as_mut() else { return false };
                    let now = std::time::Instant::now();
                    let before = state.keys.len();
                    state
                        .keys
                        .retain(|(_, ts)| now.duration_since(*ts).as_millis() < FADE_AFTER_MS);
                    state.keys.len() != before
                });
                if needs_redraw {
                    OVERLAY.with(|slot| {
                        if let Some(state) = slot.borrow().as_ref() {
                            unsafe { state.view.setNeedsDisplay(true) };
                        }
                    });
                }
            }
        }
    );

    fn build(mtm: MainThreadMarker) -> Result<OverlayState, String> {
        // Position bottom-centre of the main screen.
        let screen_frame = NSScreen::mainScreen(mtm)
            .map(|s| s.frame())
            .ok_or_else(|| "no main screen".to_string())?;
        let win_w = (screen_frame.size.width * 0.6).clamp(360.0, PANEL_MAX_W);
        let origin = NSPoint {
            x: screen_frame.origin.x + (screen_frame.size.width - win_w) / 2.0,
            y: screen_frame.origin.y + PANEL_MARGIN_BOTTOM,
        };
        let rect = NSRect {
            origin,
            size: NSSize {
                width: win_w,
                height: PANEL_H,
            },
        };

        // Borderless transparent click-through window.
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
            window.setLevel(NSStatusWindowLevel);
            window.setOpaque(false);
            window.setBackgroundColor(Some(&NSColor::clearColor()));
            window.setIgnoresMouseEvents(true);
            window.setHasShadow(false);
            window.setReleasedWhenClosed(false);
        }

        // Custom pill view fills the content area.
        let view_rect = NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: rect.size,
        };
        let view: Retained<PillView> = unsafe {
            let alloc = PillView::alloc(mtm);
            msg_send![alloc, initWithFrame: view_rect]
        };
        if let Some(content) = window.contentView() {
            unsafe { content.addSubview(&view) };
        }
        unsafe { window.orderFrontRegardless() };

        // Global key monitor. Block runs on the main thread.
        let block = RcBlock::new(|event_ptr: std::ptr::NonNull<NSEvent>| {
            let s = format_event(unsafe { event_ptr.as_ref() });
            if s.is_empty() {
                return;
            }
            push_key(s);
        });
        let mask = NSEventMask::KeyDown | NSEventMask::FlagsChanged;
        let monitor: Retained<AnyObject> = match NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &block) {
            Some(m) => m,
            None => {
                return Err(
                    "NSEvent global monitor returned None — likely missing Accessibility permission"
                        .to_string(),
                )
            }
        };

        // Tick timer to expire old keys.
        let handler: Retained<TickHandler> = unsafe { msg_send![TickHandler::alloc(), init] };
        let timer: Retained<NSTimer> = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                0.1,
                handler.as_ref(),
                sel!(tick:),
                None,
                true, // repeating
            )
        };

        Ok(OverlayState {
            window,
            view,
            monitor,
            keys: VecDeque::new(),
            _timer: timer,
            _handler: handler,
        })
    }

    /// Push a chunk into the keystroke buffer and request a redraw.
    fn push_key(s: String) {
        let now = std::time::Instant::now();
        OVERLAY.with(|slot| {
            let mut g = slot.borrow_mut();
            let Some(state) = g.as_mut() else { return };
            state.keys.push_back((s, now));
            while state.keys.len() > MAX_KEYS {
                state.keys.pop_front();
            }
            unsafe { state.view.setNeedsDisplay(true) };
        });
    }

    /// Build a display string for an NSEvent — either a glyph like
    /// "⌘C" / "↩" / "⌫" or a small token like "Tab", "Space". Returns
    /// empty for events we don't want to show (pure modifier-only
    /// flagsChanged with no character, for instance).
    fn format_event(event: &NSEvent) -> String {
        let mods = unsafe { event.modifierFlags() };
        let cmd = mods.contains(NSEventModifierFlags::Command);
        let opt = mods.contains(NSEventModifierFlags::Option);
        let ctrl = mods.contains(NSEventModifierFlags::Control);
        let shift = mods.contains(NSEventModifierFlags::Shift);

        let chars = unsafe { event.charactersIgnoringModifiers() };
        let raw = chars.map(|c| c.to_string()).unwrap_or_default();
        let glyph = key_glyph(&raw);

        // If only modifier glyphs (no actual key), suppress to avoid
        // spamming the overlay every time the user presses Cmd.
        if glyph.is_empty() {
            return String::new();
        }

        let mut out = String::new();
        if ctrl {
            out.push('⌃');
        }
        if opt {
            out.push('⌥');
        }
        if shift {
            out.push('⇧');
        }
        if cmd {
            out.push('⌘');
        }
        out.push_str(&glyph);
        out
    }

    fn key_glyph(raw: &str) -> String {
        match raw {
            "" => String::new(),
            " " => "Space".to_string(),
            "\t" => "Tab".to_string(),
            "\r" | "\n" => "↩".to_string(),
            "\u{1b}" => "Esc".to_string(),
            "\u{7f}" | "\u{8}" => "⌫".to_string(),
            other => {
                // Arrow keys come through as multi-byte private-use codes.
                // F-keys are also private-use. Map a few common ones.
                let first = other.chars().next().unwrap_or('\0') as u32;
                match first {
                    0xF700 => "↑".into(),
                    0xF701 => "↓".into(),
                    0xF702 => "←".into(),
                    0xF703 => "→".into(),
                    0xF704..=0xF74F => format!("F{}", first - 0xF703),
                    _ => other.to_uppercase(),
                }
            }
        }
    }

    /// Draw the rounded dark-glass pill + the recent key chunks.
    fn draw_overlay(view: &PillView) {
        let keys = OVERLAY.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|s| s.keys.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        });
        if keys.is_empty() {
            return;
        }
        let text = keys.join("   ");

        let bounds = unsafe { view.bounds() };

        // Pill geometry — auto-sizes to text width, centred horizontally.
        let pad_x = 18.0_f64;
        let pad_y = 10.0_f64;
        let font_size = 22.0_f64;
        let approx_w = (text.chars().count() as f64) * (font_size * 0.65);
        let pill_w = (approx_w + pad_x * 2.0).min(bounds.size.width - 12.0);
        let pill_h = font_size + pad_y * 2.0;
        let pill_rect = NSRect {
            origin: NSPoint {
                x: (bounds.size.width - pill_w) / 2.0,
                y: (bounds.size.height - pill_h) / 2.0,
            },
            size: NSSize {
                width: pill_w,
                height: pill_h,
            },
        };

        unsafe {
            // Rounded dark-glass background.
            let bg = NSColor::colorWithSRGBRed_green_blue_alpha(0.05, 0.06, 0.08, 0.78);
            bg.setFill();
            let path = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                pill_rect,
                pill_h / 2.0,
                pill_h / 2.0,
            );
            path.fill();

            // Brand-coloured 1-px outline.
            let stroke = NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 0.24, 0.33, 0.8);
            stroke.setStroke();
            path.setLineWidth(1.0);
            path.stroke();

            // Text. Draw via raw msg_send to side-step the strict
            // NSDictionary<NSAttributedStringKey, AnyObject> generics on
            // -[NSString drawAtPoint:withAttributes:].
            let font = objc2_app_kit::NSFont::boldSystemFontOfSize(font_size);
            let white = NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 1.0, 1.0, 1.0);
            let keys: [&NSObject; 2] = [
                &*NSString::from_str("NSFont") as &NSObject,
                &*NSString::from_str("NSColor") as &NSObject,
            ];
            let vals: [&NSObject; 2] = [(&*font) as &NSObject, (&*white) as &NSObject];
            let attrs: Retained<NSObject> = msg_send![
                objc2::class!(NSDictionary),
                dictionaryWithObjects: vals.as_ptr(),
                forKeys: keys.as_ptr(),
                count: 2usize,
            ];
            let ns = NSString::from_str(&text);
            // sizeWithAttributes: gives us the rendered width so we can centre.
            let size: NSSize = msg_send![&*ns, sizeWithAttributes: &*attrs];
            let origin = NSPoint {
                x: pill_rect.origin.x + (pill_w - size.width) / 2.0,
                y: pill_rect.origin.y + (pill_h - size.height) / 2.0,
            };
            let _: () = msg_send![&*ns, drawAtPoint: origin, withAttributes: &*attrs];
        }
    }
}
