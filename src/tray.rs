// Menu-bar tray icon. M1 ships a minimal menu: Region / Fullscreen / Open
// folder / Quit. The icon is a tiny built-in monochrome glyph; we'll swap in
// a proper template-image .icns later.

use anyhow::{Context, Result};
use std::cell::RefCell;
use std::sync::OnceLock;
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

// Cached idle + recording icons so set_recording_indicator() doesn't
// rebuild them on every toggle.
thread_local! {
    static TRAY: RefCell<Option<TrayIcon>> = const { RefCell::new(None) };
    static ICON_IDLE: RefCell<Option<Icon>> = const { RefCell::new(None) };
    static ICON_RECORDING: RefCell<Option<Icon>> = const { RefCell::new(None) };
}

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
    RecordVideo,
    RecordGif,
    OpenFolder,
    RevealSettings,
    RevealLog,
    Preferences,
    Help,
    History,
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
    record_video: MenuId,
    record_gif: MenuId,
    open_folder: MenuId,
    reveal_settings: MenuId,
    reveal_log: MenuId,
    preferences: MenuId,
    help: MenuId,
    history: MenuId,
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
    // Recording — wording flips at runtime if a recording is in progress.
    let record_video = MenuItem::new(record_video_label(), true, None);
    let record_gif = MenuItem::new(record_gif_label(), true, None);
    let open_folder = MenuItem::new("Open Save Folder", true, None);
    let preferences = MenuItem::new("Preferences…\t⌃⌥⌘,", true, None);
    let help = MenuItem::new("Cheat Sheet\t⌃⌥⌘/", true, None);
    let history = MenuItem::new("History…\t⌃⌥⌘H", true, None);
    let reveal_settings = MenuItem::new("Edit settings.toml directly…", true, None);
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
        record_video: record_video.id().clone(),
        record_gif: record_gif.id().clone(),
        open_folder: open_folder.id().clone(),
        reveal_settings: reveal_settings.id().clone(),
        reveal_log: reveal_log.id().clone(),
        preferences: preferences.id().clone(),
        help: help.id().clone(),
        history: history.id().clone(),
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
    menu.append(&record_video).ok();
    menu.append(&record_gif).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    menu.append(&open_folder).ok();
    menu.append(&preferences).ok();
    menu.append(&help).ok();
    menu.append(&history).ok();
    menu.append(&reveal_settings).ok();
    menu.append(&reveal_log).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    menu.append(&about).ok();
    menu.append(&quit).ok();

    // Cache both icons up front; swap via set_recording_indicator(bool).
    let idle = make_icon();
    let recording = make_recording_icon();
    ICON_IDLE.with(|c| *c.borrow_mut() = Some(idle.clone()));
    ICON_RECORDING.with(|c| *c.borrow_mut() = Some(recording));

    let tray = TrayIconBuilder::new()
        .with_tooltip("Screenshot Ultra")
        .with_menu(Box::new(menu))
        .with_icon(idle)
        .with_icon_as_template(true)
        .build()
        .context("building tray icon")?;

    Ok(tray)
}

/// Hand the constructed TrayIcon to this module so we can swap its
/// icon later via set_recording_indicator. Call once, right after
/// `build()`, before the event loop starts.
pub fn register(tray: TrayIcon) {
    TRAY.with(|t| *t.borrow_mut() = Some(tray));
}

