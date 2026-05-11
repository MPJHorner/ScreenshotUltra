// Screenshot Ultra — entry point.
//
// M1 milestone: menu-bar agent + global hotkey listener + region/fullscreen
// capture via macOS `screencapture` CLI + clipboard + disk sinks + NDJSON log.

mod capture;
mod hotkeys;
mod logging;
mod pin;
mod quick_tray;
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
    if handle_cli_args() {
        return Ok(());
    }

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
                    Some(tray::MenuAction::Window) => {
                        handle_action(hotkeys::Action::Window, &settings)
                    }
                    Some(tray::MenuAction::SilentRegion) => {
                        handle_action(hotkeys::Action::SilentRegion, &settings)
                    }
                    Some(tray::MenuAction::SilentFullscreen) => {
                        handle_action(hotkeys::Action::SilentFullscreen, &settings)
                    }
                    Some(tray::MenuAction::SilentWindow) => {
                        handle_action(hotkeys::Action::SilentWindow, &settings)
                    }
                    Some(tray::MenuAction::PinLast) => {
                        handle_action(hotkeys::Action::PinLast, &settings)
                    }
                    Some(tray::MenuAction::RepeatLast) => {
                        handle_action(hotkeys::Action::RepeatLast, &settings)
                    }
                    Some(tray::MenuAction::OpenFolder) => {
                        let _ = std::process::Command::new("open")
                            .arg(settings.general.save_folder_expanded())
                            .status();
                    }
                    Some(tray::MenuAction::RevealSettings) => {
                        if let Ok(path) = settings::Settings::path() {
                            let _ = std::process::Command::new("open").arg(&path).status();
                        }
                    }
                    Some(tray::MenuAction::RevealLog) => {
                        let _ = std::process::Command::new("open")
                            .arg("-R")
                            .arg(logging::log_path_for_reveal())
                            .status();
                    }
                    Some(tray::MenuAction::Quit) => *control_flow = ControlFlow::Exit,
                    None => {}
                },
            }
        }
    })
}

/// Handle one-shot CLI flags. Returns true when the program should exit
/// without starting the event loop. Only the first argument is consulted;
/// we don't have any flags that compose so there's no need for a real parser.
fn handle_cli_args() -> bool {
    let Some(arg) = std::env::args().nth(1) else {
        return false;
    };
    match arg.as_str() {
        "--version" | "-v" => {
            println!("screenshot-ultra {}", env!("CARGO_PKG_VERSION"));
            true
        }
        "--help" | "-h" => {
            print_help();
            true
        }
        "--settings-path" => {
            match Settings::path() {
                Ok(p) => println!("{}", p.display()),
                Err(err) => eprintln!("error: {err:#}"),
            }
            true
        }
        "--print-defaults" => {
            let s = Settings::default();
            match toml::to_string_pretty(&s) {
                Ok(raw) => println!("{raw}"),
                Err(err) => eprintln!("error: {err:#}"),
            }
            true
        }
        _ => {
            eprintln!("unknown argument: {arg}");
            eprintln!("try --help");
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!(
        "screenshot-ultra {} — snappy hotkey-first macOS screenshot & screen recorder

USAGE:
    screenshot-ultra [FLAGS]

FLAGS:
    -h, --help           Print this help message and exit
    -v, --version        Print version and exit
        --settings-path  Print the path to settings.toml and exit
        --print-defaults Print the default settings.toml contents and exit

Running without flags launches the menu-bar agent. Hotkeys are configured
in settings.toml (see --settings-path).

Docs: https://github.com/MPJHorner/ScreenshotUltra",
        env!("CARGO_PKG_VERSION")
    );
}

fn handle_action(action: hotkeys::Action, settings: &Settings) {
    match action {
        hotkeys::Action::PinLast => {
            match capture::last() {
                Some(last) => {
                    pin::pin(&last.path);
                    logging::event(serde_json::json!({
                        "evt": "pin",
                        "path": last.path.display().to_string(),
                    }));
                }
                None => eprintln!("pin: no previous capture to pin"),
            }
            return;
        }
        hotkeys::Action::RepeatLast => {
            match capture::last() {
                Some(last) => run_capture(last.mode, last.show_tray, settings),
                None => eprintln!("repeat_last: no previous capture"),
            }
            return;
        }
        _ => {}
    }

    let mode = match action {
        hotkeys::Action::Region | hotkeys::Action::SilentRegion => CaptureMode::Region,
        hotkeys::Action::Fullscreen | hotkeys::Action::SilentFullscreen => CaptureMode::Fullscreen,
        hotkeys::Action::Window | hotkeys::Action::SilentWindow => CaptureMode::Window,
        hotkeys::Action::PinLast | hotkeys::Action::RepeatLast => unreachable!(),
    };
    run_capture(mode, action.show_tray(), settings);
}

fn run_capture(mode: CaptureMode, show_tray: bool, settings: &Settings) {
    if let Err(err) = capture::run(mode, show_tray, settings) {
        eprintln!("capture failed: {err:#}");
        logging::event(serde_json::json!({
            "evt": "error",
            "where": "capture",
            "mode": mode.as_str(),
            "error": format!("{err:#}"),
        }));
    }
}
