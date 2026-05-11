// Output sinks. M1 ships clipboard + disk. Disk is implicit (the capture is
// written there directly); clipboard is handled here.

use anyhow::{Context, Result};
use std::path::Path;

/// Copy a PNG/JPG file to the macOS clipboard as an image.
///
/// `arboard` only accepts raw RGBA, so for now we shell out to `osascript`,
/// which is reliable, dependency-free, and handles every format
/// `screencapture` can produce. We'll swap to a native path with the
/// ScreenCaptureKit backend.
pub fn clipboard_copy_image(path: &Path) -> Result<()> {
    let abs = path
        .canonicalize()
        .with_context(|| format!("canonicalising {}", path.display()))?;
    let script = format!(
        "set the clipboard to (read (POSIX file \"{}\") as JPEG picture)",
        abs.display()
    );
    let output = std::process::Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .context("running osascript for clipboard copy")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("osascript clipboard copy failed: {stderr}");
    }
    Ok(())
}
