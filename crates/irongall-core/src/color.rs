use crate::error::{Error, Result};

/// sRGB color used throughout palettes and writers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parse `#RRGGBB` or `RRGGBB` (also allows `0x` prefix).
    pub fn parse(input: &str) -> Result<Self> {
        let s = input.trim();
        let s = s.strip_prefix('#').unwrap_or(s);
        let s = s.strip_prefix("0x").unwrap_or(s);
        let s = s.strip_prefix("0X").unwrap_or(s);
        if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::user(format!("invalid hex color: {input}")));
        }
        let n = u32::from_str_radix(s, 16)
            .map_err(|_| Error::user(format!("invalid hex color: {input}")))?;
        Ok(Self::new(
            ((n >> 16) & 0xff) as u8,
            ((n >> 8) & 0xff) as u8,
            (n & 0xff) as u8,
        ))
    }

    pub fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn hex_lower(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn hex_bare(self) -> String {
        format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// `r,g,b` decimal, as used by KDE `kdeglobals`.
    pub fn rgb_csv(self) -> String {
        format!("{},{},{}", self.r, self.g, self.b)
    }

    pub fn to_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Relative luminance in 0..=1 (sRGB, Rec. 709).
    pub fn luminance(self) -> f32 {
        fn lin(c: u8) -> f32 {
            let s = c as f32 / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    pub fn is_dark(self) -> bool {
        self.luminance() < 0.5
    }

    /// Lighten toward white; used to derive bright ANSI when Base24 is absent.
    pub fn lighten(self, amount: f32) -> Self {
        let t = amount.clamp(0.0, 1.0);
        Self::new(
            lerp(self.r, 255, t),
            lerp(self.g, 255, t),
            lerp(self.b, 255, t),
        )
    }
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_with_and_without_hash() {
        assert_eq!(Rgb::parse("#0A1528").unwrap(), Rgb::new(0x0A, 0x15, 0x28));
        assert_eq!(Rgb::parse("0A1528").unwrap(), Rgb::new(0x0A, 0x15, 0x28));
    }

    #[test]
    fn dark_vs_light_luminance() {
        assert!(Rgb::parse("000000").unwrap().is_dark());
        assert!(!Rgb::parse("ffffff").unwrap().is_dark());
        assert!(Rgb::parse("0A1528").unwrap().is_dark());
    }
}
