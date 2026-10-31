//! The title screen and the game-over card.
//!
//! The title is a composed tableau rather than a demo effect: the logo hangs as a lit factory
//! sign, a single belt keeps feeding the hero's jar underneath, and the whole thing is framed
//! by a website-style cookie banner docked at the bottom (whose REJECT button runs away).

use super::{Renderer, belt_at, hash, scene, site_at};
use crate::gfx::font::{self, Font};
use crate::gfx::palette::*;
use crate::gfx::{Mask, Screen, W};
use crate::logic::layout::{BELT_END_X, BELT_START_X, CATCH_X, COOKIE_SIZE};

const REJECT_QUIPS: [&str; 5] = [
    "NICE TRY.",
    "ARE YOU SURE? THEY'RE FRESH.",
    "REJECT? IN THIS ECONOMY?",
    "THAT BUTTON IS DECORATIVE.",
    "FINE. IT'S GONE NOW.",
];

/// The sign holding the logo: x, y, w, h.
const SIGN: (i32, i32, i32, i32) = (36, 12, 248, 92);
/// Top of the demo belt under the sign.
const BELT_Y: i32 = 136;
/// Top of the cookie banner docked at the bottom.
const STRIP_Y: i32 = 156;
const BELT_SPEED: f32 = 36.0;
const SPIT_TIME: f32 = 0.35;
const DROP_TIME: f32 = 0.14;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleAction {
    None,
    Start,
    Nope,
    Moved,
}

/// A cookie on the title screen's demo belt.
#[derive(Clone, Copy, Debug)]
struct DemoCookie {
    x: f32,
    y: f32,
    age: f32,
    golden: bool,
    dropping: bool,
}

/// The little loop under the logo: one site, one belt, one jar that always catches.
#[derive(Clone, Debug, Default)]
struct Demo {
    cookies: Vec<DemoCookie>,
    spawn: f32,
    spat: f32,
    phase: f32,
    jar: u32,
    joy: f32,
    glow: f32,
    count: u32,
}

impl Demo {
    fn update(&mut self, dt: f32) {
        self.phase += BELT_SPEED * dt;
        self.spat += dt;
        self.joy = (self.joy - dt).max(0.0);
        self.glow = (self.glow - dt).max(0.0);
        self.spawn -= dt;
        if self.spawn <= 0.0 {
            const GAPS: [f32; 5] = [1.6, 2.3, 1.3, 2.8, 1.9];
            self.spawn = GAPS[self.count as usize % GAPS.len()];
            self.count += 1;
            self.spat = 0.0;
            self.cookies.push(DemoCookie {
                x: 26.0,
                y: (BELT_Y - 14) as f32,
                age: 0.0,
                golden: self.count.is_multiple_of(5),
                dropping: false,
            });
        }
        let belt_y = (BELT_Y as f32) - COOKIE_SIZE;
        let mut caught = 0;
        self.cookies.retain_mut(|c| {
            c.age += dt;
            if c.age < SPIT_TIME {
                let k = c.age / SPIT_TIME;
                c.x = 26.0 + (BELT_START_X - 26.0) * k;
                let y0 = (BELT_Y - 14) as f32;
                c.y = y0 + (belt_y - y0) * k - 10.0 * (k * std::f32::consts::PI).sin();
            } else if !c.dropping {
                c.y = belt_y;
                c.x += BELT_SPEED * dt;
                if c.x >= CATCH_X {
                    c.x = CATCH_X;
                    c.dropping = true;
                    c.age = 0.0;
                }
            } else {
                let k = (c.age / DROP_TIME).min(1.0);
                c.x = CATCH_X + (BELT_END_X + 2.0 - CATCH_X) * k;
                c.y = belt_y + (BELT_Y as f32 + 2.0 - belt_y) * k * k;
                if k >= 1.0 {
                    caught += 1;
                    return false;
                }
            }
            true
        });
        for _ in 0..caught {
            self.joy = 0.5;
            self.jar += 1;
            if self.jar >= crate::logic::JAR_CAPACITY {
                self.jar = 0;
                self.glow = 0.8;
            }
        }
    }
}

/// Title screen state. Pure UI logic, no engine types.
pub struct Title {
    pub t: f32,
    /// 0 = ACCEPT ALL, 1 = REJECT.
    pub sel: usize,
    pub tries: usize,
    pub quip_t: f32,
    /// Reject button rectangle; it moves every time you press it.
    pub reject: (i32, i32, i32, i32),
    demo: Demo,
}

