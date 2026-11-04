//! Bending a `Path` along a guide through the public API. A point (x, y) of the path goes to the place `start * length + x`
//! along the guide, `y` to the right of the way the guide runs. Expected points are worked out by hand on straight
//! guides, and from the angle on a circle with the test's own `sin` and `cos`, never from the library's mapping.

use crate::common::{Outcome, commands, distance, geometry_refusal, parameter_refusal};
use bullterrier_vector::{Error, Path, Point};
use proptest::prelude::*;
use std::f64::consts::PI;

/// The points of one sub-path of lines.
type Line = Vec<(f64, f64)>;

/// The points a path made of lines goes through, one list per sub-path.
fn corners(path: &Path) -> Result<Vec<Line>, Box<dyn std::error::Error>> {
    let mut found: Vec<Line> = Vec::new();
    for (letter, numbers) in commands(&path.to_svg_path())? {
        match (letter, &numbers[..]) {
            ('M', &[x, y]) => found.push(vec![(x, y)]),
            ('L', &[x, y]) => found.last_mut().ok_or("a line before a move")?.push((x, y)),
            ('Z', []) => {}
            _ => return Err(format!("not a polyline command: {letter} {numbers:?}").into()),
        }
    }
    Ok(found)
}

/// The points of a path with one sub-path.
fn points(path: &Path) -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
    let mut found = corners(path)?;
    if found.len() == 1 {
        Ok(found.remove(0))
    } else {
        Err(format!("{} sub-paths, not one", found.len()).into())
    }
}

fn assert_near(found: (f64, f64), expected: (f64, f64), tolerance: f64, what: &str) {
    assert!(
        distance(found, expected) <= tolerance,
        "{what}: {found:?} is not within {tolerance} of {expected:?}"
    );
}

/// Where the lone point `(x, y)` goes along `guide`.
fn image(
    guide: &Path,
    start: f64,
    near: Option<Point>,
    x: f64,
    y: f64,
) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let bent = Path::start((x, y))?.along(guide, start, near)?;
    Ok(points(&bent)?[0])
}

fn svg(data: &str) -> Result<Path, Error> {
    Path::from_svg_path(data)
}

// A straight guide ---------------------------------------------------------------------------------------------------

#[test]
fn a_straight_guide_carries_x_along_it_and_y_to_its_right() -> Outcome {
    let east = Path::line((10.0, 20.0), (110.0, 20.0))?;
    let south = Path::line((7.0, 1.0), (7.0, 101.0))?;
    // 3 right and 4 down: 5 long, running (0.6, 0.8), so its right is (-0.8, 0.6)
    let slant = Path::line((0.0, 0.0), (3.0, 4.0))?;
    // (case, guide, start, the path's points, where they go)
    let rows = [
        (
            "east",
            &east,
            0.0,
            vec![(0.0, 0.0), (5.0, 3.0)],
            vec![(10.0, 20.0), (15.0, 23.0)],
        ),
        (
            "start is a fraction of the length",
            &east,
            0.25,
            vec![(0.0, 0.0), (5.0, 3.0)],
            vec![(35.0, 20.0), (40.0, 23.0)],
        ),
        (
            "south, with the right of travel to the west",
            &south,
            0.5,
            vec![(0.0, 0.0), (10.0, -2.0)],
            vec![(7.0, 51.0), (9.0, 61.0)],
        ),
        (
            "slant: along 2 and right 1, then left 1",
            &slant,
            0.0,
            vec![(0.0, 0.0), (2.0, 1.0), (2.0, -1.0)],
            vec![(0.0, 0.0), (0.4, 2.2), (2.0, 1.0)],
        ),
    ];
    for (case, guide, start, shape, expected) in rows {
        let bent = Path::polyline(shape, false)?.along(guide, start, None)?;
        let found = points(&bent)?;
        assert_eq!(found.len(), expected.len(), "{case}: {found:?}");
        for (found, expected) in found.into_iter().zip(expected) {
            assert_near(found, expected, 1e-9, case);
        }
    }
    Ok(())
}

#[test]
fn along_a_straight_guide_a_curve_is_moved_whole() -> Outcome {
    // a circle of radius 20 at (50, 0) bent along an east guide 100 down is the same circle 100 down
    let guide = Path::line((0.0, 100.0), (500.0, 100.0))?;
    let circle = Path::circle((50.0, 0.0), 20.0)?;
    let bent = circle.along(&guide, 0.0, None)?;
    let (want, found) = (circle.translate(0.0, 100.0)?.bounds()?, bent.bounds()?);
    for (found, want, what) in [
        (found.x, want.x, "left"),
        (found.y, want.y, "top"),
        (found.width, want.width, "width"),
        (found.height, want.height, "height"),
    ] {
        assert!((found - want).abs() <= 0.03, "{what}: {found} against {want}");
    }
    assert!((bent.length() - circle.length()).abs() <= 0.1 * circle.length().sqrt());
    Ok(())
}

