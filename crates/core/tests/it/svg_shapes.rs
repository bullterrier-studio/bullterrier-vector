//! The element `Drawing::to_svg` writes for each shape: its paint, stroke, opacity and blend, read back with an XML
//! parser. Expected attributes are written by hand for each row; the shapes are read separately with `shapes()`.

use crate::common::{Outcome, parameter_refusal};
use bullterrier_vector::{Blend, Cap, Color, Drawing, Error, Join, Path, Size, Stroke, Values};
use std::collections::BTreeMap;

type Attributes = BTreeMap<String, String>;

fn square() -> Result<Path, Error> {
    Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)
}

fn color(text: &str) -> Result<Color, Error> {
    Color::parse(text)
}

/// The attributes of every `path` element of the SVG of `drawing` at `width` by `height`, in paint order.
fn elements(drawing: &Drawing, width: f64, height: f64) -> Result<Vec<Attributes>, Box<dyn std::error::Error>> {
    let text = drawing.to_svg(Size::px(width, height), &Values::new())?;
    let document = roxmltree::Document::parse(&text)?;
    Ok(document
        .root_element()
        .children()
        .filter(roxmltree::Node::is_element)
        .map(|element| {
            assert_eq!(element.tag_name().name(), "path", "only paths are children of the root");
            element
                .attributes()
                .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()))
                .collect()
        })
        .collect())
}

/// The attributes of the only element, without its path data.
fn only(drawing: &Drawing, width: f64, height: f64) -> Result<Attributes, Box<dyn std::error::Error>> {
    let mut found = elements(drawing, width, height)?;
    let mut element = match found.as_mut_slice() {
        [only] => std::mem::take(only),
        other => return Err(format!("expected one element, got {}", other.len()).into()),
    };
    element.remove("d").ok_or("no path data")?;
    Ok(element)
}

