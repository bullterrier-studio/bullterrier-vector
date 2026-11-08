//! `Color`: a paint colour, or no paint.

use crate::Error;
use crate::key::{Key, Keyed};
use std::fmt;

/// The alpha of a colour written without one.
// constant: the largest alpha byte, fully opaque
const OPAQUE: u8 = 0xff;

/// A colour with red, green, blue and alpha, or [`Color::NONE`] for no paint at all. Written as `"#rrggbb"` or
/// `"#rrggbbaa"` (hex digits in either case), or `"none"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    rgba: Option<[u8; 4]>,
}

impl Color {
    /// No paint: a fill or stroke of this colour paints nothing.
    pub const NONE: Self = Self { rgba: None };

    /// The opaque colour `0xrrggbb`; only the lowest 24 bits are read.
    #[must_use]
    pub const fn hex(rgb: u32) -> Self {
        let [_, red, green, blue] = rgb.to_be_bytes();
        Self {
            rgba: Some([red, green, blue, OPAQUE]),
        }
    }

    /// The colour of these red, green, blue and alpha bytes.
    pub(crate) const fn from_bytes(rgba: [u8; 4]) -> Self {
        Self { rgba: Some(rgba) }
    }

    /// The colour written as `"#rrggbb"` (opaque) or `"#rrggbbaa"`, in either case, or `"none"` for [`Color::NONE`].
    /// There are no colour names, no three-digit form and no `rgb()`.
    ///
    /// # Errors
    /// [`Error::Parameter`] naming `color` if `text` is none of these.
    pub fn parse(text: &str) -> Result<Self, Error> {
        if text == "none" {
            return Ok(Self::NONE);
        }
        text.strip_prefix('#')
            .map(str::as_bytes)
            .filter(|digits| matches!(digits.len(), 6 | 8))
            .and_then(|digits| {
                let mut rgba = [OPAQUE; 4];
                for (slot, pair) in rgba.iter_mut().zip(digits.chunks(2)) {
                    *slot = hex_byte(pair)?;
                }
                Some(Self { rgba: Some(rgba) })
            })
            .ok_or_else(|| {
                Error::parameter(
                    "color",
                    format!("must be \"#rrggbb\", \"#rrggbbaa\" or \"none\", got {text:?}"),
                )
            })
    }

    /// The red, green, blue and alpha bytes, or `None` for [`Color::NONE`].
    #[must_use]
    pub fn rgba(self) -> Option<[u8; 4]> {
        self.rgba
    }
}

impl Keyed for Color {
    fn write_key(&self, key: &mut Key) {
        match self.rgba {
            None => key.word(0),
            Some(rgba) => {
                key.word(1);
                key.word(u64::from(u32::from_be_bytes(rgba)));
            }
        }
    }
}

/// The byte two hex digits write, in either case.
fn hex_byte(pair: &[u8]) -> Option<u8> {
    let [high, low] = pair else { return None };
    Some(hex_digit(*high)? << 4 | hex_digit(*low)?)
}

fn hex_digit(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

impl fmt::Display for Color {
    /// `#rrggbbaa` in lower case, or `none`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.rgba {
            Some([red, green, blue, alpha]) => write!(formatter, "#{red:02x}{green:02x}{blue:02x}{alpha:02x}"),
            None => formatter.write_str("none"),
        }
    }
}
