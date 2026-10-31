//! Chunky shaded arcade logos built from font glyphs.

use super::hash;
use crate::gfx::font::Font;
use crate::gfx::palette::*;
use crate::gfx::{Mask, Screen};

/// Colors for a shaded logo word.
pub(crate) struct LogoStyle {
    pub top: Rgb,
    pub mid: Rgb,
    pub bottom: Rgb,
    pub highlight: Rgb,
    pub shade: Rgb,
    pub outline: Rgb,
    pub chips: bool,
}

/// A word blown up 4x with Scale2x twice, so the 5x7 glyphs get smooth diagonals.
pub(crate) fn logo_mask(text: &str) -> Mask {
    Font::Normal.mask(text).scale2x().scale2x()
}

/// Paint a mask as a chunky arcade logo: drop shadow, two-tone outline, vertical gradient,
/// bevel light and shade, corner glints, and optionally chocolate chips.
pub(crate) fn paint_logo(s: &mut Screen, m: &Mask, x: i32, y: i32, st: &LogoStyle) {
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
