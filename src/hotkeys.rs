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
    Window,
    SilentRegion,
    SilentFullscreen,
    SilentWindow,
    PinLast,
    RepeatLast,
    OpenClipboardImage,
    ColorPicker,
    Preferences,
}

impl Action {
    pub fn id(self) -> &'static str {
        match self {
            Action::Region => "region",
            Action::Fullscreen => "fullscreen",
            Action::Window => "window",
            Action::SilentRegion => "silent_region",
            Action::SilentFullscreen => "silent_fullscreen",
            Action::SilentWindow => "silent_window",
            Action::PinLast => "pin_last",
            Action::RepeatLast => "repeat_last",
            Action::OpenClipboardImage => "open_clipboard_image",
            Action::ColorPicker => "color_picker",
            Action::Preferences => "preferences",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Action::Region => "Region (tray)",
            Action::Fullscreen => "Fullscreen (tray)",
            Action::Window => "Window (tray)",
            Action::SilentRegion => "Region (silent)",
            Action::SilentFullscreen => "Fullscreen (silent)",
            Action::SilentWindow => "Window (silent)",
            Action::PinLast => "Pin last capture",
            Action::RepeatLast => "Repeat last capture",
            Action::OpenClipboardImage => "Open clipboard image",
            Action::ColorPicker => "Colour picker",
            Action::Preferences => "Preferences…",
        }
    }
    pub fn show_tray(self) -> bool {
        matches!(self, Action::Region | Action::Fullscreen | Action::Window)
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
        (Action::Window, cfg.window.clone()),
        (Action::SilentRegion, cfg.silent_region.clone()),
        (Action::SilentFullscreen, cfg.silent_fullscreen.clone()),
        (Action::SilentWindow, cfg.silent_window.clone()),
        (Action::PinLast, cfg.pin_last.clone()),
        (Action::RepeatLast, cfg.repeat_last.clone()),
        (Action::OpenClipboardImage, cfg.open_clipboard_image.clone()),
        (Action::ColorPicker, cfg.color_picker.clone()),
        (Action::Preferences, cfg.preferences.clone()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_accelerator() {
        let hk = parse("ctrl+alt+cmd+1").unwrap();
        assert!(hk.mods.contains(Modifiers::CONTROL));
        assert!(hk.mods.contains(Modifiers::ALT));
        assert!(hk.mods.contains(Modifiers::SUPER));
        assert_eq!(hk.key, Code::Digit1);
    }

    #[test]
    fn accepts_modifier_aliases() {
        let hk = parse("control+option+command+r").unwrap();
        assert!(hk.mods.contains(Modifiers::CONTROL));
        assert!(hk.mods.contains(Modifiers::ALT));
        assert!(hk.mods.contains(Modifiers::SUPER));
        assert_eq!(hk.key, Code::KeyR);
    }

    #[test]
    fn parses_punctuation_keys() {
        let hk = parse("ctrl+alt+cmd+.").unwrap();
        assert_eq!(hk.key, Code::Period);
        let hk = parse("ctrl+alt+cmd+period").unwrap();
        assert_eq!(hk.key, Code::Period);
    }

    #[test]
    fn rejects_modifier_only_binding() {
        assert!(parse("ctrl+alt+cmd").is_err());
    }

    #[test]
    fn rejects_unknown_key() {
        let err = parse("ctrl+alt+cmd+f25").unwrap_err();
        assert!(format!("{err}").contains("unsupported key"));
    }

    #[test]
    fn whitespace_is_tolerated() {
        let hk = parse(" ctrl + alt + cmd + 3 ").unwrap();
        assert_eq!(hk.key, Code::Digit3);
    }

    #[test]
    fn register_all_skips_empty_bindings() {
        // Default config has empty silent_* slots; the rest should register.
        let cfg = Hotkeys::default();
        let reg = register_all(&cfg).unwrap();
        // region/window/fullscreen + pin_last + repeat_last
        // + open_clipboard_image + color_picker + preferences = 8
        assert_eq!(reg.actions.len(), 8);
        // None should be a silent_* action since they're empty by default.
        assert!(reg.actions.iter().all(|(a, _)| !matches!(
            a,
            Action::SilentRegion | Action::SilentFullscreen | Action::SilentWindow
        )));
    }
}
