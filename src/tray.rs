//! Tray menü felépítése és menu-item ID -> parancs fordítása.
//!
//! Ez a modul csak UI-t és ID-ket ismer — az IOKit logika nem él itt,
//! csak a `TrayCommand` enum visszaadása a `main.rs` felé.

use std::time::Duration;
use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};

/// A menübõl érkezõ parancsok (a main.rs dolgozza fel).
#[derive(Debug)]
pub enum TrayCommand {
    /// "Ébren tartás bekapcsolása" — idõkorlát nélkül.
    StartIndefinite,
    /// "Ébren tartás N percig" — a megadott teljes idõtartam.
    StartTimed(Duration),
    /// "Leállítás" — release + inaktív.
    Stop,
    /// "Kijelzõt is ébren tartsa" checkbox váltás.
    ToggleDisplayAwake,
    /// "Kilépés" — release + event loop exit.
    Quit,
}

/// A menü létrehozása során kapott menu-item handle-ek. Ezeket a
/// main.rs-ben élve kell tartani, mert a `Menu` weak ref-ként hivatkozik
/// rájuk; egyben ezek kellenek a `set_text` / `set_checked` / `set_enabled`
/// frissítésekhez és az ID -> parancs fordításhoz.
#[allow(dead_code)]
pub struct MenuHandles {
    pub status: MenuItem, // disabled, csak státuszt mutat
    pub indefinite: MenuItem,
    pub t15: MenuItem,
    pub t30: MenuItem,
    pub t60: MenuItem,
    pub t120: MenuItem,
    pub stop: MenuItem,
    pub display: CheckMenuItem,
    pub quit: MenuItem,
}

/// Felépíti a tray menüt az implementációs terv 5. szakasza szerint:
///
/// ```text
/// KeepAwake: <állapot>            (disabled status)
/// ─────────────────────────
/// Ébren tartás bekapcsolása
/// Ébren tartás 15 percig
/// Ébren tartás 30 percig
/// Ébren tartás 1 óráig
/// Ébren tartás 2 óráig
/// Leállítás
/// ─────────────────────────
/// Kijelzõt is ébren tartsa ✓     (checkbox)
/// ─────────────────────────
/// Kilépés
/// ```
pub fn build_menu() -> (Menu, MenuHandles) {
    let status = MenuItem::with_id("status", "KeepAwake: inaktív", false, None);
    let indefinite = MenuItem::with_id("indefinite", "Ébren tartás bekapcsolása", true, None);
    let t15 = MenuItem::with_id("t15", "Ébren tartás 15 percig", true, None);
    let t30 = MenuItem::with_id("t30", "Ébren tartás 30 percig", true, None);
    let t60 = MenuItem::with_id("t60", "Ébren tartás 1 óráig", true, None);
    let t120 = MenuItem::with_id("t120", "Ébren tartás 2 óráig", true, None);
    let stop = MenuItem::with_id("stop", "Leállítás", false, None); // induláskor inaktív
    let display = CheckMenuItem::with_id("display", "Kijelzõt is ébren tartsa", true, false, None);
    let quit = MenuItem::with_id("quit", "Kilépés", true, None);

    let menu = Menu::new();
    menu.append_items(&[
        &status,
        &PredefinedMenuItem::separator(),
        &indefinite,
        &t15,
        &t30,
        &t60,
        &t120,
        &stop,
        &PredefinedMenuItem::separator(),
        &display,
        &PredefinedMenuItem::separator(),
        &quit,
    ])
    .expect("KeepAwake: menü felépítés sikertelen");

    let handles = MenuHandles {
        status,
        indefinite,
        t15,
        t30,
        t60,
        t120,
        stop,
        display,
        quit,
    };
    (menu, handles)
}

/// Egy menu event ID-jét `TrayCommand`-dá fordítja. Ismeretlen ID -> `None`.
pub fn command_for_id(id: &MenuId) -> Option<TrayCommand> {
    // Minden ID-t string-ként hasonlítunk össze. A `MenuId` mezõje publikus,
    // így `.0.as_str()` biztonságosan adja vissza a nyers azonosítót.
    let s = id.0.as_str();
    match s {
        "indefinite" => Some(TrayCommand::StartIndefinite),
        "t15" => Some(TrayCommand::StartTimed(Duration::from_secs(15 * 60))),
        "t30" => Some(TrayCommand::StartTimed(Duration::from_secs(30 * 60))),
        "t60" => Some(TrayCommand::StartTimed(Duration::from_secs(60 * 60))),
        "t120" => Some(TrayCommand::StartTimed(Duration::from_secs(2 * 60 * 60))),
        "stop" => Some(TrayCommand::Stop),
        "display" => Some(TrayCommand::ToggleDisplayAwake),
        "quit" => Some(TrayCommand::Quit),
        // "status" disabled, sosem érkezik event; minden más (predefined
        // separator) nem parancs.
        _ => None,
    }
}
