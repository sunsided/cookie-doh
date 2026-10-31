//! Cosmetic effects driven by [`crate::logic::Event`]s: crumbs, sparkles, score pop-ups,
//! screen shake and flashes. Purely visual; the rules never read any of it.

use crate::gfx::palette::*;
use crate::logic::layout::*;
use crate::logic::{CookieKind, DohCause, Event, Rng, World};

#[derive(Clone, Debug)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub color: Rgb,
    pub gravity: f32,
    /// Lands and stays on the floor as a crumb.
    pub settles: bool,
}

#[derive(Clone, Debug)]
pub struct FloatText {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub life: f32,
    pub color: Rgb,
}

#[derive(Default)]
pub struct Fx {
    pub particles: Vec<Particle>,
    pub texts: Vec<FloatText>,
    /// Crumbs left on the floor.
    pub crumbs: Vec<(i32, i32, Rgb)>,
    pub shake: f32,
    pub flash: Option<(Rgb, f32)>,
    /// Time since each lane last spat (mouth animation).
    pub spat: [f32; LANES],
    /// Seconds the jar has been glowing after a full-jar bonus.
    pub jar_glow: f32,
    rng: Option<Rng>,
}

impl Fx {
    pub fn new() -> Self {
        Self {
            rng: Some(Rng::new(0x5eed)),
            spat: [9.0; LANES],
            ..Default::default()
        }
    }

    fn rng(&mut self) -> &mut Rng {
        self.rng.get_or_insert_with(|| Rng::new(0x5eed))
    }

    pub fn burst(
        &mut self,
        x: f32,
        y: f32,
        n: usize,
        colors: &[Rgb],
        speed: f32,
        gravity: f32,
        settles: bool,
    ) {
        for _ in 0..n {
            let r = self.rng();
            let a = r.range(0.0, std::f32::consts::TAU);
            let v = r.range(0.3, 1.0) * speed;
            let life = r.range(0.4, 0.9);
            let color = colors[r.below(colors.len())];
            self.particles.push(Particle {
                x,
                y,
                vx: a.cos() * v,
                vy: a.sin() * v - speed * 0.5,
                life,
                color,
                gravity,
                settles,
            });
        }
    }

    pub fn text(&mut self, x: f32, y: f32, text: impl Into<String>, color: Rgb) {
        self.texts.push(FloatText {
            x,
            y,
            text: text.into(),
            life: 1.0,
            color,
        });
    }

    pub fn on_event(&mut self, e: &Event, w: &World) {
        let jar_x = BELT_END_X + 6.0;
        match *e {
            Event::Spit { lane } => self.spat[lane] = 0.0,
            Event::Accepted { lane, kind, points } => {
                let y = LANE_Y[lane];
                let colors: &[Rgb] = match kind {
                    CookieKind::Plain => &[WHITE, COOKIE_HI, YELLOW_LT],
                    CookieKind::Golden => &[YELLOW, YELLOW_LT, WHITE, ORANGE],
                };
                let n = if kind == CookieKind::Golden { 24 } else { 8 };
                self.burst(jar_x, y, n, colors, 50.0, 0.0, false);
                self.text(
                    jar_x - 8.0,
                    y - 22.0,
                    format!("+{points}"),
                    if kind == CookieKind::Golden {
                        YELLOW
                    } else {
                        WHITE
                    },
                );
            }
            Event::JarFull { points } => {
                let y = w.player.pos * 39.0 + LANE_Y[0];
                self.burst(
                    jar_x,
                    y,
                    30,
                    &[YELLOW, GREEN_LT, CYAN_LT, MAGENTA, WHITE],
                    80.0,
                    60.0,
                    false,
                );
                self.text(
                    jar_x - 30.0,
                    y - 32.0,
                    format!("JAR FULL +{points}"),
                    GREEN_LT,
                );
                self.jar_glow = 0.8;
            }
            Event::Crumbled { x, kind } => {
                let colors: &[Rgb] = match kind {
                    CookieKind::Plain => &[COOKIE, COOKIE_LT, BROWN, CHIP],
                    CookieKind::Golden => &[YELLOW, ORANGE, YELLOW_LT],
                };
                self.burst(x, FLOOR_Y - 3.0, 26, colors, 70.0, 300.0, true);
                self.shake = 0.35;
            }
            Event::Doh(cause) => {
                self.flash = Some((RED, 0.45));
                if cause == DohCause::Bitten {
                    self.shake = 0.4;
                    let y = w.player.pos * 39.0 + LANE_Y[0];
                    self.burst(
                        SPIDER_X,
                        y - 6.0,
                        16,
                        &[WHITE, YELLOW, CYAN_LT],
                        70.0,
                        0.0,
                        false,
                    );
                }
            }
            Event::SpiderAte { lane } => {
                let y = LANE_Y[lane];
                self.burst(
                    SPIDER_X - 4.0,
                    y - 4.0,
                    10,
                    &[COOKIE, COOKIE_LT, CHIP],
                    40.0,
                    200.0,
                    true,
                );
                self.text(SPIDER_X - 22.0, y - 26.0, "NOM!", MAGENTA);
            }
            Event::PopupAccepted { points } => {
                self.text(140.0, 96.0, format!("ACCEPTED +{points}"), GREEN_LT);
                self.flash = Some((WHITE, 0.25));
            }
            Event::LevelClear { .. } => {
                self.flash = Some((YELLOW_LT, 0.4));
                for y in LANE_Y {
                    for k in 0..6 {
                        let x = 60.0 + k as f32 * 34.0;
                        self.burst(
                            x,
                            y - 4.0,
                            4,
                            &[YELLOW, WHITE, COOKIE_HI],
                            50.0,
                            30.0,
                            false,
                        );
                    }
                }
            }
            Event::LevelStart { .. } => self.crumbs.clear(),
            _ => {}
        }
    }

    pub fn update(&mut self, dt: f32) {
        for s in &mut self.spat {
            *s += dt;
        }
        self.shake = (self.shake - dt).max(0.0);
        self.jar_glow = (self.jar_glow - dt).max(0.0);
        if let Some((c, a)) = self.flash {
            let a = a - dt * 1.2;
            self.flash = (a > 0.0).then_some((c, a));
        }
        let mut crumbs = Vec::new();
        self.particles.retain_mut(|p| {
            p.vy += p.gravity * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life -= dt;
            if p.settles && p.y >= FLOOR_Y + 1.0 {
                crumbs.push((
                    p.x as i32,
                    FLOOR_Y as i32 + 1 + (p.x as i32 % 3).abs(),
                    p.color,
                ));
                return false;
            }
            p.life > 0.0 || (p.settles && p.y < FLOOR_Y)
        });
        self.crumbs.extend(crumbs);
        if self.crumbs.len() > 400 {
            let excess = self.crumbs.len() - 400;
            self.crumbs.drain(..excess);
        }
        self.texts.retain_mut(|t| {
            t.y -= 18.0 * dt;
            t.life -= dt * 0.8;
            t.life > 0.0
        });
    }

    /// Integer pixel offset for screen shake.
    pub fn shake_offset(&self, t: f32) -> (i32, i32) {
        if self.shake <= 0.0 {
            return (0, 0);
        }
        let m = (self.shake * 8.0).ceil();
        (
            ((t * 91.0).sin() * m) as i32,
            ((t * 77.0).cos() * m * 0.6) as i32,
        )
    }
}
