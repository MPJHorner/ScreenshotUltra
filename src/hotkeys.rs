// Global hotkey registration. Parses `ctrl+alt+cmd+1` style strings into
// `global_hotkey::hotkey::HotKey` and remembers which action each id maps to.

use anyhow::{anyhow, Context, Result};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;
use std::collections::HashMap;

use crate::settings::Hotkeys;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Region,
    Fullscreen,
    SilentRegion,
    SilentFullscreen,
}

impl Action {
    pub fn id(self) -> &'static str {
        match self {
            Action::Region => "region",
            Action::Fullscreen => "fullscreen",
            Action::SilentRegion => "silent_region",
            Action::SilentFullscreen => "silent_fullscreen",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Action::Region => "Region (tray)",
            Action::Fullscreen => "Fullscreen (tray)",
            Action::SilentRegion => "Region (silent)",
            Action::SilentFullscreen => "Fullscreen (silent)",
        }
    }
    pub fn show_tray(self) -> bool {
        matches!(self, Action::Region | Action::Fullscreen)
    }
}

pub struct Registered {
    _manager: GlobalHotKeyManager,
    by_id: HashMap<u32, Action>,
    pub actions: Vec<(Action, String)>,
}

impl Registered {
    pub fn action_for_id(&self, id: u32) -> Option<Action> {
        self.by_id.get(&id).copied()
    }
}

pub fn register_all(cfg: &Hotkeys) -> Result<Registered> {
    let manager = GlobalHotKeyManager::new().context("creating GlobalHotKeyManager")?;
    let mut by_id = HashMap::new();
    let mut actions = Vec::new();

    for (action, accel) in [
        (Action::Region, cfg.region.clone()),
        (Action::Fullscreen, cfg.fullscreen.clone()),
        (Action::SilentRegion, cfg.silent_region.clone()),
        (Action::SilentFullscreen, cfg.silent_fullscreen.clone()),
    ] {
        // Empty string = intentionally unbound. Skip silently.
        if accel.trim().is_empty() {
            continue;
        }
        let hk = parse(&accel)
            .with_context(|| format!("parsing hotkey for {}: {accel}", action.id()))?;
        manager
            .register(hk)
            .with_context(|| format!("registering hotkey {accel}"))?;
        by_id.insert(hk.id(), action);
        actions.push((action, accel));
    }

    Ok(Registered {
        _manager: manager,
        by_id,
        actions,
    })
}

fn parse(accel: &str) -> Result<HotKey> {
    let mut mods = Modifiers::empty();
    let mut code: Option<Code> = None;
    for raw in accel.split('+') {
        let part = raw.trim().to_ascii_lowercase();
        match part.as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" | "option" | "opt" => mods |= Modifiers::ALT,
            "cmd" | "command" | "super" | "meta" => mods |= Modifiers::SUPER,
            "shift" => mods |= Modifiers::SHIFT,
            other => code = Some(parse_code(other)?),
        }
    }
    let code = code.ok_or_else(|| anyhow!("hotkey has no key (only modifiers): {accel}"))?;
    Ok(HotKey::new(Some(mods), code))
}

fn parse_code(s: &str) -> Result<Code> {
    Ok(match s {
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "." | "period" => Code::Period,
        "," | "comma" => Code::Comma,
        "/" | "slash" => Code::Slash,
        "\\" | "backslash" => Code::Backslash,
        "space" => Code::Space,
        "tab" => Code::Tab,
        "enter" | "return" => Code::Enter,
        "esc" | "escape" => Code::Escape,
        other => anyhow::bail!("unsupported key: {other}"),
    })
}