pub const ACCEPT_BTN: (i32, i32, i32, i32) = (20, 176, 86, 15);
const REJECT_BTN: (i32, i32, i32, i32) = (114, 176, 66, 15);

impl Default for Title {
    fn default() -> Self {
        Self::new()
    }
}

impl Title {
    pub fn new() -> Self {
        Self {
            t: 0.0,
            sel: 0,
            tries: 0,
            quip_t: 0.0,
            reject: REJECT_BTN,
            demo: Demo {
                spawn: 0.4,
                spat: 9.0,
                jar: 3,
                ..Demo::default()
            },
        }
    }

    pub fn reject_alive(&self) -> bool {
        self.tries < REJECT_QUIPS.len()
    }

    pub fn update(&mut self, dt: f32) {
        self.t += dt;
        self.quip_t = (self.quip_t - dt).max(0.0);
        self.demo.update(dt);
    }

    pub fn select(&mut self, sel: usize) -> TitleAction {
        let sel = if self.reject_alive() { sel.min(1) } else { 0 };
        if sel == self.sel {
            return TitleAction::None;
        }
        self.sel = sel;
        TitleAction::Moved
    }

    pub fn confirm(&mut self) -> TitleAction {
        if self.sel == 0 {
            return TitleAction::Start;
        }
        self.tries += 1;
        self.quip_t = 2.5;
        // The button flees and shrinks, but stays in the banner.
        let (_, _, w, h) = self.reject;
        let spots = [(140, 178), (118, 176), (150, 179), (126, 177)];
        let (nx, ny) = spots[self.tries % spots.len()];
        let nw = (w - 12).max(28);
        self.reject = (nx.min(188 - nw), ny, nw, (h - 2).max(10));
        if !self.reject_alive() {
            self.sel = 0;
        }
        TitleAction::Nope
    }

    pub fn quip(&self) -> Option<&'static str> {
        (self.quip_t > 0.0 && self.tries > 0)
            .then(|| REJECT_QUIPS[(self.tries - 1).min(REJECT_QUIPS.len() - 1)])
    }

    /// Which button is at framebuffer pixel `(x, y)`.
    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        let inside = |(bx, by, bw, bh): (i32, i32, i32, i32)| {
            x >= bx && x < bx + bw && y >= by && y < by + bh
        };
        if inside(ACCEPT_BTN) {
            Some(0)
        } else if self.reject_alive() && inside(self.reject) {
            Some(1)
        } else {
            None
        }
    }

    /// How far the spider hangs below the sign. Mostly it lurks; every ten seconds it drops
    /// down for a look at the hero.
    fn spider_drop(&self) -> f32 {
        let u = self.t % 10.0;
        let ease = |k: f32| k * k * (3.0 - 2.0 * k);
        let low = 26.0;
        if u < 6.5 {
            3.0 + (self.t * 2.0).sin().round()
        } else if u < 7.6 {
            3.0 + (low - 3.0) * ease((u - 6.5) / 1.1)
        } else if u < 8.6 {
            low
        } else {
            low - (low - 3.0) * ease((u - 8.6) / 1.4)
        }
    }
}

/// Colors for a shaded logo word.
struct LogoStyle {
    top: Rgb,
    mid: Rgb,
    bottom: Rgb,
    highlight: Rgb,
    shade: Rgb,
    outline: Rgb,
    chips: bool,
}

/// A word blown up 4x with Scale2x twice, so the 5x7 glyphs get smooth diagonals.
fn logo_mask(text: &str) -> Mask {
    Font::Normal.mask(text).scale2x().scale2x()
}

