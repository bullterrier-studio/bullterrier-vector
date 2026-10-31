//! The region a `Path` fills (`contains`, the nonzero rule) and its separate parts (`split`), through the public API.
//! `contains` is checked against a different method than the library's: the sum of the signed angles a closed
//! polyline turns through round the point, which is 2 pi times the winding number.

mod common;

use bullterrier_vector::Path;
use common::{Corners, Outcome, build, clearance, distance, log_uniform, parameter_refusal, step, winding};
use proptest::prelude::*;
use std::f64::consts::PI;

#[test]
fn the_reference_counts_turns_and_clearances() {
    let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    let backwards = [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)];
    let twice = [
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
    ];
    // known good: once round, once the other way, twice round, not at all
    assert_eq!(winding(&square, (3.0, 4.0)), 1);
    assert_eq!(winding(&backwards, (3.0, 4.0)), -1);
    assert_eq!(winding(&twice, (3.0, 4.0)), 2);
    // known bad: a point outside is round nothing, and a parity count would call the doubled square outside
    assert_eq!(winding(&square, (13.0, 4.0)), 0);
    assert_eq!(winding(&twice, (13.0, 4.0)), 0);
    // two lobes of a figure eight wind opposite ways
    let eight = [(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)];
    assert_eq!(winding(&eight, (8.0, 5.0)), -winding(&eight, (2.0, 5.0)));
    assert!((clearance(&square, (3.0, 4.0)) - 3.0).abs() < 1e-12);
    assert!(clearance(&square, (10.0, 4.0)) < 1e-12);
    assert!((clearance(&square, (13.0, 14.0)) - 5.0).abs() < 1e-12);
}

// Contains ---------------------------------------------------------------------------------------------------------

#[test]
fn a_rectangle_contains_what_is_inside_it() -> Outcome {
    let rect = Path::rect(3.0, -4.0, 30.0, 17.5, 0.0)?;
    let cases = [
        ((10.0, 0.0), true, "inside"),
        ((3.5, -3.5), true, "just inside the top left corner"),
        ((32.5, 13.0), true, "just inside the bottom right corner"),
        ((40.0, 0.0), false, "right of it"),
        ((2.5, 0.0), false, "just left of it"),
        ((18.0, -4.5), false, "just above it"),
        ((18.0, -3.5), true, "just below its top edge"),
        ((3.0, 20.0), false, "below it"),
    ];
    for (point, expected, what) in cases {
        assert_eq!(rect.contains(point)?, expected, "{what}");
    }
    Ok(())
}

#[test]
fn a_curved_outline_contains_what_is_inside_its_curves() -> Outcome {
    // a lens: two quadratics bulging 30 up and 30 down over a chord of 100 (their apexes are half-way to the controls)
    let lens = Path::from_svg_path("M0 0 Q50 -60 100 0 Q50 60 0 0 Z")?;
    let cases = [
        ((50.0, 0.0), true, "the middle"),
        ((50.0, 25.0), true, "below the middle, within the bulge"),
        ((50.0, -25.0), true, "above the middle, within the bulge"),
        ((50.0, 35.0), false, "below the bulge"),
        ((50.0, -35.0), false, "above the bulge"),
        ((10.0, 25.0), false, "under the pointed end"),
        ((-5.0, 0.0), false, "beyond the point"),
    ];
    for (point, expected, what) in cases {
        assert_eq!(lens.contains(point)?, expected, "{what}");
    }
    Ok(())
}

#[test]
fn an_open_outline_fills_as_if_closed() -> Outcome {
    let open = Path::polyline([(0.0, 0.0), (10.0, 0.0), (5.0, 8.0)], false)?;
    assert!(open.contains((5.0, 3.0))?);
    assert!(!open.contains((5.0, 9.0))?);
    assert!(!open.contains((1.0, 6.0))?, "outside the closing side");
    Ok(())
}

#[test]
fn a_hole_runs_the_other_way_and_a_second_square_the_same_way_fills_it_twice() -> Outcome {
    let outer = "M0 0 L100 0 L100 100 L0 100 Z";
    // the hole runs counter-clockwise; the island runs clockwise like the outer square
    let ring = Path::from_svg_path(&format!("{outer} M25 25 L25 75 L75 75 L75 25 Z"))?;
    let doubled = Path::from_svg_path(&format!("{outer} M25 25 L75 25 L75 75 L25 75 Z"))?;
    assert!(ring.contains((10.0, 50.0))?, "in the ring");
    assert!(!ring.contains((50.0, 50.0))?, "in the hole");
    assert!(doubled.contains((10.0, 50.0))?, "round once");
    assert!(
        doubled.contains((50.0, 50.0))?,
        "round twice is inside under the nonzero rule"
    );
    assert!(!ring.contains((150.0, 50.0))?, "outside");
    Ok(())
}

