//! Draws the game into the 320x200 framebuffer.

pub mod art;
pub mod fx;
pub mod scene;
pub mod title;

use art::Art;
use fx::Fx;
use scene::{HUD_H, LANE_GAP, METER, SITES};

use crate::gfx::font::{self, Font};
use crate::gfx::palette::*;
use crate::gfx::{Screen, W};
use crate::logic::layout::*;
use crate::logic::{CookieKind, CookieState, DohCause, JAR_CAPACITY, Phase, SpiderState, World};

pub struct Renderer {
    pub art: Art,
    bg: Screen,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic per-pixel hash for twinkles and blinking LEDs.
fn hash(a: i32, b: i32) -> u32 {
    let mut h = (a as u32).wrapping_mul(0x9e37_79b1) ^ (b as u32).wrapping_mul(0x85eb_ca77);
    h ^= h >> 15;
    h = h.wrapping_mul(0xc2b2_ae3d);
    h ^ (h >> 13)
}

/// Top of the belt the lift is level with, for a fractional lane position.
fn lift_y(pos: f32) -> i32 {
    (LANE_Y[0] + pos * LANE_GAP).round() as i32
}

pub struct GameView<'a> {
    pub world: &'a World,
    pub fx: &'a Fx,
    pub t: f32,
    pub paused: bool,
}

impl Renderer {
    pub fn new() -> Self {
        let art = Art::new();
        let mut bg = Screen::new();
        scene::paint(&mut bg, &art);
        Self { art, bg }
    }

    pub fn draw_game(&self, s: &mut Screen, v: &GameView<'_>) {
        let w = v.world;
        s.copy_from(&self.bg);
        self.ambient(s, w, v.t);
        for (x, y, c) in &v.fx.crumbs {
            s.put(*x, *y, *c);
        }
        for lane in 0..LANES {
            self.belt(s, w, lane);
            self.site(s, v.fx, lane, v.t);
        }
        for c in &w.cookies {
            if matches!(c.state, CookieState::Spat { .. } | CookieState::OnBelt) {
                self.cookie(s, c.kind, c.x, c.y);
            }
        }
        self.lift(s, w, v.fx, v.t);
        for c in &w.cookies {
            if matches!(c.state, CookieState::Falling { .. }) {
                self.cookie(s, c.kind, c.x, c.y);
            }
        }
        self.spider(s, w, v.t);
        self.particles(s, v.fx);
        if let Some(p) = w.popup {
            self.popup(s, p.lane, p.age, v.t);
        }
        self.hud(s, w);
        self.overlay(s, w, v.t);
        if v.paused {
            s.blend_rect(0, HUD_H, W, 200 - HUD_H, NAVY, 0.6);
            dialog(s, 100, 80, 120, 40, "PAUSED");
            font::draw_centered(s, Font::Tiny, 160, 98, "P - RESUME", NAVY);
            font::draw_centered(s, Font::Tiny, 160, 107, "ESC - QUIT TO TITLE", NAVY);
        }
    }

    /// Twinkling stars, blinking LEDs and the cookie meter.
    fn ambient(&self, s: &mut Screen, w: &World, t: f32) {
        let tick = (t * 4.0) as i32;
        for k in 0..12 {
            let h = hash(k, tick / 3);
            if h.is_multiple_of(5) {
                let x = [62, 122, 182][k as usize % 3] + (h >> 8) as i32 % 48;
                let y = HUD_H + 4 + (h >> 16) as i32 % 18;
                if s.get(x, y).2 > 60 {
                    s.put(x, y, WHITE);
                }
            }
        }
        // Server cabinet LEDs.
        for lane in 1..LANES {
            let cy = LANE_Y[lane - 1] as i32 + 9 + 3 + 7 + 3;
            let ch = LANE_Y[lane] as i32 - 1 - (cy - 3) - 1 - 6;
            for (j, x) in (70..236).step_by(42).enumerate() {
                for row in 0..(ch / 3).max(0) {
                    for col in 0..3 {
                        let h = hash(
                            j as i32 * 31 + col + lane as i32 * 7,
                            row + tick / (2 + col),
                        );
                        let c = match h % 7 {
                            0 => GREEN_LT,
                            1 => RED,
                            2 => YELLOW,
                            _ => continue,
                        };
                        s.put(x + 20 + col * 3, cy + row * 3, c);
                    }
                }
            }
        }
        // Cookie meter: how much of the quota has been accepted.
        let (mx, my, mw, mh) = METER;
        let frac = (w.accepted as f32 / w.tuning.quota.max(1) as f32).min(1.0);
        let fill = (frac * mh as f32) as i32;
        if fill > 0 {
            for y in (my + mh - fill)..(my + mh) {
                for x in mx..mx + mw {
                    let c = if hash(x, y).is_multiple_of(9) {
                        CHIP
                    } else if x < mx + 3 {
                        COOKIE_HI
                    } else {
                        COOKIE_LT
                    };
                    s.put(x, y, c);
                }
            }
            let top = my + mh - fill;
            s.hline(mx, mx + mw - 1, top, COOKIE_HI);
        }
        // Glass glare.
        s.vline(mx + mw - 3, my + 2, my + mh - 3, PALE.scale(0.5));
        font::draw(
            s,
            Font::Tiny,
            mx,
            my + mh + 1,
            &format!("{:>3}", w.remaining()),
            PALE,
        );
    }