/// Paint a mask as a chunky arcade logo: drop shadow, two-tone outline, vertical gradient,
/// bevel light and shade, corner glints, and optionally chocolate chips.
fn paint_logo(s: &mut Screen, m: &Mask, x: i32, y: i32, st: &LogoStyle) {
    let inside = |px: i32, py: i32| m.get(px, py);
    for py in 0..m.h {
        for px in 0..m.w {
            if inside(px, py) {
                s.blend(x + px + 3, y + py + 4, BLACK, 0.55);
            }
        }
    }
    for (r2, c) in [(5, BLACK), (2, st.outline)] {
        for py in 0..m.h {
            for px in 0..m.w {
                if !inside(px, py) {
                    continue;
                }
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        if dx * dx + dy * dy <= r2 && !inside(px + dx, py + dy) {
                            s.put(x + px + dx, y + py + dy, c);
                        }
                    }
                }
            }
        }
    }
    for py in 0..m.h {
        let k = py as f32 / (m.h - 1) as f32;
        let base = if k < 0.5 {
            st.top.lerp(st.mid, k * 2.0)
        } else {
            st.mid.lerp(st.bottom, (k - 0.5) * 2.0)
        };
        for px in 0..m.w {
            if !inside(px, py) {
                continue;
            }
            let mut c = base;
            if !inside(px - 1, py - 1) {
                c = st.highlight;
            } else if !inside(px + 1, py + 1) || !inside(px + 2, py + 2) {
                c = st.shade;
            }
            if !inside(px, py - 1) && !inside(px - 1, py) {
                c = WHITE;
            }
            if st.chips && c == base {
                let solid = (-2..=2).all(|d| inside(px + d, py) && inside(px, py + d));
                let cell = hash(px / 3 + x, py / 3 + y);
                if solid && cell.is_multiple_of(7) && (px % 3 != 2) && (py % 3 != 2) {
                    c = if px % 3 == 0 && py % 3 == 0 {
                        BROWN
                    } else {
                        CHIP
                    };
                }
            }
            s.put(x + px, y + py, c);
        }
    }
}

impl Renderer {
    pub fn draw_title(&self, s: &mut Screen, ti: &Title, hiscore: u32) {
        let t = ti.t;
        s.clear(NIGHT);
        scene::wall(s);
        scene::web(s, 0, 9, 1, 30.0);
        scene::web(s, W - 1, 9, -1, 30.0);
        self.girder(s);
        self.sign(s, t);
        self.demo_scene(s, ti, hiscore);
        self.banner(s, ti);
    }

    /// The steel beam everything hangs from.
    fn girder(&self, s: &mut Screen) {
        s.bevel(0, 0, W, 9, STEEL_LT, STEEL, SLATE);
        s.hline(0, W - 1, 3, SLATE);
        s.hline(0, W - 1, 4, STEEL_LT.scale(0.8));
        for x in (5..W).step_by(12) {
            s.put(x, 1, PALE);
            s.put(x, 7, SLATE.scale(0.6));
        }
    }

