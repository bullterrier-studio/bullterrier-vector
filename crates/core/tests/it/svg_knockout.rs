//! The SVG `Drawing::to_svg` writes for a knockout: the mask over the group of what is below it, read back with an XML
//! parser against markup written by hand, and the picture it makes against the pixels, drawn by a second renderer
//! (`resvg`).

use crate::common::svg_oracle::{Element, body, element, numbers, svg_agrees};
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Blend, Cap, Color, Drawing, Error, Path, Shadow, Size, Stroke, Values};

const RED: Color = Color::hex(0xff_00_00);
const GREEN: Color = Color::hex(0x00_ff_00);
const BLUE: Color = Color::hex(0x00_00_ff);

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

fn rect(left: f64, top: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(left, top, width, height, 0.0)
}

fn filled(left: f64, top: f64, width: f64, height: f64, color: Color) -> Result<Drawing, Error> {
    Ok(Drawing::from(rect(left, top, width, height)?).fill(color))
}

fn hole(left: f64, top: f64, width: f64, height: f64) -> Result<Drawing, Error> {
    Ok(filled(left, top, width, height, BLUE)?.knockout())
}

/// The `<mask>` called `id` over the region from `origin` on both axes, `extent` wide and high: white all over, then
/// `holes` in black.
fn mask(id: &str, (origin, extent): (&str, &str), holes: Vec<Element>) -> Element {
    let mut children = vec![element(
        "rect",
        &[
            ("x", origin),
            ("y", origin),
            ("width", extent),
            ("height", extent),
            ("fill", "#ffffff"),
        ],
        vec![],
    )];
    children.extend(holes);
    element(
        "mask",
        &[
            ("id", id),
            ("maskUnits", "userSpaceOnUse"),
            ("x", origin),
            ("y", origin),
            ("width", extent),
            ("height", extent),
        ],
        children,
    )
}

/// The mask over the canvas of 100 and nothing more.
const CANVAS: (&str, &str) = ("0", "100");

/// A path of data `d` painted black, as the mask paints a knockout; `more` are its other attributes.
fn black(d: &str, more: &[(&str, &str)]) -> Element {
    let mut attributes = vec![("d", d), ("fill", "#000000")];
    attributes.extend_from_slice(more);
    element("path", &attributes, vec![])
}

fn red_square() -> Element {
    element(
        "path",
        &[("d", "M0 0 L100 0 L100 100 L0 100 Z"), ("fill", "#ff0000")],
        vec![],
    )
}

/// The `<g>` that `mask` `id` cuts.
fn group(id: &str, children: Vec<Element>) -> Element {
    element("g", &[("mask", &format!("url(#{id})"))], children)
}

const HOLE: &str = "M30 30 L70 30 L70 70 L30 70 Z";

// The markup --------------------------------------------------------------------------------------------------------

#[test]
fn what_is_below_a_knockout_is_a_group_with_a_mask_that_has_the_knockout_in_black() -> Outcome {
    let drawing = Drawing::new([filled(0.0, 0.0, 100.0, 100.0, RED)?, hole(30.0, 30.0, 40.0, 40.0)?]);
    let wanted = vec![
        mask("knockout-1", CANVAS, vec![black(HOLE, &[])]),
        group("knockout-1", vec![red_square()]),
    ];
    assert_eq!(body(&drawing, 100.0)?, wanted);
    Ok(())
}

#[test]
fn a_stroke_is_black_in_the_mask_with_the_stroke_attributes_of_the_shape() -> Outcome {
    // a stroke 10 wide with butt ends and no fill
    let ring = Drawing::from(rect(30.0, 30.0, 40.0, 40.0)?)
        .fill(Color::NONE)
        .stroke(Stroke::new(color("#0000ff20")?).width(10.0).cap(Cap::Butt))
        .knockout();
    let wanted = vec![
        mask(
            "knockout-1",
            CANVAS,
            vec![element(
                "path",
                &[
                    ("d", HOLE),
                    ("fill", "none"),
                    ("stroke", "#000000"),
                    ("stroke-width", "10"),
                    ("stroke-linejoin", "round"),
                    ("stroke-linecap", "butt"),
                    ("stroke-miterlimit", "4"),
                ],
                vec![],
            )],
        ),
        group("knockout-1", vec![red_square()]),
    ];
    assert_eq!(
        body(&Drawing::new([filled(0.0, 0.0, 100.0, 100.0, RED)?, ring]), 100.0)?,
        wanted
    );
    Ok(())
}

