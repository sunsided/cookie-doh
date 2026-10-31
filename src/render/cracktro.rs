//! The optional cracktro: "COOKIES & CRACKERS proudly present...". Copper bars, a parallax
//! starfield, a wobbling logo, typed info pages, orbiting cookie bobs, and a rainbow sine
//! scroller with a water reflection. All the nonsense, none of the annoyance.

use super::Renderer;
use super::logo::{LogoStyle, logo_mask, paint_logo};
use crate::gfx::font::{self, Font};
use crate::gfx::palette::*;
use crate::gfx::{H, Mask, Screen, Sprite, W};

const PAGES: [[&str; 3]; 4] = [
    ["COOKIES & CRACKERS", "PROUDLY PRESENT", "COOKIE D'OH!"],
    ["CRACKED, TRAINED", "AND BAKED BY", "MARKUS MAYER"],
    [
        "+1 TRAINER:",
        "INFINITE COOKIES",
        "(YOU STILL HAVE TO ACCEPT THEM)",
    ],
    ["RELEASED ON THE", "WORLD WIDE WEB", "IN 2026"],
];
const PAGE_TIME: f32 = 5.0;
const TYPE_SPEED: f32 = 28.0;

const SCROLL_TEXT: &str = "COOKIES & CRACKERS PROUDLY PRESENT ANOTHER FINE RELEASE...   \
    COOKIE D'OH! - ACCEPT ALL COOKIES   ...   CRACKED, TRAINED AND BAKED BY MARKUS MAYER   ...   \
    GREETINGS FLY OUT TO: THE WEB CRAWLER - THE CACHE CLEANER - GRANDMA'S OVEN - \
    EVERY COOKIE BANNER EVER MADE - THE NIGHT SHIFT AT THE INTERNET - AND YOU, FOR READING \
    THIS FAR   ...   NO COOKIES WERE REJECTED DURING THE MAKING OF THIS INTRO   ...   \
    PRESS SPACE TO ACCEPT ALL COOKIES   ...   WRAP!            ";
const SCROLL_SPEED: f32 = 58.0;
/// Waterline of the scroller's reflection.
const WATER_Y: i32 = 178;
const KEY: Rgb = Rgb(255, 0, 254);

/// Cracktro state: just a clock.
#[derive(Default)]
pub struct Cracktro {
    pub t: f32,
}

impl Cracktro {
    pub fn update(&mut self, dt: f32) {
        self.t += dt;
    }
}

/// Pre-rendered pieces: the group logo and the smoothed scroller glyphs.
pub struct CracktroArt {
    logo: Sprite,
    glyphs: Vec<(char, Mask)>,
}

impl CracktroArt {
    pub fn new() -> Self {
        let chrome = LogoStyle {
            top: WHITE,
            mid: CYAN_LT,
            bottom: PURPLE,
            highlight: WHITE,
            shade: BLUE_DK,
            outline: NAVY,
            chips: false,
        };
        let mut s = Screen::new();
        s.clear(KEY);
        let (l1, l2) = (logo_mask("COOKIES &"), logo_mask("CRACKERS"));
        let w = l1.w.max(l2.w) + 8;
        paint_logo(&mut s, &l1, 4 + (w - 8 - l1.w) / 2, 3, &chrome);
        paint_logo(&mut s, &l2, 4 + (w - 8 - l2.w) / 2, 3 + l1.h + 4, &chrome);
        let h = l1.h + l2.h + 12;
        let mut px = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let c = s.get(x, y);
                px.push((c != KEY).then_some(c));
            }
        }
        let mut glyphs = Vec::new();
        for ch in SCROLL_TEXT.chars() {
            if !glyphs.iter().any(|(c, _)| *c == ch) {
                glyphs.push((ch, Font::Normal.mask(&ch.to_string()).scale2x()));
            }
        }
        Self {
            logo: Sprite { w, h, px },
            glyphs,
        }
    }
}

impl Default for CracktroArt {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    /// `sound_on`: audio has started (on the web it waits for the first key press).
    pub fn draw_cracktro(&self, s: &mut Screen, c: &Cracktro, sound_on: bool) {
        let t = c.t;
        s.clear(BLACK);
        starfield(s, t);
        copper(s, t);
        self.wobbly_logo(s, t);
        self.bobs(s, t);
        info_page(s, t);
        self.scroller(s, t);
        water(s, t);
        let hint = if sound_on {
            "SPACE: ON WITH THE GAME"
        } else {
            "PRESS ANY KEY FOR SOUND"
        };
        if (t * 2.0) as i32 % 2 == 0 {
            font::draw_centered(s, Font::Tiny, W / 2, 137, hint, GREY_LT);
        }
    }

    fn wobbly_logo(&self, s: &mut Screen, t: f32) {
        let logo = &self.crack.logo;
        let x0 = W / 2 - logo.w / 2;
        let y0 = 6;
        for y in 0..logo.h {
            let off = ((y as f32 * 0.13 + t * 2.6).sin() * 5.0) as i32;
            for x in 0..logo.w {
                if let Some(c) = logo.px[(y * logo.w + x) as usize] {
                    s.put(x0 + x + off, y0 + y, c);
                }
            }
        }
    }

