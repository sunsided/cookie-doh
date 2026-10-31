//! 1-bit masks, for building shaded logos out of font glyphs.

/// A 1-bit image. Out-of-bounds reads are `false`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mask {
    pub w: i32,
    pub h: i32,
    bits: Vec<bool>,
}

impl Mask {
    pub fn new(w: i32, h: i32) -> Self {
        Self {
            w,
            h,
            bits: vec![false; (w * h) as usize],
        }
    }

    pub fn get(&self, x: i32, y: i32) -> bool {
        (0..self.w).contains(&x) && (0..self.h).contains(&y) && self.bits[(y * self.w + x) as usize]
    }

    pub fn set(&mut self, x: i32, y: i32, v: bool) {
        if (0..self.w).contains(&x) && (0..self.h).contains(&y) {
            self.bits[(y * self.w + x) as usize] = v;
        }
    }

    /// Double the size with Scale2x (EPX): diagonal steps become smooth slopes instead of
    /// staircases, which is what makes a blown-up 5x7 font look drawn rather than zoomed.
    pub fn scale2x(&self) -> Mask {
        let mut out = Mask::new(self.w * 2, self.h * 2);
        for y in 0..self.h {
            for x in 0..self.w {
                let p = self.get(x, y);
                let (a, b, c, d) = (
                    self.get(x, y - 1),
                    self.get(x + 1, y),
                    self.get(x - 1, y),
                    self.get(x, y + 1),
                );
                let e0 = if c == a && c != d && a != b { a } else { p };
                let e1 = if a == b && a != c && b != d { b } else { p };
                let e2 = if d == c && d != b && c != a { c } else { p };
                let e3 = if b == d && b != a && d != c { d } else { p };
                out.set(2 * x, 2 * y, e0);
                out.set(2 * x + 1, 2 * y, e1);
                out.set(2 * x, 2 * y + 1, e2);
                out.set(2 * x + 1, 2 * y + 1, e3);
            }
        }
        out
    }

    /// Clear a filled circle (for taking a bite out of something).
    pub fn carve_circle(&mut self, cx: i32, cy: i32, r: i32) {
        for y in cy - r..=cy + r {
            for x in cx - r..=cx + r {
                if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r {
                    self.set(x, y, false);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale2x_doubles_and_smooths_diagonals() {
        // A diagonal of two pixels.
        let mut m = Mask::new(2, 2);
        m.set(0, 0, true);
        m.set(1, 1, true);
        let s = m.scale2x();
        assert_eq!((s.w, s.h), (4, 4));
        // The gap between them gets bridged.
        assert!(s.get(1, 1) && s.get(2, 2));
        assert!(s.get(2, 1) || s.get(1, 2));
        // A solid block stays solid inside, with its outer corners rounded off.
        let mut b = Mask::new(3, 3);
        for y in 0..3 {
            for x in 0..3 {
                b.set(x, y, true);
            }
        }
        let sb = b.scale2x();
        assert!((1..5).all(|y| (1..5).all(|x| sb.get(x, y))));
        assert!(!sb.get(0, 0) && !sb.get(5, 5));
        assert!(sb.get(2, 0) && sb.get(0, 2));
    }
}