#[test]
fn the_alpha_the_opacity_and_the_blend_of_the_knockout_are_not_written() -> Outcome {
    let faint = filled(30.0, 30.0, 40.0, 40.0, color("#0000ff10")?)?
        .opacity(0.1)
        .blend(Blend::Multiply)
        .shadow(Shadow::new().dx(5.0).blur(0.0))
        .knockout();
    let wanted = vec![
        mask("knockout-1", CANVAS, vec![black(HOLE, &[])]),
        group("knockout-1", vec![red_square()]),
    ];
    assert_eq!(
        body(&Drawing::new([filled(0.0, 0.0, 100.0, 100.0, RED)?, faint]), 100.0)?,
        wanted
    );
    Ok(())
}

#[test]
fn a_shape_after_the_knockout_is_outside_the_group_and_the_mask_is_named_by_the_knockouts_index() -> Outcome {
    let drawing = Drawing::new([
        filled(0.0, 0.0, 100.0, 100.0, RED)?,
        hole(30.0, 30.0, 40.0, 40.0)?,
        filled(40.0, 40.0, 20.0, 20.0, GREEN)?,
    ]);
    let green = element(
        "path",
        &[("d", "M40 40 L60 40 L60 60 L40 60 Z"), ("fill", "#00ff00")],
        vec![],
    );
    let wanted = vec![
        mask("knockout-1", CANVAS, vec![black(HOLE, &[])]),
        group("knockout-1", vec![red_square()]),
        green,
    ];
    assert_eq!(body(&drawing, 100.0)?, wanted);
    Ok(())
}

#[test]
fn knockouts_in_a_row_share_one_mask_and_ones_with_a_shape_between_nest() -> Outcome {
    let second = "M10 10 L20 10 L20 20 L10 20 Z";
    let in_a_row = Drawing::new([
        filled(0.0, 0.0, 100.0, 100.0, RED)?,
        hole(30.0, 30.0, 40.0, 40.0)?,
        hole(10.0, 10.0, 10.0, 10.0)?,
    ]);
    assert_eq!(
        body(&in_a_row, 100.0)?,
        vec![
            mask("knockout-1", CANVAS, vec![black(HOLE, &[]), black(second, &[])]),
            group("knockout-1", vec![red_square()]),
        ]
    );
    // a shape between them is below the second: it is in the second's group, beside the first's
    let blue = element(
        "path",
        &[("d", "M0 0 L100 0 L100 50 L0 50 Z"), ("fill", "#0000ff")],
        vec![],
    );
    let nested = Drawing::new([
        filled(0.0, 0.0, 100.0, 100.0, RED)?,
        hole(30.0, 30.0, 40.0, 40.0)?,
        filled(0.0, 0.0, 100.0, 50.0, BLUE)?,
        hole(10.0, 10.0, 10.0, 10.0)?,
    ]);
    assert_eq!(
        body(&nested, 100.0)?,
        vec![
            mask("knockout-3", CANVAS, vec![black(second, &[])]),
            group(
                "knockout-3",
                vec![
                    mask("knockout-1", CANVAS, vec![black(HOLE, &[])]),
                    group("knockout-1", vec![red_square()]),
                    blue,
                ]
            ),
        ]
    );
    Ok(())
}

