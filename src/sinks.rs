// Output sinks. M1 shipped clipboard + disk (disk is implicit; the capture
// writes there directly). M2 added a shell-command sink and a shutter sound
// hook.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

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

/// Pull a PNG image out of the macOS clipboard and write it to `out`.
/// Returns `Ok(true)` if an image was extracted, `Ok(false)` if the
/// clipboard didn't contain an image (not an error — just nothing to do),
/// `Err(_)` on I/O or scripting failures.
///
/// Uses `osascript` so we don't need to pull in a Cocoa pasteboard binding
/// for this one-off path.
pub fn clipboard_paste_image_to(out: &Path) -> Result<bool> {
    let out_str = out.to_string_lossy().into_owned();
    let script = format!(
        "try
    set png to the clipboard as «class PNGf»
    set f to (open for access POSIX file \"{out_str}\" with write permission)
    set eof of f to 0
    write png to f
    close access f
    return \"ok\"
on error
    return \"no-image\"
end try"
    );
    let output = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .context("running osascript for clipboard paste")?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match stdout.as_str() {
        "ok" => Ok(true),
        "no-image" => Ok(false),
        other => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("osascript paste returned unexpected output: {other} / {stderr}")
        }
    }
}

/// Run a shell command with the captured file path substituted for `$1`.
/// The command is executed via `/bin/sh -c "<cmd>" -- <path>` so users can
/// write idiomatic shell with quoting, pipes, env vars, etc. We do not
/// wait for completion — the child is detached so a slow uploader can't
/// block the capture pipeline. Returns Ok(()) once the child is spawned.
///
/// Empty / whitespace-only commands are treated as "no shell sink".
pub fn shell_sink(cmd: &str, path: &Path) -> Result<bool> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return Ok(false);
    }
    let path_str = path.to_string_lossy().into_owned();
    Command::new("/bin/sh")
        .arg("-c")
        .arg(cmd)
        .arg("--")
        .arg(&path_str)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("spawning shell sink: {cmd}"))?;
    Ok(true)
}

/// Play the macOS "Grab" shutter sound. Cheap and non-blocking.
#[cfg(target_os = "macos")]
pub fn play_shutter() {
    let _ = Command::new("/usr/bin/afplay")
        .arg("/System/Library/Sounds/Grab.aiff")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

#[cfg(not(target_os = "macos"))]
pub fn play_shutter() {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn empty_shell_command_is_a_no_op() {
        let p = PathBuf::from("/tmp/whatever.png");
        assert!(!shell_sink("", &p).unwrap());
        assert!(!shell_sink("   \t\n  ", &p).unwrap());
    }

    #[test]
    fn shell_sink_runs_simple_command() {
        // Use `true` which always succeeds. spawn() returns immediately;
        // we only care it didn't error.
        let p = PathBuf::from("/tmp/whatever.png");
        assert!(shell_sink("true", &p).unwrap());
    }
}