#[test]
fn a_star_that_crosses_itself_is_filled_through_its_middle() -> Outcome {
    // the pentagram: every second corner of a regular pentagon; its middle is wound round twice
    let corners: Vec<(f64, f64)> = (0..5)
        .map(|index| {
            let angle = 2.0 * PI * f64::from(index) * 2.0 / 5.0;
            (50.0 * libm::sin(angle), -50.0 * libm::cos(angle))
        })
        .collect();
    let pentagram = Path::polyline(corners, true)?;
    assert!(pentagram.contains((0.0, 0.0))?, "the middle");
    assert!(pentagram.contains((0.0, -40.0))?, "a tip");
    assert!(!pentagram.contains((0.0, -60.0))?, "beyond a tip");
    // between the top tip and the next one, 30 from the middle where the star's notch is only 19 deep
    assert!(!pentagram.contains((17.6, -24.3))?, "in a notch");
    Ok(())
}

#[test]
fn nothing_contains_nothing_and_separate_sub_paths_are_separate_regions() -> Outcome {
    assert!(!Path::new().contains((0.0, 0.0))?);
    assert!(!Path::start((1.0, 2.0))?.contains((1.0, 2.0))?);
    let two = Path::from_svg_path("M0 0 L10 0 L10 10 L0 10 Z M20 0 L30 0 L30 10 L20 10 Z")?;
    assert!(two.contains((5.0, 5.0))?);
    assert!(two.contains((25.0, 5.0))?);
    assert!(!two.contains((15.0, 5.0))?, "between them");
    Ok(())
}

#[test]
fn a_point_that_is_not_a_number_cannot_be_asked_about() -> Outcome {
    let square = Path::rect(0.0, 0.0, 10.0, 10.0, 0.0)?;
    let (name, message) = parameter_refusal(square.contains((f64::NAN, 5.0)))?;
    assert_eq!((name.as_str(), message.contains("NaN")), ("point", true), "{message}");
    let (name, message) = parameter_refusal(square.contains((5.0, f64::NEG_INFINITY)))?;
    assert_eq!((name.as_str(), message.contains("inf")), ("point", true), "{message}");
    Ok(())
}

proptest! {
    // Random closed polylines, many of which cross themselves: the library's answer is the reference's, away from
    // the edges. An open polyline fills as the closed one does.
    #[test]
    fn a_polyline_contains_what_is_wound_round(
        size in log_uniform(-2.0, 5.0),
        corners in prop::collection::vec((-1.0f64..1.0, -1.0f64..1.0), 3..10),
        point in (-1.2f64..1.2, -1.2f64..1.2),
    ) {
        let corners: Corners = corners.into_iter().map(|(x, y)| (x * size, y * size)).collect();
        let point = (point.0 * size, point.1 * size);
        prop_assume!(clearance(&corners, point) > 1e-6 * size);
        let expected = winding(&corners, point) != 0;
        for closed in [true, false] {
            let path = Path::polyline(corners.clone(), closed).map_err(|e| TestCaseError::fail(e.to_string()))?;
            let found = path.contains(point).map_err(|e| TestCaseError::fail(e.to_string()))?;
            prop_assert_eq!(found, expected, "closed {}", closed);
        }
    }

    // A circle contains the points nearer its centre than its radius.
    #[test]
    fn a_circle_contains_what_is_nearer_than_its_radius(
        radius in log_uniform(0.0, 5.0),
        centre in (-1000.0f64..1000.0, -1000.0f64..1000.0),
        away in (0.0f64..2.0, 0.0f64..360.0),
    ) {
        // the cubic strays 2.7e-4 of the radius and the flattening 0.01 px from the circle: stay clear of both
        prop_assume!((away.0 - 1.0).abs() * radius > 1e-3 * radius + 0.02);
        let angle = away.1.to_radians();
        let point = (centre.0 + away.0 * radius * libm::cos(angle), centre.1 + away.0 * radius * libm::sin(angle));
        let circle = Path::circle(centre, radius).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let found = circle.contains(point).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(found, distance(point, centre) < radius);
    }
}

// Split ------------------------------------------------------------------------------------------------------------

const OUTER: &str = "M0 0 L100 0 L100 100 L0 100 Z";
const HOLE: &str = "M25 25 L25 75 L75 75 L75 25 Z";
const ISLAND: &str = "M40 40 L60 40 L60 60 L40 60 Z";
const POND: &str = "M45 45 L45 55 L55 55 L55 45 Z";

fn parts(data: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(Path::from_svg_path(data)?
        .split()
        .iter()
        .map(Path::to_svg_path)
        .collect())
}

