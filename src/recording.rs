// Screen recording — video (.mov) and GIF.
//
// M3 v0 ships on top of macOS's own `screencapture -v` for the actual
// frame grabbing. SIGINT cleanly finalises the .mov, so start/stop
// recording is just: spawn child → store its PID → send SIGINT to stop.
//
// GIF is post-processed from the .mov via `ffmpeg` if available; if
// not, we leave the .mov on disk and the Quick Tray opens it. A native
// ScreenCaptureKit + AVAssetWriter + `gifski` pipeline lands in a
// follow-up — same `start_video()` / `stop_recording()` boundary so
// the swap is contained.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Instant;

use crate::capture::{render_path_for_recording, write_history_index};
use crate::keystroke_overlay;
use crate::logging;
use crate::quick_tray;
use crate::settings::Settings;
use crate::sinks;
use crate::tray;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingKind {
    Video,
    Gif,
}

impl RecordingKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RecordingKind::Video => "video",
            RecordingKind::Gif => "gif",
        }
    }
}

struct ActiveRecording {
    child: Child,
    kind: RecordingKind,
    path: PathBuf,
    started: Instant,
}

static ACTIVE: Mutex<Option<ActiveRecording>> = Mutex::new(None);

/// `true` while a recording is in progress. Used for tray menu wording.
pub fn is_recording() -> bool {
    ACTIVE.lock().ok().map(|g| g.is_some()).unwrap_or(false)
}

/// Begin an open-ended recording. `kind` controls only the eventual
/// output extension — the screencapture pipeline always records .mov,
/// and GIF is produced by post-processing on stop.
pub fn start(kind: RecordingKind, settings: &Settings) -> Result<()> {
    if is_recording() {
        anyhow::bail!("a recording is already in progress; stop it first");
    }

    let folder = settings.general.save_folder_expanded();
    std::fs::create_dir_all(&folder)
        .with_context(|| format!("creating save folder {}", folder.display()))?;
    // The frame grabber always writes a .mov; GIF post-process renames.
    let path = render_path_for_recording(&folder, kind, &settings.general.filename_template);

    let mut cmd = Command::new("/usr/sbin/screencapture");
    cmd.arg("-v"); // streaming video, SIGINT to finalise
    if settings.recording.show_clicks {
        cmd.arg("-k");
    }
    if settings.recording.record_microphone {
        cmd.arg("-g");
    }
    cmd.arg(&path);

    let child = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| "spawning screencapture -v")?;

    logging::event(serde_json::json!({
        "evt": "recording_start",
        "kind": kind.as_str(),
        "path": path.display().to_string(),
        "show_clicks": settings.recording.show_clicks,
        "microphone": settings.recording.record_microphone,
    }));
    eprintln!("recording: started → {}", path.display());

    *ACTIVE.lock().unwrap() = Some(ActiveRecording {
        child,
        kind,
        path,
        started: Instant::now(),
    });
    tray::set_recording_indicator(true);
    if settings.recording.keystroke_overlay {
        keystroke_overlay::start();
    }
    Ok(())
}

/// Stop the current recording. SIGINT finalises the .mov; we wait
/// briefly, then post-process for GIF if needed and route through
/// the Quick Tray.
pub fn stop(settings: &Settings) -> Result<()> {
    let Some(mut rec) = ACTIVE.lock().unwrap().take() else {
        anyhow::bail!("no recording is in progress");
    };
    let pid = rec.child.id() as i32;
    // SIGINT is what screencapture -v expects to finalise the file.
    unsafe {
        libc_kill(pid, 2 /* SIGINT */);
    }
    // Wait up to 5 s for the process to finalise the moov atom.
    let _ = rec.child.wait();
    let duration_s = rec.started.elapsed().as_secs_f64();

    if !rec.path.exists() {
        anyhow::bail!("recording stopped but no file at {}", rec.path.display());
    }
    let mut bytes = std::fs::metadata(&rec.path).map(|m| m.len()).unwrap_or(0);

    // For GIF, transcode via ffmpeg if available. If not, we keep the
    // .mov and rename the extension to make that obvious — better than
    // crashing.
    let final_path = match rec.kind {
        RecordingKind::Video => rec.path.clone(),
        RecordingKind::Gif => {
            let gif_path = rec.path.with_extension("gif");
            match transcode_to_gif(&rec.path, &gif_path) {
                Ok(()) => {
                    let _ = std::fs::remove_file(&rec.path);
                    bytes = std::fs::metadata(&gif_path)
                        .map(|m| m.len())
                        .unwrap_or(bytes);
                    gif_path
                }
                Err(err) => {
                    eprintln!("recording: GIF transcode failed ({err:#}); keeping .mov");
                    rec.path.clone()
                }
            }
        }
    };

    let mut sinks_fired = vec!["disk"];
    let shell_cmd = settings.sinks.shell_for(rec.kind.as_str());
    if !shell_cmd.trim().is_empty() && matches!(sinks::shell_sink(shell_cmd, &final_path), Ok(true))
    {
        sinks_fired.push("shell");
    }

    let event = serde_json::json!({
        "evt": "recording_stop",
        "kind": rec.kind.as_str(),
        "duration_s": duration_s,
        "bytes": bytes,
        "saved_to": final_path.display().to_string(),
        "sinks": sinks_fired,
    });
    logging::event(event.clone());
    write_history_index(&settings.general.save_folder_expanded(), &event);

    eprintln!(
        "recording: {} stopped ({} bytes, {:.1}s)",
        rec.kind.as_str(),
        bytes,
        duration_s
    );

    quick_tray::show(&final_path, settings.general.quick_tray_timeout_ms);
    tray::set_recording_indicator(false);
    keystroke_overlay::stop();

    // Recordings don't render as Quick-Tray thumbnails (NSImage can't
    // load .mov frames without AVFoundation), so reinforce with a
    // notification banner: at minimum the user knows where the file is.
    let name = final_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("recording");
    let pretty_size = pretty_bytes(bytes);
    sinks::notify(
        "Screenshot Ultra — recording saved",
        &format!("{name}  ·  {pretty_size}  ·  {:.1}s", duration_s),
    );

    Ok(())
}