    fn belt(&self, s: &mut Screen, w: &World, lane: usize) {
        let y = LANE_Y[lane] as i32;
        let (x0, x1) = (BELT_START_X as i32 - 4, BELT_END_X as i32);
        let phase = w.belt_phase[lane];
        let off = phase as i32 % 6;
        s.hline(x0, x1 - 1, y, GREY);
        for x in (x0 + off..x1).step_by(6) {
            s.vline(x, y + 1, y + 4, GREY_LT.scale(0.7));
            s.put(x, y, GREY_LT);
            s.put(x + 1, y + 2, BLACK);
        }
        for cx in [x0 + 2, x1 - 3] {
            s.fill_circle(cx, y + 3, 3, GREY);
            s.circle_outline(cx, y + 3, 3, GREY_LT);
            let a = phase / 3.0;
            s.put(
                cx + (a.cos() * 2.0).round() as i32,
                y + 3 + (a.sin() * 2.0).round() as i32,
                BLACK,
            );
            s.put(cx, y + 3, GREY_DK);
        }
    }

    /// A website window with a face that spits the cookies.
    fn site(&self, s: &mut Screen, fx: &Fx, lane: usize, t: f32) {
        let y = LANE_Y[lane] as i32;
        let (x, top, ww, wh) = (13, y - 27, 28, 24);
        let (name, bar) = SITES[lane];
        // Shadow, frame, title bar.
        s.fill_rect(x + 2, top + 2, ww, wh, BLACK);
        s.bevel(x, top, ww, wh, WHITE, PALE, GREY);
        s.fill_rect(x + 1, top + 1, ww - 2, 6, bar);
        s.hline(x + 1, x + ww - 2, top + 1, bar.lerp(WHITE, 0.4));
        font::draw(s, Font::Tiny, x + 3, top + 2, name, WHITE);

        let spat = fx.spat[lane];
        let blink = (t * 0.6 + lane as f32 * 0.37).fract() < 0.04;
        // Eyes.
        let ey = top + 10;
        for ex in [x + 6, x + 16] {
            if blink {
                s.hline(ex, ex + 2, ey + 1, NAVY);
            } else {
                s.fill_rect(ex, ey, 3, 3, NAVY);
                s.put(ex, ey, WHITE);
            }
        }
        // Mouth: wide open right after a spit, a smug grin otherwise.
        let (mx, my) = (x + 13, top + 15);
        if spat < 0.3 {
            s.fill_rect(mx - 1, my - 1, 9, 6, NAVY);
            s.fill_rect(mx + 1, my + 2, 5, 2, RED);
            s.fill_circle(x + 5, my + 1, 1, PINK);
            s.fill_circle(x + 23, my + 1, 1, PINK);
        } else {
            s.hline(mx + 1, mx + 5, my + 2, NAVY);
            s.put(mx, my + 1, NAVY);
            s.put(mx + 6, my + 1, NAVY);
        }
    }