    /// Cookies orbiting the info text on a Lissajous curve.
    fn bobs(&self, s: &mut Screen, t: f32) {
        for i in 0..10 {
            let p = i as f32 * 0.628;
            let x = 156.0 + 138.0 * (t * 0.9 + p).sin();
            let y = 108.0 + 22.0 * (t * 1.7 + p * 2.0).sin();
            let spr = if i % 3 == 0 {
                &self.art.golden
            } else {
                &self.art.cookie
            };
            s.blit(spr, x as i32, y as i32);
        }
    }

    fn scroller(&self, s: &mut Screen, t: f32) {
        let adv = 12;
        let len = SCROLL_TEXT.chars().count() as i32 * adv;
        let start = W - ((t * SCROLL_SPEED) as i32 % (len + W));
        for (i, ch) in SCROLL_TEXT.chars().enumerate() {
            let x = start + i as i32 * adv;
            if x < -adv || x > W {
                continue;
            }
            let Some((_, m)) = self.crack.glyphs.iter().find(|(c, _)| *c == ch) else {
                continue;
            };
            let y = 152 + ((x as f32 * 0.035 + t * 3.0).sin() * 9.0) as i32;
            let hue = (i as f32 * 0.045 + t * 0.25).fract();
            let base = hsv(hue, 0.65, 1.0);
            for gy in 0..m.h {
                for gx in 0..m.w {
                    if m.get(gx, gy) {
                        s.put(x + gx + 1, y + gy + 1, BLACK);
                    }
                }
            }
            for gy in 0..m.h {
                let c = if gy < 3 {
                    base.lerp(WHITE, 0.5)
                } else {
                    base.scale(1.0 - gy as f32 * 0.025)
                };
                for gx in 0..m.w {
                    if m.get(gx, gy) {
                        s.put(x + gx, y + gy, c);
                    }
                }
            }
        }
    }
}

fn starfield(s: &mut Screen, t: f32) {
    for (layer, (speed, color, n)) in [(14.0, SLATE, 50), (34.0, STEEL_LT, 30), (80.0, WHITE, 14)]
        .into_iter()
        .enumerate()
    {
        for k in 0..n {
            let seed = k * 7919 + layer as i32 * 104_729;
            let x = ((seed % 977) as f32 * 3.3 - t * speed).rem_euclid(W as f32) as i32;
            let y = (seed / 7 % (H - 30)).abs();
            s.put(x, y, color);
            if layer == 2 {
                s.put(x + 1, y, color.scale(0.5));
                s.put(x + 2, y, color.scale(0.25));
            }
        }
    }
}

/// Raster bars bouncing behind the logo.
fn copper(s: &mut Screen, t: f32) {
    const BARS: [Rgb; 5] = [RED, ORANGE, YELLOW, GREEN, MAGENTA];
    for (k, base) in BARS.into_iter().enumerate() {
        let cy = 40 + ((t * 1.6 + k as f32 * 0.55).sin() * 30.0) as i32;
        for dy in -5i32..=5 {
            let f = 1.0 - dy.abs() as f32 / 6.0;
            let c = base.scale(f).lerp(WHITE, if dy == 0 { 0.5 } else { 0.0 });
            s.fill_rect(0, cy + dy, W, 1, c);
        }
    }
}

/// The current info page, typed out character by character.
fn info_page(s: &mut Screen, t: f32) {
    let page = (t / PAGE_TIME) as usize % PAGES.len();
    let local = t % PAGE_TIME;
    let mut budget = (local * TYPE_SPEED) as usize;
    let colors = [WHITE, YELLOW, CYAN_LT];
    for (row, line) in PAGES[page].iter().enumerate() {
        let shown: String = line.chars().take(budget).collect();
        budget = budget.saturating_sub(line.chars().count());
        let font = if line.len() > 24 {
            Font::Tiny
        } else {
            Font::Normal
        };
        let y = 92 + row as i32 * 12;
        let x = W / 2 - font.measure(line, 1) / 2;
        font::draw_shadow(s, font, x, y, &shown, colors[row], BLACK);
        let typing = shown.len() < line.len() && (!shown.is_empty() || row == 0);
        if typing && (t * 8.0) as i32 % 2 == 0 {
            let cx = x + font.measure(&shown, 1) + 1;
            s.fill_rect(cx, y, font.advance() - 1, font.height(), colors[row]);
            break;
        }
    }
}

/// Mirror the scroller into rippling water below it.
fn water(s: &mut Screen, t: f32) {
    for r in 0..(H - WATER_Y) {
        let src_y = WATER_Y - 1 - r;
        let off = ((r as f32 * 0.7 + t * 5.0).sin() * (1.0 + r as f32 * 0.08)) as i32;
        for x in 0..W {
            let c = s.get(x + off, src_y);
            s.put(x, WATER_Y + r, c.lerp(BLUE_DK, 0.55).scale(0.85));
        }
    }
    s.hline(0, W - 1, WATER_Y, CYAN.scale(0.6));
}

/// Hue (0..1), saturation, value to RGB.
fn hsv(h: f32, s: f32, v: f32) -> Rgb {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - f * s), v * (1.0 - (1.0 - f) * s));
    let (r, g, b) = match i as i32 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}
