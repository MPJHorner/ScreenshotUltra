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
    OpenFolder,
    Quit,
}

struct Ids {
    region: MenuId,
    fullscreen: MenuId,
    open_folder: MenuId,
    quit: MenuId,
}

static IDS: OnceLock<Ids> = OnceLock::new();

pub fn build() -> Result<TrayIcon> {
    let menu = Menu::new();
    let region = MenuItem::new("Capture Region\t⌃⌥⌘1", true, None);
    let fullscreen = MenuItem::new("Capture Fullscreen\t⌃⌥⌘3", true, None);
    let open_folder = MenuItem::new("Open Save Folder", true, None);
    let quit = MenuItem::new("Quit Screenshot Ultra", true, None);

    let ids = Ids {
        region: region.id().clone(),
        fullscreen: fullscreen.id().clone(),
        open_folder: open_folder.id().clone(),
        quit: quit.id().clone(),
    };
    let _ = IDS.set(ids);

    menu.append(&region).ok();
    menu.append(&fullscreen).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    menu.append(&open_folder).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
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
    } else if id == &ids.open_folder {
        Some(MenuAction::OpenFolder)
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
