//! The separable blend modes beyond multiply, in the pixels, the SVG and the mesh. The oracle is the W3C Compositing and
//! Blending Level 1 formula of each mode, written here in f64: the backdrop and source colours are opaque greys taken
//! from a grid that crosses every branch of the formulas, so `B(cb, cs)` is the pixel where they overlap.

use crate::common::mesh::mesh_of;
use crate::common::pixels::render;
use crate::common::svg_oracle::svg_agrees;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Blend, Color, Drawing, Error, Inputs, Mesh, Path, Shadow, Size, Values};

/// A mode, its CSS word, and the W3C `B(cb, cs)` for one channel in 0 to 1.
type Mode = (Blend, &'static str, fn(f64, f64) -> f64);

fn screen(cb: f64, cs: f64) -> f64 {
    cb + cs - cb * cs
}

fn hard_light(cb: f64, cs: f64) -> f64 {
    if cs <= 0.5 {
        cb * 2.0 * cs
    } else {
        screen(cb, 2.0 * cs - 1.0)
    }
}

fn soft_light(cb: f64, cs: f64) -> f64 {
    if cs <= 0.5 {
        cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
    } else {
        let d = if cb <= 0.25 {
            ((16.0 * cb - 12.0) * cb + 4.0) * cb
        } else {
            cb.sqrt()
        };
        cb + (2.0 * cs - 1.0) * (d - cb)
    }
}

fn color_dodge(cb: f64, cs: f64) -> f64 {
    if cb == 0.0 {
        0.0
    } else if cs >= 1.0 {
        1.0
    } else {
        (cb / (1.0 - cs)).min(1.0)
    }
}

fn color_burn(cb: f64, cs: f64) -> f64 {
    if cb >= 1.0 {
        1.0
    } else if cs <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - cb) / cs).min(1.0)
    }
}

const MODES: [Mode; 10] = [
    (Blend::Screen, "screen", screen),
    // overlay is hard-light with the two swapped
    (Blend::Overlay, "overlay", |cb, cs| hard_light(cs, cb)),
    (Blend::Darken, "darken", f64::min),
    (Blend::Lighten, "lighten", f64::max),
    (Blend::ColorDodge, "color-dodge", color_dodge),
    (Blend::ColorBurn, "color-burn", color_burn),
    (Blend::HardLight, "hard-light", hard_light),
    (Blend::SoftLight, "soft-light", soft_light),
    (Blend::Difference, "difference", |cb, cs| (cb - cs).abs()),
    (Blend::Exclusion, "exclusion", |cb, cs| cb + cs - 2.0 * cb * cs),
];

/// Channel values on both sides of 0.25 and 0.5, and at the ends.
const GRID: [u8; 7] = [0, 63, 64, 127, 128, 200, 255];

fn grey(level: u8) -> Result<Color, Error> {
    Color::parse(&format!("#{level:02x}{level:02x}{level:02x}"))
}

fn square(left: f64, size: f64, fill: Color) -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::rect(left, left, size, size, 0.0)?).fill(fill))
}

/// A backdrop over the whole 12 pixels and a source over the middle 4 by 4, blended.
fn pair(cb: Color, cs: Color, blend: Blend) -> Result<Drawing, Error> {
    Ok(Drawing::new([
        square(0.0, 12.0, cb)?,
        square(4.0, 4.0, cs)?.blend(blend),
    ]))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a channel in 0 to 1 rounded to 0 to 255"
)]
fn level(value: f64) -> u8 {
    (value * 255.0).round() as u8
}

#[test]
fn a_blend_is_the_w3c_formula_of_its_mode_on_opaque_colours() -> Outcome {
    let mut most = 0;
    for (blend, word, formula) in MODES {
        assert_eq!(blend.name(), word);
        assert_eq!(Blend::parse(word)?, blend);
        for cb in GRID {
            for cs in GRID {
                let image = render(&pair(grey(cb)?, grey(cs)?, blend)?, 12, 12)?;
                let want = level(formula(f64::from(cb) / 255.0, f64::from(cs) / 255.0));
                let got = image.pixel(6, 6);
                let off = got[..3].iter().map(|&c| c.abs_diff(want)).max().unwrap_or(0);
                most = most.max(off);
                assert!(
                    off <= 1 && got[3] == 255,
                    "{word}: cb {cb} cs {cs}: wanted {want}, got {got:?}"
                );
                assert_eq!(image.pixel(1, 1), [cb, cb, cb, 255], "{word}: the backdrop alone");
            }
        }
    }
    assert!(most <= 1, "{most}");
    Ok(())
}