    fn cookie(&self, s: &mut Screen, kind: CookieKind, x: f32, y: f32) {
        let spr = match kind {
            CookieKind::Plain => &self.art.cookie,
            CookieKind::Golden => &self.art.golden,
        };
        let (x, y) = (x.round() as i32, y.round() as i32);
        if kind == CookieKind::Golden {
            s.blit_outline(spr, x, y, YELLOW_LT);
        }
        s.blit(spr, x, y);
    }

    fn lift(&self, s: &mut Screen, w: &World, fx: &Fx, t: f32) {
        let py = lift_y(w.player.pos);
        let plat = py + 17;
        // Cables and counterweight.
        s.vline(277, HUD_H + 8, plat, GREY);
        s.vline(282, HUD_H + 8, (HUD_H + 8) + 12, GREY);
        let cw =
            HUD_H + 26 + ((LANES - 1) as f32 * LANE_GAP - (py - LANE_Y[0] as i32) as f32) as i32;
        s.vline(292, HUD_H + 8, cw, GREY);
        s.bevel(286, cw, 6, 16, GREY_LT, GREY, GREY_DK);
        s.hline(287, 290, cw + 5, YELLOW.scale(0.7));
        s.hline(287, 290, cw + 10, YELLOW.scale(0.7));

        // Platform.
        s.bevel(248, plat, 32, 4, STEEL_LT, STEEL, SLATE);
        for x in 249..279 {
            if ((x + plat) / 2) % 2 == 0 {
                s.put(x, plat + 3, YELLOW);
            }
        }
        s.fill_rect(279, plat - 1, 3, 6, GREY);

        let (bx, by) = (252, py - 15);
        let a = &self.art;
        let doh = matches!(w.phase, Phase::Doh { .. });
        s.blit(&a.body, bx, by);

        // Face.
        let spider_coming =
            matches!(w.spider, SpiderState::Down { lane, .. } if lane == w.player.nearest());
        if doh {
            s.blit(&a.brows_doh, bx + 3, by + 5);
            s.blit(&a.mouth_doh, bx + 7, by + 11);
        } else if w.player.joy > 0.0 || matches!(w.phase, Phase::Clear { .. }) {
            s.blit(&a.mouth_happy, bx + 7, by + 12);
        } else if spider_coming {
            s.blit(&a.mouth_worry, bx + 7, by + 12);
            if (t * 3.0).fract() < 0.6 {
                s.put(bx + 17, by + 6 + ((t * 3.0).fract() * 6.0) as i32, CYAN_LT);
            }
        } else if (t * 0.35).fract() < 0.03 {
            // Blink: lenses go blank.
            s.fill_rect(bx + 5, by + 8, 3, 1, CYAN_LT);
            s.fill_rect(bx + 12, by + 8, 3, 1, CYAN_LT);
        }

        // The jar (dropped onto the platform during a D'oh).
        let (jx, jy) = if doh { (256, plat - 14) } else { (256, py + 1) };
        s.blit(&a.jar_back, jx, jy);
        const PILE: [(i32, i32); 10] = [
            (1, 9),
            (6, 9),
            (3, 8),
            (1, 6),
            (6, 6),
            (4, 5),
            (2, 3),
            (6, 3),
            (4, 2),
            (4, 0),
        ];
        for &(px, py) in PILE.iter().take(w.player.jar.min(JAR_CAPACITY) as usize) {
            s.blit(&a.mini, jx + px, jy + py);
        }
        for c in &w.cookies {
            if let CookieState::DropIn { .. } = c.state {
                self.cookie(s, c.kind, c.x, c.y);
            }
        }
        s.blit(&a.jar_front, jx, jy);
        if fx.jar_glow > 0.0 && (t * 16.0) as i32 % 2 == 0 {
            s.blit_outline(&a.jar_front, jx, jy, YELLOW);
        }
        if doh {
            // Hands to the forehead.
            s.blit(&a.hand, bx, by + 6);
            s.blit_flipped(&a.hand, bx + 17, by + 6);
            if let Phase::Doh {
                cause: DohCause::Bitten,
                ..
            } = w.phase
            {
                for k in 0..3 {
                    let ang = t * 6.0 + k as f32 * 2.1;
                    s.put(
                        bx + 10 + (ang.cos() * 9.0) as i32,
                        by - 2 + (ang.sin() * 3.0) as i32,
                        YELLOW,
                    );
                }
            }
            self.doh_bubble(s, w, py);
        } else {
            s.blit(&a.hand, bx + 2, by + 21);
            s.blit_flipped(&a.hand, bx + 15, by + 21);
        }
    }

