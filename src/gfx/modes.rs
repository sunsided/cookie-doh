//! Historical display modes: the finished frame is reduced to a real hardware palette with
//! ordered (Bayer) dithering, the way a game ported to that machine would have looked.

use std::collections::HashMap;

use super::{H, Rgb, Screen, W};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GfxMode {
    /// The game as drawn: VGA / Amiga AGA, a few dozen freely chosen colors.
    #[default]
    Vga,
    /// IBM EGA, the fixed 16-color RGBI palette.
    Ega,
    /// Commodore 64 multicolor: 16 colors, double-wide pixels (160x200), with a hi-res
    /// status bar on top the way C64 games split the screen with a raster interrupt.
    C64,
    /// IBM CGA palette 1, high intensity: black, cyan, magenta, white.
    Cga,
    /// Hercules on an amber monochrome monitor: one bit per pixel.
    Hercules,
}

const EGA: [u32; 16] = [
    0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555,
    0x5555ff, 0x55ff55, 0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff,
];

/// The "Pepto" C64 palette.
const C64: [u32; 16] = [
    0x000000, 0xffffff, 0x813338, 0x75cec8, 0x8e3c97, 0x56ac4d, 0x2e2c9b, 0xedf171, 0x8e5029,
    0x553800, 0xc46c71, 0x4a4a4a, 0x7b7b7b, 0xa9ff9f, 0x706deb, 0xb2b2b2,
];

const CGA: [u32; 4] = [0x000000, 0x55ffff, 0xff55ff, 0xffffff];

const AMBER: Rgb = Rgb::hex(0xffb000);

/// Rows at the top kept at full resolution in C64 mode (the HUD).
const C64_HIRES_ROWS: i32 = 14;

const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

impl GfxMode {
    pub const ALL: [GfxMode; 5] = [
        GfxMode::Vga,
        GfxMode::Ega,
        GfxMode::C64,
        GfxMode::Cga,
        GfxMode::Hercules,
    ];

    pub fn name(self) -> &'static str {
        match self {
            GfxMode::Vga => "VGA",
            GfxMode::Ega => "EGA",
            GfxMode::C64 => "C64",
            GfxMode::Cga => "CGA",
            GfxMode::Hercules => "HERCULES",
        }
    }

    /// A one-line description for the on-screen toast.
    pub fn blurb(self) -> &'static str {
        match self {
            GfxMode::Vga => "VGA - AS NATURE INTENDED",
            GfxMode::Ega => "EGA - 16 COLORS",
            GfxMode::C64 => "C64 - 16 COLORS, FAT PIXELS",
            GfxMode::Cga => "CGA - 4 COLORS. SORRY.",
            GfxMode::Hercules => "HERCULES - AMBER MONOCHROME",
        }
    }

    pub fn next(self) -> GfxMode {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// Parse a mode name, case-insensitively (`"cga"`, `"C64"`, ...).
    pub fn parse(s: &str) -> Option<GfxMode> {
        let s = s.trim().to_ascii_uppercase();
        Self::ALL
            .into_iter()
            .find(|m| m.name() == s || (s == "HGC" && *m == GfxMode::Hercules))
    }

    fn palette(self) -> &'static [u32] {
        match self {
            GfxMode::Vga | GfxMode::Hercules => &[],
            GfxMode::Ega => &EGA,
            GfxMode::C64 => &C64,
            GfxMode::Cga => &CGA,
        }
    }

    /// How far the dither nudges a color before snapping it to the palette.
    fn spread(self) -> f32 {
        match self {
            GfxMode::Vga => 0.0,
            GfxMode::Ega => 56.0,
            GfxMode::C64 => 48.0,
            GfxMode::Cga => 110.0,
            GfxMode::Hercules => 1.0,
        }
    }
}

/// Perceptual-ish color distance ("redmean").
fn distance(a: Rgb, b: Rgb) -> i32 {
    let rm = (a.0 as i32 + b.0 as i32) / 2;
    let (dr, dg, db) = (
        a.0 as i32 - b.0 as i32,
        a.1 as i32 - b.1 as i32,
        a.2 as i32 - b.2 as i32,
    );
    (((512 + rm) * dr * dr) >> 8) + 4 * dg * dg + (((767 - rm) * db * db) >> 8)
}

/// Converts frames to a [`GfxMode`], caching palette lookups across frames.
#[derive(Default)]
pub struct Converter {
    pub mode: GfxMode,
    cache: HashMap<u32, Rgb>,
}

impl Converter {
    pub fn new(mode: GfxMode) -> Self {
        Self {
            mode,
            cache: HashMap::new(),
        }
    }

    pub fn set_mode(&mut self, mode: GfxMode) {
        if mode != self.mode {
            self.mode = mode;
            self.cache.clear();
        }
    }

