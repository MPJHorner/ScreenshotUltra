// In-place auto-installer. When the user clicks "Install Now" on the
// update dialog we:
//
//   1. Download the new release .zip + its .sha256 sidecar from GitHub.
//   2. Verify the checksum with /usr/bin/shasum so a man-in-the-middle
//      release-asset swap would have to fool the GitHub TLS cert *and*
//      land a matching .sha256 in the same release.
//   3. Unpack via /usr/bin/ditto -xk into a staging directory.
//   4. Locate the extracted `Screenshot Ultra.app`.
//   5. Write a tiny installer shell script to /tmp/, spawn it detached,
//      and quit ourselves. The script waits for our process to exit,
//      atomically swaps the bundle in /Applications/, clears the
//      Gatekeeper quarantine flag, and relaunches the new version.
//
// We never auto-replace without an explicit user click — Apple's app
// security model assumes the user trusts upgrades, and without a
// signed updater chain we'd be one compromised GitHub token away from
// shipping malware. The opt-in dialog keeps the user in the loop.
//
// Once M6 (signing + notarisation) lands, this code stays — we just
// add a signature-verification step before the swap.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const REPO_RELEASE_URL: &str = "https://github.com/MPJHorner/ScreenshotUltra/releases";

/// Guard so a double-click on "Install Now" doesn't run two downloads.
static INSTALLING: AtomicBool = AtomicBool::new(false);

/// Kick off the install flow for `latest_version`. Runs on a background
/// thread so the UI stays responsive while we curl ~3 MB of universal
/// zip + verify the checksum.
pub fn install_async(latest_version: String) {
    if INSTALLING.swap(true, Ordering::SeqCst) {
        crate::sinks::notify(
            "Screenshot Ultra",
            "An update is already being prepared.",
        );
        return;
    }

    std::thread::spawn(move || {
        let result = install_blocking(&latest_version);
        // We don't release INSTALLING on success because we're about
        // to quit anyway; on failure we release so the user can retry.
        match result {
            Ok(()) => {
                // We get here only if the helper spawn returns. The
                // helper is detached and waits for our process to exit
                // before swapping the bundle — see `relaunch_helper`.
                crate::logging::event(serde_json::json!({
                    "evt": "update_installing",
                    "version": latest_version,
                }));
                // Give the helper time to take a snapshot of our PID
                // before we terminate.
                std::thread::sleep(Duration::from_millis(300));
                quit_app();
            }
            Err(err) => {
                INSTALLING.store(false, Ordering::SeqCst);
                eprintln!("installer: {err}");
                crate::logging::event(serde_json::json!({
                    "evt": "update_install_failed",
                    "version": latest_version,
                    "error": err,
                }));
                crate::sinks::notify(
                    "Screenshot Ultra — couldn't install update",
                    &err,
                );
            }
        }
    });
}

fn install_blocking(latest: &str) -> Result<(), String> {
    let install_path = locate_installed_app()
        .ok_or_else(|| {
            "couldn't find /Applications/Screenshot Ultra.app — the in-place \
             installer is only wired up when the app lives in /Applications. \
             Download the new version manually from the Releases page."
                .to_string()
        })?;
    if !is_writable(&install_path) {
        return Err(format!(
            "/Applications/Screenshot Ultra.app isn't writable by this user. \
             Update manually from {REPO_RELEASE_URL}/tag/v{latest}"
        ));
    }

    // 1. Pick a temp staging dir.
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let staging = std::env::temp_dir().join(format!("screenshot-ultra-update-{stamp}"));
    std::fs::create_dir_all(&staging)
        .map_err(|e| format!("creating staging dir {}: {e}", staging.display()))?;

    let zip_name = format!("ScreenshotUltra-v{latest}-universal.zip");
    let zip_path = staging.join(&zip_name);
    let sha_path = staging.join(format!("{zip_name}.sha256"));

    let zip_url = format!("{REPO_RELEASE_URL}/download/v{latest}/{zip_name}");
    let sha_url = format!("{zip_url}.sha256");

    // 2. Download both. curl exits non-zero on HTTP errors thanks to -f.
    notify_progress("Downloading update…", latest);
    curl_to(&zip_url, &zip_path)?;
    curl_to(&sha_url, &sha_path)?;

    // 3. Verify the checksum. The .sha256 file format from
    //    `shasum -a 256 file.zip > file.zip.sha256` is
    //    "<hex>  <filename>\n" — `shasum -c` validates that line.
    verify_sha256(&staging, &sha_path)?;

    // 4. Unpack. ditto -xk preserves resource forks + xattrs faithfully.
    notify_progress("Verifying & extracting…", latest);
    let extract_dir = staging.join("unpacked");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("creating {}: {e}", extract_dir.display()))?;
    let status = Command::new("/usr/bin/ditto")
        .arg("-xk")
        .arg(&zip_path)
        .arg(&extract_dir)
        .status()
        .map_err(|e| format!("running ditto: {e}"))?;
    if !status.success() {
        return Err(format!("ditto exited with {status}"));
    }

    // 5. Locate the extracted .app — its name should match exactly.
    let staged_app = extract_dir.join("Screenshot Ultra.app");
    if !staged_app.is_dir() {
        return Err(format!(
            "extracted zip didn't contain Screenshot Ultra.app at {}",
            staged_app.display()
        ));
    }

    // 6. Spawn the detached helper.
    spawn_swap_helper(&staged_app, &install_path)?;
    crate::sinks::notify(
        "Screenshot Ultra — installing update",
        &format!("Relaunching as v{latest}…"),
    );
    Ok(())
}

