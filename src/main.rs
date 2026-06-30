//! KeepAwake — macOS menüsor utility, ami IOKit power assertion-nel
//! ébren tartja a gépet. Tray ikon a hátralévõ idõt (kávéscsésze
//! töltöttsége) mutatja.
//!
//! Architektúra (lásd implementation.md 10. szakasz):
//!   - `awake.rs`      — IOKit FFI + assertion életciklus
//!   - `timer.rs`      — idõzített mód állapota
//!   - `icons.rs`      — PNG frame-ek + cache + frame kiválasztás
//!   - `app_state.rs`  — alkalmazás állapot modell
//!   - `tray.rs`       — menü + parancsok
//!   - `main.rs`       — tao event loop + események bekötése

mod app_state;
mod awake;
mod icons;
mod timer;
mod tray;

use std::time::{Duration, Instant};

use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};

use app_state::{AppState, AwakeMode};
use awake::AwakeController;
use icons::{IconCache, IconFrame};
use timer::TimerState;
use tray::{command_for_id, build_menu, TrayCommand};

/// A tao event loop számára küldhetõ egyedi események: a tray-icon és a
/// menü event-eket egy user-event channel-be tereljük, hogy a loop felébredjen
/// (a tao/tray-icon dokumentáció ajánlott mintája).
enum UserEvent {
    #[allow(dead_code)]
    TrayIconEvent(tray_icon::TrayIconEvent),
    MenuEvent(tray_icon::menu::MenuEvent),
}

/// Tick ciklus periódusa — másodpercenként frissítjük az ikont/tooltip-ot.
const TICK_INTERVAL: Duration = Duration::from_secs(1);