    fn nearest(&mut self, c: Rgb) -> Rgb {
        let key = (c.0 as u32) << 16 | (c.1 as u32) << 8 | c.2 as u32;
        let pal = self.mode.palette();
        *self.cache.entry(key).or_insert_with(|| {
            pal.iter()
                .map(|&p| Rgb::hex(p))
                .min_by_key(|&p| distance(c, p))
                .unwrap_or(c)
        })
    }

    /// Reduce `s` in place.
    pub fn apply(&mut self, s: &mut Screen) {
        let mode = self.mode;
        if mode == GfxMode::Vga {
            return;
        }
        let spread = mode.spread();
        let luma = |c: Rgb| (0.30 * c.0 as f32 + 0.59 * c.1 as f32 + 0.11 * c.2 as f32) / 255.0;
        // Hercules works on brightness with the edges sharpened, so shapes keep an outline
        // instead of melting into the dithered backdrop.
        let lum: Vec<f32> = if mode == GfxMode::Hercules {
            s.px.iter().map(|&c| luma(c)).collect()
        } else {
            Vec::new()
        };
        for y in 0..H {
            let wide = mode == GfxMode::C64 && y >= C64_HIRES_ROWS;
            let mut x = 0;
            while x < W {
                let i = (y * W + x) as usize;
                // C64 multicolor: two screen pixels make one fat pixel.
                let (c, bx) = if wide {
                    (s.px[i].lerp(s.px[i + 1], 0.5), (x / 2) as usize)
                } else {
                    (s.px[i], x as usize)
                };
                let b = BAYER[(y & 3) as usize][bx & 3] as f32 / 16.0 - 0.5 + 1.0 / 32.0;
                let out = if mode == GfxMode::Hercules {
                    let at = |dx: i32, dy: i32| {
                        let (xx, yy) = ((x + dx).clamp(0, W - 1), (y + dy).clamp(0, H - 1));
                        lum[(yy * W + xx) as usize]
                    };
                    let l = at(0, 0);
                    let around = (at(-1, 0) + at(1, 0) + at(0, -1) + at(0, 1)) / 4.0;
                    // Sharpen, then push the darks down so the night backdrop goes black.
                    let v = (l + 2.0 * (l - around)).clamp(0.0, 1.0).powf(1.35);
                    if v > 0.5 - b { AMBER } else { Rgb(0, 0, 0) }
                } else {
                    let n = |v: u8| (v as f32 + b * spread).clamp(0.0, 255.0) as u8;
                    self.nearest(Rgb(n(c.0), n(c.1), n(c.2)))
                };
                s.px[i] = out;
                if wide {
                    s.px[i + 1] = out;
                    x += 2;
                } else {
                    x += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rainbow() -> Screen {
        let mut s = Screen::new();
        for y in 0..H {
            for x in 0..W {
                s.put(
                    x,
                    y,
                    Rgb(
                        (x * 255 / W) as u8,
                        (y * 255 / H) as u8,
                        ((x + y) % 256) as u8,
                    ),
                );
            }
        }
        s
    }

    #[test]
    fn modes_stay_within_their_palette() {
        for mode in [GfxMode::Ega, GfxMode::C64, GfxMode::Cga] {
            let mut s = rainbow();
            Converter::new(mode).apply(&mut s);
            let pal: Vec<Rgb> = mode.palette().iter().map(|&p| Rgb::hex(p)).collect();
            assert!(s.px.iter().all(|c| pal.contains(c)), "{mode:?}");
        }
        let mut s = rainbow();
        Converter::new(GfxMode::Hercules).apply(&mut s);
        assert!(s.px.iter().all(|c| *c == AMBER || *c == Rgb(0, 0, 0)));
    }

    #[test]
    fn c64_pixels_are_double_wide() {
        let mut s = rainbow();
        Converter::new(GfxMode::C64).apply(&mut s);
        for y in C64_HIRES_ROWS..H {
            for x in (0..W).step_by(2) {
                assert_eq!(s.get(x, y), s.get(x + 1, y));
            }
        }
    }

    #[test]
    fn modes_parse_and_cycle() {
        assert_eq!(GfxMode::parse("cga"), Some(GfxMode::Cga));
        assert_eq!(GfxMode::parse("hgc"), Some(GfxMode::Hercules));
        assert_eq!(GfxMode::parse("svga"), None);
        let mut m = GfxMode::Vga;
        for _ in 0..GfxMode::ALL.len() {
            m = m.next();
        }
        assert_eq!(m, GfxMode::Vga);
    }

    #[test]
    fn vga_is_untouched() {
        let mut s = rainbow();
        let before = s.px.clone();
        Converter::new(GfxMode::Vga).apply(&mut s);
        assert_eq!(s.px, before);
    }
}