/// Flip the menu-bar icon between the idle aperture and the filled
/// "REC" dot. The icons are cached so each call is O(1).
pub fn set_recording_indicator(on: bool) {
    TRAY.with(|t| {
        if let Some(tray) = t.borrow().as_ref() {
            let icon = if on {
                ICON_RECORDING.with(|c| c.borrow().clone())
            } else {
                ICON_IDLE.with(|c| c.borrow().clone())
            };
            if let Some(icon) = icon {
                let _ = tray.set_icon(Some(icon));
            }
            let tooltip = if on {
                "Screenshot Ultra — recording"
            } else {
                "Screenshot Ultra"
            };
            let _ = tray.set_tooltip(Some(tooltip));
        }
    });
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
    } else if id == &ids.record_video {
        Some(MenuAction::RecordVideo)
    } else if id == &ids.record_gif {
        Some(MenuAction::RecordGif)
    } else if id == &ids.open_folder {
        Some(MenuAction::OpenFolder)
    } else if id == &ids.reveal_settings {
        Some(MenuAction::RevealSettings)
    } else if id == &ids.reveal_log {
        Some(MenuAction::RevealLog)
    } else if id == &ids.preferences {
        Some(MenuAction::Preferences)
    } else if id == &ids.help {
        Some(MenuAction::Help)
    } else if id == &ids.history {
        Some(MenuAction::History)
    } else if id == &ids.about {
        Some(MenuAction::About)
    } else if id == &ids.quit {
        Some(MenuAction::Quit)
    } else {
        None
    }
}

/// Label text for the "Record Video" menu item. Currently static; a future
/// pass can swap the text live when a recording is in progress so the
/// menu reads "Stop Recording" mid-take. (Requires retaining the
/// `MenuItem` and calling `set_text` on it — the tray-icon crate
/// supports this.)
fn record_video_label() -> &'static str {
    "Record Video / Stop\t⌃⌥⌘V"
}

fn record_gif_label() -> &'static str {
    "Record GIF / Stop\t⌃⌥⌘G"
}

/// 22×22 solid filled disc — the "recording" state. macOS tints
/// template images, but the dot still reads as filled + obviously
/// different from the open-iris idle state.
fn make_recording_icon() -> Icon {
    const SIZE: u32 = 22;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    let center = (SIZE as f32 - 1.0) / 2.0;
    let r = 7.5_f32;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = ((y * SIZE + x) * 4) as usize;
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > r {
                continue;
            }
            // Soft edge.
            let alpha = if dist > r - 0.6 {
                (((r - dist) / 0.6).clamp(0.0, 1.0) * 255.0) as u8
            } else {
                255
            };
            rgba[i] = 0;
            rgba[i + 1] = 0;
            rgba[i + 2] = 0;
            rgba[i + 3] = alpha;
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("valid icon")
}

/// 22×22 monochrome aperture glyph (matches the .app icon's iris).
/// Template-style (alpha-only) so macOS tints it automatically for light /
/// dark menu bars.
///
/// Geometry: an outer ring at radius 9, an inner lens hole at radius 3.5,
/// and six radial "blades" — pixels in the annular region (4 < r < 9) whose
/// angle is close to one of six evenly-spaced 60° spokes get cleared so the
/// blades read as gaps. Result: a recognisable camera-aperture silhouette
/// at the macOS menu-bar size.
fn make_icon() -> Icon {
    const SIZE: u32 = 22;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    let center = (SIZE as f32 - 1.0) / 2.0;
    let r_outer = 9.5_f32;
    let r_inner = 3.5_f32;
    let blade_w = 0.45_f32; /* spoke half-width in radians */

    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = ((y * SIZE + x) * 4) as usize;
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let r = (dx * dx + dy * dy).sqrt();

            // Outside the aperture circle or inside the lens hole → empty.
            if r > r_outer || r < r_inner {
                continue;
            }
            // Six spokes at 30°, 90°, 150°, 210°, 270°, 330° (offset 30° so
            // we get a hexagonal aperture, not a square one).
            let theta = dy.atan2(dx).rem_euclid(std::f32::consts::FRAC_PI_3);
            let dist_to_spoke = (theta - std::f32::consts::FRAC_PI_6).abs();
            if dist_to_spoke < blade_w {
                continue; /* leave this pixel transparent — it's a blade gap */
            }
            // Anti-alias the outer edge.
            let alpha = if r > r_outer - 0.6 {
                (((r_outer - r) / 0.6).clamp(0.0, 1.0) * 255.0) as u8
            } else if r < r_inner + 0.6 {
                (((r - r_inner) / 0.6).clamp(0.0, 1.0) * 255.0) as u8
            } else {
                255
            };
            rgba[i] = 0;
            rgba[i + 1] = 0;
            rgba[i + 2] = 0;
            rgba[i + 3] = alpha;
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("valid icon")
}
