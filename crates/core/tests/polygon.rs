//! The walking skeleton's behaviour through the public API only: `polygon_svg` and its `Error`.

use bullterrier_vector::{Error, polygon_svg};
use proptest::prelude::*;

type Outcome = Result<(), Box<dyn std::error::Error>>;
/// The parameter an invalid request must name and the value text its message carries; `None` when it must succeed.
type Refusal = Option<(&'static str, &'static str)>;

/// The `points` attribute of the one `<polygon>` in `svg`.
fn points_attribute(svg: &str) -> Result<String, Box<dyn std::error::Error>> {
    let document = roxmltree::Document::parse(svg)?;
    let polygon = document
        .descendants()
        .find(|node| node.has_tag_name("polygon"))
        .ok_or("no <polygon>")?;
    Ok(polygon.attribute("points").ok_or("no points attribute")?.to_owned())
}

/// The vertices of the polygon in `svg`, parsed back from its `points` attribute.
fn points(svg: &str) -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
    points_attribute(svg)?
        .split(' ')
        .map(|pair| {
            let (x, y) = pair.split_once(',').ok_or("a point without a comma")?;
            Ok((x.parse()?, y.parse()?))
        })
        .collect()
}

/// The four numbers of the root element's `viewBox`.
fn view_box(svg: &str) -> Result<[f64; 4], Box<dyn std::error::Error>> {
    let document = roxmltree::Document::parse(svg)?;
    let text = document.root_element().attribute("viewBox").ok_or("no viewBox")?;
    let numbers = text.split(' ').map(str::parse).collect::<Result<Vec<f64>, _>>()?;
    numbers.try_into().map_err(|_| "viewBox without four numbers".into())
}

#[test]
fn a_square_of_radius_10_has_its_corners_on_the_axes() -> Outcome {
    let svg = polygon_svg(4, 10.0)?;
    assert_eq!(points(&svg)?, [(0.0, -10.0), (10.0, 0.0), (0.0, 10.0), (-10.0, 0.0)]);
    assert_eq!(points_attribute(&svg)?, "0,-10 10,0 0,10 -10,0");
    Ok(())
}

#[test]
fn a_triangle_of_radius_2_5_matches_hand_computed_corners() -> Outcome {
    // 2.5 * sqrt(3) / 2 = 2.1650635..., rounded to six decimals
    let svg = polygon_svg(3, 2.5)?;
    assert_eq!(points_attribute(&svg)?, "0,-2.5 2.165064,1.25 -2.165064,1.25");
    Ok(())
}

#[test]
fn the_view_box_frames_the_circle() -> Outcome {
    assert_eq!(view_box(&polygon_svg(7, 3.5)?)?, [-3.5, -3.5, 7.0, 7.0]);
    Ok(())
}

#[test]
fn the_document_is_exactly_the_documented_shape() -> Outcome {
    assert_eq!(
        polygon_svg(4, 10.0)?,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20"><polygon points="0,-10 10,0 0,10 -10,0"/></svg>"#
    );
    Ok(())
}

proptest! {
    #[test]
    fn one_point_per_side(sides in 3u32..=1000, radius in 0.001f64..=1_000_000.0) {
        let svg = polygon_svg(i64::from(sides), radius).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let found = points(&svg).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(found.len(), sides as usize);
    }

    #[test]
    fn every_point_lies_on_the_circle(sides in 3u32..=1000, radius in 0.001f64..=1_000_000.0) {
        let svg = polygon_svg(i64::from(sides), radius).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let found = points(&svg).map_err(|e| TestCaseError::fail(e.to_string()))?;
        // oracle: Pythagoras; each coordinate is rounded to 1e-6, so at most 0.5e-6 off, and the distance sqrt(2) times that
        let tolerance = 1e-6 * std::f64::consts::SQRT_2 + radius * 1e-12;
        for (x, y) in found {
            let distance = (x * x + y * y).sqrt();
            prop_assert!((distance - radius).abs() <= tolerance, "({x}, {y}) is {distance} from the centre, not {radius}");
        }
    }

    #[test]
    fn points_are_written_plainly(sides in 3u32..=1000, radius in 0.001f64..=1_000_000.0) {
        let svg = polygon_svg(i64::from(sides), radius).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let attribute = points_attribute(&svg).map_err(|e| TestCaseError::fail(e.to_string()))?;
        for forbidden in ["e", "E", "NaN", "inf"] {
            prop_assert!(!attribute.contains(forbidden), "{forbidden} in {attribute}");
        }
        let lone_negative_zero = attribute
            .split([' ', ','])
            .any(|token| token == "-0");
        prop_assert!(!lone_negative_zero, "-0 in {attribute}");
    }
}

#[test]
fn invalid_values_are_refused_naming_the_parameter() {
    // (sides, radius, refusal); the boundaries that succeed have none
    let table: [(i64, f64, Refusal); 10] = [
        (2, 10.0, Some(("sides", "2"))),
        (1001, 10.0, Some(("sides", "1001"))),
        (-1, 10.0, Some(("sides", "-1"))),
        (5, 0.0, Some(("radius", "0"))),
        (5, -1.0, Some(("radius", "-1"))),
        (5, 1_000_000.5, Some(("radius", "1000000.5"))),
        (5, f64::NAN, Some(("radius", "NaN"))),
        (5, f64::INFINITY, Some(("radius", "inf"))),
        (3, 1e-9, None),
        (1000, 1_000_000.0, None),
    ];
    for (sides, radius, expected) in table {
        match polygon_svg(sides, radius) {
            Err(Error::Parameter { parameter, message }) => {
                let (name, value) = expected.unwrap_or(("(none)", "(none)"));
                assert_eq!(parameter, name, "({sides}, {radius})");
                assert!(
                    message.contains(value),
                    "({sides}, {radius}): {message:?} lacks {value:?}"
                );
            }
            Err(other) => assert_eq!(other.to_string(), "a parameter error", "({sides}, {radius})"),
            Ok(_) => assert_eq!(expected, None, "({sides}, {radius}) was accepted"),
        }
    }
}