// A circle guide -----------------------------------------------------------------------------------------------------

/// The point `distance` along the clockwise circle of `radius` round `centre`, walked from the angle `from` (0 at the top,
/// growing clockwise on the screen), with `y` to the right of travel, towards the centre.
fn on_circle(centre: (f64, f64), radius: f64, from: f64, distance: f64, y: f64) -> (f64, f64) {
    let angle = from + distance / radius;
    let reach = radius - y;
    (centre.0 + reach * libm::sin(angle), centre.1 - reach * libm::cos(angle))
}

#[test]
fn along_a_circle_the_angle_is_the_distance_over_the_radius_and_y_moves_the_radius() -> Outcome {
    let (centre, radius) = ((50.0, -20.0), 100.0);
    let guide = Path::circle(centre, radius)?;
    let around = 2.0 * PI * radius;
    // (start, y): the last start runs past the end of the circle, which wraps
    for (start, y) in [(0.0, 0.0), (0.25, 0.0), (0.8, 12.0), (0.8, -12.0), (-0.3, 0.0)] {
        let bent = Path::line((0.0, y), (300.0, y))?.along(&guide, start, None)?;
        let found = points(&bent)?;
        let reach = radius - y;
        for point in &found {
            let from_centre = distance(*point, centre);
            assert!(
                (from_centre - reach).abs() <= 0.06,
                "start {start}, y {y}: {point:?} is {from_centre} from the centre, not {reach}"
            );
        }
        let at = start * around;
        assert_near(found[0], on_circle(centre, radius, 0.0, at, y), 0.1, "first point");
        assert_near(
            found[found.len() - 1],
            on_circle(centre, radius, 0.0, at + 300.0, y),
            0.1,
            "last point",
        );
    }
    Ok(())
}

#[test]
fn a_closed_guide_wraps_the_distance_round_its_length() -> Outcome {
    let guide = Path::circle((0.0, 0.0), 80.0)?;
    let round = guide.length();
    let shape = Path::polyline([(0.0, 0.0), (30.0, 5.0), (60.0, -5.0)], false)?;
    let reference = points(&shape.along(&guide, 0.25, None)?)?;
    for start in [1.25, -0.75, 7.25] {
        let found = points(&shape.along(&guide, start, None)?)?;
        assert_eq!(found.len(), reference.len(), "start {start}");
        for (found, expected) in found.into_iter().zip(&reference) {
            assert_near(found, *expected, 1e-6, &format!("start {start}"));
        }
    }
    // a point a whole turn on lands where it began
    for x in [5.0, 40.0] {
        for turns in [1.0, -2.0] {
            assert_near(
                image(&guide, 0.0, None, x + turns * round, 3.0)?,
                image(&guide, 0.0, None, x, 3.0)?,
                1e-6,
                &format!("x {x}, {turns} turns"),
            );
        }
    }
    Ok(())
}

#[test]
fn near_moves_the_start_of_a_closed_guide() -> Outcome {
    let (centre, radius) = ((0.0, 0.0), 100.0);
    let guide = Path::circle(centre, radius)?;
    // (near, the angle the guide is walked from): the nearest point of the circle, which is then where x = 0 lies
    let rows = [
        (None, 0.0),
        (Some(Point::new(0.0, 1000.0)), PI),
        (Some(Point::new(1000.0, 0.0)), PI / 2.0),
        (Some(Point::new(-300.0, 0.0)), 3.0 * PI / 2.0),
    ];
    for (near, from) in rows {
        let bent = Path::line((0.0, 0.0), (10.0, 0.0))?.along(&guide, 0.0, near)?;
        let found = points(&bent)?;
        assert_near(
            found[0],
            on_circle(centre, radius, from, 0.0, 0.0),
            0.06,
            &format!("{near:?} start"),
        );
        assert_near(
            found[found.len() - 1],
            on_circle(centre, radius, from, 10.0, 0.0),
            0.06,
            &format!("{near:?} end"),
        );
    }
    Ok(())
}