    fn doh_bubble(&self, s: &mut Screen, w: &World, py: i32) {
        let Phase::Doh { t, .. } = w.phase else {
            return;
        };
        let age = crate::logic::DOH_TIME - t;
        if age < 0.04 {
            return;
        }
        let (bw, bh) = (68, 24);
        let bx = 176;
        let by = (py - 44).max(HUD_H + 2);
        // Tail towards the mouth.
        for k in 0..8 {
            s.hline(bx + bw - 14 + k, bx + bw - 8 + k, by + bh - 1 + k, WHITE);
        }
        s.line(bx + bw - 14, by + bh - 1, bx + bw, by + bh + 7, BLACK);
        s.line(bx + bw - 8, by + bh - 1, bx + bw, by + bh + 7, BLACK);
        s.round_box(bx, by, bw, bh, WHITE, BLACK);
        for k in 0..8 {
            s.hline(
                bx + bw - 13 + k,
                bx + bw - 9 + k,
                by + bh - 1 + k - 1,
                WHITE,
            );
        }
        let wobble = ((age * 30.0).sin() * 1.5) as i32;
        font::draw_outlined(
            s,
            Font::Normal,
            bx + 6,
            by + 5 + wobble,
            "D'OH!",
            2,
            &|row| if row < 3 { PINK } else { RED },
            RED_DK,
        );
    }

    fn spider(&self, s: &mut Screen, w: &World, t: f32) {
        let x = SPIDER_X as i32;
        if let SpiderState::Down { lane, .. } = w.spider {
            // Warning sign at the lane it's heading for.
            if (t * 6.0) as i32 % 2 == 0 {
                let y = LANE_Y[lane] as i32 - 24;
                s.fill_rect(241, y, 5, 9, RED);
                s.rect_outline(240, y - 1, 7, 11, YELLOW);
                s.vline(243, y + 1, y + 4, WHITE);
                s.put(243, y + 6, WHITE);
            }
        }
        let Some(y) = w.spider_y() else { return };
        let y = y.round() as i32;
        s.vline(x, CEILING_Y as i32, y + 1, PALE);
        let moving = !matches!(w.spider, SpiderState::Hang { .. });
        let frame = if moving {
            (t * 10.0) as usize % 2
        } else {
            (t * 3.0) as usize % 2
        };
        s.blit(&self.art.spider[frame], x - 7, y);
    }

    fn particles(&self, s: &mut Screen, fx: &Fx) {
        for p in &fx.particles {
            s.put(p.x.round() as i32, p.y.round() as i32, p.color);
        }
        for t in &fx.texts {
            if t.life > 0.25 || (t.life * 20.0) as i32 % 2 == 0 {
                font::draw_shadow(
                    s,
                    Font::Tiny,
                    t.x as i32,
                    t.y as i32,
                    &t.text,
                    t.color,
                    BLACK,
                );
            }
        }
    }