#[test]
fn an_outline_and_its_holes_are_one_part() -> Outcome {
    assert_eq!(parts(&format!("{OUTER} {HOLE}"))?, [format!("{OUTER} {HOLE}")]);
    // a hole written before its outline still belongs to it, and the part keeps the written order
    assert_eq!(parts(&format!("{HOLE} {OUTER}"))?, [format!("{HOLE} {OUTER}")]);
    Ok(())
}

#[test]
fn separate_outlines_and_lines_are_parts_of_their_own_in_path_order() -> Outcome {
    let line = "M200 0 L300 40";
    let other = "M500 500 L600 500 L600 600 L500 600 Z";
    assert_eq!(
        parts(&format!("{line} {OUTER} {HOLE} {other}"))?,
        [line.to_owned(), format!("{OUTER} {HOLE}"), other.to_owned()]
    );
    // each outline takes the holes inside it
    let hole_in_other = "M520 520 L520 580 L580 580 L580 520 Z";
    assert_eq!(
        parts(&format!("{OUTER} {other} {hole_in_other} {HOLE}"))?,
        [format!("{OUTER} {HOLE}"), format!("{other} {hole_in_other}")]
    );
    Ok(())
}

#[test]
fn an_island_in_a_hole_is_a_part_of_its_own() -> Outcome {
    assert_eq!(
        parts(&format!("{OUTER} {HOLE} {ISLAND}"))?,
        [format!("{OUTER} {HOLE}"), ISLAND.to_owned()]
    );
    Ok(())
}

#[test]
fn a_hole_belongs_to_the_innermost_outline_round_it() -> Outcome {
    // outline, hole, island in the hole, pond in the island: two parts, whatever order they are written in
    assert_eq!(
        parts(&format!("{OUTER} {HOLE} {ISLAND} {POND}"))?,
        [format!("{OUTER} {HOLE}"), format!("{ISLAND} {POND}")]
    );
    assert_eq!(
        parts(&format!("{POND} {ISLAND} {HOLE} {OUTER}"))?,
        [format!("{POND} {ISLAND}"), format!("{HOLE} {OUTER}")]
    );
    Ok(())
}

#[test]
fn an_open_line_is_never_an_outline_with_holes_and_never_a_hole() -> Outcome {
    // an open triangle fills as if closed, but a square inside it is not its hole; a line inside a square is no hole
    let triangle = "M0 0 L100 0 L50 90";
    let square = "M40 10 L60 10 L60 30 L40 30 Z";
    assert_eq!(
        parts(&format!("{triangle} {square}"))?,
        [triangle.to_owned(), square.to_owned()]
    );
    let line = "M10 10 L20 20";
    assert_eq!(parts(&format!("{OUTER} {line}"))?, [OUTER.to_owned(), line.to_owned()]);
    Ok(())
}

#[test]
fn a_lone_start_is_a_part_and_an_empty_path_has_none() -> Outcome {
    assert_eq!(parts("M3 4")?, ["M3 4"]);
    assert_eq!(parts("M3 4 M5 6")?, ["M3 4", "M5 6"]);
    assert_eq!(Path::new().split().len(), 0);
    Ok(())
}

/// The sub-paths of canonical path data, each as its own text.
fn sub_paths(data: &str) -> Vec<String> {
    data.split('M')
        .filter(|text| !text.is_empty())
        .map(|text| format!("M{}", text.trim_end()))
        .collect()
}

proptest! {
    // Splitting loses nothing and adds nothing: the parts hold the sub-paths of the whole, and their lengths add up.
    #[test]
    fn the_parts_are_the_whole(
        size in log_uniform(0.0, 4.0),
        start in (-1.0f64..1.0, -1.0f64..1.0),
        steps in prop::collection::vec(step((-1.0f64..1.0, -1.0f64..1.0)), 0..14),
    ) {
        let scale = |(x, y): (f64, f64)| (x * size, y * size);
        let steps: Vec<_> = steps.into_iter().map(|step| match step {
            common::Step::Move(p) => common::Step::Move(scale(p)),
            common::Step::Line(p) => common::Step::Line(scale(p)),
            common::Step::Quad(c, p) => common::Step::Quad(scale(c), scale(p)),
            common::Step::Curve(a, b, p) => common::Step::Curve(scale(a), scale(b), scale(p)),
            common::Step::Close => common::Step::Close,
        }).collect();
        let whole = build(scale(start), &steps).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let pieces = whole.split();
        let mut found: Vec<String> = pieces.iter().flat_map(|part| sub_paths(&part.to_svg_path())).collect();
        let mut expected = sub_paths(&whole.to_svg_path());
        found.sort();
        expected.sort();
        prop_assert_eq!(found, expected);
        let total: f64 = pieces.iter().map(Path::length).sum();
        prop_assert!((total - whole.length()).abs() <= 1e-9 * (whole.length() + 1.0), "{} against {}", total, whole.length());
    }
}