    /// The logo as a lit sign: chains, a riveted frame with chasing bulbs, a backlit panel.
    fn sign(&self, s: &mut Screen, t: f32) {
        let (x, y, w, h) = SIGN;
        // Chains.
        for cx in [x + 34, x + w - 35] {
            for cy in 9..y {
                s.put(cx + (cy % 2), cy, GREY_LT);
                s.put(cx - 1 + (cy % 2) * 2, cy, GREY);
            }
        }
        s.fill_rect(x + 4, y + 4, w, h, BLACK);
        s.bevel(x, y, w, h, STEEL_LT, STEEL, NAVY);
        s.rect_outline(x + 5, y + 5, w - 10, h - 10, NAVY);
        // Backlit panel with a soft glow behind the letters.
        let (ix, iy, iw, ih) = (x + 6, y + 6, w - 12, h - 12);
        s.gradient_rect(ix, iy, iw, ih, PURPLE_DK, NAVY, 5);
        let (gx, gy) = (160.0, (iy + 34) as f32);
        for py in iy..iy + ih {
            for px in ix..ix + iw {
                let d = ((px as f32 - gx) / 112.0).powi(2) + ((py as f32 - gy) / 36.0).powi(2);
                if d < 1.0 {
                    let a = (1.0 - d) * 0.35;
                    // Banded, so it reads as palette steps, not a smooth airbrush.
                    let a = (a * 6.0).floor() / 6.0;
                    s.blend(px, py, PURPLE, a);
                }
            }
        }
        // Chasing bulbs around the frame.
        let mut bulbs = Vec::new();
        for bx in (x + 6..x + w - 4).step_by(8) {
            bulbs.push((bx, y + 2));
        }
        for by in (y + 10..y + h - 4).step_by(8) {
            bulbs.push((x + w - 3, by));
        }
        for bx in (x + 6..x + w - 4).step_by(8).rev() {
            bulbs.push((bx, y + h - 3));
        }
        for by in (y + 10..y + h - 4).step_by(8).rev() {
            bulbs.push((x + 2, by));
        }
        let step = (t * 8.0) as usize;
        for (i, &(bx, by)) in bulbs.iter().enumerate() {
            if (i + step).is_multiple_of(3) {
                for (dx, dy) in [
                    (-1, 0),
                    (2, 0),
                    (0, -1),
                    (0, 2),
                    (1, -1),
                    (1, 2),
                    (-1, 1),
                    (2, 1),
                ] {
                    s.blend(bx + dx, by + dy, YELLOW, 0.45);
                }
                s.fill_rect(bx, by, 2, 2, YELLOW_LT);
                s.put(bx, by, WHITE);
            } else {
                s.fill_rect(bx, by, 2, 2, ORANGE.scale(0.45));
            }
        }
        // Bolts.
        for (bx, by) in [
            (x + 3, y + 3),
            (x + w - 4, y + 3),
            (x + 3, y + h - 4),
            (x + w - 4, y + h - 4),
        ] {
            s.put(bx, by, PALE);
        }

        // The words.
        let cookie = logo_mask("COOKIE");
        let cookie_style = LogoStyle {
            top: COOKIE_HI,
            mid: COOKIE_LT,
            bottom: COOKIE,
            highlight: YELLOW_LT,
            shade: BROWN,
            outline: BROWN_DK,
            chips: true,
        };
        // Centered together with the bitten cookie that follows it.
        let cx = 160 - (cookie.w + 36) / 2;
        paint_logo(s, &cookie, cx, y + 9, &cookie_style);

        // A big cookie with a bite out of it, hanging off the end of the word.
        let mut big = Mask::new(26, 26);
        for py in 0..26 {
            for px in 0..26 {
                let (dx, dy) = (px as f32 - 12.5, py as f32 - 12.5);
                big.set(px, py, dx * dx + dy * dy <= 12.5 * 12.5);
            }
        }
        for (bx, by, r) in [(24, 4, 5), (27, 10, 5), (20, -1, 4)] {
            big.carve_circle(bx, by, r);
        }
        let (kx, ky) = (cx + cookie.w + 10, y + 13);
        paint_logo(
            s,
            &big,
            kx,
            ky,
            &LogoStyle {
                chips: false,
                ..cookie_style
            },
        );
        for (px, py, c) in [
            (6, 7, CHIP),
            (14, 5, CHIP),
            (9, 15, CHIP),
            (17, 13, CHIP),
            (5, 18, CHIP),
            (13, 20, CHIP),
            (19, 19, CHIP),
        ] {
            s.fill_rect(kx + px, ky + py, 2, 2, c);
            s.put(kx + px, ky + py, BROWN);
        }
        // Crumbs falling from the bite.
        for (px, py, c) in [
            (26, 15, COOKIE_LT),
            (24, 21, COOKIE),
            (28, 27, BROWN),
            (25, 31, COOKIE_LT),
        ] {
            s.put(kx + px, ky + py, c);
        }

        let doh = logo_mask("D'OH!");
        let doh_style = LogoStyle {
            top: YELLOW_LT,
            mid: YELLOW,
            bottom: ORANGE,
            highlight: WHITE,
            shade: RED,
            outline: RED_DK,
            chips: false,
        };
        paint_logo(s, &doh, 160 - doh.w / 2, y + 41, &doh_style);

        // Subtitle ribbon.
        let text = "ACCEPT ALL COOKIES";
        let tw = Font::Normal.measure(text, 1);
        let (rx, ry, rw, rh) = (160 - tw / 2 - 6, y + 73, tw + 12, 11);
        for (tx, dir) in [(rx - 9, 1), (rx + rw - 1, -1)] {
            // Tails, folded behind, with a notch.
            s.fill_rect(tx, ry + 3, 10, rh, RED_DK);
            let nx = if dir == 1 { tx } else { tx + 9 };
            for k in 0..3 {
                s.vline(
                    nx + dir * k,
                    ry + 3 + 3 + k - 2,
                    ry + 3 + rh - 4 - k + 2,
                    NAVY.lerp(PURPLE_DK, 0.5),
                );
            }
        }
        s.fill_rect(rx, ry, rw, rh, RED);
        s.hline(rx, rx + rw - 1, ry, PINK);
        s.hline(rx, rx + rw - 1, ry + rh - 1, RED_DK);
        font::draw_shadow(s, Font::Normal, 160 - tw / 2, ry + 2, text, WHITE, RED_DK);
    }

