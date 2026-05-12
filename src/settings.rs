// Settings: load `~/Library/Application Support/ScreenshotUltra/settings.toml`,
// fall back to defaults, and write defaults on first run.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default)]
    pub general: General,
    #[serde(default)]
    pub capture: CaptureOptions,
    #[serde(default)]
    pub recording: RecordingOptions,
    #[serde(default)]
    pub hotkeys: Hotkeys,
    #[serde(default)]
    pub sinks: Sinks,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordingOptions {
    /// Highlight mouse clicks in the video output (passes `-k` to
    /// screencapture).
    pub show_clicks: bool,
    /// Capture audio from the default input device (passes `-g` to
    /// screencapture). System audio capture requires a separate
    /// virtual audio device on macOS; not exposed yet.
    pub record_microphone: bool,
    /// Show a floating overlay of recently-typed keys while recording.
    /// Requires Accessibility permission (macOS prompts on first use).
    #[serde(default)]
    pub keystroke_overlay: bool,
}

impl Default for RecordingOptions {
    fn default() -> Self {
        Self {
            show_clicks: true,
            record_microphone: false,
            keystroke_overlay: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaptureOptions {
    /// Include the mouse cursor in the capture (passes `-C` to screencapture).
    #[serde(default)]
    pub include_cursor: bool,
    /// Fullscreen scope:
    ///   "main"    — main display only (single file). Default.
    ///   "all"     — every connected display, one file per display.
    pub fullscreen_scope: String,
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            include_cursor: false,
            fullscreen_scope: "main".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct General {
    pub save_folder: String,
    pub filename_template: String,
    pub default_image_format: String,
    pub copy_on_capture: bool,
    pub play_shutter_sound: bool,
    pub show_in_dock: bool,
    #[serde(default = "default_tray_timeout_ms")]
    pub quick_tray_timeout_ms: u64,
}

fn default_tray_timeout_ms() -> u64 {
    6000
}

// Per-field default functions. We use these instead of relying on
// `impl Default for Hotkeys` because serde's `#[serde(default)]` falls
// back to `Default::default()` on the FIELD type (which is `String` →
// `""`), not the parent struct. That means anyone upgrading from an
// older config would silently lose new bindings unless we name a
// concrete default per slot.
fn default_region() -> String {
    "ctrl+alt+cmd+1".into()
}
fn default_window() -> String {
    "ctrl+alt+cmd+2".into()
}
fn default_fullscreen() -> String {
    "ctrl+alt+cmd+3".into()
}
fn default_pin_last() -> String {
    "ctrl+alt+cmd+period".into()
}
fn default_repeat_last() -> String {
    "ctrl+alt+cmd+r".into()
}
fn default_open_clipboard_image() -> String {
    "ctrl+alt+cmd+e".into()
}
fn default_color_picker() -> String {
    "ctrl+alt+cmd+p".into()
}
fn default_preferences() -> String {
    "ctrl+alt+cmd+comma".into()
}
fn default_record_video() -> String {
    "ctrl+alt+cmd+v".into()
}
fn default_record_gif() -> String {
    "ctrl+alt+cmd+g".into()
}
fn default_help() -> String {
    "ctrl+alt+cmd+slash".into()
}
fn default_history() -> String {
    "ctrl+alt+cmd+h".into()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hotkeys {
    /// Region capture with Quick Tray (the standard flow). Empty = unbound.
    #[serde(default = "default_region")]
    pub region: String,
    /// Fullscreen capture with Quick Tray (the standard flow). Empty = unbound.
    #[serde(default = "default_fullscreen")]
    pub fullscreen: String,
    /// Window capture with Quick Tray. Empty = unbound.
    #[serde(default = "default_window")]
    pub window: String,
    /// Region capture, silent: save to disk + clipboard, no UI. Empty = unbound.
    #[serde(default)]
    pub silent_region: String,
    /// Fullscreen capture, silent. Empty = unbound.
    #[serde(default)]
    pub silent_fullscreen: String,
    /// Window capture, silent. Empty = unbound.
    #[serde(default)]
    pub silent_window: String,
    /// Pin the most recent capture to the screen. Empty = unbound.
    #[serde(default = "default_pin_last")]
    pub pin_last: String,
    /// Repeat the previous capture mode. Empty = unbound.
    #[serde(default = "default_repeat_last")]
    pub repeat_last: String,
    /// Paste the clipboard's image and run it through the Quick Tray flow.
    /// Empty = unbound.
    #[serde(default = "default_open_clipboard_image")]
    pub open_clipboard_image: String,
    /// Show macOS's system colour-sampler magnifier; copies the picked
    /// hex onto the clipboard. Empty = unbound.
    #[serde(default = "default_color_picker")]
    pub color_picker: String,
    /// Open the Preferences window. Empty = unbound.
    #[serde(default = "default_preferences")]
    pub preferences: String,
    /// Toggle video recording on/off. Empty = unbound.
    #[serde(default = "default_record_video")]
    pub record_video: String,
    /// Toggle GIF recording on/off. Empty = unbound.
    #[serde(default = "default_record_gif")]
    pub record_gif: String,
    /// Show the keyboard-shortcut cheat sheet window. Empty = unbound.
    #[serde(default = "default_help")]
    pub help: String,
    /// Open the per-folder history window. Empty = unbound.
    #[serde(default = "default_history")]
    pub history: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sinks {
    pub clipboard: bool,
    pub disk: bool,
    /// Optional shell command run after a successful capture. The captured
    /// file path is passed as `$1`. Empty = disabled. Examples:
    ///
    ///   shell = "scp $1 user@host:/var/www/img/"
    ///   shell = "rclone copy $1 remote:bucket/"
    ///   shell = "/usr/local/bin/slack-upload $1"
    ///
    /// Runs detached so it doesn't block the capture pipeline.
    #[serde(default)]
    pub shell: String,
}

impl Default for General {
    fn default() -> Self {
        Self {
            save_folder: "~/Pictures/ScreenshotUltra".into(),
            filename_template: "{date}_{time}_{mode}_{seq}".into(),
            default_image_format: "png".into(),
            copy_on_capture: true,
            play_shutter_sound: true,
            show_in_dock: false,
            quick_tray_timeout_ms: 6000,
        }
    }
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            region: "ctrl+alt+cmd+1".into(),
            fullscreen: "ctrl+alt+cmd+3".into(),
            window: "ctrl+alt+cmd+2".into(),
            silent_region: String::new(),
            silent_fullscreen: String::new(),
            silent_window: String::new(),
            pin_last: "ctrl+alt+cmd+period".into(),
            repeat_last: "ctrl+alt+cmd+r".into(),
            open_clipboard_image: "ctrl+alt+cmd+e".into(),
            color_picker: "ctrl+alt+cmd+p".into(),
            preferences: "ctrl+alt+cmd+comma".into(),
            record_video: "ctrl+alt+cmd+v".into(),
            record_gif: "ctrl+alt+cmd+g".into(),
            help: "ctrl+alt+cmd+slash".into(),
            history: "ctrl+alt+cmd+h".into(),
        }
    }
}

impl Default for Sinks {
    fn default() -> Self {
        Self {
            clipboard: true,
            disk: true,
            shell: String::new(),
        }
    }
}

impl Settings {
    pub fn load_or_default() -> Result<Self> {
        let path = Self::path()?;
        if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let parsed: Settings =
                toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
            // If the on-disk file is missing fields (e.g. user upgraded from
            // an older version), rewrite it so all current options show up
            // next time they edit. We only rewrite when the round-tripped
            // TOML differs from the raw — preserves comments and ordering
            // when the file is already up to date.
            if let Ok(round_trip) = toml::to_string_pretty(&parsed) {
                if round_trip.trim() != raw.trim() {
                    let _ = std::fs::write(&path, round_trip);
                }
            }
            Ok(parsed)
        } else {
            let s = Settings::default();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let raw = toml::to_string_pretty(&s).context("serialising default settings")?;
            std::fs::write(&path, raw).ok();
            Ok(s)
        }
    }

    pub fn path() -> Result<PathBuf> {
        let base = dirs::config_dir()
            .or_else(dirs::home_dir)
            .context("no config dir")?;
        Ok(base.join("ScreenshotUltra").join("settings.toml"))
    }
}

impl General {
    pub fn save_folder_expanded(&self) -> PathBuf {
        expand_tilde(&self.save_folder)
    }
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_serialise_and_round_trip() {
        let s = Settings::default();
        let raw = toml::to_string_pretty(&s).unwrap();
        let back: Settings = toml::from_str(&raw).unwrap();
        assert_eq!(s.general.save_folder, back.general.save_folder);
        assert_eq!(s.hotkeys.region, back.hotkeys.region);
        assert_eq!(s.hotkeys.pin_last, back.hotkeys.pin_last);
        assert_eq!(s.sinks.clipboard, back.sinks.clipboard);
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing_sections() {
        // Old config without the new hotkey slots — should still parse.
        let raw = r#"
            [general]
            save_folder = "~/foo"
            filename_template = "x"
            default_image_format = "png"
            copy_on_capture = false
            play_shutter_sound = false
            show_in_dock = true

            [hotkeys]
            region = "ctrl+1"
            fullscreen = "ctrl+3"
            window = ""
            silent_region = ""
            silent_fullscreen = ""
            silent_window = ""
            pin_last = ""
            repeat_last = ""

            [sinks]
            clipboard = true
            disk = true
        "#;
        let s: Settings = toml::from_str(raw).unwrap();
        assert_eq!(s.general.save_folder, "~/foo");
        assert!(!s.general.copy_on_capture);
        // quick_tray_timeout_ms wasn't in the toml — should default to 6000.
        assert_eq!(s.general.quick_tray_timeout_ms, 6000);
    }

    #[test]
    fn empty_silent_slots_are_acceptable() {
        let mut s = Settings::default();
        s.hotkeys.silent_region = String::new();
        s.hotkeys.silent_fullscreen = String::new();
        let raw = toml::to_string(&s).unwrap();
        let back: Settings = toml::from_str(&raw).unwrap();
        assert!(back.hotkeys.silent_region.is_empty());
        assert!(back.hotkeys.silent_fullscreen.is_empty());
    }

    #[test]
    fn tilde_expansion() {
        let p = expand_tilde("~/Pictures/foo");
        assert!(p.is_absolute());
        assert!(p.ends_with("Pictures/foo"));
        assert!(!p.to_string_lossy().contains('~'));
    }

    #[test]
    fn tilde_alone_resolves_to_home() {
        let p = expand_tilde("~");
        assert!(p.is_absolute());
        assert_eq!(p, dirs::home_dir().unwrap());
    }

    #[test]
    fn non_tilde_paths_pass_through() {
        let p = expand_tilde("/tmp/foo");
        assert_eq!(p, PathBuf::from("/tmp/foo"));
    }
}
