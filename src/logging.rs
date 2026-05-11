// NDJSON event log: one JSON object per line, appended to
// `~/Library/Logs/ScreenshotUltra/log.ndjson`. Mirrors MailBox Ultra.

use anyhow::Result;
use chrono::Utc;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::settings::Settings;

static LOG: OnceLock<Mutex<Option<File>>> = OnceLock::new();

pub fn init(_settings: &Settings) -> Result<()> {
    let path = log_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok();
    LOG.set(Mutex::new(file))
        .map_err(|_| anyhow::anyhow!("logging already initialised"))?;
    Ok(())
}

pub fn event(mut value: serde_json::Value) {
    if let serde_json::Value::Object(ref mut m) = value {
        m.insert(
            "ts".to_string(),
            serde_json::Value::String(Utc::now().to_rfc3339()),
        );
    }
    let line = serde_json::to_string(&value).unwrap_or_default();
    if let Some(slot) = LOG.get() {
        if let Ok(mut guard) = slot.lock() {
            if let Some(file) = guard.as_mut() {
                let _ = writeln!(file, "{line}");
                let _ = file.flush();
            }
        }
    }
}

fn log_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
    Ok(home.join("Library/Logs/ScreenshotUltra/log.ndjson"))
}

/// Resolve the log file path for the "Reveal Log File" menu item.
/// Falls back to the user's home dir if `$HOME` is unset (unlikely on macOS).
pub fn log_path_for_reveal() -> PathBuf {
    log_path().unwrap_or_else(|_| PathBuf::from("."))
}