#[test]
fn the_direction_is_the_guides_own_so_far_from_the_guide_a_point_still_lands_on_its_angle() -> Outcome {
    // on a small circle a point 20 from the guide would be moved sideways by 0.2 if the guide's straight edges gave
    // the direction instead of the curve
    let (centre, radius) = ((5.0, 5.0), 30.0);
    let guide = Path::circle(centre, radius)?;
    for x in [0.0, 17.0, 40.0, 77.0, 120.0, 180.0] {
        for y in [-20.0, 15.0] {
            assert_near(
                image(&guide, 0.0, None, x, y)?,
                on_circle(centre, radius, 0.0, x, y),
                0.05,
                &format!("x {x}, y {y}"),
            );
        }
    }
    Ok(())
}

// An open guide ------------------------------------------------------------------------------------------------------

#[test]
fn past_the_ends_of_an_open_guide_the_path_goes_on_straight_along_the_end_direction() -> Outcome {
    // 100 east, then 100 south: 200 long
    let guide = svg("M0 0 L100 0 L100 100")?;
    // (case, start, x, y, where it goes)
    let rows = [
        ("inside, on the second leg", 0.0, 150.0, 0.0, (100.0, 50.0)),
        ("past the end, which runs south", 0.0, 250.0, 0.0, (100.0, 150.0)),
        (
            "past the end, to the right of south is west",
            0.0,
            250.0,
            5.0,
            (95.0, 150.0),
        ),
        ("before the start, which runs east", 0.0, -20.0, 0.0, (-20.0, 0.0)),
        (
            "before the start, to the right of east is south",
            0.0,
            -20.0,
            5.0,
            (-20.0, 5.0),
        ),
        ("a negative start", -0.5, 0.0, 0.0, (-100.0, 0.0)),
        ("a start past the end", 1.5, 0.0, 0.0, (100.0, 200.0)),
        (
            "at the corner the way on is the later leg",
            0.0,
            100.0,
            5.0,
            (95.0, 0.0),
        ),
    ];
    for (case, start, x, y, expected) in rows {
        assert_near(image(&guide, start, None, x, y)?, expected, 1e-9, case);
    }
    Ok(())
}

#[test]
fn the_image_is_continuous_across_the_ends_of_an_open_guide() -> Outcome {
    let guide = svg("M0 0 L100 0 L100 100")?;
    let step = 1e-6;
    for (end, y) in [(0.0, 0.0), (0.0, 5.0), (200.0, 0.0), (200.0, 5.0)] {
        let before = image(&guide, 0.0, None, end - step, y)?;
        let after = image(&guide, 0.0, None, end + step, y)?;
        assert!(
            distance(before, after) <= 2.0 * step + 1e-9,
            "end {end}, y {y}: {before:?} and {after:?}"
        );
    }
    Ok(())
}

#[test]
fn sub_paths_of_a_guide_are_walked_in_order_as_one_length() -> Outcome {
    // 10 east, then 30 south from (0, 5): 40 in all
    let open = svg("M0 0 L10 0 M0 5 L0 35")?;
    // two squares of 40 round, 80 in all
    let closed = svg("M0 0 L10 0 L10 10 L0 10 Z M100 0 L110 0 L110 10 L100 10 Z")?;
    // a closed square, then an open line: open, so it goes on straight past the end
    let mixed = svg("M0 0 L10 0 L10 10 L0 10 Z M100 0 L110 0")?;
    // (case, guide, x, y, where it goes)
    let rows = [
        ("open: in the first", &open, 5.0, 0.0, (5.0, 0.0)),
        ("open: in the second", &open, 15.0, 0.0, (0.0, 10.0)),
        ("open: right of south is west", &open, 15.0, 2.0, (-2.0, 10.0)),
        ("open: the boundary is the later one's", &open, 10.0, 0.0, (0.0, 5.0)),
        ("open: before the start", &open, -3.0, 0.0, (-3.0, 0.0)),
        ("open: past the end, which runs south", &open, 45.0, 0.0, (0.0, 40.0)),
        ("closed: in the second", &closed, 45.0, 0.0, (105.0, 0.0)),
        ("closed: past the end wraps", &closed, 85.0, 0.0, (5.0, 0.0)),
        ("closed: before the start wraps", &closed, -35.0, 0.0, (105.0, 0.0)),
        ("mixed: past the end goes on east", &mixed, 60.0, 0.0, (120.0, 0.0)),
    ];
    for (case, guide, x, y, expected) in rows {
        assert_near(image(guide, 0.0, None, x, y)?, expected, 1e-9, case);
    }
    Ok(())
}

// What is kept -------------------------------------------------------------------------------------------------------

