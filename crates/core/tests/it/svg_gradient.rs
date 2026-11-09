//! The SVG `Drawing::to_svg` writes for a gradient fill or stroke: the gradient element read back with an XML parser
//! against attributes written by hand, where it lies at another scale, how a shadow tints it, and the picture it makes
//! against the pixels, drawn by a second renderer (`resvg`).

use crate::common::Outcome;
use crate::common::svg_oracle::{Element, body, element, svg_agrees};
use bullterrier_vector::{Blend, Cap, Color, Drawing, Error, Gradient, Halftone, Path, Shadow, Stroke};

const RED: Color = Color::hex(0xff_00_00);
const BLUE: Color = Color::hex(0x00_00_ff);

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

fn rect(left: f64, top: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(left, top, width, height, 0.0)
}

fn stop(offset: &str, attributes: &[(&str, &str)]) -> Element {
    let mut all = vec![("offset", offset)];
    all.extend_from_slice(attributes);
    element("stop", &all, vec![])
}

// The markup --------------------------------------------------------------------------------------------------------

#[test]
fn a_gradient_fill_is_a_linear_gradient_in_user_space_written_before_the_path_that_uses_it() -> Outcome {
    let half = (128.0_f64 / 255.0).to_string();
    let drawing = Drawing::from(rect(10.0, 10.0, 80.0, 80.0)?).fill(Gradient::linear(
        (0.0, 5.0),
        (0.0, 95.0),
        [(0.0, RED), (0.25, color("#00ff00")?), (1.0, color("#0000ff80")?)],
    )?);
    let wanted = vec![
        element(
            "linearGradient",
            &[
                ("id", "gradient-0-fill"),
                ("gradientUnits", "userSpaceOnUse"),
                ("x1", "0"),
                ("y1", "5"),
                ("x2", "0"),
                ("y2", "95"),
            ],
            vec![
                stop("0", &[("stop-color", "#ff0000")]),
                stop("0.25", &[("stop-color", "#00ff00")]),
                stop("1", &[("stop-color", "#0000ff"), ("stop-opacity", &half)]),
            ],
        ),
        element(
            "path",
            &[
                ("d", "M10 10 L90 10 L90 90 L10 90 Z"),
                ("fill", "url(#gradient-0-fill)"),
            ],
            vec![],
        ),
    ];
    assert_eq!(body(&drawing, 100.0)?, wanted);
    Ok(())
}

#[test]
fn a_gradient_stroke_is_a_radial_gradient_named_for_its_shape_and_its_use() -> Outcome {
    let drawing = Drawing::new([
        Drawing::from(rect(0.0, 0.0, 10.0, 10.0)?).fill(RED),
        Drawing::from(Path::line((10.0, 50.0), (90.0, 50.0))?).stroke(
            Stroke::new(Gradient::radial((50.0, 40.0), 30.0, [(0.0, RED), (1.0, BLUE)])?)
                .width(8.0)
                .cap(Cap::Butt),
        ),
    ]);
    let wanted_gradient = element(
        "radialGradient",
        &[
            ("id", "gradient-1-stroke"),
            ("gradientUnits", "userSpaceOnUse"),
            ("cx", "50"),
            ("cy", "40"),
            ("r", "30"),
        ],
        vec![
            stop("0", &[("stop-color", "#ff0000")]),
            stop("1", &[("stop-color", "#0000ff")]),
        ],
    );
    let wanted_path = element(
        "path",
        &[
            ("d", "M10 50 L90 50"),
            ("fill", "none"),
            ("stroke", "url(#gradient-1-stroke)"),
            ("stroke-width", "8"),
            ("stroke-linejoin", "round"),
            ("stroke-linecap", "butt"),
            ("stroke-miterlimit", "4"),
        ],
        vec![],
    );
    let found = body(&drawing, 100.0)?;
    assert_eq!(found.len(), 3, "{found:?}");
    assert_eq!((&found[1], &found[2]), (&wanted_gradient, &wanted_path));
    Ok(())
}

#[test]
fn a_fill_and_a_stroke_each_have_a_gradient_of_their_own() -> Outcome {
    let fade = Gradient::linear((0.0, 0.0), (100.0, 0.0), [(0.0, RED), (1.0, BLUE)])?;
    let across = Gradient::linear((0.0, 0.0), (0.0, 100.0), [(0.0, BLUE), (1.0, RED)])?;
    let drawing = Drawing::from(rect(10.0, 10.0, 80.0, 80.0)?).fill(fade).stroke(across);
    let found = body(&drawing, 100.0)?;
    let ids: Vec<&str> = found
        .iter()
        .filter_map(|e| e.attributes.get("id").map(String::as_str))
        .collect();
    assert_eq!(ids, ["gradient-0-fill", "gradient-0-stroke"]);
    let path = found.last().ok_or("no path")?;
    assert_eq!(path.attributes["fill"], "url(#gradient-0-fill)");
    assert_eq!(path.attributes["stroke"], "url(#gradient-0-stroke)");
    Ok(())
}