#[test]
fn the_mask_covers_the_canvas_and_the_farthest_a_shadow_below_reaches() -> Outcome {
    // a blur of 10 is a deviation of 5, reaching 3 deviations and the offset; a hard copy reaches its offset; the
    // farthest shadow in the group sets the region, and the shadow of the knockout itself sets nothing
    let blurred = |dx: f64, dy: f64| {
        filled(20.0, 20.0, 40.0, 30.0, RED).map(|d| d.shadow(Shadow::new().dx(dx).dy(dy).blur(10.0)))
    };
    let hard =
        |dx: f64, dy: f64| filled(20.0, 20.0, 40.0, 30.0, RED).map(|d| d.shadow(Shadow::new().dx(dx).dy(dy).blur(0.0)));
    let far_hole = hole(30.0, 30.0, 40.0, 40.0)?.shadow(Shadow::new().dy(90.0).blur(0.0));
    let rows = [
        ("a blurred shadow", vec![blurred(0.0, 20.0)?], ("-35", "170")),
        ("a hard shadow", vec![hard(-30.0, 5.0)?], ("-30", "160")),
        (
            "the farthest of two",
            vec![hard(-30.0, 5.0)?, blurred(0.0, 0.0)?],
            ("-30", "160"),
        ),
        (
            "the shadow of the knockout",
            vec![filled(20.0, 20.0, 40.0, 30.0, RED)?],
            ("0", "100"),
        ),
    ];
    for (case, below, (origin, extent)) in rows {
        let drawing = Drawing::new(below.into_iter().chain([far_hole.clone()]));
        let found = body(&drawing, 100.0)?;
        let region = found
            .first()
            .map(|mask| {
                let attribute = |name: &str| mask.attributes.get(name).cloned().unwrap_or_default();
                (attribute("x"), attribute("y"), attribute("width"), attribute("height"))
            })
            .ok_or("no element")?;
        let wanted = (
            origin.to_owned(),
            origin.to_owned(),
            extent.to_owned(),
            extent.to_owned(),
        );
        assert_eq!(region, wanted, "{case}");
    }
    Ok(())
}

#[test]
fn at_twice_the_size_every_length_of_the_mask_is_exactly_twice() -> Outcome {
    let drawing = Drawing::new([
        filled(20.0, 20.0, 40.0, 30.0, RED)?.shadow(Shadow::new().dx(3.0).dy(-6.5).blur(9.0)),
        hole(30.0, 30.0, 40.0, 40.0)?,
    ])
    .with_size(100.0, 100.0);
    let (small, large) = (numbers(&body(&drawing, 100.0)?), numbers(&body(&drawing, 200.0)?));
    assert!(
        small.len() >= 8,
        "the region of the mask and of its rectangle: {small:?}"
    );
    assert_eq!(large, small.iter().map(|n| n * 2.0).collect::<Vec<_>>());
    Ok(())
}

#[test]
fn a_knockout_that_removes_nothing_writes_no_mask() -> Outcome {
    let below = || filled(0.0, 0.0, 100.0, 100.0, RED);
    let plain = body(&below()?, 100.0)?;
    let nothing_painted = Drawing::from(rect(30.0, 30.0, 40.0, 40.0)?)
        .fill(Color::NONE)
        .knockout();
    let no_path = Drawing::from(Path::new()).fill(BLUE).knockout();
    for (case, knocker) in [("nothing painted", nothing_painted), ("an empty path", no_path)] {
        assert_eq!(body(&Drawing::new([below()?, knocker]), 100.0)?, plain, "{case}");
    }
    // and with nothing below there is nothing to cut
    assert_eq!(body(&hole(30.0, 30.0, 40.0, 40.0)?, 100.0)?, vec![]);
    Ok(())
}

/// Every `id` in the elements, in order.
fn ids(elements: &[Element]) -> Vec<String> {
    elements
        .iter()
        .flat_map(|element| {
            element
                .attributes
                .get("id")
                .cloned()
                .into_iter()
                .chain(ids(&element.children))
        })
        .collect()
}

#[test]
fn the_ids_of_masks_and_of_filters_are_each_one_in_the_document() -> Outcome {
    let drawing = Drawing::new([
        filled(0.0, 0.0, 50.0, 50.0, RED)?.shadow(Shadow::new().blur(6.0)),
        hole(10.0, 10.0, 10.0, 10.0)?,
        filled(30.0, 30.0, 50.0, 50.0, GREEN)?.shadow(Shadow::new().blur(6.0)),
        hole(40.0, 40.0, 10.0, 10.0)?,
        filled(60.0, 0.0, 30.0, 30.0, BLUE)?,
        hole(70.0, 10.0, 10.0, 10.0)?,
    ]);
    let found = ids(&body(&drawing, 100.0)?);
    let mut sorted = found.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), found.len(), "an id is used twice: {found:?}");
    assert_eq!(
        sorted,
        ["knockout-1", "knockout-3", "knockout-5", "shadow-0", "shadow-2"]
    );
    Ok(())
}

/// `layers` times a square and a hole in it, in a drawing.
fn layered(layers: usize) -> Result<Drawing, Error> {
    let mut shapes = Vec::new();
    for _ in 0..layers {
        shapes.push(filled(0.0, 0.0, 100.0, 100.0, RED)?);
        shapes.push(hole(30.0, 30.0, 40.0, 40.0)?);
    }
    Ok(Drawing::new(shapes))
}

