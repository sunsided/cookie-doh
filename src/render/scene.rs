//! The static night-shift cookie factory, painted once into a background buffer.

use super::art::Art;
use crate::gfx::font::{self, Font};
use crate::gfx::palette::*;
use crate::gfx::{H, Screen, W};
use crate::logic::layout::*;

pub const HUD_H: i32 = 14;
/// Lanes are this far apart.
pub const LANE_GAP: f32 = 39.0;

/// Website windows that spit cookies, one per lane: (domain, title bar color).
pub const SITES: [(&str, Rgb); LANES] = [
    ("ADS.IO", RED),
    ("TRK.ME", PURPLE),
    ("NEWS.X", TEAL),
    ("SHOPPY", ORANGE),
];

/// Lift shaft rail X.
pub const RAIL_X: i32 = 281;
/// Cookie meter tube.
pub const METER: (i32, i32, i32, i32) = (302, 24, 13, 160);

pub fn paint(s: &mut Screen, art: &Art) {
    wall(s);
    windows(s, art);
    machinery(s);
    floor(s);
    webs(s);
    for lane in 0..LANES {
        belt_frame(s, lane);
        site_housing(s, lane);
    }
    mainframe(s);
    shaft(s);
    meter_tube(s, art);
}

fn wall(s: &mut Screen) {
    s.gradient_rect(0, HUD_H, W, H - HUD_H, NIGHT, NAVY, 5);
    // Riveted wall panels.
    for x in (0..W).step_by(40) {
        s.vline(x, HUD_H, H, SLATE.scale(0.7));
        s.vline(x + 1, HUD_H, H, NAVY);
        for y in (HUD_H + 6..H).step_by(13) {
            s.put(x + 4, y, SLATE);
            s.put(x + 35, y, SLATE);
        }
    }
    for y in [HUD_H + 1, 70, 109, 148] {
        s.hline(0, W, y, SLATE.scale(0.6));
    }
}

fn windows(s: &mut Screen, art: &Art) {
    // Three tall factory windows looking out at the night, behind the top belt.
    for (i, x) in [62, 122, 182].into_iter().enumerate() {
        let (y, w, h) = (HUD_H + 4, 48, 30);
        s.fill_rect(x - 2, y - 2, w + 4, h + 4, SLATE);
        s.gradient_rect(x, y, w, h, Rgb::hex(0x10164a), Rgb::hex(0x3a2a6a), 6);
        // Stars.
        for k in 0..14 {
            let sx = x + ((k * 37 + i as i32 * 11) % w);
            let sy = y + ((k * 23 + i as i32 * 7) % (h - 8));
            s.put(sx, sy, if k % 3 == 0 { WHITE } else { STEEL_LT });
        }
        // A skyline of server towers, blinking antennas (blinks are dynamic).
        let mut bx = x;
        let mut k = i as i32;
        while bx < x + w {
            let bw = 5 + (k * 7 % 6);
            let bh = 6 + (k * 13 % 11);
            s.fill_rect(bx, y + h - bh, bw.min(x + w - bx), bh, NAVY);
            for wy in (y + h - bh + 2..y + h).step_by(3) {
                for wx in (bx + 1..(bx + bw - 1).min(x + w)).step_by(2) {
                    if (wx * 3 + wy * 5 + k) % 4 == 0 {
                        s.put(wx, wy, YELLOW.scale(0.6));
                    }
                }
            }
            bx += bw + 1;
            k += 1;
        }
        if i == 1 {
            s.blit(&art.moon, x + 30, y + 3);
        }
        // Mullions.
        s.vline(x + w / 2, y, y + h - 1, SLATE);
        s.hline(x, x + w - 1, y + h / 2, SLATE);
        s.rect_outline(x - 2, y - 2, w + 4, h + 4, STEEL);
        s.hline(x - 3, x + w + 2, y + h + 2, STEEL_LT);
    }
}

/// Dim pipes and server cabinets between the belts.
fn machinery(s: &mut Screen) {
    for lane in 1..LANES {
        let top = LANE_Y[lane - 1] as i32 + 9;
        let bottom = LANE_Y[lane] as i32 - 1;
        // A pipe with flanges.
        let py = top + 3;
        s.fill_rect(46, py, 196, 4, SLATE);
        s.hline(46, 241, py, STEEL);
        s.hline(46, 241, py + 3, NAVY);
        for x in (60..240).step_by(36) {
            s.fill_rect(x, py - 1, 3, 6, STEEL);
            s.put(x + 1, py, STEEL_LT);
        }
        // Server cabinets.
        for (j, x) in (70..236).step_by(42).enumerate() {
            let (cw, cy) = (30, py + 7);
            let ch = bottom - cy - 1;
            if ch < 8 {
                continue;
            }
            s.bevel(x, cy, cw, ch, SLATE, NIGHT, NAVY);
            for vy in (cy + 3..cy + ch - 2).step_by(3) {
                s.hline(x + 3, x + 15, vy, NAVY);
            }
            // LED strip placeholder (lit dynamically).
            s.fill_rect(x + 19, cy + 3, 8, ch - 6, NAVY);
            if (j + lane) % 2 == 0 {
                // A gauge.
                s.fill_circle(x + 23, cy + ch - 6, 3, PALE.scale(0.7));
                s.put(x + 23, cy + ch - 6, RED);
                s.put(x + 24, cy + ch - 7, RED);
            }
        }
    }
}

