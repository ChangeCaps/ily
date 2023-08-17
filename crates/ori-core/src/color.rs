use std::{
    fmt::Display,
    ops::{Add, AddAssign, Mul},
};

use glam::Vec4;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[allow(missing_docs)]
impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);

    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);

    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);
}

impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub const fn grayscale(g: f32) -> Self {
        Self::rgb(g, g, g)
    }

    /// Try to parse a color from a hex string.
    pub fn try_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');

        let mut color = Self::BLACK;

        match hex.len() {
            2 => {
                color.r = u8::from_str_radix(hex, 16).ok()? as f32 / 255.0;
                color.g = color.r;
                color.b = color.r;
            }
            3 => {
                color.r = u8::from_str_radix(&hex[0..1], 16).ok()? as f32 / 15.0;
                color.g = u8::from_str_radix(&hex[1..2], 16).ok()? as f32 / 15.0;
                color.b = u8::from_str_radix(&hex[2..3], 16).ok()? as f32 / 15.0;
            }
            4 => {
                color.r = u8::from_str_radix(&hex[0..1], 16).ok()? as f32 / 15.0;
                color.g = u8::from_str_radix(&hex[1..2], 16).ok()? as f32 / 15.0;
                color.b = u8::from_str_radix(&hex[2..3], 16).ok()? as f32 / 15.0;
                color.a = u8::from_str_radix(&hex[3..4], 16).ok()? as f32 / 15.0;
            }
            6 => {
                color.r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
                color.g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
                color.b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            }
            8 => {
                color.r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
                color.g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
                color.b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
                color.a = u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0;
            }
            _ => return None,
        }

        Some(color)
    }

    /// Parse a color from a hex string.
    ///
    /// # Panics
    /// - If the string is not a valid hex color.
    pub fn hex(hex: &str) -> Self {
        Self::try_hex(hex).expect("Invalid hex color")
    }

    /// Convert the color to a hex string.
    pub fn to_hex(self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}",
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
        )
    }

    /// Returns a new color with the given hue, saturation, lightness and alpha components.
    ///
    /// See <https://en.wikipedia.org/wiki/HSL_and_HSV>.
    pub fn hsla(h: f32, s: f32, l: f32, a: f32) -> Self {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r, g, b) = match h {
            hue if (0.0..60.0).contains(&hue) => (c, x, 0.0),
            hue if (60.0..120.0).contains(&hue) => (x, c, 0.0),
            hue if (120.0..180.0).contains(&hue) => (0.0, c, x),
            hue if (180.0..240.0).contains(&hue) => (0.0, x, c),
            hue if (240.0..300.0).contains(&hue) => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Self::rgba(r + m, g + m, b + m, a)
    }

    /// Returns a new color with the given hue, saturation, lightness and alpha components.
    ///
    /// See <https://en.wikipedia.org/wiki/HSL_and_HSV>.
    pub fn hsl(h: f32, s: f32, l: f32) -> Self {
        Self::hsla(h, s, l, 1.0)
    }

    /// Convert the color to a hue, saturation, lightness and alpha tuple.
    ///
    /// See <https://en.wikipedia.org/wiki/HSL_and_HSV>.
    pub fn to_hsla(self) -> (f32, f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let delta = max - min;

        let h = if delta == 0.0 {
            0.0
        } else if max == self.r {
            60.0 * (((self.g - self.b) / delta) % 6.0)
        } else if max == self.g {
            60.0 * ((self.b - self.r) / delta + 2.0)
        } else {
            60.0 * ((self.r - self.g) / delta + 4.0)
        };

        let l = (max + min) / 2.0;

        let s = if delta == 0.0 {
            0.0
        } else {
            delta / (1.0 - (2.0 * l - 1.0).abs())
        };

        (h, s, l, self.a)
    }

    /// Convert the color to a hue, saturation, lightness tuple.
    ///
    /// See <https://en.wikipedia.org/wiki/HSL_and_HSV>.
    pub fn to_hsl(self) -> (f32, f32, f32) {
        let (h, s, l, _) = self.to_hsla();
        (h, s, l)
    }

    /// Linearly interpolate between two colors.
    ///
    /// This uses a fractor `t` between `0.0` and `1.0`.
    /// Where `0.0` is `self` and `1.0` is `other`.
    pub fn mix(self, other: Self, t: f32) -> Self {
        other * t + self * (1.0 - t)
    }

    /// Saturates the color by given `amount`.
    pub fn saturate(self, amount: f32) -> Self {
        let (h, s, l, a) = self.to_hsla();
        Self::hsla(h, s + amount, l, a)
    }

    /// Desaturates the color by given `amount`.
    pub fn desaturate(self, amount: f32) -> Self {
        let (h, s, l, a) = self.to_hsla();
        Self::hsla(h, s - amount, l, a)
    }

    /// Brighten the color by the given `amount`.
    pub fn brighten(self, amount: f32) -> Self {
        let (h, s, l, a) = self.to_hsla();
        Self::hsla(h, s, l + amount, a)
    }

    /// Darken the color by the given `amount`.
    pub fn darken(self, amount: f32) -> Self {
        let (h, s, l, a) = self.to_hsla();
        Self::hsla(h, s, l - amount, a)
    }

    /// Returns true if the color is translucent.
    pub fn is_translucent(self) -> bool {
        self.a < 1.0
    }

    /// Convert the color to sRGB.
    ///
    /// See <https://en.wikipedia.org/wiki/SRGB>.
    pub fn to_srgb(self) -> [f32; 4] {
        [self.r.powf(2.2), self.g.powf(2.2), self.b.powf(2.2), self.a]
    }
}

impl From<Color> for [f32; 4] {
    fn from(val: Color) -> Self {
        [val.r, val.g, val.b, val.a]
    }
}

impl From<[f32; 4]> for Color {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Self { r, g, b, a }
    }
}

impl From<Color> for (f32, f32, f32, f32) {
    fn from(val: Color) -> Self {
        (val.r, val.g, val.b, val.a)
    }
}

impl From<(f32, f32, f32, f32)> for Color {
    fn from((r, g, b, a): (f32, f32, f32, f32)) -> Self {
        Self { r, g, b, a }
    }
}

impl From<Color> for Vec4 {
    fn from(val: Color) -> Self {
        Vec4::new(val.r, val.g, val.b, val.a)
    }
}

impl From<Vec4> for Color {
    fn from(vec: Vec4) -> Self {
        Self::rgba(vec.x, vec.y, vec.z, vec.w)
    }
}

impl Mul<f32> for Color {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a * rhs,
        }
    }
}

impl Mul for Color {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            r: self.r * rhs.r,
            g: self.g * rhs.g,
            b: self.b * rhs.b,
            a: self.a * rhs.a,
        }
    }
}

impl Add for Color {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}

impl AddAssign for Color {
    fn add_assign(&mut self, rhs: Self) {
        self.r += rhs.r;
        self.g += rhs.g;
        self.b += rhs.b;
        self.a += rhs.a;
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgba({}, {}, {}, {})", self.r, self.g, self.b, self.a)
    }
}