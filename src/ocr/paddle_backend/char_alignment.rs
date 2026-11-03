//! 【文字识别】【字符位置】估计并校正字符边界以支持精细选字

use super::*;

/// Returns X coordinates for character boundaries (character count + 1 values),
/// sorted left to right within `bounds`.
///
/// Handles proportional spacing: boundaries follow both the recognizer's positions and
/// character widths (exp an 'i' next to a 'W' won't take up half the space).
pub(super) fn char_boundaries(bounds: &Rect, text: &str, norm: &[f32]) -> Vec<f32> {
    let widths: Vec<f32> = text.chars().map(advance).collect();
    let n = widths.len();
    let (left, right) = (bounds.left(), bounds.right());
    if n == 0 {
        return vec![left, right];
    }

    let at = |k: usize| left + bounds.width() * norm[k].clamp(0.0, 1.0);
    let total: f32 = widths.iter().sum();
    let span = if norm.len() == n && n > 1 {
        at(n - 1) - at(0)
    } else {
        0.0
    };
    if span <= 0.0 || total <= 0.0 {
        return modelled(left, right, &widths);
    }
    let unit = span / (total - 0.5 * (widths[0] + widths[n - 1])).max(f32::EPSILON);

    let mut system = Tridiagonal::new(n + 1);
    for (k, &w) in widths.iter().enumerate() {
        if w == 0.0 {
            system.add(k, (-1.0, 1.0), 0.0, MARK_WEIGHT);
            continue;
        }
        system.add(k, (0.5, 0.5), at(k), 1.0);
        system.add(k, (-1.0, 1.0), w * unit, WIDTH_WEIGHT);
    }

    let mut xs = system.solve();
    for x in &mut xs {
        *x = x.clamp(left, right);
    }
    // Now line edges are anchored to the box edge, simple as that,
    // instead of calculating them from character positions, since the
    // position of the first letter in large text often broke due to delay
    xs[0] = left;
    xs[n] = right;
    for k in 0..n {
        xs[k + 1] = xs[k + 1].max(xs[k]);
    }
    xs
}
/// Relative weight of estimated character width compared to the recognizer's position output.
const WIDTH_WEIGHT: f32 = 1.0;

/// High constraint weight enforcing near-zero width for combining diacritical marks.
const MARK_WEIGHT: f32 = 50.0;

// Tridiagonal normal equations where each constraint links two adjacent character boundaries.
struct Tridiagonal {
    diag: Vec<f32>,
    off: Vec<f32>,
    rhs: Vec<f32>,
}

impl Tridiagonal {
    fn new(len: usize) -> Self {
        Self {
            diag: vec![0.0; len],
            off: vec![0.0; len.saturating_sub(1)],
            rhs: vec![0.0; len],
        }
    }

    fn add(&mut self, k: usize, (u, v): (f32, f32), target: f32, weight: f32) {
        self.diag[k] += weight * u * u;
        self.diag[k + 1] += weight * v * v;
        self.off[k] += weight * u * v;
        self.rhs[k] += weight * u * target;
        self.rhs[k + 1] += weight * v * target;
    }

    fn solve(self) -> Vec<f32> {
        let len = self.diag.len();
        let mut c = vec![0.0f32; len];
        let mut d = vec![0.0f32; len];
        for i in 0..len {
            let (sub, cp, dp) = if i > 0 {
                (self.off[i - 1], c[i - 1], d[i - 1])
            } else {
                (0.0, 0.0, 0.0)
            };
            let denom = (self.diag[i] - sub * cp).max(1e-9);
            c[i] = self.off.get(i).map_or(0.0, |off| off / denom);
            d[i] = (self.rhs[i] - sub * dp) / denom;
        }
        let mut xs = vec![0.0f32; len];
        for i in (0..len).rev() {
            let next = if i + 1 < len { c[i] * xs[i + 1] } else { 0.0 };
            xs[i] = d[i] - next;
        }
        xs
    }
}

/// Fallback positioning based solely on estimated character widths.
/// Used for single characters or when OCR positions are missing.
pub(super) fn modelled(left: f32, right: f32, widths: &[f32]) -> Vec<f32> {
    let unit = (right - left) / widths.iter().sum::<f32>().max(f32::EPSILON);
    let mut xs = Vec::with_capacity(widths.len() + 1);
    let mut x = left;
    xs.push(left);
    for w in widths {
        x += w * unit;
        xs.push(x);
    }
    *xs.last_mut().unwrap() = right;
    xs
}

/// Returns estimated character width relative to a standard lowercase letter.
pub(super) fn advance(c: char) -> f32 {
    match c {
        'i' | 'j' | 'l' | 'I' | '|' | '.' | ',' | ':' | ';' | '!' | '\'' | '`' => 0.4,
        ' ' | 'f' | 'r' | 't' | '(' | ')' | '[' | ']' | '{' | '}' | '-' | '"' | '/' | '\\' => 0.6,
        'm' | 'w' | 'M' | 'W' | 'ш' | 'щ' | 'ж' | 'ы' | 'ю' | 'ф' => 1.5,
        'Ш' | 'Щ' | 'Ж' | 'Ы' | 'Ю' | 'Ф' | '—' | '№' => 1.6,
        _ if super::super::is_mark(c) => 0.0,
        _ if c as u32 >= 0x2E80 => 2.0,
        _ if c.is_uppercase() => 1.2,
        _ => 1.0,
    }
}
