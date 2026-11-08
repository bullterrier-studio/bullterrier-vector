//! The SVG `Drawing::to_svg` writes for a shadow: the filter of a blurred one and the copy of a hard one, read back with an
//! XML parser against attributes written by hand, and the picture it makes against the pixels, drawn by a second
//! renderer (`resvg`).

use crate::common::svg_oracle::svg_agrees;
use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Blend, Cap, Color, Drawing, Error, Path, Shadow, Size, Stroke, Values};
use std::collections::BTreeMap;

const RED: Color = Color::hex(0xff_00_00);

type Attributes = BTreeMap<String, String>;

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

fn rect(left: f64, top: f64, width: f64, height: f64) -> Result<Path, Error> {
    Path::rect(left, top, width, height, 0.0)
}

fn expected(pairs: &[(&str, &str)]) -> Attributes {
    pairs
        .iter()
        .map(|&(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

/// One element of the SVG: its name, its attributes, and its children.
#[derive(Debug, PartialEq)]
struct Element {
    name: String,
    attributes: Attributes,
    children: Vec<Element>,
}

fn read(node: roxmltree::Node<'_, '_>) -> Element {
    Element {
        name: node.tag_name().name().to_owned(),
        attributes: node
            .attributes()
            .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()))
            .collect(),
        children: node.children().filter(roxmltree::Node::is_element).map(read).collect(),
    }
}

/// The elements under the root of the SVG of `drawing` at `side` by `side`.
fn body(drawing: &Drawing, side: f64) -> Result<Vec<Element>, Box<dyn std::error::Error>> {
    let text = drawing.to_svg(Size::px(side, side), &Values::new())?;
    let document = roxmltree::Document::parse(&text)?;
    Ok(read(document.root_element()).children)
}

fn element(name: &str, attributes: &[(&str, &str)], children: Vec<Element>) -> Element {
    Element {
        name: name.to_owned(),
        attributes: expected(attributes),
        children,
    }
}

// The markup --------------------------------------------------------------------------------------------------------

#[test]
fn a_blurred_shadow_is_a_filter_on_a_group_around_the_shape() -> Outcome {
    // a deviation of 5 (blur 10), moved 4 and 6, tinted #123456 at 128/255: the filter's region is the canvas and the
    // reach of the shadow, 3 * 5 + 6 = 21 on every side
    let drawing = Drawing::from(rect(20.0, 20.0, 40.0, 30.0)?)
        .fill(RED)
        .shadow(Shadow::new().dx(4.0).dy(6.0).blur(10.0).color(color("#12345680")?));
    let strength = (128.0_f64 / 255.0).to_string();
    let wanted = vec![
        element(
            "filter",
            &[
                ("id", "shadow-0"),
                ("filterUnits", "userSpaceOnUse"),
                ("x", "-21"),
                ("y", "-21"),
                ("width", "142"),
                ("height", "142"),
                ("color-interpolation-filters", "sRGB"),
            ],
            vec![
                element(
                    "feGaussianBlur",
                    &[("in", "SourceAlpha"), ("stdDeviation", "5")],
                    vec![],
                ),
                element("feOffset", &[("dx", "4"), ("dy", "6"), ("result", "b")], vec![]),
                element(
                    "feFlood",
                    &[("flood-color", "#123456"), ("flood-opacity", &strength)],
                    vec![],
                ),
                element(
                    "feComposite",
                    &[("in2", "b"), ("operator", "in"), ("result", "s")],
                    vec![],
                ),
                element(
                    "feMerge",
                    &[],
                    vec![
                        element("feMergeNode", &[("in", "s")], vec![]),
                        element("feMergeNode", &[("in", "SourceGraphic")], vec![]),
                    ],
                ),
            ],
        ),
        element(
            "g",
            &[("filter", "url(#shadow-0)")],
            vec![element(
                "path",
                &[("d", "M20 20 L60 20 L60 50 L20 50 Z"), ("fill", "#ff0000")],
                vec![],
            )],
        ),
    ];
    assert_eq!(body(&drawing, 100.0)?, wanted);
    Ok(())
}

#[test]
fn a_shadow_with_no_blur_is_a_tinted_copy_of_the_path_before_it_and_no_filter() -> Outcome {
    // the copy is the path moved by (dx, dy) in the tint's colour, its own alphas kept (a stroke's included), and the
    // tint's alpha as the copy's opacity, so that the fill and the stroke are one silhouette
    let drawing = Drawing::from(rect(20.0, 20.0, 40.0, 30.0)?)
        .fill(color("#ff000080")?)
        .stroke(Stroke::new(color("#0000ff")?).width(3.0).cap(Cap::Butt))
        .shadow(Shadow::new().dx(4.0).dy(-6.0).blur(0.0).color(color("#12345633")?));
    let strength = (51.0_f64 / 255.0).to_string();
    let half = (128.0_f64 / 255.0).to_string();
    let stroke = [
        ("stroke-width", "3"),
        ("stroke-linejoin", "round"),
        ("stroke-linecap", "butt"),
        ("stroke-miterlimit", "4"),
    ];
    let with = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
        pairs
            .iter()
            .chain(&stroke)
            .map(|&(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    };
    let attributes = |pairs: &[(&str, &str)]| with(pairs).into_iter().collect::<Attributes>();
    let found = body(&drawing, 100.0)?;
    assert_eq!(found.len(), 2, "no filter, no group: {found:?}");
    let copy = Element {
        name: "path".to_owned(),
        attributes: attributes(&[
            ("d", "M24 14 L64 14 L64 44 L24 44 Z"),
            ("fill", "#123456"),
            ("fill-opacity", &half),
            ("stroke", "#123456"),
            ("opacity", &strength),
        ]),
        children: vec![],
    };
    let shape = Element {
        name: "path".to_owned(),
        attributes: attributes(&[
            ("d", "M20 20 L60 20 L60 50 L20 50 Z"),
            ("fill", "#ff0000"),
            ("fill-opacity", &half),
            ("stroke", "#0000ff"),
        ]),
        children: vec![],
    };
    assert_eq!(found, vec![copy, shape]);
    Ok(())
}

#[test]
fn opacity_and_blend_move_to_the_group_that_holds_the_shadow_and_the_shape() -> Outcome {
    let shape = |blur| {
        Ok::<_, Error>(
            Drawing::from(rect(20.0, 20.0, 40.0, 30.0)?)
                .fill(RED)
                .shadow(Shadow::new().dy(0.0).blur(blur))
                .opacity(0.5)
                .blend(Blend::Multiply),
        )
    };
    let group = [("opacity", "0.5"), ("style", "mix-blend-mode:multiply")];
    // blurred: on the group with the filter, and the path carries neither
    let blurred = body(&shape(8.0)?, 100.0)?;
    let (filter, grouped) = (&blurred[0], &blurred[1]);
    assert_eq!(filter.name, "filter");
    assert_eq!(grouped.name, "g");
    for (name, value) in group {
        assert_eq!(grouped.attributes.get(name).map(String::as_str), Some(value), "{name}");
        assert_eq!(grouped.children[0].attributes.get(name), None, "{name} on the path");
    }
    assert_eq!(
        grouped.attributes.get("filter").map(String::as_str),
        Some("url(#shadow-0)")
    );
    // hard: a group of the two paths, neither with opacity of its own beyond the copy's tint
    let hard = body(&shape(0.0)?, 100.0)?;
    assert_eq!(hard.len(), 1, "{hard:?}");
    assert_eq!(hard[0].name, "g");
    for (name, value) in group {
        assert_eq!(hard[0].attributes.get(name).map(String::as_str), Some(value), "{name}");
    }
    assert_eq!(hard[0].children.len(), 2);
    assert_eq!(hard[0].children[1].attributes.get("opacity"), None, "the shape");
    // without either, a hard shadow needs no group
    let plain = body(
        &Drawing::from(rect(20.0, 20.0, 40.0, 30.0)?)
            .fill(RED)
            .shadow(Shadow::new().blur(0.0)),
        100.0,
    )?;
    assert_eq!(
        plain.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["path", "path"]
    );
    Ok(())
}

/// The numbers of an element tree, in order: each attribute value that is one.
fn numbers(elements: &[Element]) -> Vec<f64> {
    let mut found = Vec::new();
    for element in elements {
        for (name, value) in &element.attributes {
            // the opacity of a colour is a share, not a length
            if name.ends_with("opacity") {
                continue;
            }
            if let Ok(number) = value.parse() {
                found.push(number);
            }
        }
        found.extend(numbers(&element.children));
    }
    found
}

#[test]
fn at_twice_the_size_every_length_of_the_filter_is_exactly_twice() -> Outcome {
    let drawing = Drawing::from(rect(20.0, 20.0, 40.0, 30.0)?)
        .fill(RED)
        .shadow(Shadow::new().dx(3.0).dy(-6.5).blur(9.0))
        .with_size(100.0, 100.0);
    let (small, large) = (body(&drawing, 100.0)?, body(&drawing, 200.0)?);
    let (small, large) = (numbers(&small), numbers(&large));
    assert!(
        small.len() >= 6,
        "the filter's region and deviation and offset: {small:?}"
    );
    assert_eq!(large, small.iter().map(|n| n * 2.0).collect::<Vec<_>>());
    Ok(())
}

#[test]
fn a_shape_that_casts_nothing_is_written_as_it_is_without_a_shadow() -> Outcome {
    let shadow = Shadow::new().blur(8.0);
    for (case, with, without) in [
        (
            "nothing painted",
            Drawing::from(rect(10.0, 10.0, 20.0, 20.0)?)
                .fill(Color::NONE)
                .shadow(shadow.clone()),
            Drawing::from(rect(10.0, 10.0, 20.0, 20.0)?).fill(Color::NONE),
        ),
        (
            "an empty path",
            Drawing::from(Path::new()).fill(RED).shadow(shadow.clone()),
            Drawing::from(Path::new()).fill(RED),
        ),
    ] {
        assert_eq!(body(&with, 50.0)?, body(&without, 50.0)?, "{case}");
    }
    Ok(())
}

#[test]
fn a_shadow_beyond_what_a_coordinate_holds_is_refused_naming_the_shape() -> Outcome {
    let rows = [
        ("dx", Shadow::new().dx(1e300)),
        ("dy", Shadow::new().dy(-1e300)),
        ("a blur", Shadow::new().blur(1e300)),
    ];
    for (case, shadow) in rows {
        let drawing = Drawing::from(rect(0.0, 0.0, 10.0, 10.0)?).fill(RED).shadow(shadow);
        let (name, message) = parameter_refusal(drawing.to_svg(Size::px(100.0, 100.0), &Values::new()))?;
        assert!(
            name == "shape 0" && message.starts_with("shadow "),
            "{case}: {name}: {message}"
        );
    }
    // a blur far past what the pixels accept is still a document: the limits of a rasteriser are not the SVG's
    let drawing = Drawing::from(rect(0.0, 0.0, 10.0, 10.0)?)
        .fill(RED)
        .shadow(Shadow::new().blur(1e6));
    assert!(
        drawing
            .to_svg(Size::px(100.0, 100.0), &Values::new())?
            .contains("stdDeviation=\"500000\"")
    );
    Ok(())
}

// The picture -------------------------------------------------------------------------------------------------------

#[test]
fn the_svg_shows_what_the_pixels_show() -> Outcome {
    let paper = Drawing::from(rect(0.0, 0.0, 120.0, 120.0)?).fill(color("#e0d8c8")?);
    let rows = [
        (
            "a blurred rectangle",
            Drawing::from(rect(40.0, 40.0, 40.0, 30.0)?)
                .fill(RED)
                .shadow(Shadow::new().dx(6.0).dy(10.0).blur(14.0)),
        ),
        (
            "a blurred disc with a stroke",
            Drawing::from(Path::circle((60.0, 60.0), 25.0)?)
                .fill(color("#2a5bff")?)
                .stroke(Stroke::new(color("#ffd400")?).width(6.0))
                .shadow(Shadow::new().dx(-8.0).dy(12.0).blur(20.0).color(color("#000000b0")?)),
        ),
        (
            "a hard copy, translucent",
            Drawing::from(rect(30.0, 30.0, 40.0, 30.0)?)
                .fill(color("#ff4f9a")?)
                .stroke(Stroke::new(color("#222222")?).width(5.0))
                .shadow(Shadow::new().dx(12.0).dy(16.0).blur(0.0).color(color("#00000080")?)),
        ),
        (
            "opacity over a shadow",
            Drawing::from(rect(40.0, 40.0, 40.0, 40.0)?)
                .fill(RED)
                .shadow(Shadow::new().dx(8.0).dy(8.0).blur(10.0))
                .opacity(0.6),
        ),
        (
            "multiply over paper",
            Drawing::from(rect(40.0, 40.0, 40.0, 40.0)?)
                .fill(color("#ff4f9a")?)
                .shadow(Shadow::new().dx(8.0).dy(8.0).blur(10.0).color(color("#202080c0")?))
                .blend(Blend::Multiply),
        ),
        (
            "a shape off the canvas",
            Drawing::from(rect(-200.0, 40.0, 40.0, 40.0)?)
                .fill(RED)
                .shadow(Shadow::new().dx(250.0).blur(12.0)),
        ),
    ];
    for (case, drawing) in rows {
        let drawing = Drawing::new([paper.clone(), drawing]);
        let agreement = svg_agrees(&drawing, 120, 120)?;
        // the two renderers cover a curved edge differently (resvg's coverage is off by up to a sixth of the range
        // there): a pixel in a hundred of the edges may differ by more than 40 levels
        assert!(
            agreement.interior <= 4 && agreement.edge_over_40 * 100 <= agreement.edge_pixels,
            "{case}: {agreement:?}"
        );
    }
    Ok(())
}