fn pretty_bytes(b: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    if b >= MB {
        format!("{:.1} MB", b as f64 / MB as f64)
    } else if b >= KB {
        format!("{:.0} KB", b as f64 / KB as f64)
    } else {
        format!("{} B", b)
    }
}

/// Stop if recording, start otherwise. Used by the global toggle hotkey.
pub fn toggle(kind: RecordingKind, settings: &Settings) -> Result<()> {
    if is_recording() {
        stop(settings)
    } else {
        start(kind, settings)
    }
}

/// Transcode a `.mov` to a `.gif`. Requires `ffmpeg` on PATH; we don't
/// bundle it because it's massive — install with `brew install ffmpeg`.
fn transcode_to_gif(input: &std::path::Path, output: &std::path::Path) -> Result<()> {
    let ffmpeg = which::which_in_global("ffmpeg", std::env::var_os("PATH"))
        .ok()
        .and_then(|mut it| it.next())
        .ok_or_else(|| anyhow::anyhow!("ffmpeg not on PATH (brew install ffmpeg)"))?;
    // 12 fps + a generated palette = small GIFs, decent quality.
    let palette = output.with_extension("palette.png");
    let status = Command::new(&ffmpeg)
        .args([
            "-y",
            "-i",
            &input.to_string_lossy(),
            "-vf",
            "fps=12,scale=iw:ih:flags=lanczos,palettegen=stats_mode=diff",
        ])
        .arg(&palette)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("running ffmpeg for palette")?;
    if !status.success() {
        anyhow::bail!("ffmpeg palette pass exited with {status}");
    }
    let status = Command::new(&ffmpeg)
        .args([
            "-y",
            "-i",
            &input.to_string_lossy(),
            "-i",
            &palette.to_string_lossy(),
            "-lavfi",
            "fps=12,scale=iw:ih:flags=lanczos[x];[x][1:v]paletteuse=dither=sierra2_4a",
        ])
        .arg(output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("running ffmpeg for paletteuse")?;
    let _ = std::fs::remove_file(&palette);
    if !status.success() {
        anyhow::bail!("ffmpeg paletteuse pass exited with {status}");
    }
    Ok(())
}

// libc::kill wrapper without pulling the libc crate in for one call.
unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}
unsafe fn libc_kill(pid: i32, sig: i32) -> i32 {
    unsafe { kill(pid, sig) }
}

// Minimal in-house `which` so we don't pull another crate just for ffmpeg
// lookup. Walks `PATH` and returns the first executable match.
#[allow(dead_code)]
mod which {
    use std::ffi::OsString;
    use std::path::PathBuf;

    pub fn which_in_global(
        name: &str,
        path: Option<OsString>,
    ) -> anyhow::Result<std::vec::IntoIter<PathBuf>> {
        let Some(path) = path else {
            return Ok(vec![].into_iter());
        };
        let mut hits = Vec::new();
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(name);
            if let Ok(meta) = candidate.metadata() {
                if meta.is_file() {
                    use std::os::unix::fs::PermissionsExt;
                    if meta.permissions().mode() & 0o111 != 0 {
                        hits.push(candidate);
                    }
                }
            }
        }
        Ok(hits.into_iter())
    }
}
