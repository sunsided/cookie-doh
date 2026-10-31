//! Pixel graphics: a fixed-size RGB framebuffer and the primitives the renderer draws with.

pub mod font;
mod mask;
pub mod palette;
pub mod png;
pub mod sprite;

pub use mask::Mask;
pub use palette::Rgb;
pub use sprite::Sprite;

/// Framebuffer width in pixels (a classic 320x200 mode).
pub const W: i32 = 320;
/// Framebuffer height in pixels.
pub const H: i32 = 200;

/// A 320x200 RGB framebuffer.
#[derive(Clone)]
pub struct Screen {
    pub px: Vec<Rgb>,
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen {
    pub fn new() -> Self {
        Self {
            px: vec![Rgb(0, 0, 0); (W * H) as usize],
        }
    }

    pub fn clear(&mut self, c: Rgb) {
        self.px.fill(c);
    }

    pub fn copy_from(&mut self, other: &Screen) {
        self.px.copy_from_slice(&other.px);
    }

    #[inline]
    pub fn put(&mut self, x: i32, y: i32, c: Rgb) {
        if (0..W).contains(&x) && (0..H).contains(&y) {
            self.px[(y * W + x) as usize] = c;
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Rgb {
        if (0..W).contains(&x) && (0..H).contains(&y) {
            self.px[(y * W + x) as usize]
        } else {
            Rgb(0, 0, 0)
        }
    }

    /// Mix `c` over the pixel with opacity `a` (0..=1).
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        if (0..W).contains(&x) && (0..H).contains(&y) {
            let i = (y * W + x) as usize;
            self.px[i] = self.px[i].lerp(c, a);
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        for yy in y.max(0)..(y + h).min(H) {
            for xx in x.max(0)..(x + w).min(W) {
                self.px[(yy * W + xx) as usize] = c;
            }
        }
    }

    pub fn blend_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb, a: f32) {
        for yy in y.max(0)..(y + h).min(H) {
            for xx in x.max(0)..(x + w).min(W) {
                let i = (yy * W + xx) as usize;
                self.px[i] = self.px[i].lerp(c, a);
            }
        }
    }

    /// Checkerboard dither of two colors, the 16-color-era way to fake a third shade.
    pub fn dither_rect(&mut self, x: i32, y: i32, w: i32, h: i32, a: Rgb, b: Rgb) {
        for yy in y.max(0)..(y + h).min(H) {
            for xx in x.max(0)..(x + w).min(W) {
                let c = if (xx + yy) & 1 == 0 { a } else { b };
                self.px[(yy * W + xx) as usize] = c;
            }
        }
    }

    /// Vertical gradient from `top` to `bottom`, ordered-dithered between bands so it reads
    /// like a limited palette rather than a smooth modern ramp.
    pub fn gradient_rect(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        top: Rgb,
        bottom: Rgb,
        steps: u32,
    ) {
        const BAYER: [[f32; 4]; 4] = [
            [0.0, 8.0, 2.0, 10.0],
            [12.0, 4.0, 14.0, 6.0],
            [3.0, 11.0, 1.0, 9.0],
            [15.0, 7.0, 13.0, 5.0],
        ];
        let steps = steps.max(2) as f32;
        for yy in y.max(0)..(y + h).min(H) {
            let t = (yy - y) as f32 / (h - 1).max(1) as f32 * (steps - 1.0);
            for xx in x.max(0)..(x + w).min(W) {
                let th = BAYER[(yy & 3) as usize][(xx & 3) as usize] / 16.0;
                let band = (t + th).floor().min(steps - 1.0) / (steps - 1.0);
                self.px[(yy * W + xx) as usize] = top.lerp(bottom, band);
            }
        }
    }

    pub fn hline(&mut self, x0: i32, x1: i32, y: i32, c: Rgb) {
        for x in x0.min(x1)..=x0.max(x1) {
            self.put(x, y, c);
        }
    }

    pub fn vline(&mut self, x: i32, y0: i32, y1: i32, c: Rgb) {
        for y in y0.min(y1)..=y0.max(y1) {
            self.put(x, y, c);
        }
    }

    pub fn rect_outline(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        self.hline(x, x + w - 1, y, c);
        self.hline(x, x + w - 1, y + h - 1, c);
        self.vline(x, y, y + h - 1, c);
        self.vline(x + w - 1, y, y + h - 1, c);
    }

    /// A 3D bevelled panel: light top-left edge, dark bottom-right edge, `fill` inside.
    pub fn bevel(&mut self, x: i32, y: i32, w: i32, h: i32, light: Rgb, fill: Rgb, dark: Rgb) {
        self.fill_rect(x, y, w, h, fill);
        self.hline(x, x + w - 1, y, light);
        self.vline(x, y, y + h - 1, light);
        self.hline(x, x + w - 1, y + h - 1, dark);
        self.vline(x + w - 1, y, y + h - 1, dark);
    }

    /// Bresenham line.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Rgb) {
        let (mut x, mut y) = (x0, y0);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.put(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn fill_circle(&mut self, cx: i32, cy: i32, r: i32, c: Rgb) {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r + r / 2 {
                    self.put(cx + dx, cy + dy, c);
                }
            }
        }
    }

    pub fn circle_outline(&mut self, cx: i32, cy: i32, r: i32, c: Rgb) {
        let outer = r * r + r / 2;
        let inner = (r - 1) * (r - 1) + (r - 1) / 2;
        for dy in -r..=r {
            for dx in -r..=r {
                let d = dx * dx + dy * dy;
                if d <= outer && d > inner {
                    self.put(cx + dx, cy + dy, c);
                }
            }
        }
    }

    /// Rounded box with a 1px outline, used for speech bubbles and dialogs.
    pub fn round_box(&mut self, x: i32, y: i32, w: i32, h: i32, fill: Rgb, outline: Rgb) {
        self.fill_rect(x + 1, y + 1, w - 2, h - 2, fill);
        self.hline(x + 2, x + w - 3, y, outline);
        self.hline(x + 2, x + w - 3, y + h - 1, outline);
        self.vline(x, y + 2, y + h - 3, outline);
        self.vline(x + w - 1, y + 2, y + h - 3, outline);
        self.put(x + 1, y + 1, outline);
        self.put(x + w - 2, y + 1, outline);
        self.put(x + 1, y + h - 2, outline);
        self.put(x + w - 2, y + h - 2, outline);
    }

    pub fn blit(&mut self, s: &Sprite, x: i32, y: i32) {
        self.blit_ex(s, x, y, false, None);
    }

    pub fn blit_flipped(&mut self, s: &Sprite, x: i32, y: i32) {
        self.blit_ex(s, x, y, true, None);
    }

    /// Draw `s` at `x, y`, optionally mirrored, optionally as a flat-color silhouette.
    pub fn blit_ex(&mut self, s: &Sprite, x: i32, y: i32, flip: bool, solid: Option<Rgb>) {
        for sy in 0..s.h {
            for sx in 0..s.w {
                let src_x = if flip { s.w - 1 - sx } else { sx };
                if let Some(c) = s.px[(sy * s.w + src_x) as usize] {
                    self.put(x + sx, y + sy, solid.unwrap_or(c));
                }
            }
        }
    }

    /// Draw a 1px outline around a sprite's silhouette (for the logo and pickups).
    pub fn blit_outline(&mut self, s: &Sprite, x: i32, y: i32, c: Rgb) {
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            self.blit_ex(s, x + dx, y + dy, false, Some(c));
        }
    }

    /// Convert to RGBA8 for upload, applying an optional full-screen tint flash.
    pub fn to_rgba(&self, out: &mut [u8], flash: Option<(Rgb, f32)>) {
        for (o, p) in out.as_chunks_mut::<4>().0.iter_mut().zip(&self.px) {
            let c = match flash {
                Some((f, a)) => p.lerp(f, a),
                None => *p,
            };
            o[0] = c.0;
            o[1] = c.1;
            o[2] = c.2;
            o[3] = 255;
        }
    }
}
