//! `Drawing::to_svg` through the public API: the document it writes, read back with an XML parser, and the sizes it
//! refuses. Expected values are written by hand or come from the parser, never from the writer's own formulas.

mod common;

use bullterrier_vector::{Drawing, Size, Values};
use common::{Outcome, parameter_refusal};

/// The SVG of `drawing` at `width` by `height` pixels with no Param values.
fn svg(drawing: &Drawing, width: f64, height: f64) -> Result<String, bullterrier_vector::Error> {
    drawing.to_svg(Size::px(width, height), &Values::new())
}

fn empty() -> Drawing {
    Drawing::new(Vec::<Drawing>::new())
}

#[test]
fn an_empty_drawing_is_a_valid_svg_of_the_stated_size() -> Outcome {
    let text = svg(&empty(), 640.0, 480.0)?;
    let document = roxmltree::Document::parse(&text)?;
    let root = document.root_element();
    assert_eq!(root.tag_name().name(), "svg");
    assert_eq!(root.tag_name().namespace(), Some("http://www.w3.org/2000/svg"));
    assert_eq!(root.attribute("width"), Some("640"));
    assert_eq!(root.attribute("height"), Some("480"));
    assert_eq!(root.attribute("viewBox"), Some("0 0 640 480"));
    assert_eq!(root.attribute("style"), Some("isolation:isolate"));
    assert_eq!(root.children().filter(roxmltree::Node::is_element).count(), 0);
    Ok(())
}

#[test]
fn a_size_outside_whole_pixels_from_1_to_100000_is_refused_naming_the_side() -> Outcome {
    // (case, width, height, the side named, the value shown)
    let rows = [
        ("zero width", 0.0, 10.0, "width", "0"),
        ("zero height", 10.0, 0.0, "height", "0"),
        ("half a pixel wide", 0.5, 10.0, "width", "0.5"),
        ("a fraction high", 10.0, 1080.5, "height", "1080.5"),
        ("not a number wide", f64::NAN, 10.0, "width", "NaN"),
        ("not a number high", 10.0, f64::NAN, "height", "NaN"),
        ("infinite", f64::INFINITY, 10.0, "width", "inf"),
        ("negative", 10.0, -4.0, "height", "-4"),
        ("one past the limit wide", 100_001.0, 10.0, "width", "100001"),
        ("one past the limit high", 10.0, 100_001.0, "height", "100001"),
    ];
    for (case, width, height, side, shown) in rows {
        let (parameter, message) = parameter_refusal(svg(&empty(), width, height))?;
        assert_eq!(parameter, side, "{case}");
        assert!(message.contains(shown), "{case}: {message}");
    }
    Ok(())
}

#[test]
fn the_smallest_and_the_largest_sides_are_accepted() -> Outcome {
    for (width, height) in [(1.0, 1.0), (100_000.0, 1.0), (1.0, 100_000.0)] {
        let text = svg(&empty(), width, height)?;
        let document = roxmltree::Document::parse(&text)?;
        let root = document.root_element();
        assert_eq!(root.attribute("width"), Some(width.to_string().as_str()));
        assert_eq!(root.attribute("height"), Some(height.to_string().as_str()));
    }
    Ok(())
}
