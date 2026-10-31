//! `Path` through the public API: building, the canonical path data it writes, the shapes it constructs and its
//! bounds. Geometry is checked against closed forms, written here independently of the implementation, on the path
//! data `to_svg_path` writes.

use bullterrier_vector::{Error, Path, Point, Rect};
use proptest::prelude::*;

type Outcome = Result<(), Box<dyn std::error::Error>>;
/// A path command: its letter and the numbers after it.
type Command = (char, Vec<f64>);
/// The four points of a cubic arc: where it starts, its two handles, where it ends.
type Arc = [(f64, f64); 4];
/// The cubic arcs of an arc command, and the point the path ends on.
type ArcPath = (Vec<Arc>, (f64, f64));

/// The commands of canonical path data: each letter with the numbers after it.
fn commands(data: &str) -> Result<Vec<Command>, Box<dyn std::error::Error>> {
    let mut found: Vec<Command> = Vec::new();
    for token in data.split(' ').filter(|token| !token.is_empty()) {
        let (letter, number) = match token.chars().next() {
            Some(first @ ('M' | 'L' | 'Q' | 'C' | 'Z')) => (Some(first), &token[1..]),
            _ => (None, token),
        };
        if let Some(letter) = letter {
            found.push((letter, Vec::new()));
        }
        if !number.is_empty() {
            found
                .last_mut()
                .ok_or("a number before any command")?
                .1
                .push(number.parse()?);
        }
    }
    Ok(found)
}

/// The parameter and message of the `Error::Parameter` an operation returned; an error if it succeeded or failed
/// otherwise.
fn parameter_refusal<T>(outcome: Result<T, Error>) -> Result<(String, String), Box<dyn std::error::Error>> {
    match outcome {
        Err(Error::Parameter { parameter, message }) => Ok((parameter, message)),
        Err(other) => Err(format!("expected a parameter error, got {other}").into()),
        Ok(_) => Err("expected a parameter error, but it succeeded".into()),
    }
}

#[test]
fn the_test_reader_splits_data_into_commands() -> Outcome {
    assert_eq!(
        commands("M0 0 L1 -2.5 Z")?,
        [('M', vec![0.0, 0.0]), ('L', vec![1.0, -2.5]), ('Z', vec![])]
    );
    assert!(commands("M0 x").is_err());
    assert!(commands("5 5").is_err());
    Ok(())
}

// Building ---------------------------------------------------------------------------------------------------------

#[test]
fn every_verb_is_written_in_canonical_form() -> Outcome {
    let path = Path::start((1.5, -2.0))?
        .line_to((10.0, 4.25))?
        .quad_to((11.0, 5.0), (12.0, 6.5))?
        .curve_to((1.0, 2.0), (3.0, 4.0), (5.0, 6.0))?
        .close()?;
    assert_eq!(path.to_svg_path(), "M1.5 -2 L10 4.25 Q11 5 12 6.5 C1 2 3 4 5 6 Z");
    Ok(())
}

#[test]
fn a_new_sub_path_follows_the_one_before() -> Outcome {
    let path = Path::start((0.0, 0.0))?
        .line_to((1.0, 1.0))?
        .move_to((5.0, 5.0))?
        .line_to((6.0, 7.0))?;
    assert_eq!(path.to_svg_path(), "M0 0 L1 1 M5 5 L6 7");
    Ok(())
}

#[test]
fn a_lone_start_is_a_sub_path_and_a_new_path_has_none() -> Outcome {
    assert_eq!(Path::start((3.0, 4.0))?.to_svg_path(), "M3 4");
    assert_eq!(Path::start((3.0, 4.0))?.move_to((7.0, 8.0))?.to_svg_path(), "M3 4 M7 8");
    assert_eq!(Path::new().to_svg_path(), "");
    assert_eq!(Path::default(), Path::new());
    Ok(())
}

#[test]
fn closing_drops_a_last_line_back_to_the_start() -> Outcome {
    let with_line = Path::start((0.0, 0.0))?
        .line_to((10.0, 0.0))?
        .line_to((10.0, 10.0))?
        .line_to((0.0, 0.0))?
        .close()?;
    let without = Path::start((0.0, 0.0))?
        .line_to((10.0, 0.0))?
        .line_to((10.0, 10.0))?
        .close()?;
    assert_eq!(with_line.to_svg_path(), "M0 0 L10 0 L10 10 Z");
    assert_eq!(with_line, without);
    Ok(())
}

#[test]
fn closing_drops_every_last_line_back_to_the_start_so_that_it_is_still_the_same_path_read_back() -> Outcome {
    let twice = Path::start((0.0, 0.0))?
        .line_to((10.0, 0.0))?
        .line_to((0.0, 0.0))?
        .line_to((0.0, 0.0))?
        .close()?;
    assert_eq!(twice.to_svg_path(), "M0 0 L10 0 Z");
    let only_returns = Path::start((0.0, 0.0))?
        .line_to((0.0, 0.0))?
        .line_to((0.0, 0.0))?
        .close()?;
    assert_eq!(only_returns.to_svg_path(), "M0 0 Z");
    assert_eq!(Path::from_svg_path(&only_returns.to_svg_path())?, only_returns);
    Ok(())
}

#[test]
fn closing_keeps_a_last_segment_that_is_not_a_line_back_to_the_start() -> Outcome {
    let ends_elsewhere = Path::start((0.0, 0.0))?
        .line_to((10.0, 0.0))?
        .line_to((0.0, 5.0))?
        .close()?;
    assert_eq!(ends_elsewhere.to_svg_path(), "M0 0 L10 0 L0 5 Z");
    let curve_home = Path::start((0.0, 0.0))?
        .curve_to((4.0, 1.0), (5.0, 2.0), (0.0, 0.0))?
        .close()?;
    assert_eq!(curve_home.to_svg_path(), "M0 0 C4 1 5 2 0 0 Z");
    let quad_home = Path::start((0.0, 0.0))?.quad_to((4.0, 1.0), (0.0, 0.0))?.close()?;
    assert_eq!(quad_home.to_svg_path(), "M0 0 Q4 1 0 0 Z");
    Ok(())
}

#[test]
fn closing_a_lone_start_or_a_closed_path_is_allowed() -> Outcome {
    assert_eq!(Path::start((3.0, 4.0))?.close()?.to_svg_path(), "M3 4 Z");
    let closed = Path::start((0.0, 0.0))?
        .line_to((10.0, 0.0))?
        .line_to((10.0, 10.0))?
        .close()?;
    assert_eq!(closed.close()?, closed);
    Ok(())
}

#[test]
fn a_segment_after_closing_starts_a_new_sub_path_at_the_closed_start() -> Outcome {
    let closed = Path::start((2.0, 3.0))?
        .line_to((10.0, 3.0))?
        .line_to((10.0, 9.0))?
        .close()?;
    assert_eq!(
        closed.line_to((5.0, 6.0))?.to_svg_path(),
        "M2 3 L10 3 L10 9 Z M2 3 L5 6"
    );
    Ok(())
}

#[test]
fn verbs_return_new_paths_and_leave_the_old_one_alone() -> Outcome {
    let base = Path::start((0.0, 0.0))?;
    let longer = base.line_to((1.0, 2.0))?;
    assert_eq!(base.to_svg_path(), "M0 0");
    assert_eq!(longer.to_svg_path(), "M0 0 L1 2");
    Ok(())
}

#[test]
fn points_may_be_tuples_or_points() -> Outcome {
    let from_tuples = Path::start((1.0, 2.0))?.line_to((3.0, 4.0))?;
    let from_points = Path::start(Point::new(1.0, 2.0))?.line_to(Point::new(3.0, 4.0))?;
    assert_eq!(from_tuples, from_points);
    Ok(())
}

#[test]
fn a_segment_before_any_start_is_refused_naming_the_verb() -> Outcome {
    let empty = Path::new();
    // (verb, what the verb returned)
    let table = [
        ("line_to", empty.line_to((1.0, 2.0))),
        ("quad_to", empty.quad_to((1.0, 2.0), (3.0, 4.0))),
        ("curve_to", empty.curve_to((1.0, 2.0), (3.0, 4.0), (5.0, 6.0))),
        ("close", empty.close()),
    ];
    for (verb, outcome) in table {
        let (parameter, message) = parameter_refusal(outcome)?;
        assert_eq!(parameter, verb);
        assert!(message.contains("move_to"), "{verb}: {message}");
    }
    Ok(())
}

