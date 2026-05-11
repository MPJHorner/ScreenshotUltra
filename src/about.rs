// "About" window — a tiny native NSAlert showing version, repo URL, and
// a quick description. No heavy GUI needed for v0.

#[cfg(not(target_os = "macos"))]
pub fn show() {}

#[cfg(target_os = "macos")]
pub use mac::show;

#[cfg(target_os = "macos")]
mod mac {
    #![allow(unused_unsafe)]

    use objc2::{msg_send, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSAlert, NSApplication, NSApplicationActivationPolicy};
    use objc2_foundation::{NSString, NSURL};

    pub fn show() {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let app = NSApplication::sharedApplication(mtm);
        unsafe {
            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
            app.activate();
        }

        let alert: objc2::rc::Retained<NSAlert> = unsafe { msg_send![NSAlert::alloc(mtm), init] };
        let title = NSString::from_str(&format!("Screenshot Ultra {}", env!("CARGO_PKG_VERSION")));
        let body = NSString::from_str(
            "A snappy, hotkey-first macOS screenshot and screen recorder.\n\
             Local-first. No cloud. No telemetry. No account.\n\n\
             https://github.com/MPJHorner/ScreenshotUltra",
        );
        let ok = NSString::from_str("OK");
        let github = NSString::from_str("Open GitHub");
        unsafe {
            alert.setMessageText(&title);
            alert.setInformativeText(&body);
            alert.addButtonWithTitle(&ok);
            alert.addButtonWithTitle(&github);
            let response = alert.runModal();
            // The second button (GitHub) returns NSAlertSecondButtonReturn = 1001
            if response == 1001 {
                let url_s = NSString::from_str("https://github.com/MPJHorner/ScreenshotUltra");
                if let Some(url) = NSURL::URLWithString(&url_s) {
                    let workspace = objc2_app_kit::NSWorkspace::sharedWorkspace();
                    workspace.openURL(&url);
                }
            }
        }
    }
}