fn locate_installed_app() -> Option<PathBuf> {
    // We only auto-install when the app lives in /Applications. Anywhere
    // else (cargo run, ~/Downloads, etc.) gets the manual-update message.
    let p = PathBuf::from("/Applications/Screenshot Ultra.app");
    if p.is_dir() {
        Some(p)
    } else {
        None
    }
}

fn is_writable(path: &Path) -> bool {
    // Probe by trying to create + remove a sentinel file alongside the
    // app bundle. Less brittle than checking permissions bits across
    // ACLs / SIP / network-mounted volumes.
    let Some(parent) = path.parent() else {
        return false;
    };
    let probe = parent.join(format!(".screenshot-ultra-write-probe-{}", std::process::id()));
    match std::fs::write(&probe, b"") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn curl_to(url: &str, dest: &Path) -> Result<(), String> {
    let status = Command::new("/usr/bin/curl")
        .args([
            "-fsSL",
            "--max-time",
            "120",
            "--retry",
            "2",
            "-A",
            "ScreenshotUltra-installer",
            url,
            "-o",
        ])
        .arg(dest)
        .status()
        .map_err(|e| format!("running curl: {e}"))?;
    if !status.success() {
        return Err(format!(
            "couldn't download {url} (curl exited with {status})"
        ));
    }
    Ok(())
}

fn verify_sha256(workdir: &Path, sha_path: &Path) -> Result<(), String> {
    // -c reads "<hex>  <filename>" from the sha256 file and validates
    // the matching file in the current directory.
    let output = Command::new("/usr/bin/shasum")
        .arg("-a")
        .arg("256")
        .arg("-c")
        .arg(sha_path)
        .current_dir(workdir)
        .output()
        .map_err(|e| format!("running shasum: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "SHA-256 verification failed for the downloaded zip.\n\n\
             shasum stdout:\n{stdout}\n\nshasum stderr:\n{stderr}\n\n\
             Refusing to install."
        ));
    }
    Ok(())
}

fn notify_progress(stage: &str, latest: &str) {
    crate::sinks::notify(
        "Screenshot Ultra — update",
        &format!("{stage} (v{latest})"),
    );
}

/// Embedded helper script. Written to /tmp/, marked executable, then
/// spawned with stdio→/dev/null. Polls for our process to exit, swaps
/// the bundle, clears quarantine, relaunches.
const SWAP_HELPER: &str = r##"#!/bin/bash
# Screenshot Ultra in-place updater. Spawned by the running app right
# before it terminates; this script does the actual file moves and
# relaunches the new version.
set -euo pipefail
STAGED="$1"          # /tmp/screenshot-ultra-update-…/unpacked/Screenshot Ultra.app
CURRENT="$2"         # /Applications/Screenshot Ultra.app
PID="$3"             # the running app's pid, to wait on

# Wait up to 15s for the running app to exit cleanly.
for _ in $(seq 1 30); do
    if ! kill -0 "$PID" 2>/dev/null; then break; fi
    sleep 0.5
done
# Belt-and-braces: if it's still running, give it a polite signal.
if kill -0 "$PID" 2>/dev/null; then
    kill -TERM "$PID" 2>/dev/null || true
    sleep 1
fi

BACKUP_DIR="$(dirname "$CURRENT")"
BACKUP="$BACKUP_DIR/.ScreenshotUltra.backup-$$"

