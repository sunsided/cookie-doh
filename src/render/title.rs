//! The title screen (with its suspiciously familiar consent banner) and the game-over card.

use super::Renderer;
use crate::gfx::font::{self, Font};
use crate::gfx::palette::*;
use crate::gfx::{Screen, W};

const REJECT_QUIPS: [&str; 5] = [
    "NICE TRY.",
    "ARE YOU SURE? THEY'RE FRESH.",
    "REJECT? IN THIS ECONOMY?",
    "THAT BUTTON IS DECORATIVE.",
    "FINE. IT'S GONE NOW.",
];

const TICKER: &str = "COOKIE D'OH! - ACCEPT ALL COOKIES   ***   UP/DOWN: RIDE THE LIFT   \
    ***   EVERY COOKIE MUST BE ACCEPTED - DROP ONE AND IT'S D'OH!   ***   SPACE: ACCEPT ALL \
    POPUPS   ***   BEWARE OF THE WEB CRAWLER - DON'T STAND WHERE IT LANDS   ***   GOLDEN \
    COOKIES ARE WORTH 10X   ***   FILL THE JAR FOR A BONUS   ***   M: MUTE   C: SCANLINES   ESC: PAUSE, TWICE TO QUIT   \
    ***   ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleAction {
    None,
    Start,
    Nope,
    Moved,
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
}

pub const ACCEPT_BTN: (i32, i32, i32, i32) = (56, 150, 92, 17);

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
            reject: (184, 150, 76, 17),
        }
    }

    pub fn reject_alive(&self) -> bool {
        self.tries < REJECT_QUIPS.len()
    }

    pub fn update(&mut self, dt: f32) {
        self.t += dt;
        self.quip_t = (self.quip_t - dt).max(0.0);
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
        // The button flees and shrinks.
        let (_, _, w, h) = self.reject;
        let spots = [(212, 156), (176, 146), (230, 158), (196, 152)];
        let (nx, ny) = spots[self.tries % spots.len()];
        let nw = (w - 14).max(30);
        self.reject = (nx.min(286 - nw), ny, nw, (h - 2).max(11));
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
}

impl Renderer {
    pub fn draw_title(&self, s: &mut Screen, ti: &Title, hiscore: u32) {
        let t = ti.t;
        s.clear(NAVY);
        // Parallax starfield.
        for k in 0..60 {
            let speed = 6.0 + (k % 3) as f32 * 10.0;
            let x = ((k * 53) as f32 - t * speed).rem_euclid(W as f32) as i32;
            let y = (k * 37) % 200;
            s.put(x, y, if k % 3 == 2 { PALE } else { SLATE });
        }
        // Copper bars, of course.
        for (k, base) in [RED, YELLOW, CYAN].into_iter().enumerate() {
            let cy = 48 + (34.0 * (t * 1.3 + k as f32 * 2.1).sin()) as i32;
            for dy in -7i32..=7 {
                let f = 1.0 - dy.abs() as f32 / 8.0;
                let c = base
                    .scale(f)
                    .lerp(WHITE, if dy.abs() <= 1 { 0.4 } else { 0.0 });
                s.blend_rect(0, cy + dy, W, 1, c, 0.85);
            }
        }

        self.logo(s, t);

        // Subtitle, waving.
        let sub = "ACCEPT ALL COOKIES";
        let sx = 160 - Font::Normal.measure(sub, 2) / 2;
        for (i, ch) in sub.chars().enumerate() {
            let y = 88 + ((t * 4.0 + i as f32 * 0.45).sin() * 2.0) as i32;
            let x = sx + i as i32 * 12;
            let mut b = [0u8; 4];
            let ch = ch.encode_utf8(&mut b);
            font::draw_with(s, Font::Normal, x + 2, y + 2, ch, 2, &|_| RED_DK);
            font::draw_with(s, Font::Normal, x, y, ch, 2, &|row| {
                if row < 3 { YELLOW_LT } else { YELLOW }
            });
        }

        // The cast.
        let bob = ((t * 2.0).sin() * 1.5) as i32;
        s.blit(&self.art.body, 22, 44 + bob);
        s.blit(&self.art.mouth_happy, 29, 56 + bob);
        s.blit(&self.art.jar_back, 26, 61 + bob);
        for (i, &(px, py)) in [(1, 9), (6, 9), (3, 8), (1, 6), (6, 6)].iter().enumerate() {
            let spr = if i == 2 {
                &self.art.mini_golden
            } else {
                &self.art.mini
            };
            s.blit(spr, 26 + px, 61 + bob + py);
        }
        s.blit(&self.art.jar_front, 26, 61 + bob);
        s.blit(&self.art.hand, 24, 66 + bob);
        s.blit_flipped(&self.art.hand, 37, 66 + bob);
        let swing = (t * 1.7).sin() * 6.0;
        let len = 46.0 + (t * 0.9).sin() * 8.0;
        let (sx, sy) = (285.0 + swing, len);
        s.line(285, 0, sx as i32, sy as i32, PALE);
        s.blit(
            &self.art.spider[(t * 3.0) as usize % 2],
            sx as i32 - 7,
            sy as i32,
        );

        font::draw_centered_shadow(
            s,
            Font::Normal,
            160,
            3,
            &format!("HI-SCORE {hiscore:07}"),
            PALE,
            BLACK,
        );

        self.banner(s, ti);

        // Ticker.
        s.fill_rect(0, 190, W, 10, BLACK);
        let len = Font::Normal.measure(TICKER, 1) + 6;
        let x = W - ((t * 45.0) as i32 % (len + W));
        font::draw(s, Font::Normal, x, 192, TICKER, CYAN_LT);
    }