#[test]
fn the_geometry_moves_and_grows_with_the_frame_as_a_path_does() -> Outcome {
    // a frame of 100 in an output of 250 and 200: scale 2, centred across by 25
    let drawing = Drawing::from(rect(10.0, 10.0, 80.0, 80.0)?)
        .fill(Gradient::radial((10.0, 20.0), 30.0, [(0.0, RED), (1.0, BLUE)])?)
        .with_size(100.0, 100.0);
    let svg = drawing.to_svg(
        bullterrier_vector::Size::px(250.0, 200.0),
        &bullterrier_vector::Values::new(),
    )?;
    assert!(svg.contains(r#"cx="45" cy="40" r="60""#), "{svg}");
    Ok(())
}

#[test]
fn a_hard_shadow_of_a_gradient_is_a_copy_in_the_tint_that_follows_the_gradients_alpha() -> Outcome {
    let drawing = Drawing::from(rect(10.0, 10.0, 40.0, 40.0)?)
        .fill(Gradient::linear(
            (10.0, 0.0),
            (50.0, 0.0),
            [(0.0, color("#ff000000")?), (1.0, RED)],
        )?)
        .shadow(Shadow::new().dx(5.0).dy(7.0).blur(0.0).color(color("#11223380")?));
    let found = body(&drawing, 100.0)?;
    let tinted = found
        .iter()
        .find(|e| e.attributes.get("id").is_some_and(|id| id == "gradient-0-shadow-fill"))
        .ok_or("no tinted gradient")?;
    // the line moved with the shadow, the stops in the tint with their own alphas
    assert_eq!(tinted.attributes["x1"], "15");
    assert_eq!(tinted.attributes["x2"], "55");
    assert_eq!(tinted.attributes["y1"], "7");
    let colours: Vec<&str> = tinted
        .children
        .iter()
        .map(|s| s.attributes["stop-color"].as_str())
        .collect();
    assert_eq!(colours, ["#112233", "#112233"]);
    assert_eq!(tinted.children[0].attributes["stop-opacity"], "0");
    assert!(!tinted.children[1].attributes.contains_key("stop-opacity"));
    Ok(())
}

// The picture -------------------------------------------------------------------------------------------------------

/// The paper and the gradients the rows below draw over it and with.
struct Kit {
    paper: Drawing,
    fade: Gradient,
    clear: Gradient,
    glow: Gradient,
    ring: Gradient,
}

fn kit() -> Result<Kit, Error> {
    let paper = Drawing::from(rect(0.0, 0.0, 120.0, 120.0)?).fill(color("#fff3d6")?);
    let fade = Gradient::linear(
        (20.0, 20.0),
        (100.0, 80.0),
        [(0.0, color("#ff0066")?), (0.6, color("#ffcc00")?), (1.0, BLUE)],
    )?;
    let clear = Gradient::linear(
        (20.0, 0.0),
        (100.0, 0.0),
        [(0.0, color("#ff000000")?), (1.0, color("#0000ffff")?)],
    )?;
    let glow = Gradient::radial(
        (60.0, 60.0),
        40.0,
        [(0.0, color("#ffffffff")?), (1.0, color("#00000000")?)],
    )?;
    let ring = Gradient::radial(
        (60.0, 60.0),
        50.0,
        [(0.0, color("#00aa44c0")?), (1.0, color("#aa0044c0")?)],
    )?;
    Ok(Kit {
        paper,
        fade,
        clear,
        glow,
        ring,
    })
}

/// An error unless every row paints something over the paper and the SVG shows what the pixels show.
fn agree(paper: &Drawing, rows: Vec<(&str, Drawing)>) -> Outcome {
    for (case, drawing) in rows {
        let drawing = Drawing::new([paper.clone(), drawing]);
        // the row must paint something over the paper, or two blank pictures would agree
        let size = bullterrier_vector::Size::px(120.0, 120.0);
        let values = bullterrier_vector::Values::new();
        assert!(
            drawing.to_rgba(size, &values)? != paper.to_rgba(size, &values)?,
            "{case}: paints nothing"
        );
        let agreement = svg_agrees(&drawing, 120, 120)?;
        assert!(
            agreement.interior <= 4 && agreement.edge_over_40 * 100 <= agreement.edge_pixels,
            "{case}: {agreement:?}"
        );
    }
    Ok(())
}

#[test]
fn the_svg_and_the_pixels_show_the_same_gradient_fills_and_strokes() -> Outcome {
    let Kit {
        paper,
        fade,
        clear,
        glow,
        ring,
    } = kit()?;
    let cross = Path::from_svg_path("M15 15 L105 105 L15 105 L105 15")?;
    let rows = vec![
        (
            "a gradient fill",
            Drawing::from(rect(10.0, 10.0, 100.0, 100.0)?).fill(fade.clone()),
        ),
        (
            "translucent stops of other colours",
            Drawing::from(rect(10.0, 10.0, 100.0, 100.0)?).fill(clear.clone()),
        ),
        (
            "a radial glow",
            Drawing::from(Path::circle((60.0, 60.0), 45.0)?).fill(glow.clone()),
        ),
        (
            "a gradient stroke",
            Drawing::from(rect(25.0, 25.0, 70.0, 70.0)?).stroke(Stroke::new(fade.clone()).width(12.0)),
        ),
        (
            "a fill and a stroke",
            Drawing::from(rect(25.0, 25.0, 70.0, 70.0)?)
                .fill(clear.clone())
                .stroke(Stroke::new(ring.clone()).width(10.0)),
        ),
        (
            "a self-crossing stroke of translucent stops",
            Drawing::from(cross.clone()).stroke(Stroke::new(ring.clone()).width(14.0)),
        ),
        (
            "round ends of translucent stops",
            Drawing::from(Path::from_svg_path("M20 60 L100 60")?)
                .stroke(Stroke::new(ring.clone()).width(24.0).cap(Cap::Round)),
        ),
    ];
    agree(&paper, rows)
}

#[test]
fn the_svg_and_the_pixels_show_the_same_shadows_and_layers_of_gradients() -> Outcome {
    let Kit {
        paper,
        fade,
        clear,
        ring,
        ..
    } = kit()?;
    let rows = vec![
        (
            "a blurred shadow follows the alpha",
            Drawing::from(rect(30.0, 30.0, 50.0, 50.0)?)
                .fill(clear.clone())
                .shadow(Shadow::new().dx(8.0).dy(10.0).blur(14.0).color(color("#000000d0")?)),
        ),
        (
            "a hard shadow follows the alpha",
            Drawing::from(rect(30.0, 30.0, 50.0, 50.0)?)
                .fill(clear.clone())
                .shadow(Shadow::new().dx(10.0).dy(12.0).blur(0.0).color(color("#000000a0")?)),
        ),
        (
            "a hard shadow of a gradient stroke",
            Drawing::from(rect(30.0, 30.0, 50.0, 50.0)?)
                .stroke(Stroke::new(ring.clone()).width(10.0))
                .shadow(Shadow::new().dx(10.0).dy(12.0).blur(0.0)),
        ),
        (
            "opacity over a gradient",
            Drawing::from(rect(20.0, 20.0, 80.0, 80.0)?)
                .fill(fade.clone())
                .stroke(Stroke::new(ring.clone()).width(10.0))
                .opacity(0.5),
        ),
        (
            "multiply of a gradient",
            Drawing::from(rect(20.0, 20.0, 80.0, 80.0)?)
                .fill(fade.clone())
                .blend(Blend::Multiply),
        ),
        (
            "a halftone with a gradient stroke",
            Drawing::from(rect(20.0, 20.0, 80.0, 80.0)?)
                .halftone(Halftone::new(RED, 0.5).pitch(10.0))
                .stroke(Stroke::new(fade.clone()).width(8.0)),
        ),
    ];
    agree(&paper, rows)
}

#[test]
fn a_knockout_with_a_gradient_cuts_as_one_with_a_colour_does() -> Outcome {
    let paper = Drawing::from(rect(0.0, 0.0, 120.0, 120.0)?).fill(color("#336699")?);
    let fade = Gradient::linear((20.0, 20.0), (100.0, 80.0), [(0.0, RED), (1.0, color("#0000ff20")?)])?;
    let cut = |paint: Drawing| Drawing::new([paper.clone(), paint.knockout()]);
    let with_gradient = cut(Drawing::from(rect(30.0, 30.0, 60.0, 60.0)?)
        .fill(fade.clone())
        .stroke(fade));
    let with_colour = cut(Drawing::from(rect(30.0, 30.0, 60.0, 60.0)?).fill(RED).stroke(RED));
    let (a, b) = (
        with_gradient.to_rgba(
            bullterrier_vector::Size::px(120.0, 120.0),
            &bullterrier_vector::Values::new(),
        )?,
        with_colour.to_rgba(
            bullterrier_vector::Size::px(120.0, 120.0),
            &bullterrier_vector::Values::new(),
        )?,
    );
    assert!(a == b, "the pixels of the two knockouts differ");
    assert_eq!(
        with_gradient.to_svg(
            bullterrier_vector::Size::px(120.0, 120.0),
            &bullterrier_vector::Values::new()
        )?,
        with_colour.to_svg(
            bullterrier_vector::Size::px(120.0, 120.0),
            &bullterrier_vector::Values::new()
        )?
    );
    Ok(())
}
