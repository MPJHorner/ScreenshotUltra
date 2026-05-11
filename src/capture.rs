// Capture pipeline (M1 backend = macOS `screencapture` CLI).
//
// This is intentionally a thin shell-out so we can ship M1 end-to-end today.
// A native ScreenCaptureKit backend lands in a later milestone behind the same
// `run()` entry point.

use anyhow::{Context, Result};
use chrono::Local;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use crate::logging;
use crate::settings::Settings;
use crate::sinks;

static SEQ: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone, Copy)]
pub enum CaptureMode {
    Region,
    Fullscreen,
}

impl CaptureMode {
    pub fn as_str(self) -> &'static str {
        match self {
            CaptureMode::Region => "region",
            CaptureMode::Fullscreen => "fullscreen",
        }
    }
}

pub fn run(mode: CaptureMode, settings: &Settings) -> Result<()> {
    let start = Instant::now();
    let folder = settings.general.save_folder_expanded();
    std::fs::create_dir_all(&folder)
        .with_context(|| format!("creating save folder {}", folder.display()))?;

    let path = render_path(
        &folder,
        mode,
        &settings.general.filename_template,
        &settings.general.default_image_format,
    );

    // `screencapture` flags:
    //   -x  no sound (we play our own shutter if enabled)
    //   -i  interactive (region selector with crosshair); supports window via space
    //   -t  format (png|jpg|tiff|pdf|bmp)
    let mut cmd = Command::new("/usr/sbin/screencapture");
    cmd.arg("-x")
        .arg("-t")
        .arg(&settings.general.default_image_format);
    match mode {
        CaptureMode::Region => {
            cmd.arg("-i"); // interactive region (esc cancels → no file written)
        }
        CaptureMode::Fullscreen => {
            // default = all displays, one file per display. -m would limit to main display.
            // For M1 we capture main display only for a single-file result.
            cmd.arg("-m");
        }
    }
    cmd.arg(&path);

    let status = cmd
        .status()
        .with_context(|| "running /usr/sbin/screencapture")?;

    if !status.success() {
        anyhow::bail!("screencapture exited with status {status}");
    }

    // Region capture: if the user pressed Esc, no file is written.
    if !path.exists() {
        logging::event(serde_json::json!({
            "evt": "capture_cancelled",
            "mode": mode.as_str(),
        }));
        return Ok(());
    }

    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let mut sinks_fired = vec!["disk"];

    if settings.sinks.clipboard && settings.general.copy_on_capture {
        match sinks::clipboard_copy_image(&path) {
            Ok(()) => sinks_fired.push("clipboard"),
            Err(err) => eprintln!("clipboard sink failed: {err:#}"),
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    logging::event(serde_json::json!({
        "evt": "capture",
        "mode": mode.as_str(),
        "bytes": bytes,
        "fmt": settings.general.default_image_format,
        "saved_to": path.display().to_string(),
        "sinks": sinks_fired,
        "duration_ms": duration_ms,
    }));

    eprintln!(
        "captured {} ({} bytes, {} ms)",
        path.display(),
        bytes,
        duration_ms
    );

    Ok(())
}

fn render_path(folder: &std::path::Path, mode: CaptureMode, template: &str, fmt: &str) -> PathBuf {
    let now = Local::now();
    let date = now.format("%Y%m%d").to_string();
    let time = now.format("%H%M%S").to_string();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let name = template
        .replace("{date}", &date)
        .replace("{time}", &time)
        .replace("{mode}", mode.as_str())
        .replace("{seq}", &format!("{seq:03}"));
    folder.join(format!("{name}.{fmt}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_renders_known_tokens() {
        let folder = std::path::PathBuf::from("/tmp/x");
        let path = render_path(&folder, CaptureMode::Region, "{mode}_{seq}", "png");
        // mode is rendered; seq is zero-padded to three digits
        let s = path.to_string_lossy().to_string();
        assert!(s.starts_with("/tmp/x/region_"));
        assert!(s.ends_with(".png"));
    }

    #[test]
    fn expand_tilde_resolves_home() {
        let p = crate::settings::expand_tilde("~/foo");
        assert!(p.is_absolute());
        assert!(p.ends_with("foo"));
    }
}
