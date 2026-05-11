// Menu-bar tray icon. M1 ships a minimal menu: Region / Fullscreen / Open
// folder / Quit. The icon is a tiny built-in monochrome glyph; we'll swap in
// a proper template-image .icns later.

use anyhow::{Context, Result};
use std::sync::OnceLock;
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub enum MenuAction {
    Region,
    Fullscreen,
    Window,
    SilentRegion,
    SilentFullscreen,
    SilentWindow,
    PinLast,
    RepeatLast,
    Timed3s,
    Timed5s,
    Timed10s,
    OpenFolder,
    RevealSettings,
    RevealLog,
    About,
    Quit,
}

struct Ids {
    region: MenuId,
    fullscreen: MenuId,
    window: MenuId,
    silent_region: MenuId,
    silent_fullscreen: MenuId,
    silent_window: MenuId,
    pin_last: MenuId,
    repeat_last: MenuId,
    timed_3s: MenuId,
    timed_5s: MenuId,
    timed_10s: MenuId,
    open_folder: MenuId,
    reveal_settings: MenuId,
    reveal_log: MenuId,
    about: MenuId,
    quit: MenuId,
}

static IDS: OnceLock<Ids> = OnceLock::new();

pub fn build() -> Result<TrayIcon> {
    let menu = Menu::new();
    let region = MenuItem::new("Capture Region…\t⌃⌥⌘1", true, None);
    let window = MenuItem::new("Capture Window…\t⌃⌥⌘2", true, None);
    let fullscreen = MenuItem::new("Capture Fullscreen\t⌃⌥⌘3", true, None);
    let silent_region = MenuItem::new("Region (silent)", true, None);
    let silent_window = MenuItem::new("Window (silent)", true, None);
    let silent_fullscreen = MenuItem::new("Fullscreen (silent)", true, None);
    let pin_last = MenuItem::new("Pin Last Capture\t⌃⌥⌘.", true, None);
    let repeat_last = MenuItem::new("Repeat Last Capture\t⌃⌥⌘R", true, None);
    let timed_3s = MenuItem::new("Fullscreen in 3 s", true, None);
    let timed_5s = MenuItem::new("Fullscreen in 5 s", true, None);
    let timed_10s = MenuItem::new("Fullscreen in 10 s", true, None);
    let open_folder = MenuItem::new("Open Save Folder", true, None);
    let reveal_settings = MenuItem::new("Edit settings.toml…", true, None);
    let reveal_log = MenuItem::new("Reveal Log File", true, None);
    let about = MenuItem::new("About Screenshot Ultra…", true, None);
    let quit = MenuItem::new("Quit Screenshot Ultra", true, None);

    let ids = Ids {
        region: region.id().clone(),
        fullscreen: fullscreen.id().clone(),
        window: window.id().clone(),
        silent_region: silent_region.id().clone(),
        silent_fullscreen: silent_fullscreen.id().clone(),
        silent_window: silent_window.id().clone(),
        pin_last: pin_last.id().clone(),
        repeat_last: repeat_last.id().clone(),
        timed_3s: timed_3s.id().clone(),
        timed_5s: timed_5s.id().clone(),
        timed_10s: timed_10s.id().clone(),
        open_folder: open_folder.id().clone(),
        reveal_settings: reveal_settings.id().clone(),
        reveal_log: reveal_log.id().clone(),
        about: about.id().clone(),
        quit: quit.id().clone(),
    };
    let _ = IDS.set(ids);

    menu.append(&region).ok();
    menu.append(&window).ok();
    menu.append(&fullscreen).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    menu.append(&silent_region).ok();
    menu.append(&silent_window).ok();
    menu.append(&silent_fullscreen).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    menu.append(&pin_last).ok();
    menu.append(&repeat_last).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    menu.append(&timed_3s).ok();
    menu.append(&timed_5s).ok();
    menu.append(&timed_10s).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    menu.append(&open_folder).ok();
    menu.append(&reveal_settings).ok();
    menu.append(&reveal_log).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    menu.append(&about).ok();
    menu.append(&quit).ok();

    let icon = make_icon();

    let tray = TrayIconBuilder::new()
        .with_tooltip("Screenshot Ultra")
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_icon_as_template(true)
        .build()
        .context("building tray icon")?;

    Ok(tray)
}

pub fn menu_action(id: &MenuId) -> Option<MenuAction> {
    let ids = IDS.get()?;
    if id == &ids.region {
        Some(MenuAction::Region)
    } else if id == &ids.fullscreen {
        Some(MenuAction::Fullscreen)
    } else if id == &ids.window {
        Some(MenuAction::Window)
    } else if id == &ids.silent_region {
        Some(MenuAction::SilentRegion)
    } else if id == &ids.silent_fullscreen {
        Some(MenuAction::SilentFullscreen)
    } else if id == &ids.silent_window {
        Some(MenuAction::SilentWindow)
    } else if id == &ids.pin_last {
        Some(MenuAction::PinLast)
    } else if id == &ids.repeat_last {
        Some(MenuAction::RepeatLast)
    } else if id == &ids.timed_3s {
        Some(MenuAction::Timed3s)
    } else if id == &ids.timed_5s {
        Some(MenuAction::Timed5s)
    } else if id == &ids.timed_10s {
        Some(MenuAction::Timed10s)
    } else if id == &ids.open_folder {
        Some(MenuAction::OpenFolder)
    } else if id == &ids.reveal_settings {
        Some(MenuAction::RevealSettings)
    } else if id == &ids.reveal_log {
        Some(MenuAction::RevealLog)
    } else if id == &ids.about {
        Some(MenuAction::About)
    } else if id == &ids.quit {
        Some(MenuAction::Quit)
    } else {
        None
    }
}

/// 22×22 monochrome glyph drawn in code: a small camera-like square with a
/// circular lens. Template-style (alpha-only), so macOS tints it automatically.
fn make_icon() -> Icon {
    const SIZE: u32 = 22;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    let cx = SIZE as i32 / 2;
    let cy = SIZE as i32 / 2;
    for y in 0..SIZE as i32 {
        for x in 0..SIZE as i32 {
            let i = ((y * SIZE as i32 + x) * 4) as usize;
            // outer body: rounded square 2..20
            let body = (2..=19).contains(&x) && (5..=18).contains(&y);
            // viewfinder bump on top
            let bump = (7..=12).contains(&x) && (3..=5).contains(&y);
            // lens circle (transparent inner ring)
            let dx = x - cx;
            let dy = y - cy - 1;
            let r2 = dx * dx + dy * dy;
            let lens_outer = r2 <= 5 * 5;
            let lens_inner = r2 <= 3 * 3;

            let on = (body || bump) && !lens_inner;
            // Anti-alias the lens edge a touch.
            let alpha = if on {
                if lens_outer && !lens_inner {
                    255
                } else {
                    230
                }
            } else {
                0
            };
            rgba[i] = 0;
            rgba[i + 1] = 0;
            rgba[i + 2] = 0;
            rgba[i + 3] = alpha;
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("valid icon")
}
