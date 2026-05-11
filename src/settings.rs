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
    pub hotkeys: Hotkeys,
    #[serde(default)]
    pub sinks: Sinks,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hotkeys {
    /// Region capture with Quick Tray (the standard flow). Empty = unbound.
    pub region: String,
    /// Fullscreen capture with Quick Tray (the standard flow). Empty = unbound.
    pub fullscreen: String,
    /// Region capture, silent: save to disk + clipboard, no UI. Empty = unbound.
    #[serde(default)]
    pub silent_region: String,
    /// Fullscreen capture, silent. Empty = unbound.
    #[serde(default)]
    pub silent_fullscreen: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sinks {
    pub clipboard: bool,
    pub disk: bool,
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
            silent_region: String::new(),
            silent_fullscreen: String::new(),
        }
    }
}

impl Default for Sinks {
    fn default() -> Self {
        Self {
            clipboard: true,
            disk: true,
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