#[test]
fn a_translucent_source_mixes_the_blend_into_the_backdrop_by_its_alpha() -> Outcome {
    // W3C: the result is (1 - a) cb + a B(cb, cs), with a the alpha of the colour times the opacity
    for (blend, word, formula) in MODES {
        for (case, cb, cs, alpha, opacity) in [
            ("a colour alpha", 200_u8, 90_u8, 128_u8, 1.0),
            ("an opacity", 70, 220, 255, 0.5),
            ("both", 140, 30, 204, 0.5),
        ] {
            let source = Color::parse(&format!("#{cs:02x}{cs:02x}{cs:02x}{alpha:02x}"))?;
            let drawing = Drawing::new([
                square(0.0, 12.0, grey(cb)?)?,
                square(4.0, 4.0, source)?.opacity(opacity).blend(blend),
            ]);
            let got = render(&drawing, 12, 12)?.pixel(6, 6);
            let a = f64::from(alpha) / 255.0 * opacity;
            let cb_f = f64::from(cb) / 255.0;
            let want = level((1.0 - a) * cb_f + a * formula(cb_f, f64::from(cs) / 255.0));
            assert!(
                got[..3].iter().all(|&c| c.abs_diff(want) <= 2) && got[3] == 255,
                "{word}, {case}: wanted {want}, got {got:?}"
            );
        }
    }
    Ok(())
}

fn disc(x: f64, y: f64, radius: f64, fill: &str) -> Result<Drawing, Error> {
    Ok(Drawing::from(Path::circle((x, y), radius)?).fill(Color::parse(fill)?))
}

#[test]
fn the_svg_blends_as_the_pixels_do() -> Outcome {
    let paper =
        || Ok::<_, Error>(Drawing::from(Path::rect(0.0, 0.0, 120.0, 120.0, 0.0)?).fill(Color::parse("#e8a060")?));
    for (_, word, _) in MODES {
        let blend = Blend::parse(word)?;
        let rows = [
            ("plain", disc(60.0, 60.0, 30.0, "#3070c0")?.blend(blend)),
            ("opacity", disc(60.0, 60.0, 30.0, "#3070c0")?.opacity(0.6).blend(blend)),
            (
                "shadow",
                disc(50.0, 50.0, 25.0, "#3070c0")?
                    .shadow(
                        Shadow::new()
                            .dx(14.0)
                            .dy(10.0)
                            .blur(0.0)
                            .color(Color::parse("#802020c0")?),
                    )
                    .blend(blend),
            ),
        ];
        for (case, ink) in rows {
            let agreement = svg_agrees(&Drawing::new([paper()?, ink]), 120, 120)?;
            assert!(
                agreement.interior <= 4 && agreement.edge_over_40 * 100 <= agreement.edge_pixels,
                "{word}, {case}: {agreement:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_mesh_refuses_every_blend_it_has_no_range_for_naming_the_shape_and_the_mode() -> Outcome {
    let shape = |blend| -> Result<Drawing, Error> { Ok(square(0.0, 10.0, grey(90)?)?.blend(blend)) };
    for (blend, word, _) in MODES {
        let drawing = Drawing::new([shape(Blend::Normal)?, shape(blend)?]);
        let mut mesh = Mesh::new();
        let (name, message) = parameter_refusal(drawing.to_mesh(Size::px(20.0, 20.0), &Values::new(), &mut mesh))?;
        assert!(
            name == "blend" && message.contains(word) && message.contains("shape 1") && message.contains("to_rgba"),
            "{word}: {name}: {message}"
        );
        assert!(mesh.vertices().is_empty(), "{word}: the mesh is left as it was");
    }
    // the two it has: normal is range 0, multiply range 1
    for (blend, range) in [(Blend::Normal, 0), (Blend::Multiply, 1)] {
        let mesh = mesh_of(&shape(blend)?, 20.0, 20.0)?;
        assert_eq!(mesh.ranges()[2], range);
    }
    // a knockout ignores its blend, so it is not refused for it
    mesh_of(&shape(Blend::Screen)?.knockout(), 20.0, 20.0)?;
    Ok(())
}

#[test]
fn a_document_writes_the_css_word_and_reads_it_back() -> Outcome {
    for (blend, word, _) in MODES {
        let drawing = square(0.0, 10.0, grey(90)?)?.blend(blend);
        let text = drawing.to_json()?;
        assert!(text.contains(&format!("{{\"mode\":\"{word}\"}}")), "{word}: {text}");
        assert_eq!(Drawing::from_json(&text, &Inputs::new())?.to_json()?, text, "{word}");
        assert_eq!(
            Drawing::from_json(&text, &Inputs::new())?.shapes(&Values::new())?[0]
                .style()
                .blend(),
            blend,
            "{word}"
        );
    }
    Ok(())
}
