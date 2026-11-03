//! Reading the pixels `Drawing::to_rgba` returns, and the closed-form shapes the raster tests measure them against.

use bullterrier_vector::{Drawing, Error, Size, Values};

/// The straight-alpha RGBA image of a drawing: rows top to bottom, four bytes a pixel.
pub(crate) struct Image {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) bytes: Vec<u8>,
}

impl Image {
    /// The `[red, green, blue, alpha]` of the pixel in column `x` of row `y`, counted from the top left.
    pub(crate) fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let at = (y * self.width + x) * 4;
        [
            self.bytes[at],
            self.bytes[at + 1],
            self.bytes[at + 2],
            self.bytes[at + 3],
        ]
    }

    pub(crate) fn alpha(&self, x: usize, y: usize) -> u8 {
        self.pixel(x, y)[3]
    }

    /// Whether every pixel is `rgba`.
    pub(crate) fn all_pixels_are(&self, rgba: [u8; 4]) -> bool {
        self.bytes.as_chunks::<4>().0.iter().all(|pixel| *pixel == rgba)
    }

    /// The coverage of the whole image: the sum of every pixel's alpha, in pixels.
    pub(crate) fn covered_area(&self) -> f64 {
        self.bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|rgba| f64::from(rgba[3]))
            .sum::<f64>()
            / 255.0
    }
}

/// `drawing` drawn at `width` by `height` pixels with no Param values.
pub(crate) fn render(drawing: &Drawing, width: usize, height: usize) -> Result<Image, Error> {
    #[expect(clippy::cast_precision_loss, reason = "test sizes are small")]
    let size = Size::px(width as f64, height as f64);
    let bytes = drawing.to_rgba(size, &Values::new())?;
    Ok(Image { width, height, bytes })
}

/// The rows every output refuses as a size, whatever its largest side: (case, width, height, the side named, the
/// value shown).
pub(crate) const SIZE_REFUSALS: [(&str, f64, f64, &str, &str); 8] = [
    ("zero width", 0.0, 10.0, "width", "0"),
    ("zero height", 10.0, 0.0, "height", "0"),
    ("half a pixel wide", 0.5, 10.0, "width", "0.5"),
    ("a fraction high", 10.0, 1080.5, "height", "1080.5"),
    ("not a number wide", f64::NAN, 10.0, "width", "NaN"),
    ("not a number high", 10.0, f64::NAN, "height", "NaN"),
    ("infinite", f64::INFINITY, 10.0, "width", "inf"),
    ("negative", 10.0, -4.0, "height", "-4"),
];

/// The rows a pixel output refuses as a size: those every output refuses, then the pixel limits (65,000 a side, 2^27
/// pixels in all). One table for `to_rgba` and `to_png`.
pub(crate) fn pixel_size_refusals() -> Vec<(&'static str, f64, f64, &'static str, &'static str)> {
    let pixel_limits = [
        ("one past the limit wide", 65_001.0, 10.0, "width", "65001"),
        ("one past the limit high", 10.0, 65_001.0, "height", "65001"),
        ("the svg limit wide", 100_000.0, 1.0, "width", "100000"),
        (
            "one column past 2^27 pixels",
            16_385.0,
            8192.0,
            "size",
            "width 16385 by height 8192",
        ),
        (
            "one row past 2^27 pixels",
            8192.0,
            16_385.0,
            "size",
            "width 8192 by height 16385",
        ),
        (
            "the largest square",
            65_000.0,
            65_000.0,
            "size",
            "width 65000 by height 65000",
        ),
    ];
    SIZE_REFUSALS.into_iter().chain(pixel_limits).collect()
}

/// The area where the axis-aligned rectangle `[left, right] x [top, bottom]` overlaps the pixel square in column `x`
/// and row `y`: exact, by clipping.
pub(crate) fn overlap(rect: (f64, f64, f64, f64), x: usize, y: usize) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "test sizes are small")]
    let (px, py) = (x as f64, y as f64);
    let (left, top, right, bottom) = rect;
    let across = (right.min(px + 1.0) - left.max(px)).max(0.0);
    let down = (bottom.min(py + 1.0) - top.max(py)).max(0.0);
    across * down
}