# Try to back up the current bundle. If the backup move fails (perms)
# we abort *before* destroying anything.
if [ -d "$CURRENT" ]; then
    if ! mv "$CURRENT" "$BACKUP" 2>/tmp/screenshot-ultra-installer.err; then
        echo "installer: cannot back up $CURRENT" >&2
        open -a "$CURRENT" 2>/dev/null || true
        exit 1
    fi
fi

# Move the staged app into place. If anything fails, restore the
# backup so the user isn't left without an app.
if ! mv "$STAGED" "$CURRENT" 2>>/tmp/screenshot-ultra-installer.err; then
    echo "installer: failed to move new app into place; restoring backup" >&2
    [ -d "$BACKUP" ] && mv "$BACKUP" "$CURRENT"
    open -a "$CURRENT" 2>/dev/null || true
    exit 1
fi

# Clear Gatekeeper's quarantine xattr so the user doesn't see a
# "downloaded from internet" prompt on first launch of the new version.
xattr -dr com.apple.quarantine "$CURRENT" 2>/dev/null || true

# Wait a beat for macOS LaunchServices to notice the new bundle.
sleep 1

# Relaunch the new app.
open "$CURRENT"

# Clean up the backup once the relaunch is in flight.
sleep 5
rm -rf "$BACKUP" 2>/dev/null || true
"##;

fn spawn_swap_helper(staged_app: &Path, install_path: &Path) -> Result<(), String> {
    let helper = std::env::temp_dir().join("screenshot-ultra-installer.sh");
    std::fs::write(&helper, SWAP_HELPER)
        .map_err(|e| format!("writing helper to {}: {e}", helper.display()))?;
    let mut perms = std::fs::metadata(&helper)
        .map_err(|e| format!("statting helper: {e}"))?
        .permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    std::fs::set_permissions(&helper, perms)
        .map_err(|e| format!("chmodding helper: {e}"))?;

    let pid = std::process::id().to_string();
    Command::new("/bin/bash")
        .arg(&helper)
        .arg(staged_app)
        .arg(install_path)
        .arg(&pid)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("spawning helper: {e}"))?;
    Ok(())
}

fn quit_app() {
    // Quit on the main thread via NSApplication.terminate. We can't
    // easily marshal back from a worker thread, so just exit() — by
    // this point the helper script has our PID and is patiently
    // polling for us to die.
    std::process::exit(0);
}

/// Show a synchronous NSAlert offering Install Now / Later / Skip.
/// Must run on the main thread. Returns the user's choice.
#[cfg(target_os = "macos")]
pub fn offer_install(latest: &str) -> InstallChoice {
    use objc2::rc::Retained;
    use objc2::{msg_send, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSAlert, NSApplication, NSApplicationActivationPolicy};
    use objc2_foundation::NSString;

    let Some(mtm) = MainThreadMarker::new() else {
        // Off main thread — can't show a modal. Caller should retry.
        return InstallChoice::Later;
    };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.activate();
    let alert: Retained<NSAlert> = unsafe { msg_send![NSAlert::alloc(mtm), init] };
    alert.setMessageText(&NSString::from_str(&format!(
        "Screenshot Ultra v{latest} is available"
    )));
    alert.setInformativeText(&NSString::from_str(
        "Install now and relaunch? The download is ~3 MB and \
         takes a few seconds.\n\nWe'll verify the SHA-256 against the \
         published checksum before installing.",
    ));
    alert.addButtonWithTitle(&NSString::from_str("Install Now"));
    alert.addButtonWithTitle(&NSString::from_str("Later"));
    alert.addButtonWithTitle(&NSString::from_str("Skip This Version"));
    let resp = alert.runModal();
    match resp {
        1000 => InstallChoice::InstallNow,
        1001 => InstallChoice::Later,
        1002 => InstallChoice::Skip,
        _ => InstallChoice::Later,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn offer_install(_latest: &str) -> InstallChoice {
    InstallChoice::Later
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallChoice {
    InstallNow,
    Later,
    Skip,
}

/// The path of the "skip this version" marker — checked on update
/// discovery so we don't re-prompt for the same release.
fn skip_marker_path(version: &str) -> Option<PathBuf> {
    let base = dirs::config_dir().or_else(dirs::home_dir)?;
    Some(
        base.join("ScreenshotUltra")
            .join(format!(".skipped-update-{version}")),
    )
}

pub fn is_skipped(version: &str) -> bool {
    skip_marker_path(version)
        .map(|p| p.exists())
        .unwrap_or(false)
}

pub fn mark_skipped(version: &str) {
    if let Some(path) = skip_marker_path(version) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, b"");
    }
}