#[test]
fn a_point_that_is_not_finite_is_refused_naming_the_argument_and_the_value() -> Outcome {
    let start = Path::start((0.0, 0.0))?;
    let nan = f64::NAN;
    // (the argument it must name, the value text it must carry, what the verb returned)
    let table = [
        ("point", "NaN", Path::start((nan, 1.0))),
        ("point", "inf", Path::new().move_to((1.0, f64::INFINITY))),
        ("point", "-inf", start.line_to((f64::NEG_INFINITY, 1.0))),
        ("control", "NaN", start.quad_to((1.0, nan), (3.0, 4.0))),
        ("point", "inf", start.quad_to((1.0, 2.0), (f64::INFINITY, 4.0))),
        ("control1", "NaN", start.curve_to((nan, 2.0), (3.0, 4.0), (5.0, 6.0))),
        (
            "control2",
            "inf",
            start.curve_to((1.0, 2.0), (3.0, f64::INFINITY), (5.0, 6.0)),
        ),
        (
            "point",
            "-inf",
            start.curve_to((1.0, 2.0), (3.0, 4.0), (5.0, f64::NEG_INFINITY)),
        ),
    ];
    for (argument, value, outcome) in table {
        let (parameter, message) = parameter_refusal(outcome)?;
        assert_eq!(parameter, argument);
        assert!(message.contains(value), "{argument}: {message:?} lacks {value:?}");
    }
    Ok(())
}

// Writing ----------------------------------------------------------------------------------------------------------

#[test]
fn numbers_are_the_shortest_that_read_back_exactly_and_never_use_an_exponent() -> Outcome {
    // (the number given, how it is written)
    let table = [
        (0.1 + 0.2, "0.30000000000000004"),
        (1e21, "1000000000000000000000"),
        (1e-7, "0.0000001"),
        (-123.456, "-123.456"),
        (5.0, "5"),
        (-0.0, "0"),
    ];
    for (number, written) in table {
        let path = Path::start((number, 1.0))?;
        assert_eq!(path.to_svg_path(), format!("M{written} 1"), "{number:e}");
    }
    Ok(())
}

// Constructors -----------------------------------------------------------------------------------------------------

/// The point each command ends on, in order (`Z` ends nowhere).
fn end_points(data: &str) -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
    commands(data)?
        .into_iter()
        .filter(|(letter, _)| *letter != 'Z')
        .map(|(_, numbers)| match numbers[..] {
            [.., x, y] => Ok((x, y)),
            _ => Err("a command without a point".into()),
        })
        .collect()
}

/// The area the closed polygon through `corners` encloses, positive when it runs clockwise on screen (`y` down): the
/// shoelace formula.
fn shoelace_area(corners: &[(f64, f64)]) -> f64 {
    let mut doubled = 0.0;
    for (index, (x, y)) in corners.iter().enumerate() {
        let (next_x, next_y) = corners[(index + 1) % corners.len()];
        doubled += x * next_y - next_x * y;
    }
    doubled / 2.0
}

fn distance(from: (f64, f64), to: (f64, f64)) -> f64 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    (dx * dx + dy * dy).sqrt()
}

/// Numbers spread evenly over the orders of magnitude from `10^low` to `10^high`: sizes are log-uniform.
fn log_uniform(low: f64, high: f64) -> impl Strategy<Value = f64> {
    (low..high).prop_map(|exponent| libm::pow(10.0, exponent))
}

/// The handle length that makes a cubic quarter arc follow a circle: 4/3 (sqrt 2 - 1) of its radius.
fn kappa() -> f64 {
    4.0 / 3.0 * (2.0_f64.sqrt() - 1.0)
}

/// The middle (t = 1/2) of the cubic Bezier `from`, `c1`, `c2`, `to`.
fn cubic_middle(from: (f64, f64), c1: (f64, f64), c2: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    (
        (from.0 + 3.0 * c1.0 + 3.0 * c2.0 + to.0) / 8.0,
        (from.1 + 3.0 * c1.1 + 3.0 * c2.1 + to.1) / 8.0,
    )
}

/// The cubic arcs of canonical `data`, each as its four points.
fn cubics(data: &str) -> Result<Vec<Arc>, Box<dyn std::error::Error>> {
    let mut arcs = Vec::new();
    let mut current = (0.0, 0.0);
    for (letter, n) in commands(data)? {
        match (letter, &n[..]) {
            ('M' | 'L', &[x, y]) => current = (x, y),
            ('C', &[x1, y1, x2, y2, x, y]) => {
                arcs.push([current, (x1, y1), (x2, y2), (x, y)]);
                current = (x, y);
            }
            ('Z', []) => {}
            _ => return Err(format!("unexpected command {letter} {n:?}").into()),
        }
    }
    Ok(arcs)
}

fn assert_close(found: (f64, f64), expected: (f64, f64), tolerance: f64, what: &str) {
    assert!(
        distance(found, expected) <= tolerance,
        "{what}: {found:?} is not within {tolerance} of {expected:?}"
    );
}

#[test]
fn a_line_is_one_open_segment() -> Outcome {
    assert_eq!(Path::line((1.0, 2.0), (30.0, -4.5))?.to_svg_path(), "M1 2 L30 -4.5");
    let (name, message) = parameter_refusal(Path::line((f64::NAN, 2.0), (3.0, 4.0)))?;
    assert_eq!((name.as_str(), message.contains("NaN")), ("start", true), "{message}");
    let (name, message) = parameter_refusal(Path::line((1.0, 2.0), (3.0, f64::INFINITY)))?;
    assert_eq!((name.as_str(), message.contains("inf")), ("end", true), "{message}");
    Ok(())
}

#[test]
fn a_polyline_runs_through_its_points_open_or_closed() -> Outcome {
    let points = [(1.0, 2.0), (30.0, -4.5), (7.0, 8.0)];
    assert_eq!(Path::polyline(points, false)?.to_svg_path(), "M1 2 L30 -4.5 L7 8");
    assert_eq!(Path::polyline(points, true)?.to_svg_path(), "M1 2 L30 -4.5 L7 8 Z");
    Ok(())
}

#[test]
fn a_closed_polyline_does_not_store_a_last_point_on_its_first() -> Outcome {
    let repeated = [(1.0, 2.0), (30.0, -4.5), (7.0, 8.0), (1.0, 2.0)];
    assert_eq!(Path::polyline(repeated, true)?.to_svg_path(), "M1 2 L30 -4.5 L7 8 Z");
    assert_eq!(
        Path::polyline(repeated, false)?.to_svg_path(),
        "M1 2 L30 -4.5 L7 8 L1 2"
    );
    Ok(())
}

#[test]
fn a_polyline_needs_two_finite_points() -> Outcome {
    let (name, message) = parameter_refusal(Path::polyline([(1.0, 2.0)], false))?;
    assert_eq!(name, "points");
    assert!(message.contains("got 1"), "{message}");
    let (name, _) = parameter_refusal(Path::polyline(Vec::<(f64, f64)>::new(), true))?;
    assert_eq!(name, "points");
    let (name, message) = parameter_refusal(Path::polyline([(1.0, 2.0), (f64::NAN, 3.0)], false))?;
    assert_eq!(name, "points");
    assert!(message.contains("NaN"), "{message}");
    Ok(())
}

#[test]
fn a_rectangle_starts_at_the_left_of_its_top_edge_and_runs_clockwise() -> Outcome {
    let rect = Path::rect(10.0, 20.0, 30.0, 5.0, 0.0)?;
    assert_eq!(rect.to_svg_path(), "M10 20 L40 20 L40 25 L10 25 Z");
    Ok(())
}

