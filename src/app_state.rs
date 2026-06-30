//! Alkalmazás állapot: az ébren tartás módja, kijelzõ flag, és az
//! aktuális ikon frame — egy helyen összefogva.

use crate::icons::{frame_for_remaining, IconFrame};
use crate::timer::TimerState;
use std::time::Duration;

/// Az ébren tartás módja. `Expired` egy tranziens nyugalmi állapot, ami
/// timer lejárat után jelzi, hogy az idõzítés lefutott (külön kell a
/// tiszta inaktív állapottól, mert a tooltip "lejárt"-ot mutat).
#[derive(Debug)]
pub enum AwakeMode {
    /// Semmi nincs blokkolva.
    Inactive,
    /// Manuális, idõkorlát nélküli ébren tartás.
    Indefinite,
    /// Idõzített ébren tartás a megadott teljes idõtartammal.
    Timed(TimerState),
    /// Az idõzítés lejött, az assertion már release-elve.
    Expired,
}

impl AwakeMode {
    pub fn is_active(&self) -> bool {
        matches!(self, AwakeMode::Indefinite | AwakeMode::Timed(_))
    }
}

/// A teljes alkalmazás állapot. Minden UI frissítés ebbõl származik.
pub struct AppState {
    pub awake_mode: AwakeMode,
    pub keep_display_awake: bool,
    /// Utoljára a tray-re állított frame — csak változáskor hívunk
    /// `set_icon(...)`-t, felesleges átállítás elkerülésére.
    pub current_icon_frame: IconFrame,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            awake_mode: AwakeMode::Inactive,
            keep_display_awake: false,
            current_icon_frame: IconFrame::Empty,
        }
    }

    /// True, ha épp aktív session (rendszer assertion él).
    pub fn is_active(&self) -> bool {
        self.awake_mode.is_active()
    }

    /// Tooltip szöveg az aktuális módból.
    pub fn tooltip(&self) -> String {
        match &self.awake_mode {
            AwakeMode::Inactive => "KeepAwake: inaktív".to_string(),
            AwakeMode::Indefinite => "KeepAwake: aktív".to_string(),
            AwakeMode::Timed(t) => {
                let r = t.remaining();
                let m = r.as_secs() / 60;
                let s = r.as_secs() % 60;
                format!("KeepAwake: {:02}:{:02} van hátra", m, s)
            }
            AwakeMode::Expired => "KeepAwake: lejárt".to_string(),
        }
    }

    /// A menü elsõ (disabled) státusz sorának szövege: ugyanaz, mint a
    /// tooltip (a terv 9. szakasza szerint a menüfej is állapotot mutat).
    #[allow(dead_code)]
    pub fn status_text(&self) -> String {
        self.tooltip()
    }

    /// Az aktuális állapot alapján elvárt ikon frame.
    pub fn desired_frame(&self) -> IconFrame {
        match &self.awake_mode {
            AwakeMode::Inactive | AwakeMode::Expired => IconFrame::Empty,
            AwakeMode::Indefinite => IconFrame::Full,
            AwakeMode::Timed(t) => frame_for_remaining(t.remaining(), t.total()),
        }
    }

    /// (remaining, total) a timed módban — a frame logikának kényelmes.
    #[allow(dead_code)]
    pub fn timed_window(&self) -> Option<(Duration, Duration)> {
        match &self.awake_mode {
            AwakeMode::Timed(t) => Some((t.remaining(), t.total())),
            _ => None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