fn expected(pairs: &[(&str, &str)]) -> Attributes {
    pairs
        .iter()
        .map(|&(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

#[test]
fn a_shape_is_one_path_with_its_paint_stroke_opacity_and_blend() -> Outcome {
    // (case, the drawing, the output side of its 20 by 20 frame, every attribute but the path data)
    let styled = Drawing::from(square()?)
        .fill(color("#ff4f9a")?)
        .stroke(
            Stroke::new(color("#2a5bff")?)
                .width(3.0)
                .join(Join::Miter)
                .cap(Cap::Square)
                .miter_limit(2.5),
        )
        .opacity(0.5)
        .blend(Blend::Multiply);
    let rows = [
        (
            "everything at once, in a frame drawn at twice its size: the width is doubled, the miter limit is not",
            styled.with_size(20.0, 20.0),
            40.0,
            expected(&[
                ("fill", "#ff4f9a"),
                ("stroke", "#2a5bff"),
                ("stroke-width", "6"),
                ("stroke-linejoin", "miter"),
                ("stroke-linecap", "square"),
                ("stroke-miterlimit", "2.5"),
                ("opacity", "0.5"),
                ("style", "mix-blend-mode:multiply"),
            ]),
        ),
        (
            "a closed outline with no paint is filled black",
            Drawing::from(square()?).with_size(20.0, 20.0),
            20.0,
            expected(&[("fill", "#000000")]),
        ),
        (
            "an open line with no paint is stroked black with the defaults",
            Drawing::from(Path::line((0.0, 0.0), (10.0, 5.0))?).with_size(20.0, 20.0),
            20.0,
            expected(&[
                ("fill", "none"),
                ("stroke", "#000000"),
                ("stroke-width", "2"),
                ("stroke-linejoin", "round"),
                ("stroke-linecap", "round"),
                ("stroke-miterlimit", "4"),
            ]),
        ),
        (
            "a fill of none is written, and nothing else",
            Drawing::from(square()?).fill(Color::NONE).stroke(color("#102030")?),
            20.0,
            expected(&[
                ("fill", "none"),
                ("stroke", "#102030"),
                ("stroke-width", "2"),
                ("stroke-linejoin", "round"),
                ("stroke-linecap", "round"),
                ("stroke-miterlimit", "4"),
            ]),
        ),
        (
            "a drawing without a frame draws a width of 2 as 2 at any output size",
            Drawing::from(square()?).stroke(color("#102030")?).fill(Color::NONE),
            300.0,
            expected(&[
                ("fill", "none"),
                ("stroke", "#102030"),
                ("stroke-width", "2"),
                ("stroke-linejoin", "round"),
                ("stroke-linecap", "round"),
                ("stroke-miterlimit", "4"),
            ]),
        ),
        (
            "opacity 0 is written, and a blend of normal is not",
            Drawing::from(square()?).opacity(0.0).blend(Blend::Normal),
            20.0,
            expected(&[("fill", "#000000"), ("opacity", "0")]),
        ),
        (
            "opacities multiply into one",
            Drawing::from(square()?).opacity(0.5).opacity(0.5),
            20.0,
            expected(&[("fill", "#000000"), ("opacity", "0.25")]),
        ),
    ];
    for (case, drawing, side, wanted) in rows {
        assert_eq!(only(&drawing, side, side)?, wanted, "{case}");
    }
    Ok(())
}

#[test]
fn the_alpha_of_a_colour_is_a_fill_or_stroke_opacity_of_alpha_over_255() -> Outcome {
    let drawing = Drawing::from(square()?)
        .fill(color("#ff4f9a80")?)
        .stroke(color("#2a5bff40")?);
    let found = only(&drawing, 10.0, 10.0)?;
    assert_eq!(found["fill"], "#ff4f9a");
    assert_eq!(found["stroke"], "#2a5bff");
    for (name, byte) in [("fill-opacity", 128.0), ("stroke-opacity", 64.0)] {
        let opacity: f64 = found[name].parse()?;
        assert!(
            (opacity * 255.0 - byte).abs() < 1e-9,
            "{name} is {opacity}, not {byte}/255"
        );
    }
    // an opaque colour, written with or without its alpha, has no opacity attribute
    let opaque = only(
        &Drawing::from(square()?)
            .fill(color("#ff4f9aff")?)
            .stroke(color("#2a5bff")?),
        10.0,
        10.0,
    )?;
    assert!(
        !opaque.contains_key("fill-opacity") && !opaque.contains_key("stroke-opacity"),
        "{opaque:?}"
    );
    Ok(())
}

#[test]
fn shapes_are_written_in_paint_order_and_an_empty_path_writes_nothing() -> Outcome {
    let drawing = Drawing::new([
        Drawing::from(square()?).fill(color("#111111")?),
        Drawing::from(Path::default()),
        Drawing::from(square()?).fill(color("#222222")?),
        Drawing::from(Path::default()).fill(color("#333333")?),
        Drawing::from(square()?).fill(color("#444444")?),
    ]);
    let mut fills = Vec::new();
    for mut element in elements(&drawing, 10.0, 10.0)? {
        fills.push(element.remove("fill").ok_or("an element without a fill")?);
    }
    assert_eq!(fills, ["#111111", "#222222", "#444444"]);
    Ok(())
}

#[test]
fn a_shape_scaled_beyond_the_coordinate_limit_is_refused_naming_the_shape() -> Outcome {
    // a coordinate of 1e99 in a frame of 1 pixel drawn at 100,000 pixels is 1e104
    let far = Path::line((0.0, 0.0), (1e99, 0.0))?;
    let drawing = Drawing::new([Drawing::from(square()?), Drawing::from(far)]).with_size(1.0, 1.0);
    let (parameter, message) = parameter_refusal(drawing.to_svg(Size::px(100_000.0, 100_000.0), &Values::new()))?;
    assert_eq!(parameter, "shape 1");
    assert!(message.contains("1e100"), "{message}");
    Ok(())
}

#[test]
fn a_stroke_width_scaled_beyond_the_coordinate_limit_is_refused_naming_the_shape() -> Outcome {
    // a width of 1e99 (accepted: a width has no upper bound) in a frame of 1 pixel drawn at 100,000 pixels is 1e104;
    // 1e308 would be written as `inf`
    for width in [1e99, 1e308] {
        let drawing = Drawing::from(square()?)
            .stroke(Stroke::new(color("#102030")?).width(width))
            .with_size(1.0, 1.0);
        let (parameter, message) = parameter_refusal(drawing.to_svg(Size::px(100_000.0, 100_000.0), &Values::new()))?;
        assert_eq!(parameter, "shape 0", "width {width}");
        assert!(message.contains("stroke width"), "width {width}: {message}");
    }
    Ok(())
}