#[test]
fn knockouts_nest_to_128_layers_and_a_129th_is_refused_naming_its_shape() -> Outcome {
    // a document nested past 256 levels is refused by common XML readers; every layer is one level
    let size = Size::px(100.0, 100.0);
    let fits = layered(128)?.to_svg(size, &Values::new())?;
    assert_eq!(fits.matches("<mask ").count(), 128);
    let (name, message) = parameter_refusal(layered(129)?.to_svg(size, &Values::new()))?;
    assert!(
        name == "shape 257" && message.contains("MAX_KNOCKOUT_LAYERS 128"),
        "{name}: {message}"
    );
    // knockouts in a row are one layer, however many
    let mut row = vec![filled(0.0, 0.0, 100.0, 100.0, RED)?];
    row.extend(
        (0..300)
            .map(|_| hole(30.0, 30.0, 40.0, 40.0))
            .collect::<Result<Vec<_>, _>>()?,
    );
    assert_eq!(
        Drawing::new(row)
            .to_svg(size, &Values::new())?
            .matches("<mask ")
            .count(),
        1
    );
    Ok(())
}

// The picture -------------------------------------------------------------------------------------------------------

#[test]
fn the_svg_shows_what_the_pixels_show() -> Outcome {
    let paper = Drawing::from(rect(0.0, 0.0, 120.0, 120.0)?).fill(color("#e0d8c8")?);
    let disc = |x: f64, y: f64, r: f64| Path::circle((x, y), r);
    let rows = [
        (
            "a disc cut out of a square",
            Drawing::new([
                filled(10.0, 10.0, 100.0, 100.0, color("#2a5bff")?)?,
                Drawing::from(disc(60.0, 60.0, 30.0)?).fill(RED).knockout(),
            ]),
        ),
        (
            "a stroke cut out of a translucent square",
            Drawing::new([
                filled(10.0, 10.0, 100.0, 100.0, color("#2a5bff90")?)?,
                Drawing::from(disc(60.0, 60.0, 30.0)?)
                    .fill(Color::NONE)
                    .stroke(Stroke::new(RED).width(12.0))
                    .knockout(),
            ]),
        ),
        (
            "a fill and a stroke, over a shape drawn after",
            Drawing::new([
                filled(10.0, 10.0, 100.0, 100.0, color("#2a5bff")?)?,
                Drawing::from(disc(45.0, 60.0, 25.0)?)
                    .fill(RED)
                    .stroke(Stroke::new(GREEN).width(10.0))
                    .knockout(),
                Drawing::from(disc(75.0, 60.0, 20.0)?).fill(color("#ffd400")?),
            ]),
        ),
        (
            "a blurred shadow cut",
            Drawing::new([
                filled(20.0, 20.0, 50.0, 50.0, color("#ff4f9a")?)?.shadow(Shadow::new().dx(10.0).dy(14.0).blur(16.0)),
                Drawing::from(disc(60.0, 60.0, 22.0)?).fill(RED).knockout(),
            ]),
        ),
        (
            "knockouts with a shape between",
            Drawing::new([
                filled(10.0, 10.0, 100.0, 100.0, RED)?,
                filled(20.0, 20.0, 30.0, 30.0, BLUE)?.knockout(),
                filled(0.0, 0.0, 120.0, 50.0, color("#2a5bff")?)?,
                filled(40.0, 40.0, 30.0, 30.0, BLUE)?.knockout(),
            ]),
        ),
        (
            "multiply after a knockout",
            Drawing::new([
                filled(10.0, 10.0, 100.0, 100.0, color("#ffd400")?)?,
                Drawing::from(disc(60.0, 60.0, 30.0)?).fill(RED).knockout(),
                filled(30.0, 30.0, 60.0, 60.0, color("#ff4f9a")?)?.blend(Blend::Multiply),
            ]),
        ),
    ];
    for (case, drawing) in rows {
        let drawing = Drawing::new([paper.clone(), drawing]);
        let agreement = svg_agrees(&drawing, 120, 120)?;
        assert!(
            agreement.interior <= 4 && agreement.edge_over_40 * 100 <= agreement.edge_pixels,
            "{case}: {agreement:?}"
        );
    }
    Ok(())
}
