//! The rules of Cookie D'oh!, free of any engine types.
//!
//! Websites on the left spit cookies onto four conveyor belts. The player rides a lift on the
//! right and has to *accept* every cookie into the jar. A cookie that drops past the jar
//! crumbles on the floor: "D'oh!". Meanwhile the Web Crawler (a spider) drops in to the jar
//! and bites anyone standing there, and consent popups hide the belts until accepted.

pub mod cleaner;
pub mod layout;
mod rng;
pub mod tuning;

pub use rng::Rng;
pub use tuning::Tuning;

use cleaner::{Cleaner, CleanerSignal};
use layout::*;

/// Number of D'ohs before the session expires.
pub const START_LIVES: u32 = 3;
/// Cookies the jar holds before it is emptied for a bonus.
pub const JAR_CAPACITY: u32 = 10;
/// Seconds a spat cookie flies before it lands on the belt.
pub const SPIT_TIME: f32 = 0.35;
/// Seconds a caught cookie takes to drop into the jar.
pub const DROP_IN_TIME: f32 = 0.14;
/// Lanes per second the lift travels.
pub const LIFT_SPEED: f32 = 14.0;
/// Gravity for cookies that fell off a belt, px/s².
pub const GRAVITY: f32 = 420.0;
/// Horizontal speed of a cookie tipping off the belt end.
pub const TIP_SPEED: f32 = 22.0;
/// Freeze after a D'oh.
pub const DOH_TIME: f32 = 3.0;
pub const INTRO_TIME: f32 = 2.0;
pub const CLEAR_TIME: f32 = 3.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CookieKind {
    Plain,
    Golden,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CookieState {
    /// In the air from a website's mouth to the belt; `t` counts up to [`SPIT_TIME`].
    Spat {
        t: f32,
    },
    OnBelt,
    /// Accepted: dropping into the jar of `lane`. `saved` = caught one lane down, mid-fall.
    DropIn {
        lane: usize,
        t: f32,
        from: (f32, f32),
        saved: bool,
    },
    /// Tipped off the end with nobody there. It drops onto the end of the belt below (or the
    /// floor) and smashes there, unless the jar one lane down catches it on the way.
    Falling {
        vx: f32,
        vy: f32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cookie {
    pub id: u32,
    pub lane: usize,
    pub kind: CookieKind,
    /// Top-left corner, framebuffer pixels.
    pub x: f32,
    pub y: f32,
    pub state: CookieState,
}

impl Cookie {
    pub fn center_x(&self) -> f32 {
        self.x + COOKIE_SIZE / 2.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    /// The lane the lift is heading to.
    pub lane: usize,
    /// Animated lift position in lanes (fractional while travelling).
    pub pos: f32,
    /// Cookies in the jar.
    pub jar: u32,
    /// Counts down after a catch, for the happy face.
    pub joy: f32,
}

impl Player {
    /// The lane the lift is parked at, if it isn't moving.
    pub fn parked(&self) -> Option<usize> {
        ((self.pos - self.lane as f32).abs() < 0.2).then_some(self.lane)
    }

    /// The lane the lift is closest to right now.
    pub fn nearest(&self) -> usize {
        (self.pos.round() as usize).min(LANES - 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpiderState {
    Away { t: f32 },
    Down { lane: usize, y: f32 },
    Hang { lane: usize, t: f32 },
    Up { y: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Popup {
    /// The upper of the two lanes it covers.
    pub lane: usize,
    pub age: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DohCause {
    /// A cookie crumbled on the floor (its lane).
    Dropped(usize),
    Bitten,
    /// Parked at the bottom lane when the Cache Cleaner docked.
    Bumped,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// "LEVEL n - GET READY"
    Intro {
        t: f32,
    },
    Play,
    Doh {
        t: f32,
        cause: DohCause,
    },
    Clear {
        t: f32,
    },
    Over,
}

/// Things that happened during an update, for sound and particles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Spit {
        lane: usize,
    },
    Moved,
    Accepted {
        lane: usize,
        kind: CookieKind,
        points: u32,
        /// Caught mid-fall one lane down (pays double).
        saved: bool,
    },
    JarFull {
        points: u32,
    },
    TippedOff {
        lane: usize,
    },
    /// A cookie smashed at `x, y` (on the belt end below, or the floor).
    Crumbled {
        x: f32,
        y: f32,
        kind: CookieKind,
    },
    Doh(DohCause),
    SpiderComing {
        lane: usize,
    },
    SpiderAte {
        lane: usize,
    },
    CleanerComing,
    CleanerAte,
    PopupOpened,
    PopupAccepted {
        points: u32,
    },
    LevelClear {
        bonus: u32,
    },
    LevelStart {
        level: u32,
    },
    GameOver,
}

#[derive(Clone, Debug)]
pub struct World {
    pub level: u32,
    pub score: u32,
    pub lives: u32,
    /// Cookies accepted this level.
    pub accepted: u32,
    pub phase: Phase,
    pub player: Player,
    pub cookies: Vec<Cookie>,
    pub spider: SpiderState,
    pub cleaner: Cleaner,
    pub popup: Option<Popup>,
    /// Per-lane belt speed, px/s.
    pub belt_speed: [f32; LANES],
    /// Accumulated belt travel per lane, for animating the treads.
    pub belt_phase: [f32; LANES],
    /// Per-lane countdown to the next spit; also drives the mouth animation.
    pub spawn_timer: [f32; LANES],
    pub popup_timer: f32,
    /// Spider and popups on (tests switch them off to check the cookie scheduler alone).
    pub hazards: bool,
    pub tuning: Tuning,
    pub time: f32,
    pub events: Vec<Event>,
    rng: Rng,
    next_id: u32,
}

impl World {
    pub fn new(seed: u64) -> Self {
        let tuning = tuning::for_level(1);
        let mut w = World {
            level: 1,
            score: 0,
            lives: START_LIVES,
            accepted: 0,
            phase: Phase::Intro { t: INTRO_TIME },
            player: Player {
                lane: 1,
                pos: 1.0,
                jar: 0,
                joy: 0.0,
            },
            cookies: Vec::new(),
            spider: SpiderState::Away { t: 10.0 },
            cleaner: Cleaner::new(20.0),
            popup: None,
            belt_speed: [0.0; LANES],
            belt_phase: [0.0; LANES],
            spawn_timer: [0.0; LANES],
            popup_timer: 0.0,
            hazards: true,
            tuning,
            time: 0.0,
            events: Vec::new(),
            rng: Rng::new(seed),
            next_id: 1,
        };
        w.start_level(1);
        w
    }

    /// Cookies still needed to clear the level.
    pub fn remaining(&self) -> u32 {
        self.tuning.quota.saturating_sub(self.accepted)
    }

    fn start_level(&mut self, level: u32) {
        self.level = level;
        self.tuning = tuning::for_level(level);
        self.accepted = 0;
        let (lo, hi) = self.tuning.belt_speed;
        for lane in 0..LANES {
            self.belt_speed[lane] = self.rng.range(lo, hi);
            // Stagger the first spits so the level eases in.
            self.spawn_timer[lane] = self.rng.range(0.5, 3.5) + lane as f32 * 0.3;
        }
        self.cookies.clear();
        self.popup = None;
        self.reset_hazard_timers();
        self.phase = Phase::Intro { t: INTRO_TIME };
        self.events.push(Event::LevelStart { level });
    }

    fn reset_hazard_timers(&mut self) {
        let (lo, hi) = self.tuning.spider_gap;
        self.spider = SpiderState::Away {
            t: self.rng.range(lo, hi) * 0.6,
        };
        self.popup_timer = match self.tuning.popup_gap {
            Some((lo, hi)) => self.rng.range(lo, hi),
            None => f32::INFINITY,
        };
        // The Cache Cleaner keeps its own schedule across levels and D'ohs; it only backs off
        // if it was on its way, and never waits longer than this level allows.
        self.cleaner.retreat();
        let (_, hi) = self.tuning.cleaner_gap;
        if let cleaner::CleanerState::Patrol { t } = &mut self.cleaner.state {
            *t = t.min(hi);
        }
    }

    // ----- input -----------------------------------------------------------------------

    pub fn can_move(&self) -> bool {
        matches!(
            self.phase,
            Phase::Intro { .. } | Phase::Play | Phase::Clear { .. }
        )
    }

    pub fn move_to(&mut self, lane: usize) {
        let lane = lane.min(LANES - 1);
        if self.can_move() && lane != self.player.lane {
            self.player.lane = lane;
            self.events.push(Event::Moved);
        }
    }

    pub fn move_up(&mut self) {
        if self.player.lane > 0 {
            self.move_to(self.player.lane - 1);
        }
    }

    pub fn move_down(&mut self) {
        self.move_to(self.player.lane + 1);
    }

    /// "Accept all" - dismisses a consent popup. Returns whether there was one.
    pub fn accept_all(&mut self) -> bool {
        if self.phase != Phase::Play || self.popup.is_none() {
            return false;
        }
        self.popup = None;
        let points = 50 * self.level;
        self.score += points;
        self.events.push(Event::PopupAccepted { points });
        if let Some((lo, hi)) = self.tuning.popup_gap {
            self.popup_timer = self.rng.range(lo, hi);
        }
        true
    }

    // ----- simulation ------------------------------------------------------------------

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        match self.phase {
            Phase::Intro { t } => {
                self.move_lift(dt);
                self.phase = if t - dt <= 0.0 {
                    Phase::Play
                } else {
                    Phase::Intro { t: t - dt }
                };
            }
            Phase::Play => self.step_play(dt),
            Phase::Doh { t, cause } => {
                if t - dt > 0.0 {
                    self.phase = Phase::Doh { t: t - dt, cause };
                } else if self.lives == 0 {
                    self.phase = Phase::Over;
                    self.events.push(Event::GameOver);
                } else {
                    // Fresh board, same level, progress kept.
                    self.cookies.clear();
                    self.popup = None;
                    self.reset_hazard_timers();
                    for lane in 0..LANES {
                        self.spawn_timer[lane] = self.rng.range(0.6, 2.5);
                    }
                    self.phase = Phase::Intro {
                        t: INTRO_TIME * 0.6,
                    };
                }
            }
            Phase::Clear { t } => {
                self.move_lift(dt);
                self.player.joy = 1.0;
                if t - dt <= 0.0 {
                    self.start_level(self.level + 1);
                } else {
                    self.phase = Phase::Clear { t: t - dt };
                }
            }
            Phase::Over => {}
        }
    }

    fn move_lift(&mut self, dt: f32) {
        let p = &mut self.player;
        let target = p.lane as f32;
        let step = LIFT_SPEED * dt;
        if (target - p.pos).abs() <= step {
            p.pos = target;
        } else {
            p.pos += step * (target - p.pos).signum();
        }
        p.joy = (p.joy - dt).max(0.0);
    }

    fn step_play(&mut self, dt: f32) {
        self.move_lift(dt);
        for lane in 0..LANES {
            self.belt_phase[lane] += self.belt_speed[lane] * dt;
        }
        if self.hazards {
            self.step_spider(dt);
            if self.phase != Phase::Play {
                return;
            }
            self.step_cleaner(dt);
            if self.phase != Phase::Play {
                return;
            }
            self.step_popup(dt);
        }
        self.step_spawns(dt);
        self.step_cookies(dt);
        if self.phase == Phase::Play && self.accepted >= self.tuning.quota {
            let bonus = 250 * self.level + 100 * self.lives;
            self.score += bonus;
            self.events.push(Event::LevelClear { bonus });
            // Leftovers vanish; the jar's contents count as delivered.
            self.cookies.clear();
            self.popup = None;
            self.spider = SpiderState::Up {
                y: self.spider_y().unwrap_or(CEILING_Y),
            };
            self.cleaner.retreat();
            self.phase = Phase::Clear { t: CLEAR_TIME };
        }
    }

    fn doh(&mut self, cause: DohCause) {
        if self.phase != Phase::Play {
            return;
        }
        self.lives = self.lives.saturating_sub(1);
        self.player.joy = 0.0;
        self.phase = Phase::Doh { t: DOH_TIME, cause };
        self.events.push(Event::Doh(cause));
    }

    /// The spider's body top, while it's on screen.
    pub fn spider_y(&self) -> Option<f32> {
        match self.spider {
            SpiderState::Away { .. } => None,
            SpiderState::Down { y, .. } | SpiderState::Up { y } => Some(y),
            SpiderState::Hang { lane, .. } => Some(spider_hang_y(lane)),
        }
    }

    fn spider_lane(&self) -> Option<usize> {
        match self.spider {
            SpiderState::Hang { lane, .. } => Some(lane),
            _ => None,
        }
    }

    fn step_spider(&mut self, dt: f32) {
        let speed = self.tuning.spider_speed;
        self.spider = match self.spider {
            SpiderState::Away { t } if t - dt > 0.0 => SpiderState::Away { t: t - dt },
            SpiderState::Away { .. } => {
                let lane = self.rng.below(LANES);
                self.events.push(Event::SpiderComing { lane });
                SpiderState::Down {
                    lane,
                    y: CEILING_Y - 12.0,
                }
            }
            SpiderState::Down { lane, y } => {
                let y = y + speed * dt;
                if y >= spider_hang_y(lane) {
                    SpiderState::Hang {
                        lane,
                        t: self.tuning.spider_hang,
                    }
                } else {
                    SpiderState::Down { lane, y }
                }
            }
            SpiderState::Hang { lane, t } if t - dt > 0.0 => SpiderState::Hang { lane, t: t - dt },
            SpiderState::Hang { lane, .. } => SpiderState::Up {
                y: spider_hang_y(lane),
            },
            SpiderState::Up { y } if y > CEILING_Y - 14.0 => SpiderState::Up {
                y: y - speed * 1.6 * dt,
            },
            SpiderState::Up { .. } => {
                let (lo, hi) = self.tuning.spider_gap;
                SpiderState::Away {
                    t: self.rng.range(lo, hi),
                }
            }
        };
        if let Some(lane) = self.spider_lane()
            && self.player.nearest() == lane
        {
            // Off it goes again, satisfied.
            self.spider = SpiderState::Up {
                y: spider_hang_y(lane),
            };
            self.doh(DohCause::Bitten);
        }
    }

    fn step_cleaner(&mut self, dt: f32) {
        let bottom = LANES - 1;
        let camping = self.player.parked() == Some(bottom);
        let (lo, hi) = self.tuning.cleaner_gap;
        let gap = self.rng.range(lo, hi);
        match self
            .cleaner
            .step(dt, camping, self.tuning.cleaner_dock, || gap)
        {
            CleanerSignal::Coming => self.events.push(Event::CleanerComing),
            CleanerSignal::Arrived | CleanerSignal::None => {}
        }
        if self.cleaner.docked() && self.player.nearest() == bottom {
            self.cleaner.retreat();
            self.doh(DohCause::Bumped);
        }
    }

    fn step_popup(&mut self, dt: f32) {
        if let Some(p) = &mut self.popup {
            p.age += dt;
            return;
        }
        self.popup_timer -= dt;
        if self.popup_timer <= 0.0 {
            self.popup = Some(Popup {
                lane: self.rng.below(LANES - 1),
                age: 0.0,
            });
            self.popup_timer = f32::INFINITY;
            self.events.push(Event::PopupOpened);
        }
    }

    /// Seconds until a cookie leaves its belt (reaches [`CATCH_X`]), if it's still coming.
    pub fn arrival_in(&self, c: &Cookie) -> Option<f32> {
        let speed = self.belt_speed[c.lane];
        match c.state {
            CookieState::Spat { t } => Some(SPIT_TIME - t + (CATCH_X - BELT_START_X) / speed),
            CookieState::OnBelt => Some(((CATCH_X - c.x) / speed).max(0.0)),
            _ => None,
        }
    }

    /// Minimum time between two arrivals on lanes `a` and `b` so a perfect player can make both.
    pub fn min_gap(a: usize, b: usize) -> f32 {
        if a == b {
            0.0
        } else {
            0.15 + a.abs_diff(b) as f32 / LIFT_SPEED * 1.6
        }
    }

    #[allow(clippy::needless_range_loop)]
    fn step_spawns(&mut self, dt: f32) {
        for lane in 0..LANES {
            self.spawn_timer[lane] -= dt;
            if self.spawn_timer[lane] > 0.0 {
                continue;
            }
            let in_flight = self
                .cookies
                .iter()
                .filter(|c| matches!(c.state, CookieState::Spat { .. } | CookieState::OnBelt))
                .count();
            if in_flight >= self.tuning.max_cookies {
                self.spawn_timer[lane] = 0.3;
                continue;
            }
            let arrival = SPIT_TIME + (CATCH_X - BELT_START_X) / self.belt_speed[lane];
            let clash = self.cookies.iter().any(|c| {
                self.arrival_in(c)
                    .is_some_and(|t| (t - arrival).abs() < Self::min_gap(lane, c.lane))
            });
            if clash {
                self.spawn_timer[lane] = self.rng.range(0.15, 0.5);
                continue;
            }
            let kind = if self.rng.chance(self.tuning.golden_chance) {
                CookieKind::Golden
            } else {
                CookieKind::Plain
            };
            self.cookies.push(Cookie {
                id: self.next_id,
                lane,
                kind,
                x: SPIT_FROM.0,
                y: LANE_Y[lane] + SPIT_FROM.1,
                state: CookieState::Spat { t: 0.0 },
            });
            self.next_id += 1;
            self.events.push(Event::Spit { lane });
            let (lo, hi) = self.tuning.spawn_gap;
            self.spawn_timer[lane] = self.rng.range(lo, hi);
        }
    }

    /// Top of the jar opening at `lane` and its horizontal extent.
    pub fn jar_mouth(lane: usize) -> (f32, f32, f32) {
        (LANE_Y[lane] + 1.0, BELT_END_X - 2.0, BELT_END_X + 14.0)
    }

    fn step_cookies(&mut self, dt: f32) {
        let parked = self.player.parked();
        let spider_lane = self.spider_lane();
        let cleaner_docked = self.hazards && self.cleaner.docked();
        let mut vacuumed = Vec::new();
        let mut accepted = Vec::new();
        let mut crumbled = Vec::new();
        let mut eaten = Vec::new();
        for (i, c) in self.cookies.iter_mut().enumerate() {
            let belt_y = LANE_Y[c.lane] - COOKIE_SIZE;
            match c.state {
                CookieState::Spat { t } => {
                    let t = t + dt;
                    let k = (t / SPIT_TIME).min(1.0);
                    c.x = SPIT_FROM.0 + (BELT_START_X - SPIT_FROM.0) * k;
                    let y0 = LANE_Y[c.lane] + SPIT_FROM.1;
                    c.y = y0 + (belt_y - y0) * k - 10.0 * (k * std::f32::consts::PI).sin();
                    c.state = if k >= 1.0 {
                        c.y = belt_y;
                        CookieState::OnBelt
                    } else {
                        CookieState::Spat { t }
                    };
                }
                CookieState::OnBelt => {
                    c.x += self.belt_speed[c.lane] * dt;
                    c.y = belt_y;
                    if c.x >= CATCH_X {
                        c.x = CATCH_X;
                        if spider_lane == Some(c.lane) {
                            eaten.push(i);
                        } else if parked == Some(c.lane) {
                            c.state = CookieState::DropIn {
                                lane: c.lane,
                                t: 0.0,
                                from: (c.x, c.y),
                                saved: false,
                            };
                        } else {
                            c.state = CookieState::Falling {
                                vx: TIP_SPEED,
                                vy: -20.0,
                            };
                            self.events.push(Event::TippedOff { lane: c.lane });
                        }
                    }
                }
                CookieState::DropIn {
                    lane,
                    t,
                    from,
                    saved,
                } => {
                    let t = t + dt;
                    let k = (t / DROP_IN_TIME).min(1.0);
                    let to = (BELT_END_X + 2.0, LANE_Y[lane] + 2.0);
                    c.x = from.0 + (to.0 - from.0) * k;
                    c.y = from.1 + (to.1 - from.1) * k * k - 4.0 * (k * std::f32::consts::PI).sin();
                    if k >= 1.0 {
                        accepted.push(i);
                    } else {
                        c.state = CookieState::DropIn {
                            lane,
                            t,
                            from,
                            saved,
                        };
                    }
                }
                CookieState::Falling { vx, vy } => {
                    let prev_y = c.y;
                    let vy = vy + GRAVITY * dt;
                    c.x += vx * dt;
                    c.y += vy * dt;
                    c.state = CookieState::Falling { vx, vy };
                    let below = c.lane + 1;
                    let bottom = c.y + COOKIE_SIZE;
                    if below < LANES {
                        // Only the jar directly below is in the way: it can dive-save it.
                        let (mouth_y, x0, x1) = Self::jar_mouth(below);
                        let cx = c.x + COOKIE_SIZE / 2.0;
                        if parked == Some(below)
                            && prev_y + COOKIE_SIZE <= mouth_y + 4.0
                            && bottom >= mouth_y
                            && (x0..=x1).contains(&cx)
                        {
                            c.state = CookieState::DropIn {
                                lane: below,
                                t: DROP_IN_TIME * 0.5,
                                from: (c.x, c.y),
                                saved: true,
                            };
                            continue;
                        }
                        // Otherwise it smashes on the end of that belt.
                        if bottom >= LANE_Y[below] + 2.0 {
                            c.y = LANE_Y[below] + 2.0 - COOKIE_SIZE;
                            crumbled.push(i);
                        }
                    } else if cleaner_docked && bottom >= FLOOR_Y - cleaner::HEIGHT {
                        // Landed on the Cache Cleaner, which clears it.
                        vacuumed.push(i);
                    } else if bottom >= FLOOR_Y {
                        crumbled.push(i);
                    }
                }
            }
        }

        let mut gone: Vec<usize> = Vec::new();
        for &i in &accepted {
            let c = &self.cookies[i];
            let base = match c.kind {
                CookieKind::Plain => 10,
                CookieKind::Golden => 100,
            };
            let (lane, saved) = match c.state {
                CookieState::DropIn { lane, saved, .. } => (lane, saved),
                _ => (c.lane, false),
            };
            let points = base * self.level * if saved { 2 } else { 1 };
            self.score += points;
            self.accepted += 1;
            self.events.push(Event::Accepted {
                lane,
                kind: c.kind,
                points,
                saved,
            });
            self.player.joy = 0.6;
            self.player.jar += 1;
            if self.player.jar >= JAR_CAPACITY {
                self.player.jar = 0;
                let points = 100 * self.level;
                self.score += points;
                self.events.push(Event::JarFull { points });
            }
            gone.push(i);
        }
        for &i in &eaten {
            self.events.push(Event::SpiderAte {
                lane: self.cookies[i].lane,
            });
            gone.push(i);
        }
        for &i in &vacuumed {
            self.events.push(Event::CleanerAte);
            gone.push(i);
        }
        let mut dropped = None;
        for &i in &crumbled {
            let c = &self.cookies[i];
            self.events.push(Event::Crumbled {
                x: c.center_x(),
                y: c.y + COOKIE_SIZE,
                kind: c.kind,
            });
            dropped.get_or_insert(c.lane);
            gone.push(i);
        }
        gone.sort_unstable();
        for i in gone.into_iter().rev() {
            self.cookies.remove(i);
        }
        if let Some(lane) = dropped {
            self.doh(DohCause::Dropped(lane));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;

    fn playing(seed: u64) -> World {
        let mut w = World::new(seed);
        w.hazards = false;
        w.phase = Phase::Play;
        w.spawn_timer = [f32::INFINITY; LANES];
        w.events.clear();
        w
    }

    fn put_cookie(w: &mut World, lane: usize, x: f32) {
        w.cookies.push(Cookie {
            id: 999,
            lane,
            kind: CookieKind::Plain,
            x,
            y: LANE_Y[lane] - COOKIE_SIZE,
            state: CookieState::OnBelt,
        });
    }

    fn run(w: &mut World, secs: f32) {
        let steps = (secs / DT) as usize;
        for _ in 0..steps {
            w.update(DT);
        }
    }

    #[test]
    fn parked_player_accepts_the_cookie() {
        let mut w = playing(1);
        w.player.lane = 2;
        w.player.pos = 2.0;
        put_cookie(&mut w, 2, CATCH_X - 1.0);
        run(&mut w, 0.5);
        assert_eq!(w.accepted, 1);
        assert_eq!(w.score, 10);
        assert_eq!(w.player.jar, 1);
        assert!(w.cookies.is_empty());
        assert!(matches!(
            w.events.as_slice(),
            [Event::Accepted { lane: 2, .. }]
        ));
    }

    #[test]
    fn missed_cookie_crumbles_and_costs_a_life() {
        let mut w = playing(1);
        w.player.lane = 0;
        w.player.pos = 0.0;
        put_cookie(&mut w, 3, CATCH_X - 1.0);
        run(&mut w, 1.0);
        assert_eq!(w.lives, START_LIVES - 1);
        assert!(matches!(
            w.phase,
            Phase::Doh {
                cause: DohCause::Dropped(3),
                ..
            }
        ));
        assert!(w.events.iter().any(|e| matches!(e, Event::Crumbled { .. })));
    }

    #[test]
    fn the_jar_one_lane_down_saves_for_double_points() {
        let mut w = playing(1);
        w.player.lane = 2;
        w.player.pos = 2.0;
        put_cookie(&mut w, 1, CATCH_X - 0.5);
        run(&mut w, 1.5);
        assert_eq!(w.accepted, 1, "events: {:?}", w.events);
        assert_eq!(w.lives, START_LIVES);
        assert_eq!(w.score, 20);
        assert!(
            w.events
                .iter()
                .any(|e| matches!(e, Event::Accepted { saved: true, .. }))
        );
    }

    /// Camping at the bottom used to catch everything that fell; now a cookie smashes on
    /// the end of the next belt down.
    #[test]
    fn jars_further_down_cannot_save() {
        let mut w = playing(1);
        w.player.lane = 3;
        w.player.pos = 3.0;
        put_cookie(&mut w, 0, CATCH_X - 0.5);
        run(&mut w, 1.5);
        assert_eq!(w.accepted, 0);
        assert!(matches!(
            w.phase,
            Phase::Doh {
                cause: DohCause::Dropped(0),
                ..
            }
        ));
        let y = w.events.iter().find_map(|e| match e {
            Event::Crumbled { y, .. } => Some(*y),
            _ => None,
        });
        assert_eq!(y, Some(LANE_Y[1] + 2.0), "smashes on the belt below");
    }

    #[test]
    fn moving_lift_cannot_catch() {
        let mut w = playing(1);
        // Bottom lane, so no lower jar can save it either.
        w.player.lane = 3;
        w.player.pos = 3.0;
        put_cookie(&mut w, 3, CATCH_X - 0.1);
        w.move_to(0);
        run(&mut w, 1.5);
        assert_eq!(w.accepted, 0);
        assert_eq!(w.lives, START_LIVES - 1);
    }

    #[test]
    fn third_doh_expires_the_session() {
        let mut w = playing(7);
        for _ in 0..START_LIVES {
            w.phase = Phase::Play;
            w.player.lane = 0;
            w.player.pos = 0.0;
            put_cookie(&mut w, 3, CATCH_X - 0.1);
            run(&mut w, 1.0);
            run(&mut w, DOH_TIME + 0.1);
        }
        assert_eq!(w.lives, 0);
        assert_eq!(w.phase, Phase::Over);
        assert!(w.events.contains(&Event::GameOver));
    }

    #[test]
    fn spider_bites_whoever_stands_at_its_lane() {
        let mut w = playing(1);
        w.hazards = true;
        w.popup_timer = f32::INFINITY;
        w.player.lane = 1;
        w.player.pos = 1.0;
        w.spider = SpiderState::Down {
            lane: 1,
            y: spider_hang_y(1) - 1.0,
        };
        run(&mut w, 0.2);
        assert!(matches!(
            w.phase,
            Phase::Doh {
                cause: DohCause::Bitten,
                ..
            }
        ));
        assert_eq!(w.lives, START_LIVES - 1);
    }

    #[test]
    fn spider_eats_cookies_at_its_lane_without_penalty() {
        let mut w = playing(1);
        w.hazards = true;
        w.popup_timer = f32::INFINITY;
        w.player.lane = 3;
        w.player.pos = 3.0;
        w.spider = SpiderState::Hang { lane: 0, t: 5.0 };
        put_cookie(&mut w, 0, CATCH_X - 0.5);
        run(&mut w, 0.3);
        assert!(w.events.contains(&Event::SpiderAte { lane: 0 }));
        assert_eq!(w.lives, START_LIVES);
        assert_eq!(w.phase, Phase::Play);
    }

    fn with_cleaner(seed: u64) -> World {
        let mut w = playing(seed);
        w.hazards = true;
        w.popup_timer = f32::INFINITY;
        w.spider = SpiderState::Away { t: 1e9 };
        w
    }

    #[test]
    fn cleaner_bumps_a_bottom_camper() {
        let mut w = with_cleaner(1);
        w.player.lane = 3;
        w.player.pos = 3.0;
        w.cleaner.state = cleaner::CleanerState::Charge;
        w.cleaner.x = cleaner::DOCK_X - 2.0;
        run(&mut w, 0.3);
        assert!(matches!(
            w.phase,
            Phase::Doh {
                cause: DohCause::Bumped,
                ..
            }
        ));
        assert_eq!(w.lives, START_LIVES - 1);
    }

    #[test]
    fn camping_at_the_bottom_summons_the_cleaner() {
        let mut camper = with_cleaner(1);
        camper.player.lane = 3;
        camper.player.pos = 3.0;
        camper.cleaner.state = cleaner::CleanerState::Patrol { t: 9.0 };
        let mut roamer = camper.clone();
        roamer.player.lane = 1;
        roamer.player.pos = 1.0;
        run(&mut camper, 3.5);
        run(&mut roamer, 3.5);
        assert!(camper.events.contains(&Event::CleanerComing));
        assert!(!roamer.events.contains(&Event::CleanerComing));
    }

    /// It used to reset its schedule every level and turn around short of the lift, so in
    /// real play it never seemed to come. It must show up early in an ordinary game.
    #[test]
    fn cleaner_visits_early_in_a_real_game() {
        for seed in 1..=8 {
            let mut w = World::new(seed);
            let mut came = false;
            for _ in 0..(20.0 / DT) as usize {
                // A decent player: always heads for the next cookie to arrive.
                let next = w
                    .cookies
                    .iter()
                    .filter_map(|c| w.arrival_in(c).map(|a| (a, c.lane)))
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                if let Some((_, lane)) = next {
                    w.move_to(lane);
                }
                w.update(DT);
                came |= w.events.contains(&Event::CleanerComing);
                w.events.clear();
            }
            assert!(came, "seed {seed}: no visit in the first 20 s");
        }
    }

    #[test]
    fn docked_cleaner_clears_bottom_cookies_without_penalty() {
        let mut w = with_cleaner(1);
        w.player.lane = 0;
        w.player.pos = 0.0;
        w.cleaner.state = cleaner::CleanerState::Docked { t: 5.0 };
        w.cleaner.x = cleaner::DOCK_X;
        put_cookie(&mut w, 3, CATCH_X - 0.5);
        run(&mut w, 1.0);
        assert!(
            w.events.contains(&Event::CleanerAte),
            "events: {:?}",
            w.events
        );
        assert_eq!(w.lives, START_LIVES);
        assert_eq!(w.phase, Phase::Play);
    }

    #[test]
    fn accepting_a_popup_scores() {
        let mut w = playing(1);
        assert!(!w.accept_all());
        w.popup = Some(Popup { lane: 1, age: 0.0 });
        assert!(w.accept_all());
        assert!(w.popup.is_none());
        assert_eq!(w.score, 50);
    }

    #[test]
    fn meeting_the_quota_clears_the_level() {
        let mut w = playing(1);
        w.accepted = w.tuning.quota - 1;
        w.player.lane = 0;
        w.player.pos = 0.0;
        put_cookie(&mut w, 0, CATCH_X - 0.5);
        run(&mut w, 0.5);
        assert!(matches!(w.phase, Phase::Clear { .. }));
        run(&mut w, CLEAR_TIME + 0.1);
        assert_eq!(w.level, 2);
        assert_eq!(w.accepted, 0);
    }

    #[test]
    fn full_jar_pays_a_bonus() {
        let mut w = playing(1);
        w.player.jar = JAR_CAPACITY - 1;
        w.player.lane = 0;
        w.player.pos = 0.0;
        put_cookie(&mut w, 0, CATCH_X - 0.5);
        run(&mut w, 0.5);
        assert_eq!(w.player.jar, 0);
        assert!(w.events.iter().any(|e| matches!(e, Event::JarFull { .. })));
    }

    /// A perfect bot that always heads for the next cookie to arrive never drops one: the
    /// spawner must never schedule two arrivals the lift can't both make.
    #[test]
    fn scheduler_is_always_fair() {
        for level in 1..=12 {
            for seed in 1..=6u64 {
                let mut w = World::new(seed * 7919 + level as u64);
                w.hazards = false;
                w.start_level(level);
                w.tuning.quota = u32::MAX;
                let mut t = 0.0;
                while t < 90.0 {
                    let next = w
                        .cookies
                        .iter()
                        .filter_map(|c| w.arrival_in(c).map(|a| (a, c.lane)))
                        .min_by(|a, b| a.0.total_cmp(&b.0));
                    if let Some((_, lane)) = next {
                        w.move_to(lane);
                    }
                    w.update(DT);
                    t += DT;
                    assert!(
                        !matches!(w.phase, Phase::Doh { .. }),
                        "level {level} seed {seed}: dropped a cookie at t={t:.2}"
                    );
                }
                assert!(
                    w.accepted > 5,
                    "level {level}: only {} accepted",
                    w.accepted
                );
            }
        }
    }
}