    fn logo(&self, s: &mut Screen, t: f32) {
        const SCALE: i32 = 4;
        let ramp = |row: i32| match row {
            0 => COOKIE_HI,
            1 | 2 => COOKIE_LT,
            3 | 4 => COOKIE,
            _ => BROWN,
        };
        for (line, y0) in [("COOKIE", 14), ("D'OH!", 48)] {
            let x0 = 160 - Font::Normal.measure(line, SCALE) / 2;
            for (i, ch) in line.chars().enumerate() {
                let x = x0 + i as i32 * 6 * SCALE;
                let y = y0 + ((t * 3.0 + i as f32 * 0.7).sin() * 2.0) as i32;
                let mut b = [0u8; 4];
                let ch = ch.encode_utf8(&mut b);
                font::draw_with(s, Font::Normal, x + 3, y + 3, ch, SCALE, &|_| BLACK);
                font::draw_outlined(s, Font::Normal, x, y, ch, SCALE, &ramp, BROWN_DK);
                // Chocolate chips baked into the letters.
                for py in y..y + 7 * SCALE {
                    for px in x..x + 5 * SCALE {
                        let c = s.get(px, py);
                        if (c == COOKIE || c == COOKIE_LT)
                            && super::hash(px / 2 + i as i32 * 3, py / 2 + y0).is_multiple_of(13)
                        {
                            s.put(px, py, CHIP);
                        }
                    }
                }
                // A highlight glint.
                s.put(x + 1, y + 1, WHITE);
            }
        }
    }

    fn banner(&self, s: &mut Screen, ti: &Title) {
        let (x, y, w, h) = (28, 114, 264, 72);
        s.blend_rect(x + 4, y + 4, w, h, BLACK, 0.6);
        s.bevel(x, y, w, h, WHITE, PALE, GREY);
        s.blit(&self.art.cookie, x + 8, y + 8);
        font::draw(
            s,
            Font::Normal,
            x + 22,
            y + 8,
            "THIS GAME USES COOKIES.",
            NAVY,
        );
        font::draw(
            s,
            Font::Tiny,
            x + 22,
            y + 18,
            "LOTS OF THEM. BY PLAYING YOU AGREE TO",
            GREY_DK,
        );
        font::draw(
            s,
            Font::Tiny,
            x + 22,
            y + 24,
            "ACCEPT ALL OF THEM. EVERY. SINGLE. ONE.",
            GREY_DK,
        );

        let blink = (ti.t * 4.0) as i32 % 2 == 0;
        let (ax, ay, aw, ah) = ACCEPT_BTN;
        s.bevel(ax, ay, aw, ah, GREEN_LT, GREEN, GREEN_DK);
        font::draw_centered_shadow(
            s,
            Font::Normal,
            ax + aw / 2,
            ay + 5,
            "ACCEPT ALL",
            WHITE,
            GREEN_DK,
        );
        if ti.reject_alive() {
            let (rx, ry, rw, rh) = ti.reject;
            s.bevel(rx, ry, rw, rh, PALE, GREY_LT, GREY);
            let label = if rw >= 50 { "REJECT" } else { "rej." };
            font::draw_centered(
                s,
                if rw >= 50 { Font::Normal } else { Font::Tiny },
                rx + rw / 2,
                ry + rh / 2 - 3,
                label,
                GREY,
            );
        } else {
            let (rx, ry, rw, rh) = (184, 150, 76, 17);
            s.bevel(rx, ry, rw, rh, GREEN_LT, GREEN, GREEN_DK);
            font::draw_centered_shadow(
                s,
                Font::Normal,
                rx + rw / 2,
                ry + 5,
                "ALSO OK",
                WHITE,
                GREEN_DK,
            );
        }
        let (bx, by, bw, bh) = if ti.sel == 0 { ACCEPT_BTN } else { ti.reject };
        if blink {
            s.rect_outline(bx - 2, by - 2, bw + 4, bh + 4, YELLOW);
            s.rect_outline(bx - 3, by - 3, bw + 6, bh + 6, ORANGE);
        }
        match ti.quip() {
            Some(q) => font::draw_centered(s, Font::Normal, 160, y + 61, q, RED),
            None => font::draw_centered(
                s,
                Font::Tiny,
                160,
                y + 62,
                "LEFT/RIGHT + SPACE  -  OR CLICK",
                GREY,
            ),
        }
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
