//! Every difficulty knob, as a function of the level (1-based).

#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    /// Cookies to accept to clear the level.
    pub quota: u32,
    /// Belt speed range in px/s; each lane rolls its own speed within it per level.
    pub belt_speed: (f32, f32),
    /// Per-lane pause between two spits, in seconds.
    pub spawn_gap: (f32, f32),
    /// Most cookies in flight at once.
    pub max_cookies: usize,
    pub golden_chance: f32,
    /// Seconds between spider visits.
    pub spider_gap: (f32, f32),
    /// Seconds the spider hangs at the jar.
    pub spider_hang: f32,
    /// Spider descent speed, px/s.
    pub spider_speed: f32,
    /// Seconds between cookie-consent popups; `None` = no popups this level.
    pub popup_gap: Option<(f32, f32)>,
}

pub fn for_level(level: u32) -> Tuning {
    let l = level.max(1) as f32 - 1.0;
    let base = (27.0 + 5.0 * l).min(75.0);
    let gap_lo = (4.2 - 0.35 * l).max(1.1);
    Tuning {
        quota: 12 + 6 * (level.max(1) - 1).min(8),
        belt_speed: (base * 0.85, base * 1.2),
        spawn_gap: (gap_lo, gap_lo + (3.0 - 0.2 * l).max(1.2)),
        max_cookies: (2 + level as usize).min(9),
        golden_chance: 0.06,
        spider_gap: ((16.0 - 1.2 * l).max(6.0), (24.0 - 1.5 * l).max(10.0)),
        spider_hang: (2.4 + 0.15 * l).min(4.0),
        spider_speed: (38.0 + 4.0 * l).min(70.0),
        popup_gap: (level >= 2).then(|| ((18.0 - 1.5 * l).max(7.0), (28.0 - 2.0 * l).max(12.0))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_ramps_up() {
        let a = for_level(1);
        let b = for_level(6);
        assert!(b.belt_speed.0 > a.belt_speed.0);
        assert!(b.spawn_gap.0 < a.spawn_gap.0);
        assert!(b.quota > a.quota);
        assert!(a.popup_gap.is_none() && b.popup_gap.is_some());
    }

    #[test]
    fn knobs_stay_sane_at_absurd_levels() {
        let t = for_level(99);
        assert!(t.belt_speed.1 <= 90.0);
        assert!(t.spawn_gap.0 >= 1.0);
        assert!(t.max_cookies <= 9);
    }
}
