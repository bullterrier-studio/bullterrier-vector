//! The rows of the style verbs: paint, stroke, opacity, blend, shadow, halftone, knockout, clip and mask.

use super::rows::{Row, fade, offset_square, square, unknown};
use bullterrier_vector::{Blend, Color, Deg, Drawing, Family, Halftone, Mark, Shadow, Stroke};

#[expect(clippy::too_many_lines, reason = "a table: one row per verb")]
pub(super) fn rows() -> Vec<Row> {
    vec![
        Row {
            verb: "fill",
            sample: || Ok(square()?.fill(fade()?)),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "stroke",
            sample: || Ok(square()?.stroke(Stroke::new(fade()?).width(3.0))),
            with: Some(|argument, value| match argument {
                "width" => Ok(square()?.stroke(Stroke::new(fade()?).width(value))),
                "miter_limit" => Ok(square()?.stroke(Stroke::new(fade()?).miter_limit(value))),
                _ => Err(unknown("stroke", argument)),
            }),
            ranges: &[
                ("width", 2.0, true),
                ("width", 1e-9, true),
                ("width", 0.0, false),
                ("width", -1.0, false),
                ("width", f64::NAN, false),
                ("miter_limit", 1.0, true),
                ("miter_limit", 1e6, true),
                ("miter_limit", 0.99, false),
                ("miter_limit", f64::NAN, false),
                ("miter_limit", f64::INFINITY, false),
            ],
            geometry: None,
        },
        Row {
            verb: "opacity",
            sample: || Ok(square()?.opacity(0.5)),
            with: Some(|argument, value| match argument {
                "alpha" => Ok(square()?.opacity(value)),
                _ => Err(unknown("opacity", argument)),
            }),
            ranges: &[
                ("alpha", 0.0, true),
                ("alpha", 0.5, true),
                ("alpha", 1.0, true),
                ("alpha", 1.000_000_1, false),
                ("alpha", -1e-9, false),
                ("alpha", f64::NAN, false),
                ("alpha", f64::INFINITY, false),
            ],
            geometry: None,
        },
        Row {
            verb: "blend",
            sample: || Ok(square()?.blend(Blend::ColorDodge)),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "shadow",
            sample: || Ok(square()?.shadow(Shadow::new().dy(4.0).blur(6.0))),
            with: Some(|argument, value| match argument {
                "dx" => Ok(square()?.shadow(Shadow::new().dx(value))),
                "dy" => Ok(square()?.shadow(Shadow::new().dy(value))),
                "blur" => Ok(square()?.shadow(Shadow::new().blur(value))),
                _ => Err(unknown("shadow", argument)),
            }),
            ranges: &[
                ("dx", -1e300, true),
                ("dx", f64::NAN, false),
                ("dx", f64::INFINITY, false),
                ("dy", 1e300, true),
                ("dy", f64::NEG_INFINITY, false),
                ("blur", 0.0, true),
                ("blur", 16.0, true),
                ("blur", -1e-9, false),
                ("blur", f64::NAN, false),
                ("blur", f64::INFINITY, false),
            ],
            geometry: None,
        },
        Row {
            verb: "halftone",
            sample: || Ok(square()?.halftone(Halftone::new(Color::hex(0), 0.5).spacing(6.0))),
            with: Some(|argument, value| {
                let dots = Halftone::new(Color::hex(0), 0.5);
                match argument {
                    "tone" => Ok(square()?.halftone(Halftone::new(Color::hex(0), value))),
                    "spacing" => Ok(square()?.halftone(dots.spacing(value))),
                    "angle" => Ok(square()?.halftone(dots.angle(Deg(value)))),
                    "aspect" => Ok(square()?.halftone(dots.shape(Mark::Ellipse).aspect(value))),
                    "amplitude" => Ok(square()?.halftone(dots.family(Family::Waves).amplitude(value))),
                    "wavelength" => Ok(square()?.halftone(dots.family(Family::Waves).wavelength(value))),
                    _ => Err(unknown("halftone", argument)),
                }
            }),
            ranges: &[
                ("tone", 0.0, true),
                ("tone", 1.0, true),
                ("tone", -1e-9, false),
                ("tone", 1.000_000_001, false),
                ("tone", f64::NAN, false),
                ("spacing", 1e300, true),
                ("spacing", 0.0, false),
                ("spacing", -1.0, false),
                ("spacing", f64::INFINITY, false),
                ("spacing", f64::NAN, false),
                ("angle", -1e300, true),
                ("angle", f64::NAN, false),
                ("angle", f64::INFINITY, false),
                ("aspect", 1.0, true),
                ("aspect", 1e30, true),
                ("aspect", 0.999_999_999, false),
                ("aspect", f64::NAN, false),
                ("aspect", f64::INFINITY, false),
                ("amplitude", -1e50, true),
                ("amplitude", f64::NAN, false),
                ("amplitude", f64::INFINITY, false),
                ("wavelength", 1e50, true),
                ("wavelength", 0.0, false),
                ("wavelength", -1.0, false),
                ("wavelength", f64::INFINITY, false),
                ("wavelength", f64::NAN, false),
            ],
            geometry: None,
        },
        Row {
            verb: "knockout",
            sample: || Ok(square()?.knockout()),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "clip",
            sample: || Ok(square()?.clip(offset_square()?)),
            with: None,
            ranges: &[],
            geometry: None,
        },
        Row {
            verb: "mask",
            sample: || Ok(square()?.mask(Drawing::from(offset_square()?).fill(Color::hex(0x80_80_80)))),
            with: None,
            ranges: &[],
            geometry: None,
        },
    ]
}