fn floor(s: &mut Screen) {
    let y = FLOOR_Y as i32;
    for ty in y..H {
        for tx in 0..W {
            let checker = ((tx / 10) + (ty - y) / 5) % 2 == 0;
            s.put(tx, ty, if checker { SLATE } else { NIGHT });
        }
    }
    s.hline(0, W, y, STEEL_LT);
    s.hline(0, W, y + 1, STEEL);
}

fn webs(s: &mut Screen) {
    // Cobwebs in the top corners: the World Wide Web, literally.
    let corners = [(40, HUD_H, 1), (247, HUD_H, -1)];
    for (cx, cy, dir) in corners {
        let c = PALE.scale(0.55);
        for k in 0..5 {
            let ang = k as f32 * 0.39;
            let (ex, ey) = (
                cx + (dir as f32 * 26.0 * ang.cos()) as i32,
                cy + (26.0 * ang.sin()) as i32,
            );
            s.line(cx, cy, ex, ey, c);
        }
        for r in [7.0f32, 13.0, 19.0, 24.0] {
            let mut prev = None;
            for k in 0..=8 {
                let ang = k as f32 * 0.196;
                let rr = r - (k % 2) as f32 * 1.5;
                let p = (
                    cx + (dir as f32 * rr * ang.cos()) as i32,
                    cy + (rr * ang.sin()) as i32,
                );
                if let Some((px, py)) = prev {
                    s.line(px, py, p.0, p.1, c);
                }
                prev = Some(p);
            }
        }
    }
}

fn belt_frame(s: &mut Screen, lane: usize) {
    let y = LANE_Y[lane] as i32;
    let (x0, x1) = (BELT_START_X as i32 - 4, BELT_END_X as i32);
    // Brackets holding the belt to the wall (the bottom lane stands on legs).
    for bx in (x0 + 24..x1 - 10).step_by(54) {
        if lane == LANES - 1 {
            s.fill_rect(bx, y + 7, 3, FLOOR_Y as i32 - y - 7, SLATE);
            s.vline(bx, y + 7, FLOOR_Y as i32 - 1, STEEL);
            s.fill_rect(bx - 2, FLOOR_Y as i32 - 2, 7, 2, STEEL);
        } else {
            s.line(bx, y + 7, bx - 9, y + 17, SLATE);
            s.line(bx + 1, y + 7, bx - 8, y + 17, STEEL);
            s.fill_rect(bx - 11, y + 16, 5, 3, STEEL);
        }
    }
    // Belt bed.
    s.fill_rect(x0, y, x1 - x0, 7, GREY_DK);
    s.hline(x0, x1 - 1, y + 5, STEEL);
    s.hline(x0, x1 - 1, y + 6, SLATE);
    // Hazard stripes on the side rail.
    for x in x0..x1 {
        let c = if ((x - y) / 3) % 2 == 0 {
            YELLOW.scale(0.8)
        } else {
            BLACK
        };
        s.put(x, y + 7, c);
        s.put(x, y + 8, c.scale(0.6));
    }
}

fn site_housing(s: &mut Screen, lane: usize) {
    let y = LANE_Y[lane] as i32;
    // Cable from the mainframe to the window.
    s.hline(10, 14, y - 14, GREY);
    s.hline(10, 14, y - 6, GREY);
    // A little ramp from the mouth to the belt.
    s.line(30, y - 4, 38, y, STEEL);
    s.line(30, y - 3, 38, y + 1, SLATE);
}

/// "THE INTERNET": the mainframe column every website hangs off.
fn mainframe(s: &mut Screen) {
    let (x, w) = (0, 12);
    s.bevel(x, HUD_H, w, FLOOR_Y as i32 - HUD_H, STEEL, SLATE, NAVY);
    for y in (HUD_H + 3..FLOOR_Y as i32 - 2).step_by(4) {
        s.hline(x + 2, x + 9, y, NIGHT);
    }
    // Vertical label in tiny font, one letter per line.
    for (i, ch) in "INTERNET".chars().enumerate() {
        let mut buf = [0u8; 4];
        font::draw(
            s,
            Font::Tiny,
            x + 4,
            30 + i as i32 * 18,
            ch.encode_utf8(&mut buf),
            PALE,
        );
    }
}

fn shaft(s: &mut Screen) {
    // Rails.
    for rx in [RAIL_X, RAIL_X + 12] {
        s.fill_rect(rx, HUD_H, 3, FLOOR_Y as i32 - HUD_H, SLATE);
        s.vline(rx, HUD_H, FLOOR_Y as i32, STEEL_LT);
        s.vline(rx + 2, HUD_H, FLOOR_Y as i32, NAVY);
        for y in (HUD_H + 4..FLOOR_Y as i32).step_by(8) {
            s.put(rx + 1, y, STEEL_LT);
        }
    }
    // Head beam and pulley.
    s.bevel(246, HUD_H, 52, 5, STEEL_LT, STEEL, SLATE);
    s.fill_circle(287, HUD_H + 8, 5, GREY);
    s.circle_outline(287, HUD_H + 8, 5, GREY_LT);
    s.fill_circle(287, HUD_H + 8, 1, GREY_DK);
}

fn meter_tube(s: &mut Screen, art: &Art) {
    let (x, y, w, h) = METER;
    s.fill_rect(x - 2, y - 8, w + 4, h + 12, NAVY);
    s.rect_outline(x - 2, y - 8, w + 4, h + 12, STEEL);
    s.blit(&art.life, x + 4, y - 7);
    s.fill_rect(x, y, w, h, Rgb::hex(0x0a1a24));
    // Tick marks.
    for k in 0..=8 {
        let ty = y + h - k * h / 8;
        s.hline(x - 1, x + 1, ty, STEEL_LT);
    }
}