#[test]
fn a_rounded_rectangle_has_quarter_arcs_in_its_corners() -> Outcome {
    // x = 10, y = 20, 30 wide, 12 high, corners of radius 2
    let (left, top, right, bottom, r) = (10.0, 20.0, 40.0, 32.0, 2.0);
    let rect = Path::rect(left, top, 30.0, 12.0, r)?;
    let data = rect.to_svg_path();
    let letters: String = commands(&data)?.iter().map(|(letter, _)| *letter).collect();
    assert_eq!(letters, "MLCLCLCLCZ");
    assert_eq!(
        end_points(&data)?,
        [
            (left + r, top),
            (right - r, top),
            (right, top + r),
            (right, bottom - r),
            (right - r, bottom),
            (left + r, bottom),
            (left, bottom - r),
            (left, top + r),
            (left + r, top),
        ]
    );
    // each corner arc is a quarter of the circle of radius r round the corner's inner centre
    let centres = [
        (right - r, top + r),
        (right - r, bottom - r),
        (left + r, bottom - r),
        (left + r, top + r),
    ];
    let arcs = cubics(&data)?;
    assert_eq!(arcs.len(), 4);
    for (arc, centre) in arcs.iter().zip(centres) {
        let middle = cubic_middle(arc[0], arc[1], arc[2], arc[3]);
        let radial_error = distance(middle, centre) - r;
        assert!(
            radial_error.abs() <= 2.7e-4 * r,
            "{middle:?} is off the circle by {radial_error}"
        );
        assert!((distance(arc[0], arc[1]) - kappa() * r).abs() < 1e-12, "{arc:?}");
    }
    Ok(())
}

#[test]
fn a_rectangle_refuses_what_it_cannot_draw() -> Outcome {
    // (x, y, width, height, radius, the argument it must name, the value text)
    let table = [
        (0.0, 0.0, -1.0, 5.0, 0.0, "width", "-1"),
        (0.0, 0.0, 5.0, -2.5, 0.0, "height", "-2.5"),
        (0.0, 0.0, 30.0, 5.0, 2.6, "radius", "2.6"),
        (0.0, 0.0, 5.0, 30.0, 2.6, "radius", "2.6"),
        (0.0, 0.0, 30.0, 5.0, -0.5, "radius", "-0.5"),
        (0.0, 0.0, f64::NAN, 5.0, 0.0, "width", "NaN"),
        (0.0, 0.0, 5.0, f64::INFINITY, 0.0, "height", "inf"),
        (f64::NAN, 0.0, 5.0, 5.0, 0.0, "x", "NaN"),
        (0.0, f64::NEG_INFINITY, 5.0, 5.0, 0.0, "y", "-inf"),
        (0.0, 0.0, 5.0, 5.0, f64::NAN, "radius", "NaN"),
    ];
    for (x, y, width, height, radius, argument, value) in table {
        let (name, message) = parameter_refusal(Path::rect(x, y, width, height, radius))?;
        assert_eq!(name, argument, "{x} {y} {width} {height} {radius}");
        assert!(message.contains(value), "{message:?} lacks {value:?}");
    }
    // the boundaries that succeed: half the shorter side, a flat rectangle, no radius
    assert!(Path::rect(0.0, 0.0, 30.0, 5.0, 2.5).is_ok());
    assert!(Path::rect(0.0, 0.0, 0.0, 5.0, 0.0).is_ok());
    Ok(())
}

#[test]
fn a_circle_starts_at_the_top_and_runs_clockwise_through_four_arcs() -> Outcome {
    // centre (3, -4), radius 7: top, right, bottom, left
    let circle = Path::circle((3.0, -4.0), 7.0)?;
    let data = circle.to_svg_path();
    let letters: String = commands(&data)?.iter().map(|(letter, _)| *letter).collect();
    assert_eq!(letters, "MCCCCZ");
    assert_eq!(
        end_points(&data)?,
        [(3.0, -11.0), (10.0, -4.0), (3.0, 3.0), (-4.0, -4.0), (3.0, -11.0)]
    );
    let first = cubics(&data)?[0];
    let handle = kappa() * 7.0;
    assert_close(
        first[1],
        (3.0 + handle, -11.0),
        1e-12,
        "first handle leaves along the tangent",
    );
    assert_close(
        first[2],
        (10.0, -4.0 - handle),
        1e-12,
        "second handle arrives along the tangent",
    );
    Ok(())
}

#[test]
fn every_arc_of_a_circle_stays_within_the_known_error_of_the_circle() -> Outcome {
    let data = Path::circle((3.0, -4.0), 7.0)?.to_svg_path();
    for arc in cubics(&data)? {
        let middle = cubic_middle(arc[0], arc[1], arc[2], arc[3]);
        let radial_error = distance(middle, (3.0, -4.0)) - 7.0;
        // a kappa arc bulges by at most 2.7e-4 of the radius, outwards
        assert!((0.0..=2.7e-4 * 7.0).contains(&radial_error), "{radial_error}");
    }
    Ok(())
}

#[test]
fn a_circle_refuses_a_radius_that_is_not_above_zero_or_a_centre_that_is_not_finite() -> Outcome {
    // (centre, radius, the argument it must name, the value text)
    let table = [
        ((0.0, 0.0), 0.0, "radius", "got 0"),
        ((0.0, 0.0), -2.5, "radius", "-2.5"),
        ((0.0, 0.0), f64::NAN, "radius", "NaN"),
        ((0.0, 0.0), f64::INFINITY, "radius", "inf"),
        ((f64::NAN, 0.0), 1.0, "center", "NaN"),
        ((0.0, f64::INFINITY), 1.0, "center", "inf"),
        // the arcs would reach beyond what a number holds
        ((1.0e308, 0.0), 1.0e308, "radius", "finite"),
    ];
    for (centre, radius, argument, value) in table {
        let (name, message) = parameter_refusal(Path::circle(centre, radius))?;
        assert_eq!(name, argument, "{centre:?} {radius}");
        assert!(message.contains(value), "{message:?} lacks {value:?}");
    }
    assert!(Path::circle((0.0, 0.0), 1e-9).is_ok());
    Ok(())
}

#[test]
fn a_square_polygon_has_its_corners_exactly_on_the_axes() -> Outcome {
    assert_eq!(
        Path::polygon((0.0, 0.0), 10.0, 4)?.to_svg_path(),
        "M0 -10 L10 0 L0 10 L-10 0 Z"
    );
    assert_eq!(
        Path::polygon((5.0, 7.0), 10.0, 4)?.to_svg_path(),
        "M5 -3 L15 7 L5 17 L-5 7 Z"
    );
    Ok(())
}

#[test]
fn a_triangle_of_radius_2_5_matches_hand_computed_corners() -> Outcome {
    // 2.5 * sqrt(3) / 2 = 2.165063509461097
    let corners = end_points(&Path::polygon((0.0, 0.0), 2.5, 3)?.to_svg_path())?;
    assert_eq!(corners.len(), 3);
    assert_close(corners[0], (0.0, -2.5), 1e-12, "top");
    assert_close(corners[1], (2.165_063_509_461_097, 1.25), 1e-12, "lower right");
    assert_close(corners[2], (-2.165_063_509_461_097, 1.25), 1e-12, "lower left");
    Ok(())
}

#[test]
fn a_polygon_refuses_the_wrong_number_of_sides_or_radius() -> Outcome {
    // (sides, radius, the argument it must name, the value text)
    let table: [(i64, f64, &str, &str); 9] = [
        (2, 10.0, "sides", "got 2"),
        (100_001, 10.0, "sides", "got 100001"),
        (-1, 10.0, "sides", "got -1"),
        // a 32-bit truncation of this would land on 3: it must be refused, naming the value as given
        (4_294_967_299, 10.0, "sides", "got 4294967299"),
        (i64::MIN, 10.0, "sides", "got -9223372036854775808"),
        (5, 0.0, "radius", "got 0"),
        (5, -1.0, "radius", "got -1"),
        (5, f64::NAN, "radius", "NaN"),
        (5, f64::INFINITY, "radius", "inf"),
    ];
    for (sides, radius, argument, value) in table {
        let (name, message) = parameter_refusal(Path::polygon((0.0, 0.0), radius, sides))?;
        assert_eq!(name, argument, "({sides}, {radius})");
        assert!(
            message.contains(value),
            "({sides}, {radius}): {message:?} lacks {value:?}"
        );
    }
    let (name, message) = parameter_refusal(Path::polygon((f64::NAN, 0.0), 1.0, 5))?;
    assert_eq!((name.as_str(), message.contains("NaN")), ("center", true), "{message}");
    // the boundaries that succeed
    assert!(Path::polygon((0.0, 0.0), 1e-9, 3).is_ok());
    assert!(Path::polygon((0.0, 0.0), 1.0e6, 100_000).is_ok());
    Ok(())
}

