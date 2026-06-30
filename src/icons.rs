//! PNG tray ikon frame-ek: betöltés, frame kiválasztás hátralévõ idõbõl,
//! memóriába cache-elés, hogy tickenként ne kelljen fájlból olvasni.

use std::collections::HashMap;
use std::time::Duration;
use tray_icon::Icon;

/// A kávéscsésze 5 frame-e. A csésze a *hátralévõ* idõt mutatja:
/// `Full` = sok idõ van hátra, `Empty` = inaktív / lejárt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconFrame {
    /// `cup_0.png` — üres csésze / inaktív / lejárt.
    Empty,
    /// `cup_1.png` — kevés idõ van hátra.
    Low,
    /// `cup_2.png` — közepes idõ van hátra.
    Mid,
    /// `cup_3.png` — sok idõ van hátra.
    High,
    /// `cup_4.png` — tele csésze / frissen indított, ill. manuális mód.
    Full,
}

impl IconFrame {
    /// 0..=4 index, ami a `cup_N.png` fájlneveknek felel meg.
    pub fn index(self) -> usize {
        self as usize
    }

    /// Mind a 5 frame iterálható sorrendben (Empty..Full).
    pub fn all() -> [IconFrame; 5] {
        [IconFrame::Empty, IconFrame::Low, IconFrame::Mid, IconFrame::High, IconFrame::Full]
    }
}

/// Hátralévõ / total arány alapján frame-et választ.
///
/// Az implementációs terv threshold-jait követi:
///   >0.80 -> Full, >0.60 -> High, >0.40 -> Mid, >0.20 -> Low, else Empty.
/// `total == 0` (manuális / inaktív) -> `Full` végtelen idõhöz tartozó
/// jelölés; az inaktív / lejárt állapotot külön kezeljük `Empty`-vel.
pub fn frame_for_remaining(remaining: Duration, total: Duration) -> IconFrame {
    if total.is_zero() {
        return IconFrame::Full;
    }
    let ratio = remaining.as_secs_f64() / total.as_secs_f64();
    if ratio > 0.80 {
        IconFrame::Full
    } else if ratio > 0.60 {
        IconFrame::High
    } else if ratio > 0.40 {
        IconFrame::Mid
    } else if ratio > 0.20 {
        IconFrame::Low
    } else {
        IconFrame::Empty
    }
}

/// Egy frame-hez tartozó PNG elérési út. A `@2x` retina változatot
/// használjuk (64×64) a menüsor éles megjelenítéséhez; a rendszer
/// template-ként skálázza/tinteli.
///
/// Az út a `CARGO_MANIFEST_DIR`-bõl indul, így `cargo run`-tól független
/// az aktuális munkakönyvtár, és dev módban mindig megtalálja az asset-et.
pub fn path_for_frame(frame: IconFrame) -> String {
    format!("{}/assets/cup_{}@2x.png", env!("CARGO_MANIFEST_DIR"), frame.index())
}

/// Egy PNG-t `tray_icon::Icon`-tá alakít.
fn load_icon(path: &str) -> Icon {
    let image = image::open(path)
        .unwrap_or_else(|e| panic!("KeepAwake: ikonfájl nem olvasható ({}) — {}", path, e))
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height)
        .unwrap_or_else(|e| panic!("KeepAwake: Icon::from_rgba hiba ({}) — {}", path, e))
}

/// Startupkor az összes 5 frame-et betölti a memóriába, hogy a tick
/// ciklusban csak pointer-költséggel legyen elérhetõ.
pub struct IconCache {
    icons: HashMap<IconFrame, Icon>,
}

impl IconCache {
    /// Betölti mind az 5 PNG-t. Hibára (hiányzó fájl / corrupt PNG)
    /// pánikol, mert startup kritikus hibának számít (a terv szerint).
    pub fn new() -> Self {
        let mut icons = HashMap::with_capacity(5);
        for frame in IconFrame::all() {
            let path = path_for_frame(frame);
            let icon = load_icon(&path);
            icons.insert(frame, icon);
        }
        Self { icons }
    }

    /// Adott frame-hez tartozó ikon referenciát ad.
    pub fn get(&self, frame: IconFrame) -> &Icon {
        self.icons
            .get(&frame)
            .expect("KeepAwake: frame hiányzik a cache-bõl (ez nem lehetene before)")
    }
}

impl Default for IconCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_when_total_zero() {
        assert_eq!(frame_for_remaining(Duration::ZERO, Duration::ZERO), IconFrame::Full);
    }

    #[test]
    fn full_when_most_remaining() {
        // 90% hátra
        assert_eq!(frame_for_remaining(Duration::from_secs(90), Duration::from_secs(100)), IconFrame::Full);
    }

    #[test]
    fn empty_when_almost_elapsed() {
        // 10% hátra
        assert_eq!(frame_for_remaining(Duration::from_secs(10), Duration::from_secs(100)), IconFrame::Empty);
    }

    #[test]
    fn mid_at_half() {
        assert_eq!(frame_for_remaining(Duration::from_secs(50), Duration::from_secs(100)), IconFrame::Mid);
    }

    #[test]
    fn paths_indexed_zero_to_four() {
        assert!(path_for_frame(IconFrame::Empty).ends_with("cup_0@2x.png"));
        assert!(path_for_frame(IconFrame::Full).ends_with("cup_4@2x.png"));
    }

    #[test]
    fn cache_loads_all_five() {
        // csak dev gépken fut, ahol az assets jelen van
        let cache = IconCache::new();
        for f in IconFrame::all() {
            assert!(cache.get(f) as *const Icon as *const () as usize != 0);
        }
    }
}