    /// The hero on duty under the sign, with a hi-score display and the lurking spider.
    fn demo_scene(&self, s: &mut Screen, ti: &Title, hiscore: u32) {
        let d = &ti.demo;
        let t = ti.t;
        let a = &self.art;

        // Hi-score LCD on the wall.
        let (lx, ly) = (124, 110);
        s.bevel(lx, ly, 72, 13, GREY_LT, GREY, GREY_DK);
        s.fill_rect(lx + 2, ly + 2, 68, 9, GREEN_DK.scale(0.6));
        font::draw(s, Font::Tiny, lx + 5, ly + 4, "HI", GREEN);
        let digits = format!("{hiscore:07}");
        font::draw(s, Font::Normal, lx + 21, ly + 3, "8888888", GREEN_DK);
        font::draw(s, Font::Normal, lx + 21, ly + 3, &digits, GREEN_LT);

        // Lift rails behind the hero.
        for rx in [281, 293] {
            s.fill_rect(rx, SIGN.1 + SIGN.3, 3, STRIP_Y - SIGN.1 - SIGN.3, SLATE);
            s.vline(rx, SIGN.1 + SIGN.3, STRIP_Y, STEEL_LT);
        }

        scene::belt_frame_at(s, BELT_Y, false);
        belt_at(s, BELT_Y, d.phase);
        s.line(30, BELT_Y - 4, 38, BELT_Y, STEEL);
        s.line(30, BELT_Y - 3, 38, BELT_Y + 1, SLATE);
        let blink = (t * 0.6).fract() < 0.04;
        site_at(s, BELT_Y, ("ADS.IO", RED), d.spat, blink);

        for c in d.cookies.iter().filter(|c| !c.dropping) {
            let spr = if c.golden { &a.golden } else { &a.cookie };
            s.blit(spr, c.x.round() as i32, c.y.round() as i32);
        }

        // Platform and hero.
        let (bx, by) = (252, BELT_Y - 15);
        s.bevel(248, BELT_Y + 17, 32, 4, STEEL_LT, STEEL, SLATE);
        s.blit(&a.body, bx, by);
        let drop = ti.spider_drop();
        if drop > 12.0 {
            s.blit(&a.mouth_worry, bx + 7, by + 12);
            if (t * 3.0).fract() < 0.6 {
                s.put(bx + 17, by + 6 + ((t * 3.0).fract() * 6.0) as i32, CYAN_LT);
            }
        } else if d.joy > 0.0 {
            s.blit(&a.mouth_happy, bx + 7, by + 12);
        } else if (t * 0.35).fract() < 0.03 {
            s.fill_rect(bx + 5, by + 8, 3, 1, CYAN_LT);
            s.fill_rect(bx + 12, by + 8, 3, 1, CYAN_LT);
        }
        let (jx, jy) = (256, BELT_Y + 1);
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
        for &(px, py) in PILE.iter().take(d.jar as usize) {
            s.blit(&a.mini, jx + px, jy + py);
        }
        for c in d.cookies.iter().filter(|c| c.dropping) {
            let spr = if c.golden { &a.golden } else { &a.cookie };
            s.blit(spr, c.x.round() as i32, c.y.round() as i32);
        }
        s.blit(&a.jar_front, jx, jy);
        if d.glow > 0.0 && (t * 16.0) as i32 % 2 == 0 {
            s.blit_outline(&a.jar_front, jx, jy, YELLOW);
        }
        s.blit(&a.hand, bx + 2, by + 21);
        s.blit_flipped(&a.hand, bx + 15, by + 21);

        // The Web Crawler, hanging off the sign.
        let (sx, top) = (262, SIGN.1 + SIGN.3);
        let sy = top + drop.round() as i32;
        s.vline(sx, top, sy + 1, PALE);
        let frame = if drop > 4.5 {
            (t * 10.0) as usize % 2
        } else {
            (t * 1.5) as usize % 2
        };
        s.blit(&a.spider[frame], sx - 7, sy);
    }