    /// "We value your privacy" - covers two belts until accepted.
    fn popup(&self, s: &mut Screen, lane: usize, age: f32, t: f32) {
        let (x, y) = (74, LANE_Y[lane] as i32 - 22);
        let (pw, ph) = (164, 70);
        let grow = (age / 0.18).min(1.0);
        if grow < 1.0 {
            let (cw, chh) = ((pw as f32 * grow) as i32, (ph as f32 * grow) as i32);
            s.rect_outline(
                x + (pw - cw) / 2,
                y + (ph - chh) / 2,
                cw.max(2),
                chh.max(2),
                WHITE,
            );
            return;
        }
        s.blend_rect(x + 4, y + 4, pw, ph, BLACK, 0.6);
        s.bevel(x, y, pw, ph, WHITE, PALE, GREY);
        s.fill_rect(x + 2, y + 2, pw - 4, 9, BLUE_DK);
        s.hline(x + 2, x + pw - 3, y + 2, BLUE_LT);
        font::draw(s, Font::Tiny, x + 5, y + 4, "COOKIE CONSENT", WHITE);
        s.bevel(x + pw - 11, y + 3, 7, 7, PALE, GREY_LT, GREY);
        font::draw(s, Font::Tiny, x + pw - 9, y + 4, "X", NAVY);
        s.blit(&self.art.cookie, x + 8, y + 16);
        font::draw(s, Font::Normal, x + 22, y + 15, "WE VALUE YOUR", NAVY);
        font::draw(s, Font::Normal, x + 22, y + 24, "PRIVACY.*", NAVY);
        font::draw(s, Font::Tiny, x + 22, y + 34, "*NOT REALLY.", GREY);
        // The one button that matters.
        let lit = (t * 4.0) as i32 % 2 == 0;
        let (bx, by) = (x + 10, y + 44);
        s.bevel(
            bx,
            by,
            84,
            15,
            if lit { GREEN_LT } else { GREEN },
            GREEN,
            GREEN_DK,
        );
        font::draw_centered_shadow(
            s,
            Font::Normal,
            bx + 42,
            by + 4,
            "ACCEPT ALL",
            WHITE,
            GREEN_DK,
        );
        font::draw(s, Font::Tiny, bx + 22, by + 17, "PRESS SPACE", GREY);
        s.bevel(x + 102, by + 2, 52, 11, PALE, GREY_LT, GREY);
        font::draw(s, Font::Tiny, x + 106, by + 5, "MANAGE...", GREY);
    }

    fn hud(&self, s: &mut Screen, w: &World) {
        s.bevel(0, 0, W, HUD_H, WHITE, GREY_LT, GREY);
        for (i, c) in [RED, YELLOW, GREEN].into_iter().enumerate() {
            s.fill_circle(6 + i as i32 * 7, 6, 2, c);
        }
        s.bevel(26, 2, 136, 10, GREY, WHITE, PALE);
        s.blit(&self.art.lock, 29, 4);
        font::draw(
            s,
            Font::Tiny,
            35,
            5,
            &format!("COOKIE-DOH.WEB/LEVEL-{:02}", w.level),
            NAVY,
        );
        font::draw(s, Font::Tiny, 168, 5, "SCORE", GREY);
        font::draw(s, Font::Normal, 189, 4, &format!("{:07}", w.score), NAVY);
        font::draw(s, Font::Tiny, 236, 5, "LV", GREY);
        font::draw(s, Font::Normal, 245, 4, &format!("{:02}", w.level), NAVY);
        for i in 0..crate::logic::START_LIVES {
            let spr = if i < w.lives {
                &self.art.life
            } else {
                &self.art.life_lost
            };
            s.blit(spr, 292 + i as i32 * 8, 5);
        }
    }

    fn overlay(&self, s: &mut Screen, w: &World, t: f32) {
        match w.phase {
            Phase::Intro { .. } => {
                dialog(s, 92, 74, 136, 44, &format!("LEVEL {:02}", w.level));
                font::draw_centered(
                    s,
                    Font::Normal,
                    160,
                    92,
                    &format!("ACCEPT {} COOKIES", w.tuning.quota),
                    NAVY,
                );
                if (t * 3.0) as i32 % 2 == 0 {
                    font::draw_centered(s, Font::Normal, 160, 104, "GET READY!", RED);
                }
            }
            Phase::Clear { .. } => {
                dialog(s, 70, 74, 180, 44, "LEVEL CLEAR");
                font::draw_centered(s, Font::Normal, 160, 92, "ALL COOKIES ACCEPTED!", NAVY);
                let bonus = 250 * w.level + 100 * w.lives;
                font::draw_centered(
                    s,
                    Font::Normal,
                    160,
                    104,
                    &format!("BONUS +{bonus}"),
                    GREEN_DK,
                );
            }
            _ => {}
        }
    }
}

/// A browser-style dialog with a title bar.
pub fn dialog(s: &mut Screen, x: i32, y: i32, w: i32, h: i32, title: &str) {
    s.blend_rect(x + 4, y + 4, w, h, BLACK, 0.6);
    s.bevel(x, y, w, h, WHITE, PALE, GREY);
    s.fill_rect(x + 2, y + 2, w - 4, 11, BLUE_DK);
    s.hline(x + 2, x + w - 3, y + 2, BLUE_LT);
    font::draw_centered_shadow(s, Font::Normal, x + w / 2, y + 4, title, WHITE, NAVY);
}