proptest! {
    // A regular polygon of n corners on a circle of radius r: every corner is r from the centre, every side is
    // 2 r sin(pi / n) long, and the area is n r^2 sin(2 pi / n) / 2, positive because it runs clockwise.
    #[test]
    fn a_polygon_is_regular_on_its_circle_and_runs_clockwise(
        sides in 3u32..=1000,
        radius in log_uniform(-3.0, 6.0),
        centre_in_radii in (-10.0f64..10.0, -10.0f64..10.0),
    ) {
        let centre = (centre_in_radii.0 * radius, centre_in_radii.1 * radius);
        let data = Path::polygon(centre, radius, i64::from(sides)).map_err(|e| TestCaseError::fail(e.to_string()))?.to_svg_path();
        let corners = end_points(&data).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let n = f64::from(sides);
        prop_assert_eq!(u32::try_from(corners.len()), Ok(sides));
        let scale = radius * 11.0;
        for corner in &corners {
            prop_assert!((distance(*corner, centre) - radius).abs() <= 1e-12 * scale);
        }
        let side = 2.0 * radius * libm::sin(std::f64::consts::PI / n);
        for (index, corner) in corners.iter().enumerate() {
            let next = corners[(index + 1) % corners.len()];
            prop_assert!((distance(*corner, next) - side).abs() <= 1e-12 * scale);
        }
        let area = n * radius * radius * libm::sin(2.0 * std::f64::consts::PI / n) / 2.0;
        // the shoelace sum cancels large terms, so measure the polygon about its centre
        let about_centre: Vec<(f64, f64)> = corners.iter().map(|c| (c.0 - centre.0, c.1 - centre.1)).collect();
        prop_assert!((shoelace_area(&about_centre) - area).abs() <= 1e-9 * area);
    }

    // No number the polygon writes is spelled with an exponent, "NaN", "inf" or a lone "-0".
    #[test]
    fn a_polygon_is_written_in_plain_decimal(sides in 3u32..=1000, radius in log_uniform(-3.0, 6.0)) {
        let data = Path::polygon((0.0, 0.0), radius, i64::from(sides)).map_err(|e| TestCaseError::fail(e.to_string()))?.to_svg_path();
        for forbidden in ["e", "E", "NaN", "inf"] {
            prop_assert!(!data.contains(forbidden), "{forbidden} in {data}");
        }
        let lone_negative_zero = data.split([' ', 'M', 'L']).any(|token| token == "-0");
        prop_assert!(!lone_negative_zero, "-0 in {data}");
    }
}

#[test]
fn a_five_pointed_star_has_a_tip_at_the_top() -> Outcome {
    let corners = end_points(&Path::star((200.0, 200.0), 150.0, 60.0, 5)?.to_svg_path())?;
    assert_eq!(corners.len(), 10);
    assert_eq!(corners[0], (200.0, 50.0));
    // the second vertex is an inner one, 36 degrees round clockwise: 60 sin 36 = 35.26711514, 60 cos 36 = 48.54101966
    assert_close(
        corners[1],
        (200.0 + 35.267_115_14, 200.0 - 48.541_019_66),
        1e-6,
        "first inner vertex",
    );
    Ok(())
}

#[test]
fn a_star_refuses_what_it_cannot_draw() -> Outcome {
    // (points, outer, inner, the argument it must name, the value text)
    let table: [(i64, f64, f64, &str, &str); 8] = [
        (1, 10.0, 4.0, "points", "got 1"),
        (100_001, 10.0, 4.0, "points", "got 100001"),
        (5, 0.0, 4.0, "outer", "got 0"),
        (5, -3.0, 4.0, "outer", "-3"),
        (5, 10.0, 0.0, "inner", "got 0"),
        (5, 10.0, f64::NAN, "inner", "NaN"),
        (5, f64::INFINITY, 4.0, "outer", "inf"),
        (-5, 10.0, 4.0, "points", "got -5"),
    ];
    for (points, outer, inner, argument, value) in table {
        let (name, message) = parameter_refusal(Path::star((0.0, 0.0), outer, inner, points))?;
        assert_eq!(name, argument, "({points}, {outer}, {inner})");
        assert!(message.contains(value), "{message:?} lacks {value:?}");
    }
    let (name, message) = parameter_refusal(Path::star((f64::NAN, 0.0), 10.0, 4.0, 5))?;
    assert_eq!((name.as_str(), message.contains("NaN")), ("center", true), "{message}");
    // the boundaries that succeed, and an inner radius above the outer
    assert!(Path::star((0.0, 0.0), 10.0, 4.0, 2).is_ok());
    assert!(Path::star((0.0, 0.0), 4.0, 10.0, 5).is_ok());
    Ok(())
}

proptest! {
    // 2p vertices alternate between the outer and the inner radius, p vertices each, 180 / p degrees apart. Two
    // neighbours are sqrt(R^2 + r^2 - 2 R r cos(pi / p)) apart (law of cosines), and the 2p triangles from the centre
    // add up to an area of p R r sin(pi / p), positive because the star runs clockwise.
    #[test]
    fn a_star_alternates_its_radii_and_runs_clockwise(
        points in 2u32..=60,
        outer in log_uniform(-2.0, 6.0),
        inner in log_uniform(-2.0, 6.0),
    ) {
        let data = Path::star((0.0, 0.0), outer, inner, i64::from(points)).map_err(|e| TestCaseError::fail(e.to_string()))?.to_svg_path();
        let vertices = end_points(&data).map_err(|e| TestCaseError::fail(e.to_string()))?;
        let p = f64::from(points);
        prop_assert_eq!(u32::try_from(vertices.len()), Ok(2 * points));
        let scale = outer.max(inner);
        for (index, vertex) in vertices.iter().enumerate() {
            let radius = if index % 2 == 0 { outer } else { inner };
            prop_assert!((distance(*vertex, (0.0, 0.0)) - radius).abs() <= 1e-12 * scale, "vertex {index}");
        }
        let angle = std::f64::consts::PI / p;
        let edge = (outer * outer + inner * inner - 2.0 * outer * inner * libm::cos(angle)).sqrt();
        for (index, vertex) in vertices.iter().enumerate() {
            let next = vertices[(index + 1) % vertices.len()];
            prop_assert!((distance(*vertex, next) - edge).abs() <= 1e-9 * scale, "edge {index}");
        }
        let area = p * outer * inner * libm::sin(angle);
        prop_assert!((shoelace_area(&vertices) - area).abs() <= 1e-9 * area);
        prop_assert_eq!(vertices[0], (0.0, -outer));
    }
}

// Bounds -----------------------------------------------------------------------------------------------------------

#[test]
fn the_bounds_of_straight_shapes_are_their_extent() -> Outcome {
    let rect = Path::rect(10.0, 20.0, 30.0, 5.0, 0.0)?;
    assert_eq!(
        rect.bounds()?,
        Rect {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 5.0
        }
    );
    let zigzag = Path::polyline([(3.0, -4.0), (-6.0, 9.0), (8.0, 1.0)], false)?;
    assert_eq!(
        zigzag.bounds()?,
        Rect {
            x: -6.0,
            y: -4.0,
            width: 14.0,
            height: 13.0
        }
    );
    Ok(())
}

#[test]
fn the_bounds_of_a_lone_start_are_a_point() -> Outcome {
    let point = Path::start((3.0, -4.0))?;
    assert_eq!(
        point.bounds()?,
        Rect {
            x: 3.0,
            y: -4.0,
            width: 0.0,
            height: 0.0
        }
    );
    Ok(())
}

#[test]
fn the_bounds_of_a_circle_are_the_square_round_it() -> Outcome {
    let circle = Path::circle((3.0, -4.0), 7.0)?;
    assert_eq!(
        circle.bounds()?,
        Rect {
            x: -4.0,
            y: -11.0,
            width: 14.0,
            height: 14.0
        }
    );
    Ok(())
}

#[test]
fn the_bounds_follow_the_curve_not_its_control_points() -> Outcome {
    // y(t) = 30 t (1 - t) peaks at 7.5 for t = 1/2, though the control points reach 10
    let arch = Path::start((0.0, 0.0))?.curve_to((0.0, 10.0), (10.0, 10.0), (10.0, 0.0))?;
    assert_eq!(
        arch.bounds()?,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 7.5
        }
    );
    // y(t) = 20 t (1 - t) peaks at 5 for t = 1/2, though the control point reaches 10
    let bow = Path::start((0.0, 0.0))?.quad_to((5.0, 10.0), (10.0, 0.0))?;
    assert_eq!(
        bow.bounds()?,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 5.0
        }
    );
    // x(t) = 3 t (1 - t)^2 * 20 + 3 t^2 (1 - t) * (-20): the curve swings out to the right first, to x = 5.77..
    let swing = Path::start((0.0, 0.0))?.curve_to((20.0, 0.0), (-20.0, 0.0), (0.0, 0.0))?;
    let found = swing.bounds()?;
    assert!((found.x - -5.773_502_691_896_258).abs() < 1e-12, "{found:?}");
    assert!((found.width - 11.547_005_383_792_516).abs() < 1e-12, "{found:?}");
    Ok(())
}