    /// The cookie banner, docked at the bottom like on every website.
    fn banner(&self, s: &mut Screen, ti: &Title) {
        let y = STRIP_Y;
        s.blend_rect(0, y - 2, W, 2, BLACK, 0.5);
        s.fill_rect(0, y, W, 200 - y, PALE);
        s.hline(0, W - 1, y, WHITE);
        s.hline(0, W - 1, y + 1, GREY_LT);
        s.blit(&self.art.cookie, 8, y + 4);
        font::draw(s, Font::Normal, 20, y + 4, "THIS GAME USES COOKIES.", NAVY);
        font::draw(s, Font::Tiny, 164, y + 5, "(LOTS OF THEM.)", GREY);
        font::draw(
            s,
            Font::Tiny,
            20,
            y + 12,
            "BY PLAYING YOU AGREE TO ACCEPT ALL OF THEM. EVERY. SINGLE. ONE.",
            GREY_DK,
        );

        let (ax, ay, aw, ah) = ACCEPT_BTN;
        s.bevel(ax, ay, aw, ah, GREEN_LT, GREEN, GREEN_DK);
        font::draw_centered_shadow(
            s,
            Font::Normal,
            ax + aw / 2,
            ay + 4,
            "ACCEPT ALL",
            WHITE,
            GREEN_DK,
        );
        if ti.reject_alive() {
            let (rx, ry, rw, rh) = ti.reject;
            s.bevel(rx, ry, rw, rh, WHITE, GREY_LT, GREY);
            let (f, label) = if rw >= 46 {
                (Font::Normal, "REJECT")
            } else {
                (Font::Tiny, "REJECT")
            };
            font::draw_centered(s, f, rx + rw / 2, ry + (rh - f.height()) / 2, label, GREY);
        } else {
            let (rx, ry, rw, rh) = REJECT_BTN;
            s.bevel(rx, ry, rw, rh, GREEN_LT, GREEN, GREEN_DK);
            font::draw_centered_shadow(
                s,
                Font::Normal,
                rx + rw / 2,
                ry + 4,
                "ALSO OK",
                WHITE,
                GREEN_DK,
            );
        }
        // Focus ring, pulsing rather than blinking off.
        let (bx, by, bw, bh) = if ti.sel == 0 { ACCEPT_BTN } else { ti.reject };
        let ring = if (ti.t * 4.0) as i32 % 2 == 0 {
            YELLOW
        } else {
            ORANGE
        };
        s.rect_outline(bx - 2, by - 2, bw + 4, bh + 4, ring);

        let hx = 198;
        match ti.quip() {
            Some(q) => {
                font::draw(s, Font::Tiny, hx, y + 24, q, RED);
            }
            None => {
                font::draw(s, Font::Tiny, hx, y + 21, "LEFT/RIGHT + SPACE", GREY);
                font::draw(s, Font::Tiny, hx, y + 28, "OR CLICK A BUTTON", GREY);
            }
        }
        font::draw(s, Font::Tiny, 20, y + 38, "(C) 2026 MARKUS MAYER", GREY);
        font::draw(s, Font::Tiny, hx, y + 38, "M MUTE  C CRT  ESC QUIT", GREY);
    }

    /// The game-over card, over the frozen last frame.
    pub fn draw_game_over(&self, s: &mut Screen, score: u32, level: u32, record: bool, t: f32) {
        s.blend_rect(0, 14, W, 186, NAVY, 0.65);
        super::dialog(s, 50, 52, 220, 96, "SESSION EXPIRED");
        font::draw_centered(s, Font::Normal, 160, 70, "YOUR COOKIES HAVE BEEN", NAVY);
        font::draw_centered(s, Font::Normal, 160, 79, "CLEARED.", NAVY);
        font::draw_centered(
            s,
            Font::Normal,
            160,
            94,
            &format!("SCORE {score:07}"),
            BLUE_DK,
        );
        font::draw_centered(
            s,
            Font::Normal,
            160,
            104,
            &format!("REACHED LEVEL {level:02}"),
            BLUE_DK,
        );
        if record && (t * 4.0) as i32 % 2 == 0 {
            font::draw_centered(s, Font::Normal, 160, 116, "NEW HI-SCORE!", RED);
        }
        if t > 1.5 {
            font::draw_centered(s, Font::Tiny, 160, 134, "PRESS SPACE TO RE-CONSENT", GREY);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_runs_out_and_accept_starts() {
        let mut t = Title::new();
        assert_eq!(t.confirm(), TitleAction::Start);
        for _ in 0..REJECT_QUIPS.len() {
            t.select(1);
            assert_eq!(t.sel, 1);
            assert_eq!(t.confirm(), TitleAction::Nope);
        }
        assert!(!t.reject_alive());
        t.select(1);
        assert_eq!(t.sel, 0, "can't select a button that is gone");
        assert_eq!(t.confirm(), TitleAction::Start);
    }
}
