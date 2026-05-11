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
use std::sync::Mutex;
use std::time::Instant;

use crate::logging;
use crate::quick_tray;
use crate::settings::Settings;
use crate::sinks;

static SEQ: AtomicU32 = AtomicU32::new(1);

/// Remembers the most recent successful capture so "pin last" and
/// "repeat last" hotkeys have something to act on.
#[derive(Debug, Clone)]
pub struct LastCapture {
    pub mode: CaptureMode,
    pub show_tray: bool,
    pub path: PathBuf,
}

static LAST: Mutex<Option<LastCapture>> = Mutex::new(None);

pub fn last() -> Option<LastCapture> {
    LAST.lock().ok().and_then(|g| g.clone())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    Region,
    Fullscreen,
    Window,
}

impl CaptureMode {
    pub fn as_str(self) -> &'static str {
        match self {
            CaptureMode::Region => "region",
            CaptureMode::Fullscreen => "fullscreen",
            CaptureMode::Window => "window",
        }
    }
}

pub fn run(mode: CaptureMode, show_tray: bool, settings: &Settings) -> Result<()> {
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
        CaptureMode::Window => {
            // Interactive window selection. -o disables the window shadow so
            // we get a tight crop; -W highlights the window under the cursor.
            cmd.arg("-W").arg("-o");
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

    if show_tray {
        quick_tray::show(&path, settings.general.quick_tray_timeout_ms);
    }

    if let Ok(mut guard) = LAST.lock() {
        *guard = Some(LastCapture {
            mode,
            show_tray,
            path: path.clone(),
        });
    }

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

    #[test]
    fn template_includes_date_and_time_tokens() {
        let folder = std::path::PathBuf::from("/tmp/x");
        let path = render_path(&folder, CaptureMode::Fullscreen, "{date}_{time}", "jpg");
        let s = path.to_string_lossy().to_string();
        // Should be /tmp/x/YYYYMMDD_HHMMSS.jpg
        assert!(s.starts_with("/tmp/x/"));
        assert!(s.ends_with(".jpg"));
        let name = s
            .strip_prefix("/tmp/x/")
            .unwrap()
            .strip_suffix(".jpg")
            .unwrap();
        let parts: Vec<&str> = name.split('_').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 8); // YYYYMMDD
        assert_eq!(parts[1].len(), 6); // HHMMSS
    }

    #[test]
    fn template_handles_unknown_tokens_literally() {
        let folder = std::path::PathBuf::from("/tmp");
        let path = render_path(&folder, CaptureMode::Region, "snap_{nope}", "png");
        // unknown tokens stay verbatim
        let s = path.to_string_lossy().to_string();
        assert!(s.contains("{nope}"));
    }

    #[test]
    fn sequence_increments() {
        let folder = std::path::PathBuf::from("/tmp");
        let p1 = render_path(&folder, CaptureMode::Region, "{seq}", "png");
        let p2 = render_path(&folder, CaptureMode::Region, "{seq}", "png");
        assert_ne!(p1, p2);
    }

    #[test]
    fn modes_have_stable_string_names() {
        // These are persisted to NDJSON log; downstream tools depend on them.
        assert_eq!(CaptureMode::Region.as_str(), "region");
        assert_eq!(CaptureMode::Fullscreen.as_str(), "fullscreen");
        assert_eq!(CaptureMode::Window.as_str(), "window");
    }
}