#[test]
fn a_closed_path_stays_closed_and_an_open_one_open_and_each_sub_path_stays_one() -> Outcome {
    let guide = Path::circle((0.0, 0.0), 200.0)?;
    let rows = [
        ("closed", svg("M0 0 L40 0 L40 30 L0 30 Z")?, vec![true]),
        ("open", svg("M0 0 L40 0 L40 30 L0 30")?, vec![false]),
        (
            "both, in order",
            svg("M0 0 L40 0 L40 30 Z M60 0 L90 10")?,
            vec![true, false],
        ),
        ("a lone point", svg("M5 5")?, vec![false]),
    ];
    for (case, shape, closed) in rows {
        let data = shape.along(&guide, 0.1, None)?.to_svg_path();
        let found: Vec<bool> = data
            .split('M')
            .skip(1)
            .map(|sub_path| sub_path.trim_end().ends_with('Z'))
            .collect();
        assert_eq!(found, closed, "{case}: {data}");
    }
    Ok(())
}

#[test]
fn a_lone_point_and_a_line_to_its_own_start_stay_what_they_are() -> Outcome {
    // bent along the line that runs east along y = 0, a path is itself
    let guide = Path::line((0.0, 0.0), (100.0, 0.0))?;
    for data in ["M5 5", "M5 5 L5 5", "M5 5 Z"] {
        assert_eq!(svg(data)?.along(&guide, 0.0, None)?.to_svg_path(), data);
    }
    Ok(())
}

#[test]
fn an_empty_path_bends_to_an_empty_path() -> Outcome {
    let guide = Path::line((0.0, 0.0), (10.0, 0.0))?;
    assert_eq!(Path::new().along(&guide, 0.5, None)?, Path::new());
    Ok(())
}

// Refusals -----------------------------------------------------------------------------------------------------------

#[test]
fn a_guide_with_no_length_or_no_points_is_a_geometry_error_naming_it() -> Outcome {
    let shape = Path::line((0.0, 0.0), (10.0, 0.0))?;
    let rows = [
        ("empty", Path::new(), "empty"),
        ("a lone point", Path::start((3.0, 4.0))?, "no length"),
        (
            "a line to itself",
            Path::start((3.0, 4.0))?.line_to((3.0, 4.0))?,
            "no length",
        ),
        ("several points, all one place", svg("M1 1 M1 1 L1 1 Z")?, "no length"),
    ];
    for (case, guide, said) in rows {
        let (verb, message) = geometry_refusal(shape.along(&guide, 0.0, None))?;
        assert!(verb == "along" && message.contains(said), "{case}: {verb}: {message}");
        // the guide is checked before the path, so an empty path is no excuse
        geometry_refusal(Path::new().along(&guide, 0.0, None))?;
    }
    Ok(())
}

#[test]
fn a_number_the_bend_cannot_use_is_refused_naming_the_argument_and_the_value() -> Outcome {
    let shape = Path::line((0.0, 0.0), (10.0, 0.0))?;
    let open = Path::line((0.0, 0.0), (100.0, 0.0))?;
    let closed = Path::circle((0.0, 0.0), 50.0)?;
    // (case, the result, the argument named, a text the message holds)
    let rows = [
        ("start NaN", shape.along(&open, f64::NAN, None), "start", "NaN"),
        (
            "start infinity",
            shape.along(&closed, f64::INFINITY, None),
            "start",
            "inf",
        ),
        (
            "start -infinity",
            shape.along(&open, f64::NEG_INFINITY, None),
            "start",
            "-inf",
        ),
        (
            "start so large an open guide would carry a point beyond 1e100",
            shape.along(&open, 1e300, None),
            "start",
            "too large",
        ),
        (
            "start so large that a closed guide's distance is not a number",
            shape.along(&closed, 1e308, None),
            "start",
            "too large",
        ),
        (
            "near NaN",
            shape.along(&closed, 0.0, Some(Point::new(f64::NAN, 0.0))),
            "near",
            "NaN",
        ),
        (
            "near beyond 1e100",
            shape.along(&closed, 0.0, Some(Point::new(0.0, 1e101))),
            "near",
            "10000000000",
        ),
        (
            "a guide so long that carrying along it passes 1e100",
            Path::line((0.0, 0.0), (1e98, 0.0))?.along(&Path::line((9.9e99, 0.0), (1e100, 0.0))?, 1.0, None),
            "start",
            "too large",
        ),
    ];
    for (case, result, argument, said) in rows {
        let (name, message) = parameter_refusal(result)?;
        assert!(name == argument && message.contains(said), "{case}: {name}: {message}");
    }
    // and a closed guide takes a start of any size that is a number
    shape.along(&closed, 1e300, None)?;
    Ok(())
}

// The accuracy of the polyline ---------------------------------------------------------------------------------------