#[test]
fn the_bounds_cover_every_sub_path() -> Outcome {
    let two = Path::start((0.0, 0.0))?
        .line_to((1.0, 1.0))?
        .move_to((-5.0, 20.0))?
        .line_to((2.0, 30.0))?;
    assert_eq!(
        two.bounds()?,
        Rect {
            x: -5.0,
            y: 0.0,
            width: 7.0,
            height: 30.0
        }
    );
    Ok(())
}

#[test]
fn the_bounds_of_an_empty_path_are_a_geometry_error() {
    match Path::new().bounds() {
        Err(Error::Geometry { verb, message }) => {
            assert_eq!(verb, "bounds");
            assert!(message.contains("empty"), "{message}");
        }
        other => assert!(matches!(other, Err(Error::Geometry { .. })), "{other:?}"),
    }
}

/// The point at `t` on the Bezier of 2 to 4 control `points`, by de Casteljau's repeated blending.
fn bezier_at(points: &[(f64, f64)], t: f64) -> (f64, f64) {
    let mut level = points.to_vec();
    while level.len() > 1 {
        level = level
            .windows(2)
            .map(|pair| {
                (
                    pair[0].0 + (pair[1].0 - pair[0].0) * t,
                    pair[0].1 + (pair[1].1 - pair[0].1) * t,
                )
            })
            .collect();
    }
    level[0]
}

/// Whether `found` is the extent of `samples`, to within 2e-3: no sample outside it (to rounding), and a sample near
/// each side. Sampling a curve at 1/4000 can miss an extremum by under 1e-7 of the control polygon's span (at most
/// 2000 here).
fn bounds_match_samples(found: Rect, samples: &[(f64, f64)]) -> bool {
    let slack = 2e-3;
    let low = |pick: fn(&(f64, f64)) -> f64| samples.iter().map(pick).fold(f64::INFINITY, f64::min);
    let high = |pick: fn(&(f64, f64)) -> f64| samples.iter().map(pick).fold(f64::NEG_INFINITY, f64::max);
    let sides = [
        (found.x, low(|p| p.0)),
        (found.x + found.width, high(|p| p.0)),
        (found.y, low(|p| p.1)),
        (found.y + found.height, high(|p| p.1)),
    ];
    let inside = found.x <= low(|p| p.0) + 1e-9
        && found.y <= low(|p| p.1) + 1e-9
        && found.x + found.width >= high(|p| p.0) - 1e-9
        && found.y + found.height >= high(|p| p.1) - 1e-9;
    inside && sides.iter().all(|(side, sampled)| (side - sampled).abs() <= slack)
}

#[test]
fn the_sample_check_accepts_the_true_bounds_and_rejects_control_polygon_bounds() {
    let controls = [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)];
    let samples: Vec<(f64, f64)> = (0..=4000)
        .map(|step| bezier_at(&controls, f64::from(step) / 4000.0))
        .collect();
    assert!(bounds_match_samples(
        Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 7.5
        },
        &samples
    ));
    assert!(!bounds_match_samples(
        Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0
        },
        &samples
    ));
    assert!(!bounds_match_samples(
        Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 7.0
        },
        &samples
    ));
}

proptest! {
    // Against a different method: sampling the curve.
    #[test]
    fn the_bounds_of_a_cubic_are_where_its_samples_reach(
        controls in prop::array::uniform4((-1000.0f64..1000.0, -1000.0f64..1000.0)),
    ) {
        let path = Path::start(controls[0]).and_then(|p| p.curve_to(controls[1], controls[2], controls[3]))
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        let found = path.bounds().map_err(|e| TestCaseError::fail(e.to_string()))?;
        let samples: Vec<(f64, f64)> = (0..=4000).map(|step| bezier_at(&controls, f64::from(step) / 4000.0)).collect();
        prop_assert!(bounds_match_samples(found, &samples), "{found:?}");
    }

    #[test]
    fn the_bounds_of_a_quad_are_where_its_samples_reach(
        controls in prop::array::uniform3((-1000.0f64..1000.0, -1000.0f64..1000.0)),
    ) {
        let path = Path::start(controls[0]).and_then(|p| p.quad_to(controls[1], controls[2]))
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        let found = path.bounds().map_err(|e| TestCaseError::fail(e.to_string()))?;
        let samples: Vec<(f64, f64)> = (0..=4000).map(|step| bezier_at(&controls, f64::from(step) / 4000.0)).collect();
        prop_assert!(bounds_match_samples(found, &samples), "{found:?}");
    }
}

// Reading path data ------------------------------------------------------------------------------------------------

/// The canonical data `from_svg_path` normalises `data` to.
fn normalised(data: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Path::from_svg_path(data)?.to_svg_path())
}

#[test]
fn absolute_commands_read_as_themselves() -> Outcome {
    // the SVG 1.1 specification's triangle, in the lower-case close it also writes
    assert_eq!(
        normalised("M 100 100 L 300 100 L 200 300 z")?,
        "M100 100 L300 100 L200 300 Z"
    );
    assert_eq!(
        normalised("M1 2 Q3 4 5 6 C7 8 9 10 11 12")?,
        "M1 2 Q3 4 5 6 C7 8 9 10 11 12"
    );
    Ok(())
}

#[test]
fn relative_and_abbreviated_commands_equal_the_absolute_path_written_by_hand() -> Outcome {
    // m, l, h, v, l, z: from (10, 20) to (15, 25), (25, 25), (25, 20), (20, 15), then back to the start
    assert_eq!(
        normalised("m10 20 l5 5 h10 v-5 l-5 -5 z")?,
        "M10 20 L15 25 L25 25 L25 20 L20 15 Z"
    );
    assert_eq!(
        normalised("M10 20 H30 V40 h-5 v-5")?,
        "M10 20 L30 20 L30 40 L25 40 L25 35"
    );
    assert_eq!(
        normalised("M10 10 c1 2 3 4 5 6 q1 1 2 2")?,
        "M10 10 C11 12 13 14 15 16 Q16 17 17 18"
    );
    Ok(())
}

#[test]
fn a_first_relative_move_is_measured_from_the_origin() -> Outcome {
    assert_eq!(normalised("m10 20 l5 5")?, "M10 20 L15 25");
    Ok(())
}

#[test]
fn extra_pairs_after_a_move_are_lines_and_other_commands_repeat() -> Outcome {
    assert_eq!(normalised("M 10 20 30 40 50 60")?, "M10 20 L30 40 L50 60");
    assert_eq!(normalised("m 10 20 1 1 1 1")?, "M10 20 L11 21 L12 22");
    assert_eq!(
        normalised("M0 0 L1 2 3 4 H5 6 V7 8")?,
        "M0 0 L1 2 L3 4 L5 4 L6 4 L6 7 L6 8"
    );
    assert_eq!(
        normalised("M0 0 C1 2 3 4 5 6 7 8 9 10 11 12")?,
        "M0 0 C1 2 3 4 5 6 C7 8 9 10 11 12"
    );
    Ok(())
}

#[test]
fn what_follows_a_close_starts_a_new_sub_path_at_the_start_of_the_closed_one() -> Outcome {
    assert_eq!(normalised("M2 3 L10 3 L10 9 Z L5 6")?, "M2 3 L10 3 L10 9 Z M2 3 L5 6");
    assert_eq!(normalised("M2 3 L10 3 L10 9 z l1 1")?, "M2 3 L10 3 L10 9 Z M2 3 L3 4");
    // a relative move after a close is measured from the start of the closed sub-path
    assert_eq!(normalised("M2 3 L10 3 Z m5 5 l1 1")?, "M2 3 L10 3 Z M7 8 L8 9");
    assert_eq!(normalised("M2 3 L10 3 Z Z")?, "M2 3 L10 3 Z");
    Ok(())
}

#[test]
fn a_line_back_to_the_start_before_a_close_is_not_kept() -> Outcome {
    assert_eq!(normalised("M0 0 L10 0 L0 0 Z")?, "M0 0 L10 0 Z");
    assert_eq!(normalised("M0 0 L10 0 L0 0")?, "M0 0 L10 0 L0 0");
    Ok(())
}

