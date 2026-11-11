//! SVG's `transform` attribute as one matrix.

use crate::Point;
use crate::math::{sin_cos_deg, tan};

/// How far a matrix may stray from a turn, a mirror and an even scale and still count as one. A stroke is scaled by a
/// number, so a matrix that stretches one way cannot carry it.
// constant: the matrices are read from decimal text and multiplied: their rounding is a few parts in 1e16
const EVEN: f64 = 1e-9;

/// The matrix `[a c e; b d f; 0 0 1]` of SVG: a point `(x, y)` goes to `(a x + c y + e, b x + d y + f)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Affine {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Affine {
    pub(super) const IDENTITY: Self = Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    #[expect(clippy::many_single_char_names, reason = "the six letters of SVG's matrix")]
    const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    pub(super) const fn translate(x: f64, y: f64) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, x, y)
    }

    /// This matrix applied after `inner`: the matrix of an element is its parent's applied after its own.
    pub(super) fn after(self, inner: Self) -> Self {
        Self::new(
            self.a * inner.a + self.c * inner.b,
            self.b * inner.a + self.d * inner.b,
            self.a * inner.c + self.c * inner.d,
            self.b * inner.c + self.d * inner.d,
            self.a * inner.e + self.c * inner.f + self.e,
            self.b * inner.e + self.d * inner.f + self.f,
        )
    }

    pub(super) fn apply(self, point: Point) -> Point {
        Point::new(
            self.a * point.x + self.c * point.y + self.e,
            self.b * point.x + self.d * point.y + self.f,
        )
    }

    /// The number a stroke width is multiplied by, if the matrix is a turn, a mirror and an even scale (and a move);
    /// `None` if it stretches one way or skews.
    pub(super) fn even_scale(self) -> Option<f64> {
        let (across, down) = (
            (self.a * self.a + self.b * self.b).sqrt(),
            (self.c * self.c + self.d * self.d).sqrt(),
        );
        let square = across * down;
        let even = (across - down).abs() <= EVEN * square.max(f64::MIN_POSITIVE);
        let right_angle = (self.a * self.c + self.b * self.d).abs() <= EVEN * square.max(f64::MIN_POSITIVE);
        (even && right_angle).then_some(across)
    }

    /// The matrix a `transform` attribute describes.
    ///
    /// # Errors
    /// What is wrong with it, as a phrase to follow the attribute's name.
    pub(super) fn parse(text: &str) -> Result<Self, String> {
        let mut total = Self::IDENTITY;
        let mut rest = text.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        while !rest.is_empty() {
            let (name, after_name) = rest
                .split_once('(')
                .ok_or_else(|| format!("is not a list of transforms, got {text:?}"))?;
            let (arguments, after) = after_name
                .split_once(')')
                .ok_or_else(|| format!("is missing a closing bracket, got {text:?}"))?;
            let numbers = arguments
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|word| !word.is_empty())
                .map(|word| {
                    word.parse::<f64>()
                        .ok()
                        .filter(|number| number.is_finite())
                        .ok_or_else(|| format!("holds {word:?}, which is not a number"))
                })
                .collect::<Result<Vec<f64>, String>>()?;
            let name = name.trim();
            total = total.after(Self::one(name, &numbers)?);
            rest = after.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        }
        Ok(total)
    }

    /// The matrix of one transform, `name(numbers)`.
    #[expect(clippy::many_single_char_names, reason = "the six letters of SVG's matrix")]
    fn one(name: &str, numbers: &[f64]) -> Result<Self, String> {
        let wrong = |wanted: &str| format!("{name}() takes {wanted}, got {} numbers", numbers.len());
        Ok(match (name, numbers) {
            ("matrix", &[a, b, c, d, e, f]) => Self::new(a, b, c, d, e, f),
            ("matrix", _) => return Err(wrong("6 numbers")),
            ("translate", &[x]) => Self::translate(x, 0.0),
            ("translate", &[x, y]) => Self::translate(x, y),
            ("scale", &[x]) => Self::new(x, 0.0, 0.0, x, 0.0, 0.0),
            ("scale", &[x, y]) => Self::new(x, 0.0, 0.0, y, 0.0, 0.0),
            ("translate" | "scale", _) => return Err(wrong("1 or 2 numbers")),
            ("rotate", &[angle]) => Self::turn(angle),
            ("rotate", &[angle, x, y]) => Self::translate(x, y)
                .after(Self::turn(angle))
                .after(Self::translate(-x, -y)),
            ("rotate", _) => return Err(wrong("1 or 3 numbers")),
            ("skewX", &[angle]) => Self::new(1.0, 0.0, tan(angle.to_radians()), 1.0, 0.0, 0.0),
            ("skewY", &[angle]) => Self::new(1.0, tan(angle.to_radians()), 0.0, 1.0, 0.0, 0.0),
            ("skewX" | "skewY", _) => return Err(wrong("1 number")),
            _ => {
                return Err(format!(
                    "holds {name:?}, which is not a transform (matrix, translate, scale, rotate, skewX, skewY)"
                ));
            }
        })
    }

    /// A turn of `degrees` clockwise on the screen, about the origin.
    fn turn(degrees: f64) -> Self {
        let (sin, cos) = sin_cos_deg(degrees);
        Self::new(cos, sin, -sin, cos, 0.0, 0.0)
    }
}