/// The distance from `point` to the nearest edge of the polyline through `line`.
fn off_polyline(line: &[(f64, f64)], point: (f64, f64)) -> f64 {
    // a lone point is its own polyline
    let mut nearest = if line.len() == 1 {
        distance(point, line[0])
    } else {
        f64::INFINITY
    };
    for pair in line.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let (along_x, along_y) = (to.0 - from.0, to.1 - from.1);
        let squared = along_x * along_x + along_y * along_y;
        let share = if squared == 0.0 {
            0.0
        } else {
            (((point.0 - from.0) * along_x + (point.1 - from.1) * along_y) / squared).clamp(0.0, 1.0)
        };
        nearest = nearest.min(distance(point, (from.0 + along_x * share, from.1 + along_y * share)));
    }
    nearest
}

proptest! {
    // Every point of the straight edge, carried along the circle by the angle, lies within the flatness of the polyline
    // the bend made, plus what the guide itself strays from a true circle: 2.7e-4 of the radius across, and a
    // pace along its four cubic arcs that is not quite a circle's, which puts a place along the guide off by a small
    // share (taken as 2.5e-4, found by trying) of the distance.
    #[test]
    fn the_polyline_stays_within_a_hundredth_of_the_true_bend(
        radius in 100.0f64..300.0,
        start in 0.0f64..1.0,
        from in (0.0f64..400.0, -30.0f64..30.0),
        to in (0.0f64..400.0, -30.0f64..30.0),
    ) {
        let fail = |e: Error| TestCaseError::fail(e.to_string());
        let centre = (10.0, -40.0);
        let guide = Path::circle(centre, radius).map_err(fail)?;
        let edge = Path::line(from, to).map_err(fail)?;
        let bent = edge.along(&guide, start, None).map_err(fail)?;
        let line = points(&bent).map_err(|e| TestCaseError::fail(e.to_string()))?;
        // an edge is halved at most 8 times
        prop_assert!(line.len() <= 257, "{} points", line.len());
        let reach = start * 2.0 * PI * radius + from.0.max(to.0);
        let tolerance = 0.02 + 3.5e-4 * radius + 2.5e-4 * reach;
        for step in 0..=100 {
            let share = f64::from(step) / 100.0;
            let (x, y) = (from.0 + share * (to.0 - from.0), from.1 + share * (to.1 - from.1));
            let ideal = on_circle(centre, radius, 0.0, start * 2.0 * PI * radius + x, y);
            let off = off_polyline(&line, ideal);
            prop_assert!(off <= tolerance, "{off} from the bend at {share} of the edge (tolerance {tolerance})");
        }
    }
}

proptest! {
    // On the guide itself (y = 0) the bend of every point of an edge is a place `point_at` of the guide gives, so the
    // polyline must stay within the flatness, a hundredth, of the guide's own places, with no allowance for the guide.
    #[test]
    fn a_polyline_on_the_guide_stays_within_a_hundredth_of_the_places_the_guide_gives(
        radius in 100.0f64..300.0,
        start in 0.0f64..1.0,
        from in 0.0f64..400.0,
        to in 0.0f64..400.0,
    ) {
        let fail = |e: Error| TestCaseError::fail(e.to_string());
        let guide = Path::circle((10.0, -40.0), radius).map_err(fail)?;
        let bent = Path::line((from, 0.0), (to, 0.0)).map_err(fail)?.along(&guide, start, None).map_err(fail)?;
        let line = points(&bent).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let length = guide.length();
        for step in 0..=100 {
            let x = from + f64::from(step) / 100.0 * (to - from);
            let place = guide.point_at((start + x / length).rem_euclid(1.0), None).map_err(fail)?;
            let off = off_polyline(&line, (place.x, place.y));
            prop_assert!(off <= 0.0101, "{off} from the guide at x {x}");
        }
    }
}

#[test]
fn a_long_edge_that_meets_the_guide_in_a_symmetric_s_follows_the_s() -> Outcome {
    // the S is point-symmetric about its middle, so the bend of the middle of the edge is the middle of the bent ends
    let guide = svg("M0 0C80 0 20 100 100 100")?;
    let length = guide.length();
    let bent = Path::line((0.0, 0.0), (length, 0.0))?.along(&guide, 0.0, None)?;
    let line = points(&bent)?;
    for step in 0..=100 {
        let at = f64::from(step) / 100.0;
        let place = guide.point_at(at, None)?;
        let off = off_polyline(&line, (place.x, place.y));
        assert!(off <= 0.0101, "{off} from the guide at {at} of it");
    }
    Ok(())
}