#[test]
fn smooth_curves_reflect_the_last_control_point_of_a_curve_before_them() -> Outcome {
    // S after C reflects (3, 4) about (5, 6): (7, 8)
    assert_eq!(
        normalised("M0 0 C1 2 3 4 5 6 S8 9 10 11")?,
        "M0 0 C1 2 3 4 5 6 C7 8 8 9 10 11"
    );
    // relative: the curve ends at (15, 16) with control (13, 14); s reflects to (17, 18); its own points are relative
    assert_eq!(
        normalised("M10 10 c1 2 3 4 5 6 s3 5 4 4")?,
        "M10 10 C11 12 13 14 15 16 C17 18 18 21 19 20"
    );
    // S after S reflects the second S's control
    assert_eq!(
        normalised("M0 0 C1 2 3 4 5 6 S8 9 10 11 S1 1 2 2")?,
        "M0 0 C1 2 3 4 5 6 C7 8 8 9 10 11 C12 13 1 1 2 2"
    );
    // without a curve before it the first control point is the current point
    assert_eq!(normalised("M0 0 L5 5 S7 8 9 9")?, "M0 0 L5 5 C5 5 7 8 9 9");
    assert_eq!(normalised("M0 0 Q1 2 3 4 S5 5 6 6")?, "M0 0 Q1 2 3 4 C3 4 5 5 6 6");
    assert_eq!(normalised("M0 0 S1 2 3 4")?, "M0 0 C0 0 1 2 3 4");
    Ok(())
}

#[test]
fn smooth_quadratics_reflect_the_last_control_point_of_a_quadratic_before_them() -> Outcome {
    // T after Q reflects (2, 4) about (6, 0): (10, -4)
    assert_eq!(normalised("M0 0 Q2 4 6 0 T12 0")?, "M0 0 Q2 4 6 0 Q10 -4 12 0");
    // and again: (10, -4) about (12, 0) is (14, 4)
    assert_eq!(
        normalised("M0 0 Q2 4 6 0 T12 0 T18 0")?,
        "M0 0 Q2 4 6 0 Q10 -4 12 0 Q14 4 18 0"
    );
    assert_eq!(normalised("M0 0 T4 4")?, "M0 0 Q0 0 4 4");
    assert_eq!(normalised("M0 0 C1 1 2 2 3 3 T6 6")?, "M0 0 C1 1 2 2 3 3 Q3 3 6 6");
    Ok(())
}

#[test]
fn numbers_are_read_in_every_spelling_svg_allows() -> Outcome {
    assert_eq!(normalised("M0.5.5 L-.5-.5")?, "M0.5 0.5 L-0.5 -0.5");
    assert_eq!(normalised("M1e2 2E-1 L+3 4.")?, "M100 0.2 L3 4");
    assert_eq!(normalised("M10-20")?, "M10 -20");
    assert_eq!(normalised("M 1,2 L3 , 4\n\t5,6")?, "M1 2 L3 4 L5 6");
    assert_eq!(normalised("M1,2,3,4")?, "M1 2 L3 4");
    assert_eq!(normalised("M-0 -0.0")?, "M0 0");
    assert_eq!(normalised("M1.5e1 2.5E+1")?, "M15 25");
    Ok(())
}

#[test]
fn empty_data_is_an_empty_path() -> Outcome {
    assert_eq!(Path::from_svg_path("")?, Path::new());
    assert_eq!(Path::from_svg_path(" \n\t ")?, Path::new());
    Ok(())
}

/// The arcs of `data` (one `M`, then one arc), as cubics, with the point the path ends on.
fn arc_of(data: &str) -> Result<ArcPath, Box<dyn std::error::Error>> {
    let canonical = normalised(data)?;
    let arcs = cubics(&canonical)?;
    let end = *end_points(&canonical)?.last().ok_or("no end")?;
    Ok((arcs, end))
}

#[test]
fn the_four_arcs_between_two_points_differ_by_the_large_arc_and_sweep_flags() -> Outcome {
    // From (5, 0) to (0, 5) with radius 5: the circle round (0, 0) or the one round (5, 5), by the short way or the long.
    // (flags, the middle of the arc, how many quarter turns it takes), each worked out by hand
    let m = 5.0 * std::f64::consts::FRAC_1_SQRT_2; // 5 cos 45 degrees = 3.5355339...
    let table = [
        ("0 1", (m, m), 1),
        ("0 0", (5.0 - m, 5.0 - m), 1),
        ("1 1", (5.0 + m, 5.0 + m), 3),
        ("1 0", (-m, -m), 3),
    ];
    for (flags, middle, pieces) in table {
        let (arcs, end) = arc_of(&format!("M5 0 A5 5 0 {flags} 0 5"))?;
        assert_eq!(arcs.len(), pieces, "flags {flags}");
        assert_eq!(arcs[0][0], (5.0, 0.0), "flags {flags}");
        assert_eq!(end, (0.0, 5.0), "flags {flags}");
        let arc = arcs[arcs.len() / 2];
        assert_close(cubic_middle(arc[0], arc[1], arc[2], arc[3]), middle, 2e-3, flags);
    }
    Ok(())
}

#[test]
fn a_relative_arc_equals_the_absolute_one() -> Outcome {
    assert_eq!(
        normalised("M10 10 a5 5 0 0 1 5 5")?,
        normalised("M10 10 A5 5 0 0 1 15 15")?
    );
    Ok(())
}

#[test]
fn arc_flags_may_be_packed_with_the_numbers_after_them() -> Outcome {
    let spaced = normalised("M0 0 a5 5 0 0 0 5 5")?;
    assert_eq!(normalised("M0 0 a5 5 0 00 5 5")?, spaced);
    assert_eq!(normalised("M0 0 a5 5 0 0,0,5,5")?, spaced);
    let large = normalised("M0 0 a5 5 0 1 1 5 5")?;
    assert_eq!(normalised("M0 0 a5 5 0 115 5")?, large);
    assert_eq!(normalised("M0 0 a5 5 0 11 5 5")?, large);
    assert_ne!(spaced, large);
    Ok(())
}

#[test]
fn radii_that_are_too_small_are_scaled_up_to_the_half_circle() -> Outcome {
    // from (0, 0) to (10, 0) the smallest circle is round (5, 0): the top half when it sweeps clockwise
    let (arcs, end) = arc_of("M0 0 A1 1 0 0 1 10 0")?;
    assert_eq!(arcs.len(), 2);
    assert_eq!(end, (10.0, 0.0));
    assert_close(arcs[0][3], (5.0, -5.0), 1e-9, "the top of the half circle");
    // a zero or a negative-sign radius pair draws the same circle as the positive one
    assert_eq!(
        normalised("M0 0 A-1 -1 0 0 1 10 0")?,
        normalised("M0 0 A1 1 0 0 1 10 0")?
    );
    Ok(())
}

#[test]
fn a_rotated_ellipse_arc_passes_where_the_ellipse_does() -> Outcome {
    // radii 10 and 5 turned 90 degrees: 5 along x, 10 along y, centred on (0, 0); from its right end to its bottom
    let (arcs, end) = arc_of("M5 0 A10 5 90 0 1 0 10")?;
    assert_eq!(arcs.len(), 1);
    assert_eq!(end, (0.0, 10.0));
    let middle = cubic_middle(arcs[0][0], arcs[0][1], arcs[0][2], arcs[0][3]);
    // x^2 / 25 + y^2 / 100 = 1 at 45 degrees round: (5 cos 45, 10 sin 45)
    assert_close(
        middle,
        (3.535_533_9, 7.071_067_8),
        5e-3,
        "the middle of the quarter ellipse",
    );
    Ok(())
}

#[test]
fn a_zero_radius_arc_is_a_line_and_an_arc_to_its_own_start_draws_nothing() -> Outcome {
    assert_eq!(normalised("M0 0 A0 5 0 0 1 10 10")?, "M0 0 L10 10");
    assert_eq!(normalised("M0 0 A5 0 0 1 1 10 10")?, "M0 0 L10 10");
    assert_eq!(normalised("M5 5 A9 9 0 1 1 5 5")?, "M5 5");
    assert_eq!(normalised("M5 5 a9 9 0 1 1 0 0 L6 6")?, "M5 5 L6 6");
    Ok(())
}

