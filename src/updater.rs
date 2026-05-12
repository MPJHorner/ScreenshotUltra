// Opt-in update check. By default Screenshot Ultra never phones home
// — plan.md §11. When the user sets `[general].check_for_updates =
// true` we poll the GitHub Releases API at startup (and again every
// 24 hours) for a newer `tag_name`, and pop a notification with the
// link if there is one. The user has to actually download the new
// .zip themselves; we never auto-install (we couldn't anyway without
// a signed updater pipeline, see milestone M6).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// One in-flight check at a time — guarded so menu spam doesn't spawn
/// a stack of curl subprocesses.
static IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// Run an update check on a background thread. Cheap, single-shot,
/// always safe to call. The caller (main.rs) calls this on startup
/// when the setting is on, and again on `Check for Updates…` clicks.
///
/// `force = true` (menu click) → on a hit, post a "newer available"
/// event into the tao loop so the main thread can show the
/// Install-Now NSAlert. `force = false` (background poll) → just
/// notify; we don't want to steal focus from a typing user.
pub fn check_now(force: bool) {
    if IN_FLIGHT.swap(true, Ordering::SeqCst) {
        return;
    }
    let current = env!("CARGO_PKG_VERSION").to_string();
    std::thread::spawn(move || {
        let result = run_check(&current);
        IN_FLIGHT.store(false, Ordering::SeqCst);
        match result {
            Ok(Some(latest)) => {
                if crate::installer::is_skipped(&latest) {
                    crate::logging::event(serde_json::json!({
                        "evt": "update_available_skipped",
                        "current": current,
                        "latest": latest,
                    }));
                    let _ = stamp_last_check();
                    return;
                }
                crate::logging::event(serde_json::json!({
                    "evt": "update_available",
                    "current": current,
                    "latest": latest,
                }));
                crate::sinks::notify(
                    "Screenshot Ultra — update available",
                    &format!(
                        "v{latest} is out (you're on v{current}). Click 'Check for Updates…' in the menu to install.",
                        latest = latest,
                        current = current
                    ),
                );
                let _ = stamp_last_check();
                if force {
                    // The check was user-initiated (menu click) so we
                    // can interrupt them with a modal. Marshal back to
                    // the main thread to show the alert.
                    show_install_dialog_on_main(latest);
                }
            }
            Ok(None) => {
                let _ = stamp_last_check();
                if force {
                    crate::sinks::notify(
                        "Screenshot Ultra is up to date",
                        &format!("You're running v{current}."),
                    );
                }
            }
            Err(err) => {
                crate::logging::event(serde_json::json!({
                    "evt": "update_check_error",
                    "error": err,
                }));
                if force {
                    crate::sinks::notify("Screenshot Ultra — couldn't check for updates", &err);
                }
            }
        }
    });
}

/// Schedule a background thread that calls `check_now` once at
/// startup (after a 30 s delay to not interfere with first-launch
/// permission prompts) and then every 24 hours.
pub fn spawn_background_scheduler() {
    std::thread::spawn(|| {
        // 30 s settling delay — lets the OS finish its first-launch
        // permission prompts and our welcome window land before we
        // make any network calls.
        std::thread::sleep(Duration::from_secs(30));
        loop {
            if !needs_check_now() {
                std::thread::sleep(Duration::from_secs(60 * 60));
                continue;
            }
            check_now(false);
            // Sleep 24 h (split into hourly polls so a `Check for
            // Updates…` click can flip the marker file and the next
            // wake won't re-fire).
            for _ in 0..24 {
                std::thread::sleep(Duration::from_secs(60 * 60));
            }
        }
    });
}