fn main() {
    // Nincs ablak, csak tray. A `tao` event loop-nak a main threaden kell
    // futnia macOS-en (a tray-icon is itt jön létre).
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // tray + menu event-ek továbbítása a loopba EventLoopProxy-n át.
    {
        let proxy = event_loop.create_proxy();
        TrayIconEvent::set_event_handler(Some(move |event| {
            let _ = proxy.send_event(UserEvent::TrayIconEvent(event));
        }));
    }
    {
        let proxy = event_loop.create_proxy();
        tray_icon::menu::MenuEvent::set_event_handler(Some(move |event| {
            let _ = proxy.send_event(UserEvent::MenuEvent(event));
        }));
    }

    // ctrlc handler CLI / fejlesztõi módból: tiszta release + kilépés.
    // (A tray menübõl is van Kilépés; ez a biztonsági háló.)
    {
        let proxy = event_loop.create_proxy();
        ctrlc::set_handler(move || {
            // Nem tudunk közvetlenül IOKit-et hívni innen (másik thread),
            // de a loop loop-ba küldünk egy Quit-et: a proxy-n nincs kilépés
            // user-event, így egyszerûen egy újraélesztést kérünk és a
            // main loop-ban a Drop végzi a tiszta release-t. Hogy biztos
            // leálljunk, a következõ tick a control_flow-ot Exit-re állítja
            // a LoopDestroyed útján — de a Drop az AwakeController-en garantálja
            // az assertion release-t processz kilépéskor.
            let _ = proxy.send_event(UserEvent::MenuEvent(make_quit_event()));
        })
        .expect("KeepAwake: ctrlc handler telepítése sikertelen");
    }

    // Ikon cache (mind az 5 frame) betöltése startupkor.
    let icon_cache = IconCache::new();

    // Menü felépítése; a handle-eket élve tartjuk a closure-ben.
    let (menu, menu_handles) = build_menu();

    // Állapot + assertion kontroller.
    let mut app_state = AppState::new();
    let mut awake = AwakeController::new();

    // A tray icon csak akkor hozható létre, ha az event loop már fut — a
    // tao `NewEvents(Init)` pillanatában építjük.
    let mut tray_icon: Option<TrayIcon> = None;

    // Cache az utolsó tooltip / status / stop-enabled értékhez, hogy csak
    // változáskor hívjunk set_*-ot.
    let mut last_tooltip = String::new();
    let mut last_stop_enabled: Option<bool> = None;
    let mut last_indefinite_enabled: Option<bool> = None;

    event_loop.run(move |event, _window_target, control_flow| {
        // Másodpercenként ébredünk a tick frissítéshez. A menu/tray event-ek
        // azonnal felébresztenek (WaitCancelled).
        *control_flow = ControlFlow::WaitUntil(Instant::now() + TICK_INTERVAL);

        match event {
            // ── Indulás: tray ikon létrehozása ─────────────────────────
            Event::NewEvents(StartCause::Init) => {
                let icon = icon_cache.get(IconFrame::Empty).clone();
                tray_icon = Some(
                    TrayIconBuilder::new()
                        .with_menu(Box::new(menu.clone()))
                        .with_tooltip("KeepAwake: inaktív")
                        .with_icon(icon)
                        .build()
                        .expect("KeepAwake: tray ikon létrehozása sikertelen"),
                );
                // macOS template kép: a rendszer tinteli light/dark menüsörhöz.
                tray_icon
                    .as_ref()
                    .expect("tray épp létrehozva")
                    .set_icon_as_template(true);
                menu_handles
                    .status
                    .set_text("KeepAwake: inaktív");
            }

            // ── Menu parancs ───────────────────────────────────────────
            Event::UserEvent(UserEvent::MenuEvent(ev)) => {
                // A checkbox eseménynél a MenuEvent csak az ID-t hozza; magát
                // a checked állapotot mi állítjuk be a saját flag-ünk alapján.
                let cmd = command_for_id(&ev.id);
                match cmd {
                    Some(TrayCommand::StartIndefinite) => {
                        if let Err(e) = awake.start(app_state.keep_display_awake) {
                            eprintln!("KeepAwake: {}", e);
                            return;
                        }
                        app_state.awake_mode = AwakeMode::Indefinite;
                    }
                    Some(TrayCommand::StartTimed(total)) => {
                        if let Err(e) = awake.start(app_state.keep_display_awake) {
                            eprintln!("KeepAwake: {}", e);
                            return;
                        }
                        app_state.awake_mode = AwakeMode::Timed(TimerState::new(total));
                    }
                    Some(TrayCommand::Stop) => {
                        awake.stop();
                        app_state.awake_mode = AwakeMode::Inactive;
                    }
                    Some(TrayCommand::ToggleDisplayAwake) => {
                        app_state.keep_display_awake = !app_state.keep_display_awake;
                        menu_handles
                            .display
                            .set_checked(app_state.keep_display_awake);
                        if let Err(e) = awake.set_keep_display_awake(app_state.keep_display_awake) {
                            eprintln!("KeepAwake: {}", e);
                        }
                    }
                    Some(TrayCommand::Quit) => {
                        awake.stop();
                        tray_icon.take();
                        *control_flow = ControlFlow::Exit;
                    }
                    None => {
                        // separator / unknown — semmi
                    }
                }
            }

            // Tray icon kattintás event — jelen milestone-ban nem használjuk,
            // de elnyeljük, hogy ne legyen dead code.
            Event::UserEvent(UserEvent::TrayIconEvent(_)) => {}

            // ── Tick: timer lejárat + UI frissítés ────────────────────
            // `MainEventsCleared` minden loop-iterációban (event vagy
            // WaitUntil lejárat) lefut — ide kerül a másodperces frissítés.
            Event::MainEventsCleared => {
                // 1. Timer lejárat ellenõrzése.
                if let AwakeMode::Timed(t) = &app_state.awake_mode {
                    if t.is_finished() {
                        awake.stop();
                        app_state.awake_mode = AwakeMode::Expired;
                    }
                }

                // 2. Ikon frame (csak változáskor set).
                let frame = app_state.desired_frame();
                if frame != app_state.current_icon_frame {
                    app_state.current_icon_frame = frame;
                    if let Some(tray) = tray_icon.as_ref() {
                        let icon = icon_cache.get(frame).clone();
                        if let Err(e) = tray.set_icon_with_as_template(Some(icon), true) {
                            eprintln!("KeepAwake: ikon frissítés hiba: {}", e);
                        }
                    }
                }

                // 3. Tooltip + menü status sor + Leállítás enabled.
                let tip = app_state.tooltip();
                if tip != last_tooltip {
                    last_tooltip = tip.clone();
                    if let Some(tray) = tray_icon.as_ref() {
                        if let Err(e) = tray.set_tooltip(Some(&tip)) {
                            eprintln!("KeepAwake: tooltip frissítés hiba: {}", e);
                        }
                    }
                    menu_handles.status.set_text(&tip);
                }

                let stop_enabled = app_state.is_active();
                if Some(stop_enabled) != last_stop_enabled {
                    last_stop_enabled = Some(stop_enabled);
                    menu_handles.stop.set_enabled(stop_enabled);
                }

                // Az idő nélküli bekapcsolás ne legyen újra választható,
                // ha már pont ez a mód aktív. Az időzített opciók szándékosan
                // aktívak maradnak, hogy lehessen másik időtartamra váltani.
                let indefinite_enabled = !matches!(app_state.awake_mode, AwakeMode::Indefinite);
                if Some(indefinite_enabled) != last_indefinite_enabled {
                    last_indefinite_enabled = Some(indefinite_enabled);
                    menu_handles.indefinite.set_enabled(indefinite_enabled);
                }
            }

            // ── Loop leállás: biztonsági release ──────────────────────
            Event::LoopDestroyed => {
                awake.stop();
            }

            _ => {}
        }
    });
}

/// Csinál egy szintetikus `MenuEvent`-et a "quit" ID-vel, amit a ctrlc
/// handler küld a loopba, hogy a normál Kilépés úton menjen a tiszta
/// leállás (stop + Drop + exit).
fn make_quit_event() -> tray_icon::menu::MenuEvent {
    // A MenuEvent csak ID-t hordoz; a konkrét MenuItem referenciát nem.
    // A `command_for_id` a "quit" string-et ismeri fel. A MenuId mezõje
    // publikus, így tuple-konstruktorral biztonságosan építhetõ.
    tray_icon::menu::MenuEvent {
        id: tray_icon::menu::MenuId("quit".to_string()),
    }
}
