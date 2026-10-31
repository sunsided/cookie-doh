//! The Cache Cleaner: a robot vacuum that patrols the floor and now and then docks under the
//! lift. Anyone parked at the bottom lane when it arrives gets bumped. It gets restless when
//! the player camps down there.

/// Center X where it docks, under the lift.
pub const DOCK_X: f32 = 266.0;
/// Patrol range of its center along the floor.
pub const PATROL: (f32, f32) = (60.0, 240.0);
pub const PATROL_SPEED: f32 = 16.0;
pub const CHARGE_SPEED: f32 = 36.0;
pub const LEAVE_SPEED: f32 = 22.0;
/// Its timer runs this much faster while the player is parked at the bottom lane.
pub const CAMP_FACTOR: f32 = 3.0;
/// Body height; cookies falling off the bottom belt land on it while it is docked.
pub const HEIGHT: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CleanerState {
    /// Wandering the floor; heads for the dock when `t` runs out.
    Patrol {
        t: f32,
    },
    /// Beeping its way to the dock.
    Charge,
    Docked {
        t: f32,
    },
    /// Back out to the patrol range.
    Leave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanerSignal {
    None,
    /// Set off for the dock (warn the player).
    Coming,
    /// Just arrived under the lift.
    Arrived,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cleaner {
    /// Center X on the floor.
    pub x: f32,
    /// +1 facing right, -1 facing left.
    pub dir: f32,
    pub state: CleanerState,
}

impl Cleaner {
    pub fn new(patrol_for: f32) -> Self {
        Self {
            x: 120.0,
            dir: 1.0,
            state: CleanerState::Patrol { t: patrol_for },
        }
    }

    pub fn docked(&self) -> bool {
        matches!(self.state, CleanerState::Docked { .. })
    }

    /// Heading for the dock or sitting in it: the bottom lane is (about to be) unsafe.
    pub fn threatening(&self) -> bool {
        matches!(
            self.state,
            CleanerState::Charge | CleanerState::Docked { .. }
        )
    }

    /// Leave the dock, or abort the charge.
    pub fn retreat(&mut self) {
        if self.threatening() {
            self.state = CleanerState::Leave;
            self.dir = -1.0;
        }
    }

    /// Advance by `dt`. `camping`: the player is parked at the bottom lane. `dock_for` is how
    /// long it stays docked; `next_patrol` rolls the next patrol duration when it leaves.
    pub fn step(
        &mut self,
        dt: f32,
        camping: bool,
        dock_for: f32,
        next_patrol: impl FnOnce() -> f32,
    ) -> CleanerSignal {
        match self.state {
            CleanerState::Patrol { t } => {
                self.x += self.dir * PATROL_SPEED * dt;
                if self.x >= PATROL.1 {
                    self.dir = -1.0;
                } else if self.x <= PATROL.0 {
                    self.dir = 1.0;
                }
                let t = t - dt * if camping { CAMP_FACTOR } else { 1.0 };
                if t <= 0.0 {
                    self.state = CleanerState::Charge;
                    self.dir = 1.0;
                    return CleanerSignal::Coming;
                }
                self.state = CleanerState::Patrol { t };
            }
            CleanerState::Charge => {
                self.dir = 1.0;
                self.x += CHARGE_SPEED * dt;
                if self.x >= DOCK_X {
                    self.x = DOCK_X;
                    self.state = CleanerState::Docked { t: dock_for };
                    return CleanerSignal::Arrived;
                }
            }
            CleanerState::Docked { t } => {
                self.state = if t - dt <= 0.0 {
                    self.dir = -1.0;
                    CleanerState::Leave
                } else {
                    CleanerState::Docked { t: t - dt }
                };
            }
            CleanerState::Leave => {
                self.dir = -1.0;
                self.x -= LEAVE_SPEED * dt;
                if self.x <= PATROL.1 {
                    self.state = CleanerState::Patrol { t: next_patrol() };
                }
            }
        }
        CleanerSignal::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(c: &mut Cleaner, secs: f32, camping: bool) -> Vec<CleanerSignal> {
        let dt = 1.0 / 60.0;
        (0..(secs / dt) as usize)
            .map(|_| c.step(dt, camping, 3.0, || 20.0))
            .filter(|s| *s != CleanerSignal::None)
            .collect()
    }

    #[test]
    fn camping_brings_it_sooner() {
        let mut idle = Cleaner::new(9.0);
        let mut camped = Cleaner::new(9.0);
        assert!(run(&mut idle, 4.0, false).is_empty());
        assert_eq!(run(&mut camped, 4.0, true), vec![CleanerSignal::Coming]);
    }

    #[test]
    fn full_cycle_returns_to_patrol() {
        let mut c = Cleaner::new(1.0);
        let signals = run(&mut c, 15.0, false);
        assert_eq!(
            signals[..2],
            [CleanerSignal::Coming, CleanerSignal::Arrived]
        );
        assert!(matches!(c.state, CleanerState::Patrol { .. }));
        assert!((PATROL.0..=PATROL.1 + 1.0).contains(&c.x));
    }
}
