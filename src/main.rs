// Screenshot Ultra — entry point.
//
// M1 milestone: menu-bar agent + global hotkey listener + region/fullscreen
// capture via macOS `screencapture` CLI + clipboard + disk sinks + NDJSON log.

mod capture;
mod hotkeys;
mod logging;
mod settings;
mod sinks;
mod tray;

use anyhow::{Context, Result};
use global_hotkey::GlobalHotKeyEvent;
use std::sync::Arc;
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::MenuEvent;

use crate::capture::CaptureMode;
use crate::settings::Settings;

#[derive(Debug, Clone)]
pub enum UserEvent {
    Hotkey(GlobalHotKeyEvent),
    Menu(MenuEvent),
}

fn main() -> Result<()> {
    let settings = Settings::load_or_default().context("loading settings")?;
    logging::init(&settings).context("initialising logging")?;

    logging::event(serde_json::json!({
        "evt": "startup",
        "version": env!("CARGO_PKG_VERSION"),
        "save_folder": settings.general.save_folder,
    }));

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // Pump hotkey + menu events into the tao event loop.
    let proxy_hk = event_loop.create_proxy();
    GlobalHotKeyEvent::set_event_handler(Some(move |e| {
        let _ = proxy_hk.send_event(UserEvent::Hotkey(e));
    }));
    let proxy_menu = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy_menu.send_event(UserEvent::Menu(e));
    }));

    let registered = hotkeys::register_all(&settings.hotkeys)?;
    let _tray = tray::build()?; // keep alive for the lifetime of the app
    let settings = Arc::new(settings);

    eprintln!(
        "Screenshot Ultra v{} — menu-bar agent running. Default hotkeys:",
        env!("CARGO_PKG_VERSION")
    );
    for (action, accel) in &registered.actions {
        eprintln!("  {:<14} {}", action.label(), accel);
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::UserEvent(ev) = event {
            match ev {
                UserEvent::Hotkey(e) => {
                    if e.state != global_hotkey::HotKeyState::Pressed {
                        return;
                    }
                    let Some(action) = registered.action_for_id(e.id) else {
                        return;
                    };
                    logging::event(serde_json::json!({
                        "evt": "hotkey",
                        "action": action.id(),
                    }));
                    handle_action(action, &settings);
                }
                UserEvent::Menu(e) => match tray::menu_action(&e.id) {
                    Some(tray::MenuAction::Region) => {
                        handle_action(hotkeys::Action::Region, &settings)
                    }
                    Some(tray::MenuAction::Fullscreen) => {
                        handle_action(hotkeys::Action::Fullscreen, &settings)
                    }
                    Some(tray::MenuAction::OpenFolder) => {
                        let _ = std::process::Command::new("open")
                            .arg(settings.general.save_folder_expanded())
                            .status();
                    }
                    Some(tray::MenuAction::Quit) => *control_flow = ControlFlow::Exit,
                    None => {}
                },
            }
        }
    })
}

fn handle_action(action: hotkeys::Action, settings: &Settings) {
    let mode = match action {
        hotkeys::Action::Region => CaptureMode::Region,
        hotkeys::Action::Fullscreen => CaptureMode::Fullscreen,
    };
    if let Err(err) = capture::run(mode, settings) {
        eprintln!("capture failed: {err:#}");
        logging::event(serde_json::json!({
            "evt": "error",
            "where": "capture",
            "mode": mode.as_str(),
            "error": format!("{err:#}"),
        }));
    }
}