/// Hop to the main thread and present the Install-Now NSAlert.
///
/// `dispatch_get_main_queue()` is a static-inline in the libdispatch
/// headers — not an exported symbol — so we reference the underlying
/// `_dispatch_main_q` global instead. Same value, just one layer down.
#[cfg(target_os = "macos")]
fn show_install_dialog_on_main(latest: String) {
    use block2::RcBlock;
    #[repr(C)]
    struct DispatchQueueS {
        _private: [u8; 0],
    }
    unsafe extern "C" {
        static _dispatch_main_q: DispatchQueueS;
        fn dispatch_async(queue: *mut std::ffi::c_void, block: *mut std::ffi::c_void);
    }
    let block = RcBlock::new(move || {
        let latest = latest.clone();
        match crate::installer::offer_install(&latest) {
            crate::installer::InstallChoice::InstallNow => {
                crate::installer::install_async(latest);
            }
            crate::installer::InstallChoice::Skip => {
                crate::installer::mark_skipped(&latest);
                crate::logging::event(serde_json::json!({
                    "evt": "update_skipped",
                    "version": latest,
                }));
            }
            crate::installer::InstallChoice::Later => {}
        }
    });
    unsafe {
        let queue = &_dispatch_main_q as *const _ as *mut std::ffi::c_void;
        dispatch_async(queue, (&*block) as *const _ as *mut std::ffi::c_void);
    }
    // dispatch_async retains the block, but RcBlock's drop releases
    // its own ref — leak our handle so the block stays alive at least
    // until libdispatch's retain has bumped the count.
    std::mem::forget(block);
}

#[cfg(not(target_os = "macos"))]
fn show_install_dialog_on_main(_latest: String) {}

fn stamp_path() -> Option<PathBuf> {
    let base = dirs::config_dir().or_else(dirs::home_dir)?;
    Some(base.join("ScreenshotUltra").join(".last-update-check"))
}

fn stamp_last_check() -> std::io::Result<()> {
    let Some(path) = stamp_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, b"")?;
    Ok(())
}

/// True if we've never checked or the last check was >24 h ago.
fn needs_check_now() -> bool {
    let Some(path) = stamp_path() else {
        return true;
    };
    let Ok(meta) = std::fs::metadata(&path) else {
        return true;
    };
    let Ok(modified) = meta.modified() else {
        return true;
    };
    modified
        .elapsed()
        .map(|d| d > Duration::from_secs(60 * 60 * 23))
        .unwrap_or(true)
}

/// Make the request and parse out a `tag_name`. Returns Some(version)
/// if a newer release exists, None if we're current. Uses /usr/bin/curl
/// so we don't pull in `reqwest` for a single GET — keeps the binary
/// small and the dep tree narrow.
fn run_check(current: &str) -> Result<Option<String>, String> {
    let url = "https://api.github.com/repos/MPJHorner/ScreenshotUltra/releases/latest";
    let output = std::process::Command::new("/usr/bin/curl")
        .args([
            "-fsSL",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: ScreenshotUltra-update-check",
            "--max-time",
            "10",
            url,
        ])
        .output()
        .map_err(|e| format!("spawning curl: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "curl exited with {} (stderr: {})",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("parsing JSON: {e}"))?;
    let tag = value
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "no tag_name in response".to_string())?;
    // Strip a leading 'v' if present.
    let latest = tag.trim_start_matches('v').to_string();
    if is_newer(&latest, current) {
        Ok(Some(latest))
    } else {
        Ok(None)
    }
}

/// Compare two SemVer-ish strings. Falls back to string comparison if
/// either doesn't parse. Conservative (returns false on any doubt).
fn is_newer(latest: &str, current: &str) -> bool {
    fn parse(s: &str) -> Option<(u64, u64, u64)> {
        let mut parts = s.split('.').map(|p| p.split('-').next().unwrap_or(p));
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        Some((major, minor, patch))
    }
    match (parse(latest), parse(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn detects_minor_bump() {
        assert!(is_newer("0.9.0", "0.8.3"));
    }

    #[test]
    fn detects_patch_bump() {
        assert!(is_newer("0.8.4", "0.8.3"));
    }

    #[test]
    fn detects_major_bump() {
        assert!(is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn equal_is_not_newer() {
        assert!(!is_newer("0.8.3", "0.8.3"));
    }

    #[test]
    fn older_is_not_newer() {
        assert!(!is_newer("0.8.2", "0.8.3"));
    }

    #[test]
    fn unparseable_is_not_newer() {
        // Conservative — refuse to claim newer on parse failure.
        assert!(!is_newer("not-a-version", "0.8.3"));
        assert!(!is_newer("0.8.3", "not-a-version"));
    }

    #[test]
    fn pre_release_suffix_strips_cleanly() {
        // "0.9.0-rc1" should still parse as 0.9.0 and be newer than 0.8.3.
        assert!(is_newer("0.9.0-rc1", "0.8.3"));
    }
}