#[test]
fn a_sweep_takes_at_most_one_cubic_per_quarter_turn() -> Outcome {
    // a half circle is two, a quarter one, three quarters three: never an extra piece for rounding
    let table = [
        ("M10 0 A10 10 0 0 1 0 10", 1),
        ("M0 0 A5 5 0 0 1 10 0", 2),
        ("M10 0 A10 10 0 1 1 0 10", 3),
        ("M0 0 A5 5 0 1 1 0.001 0", 4),
    ];
    for (data, pieces) in table {
        assert_eq!(arc_of(data)?.0.len(), pieces, "{data}");
    }
    Ok(())
}

/// The refusal `from_svg_path` gives `data`: the text of its message, which names the parameter `data`.
fn data_refusal(data: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (parameter, message) = parameter_refusal(Path::from_svg_path(data))?;
    assert_eq!(parameter, "data", "{data:?}");
    Ok(message)
}

#[test]
fn malformed_data_is_refused_naming_the_first_bad_command_and_its_byte() -> Outcome {
    // (data, what the message must carry), the byte being where the command (or its repeat) starts
    let table = [
        ("L 1 2", "`L` at byte 0"),
        ("  l 1 2", "`l` at byte 2"),
        ("M0 0 L1", "`L` at byte 5"),
        ("M0 0 X1 2", "`X` at byte 5"),
        ("M0 0 L1 2 3", "`L` at byte 10"),
        ("M0 0 L1 2,", "`L` at byte 9"),
        ("M0,,0", "`M` at byte 0"),
        ("M0 0 L,1 2", "`L` at byte 5"),
        ("M0 0 A1 1 0 2 0 5 5", "`A` at byte 5"),
        ("M0 0 A1 1 0 1 5 5", "`A` at byte 5"),
        ("M0 0 L1e 2", "`L` at byte 5"),
        ("M0 0 L1e999 2", "`L` at byte 5"),
        ("M0 0 L- 2", "`L` at byte 5"),
        ("M0 0 L1 2 Z 5", "`Z` at byte 10"),
        ("M0 0 Z,", "`Z` at byte 5"),
        ("M0 0 Q1 2 3", "`Q` at byte 5"),
        ("M1e308 0 l1e308 0", "`l` at byte 9"),
        ("M0 0 L1 2 \u{e9}", "`\u{e9}` at byte 10"),
    ];
    for (data, carries) in table {
        let message = data_refusal(data)?;
        assert!(message.contains(carries), "{data:?}: {message:?} lacks {carries:?}");
    }
    Ok(())
}

#[test]
fn well_formed_neighbours_of_the_malformed_data_are_accepted() {
    // the same shapes as the refusals, written correctly
    for data in [
        "M0 0 L1 2",
        "M0 0 L1 2 3 4",
        "M0 0 L1 2,3 4",
        "M0 0 A1 1 0 1 0 5 5",
        "M0 0 L1e2 2",
        "M0 0 L1 2 Z",
        "M0 0 Z",
    ] {
        assert!(Path::from_svg_path(data).is_ok(), "{data:?}");
    }
}

#[test]
fn what_the_library_writes_reads_back_as_the_same_path() -> Outcome {
    let paths = [
        Path::circle((3.0, -4.0), 7.0)?,
        Path::rect(10.0, 20.0, 30.0, 12.0, 2.0)?,
        Path::star((200.0, 200.0), 150.0, 60.0, 5)?,
        Path::polygon((0.1, 0.2), 33.3, 7)?,
        Path::start((0.1 + 0.2, 1e21))?
            .move_to((-0.0, 5e-324))?
            .line_to((1.0 / 3.0, 2.0))?,
        Path::new(),
    ];
    for path in paths {
        assert_eq!(
            Path::from_svg_path(&path.to_svg_path())?,
            path,
            "{}",
            path.to_svg_path()
        );
    }
    Ok(())
}

/// A coordinate across many orders of magnitude, of either sign, and the zeros.
fn coordinate() -> impl Strategy<Value = f64> {
    prop_oneof![
        1 => Just(0.0),
        1 => Just(-0.0),
        2 => -1.0e6f64..1.0e6,
        4 => (log_uniform(-12.0, 12.0), any::<bool>()).prop_map(|(size, negative)| if negative { -size } else { size }),
    ]
}

fn point() -> impl Strategy<Value = (f64, f64)> {
    (coordinate(), coordinate())
}

/// One builder step: a verb with the points it takes.
#[derive(Debug, Clone)]
enum Step {
    Move((f64, f64)),
    Line((f64, f64)),
    Quad((f64, f64), (f64, f64)),
    Curve((f64, f64), (f64, f64), (f64, f64)),
    Close,
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        point().prop_map(Step::Move),
        point().prop_map(Step::Line),
        (point(), point()).prop_map(|(c, p)| Step::Quad(c, p)),
        (point(), point(), point()).prop_map(|(a, b, p)| Step::Curve(a, b, p)),
        Just(Step::Close),
    ]
}

proptest! {
    // A path built with any verbs, written and read back, is the same path, bit for bit.
    #[test]
    fn a_path_written_and_read_back_is_the_same_path(start in point(), steps in prop::collection::vec(step(), 0..24)) {
        let mut path = Path::start(start).map_err(|e| TestCaseError::fail(e.to_string()))?;
        for step in steps {
            path = match step {
                Step::Move(p) => path.move_to(p),
                Step::Line(p) => path.line_to(p),
                Step::Quad(c, p) => path.quad_to(c, p),
                Step::Curve(a, b, p) => path.curve_to(a, b, p),
                Step::Close => path.close(),
            }
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        }
        let data = path.to_svg_path();
        let read = Path::from_svg_path(&data).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(read, path, "{}", data);
    }

    // Arbitrary text is a path or a refusal about `data`, never a panic, and a path it accepts survives writing.
    #[test]
    fn any_text_reads_as_a_path_or_a_refusal(data in prop_oneof!["\\PC{0,40}", "[MmLlHhVvCcSsQqTtAaZz0-9.,+eE \\n-]{0,60}"]) {
        match Path::from_svg_path(&data) {
            Ok(path) => {
                let written = path.to_svg_path();
                prop_assert_eq!(Path::from_svg_path(&written), Ok(path), "{}", written);
            }
            Err(Error::Parameter { parameter, .. }) => prop_assert_eq!(parameter, "data"),
            Err(other) => prop_assert!(false, "{other}"),
        }
    }

    // The same, for text that mostly parses: commands with numbers, flags and arbitrary separators.
    #[test]
    fn data_that_mostly_parses_reads_as_a_path_or_a_refusal(
        commands in prop::collection::vec(("[MmLlHhVvCcSsQqTtAaZz]", prop::collection::vec(coordinate(), 0..9)), 1..8),
        separator in prop_oneof![Just(" "), Just(","), Just(""), Just(" , ")],
    ) {
        let data: String = commands
            .iter()
            .map(|(letter, numbers)| {
                let numbers: Vec<String> = numbers.iter().map(f64::to_string).collect();
                format!("{letter}{}", numbers.join(separator))
            })
            .collect::<Vec<_>>()
            .join(" ");
        match Path::from_svg_path(&data) {
            Ok(path) => {
                let written = path.to_svg_path();
                prop_assert_eq!(Path::from_svg_path(&written), Ok(path), "{}", written);
            }
            Err(Error::Parameter { parameter, .. }) => prop_assert_eq!(parameter, "data"),
            Err(other) => prop_assert!(false, "{other}"),
        }
    }
}

/// What an arc between `from` and `to` of circle radius `radius` must be, by the geometry of circles alone: the two
/// circles of that radius through both points have their centres on the chord's perpendicular bisector, `h` from its
/// middle, where `h^2 = radius^2 - (chord / 2)^2`.
fn circle_centres(from: (f64, f64), to: (f64, f64), radius: f64) -> [(f64, f64); 2] {
    let middle = (f64::midpoint(from.0, to.0), f64::midpoint(from.1, to.1));
    let chord = distance(from, to);
    let height = (radius * radius - chord * chord / 4.0).max(0.0).sqrt();
    let normal = (-(to.1 - from.1) / chord, (to.0 - from.0) / chord);
    [
        (middle.0 + height * normal.0, middle.1 + height * normal.1),
        (middle.0 - height * normal.0, middle.1 - height * normal.1),
    ]
}

