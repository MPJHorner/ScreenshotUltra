// Screenshot Ultra — entry point.
//
// M1 milestone: menu-bar agent + global hotkey listener + region/fullscreen
// capture via macOS `screencapture` CLI + clipboard + disk sinks + NDJSON log.

mod about;
mod capture;
mod editor;
mod eyedropper;
mod help;
mod history;
mod hotkeys;
mod logging;
mod ocr;
mod pin;
mod preferences;
mod quick_tray;
mod recording;
mod settings;
mod sinks;
mod tray;

use anyhow::{Context, Result};
use global_hotkey::GlobalHotKeyEvent;
use std::sync::Arc;
use std::time::Duration;
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tray_icon::menu::MenuEvent;

use crate::capture::CaptureMode;
use crate::settings::Settings;

#[derive(Debug, Clone)]
pub enum UserEvent {
    Hotkey(GlobalHotKeyEvent),
    Menu(MenuEvent),
    ReloadSettings,
}

fn main() -> Result<()> {
    if handle_cli_args() {
        return Ok(());
    }

    let settings = Settings::load_or_default().context("loading settings")?;
    logging::init(&settings).context("initialising logging")?;
    logging::install_panic_handler();

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

    let mut registered = hotkeys::register_all(&settings.hotkeys)?;
    let tray_icon = tray::build()?;
    tray::register(tray_icon); // park the TrayIcon in the tray module so
                               // set_recording_indicator() and similar can mutate it later.
    let mut settings = Arc::new(settings);

    eprintln!(
        "Screenshot Ultra v{} — menu-bar agent running. Default hotkeys:",
        env!("CARGO_PKG_VERSION")
    );
    print_bindings(&registered);

    // Watch settings.toml in a background thread and post a ReloadSettings
    // user event whenever the file's mtime changes. The main thread handles
    // the reload so we don't have to wrestle with thread-safety inside
    // hotkeys::register_all (which holds OS-level handles).
    spawn_settings_watcher(event_loop.create_proxy());

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
                    Some(tray::MenuAction::Timed3s) => run_timed(3, &settings),
                    Some(tray::MenuAction::Timed5s) => run_timed(5, &settings),
                    Some(tray::MenuAction::Timed10s) => run_timed(10, &settings),
                    Some(tray::MenuAction::RecordVideo) => {
                        handle_action(hotkeys::Action::RecordVideo, &settings)
                    }
                    Some(tray::MenuAction::RecordGif) => {
                        handle_action(hotkeys::Action::RecordGif, &settings)
                    }
                    Some(tray::MenuAction::About) => about::show(),
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
                    Some(tray::MenuAction::Preferences) => preferences::show(),
                    Some(tray::MenuAction::Help) => help::show(&settings),
                    Some(tray::MenuAction::History) => history::show(&settings),
                    Some(tray::MenuAction::Quit) => *control_flow = ControlFlow::Exit,
                    None => {}
                },
                UserEvent::ReloadSettings => match Settings::load_or_default() {
                    Ok(new) => match hotkeys::register_all(&new.hotkeys) {
                        Ok(new_reg) => {
                            registered = new_reg;
                            settings = Arc::new(new);
                            eprintln!("settings reloaded — bindings now:");
                            print_bindings(&registered);
                            logging::event(serde_json::json!({"evt": "settings_reloaded"}));
                        }
                        Err(err) => {
                            eprintln!(
                                "settings reload: invalid hotkeys, keeping previous: {err:#}"
                            );
                            logging::event(serde_json::json!({
                                "evt": "settings_reload_error",
                                "error": format!("{err:#}"),
                            }));
                        }
                    },
                    Err(err) => {
                        eprintln!("settings reload: parse failed, keeping previous: {err:#}");
                    }
                },
            }
        }
    })
}

fn print_bindings(registered: &hotkeys::Registered) {
    for (action, accel) in &registered.actions {
        eprintln!("  {:<22} {}", action.label(), accel);
    }
}

/// Watch `settings.toml` on a background thread and post `ReloadSettings`
/// when it changes. mtime polling at 1 Hz is plenty for an interactive
/// config file; we don't need inotify/FSEvents complexity here.
fn spawn_settings_watcher(proxy: EventLoopProxy<UserEvent>) {
    std::thread::spawn(move || {
        let Ok(path) = Settings::path() else {
            return;
        };
        let mut last = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let now = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            if now != last && now.is_some() {
                last = now;
                if proxy.send_event(UserEvent::ReloadSettings).is_err() {
                    return; // main loop is gone; stop watching.
                }
            }
        }
    });
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
        hotkeys::Action::OpenClipboardImage => {
            match capture::from_clipboard(settings) {
                Ok(true) => {}
                Ok(false) => eprintln!("open_clipboard_image: no image on the clipboard"),
                Err(err) => eprintln!("open_clipboard_image: {err:#}"),
            }
            return;
        }
        hotkeys::Action::ColorPicker => {
            eyedropper::pick();
            return;
        }
        hotkeys::Action::Preferences => {
            preferences::show();
            return;
        }
        hotkeys::Action::RecordVideo => {
            if let Err(err) = recording::toggle(recording::RecordingKind::Video, settings) {
                eprintln!("record_video: {err:#}");
            }
            return;
        }
        hotkeys::Action::RecordGif => {
            if let Err(err) = recording::toggle(recording::RecordingKind::Gif, settings) {
                eprintln!("record_gif: {err:#}");
            }
            return;
        }
        hotkeys::Action::Help => {
            help::show(settings);
            return;
        }
        hotkeys::Action::History => {
            history::show(settings);
            return;
        }
        _ => {}
    }

    let mode = match action {
        hotkeys::Action::Region | hotkeys::Action::SilentRegion => CaptureMode::Region,
        hotkeys::Action::Fullscreen | hotkeys::Action::SilentFullscreen => CaptureMode::Fullscreen,
        hotkeys::Action::Window | hotkeys::Action::SilentWindow => CaptureMode::Window,
        hotkeys::Action::PinLast
        | hotkeys::Action::RepeatLast
        | hotkeys::Action::OpenClipboardImage
        | hotkeys::Action::ColorPicker
        | hotkeys::Action::Preferences
        | hotkeys::Action::RecordVideo
        | hotkeys::Action::RecordGif
        | hotkeys::Action::Help
        | hotkeys::Action::History => unreachable!(),
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

fn run_timed(delay_secs: u32, settings: &Settings) {
    if let Err(err) = capture::run_timed_fullscreen(delay_secs, settings) {
        eprintln!("timed capture failed: {err:#}");
        logging::event(serde_json::json!({
            "evt": "error",
            "where": "capture_timed",
            "delay_s": delay_secs,
            "error": format!("{err:#}"),
        }));
    }
}
