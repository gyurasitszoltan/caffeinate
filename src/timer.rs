//! Időzített ébren tartás állapota: hátralévő/eltelt idő, lejárat eldöntése.

use std::time::{Duration, Instant};

/// Egy elindított időzítés adatait tárolja.
///
/// `started_at` a `TimerState::new` hívás pillanatában rögzítõdik,
/// így a hátralévõ idõ a `(total - elapsed)` képlettel számítható.
#[derive(Debug)]
pub struct TimerState {
    started_at: Instant,
    total: Duration,
}

impl TimerState {
    pub fn new(total: Duration) -> Self {
        Self {
            started_at: Instant::now(),
            total,
        }
    }

    /// A teljes idõtartam, ameddig az idõzítés tart.
    pub fn total(&self) -> Duration {
        self.total
    }

    /// Hogyannyi idõ telt el az indulás óta.
    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(self.started_at)
    }

    /// Hátralévõ idõ (sosem negatív — saturate).
    pub fn remaining(&self) -> Duration {
        self.total.saturating_sub(self.elapsed())
    }

    /// True, ha az idõzítés lejött.
    pub fn is_finished(&self) -> bool {
        self.elapsed() >= self.total
    }

    /// Eltelt / total arány 0..=1 (fejlesztõi/diagnosztikai célokra).
    #[allow(dead_code)]
    pub fn progress_ratio(&self) -> f64 {
        if self.total.is_zero() {
            return 1.0;
        }
        (self.elapsed().as_secs_f64() / self.total.as_secs_f64()).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaining_saturates_at_zero() {
        let t = TimerState::new(Duration::from_millis(10));
        std::thread::sleep(Duration::from_millis(30));
        assert!(t.is_finished());
        assert_eq!(t.remaining(), Duration::ZERO);
    }

    #[test]
    fn progress_clamps_to_one() {
        let t = TimerState::new(Duration::from_millis(10));
        std::thread::sleep(Duration::from_millis(30));
        assert!((t.progress_ratio() - 1.0).abs() < 1e-9);
    }
}