/// The turn, in radians, from `a` to `b` as seen from `centre`: positive towards +y (clockwise on screen).
fn turn(centre: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (ax, ay, bx, by) = (a.0 - centre.0, a.1 - centre.1, b.0 - centre.0, b.1 - centre.1);
    libm::atan2(ax * by - ay * bx, ax * bx + ay * by)
}

#[test]
fn the_circle_oracle_tells_the_right_arc_from_the_wrong_one() {
    // the quarter circle from (5, 0) to (0, 5) round (0, 0): the centres are (0, 0) and (5, 5)
    let [first, second] = circle_centres((5.0, 0.0), (0.0, 5.0), 5.0);
    assert_close(first, (0.0, 0.0), 1e-12, "first centre");
    assert_close(second, (5.0, 5.0), 1e-12, "second centre");
    // clockwise on screen is positive; a quarter turn from (5, 0) to (0, 5) round (0, 0)
    assert!((turn((0.0, 0.0), (5.0, 0.0), (0.0, 5.0)) - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert!((turn((0.0, 0.0), (0.0, 5.0), (5.0, 0.0)) + std::f64::consts::FRAC_PI_2).abs() < 1e-12);
}

/// Checks that the arc from `from` to the point `chord` away in `direction` degrees, with radii `fraction` of half the
/// chord (too small), is the half circle on the chord, in the way `(large, sweep)` says.
fn half_circle_on_the_chord(
    from: (f64, f64),
    chord: f64,
    direction: f64,
    fraction: f64,
    (large, sweep): (bool, bool),
) -> Result<(), TestCaseError> {
    let (sin, cos) = (libm::sin(direction.to_radians()), libm::cos(direction.to_radians()));
    let to = (from.0 + chord * cos, from.1 + chord * sin);
    let radius = chord / 2.0 * fraction;
    let data = format!(
        "M{} {} A{radius} {radius} 0 {} {} {} {}",
        from.0,
        from.1,
        u8::from(large),
        u8::from(sweep),
        to.0,
        to.1
    );
    let (arcs, end) = arc_of(&data).map_err(|e| TestCaseError::fail(e.to_string()))?;
    prop_assert_eq!(end, to);
    // rx = ry here, so the half circle's centre is the middle of the chord
    let middle = (f64::midpoint(from.0, to.0), f64::midpoint(from.1, to.1));
    prop_assert_eq!(arcs.len(), 2, "{}", data);
    for arc in &arcs {
        prop_assert!((distance(arc[3], middle) - chord / 2.0).abs() <= 1e-9 * (chord + 100.0));
    }
    let total = turn(middle, from, arcs[0][3]) + turn(middle, arcs[0][3], to);
    prop_assert_eq!(total > 0.0, sweep, "total {}", total);
    Ok(())
}

#[test]
fn a_half_circle_stays_two_pieces_where_rounding_once_left_its_centre_off_the_chord() -> Result<(), TestCaseError> {
    // found by the property below: a radius scaled up leaves the centre's offset at rounding noise, which split the
    // half circle into three pieces; (from, chord, direction, fraction, flags)
    half_circle_on_the_chord(
        (0.0, -83.242_126_876_245_45),
        0.247_534_932_360_133_44,
        99.656_882_483_686_73,
        0.105_708_365_984_272_47,
        (true, false),
    )?;
    half_circle_on_the_chord((0.0, 0.0), 72.571_184_442_606_92, 0.0, 0.01, (false, false))?;
    Ok(())
}

proptest! {
    // An arc of a circle through two points: every junction lies on one circle of the radius (scaled up to half the
    // chord when too small), it turns the way the sweep flag says and by more than a half turn only with the
    // large-arc flag, in the fewest equal pieces of at most a quarter turn.
    #[test]
    fn a_circular_arc_lies_on_a_circle_and_obeys_its_flags(
        from in (-100.0f64..100.0, -100.0f64..100.0),
        chord in log_uniform(-1.0, 2.0),
        direction in 0.0f64..360.0,
        factor in log_uniform(0.0, 2.0),
        rotation in -720.0f64..720.0,
        large in any::<bool>(),
        sweep in any::<bool>(),
    ) {
        // the radius is 1.01 or more times half the chord, so a small arc and a large one differ by at least 17 degrees
        let (sin, cos) = (libm::sin(direction.to_radians()), libm::cos(direction.to_radians()));
        let to = (from.0 + chord * cos, from.1 + chord * sin);
        let radius = chord / 2.0 * 1.01 * factor;
        let data = format!("M{} {} A{radius} {radius} {rotation} {} {} {} {}", from.0, from.1, u8::from(large), u8::from(sweep), to.0, to.1);
        let (arcs, end) = arc_of(&data).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(end, to);
        let mut junctions = vec![arcs[0][0]];
        junctions.extend(arcs.iter().map(|arc| arc[3]));
        let [centre_a, centre_b] = circle_centres(from, to, radius);
        let spread = |centre: (f64, f64)| junctions.iter().map(|j| (distance(*j, centre) - radius).abs()).fold(0.0, f64::max);
        // of the two circles through both ends, the one the arc's pieces follow
        let stray = |centre: (f64, f64)| {
            arcs.iter()
                .map(|arc| (distance(cubic_middle(arc[0], arc[1], arc[2], arc[3]), centre) - radius).abs())
                .fold(0.0, f64::max)
        };
        let centre = if stray(centre_a) <= stray(centre_b) { centre_a } else { centre_b };
        prop_assert!(spread(centre) <= 1e-9 * (radius + 100.0), "spread {}", spread(centre));
        let turns: Vec<f64> = junctions.windows(2).map(|pair| turn(centre, pair[0], pair[1])).collect();
        let total: f64 = turns.iter().sum();
        prop_assert_eq!(total > 0.0, sweep, "total {}", total);
        prop_assert_eq!(total.abs() > std::f64::consts::PI, large, "total {}", total);
        let quarter = std::f64::consts::FRAC_PI_2;
        let pieces = f64::from(u32::try_from(turns.len()).map_err(|e| TestCaseError::fail(e.to_string()))?);
        for piece in &turns {
            prop_assert!(piece.abs() <= quarter * (1.0 + 1e-6), "{}", piece);
            prop_assert!((piece - total / pieces).abs() <= 1e-9, "unequal pieces {:?}", turns);
        }
        // the fewest pieces: one fewer would need a piece over a quarter turn
        prop_assert!((pieces - 1.0) * quarter < total.abs(), "{} pieces for {}", turns.len(), total);
        // each piece follows the circle, within a kappa arc's 2.7e-4 of the radius
        for arc in &arcs {
            let middle = cubic_middle(arc[0], arc[1], arc[2], arc[3]);
            let strays = (distance(middle, centre) - radius).abs();
            prop_assert!(strays <= 2.7e-4 * radius + 1e-9 * (radius + 100.0), "strays {}", strays);
        }
    }

    // A sweep of exactly one, two or three quarter turns takes exactly that many pieces, whatever rounding does to the
    // sweep: arcs between points that are computed on a circle, a quarter turn apart.
    #[test]
    fn whole_quarter_turns_take_one_cubic_each(
        centre in (-100.0f64..100.0, -100.0f64..100.0),
        radius in log_uniform(-1.0, 3.0),
        begin in 0.0f64..360.0,
        quarters in 1u32..=3,
    ) {
        let on_circle = |degrees: f64| {
            let radians = degrees.to_radians();
            (centre.0 + radius * libm::cos(radians), centre.1 + radius * libm::sin(radians))
        };
        let (from, to) = (on_circle(begin), on_circle(begin + 90.0 * f64::from(quarters)));
        // three quarters is a large arc; one is small; two is a half circle, which either flag draws
        let large = u8::from(quarters == 3);
        let data = format!("M{} {} A{radius} {radius} 0 {large} 1 {} {}", from.0, from.1, to.0, to.1);
        let (arcs, _) = arc_of(&data).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(u32::try_from(arcs.len()), Ok(quarters), "{}", data);
    }

    // Radii too small for the chord are scaled up until the arc is the half circle on the chord.
    #[test]
    fn a_too_small_radius_gives_the_half_circle_on_the_chord(
        from in (-100.0f64..100.0, -100.0f64..100.0),
        chord in log_uniform(-1.0, 2.0),
        direction in 0.0f64..360.0,
        fraction in 0.01f64..0.99,
        sweep in any::<bool>(),
        large in any::<bool>(),
    ) {
        half_circle_on_the_chord(from, chord, direction, fraction, (large, sweep))?;
    }
}
